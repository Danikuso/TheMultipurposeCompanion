//! Módulo `ship`: catálogo embebido de astilleros/equipamiento y búsqueda inteligente
//! de estaciones cercanas con soporte de *Stronghold Carriers* de la potencia afiliada,
//! permisos de sistema y semáforo de seguro de recompra.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tmc_core::{Module, ModuleCtx, Relevance, TopicSet};
use tmc_model::events::EventKind;
use tmc_model::net::{
    NetFailure, NetOutcome, NetQuery, NetSource, OutfittingFallbackEntry, OutfittingModuleQuery,
    OutfittingSearchParams, OutfittingStationHit, PadSize,
};
use tmc_model::source::SourceEvent;
use tmc_model::state::{Galaxy, GameState};

pub const TOPIC_CATALOG: &str = "ship.outfitting_catalog";
pub const TOPIC_SEARCH: &str = "ship.outfitting_search";
pub const ACTION_SEARCH: &str = "ship.outfitting_search";
pub const ACTION_CLEAR: &str = "ship.outfitting_clear";

const TOPICS: [&str; 2] = [TOPIC_CATALOG, TOPIC_SEARCH];
const MAX_BASKET_MODULES: usize = 10;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct SearchActionArgs {
    reference_system: Option<String>,
    ship: Option<String>,
    modules: Vec<OutfittingModuleQuery>,
    pad_mode: Option<String>,
    only_lyr_discount: bool,
    include_own_stronghold: bool,
    include_carriers: bool,
    include_planetary: bool,
    exclude_permits: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnrichedOutfittingStation {
    #[serde(flatten)]
    pub hit: OutfittingStationHit,
    pub permit_key: Option<String>,
    pub permit_requires_elite: bool,
    pub affordability: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnrichedOutfittingFallback {
    pub item_label: String,
    pub nearest_station: Option<EnrichedOutfittingStation>,
}

#[derive(Debug, Clone)]
pub struct ShipModule {
    online_services: bool,
    current_system: String,
    current_star_pos: Option<[f64; 3]>,
    current_ship_pad: Option<PadSize>,
    pledged_power: Option<String>,
    has_elite_rank: bool,
    credits: Option<i64>,
    rebuy: Option<i64>,
    last_now: Option<DateTime<Utc>>,

    seq: u64,
    active_request_id: Option<u64>,
    status: &'static str,
    pad_mode: &'static str,
    exclude_permits: bool,
    active_params: Option<OutfittingSearchParams>,
    raw_stations: Vec<OutfittingStationHit>,
    raw_fallback: Vec<OutfittingFallbackEntry>,
    fetched_at: Option<DateTime<Utc>>,
    source: Option<NetSource>,
}

impl Default for ShipModule {
    fn default() -> Self {
        Self {
            online_services: true,
            current_system: String::new(),
            current_star_pos: None,
            current_ship_pad: None,
            pledged_power: None,
            has_elite_rank: false,
            credits: None,
            rebuy: None,
            last_now: None,
            seq: 0,
            active_request_id: None,
            status: "idle",
            pad_mode: "auto",
            exclude_permits: true,
            active_params: None,
            raw_stations: Vec::new(),
            raw_fallback: Vec::new(),
            fetched_at: None,
            source: None,
        }
    }
}

impl ShipModule {
    pub fn new() -> Self {
        Self::default()
    }

    fn compute_has_elite(state: &GameState) -> bool {
        let Some(cmdr) = state.cmdr.as_ref() else {
            return false;
        };
        let r = &cmdr.ranks;
        [
            r.combat,
            r.trade,
            r.explore,
            r.soldier,
            r.exobiologist,
            r.cqc,
        ]
        .into_iter()
        .flatten()
        .any(|rank| rank >= 8)
    }

    fn sync_from_state(&mut self, state: &GameState) -> (bool, bool) {
        let mut catalog_changed = false;
        let mut budget_changed = false;

        if self.current_system != state.location.system {
            self.current_system = state.location.system.clone();
            catalog_changed = true;
        }
        let next_pos = (!state.location.system.is_empty()).then_some(state.location.star_pos);
        if self.current_star_pos != next_pos {
            self.current_star_pos = next_pos;
        }

        let next_pad = state
            .ship
            .as_ref()
            .map(|s| s.ship_type.as_str())
            .filter(|s| !s.is_empty())
            .and_then(|st| crate::data::ship_pads().pad_for(st));
        if self.current_ship_pad != next_pad {
            self.current_ship_pad = next_pad;
            catalog_changed = true;
        }

        let next_elite = Self::compute_has_elite(state);
        if self.has_elite_rank != next_elite {
            self.has_elite_rank = next_elite;
            catalog_changed = true;
        }

        let next_credits = state
            .cmdr
            .as_ref()
            .and_then(|c| c.credits)
            .or(state.session.credits_now);
        if self.credits != next_credits {
            self.credits = next_credits;
            catalog_changed = true;
            budget_changed = true;
        }

        let next_rebuy = state.ship.as_ref().and_then(|s| s.rebuy);
        if self.rebuy != next_rebuy {
            self.rebuy = next_rebuy;
            catalog_changed = true;
            budget_changed = true;
        }

        (catalog_changed, budget_changed)
    }

    fn set_power(&mut self, next: Option<String>) -> bool {
        let cleaned = next.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        if self.pledged_power != cleaned {
            self.pledged_power = cleaned;
            true
        } else {
            false
        }
    }

    fn affordability_for(&self, total_price_cr: Option<u64>) -> &'static str {
        let (Some(total), Some(credits)) = (total_price_cr, self.credits) else {
            return "unknown";
        };
        let available = u64::try_from(credits.max(0)).unwrap_or(0);
        let rebuy = u64::try_from(self.rebuy.unwrap_or(0).max(0)).unwrap_or(0);
        if available < total {
            "insufficient"
        } else if available < total.saturating_add(rebuy) {
            "tight_rebuy"
        } else {
            "ok"
        }
    }

    fn enrich_station(&self, hit: &OutfittingStationHit) -> EnrichedOutfittingStation {
        let permit = crate::data::outfitting_catalog().permit_for(&hit.system_name);
        EnrichedOutfittingStation {
            hit: hit.clone(),
            permit_key: permit.map(|p| p.permit_key.clone()),
            permit_requires_elite: permit.is_some_and(|p| p.requires_elite),
            affordability: self.affordability_for(hit.total_price_cr),
        }
    }

    fn build_enriched_views(
        &self,
    ) -> (
        Vec<EnrichedOutfittingStation>,
        usize,
        Vec<EnrichedOutfittingFallback>,
    ) {
        let mut hidden_permit_count = 0usize;
        let mut stations = Vec::with_capacity(self.raw_stations.len());
        for hit in &self.raw_stations {
            let enriched = self.enrich_station(hit);
            if self.exclude_permits && enriched.permit_key.is_some() {
                hidden_permit_count += 1;
                continue;
            }
            stations.push(enriched);
        }

        let fallback = self
            .raw_fallback
            .iter()
            .map(|entry| {
                let nearest_station = entry
                    .nearest_station
                    .as_ref()
                    .map(|st| self.enrich_station(st))
                    .filter(|st| !(self.exclude_permits && st.permit_key.is_some()));
                EnrichedOutfittingFallback {
                    item_label: entry.item_label.clone(),
                    nearest_station,
                }
            })
            .collect();

        (stations, hidden_permit_count, fallback)
    }

    fn handle_search_action(
        &mut self,
        args: &Value,
        state: &GameState,
        ctx: &mut ModuleCtx,
    ) -> bool {
        if ctx.replaying {
            return false;
        }

        let parsed: SearchActionArgs = serde_json::from_value(args.clone()).unwrap_or_default();

        if state.galaxy == Galaxy::Legacy || !self.online_services {
            self.active_request_id = None;
            self.status = "offline";
            self.raw_stations.clear();
            self.raw_fallback.clear();
            return true;
        }

        let ship = parsed
            .ship
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let modules: Vec<OutfittingModuleQuery> = parsed
            .modules
            .into_iter()
            .filter(|m| !m.name.trim().is_empty())
            .take(MAX_BASKET_MODULES)
            .collect();

        if ship.is_none() && modules.is_empty() {
            self.active_request_id = None;
            self.status = "idle";
            self.raw_stations.clear();
            self.raw_fallback.clear();
            return true;
        }

        let reference_system = parsed
            .reference_system
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| self.current_system.trim().to_string());

        if reference_system.is_empty() && self.current_star_pos.is_none() {
            self.active_request_id = None;
            self.status = "missing_origin";
            self.raw_stations.clear();
            self.raw_fallback.clear();
            return true;
        }

        let reference_coords = if reference_system.is_empty()
            || reference_system.eq_ignore_ascii_case(self.current_system.trim())
        {
            self.current_star_pos
        } else {
            None
        };

        let (pad_mode, min_pad) = match parsed.pad_mode.as_deref().map(str::trim).unwrap_or("auto")
        {
            "S" | "s" | "Small" | "small" => ("S", PadSize::Small),
            "M" | "m" | "Medium" | "medium" => ("M", PadSize::Medium),
            "L" | "l" | "Large" | "large" => ("L", PadSize::Large),
            _ => {
                let auto_pad = if ship.is_some() {
                    PadSize::Large
                } else {
                    self.current_ship_pad.unwrap_or(PadSize::Medium)
                };
                ("auto", auto_pad)
            }
        };

        let exclude_permits = parsed.exclude_permits.unwrap_or(!self.has_elite_rank);
        let include_own_stronghold = parsed.include_own_stronghold && self.pledged_power.is_some();
        let pledged_power = if include_own_stronghold {
            self.pledged_power.clone()
        } else {
            None
        };

        let params = OutfittingSearchParams {
            reference_system,
            reference_coords,
            ship,
            modules,
            min_pad,
            only_lyr_discount: parsed.only_lyr_discount,
            include_own_stronghold,
            pledged_power,
            include_carriers: parsed.include_carriers,
            include_planetary: parsed.include_planetary,
        };

        self.seq = self.seq.saturating_add(1);
        let key = format!("outfitting:{}", self.seq);
        let Some(req_id) = ctx.request(key, NetQuery::OutfittingSearch(params.clone())) else {
            return false;
        };

        decide!("ship.outfitting.searched", {
            "has_ship": params.ship.is_some(),
            "modules": params.modules.len(),
            "own_stronghold": params.include_own_stronghold,
            "only_lyr": params.only_lyr_discount,
        });

        self.active_request_id = Some(req_id);
        self.status = "loading";
        self.pad_mode = pad_mode;
        self.exclude_permits = exclude_permits;
        self.active_params = Some(params);
        self.raw_stations.clear();
        self.raw_fallback.clear();
        self.fetched_at = None;
        self.source = None;
        true
    }
}

