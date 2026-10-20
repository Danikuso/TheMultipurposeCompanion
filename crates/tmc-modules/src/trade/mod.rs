//! Módulo `trade`: mercado actual, carga con coste pagado, sesión, ruta y búsqueda.
//!
//! Topics (todos con `as_of`, el último instante del diario): `trade.market`,
//! `trade.cargo`, `trade.route`, `trade.session`, `trade.search` y
//! `trade.commodities`. Se publican solo si cambian (`as_of` no cuenta).
//!
//! Tiempo: sin reloj propio; «ahora» es el último instante visto en el diario.
//!
//! Orden por evento: la sesión (`TradeSession`) se aplica ANTES que `CostBasis`,
//! porque una venta consume el coste medio que la sesión necesita leer.
//!
//! Red: precios de venta (`CommodityImportsNearby`) de la carga y del top del
//! mercado sin venta conocida en 48 h, y exportaciones al buscar. Nunca en replay,
//! Legacy ni con `online_services` desactivado. Las escrituras al almacén llevan
//! la etiqueta `*.write` y tampoco ocurren en replay.

mod cost;
mod ingest;
pub mod market;
mod route;
mod search;
mod session;

pub use cost::{CargoAdvice, CargoKind, CargoLine, CargoView, CostBasis, cost_rebuild_task};
pub use ingest::{ArdentRow, ardent_rows, ardent_write_task, commodities_task};
pub use route::{RouteBlocker, TradeRouteState, route_params};
pub use search::{BuyOption, SearchResult, merge_buys, search_task};
pub use session::TradeSession;

use crate::data::ship_pads;
use crate::inflight::Inflight;
use crate::mining::sell::{SellOption, filter_compatible};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use market::{CurrentMarket, best_sells, best_sells_task, snapshot_from_market_json};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use tmc_core::{Module, ModuleCtx, Relevance, TopicSet};
use tmc_model::companion::{CompanionFile, CompanionKind};
use tmc_model::events::{Docked, EventKind, JournalEvent, commodity_key};
use tmc_model::market::{MarketIdentity, MarketSnapshot, PricedMarket};
use tmc_model::net::{NetOutcome, NetQuery, NetResponse, PadSize, RemoteExport, RemoteMarket};
use tmc_model::settings::ModuleSettings;
use tmc_model::source::{EddnNotice, SourceEvent};
use tmc_model::state::{FlightMode, Galaxy, GameState};

const TOPIC_MARKET: &str = "trade.market";
const TOPIC_CARGO: &str = "trade.cargo";
const TOPIC_ROUTE: &str = "trade.route";
const TOPIC_SESSION: &str = "trade.session";
const TOPIC_SEARCH: &str = "trade.search";
const TOPIC_COMMODITIES: &str = "trade.commodities";
const TOPICS: [&str; 6] = [
    TOPIC_MARKET,
    TOPIC_CARGO,
    TOPIC_ROUTE,
    TOPIC_SESSION,
    TOPIC_SEARCH,
    TOPIC_COMMODITIES,
];

const TAG_COST: &str = "trade.cost";
const TAG_SELLS: &str = "trade.best_sells";
const TAG_SEARCH: &str = "trade.search";
const TAG_COMMODITIES: &str = "trade.commodities";
const TAG_MARKET_WRITE: &str = "trade.market.write";
const TAG_ARDENT_WRITE: &str = "trade.ardent.write";

const ACTION_RESET: &str = "trade.reset_session";
const ACTION_ROUTE_REQUEST: &str = "trade.route_request";
const ACTION_ROUTE_CLEAR: &str = "trade.route_clear";
const ACTION_SEARCH: &str = "trade.search";

const COST_KEY: &str = "cost";
const COMMODITIES_KEY: &str = "commodities";

/// Intervalo mínimo entre consultas provocadas por avisos EDDN.
const EDDN_REQUERY_MIN_S: i64 = 10;
/// Antigüedad máxima de un precio de venta (almacén y Ardent), en horas.
const PRICE_MAX_AGE_H: i64 = 48;
/// Ventana de la búsqueda de mercancía (igual que `search_task`), en días.
const SEARCH_WINDOW_DAYS: i64 = 7;
/// Ventana del coste pagado reconstruido, en días.
const COST_WINDOW_DAYS: i64 = 7;
/// Mercancías consultadas al almacén como máximo.
const MAX_VIEW_COMMODITIES: usize = 12;
/// Consultas a Ardent por evento como máximo.
const MAX_IMPORT_QUERIES: usize = 10;
/// Un resultado de venta se refresca pasado este tiempo.
const RESULT_MAX_AGE_MIN: i64 = 15;
/// Un `Failed` se reintenta pasado este tiempo.
const FAILED_COOLDOWN_S: i64 = 300;
/// Eventos de coste vistos antes de recibir el corte del almacén.
const PENDING_CAP: usize = 500;
/// Candidatos remotos por mercancía enviados a la lectura del almacén.
const MAX_CANDIDATES: usize = 50;
/// Paradas y mercancías por parada que se publican.
const MAX_STOP_COMMODITIES: usize = 8;
/// Tope de la serialización de `trade.commodities` (el topic debe caber en 16 KiB).
const COMMODITIES_MAX_BYTES: usize = 15_000;
const DEFAULT_MAX_HOPS: u8 = 5;
const MAX_HOPS: u8 = 10;
const SEARCH_ROWS: usize = 10;

type Pos = (String, String, u64);

struct PendingCost {
    kind: EventKind,
    pos: Pos,
}

/// El estado guarda la carga con el tipo del archivo compañero; el módulo trabaja con el del diario.
fn journal_cargo(items: &[tmc_model::companion::CargoItem]) -> Vec<tmc_model::events::CargoItem> {
    items
        .iter()
        .map(|i| tmc_model::events::CargoItem {
            name: i.name.clone(),
            name_localised: i.name_localised.clone(),
            count: i.count,
            stolen: Some(i.stolen),
            mission_id: i.mission_id,
        })
        .collect()
}

