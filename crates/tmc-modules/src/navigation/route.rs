//! Ruta planificada activa: avance por salto, saltos omitidos y fuera de ruta.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tmc_model::net::PlannedHop;
use tmc_model::state::RouteState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanMode {
    Neutron,
    Galaxy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpOutcome {
    Advanced { skipped: usize },
    OffRoute,
    Arrived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteSource {
    Plan,
    NavRoute,
    Current,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivePlan {
    pub created_at: DateTime<Utc>,
    pub origin: String,
    pub destination: String,
    pub mode: PlanMode,
    pub params: serde_json::Value,
    pub hops: Vec<PlannedHop>,
    pub current_index: usize,
    /// No se persiste: se recalcula con `reconcile`.
    #[serde(skip)]
    pub off_route: bool,
}

fn matches(hop: &PlannedHop, system: &str, id64: Option<u64>) -> bool {
    match (hop.id64, id64) {
        (Some(a), Some(b)) => a == b,
        _ => hop.name.eq_ignore_ascii_case(system),
    }
}

impl ActivePlan {
    /// Aplica la llegada a un sistema. Si está en el plan (aunque se
    /// omitan tramos) el índice avanza hasta él; si no, marca fuera de ruta
    /// y conserva el plan.
    fn apply(&mut self, system: &str, id64: Option<u64>) -> JumpOutcome {
        let found = self
            .hops
            .iter()
            .enumerate()
            .skip(self.current_index)
            .find(|(_, h)| matches(h, system, id64))
            .map(|(i, _)| i);
        match found {
            Some(j) => {
                let skipped = j.saturating_sub(self.current_index + 1);
                self.current_index = j;
                self.off_route = false;
                if j + 1 == self.hops.len() {
                    JumpOutcome::Arrived
                } else {
                    JumpOutcome::Advanced { skipped }
                }
            }
            None => {
                self.off_route = true;
                JumpOutcome::OffRoute
            }
        }
    }

    pub fn on_jump(&mut self, system: &str, id64: u64) -> JumpOutcome {
        self.apply(system, Some(id64))
    }

    /// Como `on_jump` con el sistema actual; sin efecto si ya es el hop actual.
    pub fn reconcile(&mut self, system: &str, id64: Option<u64>) -> JumpOutcome {
        self.apply(system, id64)
    }

    pub fn next(&self) -> Option<&PlannedHop> {
        self.hops.get(self.current_index + 1)
    }

    pub fn upcoming(&self, n: usize) -> &[PlannedHop] {
        let start = (self.current_index + 1).min(self.hops.len());
        let end = start.saturating_add(n).min(self.hops.len());
        &self.hops[start..end]
    }

    /// Suma de `distance_ly` de los tramos posteriores al actual.
    pub fn remaining_ly(&self) -> f64 {
        self.upcoming(usize::MAX)
            .iter()
            .map(|h| h.distance_ly)
            .sum()
    }

    pub fn arrived(&self) -> bool {
        !self.hops.is_empty() && self.current_index + 1 >= self.hops.len()
    }
}

/// El plan activo manda; `NavRoute.json` solo sin plan; si no, sistema actual.
pub fn pick_source(plan: Option<&ActivePlan>, game: &RouteState) -> RouteSource {
    if plan.is_some() {
        RouteSource::Plan
    } else if !game.hops.is_empty() {
        RouteSource::NavRoute
    } else {
        RouteSource::Current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmc_model::companion::RouteHop;

    fn hop(name: &str, id: u64, d: f64) -> PlannedHop {
        PlannedHop {
            name: name.into(),
            id64: Some(id),
            star_pos: None,
            distance_ly: d,
            jumps: 1,
            neutron: false,
            refuel: None,
            star_class: None,
        }
    }

    fn plan(names: &[(&str, u64)]) -> ActivePlan {
        ActivePlan {
            created_at: DateTime::from_timestamp(1_700_000_000, 0).unwrap(),
            origin: names[0].0.into(),
            destination: names[names.len() - 1].0.into(),
            mode: PlanMode::Neutron,
            params: serde_json::json!({"range": 40.5}),
            hops: names
                .iter()
                .enumerate()
                .map(|(i, (n, id))| hop(n, *id, 10.0 * i as f64))
                .collect(),
            current_index: 0,
            off_route: false,
        }
    }

    fn abcd() -> ActivePlan {
        plan(&[("A", 1), ("B", 2), ("C", 3), ("D", 4)])
    }

    #[test]
    fn jump_to_next_hop_advances_index() {
        let mut p = plan(&[("A", 1), ("B", 2), ("C", 3)]);
        assert_eq!(p.on_jump("B", 2), JumpOutcome::Advanced { skipped: 0 });
        assert_eq!(p.current_index, 1);
        assert_eq!(p.next().unwrap().name, "C");
    }

    #[test]
    fn jump_skipping_hops_advances_to_reached_system() {
        let mut p = abcd();
        assert_eq!(p.on_jump("C", 3), JumpOutcome::Advanced { skipped: 1 });
        assert_eq!(p.current_index, 2);
        assert_eq!(p.next().unwrap().name, "D");
    }

    #[test]
    fn jump_outside_plan_marks_off_route_and_keeps_plan() {
        let mut p = abcd();
        assert_eq!(p.on_jump("Zeta", 99), JumpOutcome::OffRoute);
        assert!(p.off_route);
        assert_eq!(p.hops.len(), 4);
        assert_eq!(p.current_index, 0);
        assert_eq!(p.next().unwrap().name, "B");
    }

    #[test]
    fn returning_to_plan_clears_off_route() {
        let mut p = abcd();
        p.on_jump("Zeta", 99);
        assert!(p.off_route);
        assert_eq!(p.on_jump("C", 3), JumpOutcome::Advanced { skipped: 1 });
        assert!(!p.off_route);
    }

    #[test]
    fn reaching_destination_reports_arrived() {
        let mut p = abcd();
        assert_eq!(p.on_jump("D", 4), JumpOutcome::Arrived);
        assert!(p.arrived());
        assert_eq!(p.remaining_ly(), 0.0);
        assert!(p.next().is_none());
    }

    #[test]
    fn plan_has_priority_over_game_navroute() {
        let p = abcd();
        let game = RouteState {
            hops: vec![RouteHop::default()],
            ..Default::default()
        };
        assert_eq!(pick_source(Some(&p), &game), RouteSource::Plan);
    }

    #[test]
    fn navroute_used_only_without_plan_and_current_as_last_resort() {
        let game = RouteState {
            hops: vec![RouteHop::default(); 3],
            ..Default::default()
        };
        assert_eq!(pick_source(None, &game), RouteSource::NavRoute);
        assert_eq!(
            pick_source(None, &RouteState::default()),
            RouteSource::Current
        );
    }

    #[test]
    fn active_plan_serde_roundtrip_resets_off_route() {
        let mut p = abcd();
        p.current_index = 1;
        p.off_route = true;
        let json = serde_json::to_string(&p).unwrap();
        let back: ActivePlan = serde_json::from_str(&json).unwrap();
        assert!(!back.off_route);
        p.off_route = false;
        assert_eq!(back, p);
    }

    #[test]
    fn reconcile_after_restore_sets_index_or_off_route() {
        let mut p = abcd();
        assert_eq!(p.reconcile("c", None), JumpOutcome::Advanced { skipped: 1 });
        assert_eq!(p.current_index, 2);
        assert_eq!(
            p.reconcile("C", Some(3)),
            JumpOutcome::Advanced { skipped: 0 }
        );
        assert_eq!(p.current_index, 2);
        let mut q = abcd();
        assert_eq!(q.reconcile("Zeta", Some(99)), JumpOutcome::OffRoute);
        assert!(q.off_route);
        assert_eq!(q.current_index, 0);
    }

    #[test]
    fn remaining_ly_sums_leg_distances_after_current() {
        let mut p = abcd(); // distancias 0,10,20,30
        assert_eq!(p.remaining_ly(), 60.0);
        p.on_jump("B", 2);
        assert_eq!(p.remaining_ly(), 50.0);
        assert_eq!(p.upcoming(1).len(), 1);
        assert_eq!(p.upcoming(10).len(), 2);
    }
}
