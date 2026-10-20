//! Módulo `mining`: prospector, sesión de minería, precios y «dónde vender».
//!
//! Topics: `mining.prospector` (`current` + `history` de hasta 8, el más reciente
//! primero), `mining.session` y `mining.sell` (`status` + `SellPlan` aplanado).
//!
//! Tiempo: sin reloj propio; «ahora» es el último instante visto en el diario. Los
//! topics se recalculan en cada evento y solo se marcan sucios si su valor cambió
//! (`Status` se ignora: la sesión avanza con el diario).
//!
//! Red: por cada mineral de la carga (máx. 10) y por cada material de interés del
//! asteroide actual sin precio, una consulta `sell:<dirección>:<materia>`. Una
//! respuesta correcta alimenta siempre al [`PriceBook`] (ya filtrada por la
//! plataforma de la nave) pero solo entra en el plan de venta si es del sistema
//! actual. Nunca en replay, Legacy ni con `online_services` desactivado.

pub mod prices;
pub mod prospector;
pub mod sell;
pub mod session;

use crate::data::ship_pads;
use crate::inflight::Inflight;
use chrono::{DateTime, Duration, Utc};
use prices::{PriceBook, price_rebuild_task};
use prospector::ProspectorTracker;
use sell::{SellPlan, filter_compatible};
use serde::Serialize;
use serde_json::{Value, json};
use session::MiningSession;
use std::collections::{BTreeMap, HashMap};
use tmc_core::{Module, ModuleCtx, Relevance, TopicSet};
use tmc_model::companion::{CompanionFile, CompanionKind};
use tmc_model::events::{EventKind, JournalEvent, commodity_key};
use tmc_model::net::{NetOutcome, NetQuery, NetResponse, PadSize, RemoteMarket};
use tmc_model::settings::ModuleSettings;
use tmc_model::source::SourceEvent;
use tmc_model::state::{Galaxy, GameState};

const TOPIC_PROSPECTOR: &str = "mining.prospector";
const TOPIC_SESSION: &str = "mining.session";
const TOPIC_SELL: &str = "mining.sell";
const TOPICS: [&str; 3] = [TOPIC_PROSPECTOR, TOPIC_SESSION, TOPIC_SELL];

const TAG_PRICES: &str = "mining.prices";
const PRICES_KEY: &str = "prices";
const ACTION_RESET: &str = "mining.reset_session";
/// Minerales de la carga consultados (y mostrados) como máximo.
const MAX_CARGO_QUERIES: usize = 10;
/// Un resultado de venta se refresca pasado este tiempo (igual que la vigencia
/// del precio remoto en el libro).
const RESULT_MAX_AGE_MIN: i64 = 15;
/// Un `Failed` se reintenta pasado este tiempo.
const FAILED_COOLDOWN_S: i64 = 300;
/// Ventana de actividad tras lanzar un limpet o prospectar.
const ACTIVE_WINDOW_MIN: i64 = 10;
/// Re-render de la sesión sin eventos de minería (para las tasas por hora).
const SESSION_REFRESH_S: i64 = 30;

fn pad_tag(p: PadSize) -> &'static str {
    match p {
        PadSize::Small => "s",
        PadSize::Medium => "m",
        PadSize::Large => "l",
    }
}

#[derive(Serialize)]
struct SellTopic<'a> {
    status: &'static str,
    /// Último instante del diario (base de tiempo de la UI); `None` sin eventos.
    as_of: Option<DateTime<Utc>>,
    names: &'a BTreeMap<String, String>,
    #[serde(flatten)]
    plan: Option<&'a SellPlan>,
}

/// Máximo de nombres localizados recordados.
const NAMES_MAX: usize = 300;