fn pad_tag(p: PadSize) -> &'static str {
    match p {
        PadSize::Small => "s",
        PadSize::Medium => "m",
        PadSize::Large => "l",
    }
}

pub struct Trade {
    settings: ModuleSettings,
    now: Option<DateTime<Utc>>,
    cmdr: Option<String>,
    basis: CostBasis,
    /// Carga de la nave (el estado solo la guarda desde `Cargo.json`).
    inventory: Vec<tmc_model::events::CargoItem>,
    cost_loaded: bool,
    pending: VecDeque<PendingCost>,
    session: TradeSession,
    route: TradeRouteState,
    route_status: &'static str,
    route_blocker: Option<RouteBlocker>,
    /// Alcance de salto usado en la última petición (para calcular saltos por parada).
    route_hop_range: Option<f64>,
    route_key: Option<String>,
    /// Último `Docked` propio: `LocationState` no lo conserva.
    docked: Option<Docked>,
    /// Último `Market.json` crudo: puede llegar antes que `Docked`.
    raw_market: Option<Value>,
    market: Option<MarketSnapshot>,
    /// Sistema para el que valen las respuestas de red conservadas.
    addr: u64,
    pad: Option<PadSize>,
    eddn_connected: bool,
    last_eddn_query: Option<DateTime<Utc>>,
    /// Contexto de la última lectura de ventas pedida al almacén.
    sells_sent: Option<Value>,
    sells_loaded: bool,
    local_sells: BTreeMap<String, Vec<PricedMarket>>,
    /// Ventas de Ardent del sistema actual (con `updated_at` ya resuelto).
    ardent_sells: BTreeMap<String, Vec<RemoteMarket>>,
    answered: HashMap<String, DateTime<Utc>>,
    failed: HashMap<String, DateTime<Utc>>,
    pending_imports: HashSet<String>,
    search_commodity: Option<String>,
    search_sent: Option<Value>,
    search_loaded: bool,
    search_local: Vec<PricedMarket>,
    search_exports: Vec<RemoteExport>,
    search_answered: bool,
    commodities: Vec<(String, Option<String>)>,
    commodities_loaded: bool,
    inflight: Inflight,
    values: HashMap<&'static str, Value>,
}

impl Trade {
    pub fn new() -> Self {
        Self {
            settings: ModuleSettings::default(),
            now: None,
            cmdr: None,
            basis: CostBasis::default(),
            inventory: Vec::new(),
            cost_loaded: false,
            pending: VecDeque::new(),
            session: TradeSession::default(),
            route: TradeRouteState::default(),
            route_status: "idle",
            route_blocker: None,
            route_hop_range: None,
            route_key: None,
            docked: None,
            raw_market: None,
            market: None,
            addr: 0,
            pad: None,
            eddn_connected: false,
            last_eddn_query: None,
            sells_sent: None,
            sells_loaded: false,
            local_sells: BTreeMap::new(),
            ardent_sells: BTreeMap::new(),
            answered: HashMap::new(),
            failed: HashMap::new(),
            pending_imports: HashSet::new(),
            search_commodity: None,
            search_sent: None,
            search_loaded: false,
            search_local: Vec::new(),
            search_exports: Vec::new(),
            search_answered: false,
            commodities: Vec::new(),
            commodities_loaded: false,
            inflight: Inflight::default(),
            values: HashMap::new(),
        }
    }

    fn tick(&mut self, at: DateTime<Utc>) {
        if self.now.is_none() {
            self.session.reset(at);
        }
        if self.now.is_none_or(|n| at > n) {
            self.now = Some(at);
        }
    }

    fn clock(&self) -> DateTime<Utc> {
        self.now.unwrap_or(DateTime::UNIX_EPOCH)
    }

    fn online(&self, state: &GameState) -> bool {
        self.settings.online_services && state.galaxy != Galaxy::Legacy
    }

    fn ship_pad(state: &GameState) -> Option<PadSize> {
        state
            .ship
            .as_ref()
            .and_then(|s| ship_pads().pad_for(&s.ship_type))
    }

    fn cur_pad(&self, state: &GameState) -> Option<PadSize> {
        Self::ship_pad(state).or(self.pad)
    }

    fn radius(&self) -> f64 {
        f64::from(self.settings.sell_search_radius_ly)
    }

    /// Alinea sistema y plataforma. Un cambio de sistema descarta lo remoto.
    fn sync(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        let addr = state.location.system_address;
        if addr != self.addr {
            self.addr = addr;
            self.clear_remote();
            self.search_exports.clear();
            self.search_local.clear();
            self.search_loaded = false;
            self.search_answered = false;
            self.search_sent = None;
            self.inflight.finish_prefix("exports:");
            self.inflight.finish_prefix("route:");
            if self.search_commodity.is_some() {
                self.run_search(state, ctx);
                self.request_exports(state, ctx);
            }
        }
        let pad = Self::ship_pad(state);
        if pad.is_some() && self.pad != pad {
            self.pad = pad;
            self.clear_remote();
        }
    }

    /// Vacía la ruta conservando `copy_seq`: es monótono para que el portapapeles
    /// copie el primer destino de un plan nuevo.
    fn reset_route(&mut self) {
        let seq = self.route.copy_seq;
        self.route = TradeRouteState::default();
        self.route.copy_seq = seq;
        self.route_status = "idle";
        self.route_blocker = None;
        self.route_hop_range = None;
        self.route_key = None;
        self.inflight.finish_prefix("route:");
    }

    fn clear_remote(&mut self) {
        self.ardent_sells.clear();
        self.answered.clear();
        self.failed.clear();
        self.pending_imports.clear();
        self.inflight.finish_prefix("sells:");
    }

