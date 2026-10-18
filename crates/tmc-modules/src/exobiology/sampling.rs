//! Máquina de muestreo de exobiología: Log -> Sample -> Analyse con las
//! posiciones persistidas en `exo_sample` para sobrevivir a reinicios.

use super::geo::{RelDir, bearing_deg, compass, haversine_m, rel_dir, relative_bearing};
use super::persist;
use crate::data::species_table;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tmc_core::ModuleCtx;
use tmc_model::events::{OrganicScanType, ScanOrganic};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplePoint {
    pub lat: f64,
    pub lon: f64,
    pub ts: DateTime<Utc>,
}

/// Muestreo en curso (o el último completado). `scans` cuenta las fases vistas
/// aunque no hubiera posición; `points` solo guarda las que la tenían.
#[derive(Debug, Clone, PartialEq)]
pub struct Sampling {
    pub system_address: u64,
    pub body_id: u32,
    pub species: String,
    pub genus: String,
    pub points: Vec<SamplePoint>,
    pub scans: u8,
    pub complete: bool,
}

/// Una fila de `exo_sample`; `ts` es la marca del diario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExoSampleRow {
    pub cmdr: String,
    pub system_address: u64,
    pub body_id: u32,
    pub species: String,
    pub scan_type: String,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub ts: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SamplingChange {
    Started,
    Progressed(u8),
    Completed,
    Discarded { previous_species: String },
    Ignored,
}

#[derive(Debug, Default)]
pub struct SamplingTracker {
    current: Option<Sampling>,
    completed: Vec<(u64, u32, String)>,
    /// Posiciones guardadas por `ts` normalizado, para el replay (sin Status).
    stored: HashMap<String, (f64, f64)>,
}

fn ts_key(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Nanos, true)
}

fn genus_of(species: &str) -> String {
    species_table()
        .species(species)
        .map(|info| info.genus.clone())
        .unwrap_or_default()
}

impl SamplingTracker {
    pub fn sampling(&self) -> Option<&Sampling> {
        self.current.as_ref()
    }

    pub fn completed(&self) -> &[(u64, u32, String)] {
        &self.completed
    }

    pub fn on_scan_organic(
        &mut self,
        ev: &ScanOrganic,
        pos: Option<(f64, f64)>,
        ts: DateTime<Utc>,
    ) -> SamplingChange {
        let (Some(system_address), Some(body_id), Some(species)) =
            (ev.system_address, ev.body, ev.species.clone())
        else {
            return SamplingChange::Ignored;
        };
        let point = pos.map(|(lat, lon)| SamplePoint { lat, lon, ts });
        match ev.scan_type {
            OrganicScanType::Log => {
                let previous = self.current.take().filter(|cur| {
                    !cur.complete
                        && (cur.species != species
                            || cur.body_id != body_id
                            || cur.system_address != system_address)
                });
                let genus = ev.genus.clone().unwrap_or_else(|| genus_of(&species));
                self.current = Some(Sampling {
                    system_address,
                    body_id,
                    species,
                    genus,
                    points: point.into_iter().collect(),
                    scans: 1,
                    complete: false,
                });
                match previous {
                    Some(prev) => SamplingChange::Discarded {
                        previous_species: prev.species,
                    },
                    None => SamplingChange::Started,
                }
            }
            OrganicScanType::Sample | OrganicScanType::Analyse => {
                let analyse = ev.scan_type == OrganicScanType::Analyse;
                let Some(cur) = self.current.as_mut().filter(|cur| {
                    !cur.complete
                        && cur.species == species
                        && cur.body_id == body_id
                        && cur.system_address == system_address
                }) else {
                    return SamplingChange::Ignored;
                };
                cur.scans = cur.scans.saturating_add(1);
                cur.points.extend(point);
                if analyse || cur.scans >= 3 {
                    cur.complete = true;
                    let key = (system_address, body_id, cur.species.clone());
                    if !self.completed.contains(&key) {
                        self.completed.push(key);
                    }
                    SamplingChange::Completed
                } else {
                    SamplingChange::Progressed(cur.scans)
                }
            }
            OrganicScanType::Unknown => SamplingChange::Ignored,
        }
    }

