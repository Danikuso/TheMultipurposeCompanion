//! Módulo `colonisation`: seguimiento del depósito de obra, progreso de construcción y cálculo de viajes restantes.

use serde_json::json;
use tmc_core::{Module, ModuleCtx, Relevance, TopicSet};
use tmc_model::colonisation::ColonisationConstructionDepot;
use tmc_model::events::EventKind;
use tmc_model::source::SourceEvent;
use tmc_model::state::GameState;

pub const TOPIC_DEPOT: &str = "colonisation.depot";
const TOPICS: [&str; 1] = [TOPIC_DEPOT];

#[derive(Debug, Clone, Default)]
pub struct Colonisation {
    pub active_depot: Option<ColonisationConstructionDepot>,
    pub cargo_capacity: u32,
    pub is_docked: bool,
}

impl Colonisation {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Module for Colonisation {
    fn id(&self) -> &'static str {
        "colonisation"
    }

    fn topics(&self) -> Vec<String> {
        TOPICS.iter().map(|s| s.to_string()).collect()
    }

    fn relevance(&self, _state: &GameState) -> Relevance {
        if self.active_depot.is_none() {
            Relevance::Hidden
        } else if self.is_docked {
            Relevance::Active
        } else {
            Relevance::Normal
        }
    }

    fn on_event(&mut self, ev: &SourceEvent, state: &GameState, _ctx: &mut ModuleCtx) -> TopicSet {
        let mut changed = TopicSet::new();

        if let Some(ship) = &state.ship
            && let Some(cap) = ship.cargo_capacity
            && cap > 0
            && cap != self.cargo_capacity
        {
            self.cargo_capacity = cap;
            if self.active_depot.is_some() {
                changed.insert(TOPIC_DEPOT.to_string());
            }
        }

        if let SourceEvent::Journal { ev, .. } = ev {
            match &ev.kind {
                EventKind::ColonisationConstructionDepot(depot) => {
                    self.active_depot = Some((**depot).clone());
                    changed.insert(TOPIC_DEPOT.to_string());
                }
                EventKind::ColonisationContribution(contrib) => {
                    if let Some(depot) = &mut self.active_depot {
                        let matches_market = contrib.market_id.is_none()
                            || depot.market_id.is_none()
                            || contrib.market_id == depot.market_id;

                        if matches_market {
                            for r in &mut depot.resources_required {
                                let match_name = r.name.eq_ignore_ascii_case(&contrib.commodity);
                                let match_loc = r
                                    .name_localised
                                    .as_deref()
                                    .is_some_and(|l| l.eq_ignore_ascii_case(&contrib.commodity));

                                if match_name || match_loc {
                                    r.provided_amount =
                                        r.provided_amount.saturating_add(contrib.amount);
                                    break;
                                }
                            }
                            let total_req = depot.total_required_tonnes();
                            let total_prov = depot.total_provided_tonnes();
                            if total_req > 0 {
                                depot.construction_progress =
                                    (total_prov as f64 / total_req as f64).clamp(0.0, 1.0);
                            }
                            changed.insert(TOPIC_DEPOT.to_string());
                        }
                    }
                }
                EventKind::Docked(docked) => {
                    let is_depot = docked.station_type.as_deref() == Some("ConstructionDepot")
                        || self.active_depot.as_ref().is_some_and(|d| {
                            d.market_id.is_some_and(|mid| Some(mid) == docked.market_id)
                                || d.station_name.as_deref().is_some_and(|sn| {
                                    docked.station_name.as_deref().is_some_and(|dsn| dsn == sn)
                                })
                        });
                    if is_depot && !self.is_docked {
                        self.is_docked = true;
                        changed.insert(TOPIC_DEPOT.to_string());
                    }
                }
                EventKind::Undocked(_) => {
                    if self.is_docked {
                        self.is_docked = false;
                        changed.insert(TOPIC_DEPOT.to_string());
                    }
                }
                EventKind::Location(loc) => {
                    let is_depot = loc.docked
                        && (loc.station_type.as_deref() == Some("ConstructionDepot")
                            || self.active_depot.as_ref().is_some_and(|d| {
                                d.market_id.is_some_and(|mid| Some(mid) == loc.market_id)
                                    || d.station_name.as_deref().is_some_and(|sn| {
                                        loc.station_name.as_deref().is_some_and(|lsn| lsn == sn)
                                    })
                            }));
                    if is_depot != self.is_docked {
                        self.is_docked = is_depot;
                        changed.insert(TOPIC_DEPOT.to_string());
                    }
                }
                _ => {}
            }
        }

        changed
    }

    fn topic_value(&self, topic: &str) -> Option<serde_json::Value> {
        if topic != TOPIC_DEPOT {
            return None;
        }

        let Some(depot) = &self.active_depot else {
            return Some(json!({
                "active_project": false,
                "market_id": null,
                "station_name": null,
                "construction_type": null,
                "progress_percent": 0.0,
                "total_required": 0,
                "total_provided": 0,
                "total_remaining": 0,
                "cargo_capacity": self.cargo_capacity,
                "total_trips_remaining": 0,
                "is_docked": self.is_docked,
                "resources": []
            }));
        };

        let total_req = depot.total_required_tonnes();
        let total_prov = depot.total_provided_tonnes();
        let total_rem = depot.total_remaining_tonnes();
        let total_trips = depot.total_trips_remaining(self.cargo_capacity);
        let progress_pct = if total_req > 0 {
            (total_prov as f64 / total_req as f64 * 100.0).clamp(0.0, 100.0)
        } else {
            depot.construction_progress * 100.0
        };

        let resources: Vec<_> = depot
            .resources_required
            .iter()
            .map(|r| {
                json!({
                    "name": r.name,
                    "name_localised": r.name_localised,
                    "required": r.required_amount,
                    "provided": r.provided_amount,
                    "remaining": r.remaining(),
                    "progress_percent": r.progress_percent(),
                    "trips_remaining": r.trips_remaining(self.cargo_capacity),
                })
            })
            .collect();

        Some(json!({
            "active_project": true,
            "market_id": depot.market_id,
            "station_name": depot.station_name,
            "construction_type": depot.construction_type,
            "progress_percent": progress_pct,
            "total_required": total_req,
            "total_provided": total_prov,
            "total_remaining": total_rem,
            "cargo_capacity": self.cargo_capacity,
            "total_trips_remaining": total_trips,
            "is_docked": self.is_docked,
            "resources": resources,
        }))
    }
}

#[cfg(test)]
mod tests;