    /// Un cambio de comandante lo descarta todo; el primero solo invalida lo cargado.
    fn check_cmdr(&mut self, state: &GameState) {
        let Some(name) = state
            .cmdr
            .as_ref()
            .map(|c| c.name.clone())
            .filter(|n| !n.is_empty())
        else {
            return;
        };
        if self.cmdr.as_deref() == Some(name.as_str()) {
            return;
        }
        let switched = self.cmdr.is_some();
        self.cmdr = Some(name);
        decide!("trade.cmdr.changed", {"switched": switched});
        self.basis = CostBasis::default();
        self.inventory.clear();
        self.cost_loaded = false;
        self.pending.clear();
        self.commodities_loaded = false;
        self.commodities.clear();
        self.inflight.finish(COST_KEY);
        self.inflight.finish(COMMODITIES_KEY);
        if switched {
            self.session = TradeSession::default();
            self.session.reset(self.clock());
            self.reset_route();
            self.docked = None;
            self.raw_market = None;
            self.market = None;
            self.sells_sent = None;
            self.sells_loaded = false;
            self.local_sells.clear();
            self.search_commodity = None;
            self.search_sent = None;
            self.search_loaded = false;
            self.search_local.clear();
            self.search_exports.clear();
            self.search_answered = false;
            self.clear_remote();
            self.inflight.finish_prefix("exports:");
        }
    }

    fn ensure_cost_task(&mut self, ctx: &mut ModuleCtx) {
        if ctx.replaying || self.cost_loaded {
            return;
        }
        let Some(cmdr) = self.cmdr.clone() else {
            return;
        };
        if !self.inflight.begin(COST_KEY, self.clock()) {
            return;
        }
        let since = self.clock() - Duration::days(COST_WINDOW_DAYS);
        ctx.store_task(TAG_COST, cost_rebuild_task(cmdr, since));
        decide!("trade.cost.rebuild.requested", {});
    }

    fn ensure_commodities_task(&mut self, ctx: &mut ModuleCtx) {
        if ctx.replaying || self.commodities_loaded {
            return;
        }
        let Some(cmdr) = self.cmdr.clone() else {
            return;
        };
        if !self.inflight.begin(COMMODITIES_KEY, self.clock()) {
            return;
        }
        ctx.store_task(TAG_COMMODITIES, commodities_task(cmdr));
        decide!("trade.commodities.requested", {});
    }

    /// Restaura `copy_seq` si el cambio no debe provocar una copia al portapapeles.
    fn guard_copy_seq(&mut self, seq: u64, replaying: bool) {
        if replaying || !self.settings.auto_copy_next_system {
            self.route.copy_seq = seq;
        }
    }

    fn on_journal(
        &mut self,
        je: &JournalEvent,
        file: &str,
        line: u64,
        replaying: bool,
        state: &GameState,
    ) {
        let at = je.timestamp;
        self.tick(at);
        self.check_cmdr(state);
        let now = self.clock();
        match &je.kind {
            EventKind::LoadGame(_) => {
                // Nueva partida: el coste pagado se vuelve a leer del almacén.
                self.cost_loaded = false;
                self.pending.clear();
                self.inventory.clear();
                self.inflight.finish(COST_KEY);
                self.clear_remote();
                self.sells_loaded = false;
            }
            EventKind::ShipyardSwap(_) => self.inventory.clear(),
            EventKind::Docked(d) => {
                let same = self.docked.as_ref().and_then(|p| p.market_id) == d.market_id;
                self.docked = Some(d.clone());
                if !same || self.market.as_ref().map(|m| m.market_id) != d.market_id {
                    self.market = None;
                    self.sells_sent = None;
                    self.sells_loaded = false;
                    self.local_sells.clear();
                }
                let seq = self.route.copy_seq;
                let advanced = self.route.on_docked(d);
                let _ = &advanced; // solo lo lee el diagnóstico
                self.guard_copy_seq(seq, replaying);
                decide!("trade.docked", {"has_market": d.has_market(), "advanced": advanced.is_some()});
            }
            EventKind::Cargo(c) => {
                if c.vessel.as_deref().is_none_or(|v| v == "Ship")
                    && let Some(inv) = &c.inventory
                {
                    self.inventory = inv.clone();
                }
            }
            EventKind::Undocked(_) => {
                self.docked = None;
                self.market = None;
                self.sells_sent = None;
                self.sells_loaded = false;
                self.local_sells.clear();
            }
            _ => {}
        }
        // Primero la sesión (lee el coste medio) y después el coste.
        self.session.apply_event(&je.kind, &self.basis, now);
        self.basis.apply_event(&je.kind);
        if !self.cost_loaded
            && matches!(
                je.kind,
                EventKind::MarketBuy(_)
                    | EventKind::MarketSell(_)
                    | EventKind::EjectCargo(_)
                    | EventKind::Cargo(_)
            )
        {
            if self.pending.len() >= PENDING_CAP {
                decide!("trade.cost.pending_dropped_cap", {"cap": PENDING_CAP});
                self.pending.pop_front();
            }
            self.pending.push_back(PendingCost {
                kind: je.kind.clone(),
                pos: (
                    at.to_rfc3339_opts(SecondsFormat::Nanos, true),
                    file.to_owned(),
                    line,
                ),
            });
        }
    }

    fn on_cost_stored(&mut self, value: &Value) -> bool {
        if self.cmdr.is_none()
            || value.get("cmdr").and_then(Value::as_str) != self.cmdr.as_deref()
            || self.cost_loaded
        {
            decide!("trade.cost.stored.discarded", {"loaded": self.cost_loaded});
            return false;
        }
        let Ok(basis) = serde_json::from_value::<CostBasis>(value["basis"].clone()) else {
            decide!("trade.cost.stored.unreadable", {});
            return false;
        };
        let cut_pos: Option<Pos> = value.get("cut_pos").and_then(|p| {
            Some((
                p.get("ts")?.as_str()?.to_owned(),
                p.get("file")?.as_str()?.to_owned(),
                p.get("line")?.as_u64()?,
            ))
        });
        self.basis = basis;
        for p in std::mem::take(&mut self.pending) {
            if cut_pos.as_ref().is_none_or(|c| p.pos > *c) {
                self.basis.apply_event(&p.kind);
            }
        }
        self.inflight.finish(COST_KEY);
        self.cost_loaded = true;
        decide!("trade.cost.stored.applied", {"cut_pos": cut_pos.is_some()});
        true
    }

    // ----- Carga y mercado -----