    /// Procesa un ScanOrganic del diario. En vivo toma la posición del último
    /// Status y encarga `save_sample`; en replay usa la fila guardada con el
    /// mismo `ts` y no escribe nada.
    pub fn handle_scan(
        &mut self,
        ev: &ScanOrganic,
        cmdr: &str,
        ts: DateTime<Utc>,
        status_pos: Option<(f64, f64)>,
        ctx: &mut ModuleCtx,
    ) -> SamplingChange {
        let pos = if ctx.replaying {
            self.stored.get(&ts_key(ts)).copied()
        } else {
            status_pos
        };
        let change = self.on_scan_organic(ev, pos, ts);
        if !ctx.replaying
            && change != SamplingChange::Ignored
            && let (Some(system_address), Some(body_id), Some(species)) =
                (ev.system_address, ev.body, ev.species.clone())
        {
            let scan_type = match ev.scan_type {
                OrganicScanType::Log => "Log",
                OrganicScanType::Sample => "Sample",
                _ => "Analyse",
            };
            let row = ExoSampleRow {
                cmdr: cmdr.to_owned(),
                system_address,
                body_id,
                species,
                scan_type: scan_type.to_owned(),
                lat: pos.map(|p| p.0),
                lon: pos.map(|p| p.1),
                ts,
            };
            ctx.store_task("save_sample", persist::save_sample(cmdr.to_owned(), row));
        }
        change
    }

    /// SellOrganicData: vacía lo completado y borra sus filas (el muestreo
    /// incompleto se conserva).
    pub fn handle_sold(&mut self, cmdr: &str, ctx: &mut ModuleCtx) {
        self.on_sold();
        ctx.store_task("clear_completed", persist::clear_completed(cmdr.to_owned()));
    }

    /// Died: vacía el rastreador y borra todas las filas del comandante.
    pub fn handle_died(&mut self, cmdr: &str, ctx: &mut ModuleCtx) {
        self.died();
        ctx.store_task("clear_samples", persist::clear_samples(cmdr.to_owned()));
    }

    pub fn on_sold(&mut self) {
        self.completed.clear();
        if self.current.as_ref().is_some_and(|cur| cur.complete) {
            self.current = None;
        }
    }

    pub fn died(&mut self) {
        self.current = None;
        self.completed.clear();
        self.stored.clear();
    }

    /// Al empezar un replay: olvida el estado derivado pero conserva las
    /// posiciones guardadas para resolver los ScanOrganic reproducidos.
    pub fn reset_for_replay(&mut self) {
        self.current = None;
        self.completed.clear();
    }

    /// Recupera de `exo_sample` el muestreo abierto más reciente. No pisa un
    /// muestreo ya reconstruido por el replay del diario.
    pub fn restore(&mut self, rows: Vec<ExoSampleRow>) {
        for row in &rows {
            if let (Some(lat), Some(lon)) = (row.lat, row.lon) {
                self.stored.insert(ts_key(row.ts), (lat, lon));
            }
        }
        let Some(last) = rows.last() else { return };
        if self.current.is_some() {
            return;
        }
        let group: Vec<&ExoSampleRow> = rows
            .iter()
            .filter(|row| {
                row.system_address == last.system_address
                    && row.body_id == last.body_id
                    && row.species == last.species
            })
            .collect();
        let scans = u8::try_from(group.len()).unwrap_or(u8::MAX);
        let points = group
            .iter()
            .filter_map(|row| {
                Some(SamplePoint {
                    lat: row.lat?,
                    lon: row.lon?,
                    ts: row.ts,
                })
            })
            .collect();
        self.current = Some(Sampling {
            system_address: last.system_address,
            body_id: last.body_id,
            species: last.species.clone(),
            genus: genus_of(&last.species),
            points,
            scans,
            complete: false,
        });
    }

    fn previous(&self) -> Option<&SamplePoint> {
        self.current.as_ref()?.points.last()
    }

    /// Distancia en metros al último punto muestreado; `None` sin punto previo.
    pub fn distance_to_previous(&self, now: (f64, f64), radius_m: f64) -> Option<f64> {
        let prev = self.previous()?;
        Some(haversine_m(now.0, now.1, prev.lat, prev.lon, radius_m))
    }

