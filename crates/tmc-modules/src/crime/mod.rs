//! Módulo `crime`: seguimiento de notoriedad, decaimiento en vuelo, multas,
//! contrabando y localización de Interstellar Factors y Mercados Negros.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde_json::json;
use tmc_core::{Module, ModuleCtx, Relevance, TopicSet};
use tmc_model::crime::{
    CrimeStatusTopic, FactionFineBounty, NearbyFacility, StolenCargoItem, SystemSecurityLevel,
};
use tmc_model::events::EventKind;
use tmc_model::net::{NearbyService, NetOutcome, NetQuery, NetResponse, PadSize};
use tmc_model::source::SourceEvent;
use tmc_model::state::GameState;
use tmc_model::status::Flags;

pub const TOPIC_CRIME_STATUS: &str = "crime.status";
const TOPICS: [&str; 1] = [TOPIC_CRIME_STATUS];

/// Duración en segundos de vuelo activo necesaria para decaer 1 nivel de notoriedad.
pub const NOTORIETY_DECAY_INTERVAL_SECONDS: f64 = 7200.0;

#[derive(Debug, Clone)]
pub struct Crime {
    pub legal_state: String,
    pub notoriety: u8,
    pub flight_time_seconds: f64,
    pub last_timestamp: Option<DateTime<Utc>>,
    pub is_docked: bool,
    pub total_fines: u64,
    pub total_bounties: u64,
    pub factions: HashMap<String, FactionFineBounty>,
    pub stolen_items: Vec<StolenCargoItem>,
    pub nearest_interstellar_factor: Option<NearbyFacility>,
    pub nearest_black_market: Option<NearbyFacility>,
    pub system_security: SystemSecurityLevel,
    pub current_system: Option<String>,
    pub system_address: u64,
    if_requested: bool,
    bm_requested: bool,
}

impl Default for Crime {
    fn default() -> Self {
        Self {
            legal_state: "Clean".to_string(),
            notoriety: 0,
            flight_time_seconds: 0.0,
            last_timestamp: None,
            is_docked: false,
            total_fines: 0,
            total_bounties: 0,
            factions: HashMap::new(),
            stolen_items: Vec::new(),
            nearest_interstellar_factor: None,
            nearest_black_market: None,
            system_security: SystemSecurityLevel::Unknown,
            current_system: None,
            system_address: 0,
            if_requested: false,
            bm_requested: false,
        }
    }
}

impl Crime {
    pub fn new() -> Self {
        Self::default()
    }

    /// Segundos estimados para que la notoriedad decaiga a 0.
    pub fn decay_seconds_remaining(&self) -> u32 {
        if self.notoriety == 0 {
            0
        } else {
            let total_needed = (self.notoriety as f64) * NOTORIETY_DECAY_INTERVAL_SECONDS;
            (total_needed - self.flight_time_seconds).max(0.0) as u32
        }
    }

    /// Añade tiempo de vuelo activo (segundos) y descuenta niveles de notoriedad según corresponda.
    pub fn advance_flight_time(&mut self, dt_seconds: f64) -> bool {
        if self.is_docked || self.notoriety == 0 || dt_seconds <= 0.0 {
            return false;
        }

        self.flight_time_seconds += dt_seconds;
        let mut decayed = false;
        while self.flight_time_seconds >= NOTORIETY_DECAY_INTERVAL_SECONDS && self.notoriety > 0 {
            self.notoriety = self.notoriety.saturating_sub(1);
            self.flight_time_seconds -= NOTORIETY_DECAY_INTERVAL_SECONDS;
            decayed = true;
        }
        decayed
    }

    fn maybe_request_facilities(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        if ctx.replaying || self.system_address == 0 {
            return;
        }

        // Buscar Interstellar Factors si hay multas o recompensas
        if (self.total_fines > 0 || self.total_bounties > 0) && !self.if_requested {
            let query = NetQuery::NearestService {
                system_address: self.system_address,
                system_name: state.location.system.clone(),
                service: NearbyService::InterstellarFactors,
                min_pad: PadSize::Large,
            };
            if ctx
                .request(format!("if:{}", self.system_address), query)
                .is_some()
            {
                self.if_requested = true;
            }
        }

        // Buscar Mercado Negro si hay mercancías robadas en bodega
        if !self.stolen_items.is_empty() && !self.bm_requested {
            let query = NetQuery::NearestService {
                system_address: self.system_address,
                system_name: state.location.system.clone(),
                service: NearbyService::BlackMarket,
                min_pad: PadSize::Large,
            };
            if ctx
                .request(format!("bm:{}", self.system_address), query)
                .is_some()
            {
                self.bm_requested = true;
            }
        }
    }

