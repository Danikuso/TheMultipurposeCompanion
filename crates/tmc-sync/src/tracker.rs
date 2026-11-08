//! Estado transitorio del juego que EDDN, EDSM e Inara exigen en sus cabeceras y cargas.

use tmc_model::events::{EventKind, JournalEvent, LocationInfo};
use tmc_model::state::Galaxy;

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
    pub market_id: Option<u64>,
    pub station_name: Option<String>,
    pub ship_id: Option<u64>,
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
                if let Some(o) = raw_bool(ev, "Odyssey") {
                    self.state.odyssey = Some(o);
                }
            }
            EventKind::Commander(c) => {
                if c.fid.is_some() {
                    self.state.cmdr_fid = c.fid.clone();
                }
                if c.name.is_some() {
                    self.state.cmdr_name = c.name.clone();
                }
            }
            EventKind::LoadGame(l) => {
                if l.fid.is_some() {
                    self.state.cmdr_fid = l.fid.clone();
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
                // Una carga nueva parte sin estación hasta el siguiente Location/Docked.
                self.state.market_id = None;
                self.state.station_name = None;
            }
            EventKind::Location(i) | EventKind::CarrierJump(i) => self.set_location(i, true),
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
            EventKind::ShipyardSwap(s) if s.ship_id.is_some() => {
                self.state.ship_id = s.ship_id;
            }
            _ => {}
        }
    }

    pub fn snapshot(&self) -> TransientSnapshot {
        self.state.clone()
    }

    /// Descarta todo el estado al empezar a leer otro archivo de diario.
    pub fn reset_for_new_file(&mut self) {
        self.state = TransientSnapshot::default();
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
        if keep_station && i.docked {
            self.state.market_id = i.market_id;
            self.state.station_name = i.station_name.clone();
        } else {
            self.state.market_id = None;
            self.state.station_name = None;
        }
    }
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
        t.reset_for_new_file();
        let s = t.snapshot();
        assert_eq!(s.cmdr_fid, None);
        assert_eq!(s.galaxy, Galaxy::Unknown);
    }
}