    /// Dirección relativa al rumbo y punto cardinal hacia el último punto.
    pub fn direction_to_previous(
        &self,
        now: (f64, f64),
        heading: f64,
    ) -> Option<(RelDir, &'static str)> {
        let prev = self.previous()?;
        let bearing = bearing_deg(now.0, now.1, prev.lat, prev.lon);
        Some((
            rel_dir(relative_bearing(bearing, heading)),
            compass(bearing),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exobiology::persist::{self, MIGRATIONS};
    use chrono::TimeZone;
    use tmc_core::Store;
    use tmc_model::status::Status;

    const SYSTEM: u64 = 3_932_277_478_106;
    const RADIUS: f64 = 1_500_000.0;
    const A: &str = "$Codex_Ent_Bacterial_01_Name;";
    const B: &str = "$Codex_Ent_Bacterial_02_Name;";

    fn exo(n: u8) -> Status {
        Status::parse(match n {
            1 => include_str!("../../../../tests/fixtures/status/exo_01.json"),
            2 => include_str!("../../../../tests/fixtures/status/exo_02.json"),
            _ => include_str!("../../../../tests/fixtures/status/exo_03.json"),
        })
        .unwrap()
    }
    fn pos(s: &Status) -> Option<(f64, f64)> {
        Some((s.latitude?, s.longitude?))
    }
    fn at(sec: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 12, 21, 6, sec).unwrap()
    }
    fn scan(kind: OrganicScanType, species: &str, body: u32) -> ScanOrganic {
        ScanOrganic {
            scan_type: kind,
            genus: Some("$Codex_Ent_Bacterial_Genus_Name;".into()),
            species: Some(species.into()),
            system_address: Some(SYSTEM),
            body: Some(body),
            ..Default::default()
        }
    }
    fn row(kind: &str, sec: u32, lat: f64, lon: f64) -> ExoSampleRow {
        ExoSampleRow {
            cmdr: "CMDR TESTER".into(),
            system_address: SYSTEM,
            body_id: 5,
            species: A.into(),
            scan_type: kind.into(),
            lat: Some(lat),
            lon: Some(lon),
            ts: at(sec),
        }
    }
    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} != {b}");
    }

    #[test]
    fn log_sample_analyse_completes_species() {
        let mut t = SamplingTracker::default();
        let p = |n| pos(&exo(n));
        assert_eq!(
            t.on_scan_organic(&scan(OrganicScanType::Log, A, 5), p(1), at(0)),
            SamplingChange::Started
        );
        assert_eq!(
            t.on_scan_organic(&scan(OrganicScanType::Sample, A, 5), p(2), at(10)),
            SamplingChange::Progressed(2)
        );
        assert_eq!(
            t.on_scan_organic(&scan(OrganicScanType::Analyse, A, 5), p(3), at(20)),
            SamplingChange::Completed
        );
        assert_eq!(t.completed(), &[(SYSTEM, 5, A.to_string())]);
        assert_eq!(t.sampling().unwrap().points.len(), 3);
        assert!(t.sampling().unwrap().complete);
    }

    #[test]
    fn switching_species_mid_sampling_discards_previous() {
        let mut t = SamplingTracker::default();
        t.on_scan_organic(&scan(OrganicScanType::Log, A, 5), Some((10.0, 20.0)), at(0));
        t.on_scan_organic(
            &scan(OrganicScanType::Sample, A, 5),
            Some((10.0, 20.5)),
            at(5),
        );
        let change =
            t.on_scan_organic(&scan(OrganicScanType::Log, B, 5), Some((10.0, 21.0)), at(9));
        assert_eq!(
            change,
            SamplingChange::Discarded {
                previous_species: A.into()
            }
        );
        let s = t.sampling().unwrap();
        assert_eq!(
            (s.species.as_str(), s.points.len(), s.complete),
            (B, 1, false)
        );
        assert!(t.completed().is_empty());
        // A Log on a different body also discards.
        let change = t.on_scan_organic(&scan(OrganicScanType::Log, B, 9), Some((1.0, 1.0)), at(12));
        assert_eq!(
            change,
            SamplingChange::Discarded {
                previous_species: B.into()
            }
        );
        assert_eq!(t.sampling().unwrap().body_id, 9);
        // Starting over after a completed sampling is not a discard.
        t.on_scan_organic(
            &scan(OrganicScanType::Analyse, B, 9),
            Some((1.0, 1.0)),
            at(13),
        );
        let change = t.on_scan_organic(&scan(OrganicScanType::Log, A, 9), None, at(14));
        assert_eq!(change, SamplingChange::Started);
        // A Sample for a species that is not being sampled is ignored.
        let change = t.on_scan_organic(&scan(OrganicScanType::Sample, B, 9), None, at(15));
        assert_eq!(change, SamplingChange::Ignored);
    }