pub struct Mining {
    settings: ModuleSettings,
    now: Option<DateTime<Utc>>,
    cmdr: Option<String>,
    prices: PriceBook,
    /// Los últimos precios de venta ya vienen del almacén.
    prices_loaded: bool,
    prospector: ProspectorTracker,
    session: MiningSession,
    /// Toneladas por clave de materia en la bodega de la nave (sin limpets).
    inventory: BTreeMap<String, u32>,
    last_drone: Option<DateTime<Utc>>,
    last_prospect: Option<DateTime<Utc>>,
    /// Sistema para el que valen `results`, `answered` y `failed`.
    addr: u64,
    /// Última plataforma conocida de la nave.
    pad: Option<PadSize>,
    last_session_render: Option<DateTime<Utc>>,
    results: BTreeMap<String, Vec<RemoteMarket>>,
    answered: HashMap<String, DateTime<Utc>>,
    failed: HashMap<String, DateTime<Utc>>,
    fetched_at: Option<DateTime<Utc>>,
    inflight: Inflight,
    values: HashMap<&'static str, Value>,
    /// Clave de materia -> nombre localizado, aprendido de los eventos.
    names: BTreeMap<String, String>,
}

impl Mining {
    pub fn new() -> Self {
        Self {
            settings: ModuleSettings::default(),
            now: None,
            cmdr: None,
            prices: PriceBook::default(),
            prices_loaded: false,
            prospector: ProspectorTracker::default(),
            session: MiningSession::default(),
            inventory: BTreeMap::new(),
            last_drone: None,
            last_prospect: None,
            addr: 0,
            pad: None,
            last_session_render: None,
            results: BTreeMap::new(),
            answered: HashMap::new(),
            failed: HashMap::new(),
            fetched_at: None,
            inflight: Inflight::default(),
            values: HashMap::new(),
            names: BTreeMap::new(),
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

    /// Aprende el nombre localizado de una materia (ignora vacíos).
    fn learn(&mut self, raw: &str, localised: Option<&str>) {
        let Some(name) = localised.map(str::trim).filter(|n| !n.is_empty()) else {
            return;
        };
        let key = commodity_key(raw);
        if key.is_empty() {
            return;
        }
        if self.names.len() >= NAMES_MAX && !self.names.contains_key(&key) {
            return;
        }
        self.names.insert(key, name.to_owned());
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

    /// Alinea el sistema y la plataforma con el estado. Devuelve `true` si los
    /// precios remotos cambiaron (la nave cambió de plataforma).
    fn sync(&mut self, state: &GameState) -> bool {
        let addr = state.location.system_address;
        if addr != self.addr {
            self.addr = addr;
            self.results.clear();
            self.answered.clear();
            self.failed.clear();
            self.fetched_at = None;
        }
        // Una nave desconocida (p. ej. entre eventos) no cuenta como cambio.
        let pad = Self::ship_pad(state);
        let changed = pad.is_some() && self.pad != pad;
        if changed {
            // Los mercados guardados se filtraron para otra plataforma.
            self.prices.clear_remote();
            self.clear_results();
            self.pad = pad;
        }
        changed
    }

    fn clear_results(&mut self) {
        self.results.clear();
        self.answered.clear();
        self.failed.clear();
        self.fetched_at = None;
        self.inflight.finish_prefix("sell:");
    }

    /// Plataforma de la nave: la actual o, si se desconoce un instante, la última.
    fn cur_pad(&self, state: &GameState) -> Option<PadSize> {
        Self::ship_pad(state).or(self.pad)
    }

    /// Un cambio de comandante descarta lo aprendido de su historial.
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
        self.prices = PriceBook::default();
        self.prices_loaded = false;
        self.inflight.finish(PRICES_KEY);
        if switched {
            // Otro comandante: nada de lo aprendido sirve.
            self.session = MiningSession::default();
            self.session.reset(self.clock());
            self.inventory.clear();
            self.prospector = ProspectorTracker::default();
            self.last_drone = None;
            self.last_prospect = None;
            self.clear_results();
        }
    }

    /// Pide la reconstrucción de las últimas ventas (solo en vivo, una vez por
    /// comandante; si no llega el resultado en `INFLIGHT_TTL_S` se repite).
    fn ensure_prices_task(&mut self, ctx: &mut ModuleCtx) {
        if ctx.replaying || self.prices_loaded {
            return;
        }
        let Some(cmdr) = self.cmdr.clone() else {
            return;
        };
        if !self.inflight.begin(PRICES_KEY, self.clock()) {
            return;
        }
        ctx.store_task(TAG_PRICES, price_rebuild_task(cmdr));
        decide!(
            "mining.prices.rebuild.requested",
            tmc_model::diagnostics::json!({})
        );
    }

    fn on_journal(&mut self, je: &JournalEvent, state: &GameState) {
        let at = je.timestamp;
        self.tick(at);
        self.check_cmdr(state);
        let now = self.clock();
        match &je.kind {
            EventKind::ProspectedAsteroid(p) => {
                for m in &p.materials {
                    self.learn(&m.name, m.name_localised.as_deref());
                }
                let book = &self.prices;
                let f = |k: &str| book.best_price(k, now);
                self.prospector.apply(p, &f, &self.settings);
                self.last_prospect = Some(at);
                decide!("mining.prospector.verdict", {"materials": p.materials.len(),
                "verdict": self.prospector.current().map_or(-1, |v| match v.verdict {
                    prospector::Verdict::Worth { .. } => 1,
                    prospector::Verdict::NotWorth => 0,
                    prospector::Verdict::NoData => 2,
                })});
            }
            EventKind::LaunchDrone(d) => {
                if d.type_.eq_ignore_ascii_case("Prospector")
                    || d.type_.eq_ignore_ascii_case("Collection")
                {
                    self.last_drone = Some(at);
                }
            }
            EventKind::LoadGame(_) => {
                // Nueva partida: los resultados de venta se piden de nuevo.
                self.clear_results();
            }
            EventKind::MarketSell(m) => {
                self.learn(&m.type_, m.type_localised.as_deref());
                self.prices.on_market_sell(m, now);
                self.take_from_inventory(&commodity_key(&m.type_), m.count);
                decide!("mining.market_sell.recorded", {"count": m.count});
            }
            EventKind::EjectCargo(e) => {
                self.take_from_inventory(&commodity_key(&e.type_), e.count);
            }
            EventKind::MiningRefined(r) => {
                self.learn(&r.type_, r.type_localised.as_deref());
                *self.inventory.entry(commodity_key(&r.type_)).or_default() += 1;
                decide!("mining.refined.counted", {"kinds": self.inventory.len()});
            }
            EventKind::Cargo(c) => {
                let ship = c.vessel.as_deref().is_none_or(|v| v == "Ship");
                if let Some(inv) = &c.inventory {
                    for i in inv {
                        self.learn(&i.name, i.name_localised.as_deref());
                    }
                }
                if ship && let Some(inv) = &c.inventory {
                    self.inventory = inventory_of(inv.iter().map(|i| (i.name.as_str(), i.count)));
                    decide!("mining.cargo.collected", {"kinds": self.inventory.len()});
                }
            }
            _ => {}
        }
        self.session.apply_event(&je.kind, now);
    }

    fn take_from_inventory(&mut self, key: &str, count: u32) {
        if let Some(c) = self.inventory.get_mut(key) {
            *c = c.saturating_sub(count);
            if *c == 0 {
                self.inventory.remove(key);
            }
        }
    }

    /// Minerales de la carga, más toneladas primero (máx. 10).
    fn cargo_keys(&self) -> Vec<String> {
        let mut v: Vec<(&String, &u32)> = self.inventory.iter().collect();
        v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        v.into_iter()
            .take(MAX_CARGO_QUERIES)
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Materiales de interés del asteroide actual.
    fn asteroid_keys(&self) -> Vec<String> {
        self.prospector
            .current()
            .map(|c| {
                c.materials
                    .iter()
                    .filter(|m| m.interest)
                    .map(|m| m.key.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Materias del plan de venta (carga y, si caben, asteroide; máx. 10).
    fn plan_keys(&self) -> Vec<String> {
        let mut keys = self.cargo_keys();
        for k in self.asteroid_keys() {
            if keys.len() < MAX_CARGO_QUERIES && !keys.contains(&k) {
                keys.push(k);
            }
        }
        keys
    }

    /// Materias a consultar: la carga y los materiales de interés sin precio.
    fn request_keys(&self) -> Vec<String> {
        let now = self.clock();
        let mut keys = self.cargo_keys();
        for k in self.asteroid_keys() {
            if self.prices.best_price(&k, now).is_none() && !keys.contains(&k) {
                keys.push(k);
            }
        }
        keys
    }

    fn maybe_request(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        let addr = state.location.system_address;
        if ctx.replaying || addr == 0 {
            return;
        }
        if !self.online(state) {
            decide!(
                "mining.sell.request.skipped_offline",
                tmc_model::diagnostics::json!({"pending_keys": self.request_keys().len()})
            );
            return;
        }
        let now = self.clock();
        // Nave desconocida: sin filtro (`Small`), el filtrado fino se hace en local.
        let min_pad = self.cur_pad(state).unwrap_or(PadSize::Small);
        for key in self.request_keys() {
            if self
                .answered
                .get(&key)
                .is_some_and(|at| now - *at < Duration::minutes(RESULT_MAX_AGE_MIN))
            {
                decide!("mining.sell.request.skipped_fresh", {});
                continue;
            }
            if self
                .failed
                .get(&key)
                .is_some_and(|retry_at| now < *retry_at)
            {
                decide!("mining.sell.request.skipped_cooldown", {});
                continue;
            }
            let rk = format!("sell:{addr}:{key}:{}", pad_tag(min_pad));
            if !self.inflight.begin(&rk, now) {
                decide!("mining.sell.request.inflight_dedup", {});
                continue;
            }
            let query = NetQuery::CommodityImportsNearby {
                system_address: addr,
                commodity: key,
                max_distance_ly: self.settings.sell_search_radius_ly,
                min_pad,
            };
            let accepted = ctx.request(rk.clone(), query).is_some();
            if !accepted {
                self.inflight.finish(&rk);
            }
            decide!(
                "mining.sell.request.sent",
                tmc_model::diagnostics::json!({"accepted": accepted})
            );
        }
    }

    /// Devuelve `true` si cambió el libro de precios.
    fn apply_response(&mut self, r: &NetResponse, state: &GameState) -> bool {
        let Some((addr, commodity, pad)) = r
            .key
            .strip_prefix("sell:")
            .and_then(|rest| rest.split_once(':'))
            .and_then(|(a, rest)| {
                let (c, p) = rest.rsplit_once(':')?;
                Some((a.parse::<u64>().ok()?, c.to_owned(), p.to_owned()))
            })
            .filter(|_| r.module == "mining")
        else {
            return false;
        };
        self.inflight.finish(&r.key);
        // Pedida para otra plataforma (la nave cambió): no vale.
        let cur = self.cur_pad(state).unwrap_or(PadSize::Small);
        if pad != pad_tag(cur) {
            decide!("mining.sell.response.stale_pad", {});
            return false;
        }
        let current = addr == state.location.system_address && addr == self.addr;
        let key = commodity_key(&commodity);
        let now = self.clock();
        match &r.outcome {
            NetOutcome::Markets(ms) => {
                let ms = filter_compatible(ms, self.cur_pad(state));
                decide!("mining.sell.response.applied", {"candidates": ms.len(), "current": current});
                self.prices.on_remote(&key, &ms, r.fetched_at);
                if current {
                    self.results.insert(key.clone(), ms);
                    self.answered.insert(key, now);
                    self.fetched_at = self.fetched_at.max(Some(r.fetched_at));
                }
                true
            }
            NetOutcome::NoData => {
                decide!("mining.sell.response.no_data", {"current": current});
                // Sin mercados: se olvida un precio remoto antiguo.
                self.prices.on_remote(&key, &[], r.fetched_at);
                if current {
                    self.results.insert(key.clone(), Vec::new());
                    self.answered.insert(key, now);
                }
                true
            }
            NetOutcome::Failed(_) => {
                decide!("mining.sell.response.failed", {"current": current, "cooldown_s": FAILED_COOLDOWN_S});
                if current {
                    self.failed
                        .insert(key, now + Duration::seconds(FAILED_COOLDOWN_S));
                }
                false
            }
            _ => false,
        }
    }

    /// Aplica el resultado de la reconstrucción de precios. `true` si se aplicó.
    fn on_stored(&mut self, value: &Value) -> bool {
        if self.cmdr.is_none()
            || value.get("cmdr").and_then(Value::as_str) != self.cmdr.as_deref()
            || self.prices_loaded
        {
            decide!("mining.prices.stored.discarded", {"loaded": self.prices_loaded});
            return false;
        }
        decide!("mining.prices.stored.applied", {});
        self.prices.restore_local_sells(value);
        self.prices_loaded = true;
        self.inflight.finish(PRICES_KEY);
        true
    }

    fn recompute_prospector(&mut self) {
        let now = self.clock();
        let book = &self.prices;
        let f = |k: &str| book.best_price(k, now);
        self.prospector.recompute(&f, &self.settings);
    }

    fn render_prospector(&self) -> Value {
        json!({
            "current": self.prospector.current(),
            "history": self.prospector.history().iter().rev().collect::<Vec<_>>(),
        })
    }

    fn render_session(&self) -> Value {
        let now = self.clock();
        let book = &self.prices;
        let f = |k: &str| book.best_price(k, now);
        let mut v = serde_json::to_value(&self.session).unwrap_or_default();
        if let Some(o) = v.as_object_mut() {
            o.insert("as_of".into(), json!(self.now));
            o.insert("names".into(), json!(self.names));
            o.insert(
                "tons_per_hour".into(),
                json!(self.session.tons_per_hour(now)),
            );
            o.insert("est_credits".into(), json!(self.session.est_credits(&f)));
            o.insert(
                "credits_per_hour".into(),
                json!(self.session.credits_per_hour(now, &f)),
            );
            o.insert(
                "refinery_bins".into(),
                self.session
                    .refinery_bins()
                    .into_iter()
                    .map(|(key, tons)| json!({"key": key, "tons": tons}))
                    .collect(),
            );
        }
        v
    }

    fn render_sell(&self, state: &GameState) -> Value {
        if !self.online(state) {
            return json!({"status": "disabled", "as_of": self.now, "names": self.names});
        }
        let keys = self.plan_keys();
        let results: BTreeMap<String, Vec<RemoteMarket>> = self
            .results
            .iter()
            .filter(|(k, _)| keys.contains(k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let any = results.values().any(|v| !v.is_empty());
        let pending = keys
            .iter()
            .any(|k| !self.answered.contains_key(k) && !self.failed.contains_key(k));
        let status = if any {
            "ok"
        } else if pending {
            "loading"
        } else {
            "no_data"
        };
        let plan =
            any.then(|| SellPlan::build(&self.inventory, &results, self.pad, self.fetched_at));
        decide!("mining.sell.plan.built", {"status": status, "candidates": results.values().map(Vec::len).sum::<usize>(),
            "best_station": plan.as_ref().is_some_and(|p| p.best_station_total.is_some())});
        serde_json::to_value(SellTopic {
            status,
            as_of: self.now,
            names: &self.names,
            plan: plan.as_ref(),
        })
        .unwrap_or_default()
    }

    fn set(&mut self, topic: &'static str, v: Value, dirty: &mut TopicSet) {
        // `as_of` avanza con cada evento: no cuenta para detectar cambios.
        if crate::topicx::strip_as_of(self.values.get(topic))
            != crate::topicx::strip_as_of(Some(&v))
        {
            self.values.insert(topic, v);
            dirty.insert(topic.to_owned());
        }
    }
}

fn inventory_of<'a>(items: impl Iterator<Item = (&'a str, u32)>) -> BTreeMap<String, u32> {
    let mut m = BTreeMap::new();
    for (name, count) in items {
        let key = commodity_key(name);
        // Los limpets ocupan bodega pero no se venden.
        if count > 0 && key != "drones" {
            *m.entry(key).or_default() += count;
        }
    }
    m
}

impl Default for Mining {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Mining {
    fn id(&self) -> &'static str {
        "mining"
    }

    fn on_event(&mut self, ev: &SourceEvent, state: &GameState, ctx: &mut ModuleCtx) -> TopicSet {
        let mut recompute = self.sync(state);
        let mut request = false;
        let mut session_dirty = false;
        match ev {
            SourceEvent::Settings(s) => {
                let enabling = s.online_services && !self.settings.online_services;
                let radius = s.sell_search_radius_ly != self.settings.sell_search_radius_ly;
                recompute = true;
                session_dirty = true;
                self.settings = s.clone();
                if radius {
                    self.clear_results();
                }
                request = enabling || radius;
            }
            SourceEvent::Journal { ev, .. } => {
                self.on_journal(ev, state);
                self.ensure_prices_task(ctx);
                session_dirty = matches!(
                    ev.kind,
                    EventKind::MiningRefined(_)
                        | EventKind::LaunchDrone(_)
                        | EventKind::ProspectedAsteroid(_)
                        | EventKind::AsteroidCracked(_)
                        | EventKind::Cargo(_)
                        | EventKind::Loadout(_)
                        | EventKind::EjectCargo(_)
                        | EventKind::MarketSell(_)
                        | EventKind::LoadGame(_)
                );
                request = matches!(
                    ev.kind,
                    EventKind::Location(_)
                        | EventKind::FsdJump(_)
                        | EventKind::CarrierJump(_)
                        | EventKind::Cargo(_)
                        | EventKind::ProspectedAsteroid(_)
                        | EventKind::Loadout(_)
                        | EventKind::LoadGame(_)
                );
            }
            SourceEvent::Companion(CompanionFile::Cargo(f)) => {
                for i in &f.inventory {
                    self.learn(&i.name, i.name_localised.as_deref());
                }
                if f.vessel.as_deref().is_none_or(|v| v == "Ship") {
                    self.inventory =
                        inventory_of(f.inventory.iter().map(|i| (i.name.as_str(), i.count)));
                    request = true;
                    session_dirty = true;
                }
            }
            SourceEvent::Companion(CompanionFile::Raw(CompanionKind::Market, v)) => {
                let now = self.clock();
                if let Some(items) = v.get("Items").and_then(Value::as_array) {
                    for it in items {
                        let name = it.get("Name").and_then(Value::as_str);
                        let loc = it.get("Name_Localised").and_then(Value::as_str);
                        if let Some(name) = name {
                            self.learn(name, loc);
                        }
                    }
                }
                self.prices.on_market_json(v, now);
                session_dirty = true;
            }
            SourceEvent::Stored { module, tag, value } if module == "mining" => {
                if ctx.replaying || tag != TAG_PRICES {
                    return TopicSet::new();
                }
                if !self.on_stored(value) {
                    return TopicSet::new();
                }
                recompute = true;
            }
            SourceEvent::Net(r) if r.module == "mining" => {
                recompute |= self.apply_response(r, state);
            }
            SourceEvent::Action { name, .. } if name == ACTION_RESET => {
                let now = self.clock();
                self.session.reset(now);
                decide!(
                    "mining.session.reset.applied",
                    tmc_model::diagnostics::json!({})
                );
                session_dirty = true;
            }
            _ => return TopicSet::new(),
        }
        if recompute {
            self.recompute_prospector();
        }
        if request || recompute {
            self.maybe_request(state, ctx);
        }
        let mut dirty = TopicSet::new();
        let v = self.render_prospector();
        self.set(TOPIC_PROSPECTOR, v, &mut dirty);
        let now = self.clock();
        let stale = self
            .last_session_render
            .is_none_or(|t| (now - t).num_seconds() >= SESSION_REFRESH_S);
        if session_dirty || recompute || stale {
            self.last_session_render = Some(now);
            let v = self.render_session();
            self.set(TOPIC_SESSION, v, &mut dirty);
        }
        let v = self.render_sell(state);
        self.set(TOPIC_SELL, v, &mut dirty);
        dirty
    }

    fn topic_value(&self, topic: &str) -> Option<Value> {
        self.values.get(topic).cloned()
    }

    fn topics(&self) -> Vec<String> {
        TOPICS.iter().map(|t| (*t).to_owned()).collect()
    }

    fn relevance(&self, _state: &GameState) -> Relevance {
        let now = self.clock();
        let window = Duration::minutes(ACTIVE_WINDOW_MIN);
        let recent = |t: Option<DateTime<Utc>>| t.is_some_and(|t| now - t <= window);
        if recent(self.last_drone) || recent(self.last_prospect) {
            Relevance::Active
        } else {
            Relevance::Normal
        }
    }
}

#[cfg(test)]
mod tests;