impl Module for ShipModule {
    fn id(&self) -> &'static str {
        "ship"
    }

    fn topics(&self) -> Vec<String> {
        TOPICS.iter().map(|s| (*s).to_string()).collect()
    }

    fn relevance(&self, _state: &GameState) -> Relevance {
        Relevance::Normal
    }

    fn on_event(&mut self, ev: &SourceEvent, state: &GameState, ctx: &mut ModuleCtx) -> TopicSet {
        let mut changed = TopicSet::new();
        let (cat_changed, budget_changed) = self.sync_from_state(state);
        if cat_changed {
            changed.insert(TOPIC_CATALOG.to_string());
        }
        if budget_changed && !self.raw_stations.is_empty() {
            changed.insert(TOPIC_SEARCH.to_string());
        }

        match ev {
            SourceEvent::Status(st) => {
                self.last_now = Some(st.timestamp);
            }
            SourceEvent::Settings(s) => {
                self.online_services = s.online_services;
            }
            SourceEvent::Journal { ev, .. } => {
                self.last_now = Some(ev.timestamp);
                let power_changed = match &ev.kind {
                    EventKind::Powerplay(pp) => self.set_power(Some(pp.power.clone())),
                    EventKind::PowerplayJoin(j) => self.set_power(Some(j.power.clone())),
                    EventKind::PowerplayDefect(d) => self.set_power(Some(d.to_power.clone())),
                    EventKind::PowerplayMerits(m) => self.set_power(Some(m.power.clone())),
                    EventKind::PowerplayLeave(_) => self.set_power(None),
                    _ => false,
                };
                if power_changed {
                    changed.insert(TOPIC_CATALOG.to_string());
                }
            }
            SourceEvent::Action { name, args } => match name.as_str() {
                ACTION_SEARCH => {
                    if self.handle_search_action(args, state, ctx) {
                        changed.insert(TOPIC_SEARCH.to_string());
                    }
                }
                ACTION_CLEAR => {
                    self.active_request_id = None;
                    self.status = "idle";
                    self.active_params = None;
                    self.raw_stations.clear();
                    self.raw_fallback.clear();
                    self.fetched_at = None;
                    self.source = None;
                    changed.insert(TOPIC_SEARCH.to_string());
                }
                _ => {}
            },
            SourceEvent::Net(resp) if Some(resp.id) == self.active_request_id => {
                self.fetched_at = Some(resp.fetched_at);
                self.source = Some(resp.source);
                match &resp.outcome {
                    NetOutcome::OutfittingStations(res) => {
                        self.raw_stations = res.stations.clone();
                        self.raw_fallback = res.fallback.clone();
                        let (visible, _, fallback) = self.build_enriched_views();
                        self.status = if !visible.is_empty() {
                            "ok"
                        } else if !fallback.is_empty() {
                            "fallback"
                        } else {
                            "no_data"
                        };
                    }
                    NetOutcome::NoData => {
                        self.raw_stations.clear();
                        self.raw_fallback.clear();
                        self.status = "no_data";
                    }
                    NetOutcome::Disabled => {
                        self.raw_stations.clear();
                        self.raw_fallback.clear();
                        self.status = "offline";
                    }
                    NetOutcome::Failed(NetFailure::UnknownOrigin) => {
                        self.raw_stations.clear();
                        self.raw_fallback.clear();
                        self.status = "unknown_origin";
                    }
                    NetOutcome::Failed(_) => {
                        self.raw_stations.clear();
                        self.raw_fallback.clear();
                        self.status = "error";
                    }
                    _ => return changed,
                }
                changed.insert(TOPIC_SEARCH.to_string());
            }
            _ => {}
        }

        changed
    }

    fn topic_value(&self, topic: &str) -> Option<Value> {
        match topic {
            TOPIC_CATALOG => {
                let cat = crate::data::outfitting_catalog();
                Some(json!({
                    "ships": cat.ships,
                    "modules": cat.modules,
                    "classes": cat.classes,
                    "ratings": cat.ratings,
                    "permit_systems": cat.permit_systems,
                    "commander": {
                        "current_system": self.current_system,
                        "current_ship_pad": self.current_ship_pad,
                        "pledged_power": self.pledged_power,
                        "has_elite_rank": self.has_elite_rank,
                        "credits": self.credits,
                        "rebuy": self.rebuy,
                    }
                }))
            }
            TOPIC_SEARCH => {
                let (stations, hidden_permit_count, fallback) = self.build_enriched_views();
                Some(json!({
                    "status": self.status,
                    "pad_mode": self.pad_mode,
                    "exclude_permits": self.exclude_permits,
                    "hidden_permit_count": hidden_permit_count,
                    "params": self.active_params,
                    "stations": stations,
                    "fallback": fallback,
                    "fetched_at": self.fetched_at,
                    "source": self.source,
                    "as_of": self.last_now,
                }))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