    fn apply_net_response(&mut self, resp: &NetResponse) -> bool {
        if let NetOutcome::Stations(ref stations) = resp.outcome
            && let Some(target) = stations.first()
        {
            let facility = NearbyFacility {
                station_name: target.station_name.clone(),
                system_name: target.system_name.clone(),
                distance_ly: target.distance_ly,
                pad_size: target
                    .max_pad
                    .map(|p| format!("{p:?}"))
                    .unwrap_or_else(|| "L".to_string()),
                is_planetary: false,
            };

            if resp.key.starts_with("if:") {
                self.nearest_interstellar_factor = Some(facility);
                return true;
            } else if resp.key.starts_with("bm:") {
                self.nearest_black_market = Some(facility);
                return true;
            }
        }
        false
    }
}

impl Module for Crime {
    fn id(&self) -> &'static str {
        "crime"
    }

    fn topics(&self) -> Vec<String> {
        TOPICS.iter().map(|s| s.to_string()).collect()
    }

    fn relevance(&self, state: &GameState) -> Relevance {
        if state.location.system_address == 0 {
            Relevance::Hidden
        } else if self.notoriety > 0
            || self.total_bounties > 0
            || self.total_fines > 0
            || !self.stolen_items.is_empty()
            || self.legal_state != "Clean"
        {
            Relevance::Active
        } else {
            Relevance::Normal
        }
    }

    fn on_event(&mut self, ev: &SourceEvent, state: &GameState, ctx: &mut ModuleCtx) -> TopicSet {
        let mut changed = TopicSet::new();

        match ev {
            SourceEvent::Status(status) => {
                if let Some(ref ls) = status.legal_state
                    && self.legal_state != *ls
                {
                    self.legal_state = ls.clone();
                    changed.insert(TOPIC_CRIME_STATUS.to_string());
                }

                let was_docked = self.is_docked;
                self.is_docked = status.flags.contains(Flags::DOCKED);

                if let Some(prev) = self.last_timestamp {
                    let dt = (status.timestamp - prev).num_milliseconds() as f64 / 1000.0;
                    if dt > 0.0 && dt <= 60.0 && self.advance_flight_time(dt) {
                        changed.insert(TOPIC_CRIME_STATUS.to_string());
                    }
                }

                if was_docked != self.is_docked {
                    changed.insert(TOPIC_CRIME_STATUS.to_string());
                }

                self.last_timestamp = Some(status.timestamp);
                self.maybe_request_facilities(state, ctx);
            }

            SourceEvent::Journal { ev, .. } => {
                self.last_timestamp = Some(ev.timestamp);

                match &ev.kind {
                    EventKind::CommitCrime(crime) => {
                        if crime.crime_type.increases_notoriety() {
                            self.notoriety = (self.notoriety + 1).min(10);
                        }

                        if let Some(fine) = crime.fine {
                            self.total_fines = self.total_fines.saturating_add(fine);
                            let entry =
                                self.factions
                                    .entry(crime.faction.clone())
                                    .or_insert_with(|| FactionFineBounty {
                                        faction: crime.faction.clone(),
                                        system: self.current_system.clone(),
                                        fines: 0,
                                        bounties: 0,
                                    });
                            entry.fines = entry.fines.saturating_add(fine);
                        }

                        if let Some(bounty) = crime.bounty {
                            self.total_bounties = self.total_bounties.saturating_add(bounty);
                            let entry =
                                self.factions
                                    .entry(crime.faction.clone())
                                    .or_insert_with(|| FactionFineBounty {
                                        faction: crime.faction.clone(),
                                        system: self.current_system.clone(),
                                        fines: 0,
                                        bounties: 0,
                                    });
                            entry.bounties = entry.bounties.saturating_add(bounty);
                        }

                        changed.insert(TOPIC_CRIME_STATUS.to_string());
                        self.maybe_request_facilities(state, ctx);
                    }

                    EventKind::PayFines(pf) => {
                        if pf.all_fines {
                            self.total_fines = 0;
                            for f in self.factions.values_mut() {
                                f.fines = 0;
                            }
                        } else if let Some(ref fac) = pf.faction {
                            if let Some(f) = self.factions.get_mut(fac) {
                                f.fines = f.fines.saturating_sub(pf.amount);
                            }
                            self.total_fines = self.total_fines.saturating_sub(pf.amount);
                        }
                        changed.insert(TOPIC_CRIME_STATUS.to_string());
                    }

                    EventKind::PayBounties(pb) => {
                        if pb.all_bounties {
                            self.total_bounties = 0;
                            for f in self.factions.values_mut() {
                                f.bounties = 0;
                            }
                        } else if let Some(ref fac) = pb.faction {
                            if let Some(f) = self.factions.get_mut(fac) {
                                f.bounties = f.bounties.saturating_sub(pb.amount);
                            }
                            self.total_bounties = self.total_bounties.saturating_sub(pb.amount);
                        }
                        changed.insert(TOPIC_CRIME_STATUS.to_string());
                    }

                    EventKind::Resurrect => {
                        // Al reaparecer tras muerte, las recompensas activas de la nave se cancelan
                        self.total_bounties = 0;
                        for f in self.factions.values_mut() {
                            f.bounties = 0;
                        }
                        changed.insert(TOPIC_CRIME_STATUS.to_string());
                    }

                    EventKind::Cargo(c) => {
                        self.stolen_items = c
                            .inventory
                            .as_deref()
                            .unwrap_or(&[])
                            .iter()
                            .filter(|item| item.stolen_count() > 0)
                            .map(|item| StolenCargoItem {
                                name: item.name.clone(),
                                name_localised: item.name_localised.clone(),
                                count: item.count,
                                stolen: item.stolen_count(),
                                is_mission: item.is_mission(),
                            })
                            .collect();
                        changed.insert(TOPIC_CRIME_STATUS.to_string());
                        self.maybe_request_facilities(state, ctx);
                    }

                    EventKind::FsdJump(loc)
                    | EventKind::Location(loc)
                    | EventKind::CarrierJump(loc) => {
                        let sys_changed = self.system_address != loc.system_address;
                        self.current_system = Some(loc.star_system.clone());
                        self.system_address = loc.system_address;
                        let sec_str = loc
                            .system_security
                            .as_deref()
                            .or(loc.system_security_localised.as_deref())
                            .unwrap_or("");
                        self.system_security = SystemSecurityLevel::from(sec_str);

                        if sys_changed {
                            self.if_requested = false;
                            self.bm_requested = false;
                            self.nearest_interstellar_factor = None;
                            self.nearest_black_market = None;
                        }

                        changed.insert(TOPIC_CRIME_STATUS.to_string());
                        self.maybe_request_facilities(state, ctx);
                    }

                    _ => {}
                }
            }

            SourceEvent::Net(resp) if self.apply_net_response(resp) => {
                changed.insert(TOPIC_CRIME_STATUS.to_string());
            }

            _ => {}
        }

        changed
    }

    fn topic_value(&self, topic: &str) -> Option<serde_json::Value> {
        if topic == TOPIC_CRIME_STATUS {
            let topic_obj = CrimeStatusTopic {
                legal_state: self.legal_state.clone(),
                notoriety: self.notoriety,
                notoriety_decay_seconds: self.decay_seconds_remaining(),
                total_fines: self.total_fines,
                total_bounties: self.total_bounties,
                factions: self.factions.values().cloned().collect(),
                stolen_items: self.stolen_items.clone(),
                nearest_interstellar_factor: self.nearest_interstellar_factor.clone(),
                nearest_black_market: self.nearest_black_market.clone(),
                system_security: self.system_security,
                police_response_estimate_seconds: self
                    .system_security
                    .estimated_police_response_seconds(),
            };
            Some(json!(topic_obj))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests;