    /// Mercancías comerciables de la carga (sin misión, robadas ni limpets), ordenadas.
    fn cargo_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .inventory
            .iter()
            .filter(|i| {
                !i.is_mission() && i.count > i.stolen_count() && commodity_key(&i.name) != "drones"
            })
            .map(|i| commodity_key(&i.name))
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    /// Mercancías de interés: la carga y, si caben, las más caras del mercado.
    fn view_commodities(&self) -> Vec<String> {
        let mut keys = self.cargo_keys();
        keys.truncate(MAX_VIEW_COMMODITIES);
        if let Some(m) = &self.market {
            let mut offers: Vec<_> = m
                .prices
                .iter()
                .filter(|p| p.buy_price > 0 && p.stock > 0)
                .collect();
            offers.sort_by(|a, b| {
                b.buy_price
                    .cmp(&a.buy_price)
                    .then(a.commodity.cmp(&b.commodity))
            });
            for p in offers {
                if keys.len() >= MAX_VIEW_COMMODITIES {
                    break;
                }
                if !keys.contains(&p.commodity) {
                    keys.push(p.commodity.clone());
                }
            }
        }
        keys
    }

    /// Ventas de Ardent del sistema actual con menos de 48 h.
    fn fresh_ardent(&self, key: &str) -> Vec<RemoteMarket> {
        let min = self.clock() - Duration::hours(PRICE_MAX_AGE_H);
        self.ardent_sells
            .get(key)
            .map(|v| {
                v.iter()
                    .filter(|m| m.updated_at.is_some_and(|at| at >= min))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    fn sells_map(&self, state: &GameState) -> BTreeMap<String, Vec<SellOption>> {
        let pad = self.cur_pad(state);
        self.view_commodities()
            .into_iter()
            .map(|k| {
                let local = self.local_sells.get(&k).map(Vec::as_slice).unwrap_or(&[]);
                let options = best_sells(local, &self.fresh_ardent(&k), pad, 5);
                (k, options)
            })
            .collect()
    }

    fn current_market(&self, state: &GameState) -> Option<CurrentMarket> {
        self.market_with(&self.sells_map(state))
    }

    fn market_with(&self, sells: &BTreeMap<String, Vec<SellOption>>) -> Option<CurrentMarket> {
        let snap = self.market.as_ref()?;
        Some(CurrentMarket::build(snap, sells, self.clock()))
    }

    /// Pide al almacén las mejores ventas conocidas de las mercancías de interés.
    fn request_sells(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        if ctx.replaying || state.galaxy == Galaxy::Legacy {
            return;
        }
        let (Some(cmdr), Some(market_id)) =
            (self.cmdr.clone(), self.market.as_ref().map(|m| m.market_id))
        else {
            return;
        };
        let commodities = self.view_commodities();
        let center = state.location.star_pos;
        if commodities.is_empty() || !center.iter().all(|c| c.is_finite()) {
            self.sells_sent = None;
            return;
        }
        let radius = self.radius();
        let since = self.clock() - Duration::hours(PRICE_MAX_AGE_H);
        let candidates: BTreeMap<String, Vec<MarketIdentity>> = commodities
            .iter()
            .filter_map(|k| {
                let ids: Vec<MarketIdentity> = self
                    .fresh_ardent(k)
                    .into_iter()
                    .take(MAX_CANDIDATES)
                    .map(|m| MarketIdentity {
                        market_id: m.market_id,
                        station_name: m.station_name,
                        system_name: m.system_name,
                    })
                    .collect();
                (!ids.is_empty()).then(|| (k.clone(), ids))
            })
            .collect();
        let ship_pad = self.cur_pad(state);
        self.sells_sent = Some(json!({
            "cmdr": cmdr, "commodities": commodities, "center": center,
            "radius_ly": radius, "since": since, "exclude_market": market_id,
            "candidates": candidates, "ship_pad": ship_pad,
        }));
        ctx.store_task(
            TAG_SELLS,
            best_sells_task(
                cmdr,
                commodities.clone(),
                center,
                radius,
                since,
                market_id,
                candidates,
                ship_pad,
            ),
        );
        decide!("trade.sells.requested", {"commodities": commodities.len()});
    }

    /// Repite la lectura si cambió el conjunto de mercancías de interés.
    fn sync_sells(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        if ctx.replaying || state.galaxy == Galaxy::Legacy || self.market.is_none() {
            return;
        }
        let current = self.view_commodities();
        let sent = self
            .sells_sent
            .as_ref()
            .and_then(|s| s.get("commodities"))
            .cloned();
        if sent != Some(json!(current)) {
            self.request_sells(state, ctx);
        }
    }

    fn on_sells_stored(&mut self, value: &Value) -> bool {
        let Some(sent) = self.sells_sent.as_ref().and_then(Value::as_object) else {
            decide!("trade.sells.stored.discarded", {"reason": 0});
            return false;
        };
        if self.cmdr.is_none()
            || value.get("cmdr").and_then(Value::as_str) != self.cmdr.as_deref()
            || sent.iter().any(|(k, v)| value.get(k) != Some(v))
        {
            decide!("trade.sells.stored.discarded", {"reason": 1});
            return false;
        }
        let Ok(rows) =
            serde_json::from_value::<BTreeMap<String, Vec<PricedMarket>>>(value["rows"].clone())
        else {
            decide!("trade.sells.stored.discarded", {"reason": 2});
            return false;
        };
        decide!("trade.sells.stored.applied", {"commodities": rows.len()});
        self.local_sells = rows;
        self.sells_loaded = true;
        true
    }

    /// Pide a Ardent los compradores de la carga y del top del mercado sin venta reciente.
    fn maybe_request_imports(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        let addr = state.location.system_address;
        if ctx.replaying || addr == 0 || !self.sells_loaded {
            return;
        }
        if !self.online(state) {
            decide!("trade.imports.skipped_offline", {});
            return;
        }
        let sells = self.sells_map(state);
        let mut keys = self.cargo_keys();
        if let Some(m) = self.current_market(state) {
            for row in &m.rows {
                if sells.get(&row.commodity).is_none_or(Vec::is_empty)
                    && !keys.contains(&row.commodity)
                {
                    keys.push(row.commodity.clone());
                }
            }
        }
        keys.truncate(MAX_IMPORT_QUERIES);
        let now = self.clock();
        let pad = self.cur_pad(state).unwrap_or(PadSize::Small);
        for key in keys {
            if self
                .answered
                .get(&key)
                .is_some_and(|at| now - *at < Duration::minutes(RESULT_MAX_AGE_MIN))
                || self.failed.get(&key).is_some_and(|retry| now < *retry)
            {
                continue;
            }
            let rk = format!("sells:{addr}:{key}:{}", pad_tag(pad));
            if !self.inflight.begin(&rk, now) {
                decide!("trade.imports.inflight_dedup", {});
                continue;
            }
            let query = NetQuery::CommodityImportsNearby {
                system_address: addr,
                commodity: key.clone(),
                max_distance_ly: self.settings.sell_search_radius_ly,
                min_pad: pad,
            };
            let accepted = ctx.request(rk.clone(), query).is_some();
            if accepted {
                self.pending_imports.insert(key);
            } else {
                self.inflight.finish(&rk);
            }
            decide!("trade.imports.sent", {"accepted": accepted});
        }
    }

    /// Escribe (en vivo) las filas de Ardent con coordenadas conocidas.
    fn write_ardent(&self, ctx: &mut ModuleCtx, state: &GameState, rows: Vec<ArdentRow>) {
        if ctx.replaying || !self.online(state) || rows.is_empty() {
            return;
        }
        ctx.store_task(TAG_ARDENT_WRITE, ardent_write_task(rows));
    }

    fn parse_key(key: &str, prefix: &str) -> Option<(u64, String, String)> {
        let rest = key.strip_prefix(prefix)?;
        let (addr, rest) = rest.split_once(':')?;
        let (commodity, pad) = rest.rsplit_once(':')?;
        Some((addr.parse().ok()?, commodity.to_owned(), pad.to_owned()))
    }

    /// Devuelve `true` si cambió algo visible.
    fn apply_sells_response(
        &mut self,
        r: &NetResponse,
        state: &GameState,
        ctx: &mut ModuleCtx,
    ) -> bool {
        let Some((addr, commodity, pad)) = Self::parse_key(&r.key, "sells:") else {
            return false;
        };
        self.inflight.finish(&r.key);
        let key = commodity_key(&commodity);
        let cur = self.cur_pad(state).unwrap_or(PadSize::Small);
        if pad != pad_tag(cur) || addr != state.location.system_address || addr != self.addr {
            decide!("trade.imports.response.stale", {});
            return false;
        }
        let pending = self.pending_imports.remove(&key);
        let now = self.clock();
        let changed = match &r.outcome {
            NetOutcome::Markets(ms) => {
                let ms: Vec<RemoteMarket> = filter_compatible(ms, self.cur_pad(state))
                    .into_iter()
                    .map(|mut m| {
                        m.updated_at.get_or_insert(r.fetched_at);
                        m
                    })
                    .collect();
                decide!("trade.imports.response.applied", {"candidates": ms.len()});
                let rows = ardent_rows(&ms, &[], &key, r.fetched_at);
                self.write_ardent(ctx, state, rows);
                self.ardent_sells.insert(key.clone(), ms);
                self.answered.insert(key, now);
                true
            }
            NetOutcome::NoData => {
                decide!("trade.imports.response.no_data", {});
                self.ardent_sells.insert(key.clone(), Vec::new());
                self.answered.insert(key, now);
                true
            }
            NetOutcome::Failed(_) => {
                decide!("trade.imports.response.failed", {"cooldown_s": FAILED_COOLDOWN_S});
                self.failed
                    .insert(key, now + Duration::seconds(FAILED_COOLDOWN_S));
                false
            }
            _ => false,
        };
        // Con todas las respuestas pendientes recibidas, el almacén reconcilia los cierres.
        if pending && self.pending_imports.is_empty() {
            self.request_sells(state, ctx);
        }
        changed
    }

    // ----- Búsqueda -----

    fn run_search(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        let (Some(commodity), Some(cmdr)) = (self.search_commodity.clone(), self.cmdr.clone())
        else {
            return;
        };
        if ctx.replaying || state.galaxy == Galaxy::Legacy {
            return;
        }
        let center = state.location.star_pos;
        if !center.iter().all(|c| c.is_finite()) {
            return;
        }
        let radius = self.radius();
        let now = self.clock();
        let candidates: Vec<MarketIdentity> = self
            .search_exports
            .iter()
            .take(MAX_CANDIDATES)
            .map(|e| MarketIdentity {
                market_id: e.market_id,
                station_name: e.station_name.clone(),
                system_name: e.system_name.clone(),
            })
            .collect();
        let ship_pad = self.cur_pad(state);
        let since = now - Duration::days(SEARCH_WINDOW_DAYS);
        self.search_sent = Some(json!({
            "cmdr": cmdr, "commodity": commodity, "center": center, "radius_ly": radius,
            "since": since, "candidates": candidates, "ship_pad": ship_pad,
        }));
        ctx.store_task(
            TAG_SEARCH,
            search_task(cmdr, commodity, center, radius, candidates, ship_pad, now),
        );
        decide!("trade.search.requested", {"candidates": self.search_exports.len()});
    }

    fn request_exports(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        let addr = state.location.system_address;
        let Some(commodity) = self.search_commodity.clone() else {
            return;
        };
        if ctx.replaying || state.galaxy == Galaxy::Legacy || addr == 0 {
            return;
        }
        if !self.online(state) {
            decide!("trade.search.skipped_offline", {});
            return;
        }
        let pad = self.cur_pad(state).unwrap_or(PadSize::Small);
        let rk = format!("exports:{addr}:{commodity}:{}", pad_tag(pad));
        if !self.inflight.begin(&rk, self.clock()) {
            return;
        }
        let query = NetQuery::CommodityExportsNearby {
            system_address: addr,
            commodity,
            max_distance_ly: self.settings.sell_search_radius_ly,
            min_pad: pad,
        };
        let accepted = ctx.request(rk.clone(), query).is_some();
        if !accepted {
            self.inflight.finish(&rk);
        }
        decide!("trade.search.exports.sent", {"accepted": accepted});
    }

    fn start_search(&mut self, commodity: &str, state: &GameState, ctx: &mut ModuleCtx) {
        let key = commodity_key(commodity);
        self.search_local.clear();
        self.search_exports.clear();
        self.search_loaded = false;
        self.search_answered = false;
        self.search_sent = None;
        self.inflight.finish_prefix("exports:");
        if key.is_empty() {
            self.search_commodity = None;
            decide!("trade.search.cleared", {});
            return;
        }
        self.search_commodity = Some(key);
        self.run_search(state, ctx);
        self.request_exports(state, ctx);
    }

    fn on_search_stored(&mut self, value: &Value) -> bool {
        let Some(sent) = self.search_sent.as_ref().and_then(Value::as_object) else {
            decide!("trade.search.stored.discarded", {"reason": 0});
            return false;
        };
        if self.cmdr.is_none()
            || value.get("cmdr").and_then(Value::as_str) != self.cmdr.as_deref()
            || sent.iter().any(|(k, v)| value.get(k) != Some(v))
        {
            decide!("trade.search.stored.discarded", {"reason": 1});
            return false;
        }
        let Ok(rows) = serde_json::from_value::<Vec<PricedMarket>>(value["rows"].clone()) else {
            decide!("trade.search.stored.discarded", {"reason": 2});
            return false;
        };
        decide!("trade.search.stored.applied", {"rows": rows.len()});
        self.search_local = rows;
        self.search_loaded = true;
        true
    }

    fn apply_exports_response(
        &mut self,
        r: &NetResponse,
        state: &GameState,
        ctx: &mut ModuleCtx,
    ) -> bool {
        let Some((addr, commodity, pad)) = Self::parse_key(&r.key, "exports:") else {
            return false;
        };
        self.inflight.finish(&r.key);
        let cur = self.cur_pad(state).unwrap_or(PadSize::Small);
        if pad != pad_tag(cur)
            || addr != state.location.system_address
            || addr != self.addr
            || self.search_commodity.as_deref() != Some(commodity_key(&commodity).as_str())
        {
            decide!("trade.search.exports.stale", {});
            return false;
        }
        match &r.outcome {
            NetOutcome::Exports(v) => {
                let exports: Vec<RemoteExport> = v
                    .iter()
                    .cloned()
                    .map(|mut e| {
                        e.updated_at.get_or_insert(r.fetched_at);
                        e
                    })
                    .collect();
                decide!("trade.search.exports.applied", {"rows": exports.len()});
                let rows = ardent_rows(&[], &exports, &commodity, r.fetched_at);
                self.write_ardent(ctx, state, rows);
                self.search_exports = exports;
                self.search_answered = true;
                // Los candidatos remotos se reconcilian por identidad en el almacén.
                self.run_search(state, ctx);
                true
            }
            NetOutcome::NoData => {
                decide!("trade.search.exports.no_data", {});
                self.search_answered = true;
                true
            }
            NetOutcome::Failed(_) => {
                decide!("trade.search.exports.failed", {});
                self.search_answered = true;
                true
            }
            _ => false,
        }
    }

    // ----- Ruta -----

    fn route_request(&mut self, max_hops: u8, state: &GameState, ctx: &mut ModuleCtx) {
        if ctx.replaying || !self.online(state) {
            decide!("trade.route.request.skipped", {"online": self.online(state)});
            return;
        }
        let pad = self.cur_pad(state);
        match route_params(state, pad, max_hops) {
            Err(blocker) => {
                self.route_status = "blocked";
                self.route_blocker = Some(blocker);
            }
            Ok(params) => {
                self.route_blocker = None;
                let hop_range = params.max_hop_distance_ly;
                let tag = pad_tag(pad.unwrap_or(PadSize::Large));
                let rk = format!("route:{}:{tag}", state.location.system_address);
                self.inflight.finish_prefix("route:");
                self.inflight.begin(&rk, self.clock());
                let accepted = ctx
                    .request(rk.clone(), NetQuery::TradeRoute(params))
                    .is_some();
                if accepted {
                    self.route_key = Some(rk);
                    self.route_hop_range = Some(hop_range);
                    self.route_status = "loading";
                    self.route.requested_at = Some(self.clock());
                } else {
                    self.inflight.finish(&rk);
                }
                decide!("trade.route.request.sent", {"accepted": accepted, "max_hops": max_hops});
            }
        }
    }

    /// Una ruta pedida desde otro sistema de origen se acepta: el plan es válido
    /// desde su origen y `on_docked` lo hace avanzar al llegar a sus paradas.
    /// Solo se descartan las respuestas cuya clave no es la de la última petición.
    fn apply_route_response(&mut self, r: &NetResponse) -> bool {
        if self.route_key.as_deref() != Some(r.key.as_str()) {
            decide!("trade.route.response.stale", {});
            return false;
        }
        self.inflight.finish(&r.key);
        match &r.outcome {
            NetOutcome::TradeRoute(plan) if !plan.stops.is_empty() => {
                let seq = self.route.copy_seq;
                self.route.on_plan(plan.clone());
                self.guard_copy_seq(seq, false);
                self.route_status = "ok";
            }
            NetOutcome::TradeRoute(_) | NetOutcome::NoData | NetOutcome::Failed(_) => {
                decide!("trade.route.response.empty", {});
                self.route_status = "no_data";
            }
            _ => return false,
        }
        true
    }

    // ----- Render -----

    fn stamp(&self, mut v: Value) -> Value {
        if let Some(o) = v.as_object_mut() {
            o.insert("as_of".into(), json!(self.now));
        }
        v
    }

    fn render_market(&self, sells: &BTreeMap<String, Vec<SellOption>>) -> Value {
        let mut v = self
            .market_with(sells)
            .and_then(|m| serde_json::to_value(m).ok())
            .unwrap_or_else(|| json!({"station": null, "system": null, "rows": []}));
        if let Some(o) = v.as_object_mut() {
            o.insert("eddn_connected".into(), json!(self.eddn_connected));
        }
        self.stamp(v)
    }

    fn render_cargo(&self, state: &GameState, sells: &BTreeMap<String, Vec<SellOption>>) -> Value {
        let here: HashMap<String, u64> = self
            .market
            .as_ref()
            .map(|m| {
                m.prices
                    .iter()
                    .filter(|p| p.sell_price > 0)
                    .map(|p| (p.commodity.clone(), p.sell_price))
                    .collect()
            })
            .unwrap_or_default();
        let best: HashMap<String, SellOption> = sells
            .iter()
            .filter_map(|(k, v)| v.first().map(|o| (k.clone(), o.clone())))
            .collect();
        let capacity = state.ship.as_ref().and_then(|s| s.cargo_capacity);
        let view = CargoView::build(&self.inventory, &self.basis, &here, &best, capacity);
        self.stamp(serde_json::to_value(view).unwrap_or_default())
    }

    fn render_session(&self) -> Value {
        self.stamp(serde_json::to_value(&self.session).unwrap_or_default())
    }

    fn render_route(&self, state: &GameState) -> Value {
        let mut v = serde_json::to_value(&self.route).unwrap_or_default();
        if let Some(stops) = v
            .get_mut("plan")
            .and_then(|p| p.get_mut("stops"))
            .and_then(Value::as_array_mut)
        {
            for stop in stops {
                let jumps = stop
                    .get("distance_ly")
                    .and_then(Value::as_f64)
                    .zip(self.route_hop_range)
                    .map(|(d, r)| (d / r).ceil().max(1.0) as u32);
                if let Some(o) = stop.as_object_mut() {
                    o.insert("jumps".into(), json!(jumps));
                }
                if let Some(c) = stop.get_mut("commodities").and_then(Value::as_array_mut) {
                    c.truncate(MAX_STOP_COMMODITIES);
                }
            }
        }
        if let Some(o) = v.as_object_mut() {
            let status = if self.online(state) {
                self.route_status
            } else {
                "disabled"
            };
            o.insert("status".into(), json!(status));
            o.insert("blocker".into(), json!(self.route_blocker));
        }
        self.stamp(v)
    }

    fn render_search(&self, state: &GameState) -> Value {
        let Some(commodity) = &self.search_commodity else {
            return self.stamp(json!({"commodity": null, "status": "idle", "rows": []}));
        };
        let exports = &self.search_exports;
        let rows = merge_buys(
            &self.search_local,
            exports,
            self.cur_pad(state),
            SEARCH_ROWS,
        );
        let waiting_net = self.online(state) && !self.search_answered;
        let status = if !rows.is_empty() {
            "ok"
        } else if !self.search_loaded || waiting_net {
            "loading"
        } else {
            "no_data"
        };
        let result = SearchResult {
            commodity: commodity.clone(),
            rows,
        };
        self.stamp(json!({"commodity": result.commodity, "rows": result.rows, "status": status}))
    }

    fn render_commodities(&self) -> Value {
        let mut items: Vec<Value> = self
            .commodities
            .iter()
            .take(400)
            .map(|(k, n)| json!([k, n]))
            .collect();
        while serde_json::to_string(&items).map_or(0, |s| s.len()) > COMMODITIES_MAX_BYTES {
            items.pop();
        }
        self.stamp(json!({"items": items}))
    }

    fn set(&mut self, topic: &'static str, v: Value, dirty: &mut TopicSet) {
        crate::topicx::set_topic(&mut self.values, topic, v, dirty);
    }

    /// Docked conocido: el del diario o, si el módulo arrancó ya atracado, el de la ubicación.
    fn dock_context(&self, state: &GameState) -> Option<Docked> {
        if let Some(d) = &self.docked {
            return Some(d.clone());
        }
        let loc = &state.location;
        (loc.mode == FlightMode::Docked && loc.station.is_some()).then(|| Docked {
            station_name: loc.station.clone(),
            station_type: loc.station_type.clone(),
            market_id: loc.market_id,
            star_system: Some(loc.system.clone()),
            system_address: Some(loc.system_address),
            ..Docked::default()
        })
    }

    /// Procesa el último `Market.json` si encaja con la estación actual.
    fn try_market(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        let (Some(raw), Some(d)) = (&self.raw_market, self.dock_context(state)) else {
            return;
        };
        let Some(snap) = snapshot_from_market_json(raw, &d, state.location.star_pos, self.clock())
        else {
            decide!("trade.market.rejected", {});
            return;
        };
        decide!("trade.market.loaded", {"prices": snap.prices.len()});
        if !ctx.replaying && state.galaxy != Galaxy::Legacy {
            let s = snap.clone();
            ctx.store_task(
                TAG_MARKET_WRITE,
                Box::new(move |store| {
                    let applied = store.replace_markets(&[s])?;
                    Ok(json!({"applied": applied}))
                }),
            );
        }
        self.market = Some(snap);
        self.commodities_loaded = false;
        self.ensure_commodities_task(ctx);
        self.request_sells(state, ctx);
    }

    fn on_eddn(&mut self, n: &EddnNotice, state: &GameState, ctx: &mut ModuleCtx) {
        self.eddn_connected = n.connected;
        if ctx.replaying || self.market.is_none() {
            return;
        }
        let view = self.view_commodities();
        if !n
            .commodities
            .iter()
            .any(|c| view.contains(&commodity_key(c)))
        {
            return;
        }
        if self
            .last_eddn_query
            .is_some_and(|t| (n.at - t).num_seconds() < EDDN_REQUERY_MIN_S)
        {
            decide!("trade.eddn.throttled", {});
            return;
        }
        self.last_eddn_query = Some(n.at);
        self.request_sells(state, ctx);
    }
}

impl Default for Trade {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Trade {
    fn id(&self) -> &'static str {
        "trade"
    }

    fn on_event(&mut self, ev: &SourceEvent, state: &GameState, ctx: &mut ModuleCtx) -> TopicSet {
        self.sync(state, ctx);
        match ev {
            SourceEvent::Settings(s) => {
                let enabling = s.online_services && !self.settings.online_services;
                let radius = s.sell_search_radius_ly != self.settings.sell_search_radius_ly;
                self.settings = s.clone();
                if radius || enabling {
                    self.clear_remote();
                    self.request_sells(state, ctx);
                    self.maybe_request_imports(state, ctx);
                    if self.search_commodity.is_some() {
                        self.run_search(state, ctx);
                        self.request_exports(state, ctx);
                    }
                }
            }
            SourceEvent::Journal {
                ev: je, file, line, ..
            } => {
                self.on_journal(je, file, *line, ctx.replaying, state);
                self.ensure_cost_task(ctx);
                self.ensure_commodities_task(ctx);
                let relevant = matches!(
                    je.kind,
                    EventKind::Commander(_)
                        | EventKind::LoadGame(_)
                        | EventKind::Docked(_)
                        | EventKind::Undocked(_)
                        | EventKind::Location(_)
                        | EventKind::FsdJump(_)
                        | EventKind::CarrierJump(_)
                        | EventKind::Cargo(_)
                        | EventKind::MarketBuy(_)
                        | EventKind::MarketSell(_)
                        | EventKind::EjectCargo(_)
                        | EventKind::Rank(_)
                        | EventKind::Progress(_)
                        | EventKind::ShipyardSwap(_)
                );
                if !relevant {
                    return TopicSet::new();
                }
                if self.market.is_none()
                    && matches!(je.kind, EventKind::Docked(_) | EventKind::Location(_))
                {
                    self.try_market(state, ctx);
                }
                self.sync_sells(state, ctx);
                if matches!(
                    je.kind,
                    EventKind::Cargo(_)
                        | EventKind::Location(_)
                        | EventKind::FsdJump(_)
                        | EventKind::CarrierJump(_)
                ) {
                    self.maybe_request_imports(state, ctx);
                }
            }
            SourceEvent::Companion(CompanionFile::Cargo(f)) => {
                if f.vessel.as_deref().is_none_or(|v| v == "Ship") {
                    self.inventory = journal_cargo(&f.inventory);
                }
                self.sync_sells(state, ctx);
                self.maybe_request_imports(state, ctx);
            }
            SourceEvent::Companion(CompanionFile::Raw(CompanionKind::Market, v)) => {
                self.raw_market = Some(v.clone());
                self.try_market(state, ctx);
            }
            SourceEvent::Eddn(n) => self.on_eddn(n, state, ctx),
            SourceEvent::Stored { module, tag, value } if module == "trade" => {
                if ctx.replaying {
                    return TopicSet::new();
                }
                let applied = match tag.as_str() {
                    TAG_COST => self.on_cost_stored(value),
                    TAG_SELLS => {
                        let ok = self.on_sells_stored(value);
                        if ok {
                            self.maybe_request_imports(state, ctx);
                        }
                        ok
                    }
                    TAG_SEARCH => self.on_search_stored(value),
                    TAG_COMMODITIES => {
                        if self.cmdr.is_some()
                            && value.get("cmdr").and_then(Value::as_str) == self.cmdr.as_deref()
                            && let Ok(items) = serde_json::from_value::<Vec<(String, Option<String>)>>(
                                value["items"].clone(),
                            )
                        {
                            self.commodities = items;
                            self.commodities_loaded = true;
                            self.inflight.finish(COMMODITIES_KEY);
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                };
                if !applied {
                    return TopicSet::new();
                }
            }
            SourceEvent::Net(r) if r.module == "trade" => {
                let applied = if r.key.starts_with("sells:") {
                    self.apply_sells_response(r, state, ctx)
                } else if r.key.starts_with("exports:") {
                    self.apply_exports_response(r, state, ctx)
                } else if r.key.starts_with("route:") {
                    self.apply_route_response(r)
                } else {
                    false
                };
                if !applied {
                    return TopicSet::new();
                }
            }
            SourceEvent::Action { name, args } => match name.as_str() {
                ACTION_RESET => {
                    let now = self.clock();
                    self.session.reset(now);
                    decide!("trade.session.reset.applied", {});
                }
                ACTION_ROUTE_CLEAR => {
                    self.reset_route();
                    decide!("trade.route.cleared", {});
                }
                ACTION_ROUTE_REQUEST => {
                    let hops = args
                        .get("max_hops")
                        .and_then(Value::as_u64)
                        .map_or(DEFAULT_MAX_HOPS, |h| h.clamp(1, u64::from(MAX_HOPS)) as u8);
                    self.route_request(hops, state, ctx);
                }
                ACTION_SEARCH => {
                    let commodity = args
                        .get("commodity")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    self.start_search(&commodity, state, ctx);
                }
                _ => return TopicSet::new(),
            },
            _ => return TopicSet::new(),
        }
        let mut dirty = TopicSet::new();
        let sells = self.sells_map(state);
        let v = self.render_market(&sells);
        self.set(TOPIC_MARKET, v, &mut dirty);
        let v = self.render_cargo(state, &sells);
        self.set(TOPIC_CARGO, v, &mut dirty);
        let v = self.render_route(state);
        self.set(TOPIC_ROUTE, v, &mut dirty);
        let v = self.render_session();
        self.set(TOPIC_SESSION, v, &mut dirty);
        let v = self.render_search(state);
        self.set(TOPIC_SEARCH, v, &mut dirty);
        let v = self.render_commodities();
        self.set(TOPIC_COMMODITIES, v, &mut dirty);
        dirty
    }

    fn topic_value(&self, topic: &str) -> Option<Value> {
        self.values.get(topic).cloned()
    }

    fn topics(&self) -> Vec<String> {
        TOPICS.iter().map(|t| (*t).to_owned()).collect()
    }

    fn relevance(&self, state: &GameState) -> Relevance {
        let at_market = state.location.mode == FlightMode::Docked
            && self.docked.as_ref().is_some_and(Docked::has_market);
        if at_market || !self.cargo_keys().is_empty() {
            Relevance::Active
        } else {
            Relevance::Normal
        }
    }
}

#[cfg(test)]
mod tests;
