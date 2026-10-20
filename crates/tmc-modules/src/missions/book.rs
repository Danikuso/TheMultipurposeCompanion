//! Libro de misiones: estado, clasificación y reconstrucción.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use tmc_core::StoreTask;
use tmc_model::events::{EventKind, MissionAccepted, parse_line, split_multi_destination};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionKind {
    Massacre,
    Sightseeing,
    Passenger,
    Delivery,
    Collect,
    Courier,
    Other,
}

/// Clasifica por prefijo del `Name` normalizado (sin `_name`, sin distinguir mayúsculas).
pub fn classify_kind(name: &str, passenger: bool) -> MissionKind {
    let mut n = name.trim().to_ascii_lowercase();
    if let Some(s) = n.strip_suffix("_name") {
        n = s.to_string();
    }
    let n = n.strip_prefix("mission_").unwrap_or(&n);
    let starts = |p: &str| n.starts_with(p);
    if starts("massacre") {
        MissionKind::Massacre
    } else if starts("sightseeing") {
        MissionKind::Sightseeing
    } else if starts("passenger") {
        MissionKind::Passenger
    } else if starts("delivery") {
        MissionKind::Delivery
    } else if starts("collect") || starts("mining") {
        MissionKind::Collect
    } else if starts("courier") {
        MissionKind::Courier
    } else if passenger {
        MissionKind::Passenger
    } else {
        MissionKind::Other
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MissionProgress {
    pub delivered: u32,
    pub collected: u32,
    pub total: Option<u32>,
    pub kills_needed: Option<u32>,
    pub passengers: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mission {
    pub id: u64,
    pub kind: MissionKind,
    pub name_localised: String,
    pub faction: Option<String>,
    pub destination_system: Option<String>,
    pub destination_station: Option<String>,
    pub stops: Vec<String>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub expiry: Option<DateTime<Utc>>,
    pub reward: Option<u64>,
    pub progress: MissionProgress,
    pub redirected: bool,
    pub details_missing: bool,
    pub target_faction: Option<String>,
    pub wing: bool,
    /// Mercancía de la entrega (localizada si existe); `None` si se desconoce.
    pub commodity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookChange {
    Added(u64),
    Completed(u64),
    Failed(u64),
    Abandoned(u64),
    Redirected {
        id: u64,
        system: String,
        station: Option<String>,
    },
    Progress(u64),
}

#[derive(Debug, Clone, Default)]
pub struct MissionBook {
    missions: BTreeMap<u64, Mission>,
}

fn from_accepted(m: &MissionAccepted, at: Option<DateTime<Utc>>) -> Mission {
    let stops = m
        .destination_system
        .as_deref()
        .map(split_multi_destination)
        .unwrap_or_default();
    let kind = classify_kind(&m.name, m.passenger_count.is_some());
    let multi = stops.len() > 1 || kind == MissionKind::Sightseeing;
    Mission {
        id: m.mission_id,
        kind,
        name_localised: m.localised_name.clone().unwrap_or_else(|| m.name.clone()),
        faction: m.faction.clone(),
        destination_system: if multi { None } else { stops.first().cloned() },
        destination_station: m.destination_station.clone(),
        stops: if multi { stops } else { Vec::new() },
        accepted_at: at,
        expiry: m.expiry,
        reward: m.reward,
        progress: MissionProgress {
            total: m.count,
            kills_needed: m.kill_count,
            passengers: m.passenger_count,
            ..Default::default()
        },
        redirected: false,
        details_missing: false,
        target_faction: m.target_faction.clone(),
        wing: m.wing.unwrap_or(false),
        commodity: m
            .commodity_localised
            .clone()
            .or_else(|| m.commodity.clone()),
    }
}

impl MissionBook {
    pub fn active(&self) -> impl Iterator<Item = &Mission> {
        self.missions.values()
    }

    pub fn get(&self, id: u64) -> Option<&Mission> {
        self.missions.get(&id)
    }

    pub fn apply_event(&mut self, ev: &EventKind, at: DateTime<Utc>) -> Vec<BookChange> {
        let mut out = Vec::new();
        match ev {
            EventKind::MissionAccepted(m) => {
                self.missions
                    .insert(m.mission_id, from_accepted(m, Some(at)));
                out.push(BookChange::Added(m.mission_id));
            }
            EventKind::Missions(list) => {
                // Las conocidas que aparecen como falladas/completadas generan aviso;
                // las desconocidas (arranque) no. Las conocidas que no figuran en
                // ninguna lista se descartan en silencio: la lista activa manda.
                for (refs, failed) in [(&list.failed, true), (&list.complete, false)] {
                    for r in refs {
                        if !list.active.iter().any(|a| a.mission_id == r.mission_id)
                            && self.missions.remove(&r.mission_id).is_some()
                        {
                            out.push(if failed {
                                BookChange::Failed(r.mission_id)
                            } else {
                                BookChange::Completed(r.mission_id)
                            });
                        }
                    }
                }
                let mut next = BTreeMap::new();
                for r in &list.active {
                    let m = self.missions.remove(&r.mission_id).unwrap_or_else(|| {
                        let kind = classify_kind(&r.name, r.passenger_mission);
                        Mission {
                            id: r.mission_id,
                            kind,
                            // Nombre legible: nunca el interno `Mission_X_name`.
                            name_localised: format!("{kind:?}"),
                            faction: None,
                            destination_system: None,
                            destination_station: None,
                            stops: Vec::new(),
                            accepted_at: None,
                            expiry: r
                                .expires
                                .filter(|s| *s > 0)
                                .map(|s| at + Duration::seconds(s)),
                            reward: None,
                            progress: MissionProgress::default(),
                            redirected: false,
                            details_missing: true,
                            target_faction: None,
                            wing: false,
                            commodity: None,
                        }
                    });
                    next.insert(r.mission_id, m);
                }
                decide!("missions.rebuild", {
                    "active": next.len(),
                    "missing": next.values().filter(|m| m.details_missing).count(),
                });
                self.missions = next;
            }
            EventKind::MissionCompleted(m) => {
                if self.missions.remove(&m.mission_id).is_some() {
                    out.push(BookChange::Completed(m.mission_id));
                }
            }
            EventKind::MissionFailed(m) => {
                if self.missions.remove(&m.mission_id).is_some() {
                    out.push(BookChange::Failed(m.mission_id));
                }
            }
            EventKind::MissionAbandoned(m) => {
                if self.missions.remove(&m.mission_id).is_some() {
                    out.push(BookChange::Abandoned(m.mission_id));
                }
            }
            EventKind::MissionRedirected(r) => {
                if let Some(m) = self.missions.get_mut(&r.mission_id)
                    && (r.new_destination_system.is_some() || r.new_destination_station.is_some())
                {
                    if let Some(sys) = r.new_destination_system.clone() {
                        // Vuelta del turismo: un único destino nuevo.
                        m.destination_system = Some(sys);
                        m.stops.clear();
                    }
                    m.destination_station = r.new_destination_station.clone();
                    m.redirected = true;
                    out.push(BookChange::Redirected {
                        id: r.mission_id,
                        system: m.destination_system.clone().unwrap_or_default(),
                        station: m.destination_station.clone(),
                    });
                }
            }
            EventKind::CargoDepot(c) => {
                if let Some(m) = self.missions.get_mut(&c.mission_id) {
                    m.progress.delivered = c.items_delivered;
                    m.progress.collected = c.items_collected;
                    if c.total_items_to_deliver > 0 {
                        m.progress.total = Some(c.total_items_to_deliver);
                    }
                    out.push(BookChange::Progress(c.mission_id));
                }
            }
            _ => {}
        }
        out
    }

    /// Vacía el libro (cambio de comandante).
    pub fn clear(&mut self) {
        self.missions.clear();
    }

    /// Aplica el resultado de [`accepted_rebuild_task`] solo si es del comandante dado.
    pub fn on_history_result(&mut self, cmdr: &str, result: &Value) {
        if result["cmdr"].as_str() == Some(cmdr) {
            self.on_history_accepted(accepted_from_result(result));
        }
    }

    /// Completa con el historial las misiones activas que quedaron sin detalles.
    /// Las que no están activas se ignoran: la lista de `Missions` manda.
    pub fn on_history_accepted(&mut self, accepted: Vec<(MissionAccepted, DateTime<Utc>)>) {
        for (acc, at) in accepted {
            let Some(cur) = self.missions.get_mut(&acc.mission_id) else {
                continue;
            };
            if !cur.details_missing {
                continue;
            }
            let mut full = from_accepted(&acc, Some(at));
            full.expiry = cur.expiry.or(full.expiry);
            full.progress.delivered = cur.progress.delivered;
            full.progress.collected = cur.progress.collected;
            *cur = full;
        }
    }
}

/// Lectura (`missions.accepted`) de los `MissionAccepted` de los ids dados.
pub fn accepted_rebuild_task(cmdr: String, ids: Vec<u64>) -> StoreTask {
    Box::new(move |store| {
        let list = ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT e.ts, e.json FROM event e WHERE e.cmdr = ?1 AND e.name = 'MissionAccepted' \
             AND json_extract(e.json, '$.MissionID') IN ({list}) ORDER BY e.ts ASC, e.id ASC"
        );
        let rows = store.query_json(&sql, &[json!(cmdr)])?;
        let acc: Vec<Value> = rows
            .iter()
            .map(|r| json!({"ts": r[0], "json": r[1]}))
            .collect();
        Ok(json!({"cmdr": cmdr, "accepted": acc}))
    })
}

/// Interpreta el resultado de [`accepted_rebuild_task`]; ignora filas ilegibles.
pub fn accepted_from_result(result: &Value) -> Vec<(MissionAccepted, DateTime<Utc>)> {
    result["accepted"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            let ev = parse_line(r["json"].as_str()?).ok()?;
            match ev.kind {
                EventKind::MissionAccepted(m) => Some((*m, ev.timestamp)),
                _ => None,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tmc_model::events::{
        CargoDepot, MissionCompleted, MissionEnded, MissionRedirected, MissionRef, MissionsList,
    };

    fn t(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 20, h, 0, 0).unwrap()
    }
    fn accepted(id: u64, name: &str) -> MissionAccepted {
        MissionAccepted {
            mission_id: id,
            name: name.into(),
            localised_name: Some("Test".into()),
            reward: Some(1234),
            ..Default::default()
        }
    }
    fn accept(b: &mut MissionBook, a: MissionAccepted) {
        b.apply_event(&EventKind::MissionAccepted(Box::new(a)), t(12));
    }
    fn missions_ev(ids: &[u64]) -> EventKind {
        EventKind::Missions(Box::new(MissionsList {
            active: ids
                .iter()
                .map(|&id| MissionRef {
                    mission_id: id,
                    name: "Mission_Delivery_name".into(),
                    expires: Some(3600),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }))
    }

    #[test]
    fn accepted_then_completed_lifecycle() {
        let mut b = MissionBook::default();
        let c = b.apply_event(
            &EventKind::MissionAccepted(Box::new(accepted(65000001, "Mission_Delivery"))),
            t(12),
        );
        assert_eq!(c, vec![BookChange::Added(65000001)]);
        assert_eq!(b.active().count(), 1);
        let c = b.apply_event(
            &EventKind::MissionCompleted(Box::new(MissionCompleted {
                mission_id: 65000001,
                ..Default::default()
            })),
            t(13),
        );
        assert_eq!(c, vec![BookChange::Completed(65000001)]);
        assert_eq!(b.active().count(), 0);
        let c = b.apply_event(
            &EventKind::MissionFailed(MissionEnded {
                mission_id: 5,
                ..Default::default()
            }),
            t(13),
        );
        assert!(c.is_empty());
    }

    #[test]
    fn missions_event_keeps_unknown_mission_minimal() {
        let mut b = MissionBook::default();
        b.apply_event(&missions_ev(&[65000009]), t(12));
        let m = b.get(65000009).unwrap();
        assert!(m.details_missing);
        assert_eq!(m.expiry, Some(t(13)));
        assert_eq!(m.kind, MissionKind::Delivery);
    }

    #[test]
    fn accepted_not_in_missions_event_is_dropped() {
        let mut b = MissionBook::default();
        b.apply_event(&missions_ev(&[65000001]), t(12));
        b.on_history_accepted(vec![(accepted(65000002, "Mission_Delivery"), t(10))]);
        assert!(b.get(65000002).is_none());
        assert_eq!(b.active().count(), 1);
        accept(&mut b, accepted(7, "Mission_Delivery"));
        b.apply_event(&missions_ev(&[65000001]), t(12));
        assert!(b.get(7).is_none());
    }

    #[test]
    fn history_details_fill_minimal_entries() {
        let mut b = MissionBook::default();
        b.apply_event(&missions_ev(&[65000009]), t(12));
        b.on_history_accepted(vec![(accepted(65000009, "Mission_Delivery"), t(9))]);
        let m = b.get(65000009).unwrap();
        assert!(!m.details_missing);
        assert_eq!(m.reward, Some(1234));
        assert_eq!(m.expiry, Some(t(13)));
        assert_eq!(m.accepted_at, Some(t(9)));
    }

    #[test]
    fn redirected_updates_destination() {
        let mut b = MissionBook::default();
        accept(&mut b, accepted(5, "Mission_Delivery"));
        let c = b.apply_event(
            &EventKind::MissionRedirected(MissionRedirected {
                mission_id: 5,
                new_destination_system: Some("Lave".into()),
                ..Default::default()
            }),
            t(12),
        );
        let m = b.get(5).unwrap();
        assert_eq!(m.destination_system.as_deref(), Some("Lave"));
        assert!(m.redirected);
        assert!(matches!(c[0], BookChange::Redirected { id: 5, .. }));
    }

    #[test]
    fn cargo_depot_updates_progress() {
        let mut b = MissionBook::default();
        accept(&mut b, accepted(5, "Mission_Delivery"));
        let c = b.apply_event(
            &EventKind::CargoDepot(CargoDepot {
                mission_id: 5,
                update_type: "Deliver".into(),
                items_delivered: 30,
                total_items_to_deliver: 60,
                ..Default::default()
            }),
            t(12),
        );
        assert_eq!(c, vec![BookChange::Progress(5)]);
        let m = b.get(5).unwrap();
        assert_eq!(m.progress.delivered, 30);
        assert_eq!(m.progress.total, Some(60));
    }

    #[test]
    fn sightseeing_stops_from_destination() {
        let mut b = MissionBook::default();
        let mut a = accepted(5, "Mission_Sightseeing");
        a.destination_system = Some(
            "Alfa$MISSIONUTIL_MULTIPLE_INTERMEDIATE_SEPARATOR;Beta$MISSIONUTIL_MULTIPLE_FINAL_SEPARATOR;Gamma"
                .into(),
        );
        accept(&mut b, a);
        let m = b.get(5).unwrap();
        assert_eq!(m.stops.len(), 3);
        assert_eq!(m.kind, MissionKind::Sightseeing);
    }

    fn end(id: u64) -> MissionEnded {
        MissionEnded {
            mission_id: id,
            ..Default::default()
        }
    }
    fn mref(id: u64) -> MissionRef {
        MissionRef {
            mission_id: id,
            ..Default::default()
        }
    }

    #[test]
    fn failed_and_abandoned_known_ids() {
        let mut b = MissionBook::default();
        accept(&mut b, accepted(1, "Mission_Delivery"));
        accept(&mut b, accepted(2, "Mission_Delivery"));
        assert_eq!(
            b.apply_event(&EventKind::MissionFailed(end(1)), t(12)),
            vec![BookChange::Failed(1)]
        );
        assert_eq!(
            b.apply_event(&EventKind::MissionAbandoned(end(2)), t(12)),
            vec![BookChange::Abandoned(2)]
        );
    }

    #[test]
    fn missions_event_reconciles_failed_complete_and_keeps_details() {
        let mut b = MissionBook::default();
        for id in [1, 2, 3, 4] {
            accept(&mut b, accepted(id, "Mission_Delivery"));
        }
        let ev = EventKind::Missions(Box::new(MissionsList {
            active: vec![mref(1)],
            failed: vec![mref(2), mref(99)],
            complete: vec![mref(3)],
        }));
        let c = b.apply_event(&ev, t(12));
        assert_eq!(c, vec![BookChange::Failed(2), BookChange::Completed(3)]);
        assert!(!b.get(1).unwrap().details_missing);
        assert_eq!(b.get(1).unwrap().reward, Some(1234));
        assert!(b.get(4).is_none());
    }

    #[test]
    fn expires_non_positive_means_no_expiry_and_name_is_readable() {
        let mut b = MissionBook::default();
        let ev = EventKind::Missions(Box::new(MissionsList {
            active: vec![MissionRef {
                mission_id: 1,
                name: "Mission_Massacre_name".into(),
                expires: Some(0),
                ..Default::default()
            }],
            ..Default::default()
        }));
        b.apply_event(&ev, t(12));
        let m = b.get(1).unwrap();
        assert_eq!(m.expiry, None);
        assert_eq!(m.name_localised, "Massacre");
    }

    #[test]
    fn redirected_without_system_updates_station_and_multistop_collapses() {
        let mut b = MissionBook::default();
        accept(&mut b, accepted(5, "Mission_Delivery"));
        b.apply_event(
            &EventKind::MissionRedirected(MissionRedirected {
                mission_id: 5,
                new_destination_station: Some("Port".into()),
                ..Default::default()
            }),
            t(12),
        );
        assert_eq!(
            b.get(5).unwrap().destination_station.as_deref(),
            Some("Port")
        );
        let mut a = accepted(6, "Mission_Sightseeing");
        a.destination_system = Some("A$MISSIONUTIL_MULTIPLE_FINAL_SEPARATOR;B".into());
        accept(&mut b, a);
        assert_eq!(b.get(6).unwrap().stops.len(), 2);
        b.apply_event(
            &EventKind::MissionRedirected(MissionRedirected {
                mission_id: 6,
                new_destination_system: Some("Home".into()),
                ..Default::default()
            }),
            t(12),
        );
        let m = b.get(6).unwrap();
        assert!(m.stops.is_empty());
        assert_eq!(m.destination_system.as_deref(), Some("Home"));
    }

    #[test]
    fn duplicate_history_first_wins_and_cmdr_mismatch_ignored() {
        let mut b = MissionBook::default();
        b.apply_event(&missions_ev(&[9]), t(12));
        let mut first = accepted(9, "Mission_Delivery");
        first.reward = Some(1);
        let mut second = accepted(9, "Mission_Delivery");
        second.reward = Some(2);
        b.on_history_accepted(vec![(first, t(8)), (second, t(9))]);
        assert_eq!(b.get(9).unwrap().reward, Some(1));
        let mut b2 = MissionBook::default();
        b2.apply_event(&missions_ev(&[9]), t(12));
        b2.on_history_result("OTHER", &json!({"cmdr":"TESTER","accepted":[]}));
        assert!(b2.get(9).unwrap().details_missing);
        b2.clear();
        assert_eq!(b2.active().count(), 0);
    }

    #[test]
    fn classify_normalizes_suffix_and_case() {
        assert_eq!(
            classify_kind("Mission_Massacre_name", false),
            MissionKind::Massacre
        );
        assert_eq!(
            classify_kind("MISSION_PassengerVIP", false),
            MissionKind::Passenger
        );
        assert_eq!(classify_kind("Mission_Hack", false), MissionKind::Other);
    }

    #[test]
    fn rebuild_task_reads_accepted_from_store() {
        use tmc_core::{EventWriter, Store, StoredEvent};
        let store = Store::open_in_memory().unwrap();
        let writer = EventWriter::spawn(&store);
        for (i, id) in [65000009u64, 65000002, 65000003].into_iter().enumerate() {
            let raw = json!({"timestamp":"2026-10-20T09:00:00Z","event":"MissionAccepted",
                "MissionID":id,"Name":"Mission_Delivery","Reward":1234})
            .to_string();
            let ev = parse_line(&raw).unwrap();
            writer.send(StoredEvent {
                file_name: "m.log".into(),
                line: i as u64 + 1,
                ts: ev.timestamp,
                name: ev.name,
                cmdr: Some("TESTER".into()),
                json: raw,
            });
        }
        writer.flush_blocking().unwrap();
        let r = accepted_rebuild_task("TESTER".into(), vec![65000009])(&store).unwrap();
        let acc = accepted_from_result(&r);
        assert_eq!(acc.len(), 1);
        assert_eq!(acc[0].0.mission_id, 65000009);
        let mut b = MissionBook::default();
        b.apply_event(&missions_ev(&[65000009]), t(12));
        b.on_history_accepted(acc);
        assert!(!b.get(65000009).unwrap().details_missing);
    }
}
