//! Módulo `bgs`: seguimiento de facciones estelares locales, influencias relativas y sincronización del tick galáctico.

use chrono::{DateTime, Utc};
use serde_json::json;
use tmc_core::{Module, ModuleCtx, Relevance, TopicSet};
use tmc_model::events::{EventKind, FactionInfo};
use tmc_model::net::{NetOutcome, NetQuery};
use tmc_model::source::SourceEvent;
use tmc_model::state::GameState;

pub const TOPIC_SYSTEM: &str = "bgs.system";
pub const TOPIC_TICK: &str = "bgs.tick";
const TOPICS: [&str; 2] = [TOPIC_SYSTEM, TOPIC_TICK];

#[derive(Debug, Clone, Default)]
pub struct Bgs {
    pub star_system: Option<String>,
    pub system_address: Option<u64>,
    pub controlling_faction: Option<String>,
    pub controlling_faction_state: Option<String>,
    pub factions: Vec<FactionInfo>,

    // Tick galáctico
    pub latest_tick: Option<DateTime<Utc>>,
    pub tick_updated_at: Option<DateTime<Utc>>,
    pub last_now: Option<DateTime<Utc>>,
    pub tick_requested: bool,
}

impl Bgs {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Module for Bgs {
    fn id(&self) -> &'static str {
        "bgs"
    }

    fn topics(&self) -> Vec<String> {
        TOPICS.iter().map(|s| s.to_string()).collect()
    }

    fn relevance(&self, _state: &GameState) -> Relevance {
        if self.star_system.is_none() {
            Relevance::Hidden
        } else if self.factions.iter().any(|f| {
            f.faction_state.as_deref().is_some_and(|s| {
                matches!(
                    s,
                    "War" | "CivilWar" | "Election" | "CivilUnrest" | "Lockdown"
                )
            })
        }) {
            Relevance::Active
        } else {
            Relevance::Normal
        }
    }

    fn on_event(&mut self, ev: &SourceEvent, _state: &GameState, ctx: &mut ModuleCtx) -> TopicSet {
        let mut changed = TopicSet::new();

        if !self.tick_requested && !ctx.replaying {
            ctx.request("bgs:tick".to_string(), NetQuery::BgsTick);
            self.tick_requested = true;
        }

        match ev {
            SourceEvent::Status(status) => {
                self.last_now = Some(status.timestamp);
            }
            SourceEvent::Net(resp) if resp.key == "bgs:tick" => {
                if let NetOutcome::BgsTick(info) = &resp.outcome {
                    self.latest_tick = Some(info.latest_tick);
                    self.tick_updated_at = Some(info.updated_at);
                    changed.insert(TOPIC_TICK.to_string());
                }
            }
            SourceEvent::Journal { ev, .. } => {
                self.last_now = Some(ev.timestamp);

                match &ev.kind {
                    EventKind::FsdJump(loc)
                    | EventKind::Location(loc)
                    | EventKind::CarrierJump(loc) => {
                        self.star_system = Some(loc.star_system.clone());
                        self.system_address = Some(loc.system_address);
                        self.controlling_faction =
                            loc.system_faction.as_ref().and_then(|f| f.name.clone());
                        self.controlling_faction_state = loc
                            .system_faction
                            .as_ref()
                            .and_then(|f| f.faction_state.clone());

                        let mut sorted_factions = loc.factions.clone();
                        sorted_factions.sort_by(|a, b| {
                            b.influence
                                .partial_cmp(&a.influence)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        self.factions = sorted_factions;

                        changed.insert(TOPIC_SYSTEM.to_string());
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        changed
    }

    fn topic_value(&self, topic: &str) -> Option<serde_json::Value> {
        let now = self.last_now.unwrap_or_else(Utc::now);

        match topic {
            TOPIC_SYSTEM => {
                let factions_json: Vec<_> = self
                    .factions
                    .iter()
                    .map(|f| {
                        let is_controlling = self
                            .controlling_faction
                            .as_deref()
                            .is_some_and(|cf| cf == f.name);
                        let influence_pct = f
                            .influence
                            .map(|i| ((i * 100.0) * 10.0).round() / 10.0)
                            .unwrap_or(0.0);

                        json!({
                            "name": f.name,
                            "influence_percent": influence_pct,
                            "state": f.faction_state,
                            "allegiance": f.allegiance,
                            "government": f.government,
                            "happiness": f.happiness_localised,
                            "is_controlling": is_controlling,
                        })
                    })
                    .collect();

                Some(json!({
                    "star_system": self.star_system,
                    "system_address": self.system_address,
                    "controlling_faction": self.controlling_faction,
                    "controlling_faction_state": self.controlling_faction_state,
                    "factions_count": self.factions.len(),
                    "factions": factions_json,
                }))
            }
            TOPIC_TICK => {
                let seconds_since_tick = self
                    .latest_tick
                    .map(|tick| now.signed_duration_since(tick).num_seconds().max(0));
                let hours_since_tick =
                    seconds_since_tick.map(|sec| (sec as f64 / 3600.0 * 10.0).round() / 10.0);
                let is_stale = seconds_since_tick.is_some_and(|sec| sec > 28 * 3600);

                Some(json!({
                    "latest_tick": self.latest_tick,
                    "updated_at": self.tick_updated_at,
                    "seconds_since_tick": seconds_since_tick,
                    "hours_since_tick": hours_since_tick,
                    "is_stale": is_stale,
                }))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