    #[test]
    fn distance_to_previous_uses_planet_radius() {
        let mut t = SamplingTracker::default();
        let (s1, s2) = (exo(1), exo(2));
        t.on_scan_organic(&scan(OrganicScanType::Log, A, 5), pos(&s1), at(0));
        t.on_scan_organic(&scan(OrganicScanType::Sample, A, 5), pos(&s2), at(10));
        let now = pos(&exo(3)).unwrap();
        let radius = s2.planet_radius.unwrap();
        assert_eq!(radius, RADIUS);
        let expected = crate::exobiology::geo::haversine_m(10.0, 20.5, now.0, now.1, radius);
        close(expected, 12_891.102, 1.0);
        close(t.distance_to_previous(now, radius).unwrap(), expected, 1.0);
        assert_eq!(
            SamplingTracker::default().distance_to_previous(now, radius),
            None
        );
    }

    #[test]
    fn direction_to_previous_relative_to_heading() {
        let mut t = SamplingTracker::default();
        t.on_scan_organic(&scan(OrganicScanType::Log, A, 5), Some((0.0, 0.1)), at(0));
        let dir = t.direction_to_previous((0.0, 0.0), 270.0).unwrap();
        assert_eq!(dir, (RelDir::Behind, "E"));
        let dir = t.direction_to_previous((0.0, 0.0), 90.0).unwrap();
        assert_eq!(dir, (RelDir::Ahead, "E"));
    }

    fn open_store() -> Store {
        let store = Store::open_in_memory().unwrap();
        store.apply_migrations(MIGRATIONS).unwrap();
        store
    }

    fn restored(store: &Store) -> SamplingTracker {
        let result = persist::load_open_samples("CMDR TESTER".into())(store).unwrap();
        let rows: Vec<ExoSampleRow> = serde_json::from_value(result["rows"].clone()).unwrap();
        let mut t = SamplingTracker::default();
        t.restore(rows);
        t
    }

    #[test]
    fn sampling_resumes_after_restart_from_exo_sample() {
        let store = open_store();
        let c = || "CMDR TESTER".to_string();
        persist::save_sample(c(), row("Log", 0, 10.0, 20.0))(&store).unwrap();
        persist::save_sample(c(), row("Sample", 10, 10.0, 20.5))(&store).unwrap();
        // Saving the same row twice is idempotent.
        persist::save_sample(c(), row("Sample", 10, 10.0, 20.5))(&store).unwrap();
        let t = restored(&store);
        let s = t.sampling().unwrap();
        assert_eq!(
            (s.species.as_str(), s.body_id, s.scans, s.complete),
            (A, 5, 2, false)
        );
        assert_eq!(s.genus, "$Codex_Ent_Bacterial_Genus_Name;");
        let now = pos(&exo(3)).unwrap();
        let expected = crate::exobiology::geo::haversine_m(10.0, 20.5, now.0, now.1, RADIUS);
        close(t.distance_to_previous(now, RADIUS).unwrap(), expected, 1.0);
        // A completed sampling is not open; another commander sees nothing.
        persist::save_sample(c(), row("Analyse", 20, 10.0, 21.0))(&store).unwrap();
        assert!(restored(&store).sampling().is_none());
        let other = persist::load_open_samples("CMDR OTHER".into())(&store).unwrap();
        assert_eq!(other["rows"], serde_json::json!([]));
    }

    #[test]
    fn replayed_scan_organic_uses_stored_position() {
        let mut t = SamplingTracker::default();
        t.restore(vec![row("Log", 0, 10.0, 20.0)]);
        t.reset_for_replay();
        assert!(t.sampling().is_none());
        let mut ctx = ModuleCtx::new("exobiology", true, 1);
        let status_pos = pos(&exo(3));
        let change = t.handle_scan(
            &scan(OrganicScanType::Log, A, 5),
            "CMDR TESTER",
            at(0),
            status_pos,
            &mut ctx,
        );
        assert_eq!(change, SamplingChange::Started);
        let p = &t.sampling().unwrap().points[0];
        assert_eq!((p.lat, p.lon, p.ts), (10.0, 20.0, at(0)));
        // Without a stored row the replayed scan has no position: Status is not replayed.
        let change = t.handle_scan(
            &scan(OrganicScanType::Sample, A, 5),
            "CMDR TESTER",
            at(10),
            status_pos,
            &mut ctx,
        );
        assert_eq!(change, SamplingChange::Progressed(2));
        assert_eq!(t.sampling().unwrap().points.len(), 1);
        assert_eq!(t.sampling().unwrap().scans, 2);
    }

