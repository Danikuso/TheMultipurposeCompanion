//! Estado transitorio del juego que EDDN, EDSM e Inara exigen en sus cabeceras y cargas.

use std::collections::BTreeMap;
use tmc_model::companion::{CompanionFile, CompanionKind};
use tmc_model::events::{EventKind, JournalEvent, LocationInfo, Rank, ShipLockerContents};
use tmc_model::state::Galaxy;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackedCargoItem {
    pub name: String,
    pub count: u32,
    pub is_stolen: bool,
    pub mission_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackedLockerItem {
    pub name: String,
    pub count: u32,
    pub item_type: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TransientSnapshot {
    pub gameversion: Option<String>,
    pub gamebuild: Option<String>,
    pub horizons: Option<bool>,
    pub odyssey: Option<bool>,
    pub cmdr_fid: Option<String>,
    pub cmdr_name: Option<String>,
    pub system_name: Option<String>,
    pub system_address: Option<u64>,
    pub star_pos: Option<[f64; 3]>,
    pub body_name: Option<String>,
    pub body_coords: Option<(f64, f64)>,
    pub market_id: Option<u64>,
    pub station_name: Option<String>,
    pub ship_id: Option<u64>,
    pub ship_type: Option<String>,
    pub ship_name: Option<String>,
    pub ship_ident: Option<String>,
    pub ship_loadout: Option<serde_json::Value>,
    pub ranks: Option<Rank>,
    pub rank_progress: Option<Rank>,
    pub credits: Option<(u64, Option<u64>)>,
    pub engineers: Option<Vec<(String, String, Option<u8>)>>,
    pub powerplay: Option<(String, u32, u32)>,
    pub major_reputation: Option<Vec<(String, f64)>>,
    pub materials: Option<BTreeMap<String, u32>>,
    pub cargo: Option<Vec<TrackedCargoItem>>,
    pub ship_locker: Option<Vec<TrackedLockerItem>>,
    pub galaxy: Galaxy,
    pub is_beta: bool,
}

#[derive(Debug, Default)]
pub struct TransientTracker {
    state: TransientSnapshot,
}

fn raw_bool(ev: &JournalEvent, key: &str) -> Option<bool> {
    serde_json::from_str::<serde_json::Value>(ev.raw.get())
        .ok()?
        .get(key)?
        .as_bool()
}

impl TransientTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&mut self, ev: &JournalEvent) {
        match &ev.kind {
            EventKind::Fileheader(h) => {
                self.set_version(h.gameversion.as_deref(), h.build.as_deref());
            }
            EventKind::Commander(c) => {
                if let Some(fid) = &c.fid {
                    self.switch_cmdr(fid);
                }
                if c.name.is_some() {
                    self.state.cmdr_name = c.name.clone();
                }
            }
            EventKind::LoadGame(l) => {
                if let Some(fid) = &l.fid {
                    self.switch_cmdr(fid);
                }
                if l.commander.is_some() {
                    self.state.cmdr_name = l.commander.clone();
                }
                self.set_version(l.gameversion.as_deref(), l.build.as_deref());
                if l.odyssey.is_some() {
                    self.state.odyssey = l.odyssey;
                }
                if let Some(h) = raw_bool(ev, "Horizons") {
                    self.state.horizons = Some(h);
                }
                self.state.ship_id = l.ship_id;
                if l.ship.is_some() {
                    self.state.ship_type = l.ship.clone();
                }
                if l.ship_name.is_some() {
                    self.state.ship_name = l.ship_name.clone();
                }
                if l.ship_ident.is_some() {
                    self.state.ship_ident = l.ship_ident.clone();
                }
                if let Some(c) = l.credits {
                    self.state.credits = Some((c, l.loan));
                }
                // Una carga nueva parte sin estación ni cuerpo hasta Location/Docked.
                self.state.market_id = None;
                self.state.station_name = None;
                self.state.body_name = None;
                self.state.body_coords = None;
            }
            EventKind::Rank(r) => self.state.ranks = Some(r.clone()),
            EventKind::Progress(p) => self.state.rank_progress = Some(p.clone()),
            EventKind::EngineerProgress(ep) => {
                let list: Vec<(String, String, Option<u8>)> = ep
                    .entries()
                    .into_iter()
                    .filter_map(|e| {
                        let st = match e.progress {
                            tmc_model::events::EngineerStatus::Invited => "Invited",
                            tmc_model::events::EngineerStatus::Acquainted => "Acquainted",
                            tmc_model::events::EngineerStatus::Unlocked => "Unlocked",
                            tmc_model::events::EngineerStatus::Barred => "Barred",
                            _ => return None,
                        };
                        Some((e.engineer, st.to_string(), e.rank))
                    })
                    .collect();
                if !list.is_empty() {
                    self.state.engineers = Some(list);
                }
            }
            EventKind::Powerplay(pp) if !pp.power.trim().is_empty() => {
                self.state.powerplay = Some((pp.power.clone(), pp.rank, pp.merits));
            }
            EventKind::Other if ev.name == "Reputation" => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(ev.raw.get()) {
                    let reps: Vec<(String, f64)> = [
                        ("Empire", "empire"),
                        ("Federation", "federation"),
                        ("Alliance", "alliance"),
                        ("Independent", "independent"),
                    ]
                    .into_iter()
                    .filter_map(|(jk, ik)| {
                        v.get(jk)
                            .and_then(serde_json::Value::as_f64)
                            .map(|val| (ik.to_string(), val / 100.0))
                    })
                    .collect();
                    if !reps.is_empty() {
                        self.state.major_reputation = Some(reps);
                    }
                }
            }
            EventKind::Location(i) | EventKind::CarrierJump(i) => {
                self.set_location(i, true);
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(ev.raw.get())
                    && let (Some(lat), Some(lon)) = (
                        v.get("Latitude").and_then(serde_json::Value::as_f64),
                        v.get("Longitude").and_then(serde_json::Value::as_f64),
                    )
                {
                    self.state.body_coords = Some((lat, lon));
                }
            }
            EventKind::FsdJump(i) => self.set_location(i, false),
            EventKind::Docked(d) => {
                self.state.market_id = d.market_id;
                self.state.station_name = d.station_name.clone();
                if let Some(name) = &d.star_system {
                    self.state.system_name = Some(name.clone());
                }
                if d.system_address.is_some() {
                    self.state.system_address = d.system_address;
                }
            }
            EventKind::Undocked(_) => {
                self.state.market_id = None;
                self.state.station_name = None;
            }
            EventKind::ApproachBody(b) => {
                if b.body.is_some() {
                    self.state.body_name = b.body.clone();
                }
                self.state.body_coords = None;
            }
            EventKind::LeaveBody(_) | EventKind::SupercruiseEntry(_) => {
                self.state.body_name = None;
                self.state.body_coords = None;
            }
            EventKind::Touchdown(s) | EventKind::Liftoff(s) => {
                if s.body.is_some() {
                    self.state.body_name = s.body.clone();
                }
                if let (Some(lat), Some(lon)) = (s.latitude, s.longitude) {
                    self.state.body_coords = Some((lat, lon));
                }
            }
            EventKind::ShipyardSwap(s) => {
                if s.ship_id.is_some() {
                    self.state.ship_id = s.ship_id;
                    self.state.ship_name = None;
                    self.state.ship_ident = None;
                    self.state.ship_loadout = None;
                }
                if s.ship_type.is_some() {
                    self.state.ship_type = s.ship_type.clone();
                }
            }
            EventKind::Loadout(l) => {
                self.state.ship_id = l.ship_id;
                if l.ship.is_some() {
                    self.state.ship_type = l.ship.clone();
                }
                if l.ship_name.is_some() {
                    self.state.ship_name = l.ship_name.clone();
                }
                if l.ship_ident.is_some() {
                    self.state.ship_ident = l.ship_ident.clone();
                }
                if let (Some(ship_type), Some(ship_id)) = (
                    l.ship.as_deref().map(str::trim).filter(|s| !s.is_empty()),
                    l.ship_id,
                ) {
                    let raw = serde_json::from_str::<serde_json::Value>(ev.raw.get()).ok();
                    self.state.ship_loadout = Some(crate::inara::ship::build_loadout_data(
                        ship_type,
                        ship_id,
                        l,
                        raw.as_ref(),
                    ));
                }
            }
            EventKind::Other if ev.name == "SetUserShipName" => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(ev.raw.get()) {
                    if let Some(n) = v.get("UserShipName").and_then(serde_json::Value::as_str) {
                        self.state.ship_name = Some(n.to_string());
                    }
                    if let Some(i) = v.get("UserShipId").and_then(serde_json::Value::as_str) {
                        self.state.ship_ident = Some(i.to_string());
                    }
                }
            }
            EventKind::Materials(m) => {
                let mut map = BTreeMap::new();
                for item in m
                    .raw
                    .iter()
                    .chain(m.manufactured.iter())
                    .chain(m.encoded.iter())
                {
                    let key = item.name.trim().to_ascii_lowercase();
                    if !key.is_empty() && item.count > 0 {
                        *map.entry(key).or_insert(0) += item.count;
                    }
                }
                self.state.materials = Some(map);
            }
            EventKind::MaterialCollected(mc) => {
                self.adjust_material(&mc.name, mc.count as i64);
            }
            EventKind::MaterialDiscarded(md) => {
                self.adjust_material(&md.name, -(md.count as i64));
            }
            EventKind::MaterialTrade(mt) => {
                self.adjust_material(&mt.paid.material, -(mt.paid.quantity as i64));
                self.adjust_material(&mt.received.material, mt.received.quantity as i64);
            }
            EventKind::Synthesis(sy) => {
                for u in &sy.materials {
                    self.adjust_material(&u.name, -(u.count as i64));
                }
            }
            EventKind::EngineerCraft(ec) => {
                for u in &ec.ingredients {
                    self.adjust_material(&u.name, -(u.count as i64));
                }
            }
            EventKind::EngineerContribution(ec) => {
                if let Some(mat) = &ec.material {
                    self.adjust_material(mat, -(ec.quantity as i64));
                }
            }
            EventKind::TechnologyBroker(tb) => {
                for u in &tb.materials {
                    self.adjust_material(&u.name, -(u.count as i64));
                }
            }
            EventKind::MissionCompleted(mc) => {
                for r in &mc.materials_reward {
                    self.adjust_material(&r.name, r.count as i64);
                }
            }
            EventKind::Cargo(c) => {
                if let Some(inv) = &c.inventory
                    && !c
                        .vessel
                        .as_deref()
                        .is_some_and(|v| v.eq_ignore_ascii_case("SRV"))
                {
                    self.state.cargo = Some(
                        inv.iter()
                            .filter(|i| i.count > 0)
                            .map(|i| TrackedCargoItem {
                                name: i.name.clone(),
                                count: i.count,
                                is_stolen: i.stolen_count() > 0,
                                mission_id: i.mission_id,
                            })
                            .collect(),
                    );
                }
            }
            EventKind::ShipLocker(sl) if !sl.is_stub() => {
                self.state.ship_locker = Some(locker_items_from_contents(sl));
            }
            _ => {}
        }
    }

    pub fn snapshot(&self) -> TransientSnapshot {
        self.state.clone()
    }

    /// Galaxia del archivo según su `gameversion` (sin copiar la instantánea completa).
    pub fn galaxy(&self) -> Galaxy {
        self.state.galaxy
    }

    /// Descarta todo el estado al empezar a leer otro archivo de diario.
    pub fn reset_for_new_file(&mut self) {
        self.state = TransientSnapshot::default();
    }

    /// Actualiza los inventarios completos desde un archivo compañero (`Cargo.json`, `ShipLocker.json`).
    pub fn apply_companion(&mut self, comp: &CompanionFile) {
        match comp {
            CompanionFile::Cargo(cf) => {
                self.state.cargo = Some(
                    cf.inventory
                        .iter()
                        .filter(|i| i.count > 0)
                        .map(|i| TrackedCargoItem {
                            name: i.name.clone(),
                            count: i.count,
                            is_stolen: i.stolen > 0,
                            mission_id: i.mission_id,
                        })
                        .collect(),
                );
            }
            CompanionFile::Raw(CompanionKind::ShipLocker, val) => {
                if let Some(sl) = ShipLockerContents::from_value(val)
                    && !sl.is_stub()
                {
                    self.state.ship_locker = Some(locker_items_from_contents(&sl));
                }
            }
            _ => {}
        }
    }

    pub fn set_ship_loadout(&mut self, loadout: serde_json::Value) {
        self.state.ship_loadout = Some(loadout);
    }

    fn adjust_material(&mut self, name: &str, delta: i64) {
        let Some(ref mut map) = self.state.materials else {
            return;
        };
        let key = name.trim().to_ascii_lowercase();
        if key.is_empty() {
            return;
        }
        if delta >= 0 {
            let entry = map.entry(key).or_insert(0);
            *entry = entry.saturating_add(delta as u32);
        } else if let Some(cur) = map.get_mut(&key) {
            let next = cur.saturating_sub(delta.unsigned_abs() as u32);
            if next == 0 {
                map.remove(&key);
            } else {
                *cur = next;
            }
        }
    }

    /// Fija el comandante; si es otro, olvida el estado del anterior.
    fn switch_cmdr(&mut self, fid: &str) {
        if self.state.cmdr_fid.as_deref() != Some(fid) {
            self.state.system_name = None;
            self.state.system_address = None;
            self.state.star_pos = None;
            self.state.body_name = None;
            self.state.body_coords = None;
            self.state.market_id = None;
            self.state.station_name = None;
            self.state.ship_id = None;
            self.state.ship_type = None;
            self.state.ship_name = None;
            self.state.ship_ident = None;
            self.state.ship_loadout = None;
            self.state.ranks = None;
            self.state.rank_progress = None;
            self.state.credits = None;
            self.state.engineers = None;
            self.state.powerplay = None;
            self.state.major_reputation = None;
            self.state.materials = None;
            self.state.cargo = None;
            self.state.ship_locker = None;
            self.state.cmdr_fid = Some(fid.to_string());
        }
    }

    fn set_version(&mut self, gameversion: Option<&str>, build: Option<&str>) {
        if let Some(v) = gameversion {
            self.state.galaxy = Galaxy::from_game_version(v);
            let lower = v.to_ascii_lowercase();
            self.state.is_beta = lower.contains("beta") || lower.contains("alpha");
            self.state.gameversion = Some(v.to_string());
        }
        if let Some(b) = build {
            self.state.gamebuild = Some(b.to_string());
        }
    }

    fn set_location(&mut self, i: &LocationInfo, keep_station: bool) {
        self.state.system_name = Some(i.star_system.clone());
        self.state.system_address = Some(i.system_address);
        self.state.star_pos = Some(i.star_pos);
        self.state.body_name = i.body.clone();
        self.state.body_coords = None;
        if keep_station && i.docked {
            self.state.market_id = i.market_id;
            self.state.station_name = i.station_name.clone();
        } else {
            self.state.market_id = None;
            self.state.station_name = None;
        }
    }
}

