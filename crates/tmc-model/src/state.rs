//! Estado de juego derivado: reductores puros de eventos, Status y archivos compañeros.

use crate::companion::{CargoItem, CompanionFile, RouteHop};
use crate::events::{EventKind, JournalEvent, LocationInfo, Rank};
use crate::status::{Flags, Flags2, Status};
use bitflags::bitflags;
use chrono::{DateTime, Utc};
use serde::Serialize;

bitflags! {
    /// Áreas del estado que cambiaron tras aplicar una entrada.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Changes: u8 {
        const CMDR = 1 << 0;
        const SHIP = 1 << 1;
        const LOCATION = 1 << 2;
        const STATUS = 1 << 3;
        const ROUTE = 1 << 4;
        const SESSION = 1 << 5;
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub enum Galaxy {
    #[default]
    Unknown,
    Live,
    Legacy,
}

impl Galaxy {
    /// `gameversion` >= 4.0 es la galaxia Live; anterior, Legacy.
    pub fn from_game_version(v: &str) -> Self {
        match v
            .trim()
            .split('.')
            .next()
            .and_then(|m| m.parse::<u32>().ok())
        {
            Some(major) if major >= 4 => Self::Live,
            Some(_) => Self::Legacy,
            None => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub enum FlightMode {
    #[default]
    Unknown,
    Docked,
    Landed,
    NormalSpace,
    Supercruise,
    Hyperspace,
    OnFoot,
    InSrv,
    InFighter,
    InTaxi,
    Multicrew,
}

impl FlightMode {
    /// Prioridad: OnFoot > InTaxi > Multicrew > InSrv > InFighter > Docked > Landed >
    /// Hyperspace > Supercruise > NormalSpace.
    pub fn from_status(flags: Flags, flags2: Flags2) -> Self {
        if flags2.contains(Flags2::ON_FOOT) {
            Self::OnFoot
        } else if flags2.contains(Flags2::IN_TAXI) {
            Self::InTaxi
        } else if flags2.contains(Flags2::IN_MULTICREW) {
            Self::Multicrew
        } else if flags.contains(Flags::IN_SRV) {
            Self::InSrv
        } else if flags.contains(Flags::IN_FIGHTER) {
            Self::InFighter
        } else if flags.contains(Flags::DOCKED) {
            Self::Docked
        } else if flags.contains(Flags::LANDED) {
            Self::Landed
        } else if flags.contains(Flags::FSD_JUMP) {
            Self::Hyperspace
        } else if flags.contains(Flags::SUPERCRUISE) {
            Self::Supercruise
        } else {
            Self::NormalSpace
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Ranks {
    pub combat: Option<u8>,
    pub trade: Option<u8>,
    pub explore: Option<u8>,
    pub soldier: Option<u8>,
    pub exobiologist: Option<u8>,
    pub empire: Option<u8>,
    pub federation: Option<u8>,
    pub cqc: Option<u8>,
}

impl From<&Rank> for Ranks {
    fn from(r: &Rank) -> Self {
        let c = |v: Option<u32>| v.map(|n| u8::try_from(n).unwrap_or(u8::MAX));
        Self {
            combat: c(r.combat),
            trade: c(r.trade),
            explore: c(r.explore),
            soldier: c(r.soldier),
            exobiologist: c(r.exobiologist),
            empire: c(r.empire),
            federation: c(r.federation),
            cqc: c(r.cqc),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CommanderState {
    pub name: String,
    pub fid: String,
    pub credits: Option<i64>,
    pub loan: Option<i64>,
    pub ranks: Ranks,
    pub progress: Ranks,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ShipState {
    pub ship_type: String,
    pub ship_type_localised: Option<String>,
    pub ship_id: Option<u64>,
    pub name: Option<String>,
    pub ident: Option<String>,
    pub hull_health: Option<f64>,
    pub fuel_main: Option<f64>,
    pub fuel_reservoir: Option<f64>,
    pub fuel_capacity_main: Option<f64>,
    pub fuel_capacity_reserve: Option<f64>,
    pub cargo_count: Option<u32>,
    pub cargo_capacity: Option<u32>,
    pub rebuy: Option<i64>,
    pub max_jump_range: Option<f64>,
    /// Módulos con salud < 1.0 (nombre del ítem, o ranura si no hay ítem).
    pub damaged_modules: Vec<String>,
    pub cargo: Vec<CargoItem>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct LocationState {
    pub system: String,
    pub system_address: u64,
    pub star_pos: [f64; 3],
    pub body: Option<String>,
    pub body_type: Option<String>,
    pub station: Option<String>,
    pub station_type: Option<String>,
    pub market_id: Option<u64>,
    pub mode: FlightMode,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct RouteState {
    pub hops: Vec<RouteHop>,
    pub next_system: Option<String>,
    pub remaining_jumps: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SessionState {
    pub started_at: Option<DateTime<Utc>>,
    pub jumps: u32,
    pub distance_ly: f64,
    pub credits_at_start: Option<i64>,
    pub credits_now: Option<i64>,
}

impl SessionState {
    pub fn credits_delta(&self) -> Option<i64> {
        Some(self.credits_now? - self.credits_at_start?)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct GameState {
    pub cmdr: Option<CommanderState>,
    pub ship: Option<ShipState>,
    pub location: LocationState,
    pub status: Option<Status>,
    pub route: RouteState,
    pub session: SessionState,
    pub galaxy: Galaxy,
    pub game_running: bool,
}

impl GameState {
    /// Aplica un evento del diario y devuelve las áreas que cambiaron.
    pub fn apply_event(&mut self, ev: &JournalEvent) -> Changes {
        let before = self.clone();
        self.reduce_event(ev);
        self.diff(&before)
    }

    /// Aplica una lectura de Status.json.
    pub fn apply_status(&mut self, s: &Status) -> Changes {
        let before = self.clone();
        self.status = Some(s.clone());
        self.location.mode = FlightMode::from_status(s.flags, s.flags2);
        if let Some(balance) = s.balance {
            self.session.credits_now = Some(balance);
            if let Some(c) = self.cmdr.as_mut() {
                c.credits = Some(balance);
            }
        }
        if let Some(f) = &s.fuel {
            let ship = self.ship.get_or_insert_with(ShipState::default);
            ship.fuel_main = Some(f.main);
            ship.fuel_reservoir = Some(f.reservoir);
        }
        self.diff(&before)
    }

    /// Aplica un archivo compañero (Cargo.json, NavRoute.json…).
    pub fn apply_companion(&mut self, c: &CompanionFile) -> Changes {
        let before = self.clone();
        match c {
            CompanionFile::Cargo(f) => {
                if f.vessel.as_deref().is_none_or(|v| v == "Ship") {
                    let ship = self.ship.get_or_insert_with(ShipState::default);
                    ship.cargo_count = Some(f.count);
                    ship.cargo = f.inventory.clone();
                }
            }
            CompanionFile::NavRoute(f) => {
                self.route.hops = f.route.clone();
                self.recompute_next_system();
            }
            CompanionFile::Raw(..) => {}
        }
        self.diff(&before)
    }

    fn diff(&self, before: &GameState) -> Changes {
        let mut c = Changes::empty();
        c.set(Changes::CMDR, self.cmdr != before.cmdr);
        c.set(Changes::SHIP, self.ship != before.ship);
        c.set(Changes::LOCATION, self.location != before.location);
        c.set(Changes::STATUS, self.status != before.status);
        c.set(Changes::ROUTE, self.route != before.route);
        c.set(
            Changes::SESSION,
            self.session != before.session
                || self.galaxy != before.galaxy
                || self.game_running != before.game_running,
        );
        c
    }

    /// Siguiente sistema: primer salto de la ruta posterior al sistema actual;
    /// sin ruta se conserva el que fijó `FSDTarget`.
    fn recompute_next_system(&mut self) {
        let hops = &self.route.hops;
        if hops.is_empty() {
            return;
        }
        let cur = self.location.system.as_str();
        let next = match hops.iter().position(|h| h.star_system == cur) {
            Some(i) => hops.get(i + 1),
            None => hops.iter().find(|h| h.star_system != cur),
        };
        self.route.next_system = next.map(|h| h.star_system.clone());
    }

    fn set_location(&mut self, l: &LocationInfo) {
        let loc = &mut self.location;
        loc.system = l.star_system.clone();
        loc.system_address = l.system_address;
        loc.star_pos = l.star_pos;
        loc.body = l.body.clone();
        loc.body_type = l.body_type.clone();
        if l.docked {
            loc.station = l.station_name.clone();
            loc.station_type = l.station_type.clone();
            loc.market_id = l.market_id;
        } else {
            loc.station = None;
            loc.station_type = None;
            loc.market_id = None;
        }
    }

    fn reduce_event(&mut self, ev: &JournalEvent) {
        match &ev.kind {
            EventKind::Fileheader(f) => {
                if let Some(v) = &f.gameversion {
                    self.galaxy = Galaxy::from_game_version(v);
                }
            }
            EventKind::Commander(c) => {
                let cmdr = self.cmdr.get_or_insert_with(CommanderState::default);
                if let Some(n) = &c.name {
                    cmdr.name = n.clone();
                }
                if let Some(f) = &c.fid {
                    cmdr.fid = f.clone();
                }
            }
            EventKind::LoadGame(l) => {
                let credits = l.credits.and_then(|c| i64::try_from(c).ok());
                let cmdr = self.cmdr.get_or_insert_with(CommanderState::default);
                if let Some(n) = &l.commander {
                    cmdr.name = n.clone();
                }
                if let Some(f) = &l.fid {
                    cmdr.fid = f.clone();
                }
                cmdr.credits = credits;
                cmdr.loan = l.loan.and_then(|c| i64::try_from(c).ok());
                self.ship = Some(ShipState {
                    ship_type: l.ship.clone().unwrap_or_default(),
                    ship_type_localised: l.ship_localised.clone(),
                    ship_id: l.ship_id,
                    name: l.ship_name.clone(),
                    ident: l.ship_ident.clone(),
                    fuel_main: l.fuel_level,
                    fuel_capacity_main: l.fuel_capacity,
                    ..ShipState::default()
                });
                self.session = SessionState {
                    started_at: Some(ev.timestamp),
                    jumps: 0,
                    distance_ly: 0.0,
                    credits_at_start: credits,
                    credits_now: credits,
                };
                if let Some(v) = &l.gameversion {
                    self.galaxy = Galaxy::from_game_version(v);
                }
                self.game_running = true;
            }
            EventKind::Rank(r) => {
                self.cmdr.get_or_insert_with(CommanderState::default).ranks = Ranks::from(r);
            }
            EventKind::Progress(r) => {
                self.cmdr
                    .get_or_insert_with(CommanderState::default)
                    .progress = Ranks::from(r);
            }
            EventKind::Location(l) | EventKind::CarrierJump(l) => {
                self.set_location(l);
                self.location.mode = if l.docked {
                    FlightMode::Docked
                } else {
                    FlightMode::NormalSpace
                };
                self.recompute_next_system();
            }
            EventKind::FsdJump(l) => {
                self.set_location(l);
                self.location.mode = FlightMode::Supercruise;
                self.session.jumps += 1;
                self.session.distance_ly += l.jump_dist.unwrap_or(0.0);
                if let Some(f) = l.fuel_level {
                    self.ship.get_or_insert_with(ShipState::default).fuel_main = Some(f);
                }
                if self.route.hops.is_empty() {
                    if self.route.next_system.as_deref() == Some(l.star_system.as_str()) {
                        self.route.next_system = None;
                        self.route.remaining_jumps = None;
                    }
                } else {
                    self.recompute_next_system();
                }
            }
            EventKind::Docked(d) => {
                let loc = &mut self.location;
                loc.station = d.station_name.clone();
                loc.station_type = d.station_type.clone();
                loc.market_id = d.market_id;
                loc.mode = FlightMode::Docked;
                if let Some(s) = &d.star_system {
                    loc.system = s.clone();
                }
                if let Some(a) = d.system_address {
                    loc.system_address = a;
                }
            }
            EventKind::Undocked(_) => {
                let loc = &mut self.location;
                loc.station = None;
                loc.station_type = None;
                loc.market_id = None;
                loc.mode = FlightMode::NormalSpace;
            }
            EventKind::SupercruiseEntry(_) => self.location.mode = FlightMode::Supercruise,
            EventKind::SupercruiseExit(x) => {
                if let Some(s) = &x.star_system {
                    self.location.system = s.clone();
                }
                self.location.body = x.body.clone();
                self.location.body_type = x.body_type.clone();
                self.location.mode = FlightMode::NormalSpace;
            }
            EventKind::ApproachBody(b) => self.location.body = b.body.clone(),
            EventKind::LeaveBody(_) => {
                self.location.body = None;
                self.location.body_type = None;
            }
            EventKind::Touchdown => self.location.mode = FlightMode::Landed,
            EventKind::Liftoff => self.location.mode = FlightMode::NormalSpace,
            EventKind::StartJump(j) => {
                if j.jump_type.as_deref() == Some("Hyperspace") {
                    self.location.mode = FlightMode::Hyperspace;
                }
            }
            EventKind::FsdTarget(t) => {
                self.route.remaining_jumps = t.remaining_jumps_in_route;
                if self.route.hops.is_empty() {
                    self.route.next_system = t.name.clone();
                } else {
                    self.recompute_next_system();
                }
            }
            EventKind::NavRouteClear => {
                self.route = RouteState::default();
            }
            EventKind::Loadout(l) => {
                let ship = self.ship.get_or_insert_with(ShipState::default);
                let new_type = l.ship.clone().unwrap_or_default();
                if ship.ship_type != new_type {
                    ship.ship_type_localised = None;
                }
                ship.ship_type = new_type;
                ship.ship_id = l.ship_id;
                ship.name = l.ship_name.clone();
                ship.ident = l.ship_ident.clone();
                ship.hull_health = l.hull_health;
                ship.cargo_capacity = l.cargo_capacity;
                ship.max_jump_range = l.max_jump_range;
                ship.rebuy = l.rebuy.and_then(|r| i64::try_from(r).ok());
                if let Some(fc) = &l.fuel_capacity {
                    ship.fuel_capacity_main = fc.main;
                    ship.fuel_capacity_reserve = fc.reserve;
                }
                ship.damaged_modules = l
                    .modules
                    .iter()
                    .filter(|m| m.health.is_some_and(|h| h < 1.0))
                    .map(|m| {
                        m.item
                            .clone()
                            .or_else(|| m.slot.clone())
                            .unwrap_or_default()
                    })
                    .collect();
            }
            EventKind::Cargo(c) => {
                if c.vessel.as_deref().is_none_or(|v| v == "Ship") {
                    self.ship.get_or_insert_with(ShipState::default).cargo_count = c.count;
                }
            }
            EventKind::HullDamage(h) => {
                if h.fighter != Some(true) && h.player_pilot != Some(false) {
                    self.ship.get_or_insert_with(ShipState::default).hull_health = h.health;
                }
            }
            EventKind::FuelScoop(f) => {
                if let Some(t) = f.total {
                    self.ship.get_or_insert_with(ShipState::default).fuel_main = Some(t);
                }
            }
            EventKind::ShipyardSwap(s) => {
                self.ship = Some(ShipState {
                    ship_type: s.ship_type.clone().unwrap_or_default(),
                    ship_id: s.ship_id,
                    ..ShipState::default()
                });
            }
            EventKind::Shutdown => self.game_running = false,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::companion::{self, CompanionKind};
    use crate::events::parse_line;

    fn ev(line: &str) -> JournalEvent {
        parse_line(line).unwrap()
    }
    fn apply(g: &mut GameState, line: &str) -> Changes {
        g.apply_event(&ev(line))
    }
    fn status(flags: u32, flags2: u32, extra: &str) -> Status {
        Status::parse(&format!(
            r#"{{"timestamp":"2026-03-05T18:10:00Z","event":"Status","Flags":{flags},"Flags2":{flags2}{extra}}}"#
        ))
        .unwrap()
    }
    const LOAD: &str = r#"{"timestamp":"2026-03-05T18:00:12Z","event":"LoadGame","FID":"F1","Commander":"TESTER","Ship":"python","Ship_Localised":"Python","ShipID":7,"Credits":1000,"Loan":0,"gameversion":"4.1.0.0"}"#;
    fn jump(sys: &str, dist: f64, fuel: f64) -> String {
        format!(
            r#"{{"timestamp":"2026-03-05T18:03:22Z","event":"FSDJump","StarSystem":"{sys}","SystemAddress":1,"StarPos":[1.0,2.0,3.0],"JumpDist":{dist},"FuelUsed":1.0,"FuelLevel":{fuel}}}"#
        )
    }
    fn route(names: &[&str]) -> CompanionFile {
        let hops: Vec<String> = names
            .iter()
            .map(|n| format!(r#"{{"StarSystem":"{n}","SystemAddress":1,"StarPos":[0,0,0]}}"#))
            .collect();
        companion::parse(
            CompanionKind::NavRoute,
            &format!(r#"{{"Route":[{}]}}"#, hops.join(",")),
        )
        .unwrap()
    }

    #[test]
    fn loadgame_starts_session_and_sets_cmdr() {
        let mut g = GameState::default();
        g.session.jumps = 5;
        let c = apply(&mut g, LOAD);
        assert!(c.contains(Changes::CMDR | Changes::SHIP | Changes::SESSION));
        let cmdr = g.cmdr.as_ref().unwrap();
        assert_eq!((cmdr.name.as_str(), cmdr.fid.as_str()), ("TESTER", "F1"));
        assert_eq!(cmdr.credits, Some(1000));
        assert_eq!(g.session.jumps, 0);
        assert_eq!(g.session.credits_at_start, Some(1000));
        assert_eq!(
            g.session.started_at.unwrap().to_rfc3339(),
            "2026-03-05T18:00:12+00:00"
        );
        assert!(g.game_running);
    }

    #[test]
    fn fsdjump_counts_jump_distance_and_moves_location() {
        let mut g = GameState::default();
        apply(&mut g, LOAD);
        let c = apply(&mut g, &jump("Exampla", 9.5, 18.0));
        apply(&mut g, &jump("Sampleton", 8.0, 17.0));
        assert!(c.contains(Changes::LOCATION | Changes::SESSION | Changes::SHIP));
        assert_eq!(g.session.jumps, 2);
        assert!((g.session.distance_ly - 17.5).abs() < 1e-9);
        assert_eq!(g.location.system, "Sampleton");
        assert_eq!(g.location.star_pos, [1.0, 2.0, 3.0]);
        assert_eq!(g.ship.as_ref().unwrap().fuel_main, Some(17.0));
    }

    #[test]
    fn next_system_follows_navroute_after_current_system() {
        let mut g = GameState::default();
        apply(&mut g, &jump("A", 1.0, 10.0));
        // Sin ruta: FSDTarget.
        apply(
            &mut g,
            r#"{"timestamp":"2026-03-05T18:03:00Z","event":"FSDTarget","Name":"Z","SystemAddress":9,"RemainingJumpsInRoute":4}"#,
        );
        assert_eq!(g.route.next_system.as_deref(), Some("Z"));
        assert_eq!(g.route.remaining_jumps, Some(4));
        // Con ruta: primer salto posterior al sistema actual.
        let c = g.apply_companion(&route(&["A", "B", "C"]));
        assert!(c.contains(Changes::ROUTE));
        assert_eq!(g.route.next_system.as_deref(), Some("B"));
        apply(&mut g, &jump("B", 1.0, 9.0));
        assert_eq!(g.route.next_system.as_deref(), Some("C"));
    }

    #[test]
    fn navroute_clear_empties_route() {
        let mut g = GameState::default();
        apply(&mut g, &jump("A", 1.0, 10.0));
        g.apply_companion(&route(&["A", "B"]));
        let c = apply(
            &mut g,
            r#"{"timestamp":"2026-03-05T18:03:00Z","event":"NavRouteClear"}"#,
        );
        assert!(c.contains(Changes::ROUTE));
        assert!(g.route.hops.is_empty());
        assert_eq!(g.route.next_system, None);
        assert_eq!(g.route.remaining_jumps, None);
    }

    #[test]
    fn flight_mode_derived_from_status_priority() {
        let mut g = GameState::default();
        let docked = (Flags::DOCKED | Flags::IN_MAIN_SHIP).bits();
        g.apply_status(&status(docked, 0, ""));
        assert_eq!(g.location.mode, FlightMode::Docked);
        g.apply_status(&status(docked, Flags2::ON_FOOT.bits(), ""));
        assert_eq!(g.location.mode, FlightMode::OnFoot);
        let m = FlightMode::from_status;
        let none = Flags2::empty();
        assert_eq!(m(Flags::SUPERCRUISE, none), FlightMode::Supercruise);
        assert_eq!(
            m(Flags::SUPERCRUISE | Flags::FSD_JUMP, none),
            FlightMode::Hyperspace
        );
        assert_eq!(m(Flags::empty(), none), FlightMode::NormalSpace);
        assert_eq!(m(Flags::LANDED | Flags::DOCKED, none), FlightMode::Docked);
        assert_eq!(m(Flags::IN_SRV | Flags::DOCKED, none), FlightMode::InSrv);
        assert_eq!(
            m(Flags::IN_FIGHTER | Flags::DOCKED, none),
            FlightMode::InFighter
        );
        assert_eq!(
            m(Flags::IN_SRV, Flags2::IN_MULTICREW),
            FlightMode::Multicrew
        );
        assert_eq!(
            m(Flags::IN_SRV, Flags2::IN_MULTICREW | Flags2::IN_TAXI),
            FlightMode::InTaxi
        );
    }

    #[test]
    fn status_balance_updates_credits_delta() {
        let mut g = GameState::default();
        apply(&mut g, LOAD);
        let extra = r#","Balance":1500,"Fuel":{"FuelMain":12.5,"FuelReservoir":0.4}"#;
        let c = g.apply_status(&status(0, 0, extra));
        assert!(c.contains(Changes::STATUS | Changes::SESSION | Changes::CMDR | Changes::SHIP));
        assert_eq!(g.session.credits_delta(), Some(500));
        assert_eq!(g.cmdr.as_ref().unwrap().credits, Some(1500));
        let ship = g.ship.as_ref().unwrap();
        assert_eq!(
            (ship.fuel_main, ship.fuel_reservoir),
            (Some(12.5), Some(0.4))
        );
        // Mismo contenido: no cambian cmdr, nave ni sesión.
        let c2 = g.apply_status(&status(0, 0, extra));
        assert!(!c2.intersects(Changes::SESSION | Changes::CMDR | Changes::SHIP));
    }

    #[test]
    fn galaxy_live_vs_legacy_from_gameversion() {
        let fh = |v: &str| {
            format!(
                r#"{{"timestamp":"2026-03-05T18:00:01Z","event":"Fileheader","gameversion":"{v}"}}"#
            )
        };
        let mut g = GameState::default();
        apply(&mut g, &fh("4.4.1.1"));
        assert_eq!(g.galaxy, Galaxy::Live);
        apply(&mut g, &fh("3.8.0.407"));
        assert_eq!(g.galaxy, Galaxy::Legacy);
        assert_eq!(Galaxy::from_game_version("x"), Galaxy::Unknown);
    }

    #[test]
    fn loadout_marks_damaged_modules() {
        let mut g = GameState::default();
        apply(
            &mut g,
            r#"{"timestamp":"2026-03-05T18:00:14Z","event":"Loadout","Ship":"python","ShipID":7,"Modules":[{"Slot":"A","Item":"fsd","Health":1.0},{"Slot":"B","Item":"cargo","Health":0.5}]}"#,
        );
        assert_eq!(
            g.ship.as_ref().unwrap().damaged_modules,
            vec!["cargo".to_string()]
        );
    }

    #[test]
    fn basic_session_fixture_end_state() {
        let text = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/journal/basic_session.log"
        ));
        let mut g = GameState::default();
        let mut checked_target = false;
        for line in text.lines() {
            let e = parse_line(line).unwrap();
            let name = e.name.clone();
            g.apply_event(&e);
            if name == "FSDTarget" && !checked_target {
                checked_target = true;
                assert_eq!(g.route.remaining_jumps, Some(2));
                assert_eq!(g.route.next_system.as_deref(), Some("Exampla"));
            }
        }
        assert!(checked_target);
        assert_eq!(g.galaxy, Galaxy::Live);
        let cmdr = g.cmdr.as_ref().unwrap();
        assert_eq!(
            (cmdr.name.as_str(), cmdr.fid.as_str()),
            ("TESTER", "F0000001")
        );
        assert_eq!(cmdr.credits, Some(12_345_678));
        assert_eq!(cmdr.ranks.combat, Some(3));
        assert_eq!(cmdr.ranks.cqc, Some(0));
        assert_eq!(cmdr.ranks.federation, Some(1));
        assert_eq!(cmdr.progress.explore, Some(88));
        let ship = g.ship.as_ref().unwrap();
        assert_eq!(ship.ship_type, "python");
        assert_eq!(ship.ship_type_localised.as_deref(), Some("Python"));
        assert_eq!(ship.ship_id, Some(7));
        assert_eq!(ship.name.as_deref(), Some("TEST HAULER"));
        assert_eq!(ship.ident.as_deref(), Some("TS-01A"));
        assert_eq!(ship.fuel_capacity_main, Some(32.0));
        assert_eq!(ship.fuel_capacity_reserve, Some(0.63));
        assert_eq!(ship.cargo_capacity, Some(64));
        assert_eq!(ship.cargo_count, Some(12));
        assert_eq!(ship.max_jump_range, Some(18.25));
        assert_eq!(ship.rebuy, Some(500_000));
        assert_eq!(ship.hull_health, Some(0.95));
        assert_eq!(ship.fuel_main, Some(22.0));
        assert!(ship.damaged_modules.is_empty());
        assert_eq!(g.location.system, "Sampleton");
        assert_eq!(g.location.station.as_deref(), Some("Sample Hub"));
        assert_eq!(g.location.station_type.as_deref(), Some("Coriolis"));
        assert_eq!(g.location.market_id, Some(128000002));
        assert_eq!(g.location.mode, FlightMode::Docked);
        assert_eq!(g.session.jumps, 2);
        assert!((g.session.distance_ly - 17.973).abs() < 1e-9);
        assert!(!g.game_running);
        assert_eq!(g.route.next_system, None);
        serde_json::to_value(&g).unwrap();
    }

    #[test]
    fn apply_returns_only_changed_areas() {
        let mut g = GameState::default();
        let c = apply(
            &mut g,
            r#"{"timestamp":"2026-03-05T18:06:31Z","event":"UnderAttack","Target":"You"}"#,
        );
        assert_eq!(c, Changes::empty());
    }
}