    #[test]
    fn live_scan_organic_emits_save_sample_task_and_replay_does_not() {
        let mut t = SamplingTracker::default();
        let ev = scan(OrganicScanType::Log, A, 5);
        let mut live = ModuleCtx::new("exobiology", false, 1);
        t.handle_scan(&ev, "CMDR TESTER", at(0), Some((10.0, 20.0)), &mut live);
        let (_, tasks, _) = live.take_outputs();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].0, "save_sample");
        let store = open_store();
        let (_, task) = tasks.into_iter().next().unwrap();
        task(&store).unwrap();
        let rows = store
            .query_json("SELECT scan_type, lat, lon, ts FROM exo_sample", &[])
            .unwrap();
        assert_eq!(
            rows,
            vec![serde_json::json!([
                "Log",
                10.0,
                20.0,
                "2026-04-12T21:06:00.000000000Z"
            ])]
        );
        let mut replay = ModuleCtx::new("exobiology", true, 1);
        t.handle_scan(&ev, "CMDR TESTER", at(0), Some((10.0, 20.0)), &mut replay);
        assert_eq!(replay.take_outputs().1.len(), 0);
    }

    #[test]
    fn on_sold_clears_completed() {
        let mut t = SamplingTracker::default();
        for (species, body) in [(A, 5), (B, 6)] {
            t.on_scan_organic(&scan(OrganicScanType::Log, species, body), None, at(0));
            t.on_scan_organic(&scan(OrganicScanType::Analyse, species, body), None, at(5));
        }
        assert_eq!(t.completed().len(), 2);
        t.on_sold();
        assert!(t.completed().is_empty());
    }

    #[test]
    fn died_clears_sampling_tracker() {
        let mut t = SamplingTracker::default();
        t.on_scan_organic(&scan(OrganicScanType::Log, A, 5), None, at(0));
        t.on_scan_organic(&scan(OrganicScanType::Analyse, A, 5), None, at(5));
        t.on_scan_organic(&scan(OrganicScanType::Log, B, 5), None, at(9));
        assert!(t.sampling().is_some());
        assert_eq!(t.completed().len(), 1);
        t.died();
        assert!(t.sampling().is_none());
        assert!(t.completed().is_empty());
    }

    #[test]
    fn sold_keeps_incomplete_rows_and_died_removes_everything() {
        let store = open_store();
        let c = || "CMDR TESTER".to_string();
        for r in [row("Log", 0, 1.0, 1.0), row("Analyse", 5, 1.0, 1.0)] {
            persist::save_sample(c(), r)(&store).unwrap();
        }
        let mut open = row("Log", 30, 2.0, 2.0);
        open.species = B.into();
        persist::save_sample(c(), open)(&store).unwrap();
        persist::clear_completed(c())(&store).unwrap();
        let left = store
            .query_json("SELECT species FROM exo_sample", &[])
            .unwrap();
        assert_eq!(left, vec![serde_json::json!([B])]);
        persist::clear_samples(c())(&store).unwrap();
        assert!(
            store
                .query_json("SELECT 1 FROM exo_sample", &[])
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn fixture_session_drives_tracker_end_to_end() {
        let mut t = SamplingTracker::default();
        let mut ctx = ModuleCtx::new("exobiology", false, 1);
        let positions = [exo(1), exo(2), exo(3), exo(3)];
        let mut changes = Vec::new();
        let mut sold = false;
        let mut n = 0;
        let log = include_str!("../../../../tests/fixtures/journal/exobiology_session.log");
        for line in log.lines() {
            let parsed = tmc_model::events::parse_line(line).unwrap();
            match &parsed.kind {
                tmc_model::events::EventKind::ScanOrganic(ev) => {
                    let p = pos(&positions[n]);
                    n += 1;
                    changes.push(t.handle_scan(ev, "CMDR TESTER", parsed.timestamp, p, &mut ctx));
                }
                tmc_model::events::EventKind::SellOrganicData(_) => {
                    t.handle_sold("CMDR TESTER", &mut ctx);
                    sold = true;
                }
                _ => {}
            }
        }
        assert_eq!(
            changes,
            vec![
                SamplingChange::Started,
                SamplingChange::Progressed(2),
                SamplingChange::Completed,
                SamplingChange::Started,
            ]
        );
        assert!(sold && t.completed().is_empty());
        assert_eq!(t.sampling().unwrap().species, B);
    }
}