pub(crate) fn locker_items_from_contents(sl: &ShipLockerContents) -> Vec<TrackedLockerItem> {
    let mut items = Vec::new();
    for (list, item_type) in [
        (&sl.items, "Item"),
        (&sl.components, "Component"),
        (&sl.data, "Data"),
        (&sl.consumables, "Consumable"),
    ] {
        for item in list {
            if !item.name.trim().is_empty() && item.count > 0 {
                items.push(TrackedLockerItem {
                    name: item.name.clone(),
                    count: item.count,
                    item_type,
                });
            }
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::support::{core_fixture, ev};
    use tmc_model::state::Galaxy;

    #[test]
    fn tracker_sequence_fileheader_loadgame_location_fsdjump_docked() {
        let mut t = TransientTracker::new();
        t.apply(&ev(r#"{"timestamp":"2026-03-09T09:00:01Z","event":"Fileheader","part":1,"Odyssey":true,"gameversion":"4.0.0.1903","build":"r310123/r0 "}"#));
        t.apply(&ev(r#"{"timestamp":"2026-03-09T09:00:12Z","event":"LoadGame","FID":"F0000001","Commander":"CMDR TESTER","Horizons":true,"Odyssey":true,"Ship":"python","ShipID":7,"gameversion":"4.0.0.1903","build":"r310123/r0 "}"#));
        t.apply(&ev(r#"{"timestamp":"2026-03-09T09:01:00Z","event":"Location","StarSystem":"Sol","SystemAddress":10477373803,"StarPos":[0,0,0],"Docked":false}"#));
        let s = t.snapshot();
        assert_eq!(s.gameversion.as_deref(), Some("4.0.0.1903"));
        assert_eq!(s.gamebuild.as_deref(), Some("r310123/r0 "));
        assert_eq!(s.odyssey, Some(true));
        assert_eq!(s.horizons, Some(true));
        assert_eq!(s.cmdr_fid.as_deref(), Some("F0000001"));
        assert_eq!(s.cmdr_name.as_deref(), Some("CMDR TESTER"));
        assert_eq!(s.system_name.as_deref(), Some("Sol"));
        assert_eq!(s.system_address, Some(10477373803));
        assert_eq!(s.star_pos, Some([0.0, 0.0, 0.0]));
        assert_eq!(s.ship_id, Some(7));
        assert_eq!(s.galaxy, Galaxy::Live);
        assert!(!s.is_beta);

        t.apply(&ev(r#"{"timestamp":"2026-03-09T09:05:00Z","event":"FSDJump","StarSystem":"Achenar","SystemAddress":2,"StarPos":[67.5,-119.4,24.8]}"#));
        t.apply(&ev(r#"{"timestamp":"2026-03-09T09:06:00Z","event":"Docked","StationName":"Dawes Hub","MarketID":128000001,"StarSystem":"Achenar","SystemAddress":2}"#));
        let s = t.snapshot();
        assert_eq!(s.system_name.as_deref(), Some("Achenar"));
        assert_eq!(s.star_pos, Some([67.5, -119.4, 24.8]));
        assert_eq!(s.market_id, Some(128000001));
        assert_eq!(s.station_name.as_deref(), Some("Dawes Hub"));

        t.apply(&ev(
            r#"{"timestamp":"2026-03-09T09:07:00Z","event":"Undocked","StationName":"Dawes Hub"}"#,
        ));
        let s = t.snapshot();
        assert_eq!(s.market_id, None);
        assert_eq!(s.station_name, None);
    }

    #[test]
    fn beta_gameversion_flags_is_beta() {
        let mut t = TransientTracker::new();
        t.apply(&ev(r#"{"timestamp":"2026-03-09T09:00:01Z","event":"Fileheader","part":1,"gameversion":"4.1.0.100 Beta","build":"r1/r0 "}"#));
        assert!(t.snapshot().is_beta);
    }

    #[test]
    fn second_loadgame_switches_cmdr() {
        let text = std::fs::read_to_string(core_fixture("two_cmdrs.01.log")).unwrap();
        let mut t = TransientTracker::new();
        let mut fids = Vec::new();
        for line in text.lines() {
            t.apply(&ev(line));
            fids.push(t.snapshot().cmdr_fid);
        }
        assert_eq!(fids[2].as_deref(), Some("F0000001"));
        assert_eq!(fids[5].as_deref(), Some("F0000002"));
        assert_eq!(t.snapshot().system_name.as_deref(), Some("Testf"));
    }

    #[test]
    fn reset_for_new_file_clears_everything() {
        let mut t = TransientTracker::new();
        t.apply(&ev(r#"{"timestamp":"2026-03-09T09:00:12Z","event":"LoadGame","FID":"F0000001","Commander":"C","gameversion":"4.0.0.1","build":"b"}"#));
        t.apply(&ev(r#"{"timestamp":"2026-03-09T09:06:00Z","event":"Docked","StationName":"Dawes Hub","MarketID":128000001,"StarSystem":"Achenar","SystemAddress":2}"#));
        assert_eq!(t.snapshot().market_id, Some(128000001));
        t.reset_for_new_file();
        let s = t.snapshot();
        assert_eq!(s, TransientSnapshot::default());
        assert_eq!(s.cmdr_fid, None);
        assert_eq!(s.market_id, None);
        assert_eq!(s.system_name, None);
        assert_eq!(s.galaxy, Galaxy::Unknown);
    }

    #[test]
    fn loadgame_with_other_fid_clears_location() {
        let mut t = TransientTracker::new();
        t.apply(&ev(r#"{"timestamp":"2026-03-09T09:00:12Z","event":"LoadGame","FID":"F0000001","Commander":"C","gameversion":"4.0.0.1","build":"b"}"#));
        t.apply(&ev(r#"{"timestamp":"2026-03-09T09:01:00Z","event":"Location","StarSystem":"Sol","SystemAddress":10477373803,"StarPos":[0,0,0],"Docked":true,"StationName":"Abraham Lincoln","MarketID":128016640}"#));
        assert_eq!(t.snapshot().system_name.as_deref(), Some("Sol"));
        assert_eq!(t.snapshot().market_id, Some(128016640));

        // Mismo FID: conserva el sistema (relogueo del mismo comandante).
        t.apply(&ev(r#"{"timestamp":"2026-03-09T09:02:00Z","event":"LoadGame","FID":"F0000001","Commander":"C","gameversion":"4.0.0.1","build":"b"}"#));
        assert_eq!(t.snapshot().system_name.as_deref(), Some("Sol"));

        t.apply(&ev(r#"{"timestamp":"2026-03-09T09:03:00Z","event":"LoadGame","FID":"F0000002","Commander":"D","gameversion":"4.0.0.1","build":"b"}"#));
        let s = t.snapshot();
        assert_eq!(s.cmdr_fid.as_deref(), Some("F0000002"));
        assert_eq!(s.system_name, None);
        assert_eq!(s.system_address, None);
        assert_eq!(s.star_pos, None);
        assert_eq!(s.market_id, None);
        assert_eq!(s.station_name, None);
    }

    #[test]
    fn commander_with_other_fid_clears_location() {
        let mut t = TransientTracker::new();
        t.apply(&ev(r#"{"timestamp":"2026-03-09T09:00:05Z","event":"Commander","FID":"F0000001","Name":"C"}"#));
        t.apply(&ev(r#"{"timestamp":"2026-03-09T09:01:00Z","event":"FSDJump","StarSystem":"Achenar","SystemAddress":2,"StarPos":[67.5,-119.4,24.8]}"#));
        t.apply(&ev(r#"{"timestamp":"2026-03-09T09:20:05Z","event":"Commander","FID":"F0000002","Name":"D"}"#));
        let s = t.snapshot();
        assert_eq!(s.cmdr_fid.as_deref(), Some("F0000002"));
        assert_eq!(s.system_name, None);
        assert_eq!(s.system_address, None);
        assert_eq!(s.star_pos, None);
    }
}
