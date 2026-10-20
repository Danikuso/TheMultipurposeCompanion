//! Parámetros reales y avance de la ruta comercial al atracar.

use chrono::{DateTime, Utc};
use serde::Serialize;
use tmc_model::events::Docked;
use tmc_model::net::{PadSize, TradeRouteParams, TradeRoutePlan};
use tmc_model::state::{FlightMode, GameState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RouteBlocker {
    NotDocked,
    NoShip,
    NoCredits,
}

pub fn route_params(
    state: &GameState,
    ship_pad: Option<PadSize>,
    max_hops: u8,
) -> Result<TradeRouteParams, RouteBlocker> {
    let loc = &state.location;
    let station = loc.station.as_deref().map(str::trim).unwrap_or("");
    if loc.mode != FlightMode::Docked || station.is_empty() || loc.system.trim().is_empty() {
        decide!("trade.route.blocked", { "reason": 0 });
        return Err(RouteBlocker::NotDocked);
    }
    let ship = state.ship.as_ref();
    let capacity = ship.and_then(|s| s.cargo_capacity).filter(|c| *c > 0);
    let range = ship
        .and_then(|s| s.max_jump_range)
        .filter(|r| r.is_finite() && *r > 0.0);
    let (Some(max_cargo), Some(max_hop_distance_ly)) = (capacity, range) else {
        decide!("trade.route.blocked", { "reason": 1 });
        return Err(RouteBlocker::NoShip);
    };
    let credits = state
        .cmdr
        .as_ref()
        .and_then(|c| c.credits)
        .filter(|c| *c > 0);
    let Some(credits) = credits else {
        decide!("trade.route.blocked", { "reason": 2 });
        return Err(RouteBlocker::NoCredits);
    };
    Ok(TradeRouteParams {
        system: loc.system.clone(),
        station: station.to_string(),
        starting_capital: credits as u64,
        max_cargo,
        max_hops,
        max_hop_distance_ly,
        min_pad: ship_pad.unwrap_or(PadSize::Large),
        max_price_age_days: 2,
        allow_planetary: false,
    })
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct TradeRouteState {
    pub plan: Option<TradeRoutePlan>,
    pub current: usize,
    pub completed: bool,
    pub copy_target: Option<String>,
    pub copy_seq: u64,
    pub requested_at: Option<DateTime<Utc>>,
}

impl TradeRouteState {
    pub fn on_plan(&mut self, plan: TradeRoutePlan) {
        self.current = 0;
        self.completed = false;
        self.requested_at = None;
        self.copy_target = plan.stops.first().map(|s| s.to_system.clone());
        if self.copy_target.is_some() {
            self.copy_seq += 1;
        }
        decide!("trade.route.planned", { "stops": plan.stops.len() });
        self.plan = Some(plan);
    }

    /// Avanza si se atraca en el destino de la parada actual; devuelve el sistema a copiar.
    pub fn on_docked(&mut self, d: &Docked) -> Option<String> {
        if self.completed {
            return None;
        }
        let stops = &self.plan.as_ref()?.stops;
        let stop = stops.get(self.current)?;
        // Con MarketID en ambos lados manda la identidad; si no, el nombre.
        let matches = match stop.to_market_id.zip(d.market_id) {
            Some((expected, docked)) => expected == docked,
            None => {
                let (Some(system), Some(station)) =
                    (d.star_system.as_deref(), d.station_name.as_deref())
                else {
                    return None;
                };
                stop.to_station.trim().eq_ignore_ascii_case(station.trim())
                    && stop.to_system.trim().eq_ignore_ascii_case(system.trim())
            }
        };
        if !matches {
            decide!("trade.route.off_route", { "current": self.current });
            return None;
        }
        let next = stops.get(self.current + 1).map(|s| s.to_system.clone());
        self.current += 1;
        match next {
            Some(sys) => {
                self.copy_target = Some(sys.clone());
                self.copy_seq += 1;
                decide!("trade.route.advanced", { "current": self.current });
                Some(sys)
            }
            None => {
                self.completed = true;
                self.copy_target = None;
                decide!("trade.route.completed", { "stops": self.current });
                None
            }
        }
    }

    pub fn jumps(&self, stop: usize, range: f64) -> Option<u32> {
        if !range.is_finite() || range <= 0.0 {
            return None;
        }
        let distance = self.plan.as_ref()?.stops.get(stop)?.distance_ly;
        if !distance.is_finite() || distance < 0.0 {
            return None;
        }
        let jumps = (distance / range).ceil();
        (jumps.is_finite() && jumps <= f64::from(u32::MAX)).then_some(jumps as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmc_model::net::TradeStop;
    use tmc_model::state::{CommanderState, FlightMode, LocationState, ShipState};

    fn docked_state() -> GameState {
        GameState {
            location: LocationState {
                system: "A".into(),
                station: Some("Port A".into()),
                mode: FlightMode::Docked,
                ..Default::default()
            },
            ship: Some(ShipState {
                ship_type: "type9".into(),
                cargo_capacity: Some(256),
                max_jump_range: Some(22.5),
                ..Default::default()
            }),
            cmdr: Some(CommanderState {
                credits: Some(5_000_000),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn plan() -> TradeRoutePlan {
        TradeRoutePlan {
            stops: [("A", "B"), ("B", "C")]
                .into_iter()
                .map(|(from, to)| TradeStop {
                    from_system: from.into(),
                    from_station: format!("Port {from}"),
                    to_system: to.into(),
                    to_station: format!("Port {to}"),
                    from_market_id: None,
                    to_market_id: None,
                    commodities: Vec::new(),
                    stop_profit: 0,
                    distance_ly: 47.0,
                    data_updated_at: None,
                })
                .collect(),
            total_profit: 0,
        }
    }

    fn docking(system: &str, station: &str) -> Docked {
        Docked {
            star_system: Some(system.into()),
            station_name: Some(station.into()),
            ..Default::default()
        }
    }

    #[test]
    fn params_use_real_ship_and_credits() {
        let p = route_params(&docked_state(), Some(PadSize::Large), 7).unwrap();
        assert_eq!(p.system, "A");
        assert_eq!(p.station, "Port A");
        assert_eq!(p.max_cargo, 256);
        assert_eq!(p.starting_capital, 5_000_000);
        assert_eq!(p.min_pad, PadSize::Large);
        assert_eq!(p.max_hop_distance_ly, 22.5);
        assert_eq!(p.max_hops, 7);
        assert_eq!(p.max_price_age_days, 2);
        assert!(!p.allow_planetary);
    }

    #[test]
    fn not_docked_blocks_route() {
        let mut state = docked_state();
        state.location.mode = FlightMode::Supercruise;
        assert_eq!(route_params(&state, None, 3), Err(RouteBlocker::NotDocked));
    }

    #[test]
    fn docking_at_next_stop_advances_and_returns_next_system() {
        let mut route = TradeRouteState::default();
        route.on_plan(plan());
        assert_eq!(route.current, 0);
        assert!(!route.completed);
        assert_eq!(route.copy_target.as_deref(), Some("B"));
        assert_eq!(route.copy_seq, 1);
        assert_eq!(route.on_docked(&docking("B", "Port B")), Some("C".into()));
        assert_eq!(route.current, 1);
        assert_eq!(route.copy_target.as_deref(), Some("C"));
        assert_eq!(route.copy_seq, 2);
    }

    #[test]
    fn market_id_match_wins_over_names() {
        let mut p = plan();
        p.stops[0].to_market_id = Some(42);
        let mut route = TradeRouteState::default();
        route.on_plan(p);
        let mut d = docking("Renamed", "Renamed Port");
        d.market_id = Some(42);
        assert_eq!(route.on_docked(&d), Some("C".into()));
        let mut other = docking("C", "Port C");
        other.market_id = Some(7);
        let mut p2 = plan();
        p2.stops[0].to_market_id = Some(42);
        route.on_plan(p2);
        let before = route.clone();
        assert_eq!(route.on_docked(&other), None);
        assert_eq!(route, before);
    }

    #[test]
    fn on_plan_resets_requested_at() {
        let mut route = TradeRouteState {
            requested_at: Some(Utc::now()),
            ..Default::default()
        };
        route.on_plan(plan());
        assert_eq!(route.requested_at, None);
    }

    #[test]
    fn docking_elsewhere_does_not_advance() {
        let mut route = TradeRouteState::default();
        route.on_plan(plan());
        let before = route.clone();
        assert_eq!(route.on_docked(&docking("X", "Port X")), None);
        assert_eq!(route, before);
    }

    #[test]
    fn last_stop_completes_route() {
        let mut route = TradeRouteState::default();
        route.on_plan(plan());
        route.on_docked(&docking("B", "Port B"));
        let seq = route.copy_seq;
        assert_eq!(route.on_docked(&docking("C", "Port C")), None);
        assert_eq!(route.current, 2);
        assert!(route.completed);
        assert_eq!(route.copy_target, None);
        assert_eq!(route.copy_seq, seq);
    }

    #[test]
    fn jumps_estimate() {
        let mut route = TradeRouteState::default();
        route.on_plan(plan());
        assert_eq!(route.jumps(0, 22.5), Some(3));
    }

    #[test]
    fn missing_ship_capacity_or_valid_range_blocks_route() {
        let mut state = docked_state();
        state.ship = None;
        assert_eq!(route_params(&state, None, 3), Err(RouteBlocker::NoShip));
        for capacity in [None, Some(0)] {
            let mut state = docked_state();
            state.ship.as_mut().unwrap().cargo_capacity = capacity;
            assert_eq!(route_params(&state, None, 3), Err(RouteBlocker::NoShip));
        }
        for range in [
            None,
            Some(0.0),
            Some(-1.0),
            Some(f64::NAN),
            Some(f64::INFINITY),
        ] {
            let mut state = docked_state();
            state.ship.as_mut().unwrap().max_jump_range = range;
            assert_eq!(route_params(&state, None, 3), Err(RouteBlocker::NoShip));
        }
    }

    #[test]
    fn missing_or_nonpositive_credits_blocks_route() {
        let mut state = docked_state();
        state.cmdr = None;
        assert_eq!(route_params(&state, None, 3), Err(RouteBlocker::NoCredits));
        for credits in [None, Some(0), Some(-1)] {
            let mut state = docked_state();
            state.cmdr.as_mut().unwrap().credits = credits;
            assert_eq!(route_params(&state, None, 3), Err(RouteBlocker::NoCredits));
        }
    }

    #[test]
    fn missing_docked_location_blocks_route() {
        for station in [None, Some(" ".into())] {
            let mut state = docked_state();
            state.location.station = station;
            assert_eq!(route_params(&state, None, 3), Err(RouteBlocker::NotDocked));
        }
        let mut state = docked_state();
        state.location.system.clear();
        assert_eq!(route_params(&state, None, 3), Err(RouteBlocker::NotDocked));
    }

    #[test]
    fn unknown_ship_pad_uses_conservative_large_pad() {
        let p = route_params(&docked_state(), None, 3).unwrap();
        assert_eq!(p.min_pad, PadSize::Large);
        for pad in [PadSize::Small, PadSize::Medium] {
            assert_eq!(
                route_params(&docked_state(), Some(pad), 3).unwrap().min_pad,
                pad
            );
        }
    }

    #[test]
    fn same_station_in_other_system_does_not_advance() {
        let mut route = TradeRouteState::default();
        route.on_plan(plan());
        let before = route.clone();
        assert_eq!(route.on_docked(&docking("X", "Port B")), None);
        assert_eq!(route, before);
    }

    #[test]
    fn destination_names_match_case_insensitively() {
        let mut route = TradeRouteState::default();
        route.on_plan(plan());
        assert_eq!(route.on_docked(&docking("b", "port b")), Some("C".into()));
        assert_eq!(route.current, 1);
    }

    #[test]
    fn empty_plan_clears_previous_copy_target_without_new_copy() {
        let mut route = TradeRouteState::default();
        route.on_plan(plan());
        route.on_docked(&docking("B", "Port B"));
        let seq = route.copy_seq;
        route.on_plan(TradeRoutePlan {
            stops: vec![],
            total_profit: 0,
        });
        assert_eq!(route.current, 0);
        assert!(!route.completed);
        assert!(route.plan.as_ref().unwrap().stops.is_empty());
        assert_eq!(route.copy_target, None);
        assert_eq!(route.copy_seq, seq);
        assert_eq!(route.on_docked(&docking("B", "Port B")), None);
    }

    #[test]
    fn missing_docking_metadata_does_not_advance() {
        let mut route = TradeRouteState::default();
        route.on_plan(plan());
        let before = route.clone();
        for d in [
            Docked::default(),
            Docked {
                station_name: Some("Port B".into()),
                ..Default::default()
            },
        ] {
            assert_eq!(route.on_docked(&d), None);
            assert_eq!(route, before);
        }
    }

    #[test]
    fn jumps_reject_invalid_range_distance_or_index() {
        let mut route = TradeRouteState::default();
        assert_eq!(route.jumps(0, 22.5), None);
        route.on_plan(plan());
        for range in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::MIN_POSITIVE] {
            assert_eq!(route.jumps(0, range), None);
        }
        assert_eq!(route.jumps(2, 22.5), None);
        assert_eq!(route.jumps(usize::MAX, 22.5), None);
        for distance in [-1.0, f64::NAN, f64::INFINITY, f64::MAX] {
            route.plan.as_mut().unwrap().stops[0].distance_ly = distance;
            assert_eq!(route.jumps(0, 22.5), None);
        }
        route.plan.as_mut().unwrap().stops[0].distance_ly = 0.0;
        assert_eq!(route.jumps(0, 22.5), Some(0));
    }

    #[test]
    fn completed_or_out_of_bounds_route_does_not_advance() {
        let mut route = TradeRouteState::default();
        route.on_plan(plan());
        route.on_docked(&docking("B", "Port B"));
        route.on_docked(&docking("C", "Port C"));
        let before = route.clone();
        assert_eq!(route.on_docked(&docking("C", "Port C")), None);
        assert_eq!(route, before);
        route.current = usize::MAX;
        assert_eq!(route.on_docked(&docking("C", "Port C")), None);
    }
}
