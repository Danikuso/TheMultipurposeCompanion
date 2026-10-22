//! Módulo `engineering`: materiales, ingenieros y planificación de mejoras (F4).
//!
//! Integra los bloques puros de las tareas 12–19: inventario y taquilla, ingenieros, mi
//! nave, coste de ingenierizar, intercambios, lista de la compra, dónde conseguirlo y avisos.
//!
//! Topics (todos con `as_of`, el último instante del diario; se publican solo si cambian y
//! sin campos que dependan del reloj): `engineering.inventory`, `engineering.locker`,
//! `engineering.session`, `engineering.engineers`, `engineering.ship`, `engineering.flow`,
//! `engineering.goals`, `engineering.sources` y `engineering.alerts`.
//!
//! Red y almacén: nada en replay. Los comerciantes y brókers cercanos (`NearestService`)
//! nunca se piden en replay, en Legacy ni sin «servicios online»: el estado de cada fuente
//! es entonces `Disabled`. La lista de la compra se lee al conocer al comandante (en vivo) y
//! se escribe (`engineering.goals.write`) solo en vivo y fuera de Legacy. Un fallo de red
//! cuenta como «sin datos» para esa fuente (nunca se queda en `Loading`).
//!
//! El flujo «ingenierizar un módulo» está abierto en cuanto se elige una ranura
//! (`engineering.select_module`); `engineering.flow.status` es `idle` mientras no hay
//! vista previa, `ok` con coste calculado y `no_data` si el blueprint no está catalogado.

pub mod alerts;
pub mod engineers;
pub mod goals;
pub mod inventory;
pub mod locker;
mod pages;
mod pagination;
pub mod plan;
pub mod ship;
pub mod sources;
pub mod trades;

pub use goals::ENGINEERING_MIGRATION;

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use tmc_core::{
    Alert, Migration, Module, ModuleCtx, ModulePageQuery, ModuleQueryError, Relevance, Severity,
    TopicSet,
};
use tmc_model::companion::{CompanionFile, CompanionKind};
use tmc_model::events::{EngineerStatus, EventKind, JournalEvent};
use tmc_model::net::{BrokerKind, NearbyService, NetOutcome, NetQuery, NetResponse, PadSize};
use tmc_model::settings::ModuleSettings;
use tmc_model::source::SourceEvent;
use tmc_model::state::{FlightMode, Galaxy, GameState};

use crate::data::{blueprints, engineer, engineers as engineer_defs, grade_cap, materials};
use crate::data::{ship_pads, trader_rules};
use crate::inflight::Inflight;
use alerts::{EngAlertKind, alerts_from};
use engineers::{EngineerBook, EngineerStatusChange, NextStep, next_step_for};
use goals::{
    Goal, GoalInput, STATUS_OK, STATUS_TOO_MANY, ShoppingView, TAG_GOALS_READ, TAG_GOALS_WRITE,
    evaluate_goals, goals_from_result, goals_read_task, goals_write_task, pin_goal,
};
use inventory::{MaterialChange, MaterialInventory, SessionChanges};
use locker::LockerInventory;
use plan::{PlanCost, PlanRequest, plan_cost};
use ship::{CurrentEng, ShipView};
use sources::{EngineerPick, SourcesState, engineer_for, pick_engineer};
use trades::TradeProposal;

const TOPIC_CATALOGUE: &str = "engineering.catalogue";
const TOPIC_INVENTORY: &str = "engineering.inventory";
const TOPIC_LOCKER: &str = "engineering.locker";
const TOPIC_SESSION: &str = "engineering.session";
const TOPIC_ENGINEERS: &str = "engineering.engineers";
const TOPIC_SHIP: &str = "engineering.ship";
const TOPIC_FLOW: &str = "engineering.flow";
const TOPIC_GOALS: &str = "engineering.goals";
const TOPIC_SOURCES: &str = "engineering.sources";
const TOPIC_ALERTS: &str = "engineering.alerts";
const TOPICS: [&str; 10] = [
    TOPIC_CATALOGUE,
    TOPIC_INVENTORY,
    TOPIC_LOCKER,
    TOPIC_SESSION,
    TOPIC_ENGINEERS,
    TOPIC_SHIP,
    TOPIC_FLOW,
    TOPIC_GOALS,
    TOPIC_SOURCES,
    TOPIC_ALERTS,
];

const ACTION_SELECT_MODULE: &str = "engineering.select_module";
const ACTION_PREVIEW: &str = "engineering.preview";
const ACTION_CLOSE_FLOW: &str = "engineering.close_flow";
const ACTION_PIN_GOAL: &str = "engineering.pin_goal";
const ACTION_UNPIN_GOAL: &str = "engineering.unpin_goal";
const ACTION_MOVE_GOAL: &str = "engineering.move_goal";
const ACTION_SELECT_ENGINEER: &str = "engineering.select_engineer";
const ACTION_DISMISS_ALERT: &str = "engineering.dismiss_alert";
const ACTION_RESET_SESSION: &str = "engineering.reset_session";

/// Avisos conservados como máximo (los más antiguos se descartan).
const MAX_ALERTS: usize = 20;
/// Identificador del «objetivo» del flujo abierto dentro de la evaluación.
const FLOW_ID: &str = "flow";
const GOALS_KEY: &str = "goals";
const TRADER_PREFIX: &str = "trader:";
const BROKER_PREFIX: &str = "broker:";

static MIGRATIONS: [Migration; 1] = [ENGINEERING_MIGRATION];

/// Estado de un material para una línea del flujo.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlowLine {
    pub key: String,
    pub name_en: Option<String>,
    pub grade: u8,
    pub need: u32,
    pub have: u32,
    /// `"ok"`, `"trade"` o `"missing"`.
    pub state: &'static str,
    /// Lo que sigue faltando tras el intercambio propuesto.
    pub missing: u32,
    pub trade: Option<TradeProposal>,
}

#[derive(Debug, Clone)]
struct Flow {
    ship_id: Option<u64>,
    slot: String,
    request: Option<PlanRequest>,
}

/// Intención del usuario mientras aún falta la lectura inicial.
enum PendingGoalAction {
    Pin(Goal),
    Unpin(String),
    Move(String, usize),
}

/// Cambios que un evento del diario aporta al cálculo de avisos.
#[derive(Default)]
struct EventDelta {
    material: Vec<MaterialChange>,
    status: Vec<EngineerStatusChange>,
    /// Las instantáneas (`Materials`, `Loadout`, lista de arranque…) no avisan.
    alerts: bool,
    /// El evento puede cambiar algo de lo que muestra el módulo; si no, no se recalcula.
    relevant: bool,
}

/// Eventos del diario que pueden cambiar inventario, taquilla, ingenieros, nave,
/// posición, atraque o la carga (necesaria para el siguiente paso de un ingeniero).
/// Why a request cannot be pinned: `incompatible` (module cannot take it) or `no_data`
/// (unresolvable recipe context); `None` when it can.
fn pin_rejection(cost: &PlanCost) -> Option<&'static str> {
    if cost.blockers.contains(&plan::PlanBlocker::IncompatibleItem) {
        Some("incompatible")
    } else if cost.has_resolution_blocker() {
        Some("no_data")
    } else {
        None
    }
}

fn affects_engineering(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Materials(_)
            | EventKind::MaterialCollected(_)
            | EventKind::MaterialDiscarded(_)
            | EventKind::MaterialTrade(_)
            | EventKind::Synthesis(_)
            | EventKind::EngineerCraft(_)
            | EventKind::EngineerProgress(_)
            | EventKind::EngineerContribution(_)
            | EventKind::TechnologyBroker(_)
            | EventKind::MissionCompleted(_)
            | EventKind::ShipLocker(_)
            | EventKind::BackpackChange(_)
            | EventKind::Loadout(_)
            | EventKind::LoadGame(_)
            | EventKind::Commander(_)
            | EventKind::Location(_)
            | EventKind::FsdJump(_)
            | EventKind::CarrierJump(_)
            | EventKind::Docked(_)
            | EventKind::Undocked(_)
            | EventKind::Cargo(_)
    )
}

pub struct Engineering {
    settings: ModuleSettings,
    now: Option<DateTime<Utc>>,
    cmdr: Option<String>,
    inventory: MaterialInventory,
    session: SessionChanges,
    locker: LockerInventory,
    book: EngineerBook,
    ship: ShipView,
    goals: Vec<Goal>,
    goals_status: &'static str,
    goals_loaded: bool,
    pending_goals: Vec<PendingGoalAction>,
    flow: Option<Flow>,
    selected_engineer: Option<u64>,
    sources: SourcesState,
    inflight: Inflight,
    source_requests: BTreeMap<String, u64>,
    source_pad: Option<PadSize>,
    alerts: Vec<Value>,
    /// Línea base para detectar transiciones de avisos.
    at_max_prev: BTreeSet<String>,
    completable_prev: BTreeSet<String>,
    missing_prev: BTreeMap<String, u32>,
    /// Último evento de taller (`EngineerCraft`, `EngineerProgress` suelto…) sin salir aún.
    workshop_activity: bool,
    revision: u64,
    goal_pages: BTreeMap<String, Vec<Value>>,
    locker_pages: BTreeMap<String, Vec<Value>>,
    engineer_rows: Vec<Value>,
    flow_pages: pages::Pages,
    values: std::collections::HashMap<&'static str, Value>,
}

impl Engineering {
    pub fn new() -> Self {
        Self {
            settings: ModuleSettings::default(),
            now: None,
            cmdr: None,
            inventory: MaterialInventory::default(),
            session: SessionChanges::default(),
            locker: LockerInventory::default(),
            book: EngineerBook::default(),
            ship: ShipView::default(),
            goals: Vec::new(),
            goals_status: STATUS_OK,
            goals_loaded: false,
            pending_goals: Vec::new(),
            flow: None,
            selected_engineer: None,
            sources: SourcesState::default(),
            inflight: Inflight::default(),
            source_requests: BTreeMap::new(),
            source_pad: None,
            alerts: Vec::new(),
            at_max_prev: BTreeSet::new(),
            completable_prev: BTreeSet::new(),
            missing_prev: BTreeMap::new(),
            workshop_activity: false,
            revision: 0,
            goal_pages: BTreeMap::new(),
            locker_pages: BTreeMap::new(),
            engineer_rows: Vec::new(),
            flow_pages: pages::Pages::new(),
            values: std::collections::HashMap::new(),
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

    fn online(&self, state: &GameState, ctx: &ModuleCtx) -> bool {
        !ctx.replaying && self.settings.online_services && state.galaxy != Galaxy::Legacy
    }

    fn can_write(&self, state: &GameState, ctx: &ModuleCtx) -> bool {
        !ctx.replaying && state.galaxy != Galaxy::Legacy && self.cmdr.is_some()
    }

    fn stored_cmdr_ok(&self, value: &Value) -> bool {
        self.cmdr.is_some() && value.get("cmdr").and_then(Value::as_str) == self.cmdr.as_deref()
    }

    /// Un cambio de comandante lo descarta todo; la lista de la compra del nuevo se relee.
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
        self.revision += 1;
        decide!("engineering.cmdr.changed", {"switched": switched});
        if !switched {
            return;
        }
        let at = self.clock();
        self.inventory = MaterialInventory::default();
        self.session = SessionChanges::default();
        self.session.reset(at);
        self.locker = LockerInventory::default();
        self.book = EngineerBook::default();
        self.ship = ShipView::default();
        self.goals.clear();
        self.goals_status = STATUS_OK;
        self.goals_loaded = false;
        self.pending_goals.clear();
        self.flow = None;
        self.selected_engineer = None;
        self.sources = SourcesState::default();
        self.source_requests.clear();
        self.source_pad = None;
        self.alerts.clear();
        self.at_max_prev.clear();
        self.completable_prev.clear();
        self.missing_prev.clear();
        self.workshop_activity = false;
        self.inflight.finish(GOALS_KEY);
        self.inflight.finish_prefix(TRADER_PREFIX);
        self.inflight.finish_prefix(BROKER_PREFIX);
    }

    /// Pide la lista de la compra del comandante actual (solo en vivo, una vez).
    fn ensure_goals_read(&mut self, ctx: &mut ModuleCtx) {
        if ctx.replaying || self.goals_loaded {
            return;
        }
        let Some(cmdr) = self.cmdr.clone() else {
            return;
        };
        if !self.inflight.begin(GOALS_KEY, self.clock()) {
            return;
        }
        decide!("engineering.goals.read_requested", {});
        ctx.store_task(TAG_GOALS_READ, goals_read_task(cmdr));
    }

    fn on_goals_stored(&mut self, value: &Value, state: &GameState, ctx: &mut ModuleCtx) -> bool {
        if !self.stored_cmdr_ok(value) || self.goals_loaded {
            decide!("engineering.goals.discarded", {});
            return false;
        }
        self.inflight.finish(GOALS_KEY);
        let mut merged = goals_from_result(value);
        let stored = merged.len();
        let extra = !self.pending_goals.is_empty();
        for action in self.pending_goals.drain(..) {
            match action {
                PendingGoalAction::Pin(g) => {
                    (merged, self.goals_status) = pin_goal(&merged, g);
                }
                PendingGoalAction::Unpin(id) => {
                    merged.retain(|g| g.id != id);
                    self.goals_status = STATUS_OK;
                }
                PendingGoalAction::Move(id, to) => {
                    if let Some(from) = merged.iter().position(|g| g.id == id) {
                        let g = merged.remove(from);
                        merged.insert(to.min(merged.len()), g);
                    }
                }
            }
        }
        self.goals = merged;
        self.goals_loaded = true;
        decide!("engineering.goals.applied", {"stored": stored, "extra": extra});
        if extra {
            self.write_goals(state, ctx);
        }
        true
    }

    fn write_goals(&self, state: &GameState, ctx: &mut ModuleCtx) {
        if !self.can_write(state, ctx) || !self.goals_loaded {
            return;
        }
        let cmdr = self.cmdr.clone().unwrap_or_default();
        ctx.store_task(TAG_GOALS_WRITE, goals_write_task(cmdr, self.goals.clone()));
    }

    // ----- Diario -----

    fn on_journal(
        &mut self,
        je: &JournalEvent,
        state: &GameState,
        ctx: &mut ModuleCtx,
    ) -> EventDelta {
        let at = je.timestamp;
        self.tick(at);
        self.check_cmdr(state);
        let kind = &je.kind;
        let mut delta = EventDelta {
            alerts: true,
            relevant: affects_engineering(&je.kind),
            ..EventDelta::default()
        };
        let changes = self.inventory.apply_event(kind, at);
        self.session.push(&changes);
        delta.material = changes;
        self.locker.apply_event(kind);
        delta.status = self.book.apply_journal(kind, je.raw.get());
        match kind {
            EventKind::Materials(_)
            | EventKind::Loadout(_)
            | EventKind::LoadGame(_)
            | EventKind::Commander(_) => delta.alerts = false,
            EventKind::EngineerProgress(p) => {
                if p.is_startup_list() {
                    delta.alerts = false;
                } else {
                    self.workshop_activity = true;
                }
            }
            _ => {}
        }
        match kind {
            EventKind::Loadout(l) => {
                let previous_item = self
                    .flow
                    .as_ref()
                    .and_then(|f| self.row(&f.slot))
                    .map(|r| r.item.clone());
                self.ship = ShipView::from_loadout(l);
                if self.flow.as_ref().is_some_and(|f| {
                    f.ship_id != self.ship.ship_id
                        || self.row(&f.slot).is_none_or(|r| {
                            !r.engineerable || Some(&r.item) != previous_item.as_ref()
                        })
                }) {
                    self.flow = None;
                    decide!("engineering.flow.closed_ship_changed", {});
                }
            }
            EventKind::EngineerCraft(c) => {
                self.ship.apply_craft(c);
                self.workshop_activity = true;
            }
            EventKind::EngineerContribution(_) => self.workshop_activity = true,
            EventKind::Undocked(_) | EventKind::FsdJump(_) | EventKind::CarrierJump(_) => {
                self.workshop_activity = false;
            }
            _ => {}
        }
        self.ensure_goals_read(ctx);
        delta
    }

    fn row(&self, slot: &str) -> Option<&ship::ShipModuleRow> {
        self.ship
            .modules
            .iter()
            .find(|r| r.slot.eq_ignore_ascii_case(slot))
    }

    // ----- Evaluación -----

    /// Ingeniería actual de la ranura de un objetivo: la de la nave si es la misma, o la
    /// que se vio al fijarlo.
    fn current_for(&self, g: &Goal) -> Option<CurrentEng> {
        if self.ship.ship_id.is_some()
            && self.ship.ship_id == g.ship_id
            && let Some(row) = self.row(&g.slot)
        {
            return row.current.clone();
        }
        g.base.clone()
    }

    fn input_of(id: String, cost: &PlanCost) -> GoalInput {
        GoalInput {
            id,
            needs: cost.total.clone(),
            merc_coins: cost.merc_coins_total,
            blockers: cost.blockers.clone(),
        }
    }

    /// Cost of `req` for the module `item`; unresolved requests carry their blockers.
    fn cost_for(&self, item: &str, req: &PlanRequest, current: Option<&CurrentEng>) -> PlanCost {
        plan_cost(blueprints(), item, current, req)
    }

    fn pos(&self) -> Option<[f64; 3]> {
        self.book.position()
    }

    fn have_commodity(state: &GameState, name: &str) -> u32 {
        state.ship.as_ref().map_or(0, |s| {
            s.cargo
                .iter()
                .filter(|c| c.name.eq_ignore_ascii_case(name))
                .map(|c| c.count)
                .sum()
        })
    }

    fn next_step_of(&self, id: u64, state: &GameState) -> Option<NextStep> {
        let inv = &self.inventory;
        next_step_for(&self.book, id, &|k| inv.count(k), &|k| {
            Self::have_commodity(state, k)
        })
        .map(|s| s.with_credits(state.cmdr.as_ref().and_then(|c| c.credits)))
    }

    /// Ingeniero desbloqueado más cercano para un objetivo fijado (resuelto por su `Goal`).
    fn engineer_for_goal(&self, goal_id: &str) -> Option<String> {
        let g = self.goals.iter().find(|g| g.id == goal_id)?;
        let grades = plan::required_grades(
            blueprints(),
            &g.item,
            self.current_for(g).as_ref(),
            &g.request,
        );
        engineer_for(
            &g.request.module_type,
            &g.request.blueprint,
            &grades,
            &self.book,
            self.pos(),
        )
    }

    fn pick(
        &self,
        item: &str,
        req: &PlanRequest,
        current: Option<&CurrentEng>,
        state: &GameState,
    ) -> EngineerPick {
        let grades = plan::required_grades(blueprints(), item, current, req);
        let mut pick = pick_engineer(
            &req.module_type,
            &req.blueprint,
            &grades,
            &self.book,
            self.pos(),
        );
        if let Some(r) = pick
            .nearest_any
            .as_ref()
            .filter(|r| r.status != EngineerStatus::Unlocked)
        {
            pick.next_step = self.next_step_of(r.id, state);
        }
        pick
    }

    fn material_name(&self, key: &str) -> String {
        self.inventory
            .by_category()
            .values()
            .flat_map(|g| g.values().flatten())
            .find(|l| l.key == key)
            .map(|l| l.display_name().to_owned())
            .or_else(|| materials().get(key).map(|d| d.name_en.clone()))
            .unwrap_or_else(|| key.to_owned())
    }

    /// Recalcula todo y devuelve los topics que cambiaron.
    fn refresh(&mut self, state: &GameState, ctx: &mut ModuleCtx, delta: &EventDelta) -> TopicSet {
        // Necesidades de cada objetivo fijado.
        let mut needs: Vec<GoalInput> = Vec::new();
        let mut verified: BTreeMap<String, bool> = BTreeMap::new();
        for g in &self.goals {
            let cost = self.cost_for(&g.item, &g.request, self.current_for(g).as_ref());
            verified.insert(g.id.clone(), cost.verified);
            needs.push(Self::input_of(g.id.clone(), &cost));
        }
        let rules = trader_rules();
        let inv = &self.inventory;
        let have = |k: &str| inv.count(k);
        // F4 no tiene fuente demostrada del saldo de Merc Coins: siempre desconocido.
        let shopping = evaluate_goals(&needs, &have, rules, self.goals_status, None);

        // El flujo abierto va al final como un objetivo más; un cálculo bloqueado conserva sus motivos.
        let flow_cost = self.flow.as_ref().and_then(|f| {
            let req = f.request.as_ref()?;
            let row = self.row(&f.slot)?;
            Some(self.cost_for(&row.item, req, row.current.as_ref()))
        });
        let all = match &flow_cost {
            Some(c) => {
                let mut with_flow = needs.clone();
                with_flow.push(Self::input_of(FLOW_ID.to_owned(), c));
                evaluate_goals(&with_flow, &have, rules, self.goals_status, None)
            }
            None => shopping.clone(),
        };

        self.update_sources(state, ctx, &all);
        self.raise_alerts(state, ctx, &shopping, delta);

        // Línea base para la próxima transición.
        self.at_max_prev = self.inventory.at_max().into_iter().collect();
        self.completable_prev = shopping
            .goals
            .iter()
            .filter(|g| g.completable)
            .map(|g| g.id.clone())
            .collect();
        self.missing_prev = shopping.missing.clone();

        self.render_all(state, &shopping, &all, flow_cost.as_ref(), &verified)
    }

    fn update_sources(&mut self, state: &GameState, ctx: &mut ModuleCtx, all: &ShoppingView) {
        let addr = (state.location.system_address != 0).then_some(state.location.system_address);
        let pad = state
            .ship
            .as_ref()
            .and_then(|s| ship_pads().pad_for(&s.ship_type))
            .unwrap_or(PadSize::Small);
        if self.source_pad != Some(pad) || self.sources.current_system() != addr {
            self.sources = SourcesState::default();
            self.source_pad = Some(pad);
            self.source_requests.clear();
            self.inflight.finish_prefix(TRADER_PREFIX);
            self.inflight.finish_prefix(BROKER_PREFIX);
        }
        self.sources.set_current_system(addr);
        let online = self.online(state, ctx);
        if !online {
            self.source_requests.clear();
            self.inflight.finish_prefix(TRADER_PREFIX);
            self.inflight.finish_prefix(BROKER_PREFIX);
        }
        self.sources.set_disabled(!online);
        let proposals: Vec<TradeProposal> = all
            .goals
            .iter()
            .flat_map(|g| g.trades.iter().cloned())
            .collect();
        self.sources.set_needs(&all.missing, &proposals);
        if !online {
            return;
        }
        let now = self.clock();
        for q in self.sources.requests(&state.location.system, pad) {
            let NetQuery::NearestService {
                system_address,
                service,
                ..
            } = &q
            else {
                continue;
            };
            let key = match service {
                NearbyService::MaterialTrader(Some(k)) => {
                    format!("{TRADER_PREFIX}{k:?}:{system_address}")
                }
                NearbyService::TechnologyBroker(Some(k)) => {
                    format!("{BROKER_PREFIX}{k:?}:{system_address}")
                }
                _ => continue,
            };
            if !self.inflight.begin(&key, now) {
                continue;
            }
            let id = ctx.request(key.clone(), q);
            let sent = id.is_some();
            if let Some(id) = id {
                self.source_requests.insert(key.clone(), id);
            }
            if !sent {
                self.inflight.finish(&key);
            }
            decide!("engineering.sources.requested", {"sent": sent});
        }
    }

    fn on_net(&mut self, r: &NetResponse) -> bool {
        if self.source_requests.get(&r.key) != Some(&r.id) {
            decide!("engineering.sources.discarded", {});
            return false;
        }
        let parts: Vec<&str> = r.key.split(':').collect();
        let [kind, name, addr] = parts[..] else {
            return false;
        };
        let Ok(addr) = addr.parse::<u64>() else {
            return false;
        };
        let service = match (kind, name) {
            ("trader", "Raw") => {
                NearbyService::MaterialTrader(Some(tmc_model::net::TraderKind::Raw))
            }
            ("trader", "Manufactured") => {
                NearbyService::MaterialTrader(Some(tmc_model::net::TraderKind::Manufactured))
            }
            ("trader", "Encoded") => {
                NearbyService::MaterialTrader(Some(tmc_model::net::TraderKind::Encoded))
            }
            ("broker", "Human") => NearbyService::TechnologyBroker(Some(BrokerKind::Human)),
            ("broker", "Guardian") => NearbyService::TechnologyBroker(Some(BrokerKind::Guardian)),
            _ => return false,
        };
        self.inflight.finish(&r.key);
        // El servicio puede entregar primero Failed y después un reintento con el mismo id.
        if !matches!(r.outcome, NetOutcome::Failed(_)) {
            self.source_requests.remove(&r.key);
        }
        // Cualquier resultado que no sea una lista de estaciones es «sin datos».
        let rows = match &r.outcome {
            NetOutcome::Stations(v) => v.clone(),
            _ => Vec::new(),
        };
        let applied = self.sources.on_stations(addr, service, rows);
        decide!("engineering.sources.response", {"applied": applied});
        applied
    }

    // ----- Avisos -----

    fn raise_alerts(
        &mut self,
        state: &GameState,
        ctx: &mut ModuleCtx,
        shopping: &ShoppingView,
        delta: &EventDelta,
    ) {
        if !delta.alerts || ctx.replaying || !self.settings.alerts_enabled {
            return;
        }
        let at_max_now: BTreeSet<String> = self.inventory.at_max().into_iter().collect();
        let found = alerts_from(
            &delta.material,
            &self.at_max_prev,
            &at_max_now,
            &self.completable_prev,
            shopping,
            &delta.status,
            &self.missing_prev,
            &|goal| self.engineer_for_goal(goal),
        );
        let _ = state;
        let at = self.now;
        for a in found {
            let (id, kind, alert, mut extra) = match &a {
                EngAlertKind::MaterialAtMax { key } => {
                    let grade = materials().get(key).map_or(0, |d| d.grade);
                    let name = self.material_name(key);
                    (
                        format!("at_max:{key}"),
                        "material_at_max",
                        Alert::new(Severity::Info, "engineering-alert-at-max")
                            .arg("material", name)
                            .arg("grade", grade)
                            .arg("max", grade_cap(grade)),
                        json!({"key": key, "grade": grade, "max": grade_cap(grade)}),
                    )
                }
                EngAlertKind::GoalCompletable { goal, engineer } => {
                    let Some(real) = self.goals.iter().find(|g| &g.id == goal) else {
                        continue;
                    };
                    let (bp, grade) = (real.request.blueprint.as_str(), real.request.target_grade);
                    // Un objetivo cuyo blueprint ya no está catalogado no avisa.
                    if blueprints().blueprint(bp).is_none() {
                        continue;
                    }
                    let label = format!("{bp} G{grade}");
                    let alert = match engineer {
                        Some(e) => {
                            Alert::new(Severity::Info, "engineering-alert-goal-completable-with")
                                .arg("goal", label.clone())
                                .arg("engineer", e.clone())
                        }
                        None => Alert::new(Severity::Info, "engineering-alert-goal-completable")
                            .arg("goal", label.clone()),
                    };
                    (
                        format!("goal:{goal}"),
                        "goal_completable",
                        alert,
                        json!({"goal": goal, "engineer": engineer}),
                    )
                }
                EngAlertKind::EngineerUnlocked { id } => {
                    let name = engineer(*id).map_or_else(|| id.to_string(), |d| d.name.clone());
                    (
                        format!("unlocked:{id}"),
                        "engineer_unlocked",
                        Alert::new(Severity::Info, "engineering-alert-engineer-unlocked")
                            .arg("engineer", name),
                        json!({"engineer_id": id}),
                    )
                }
                EngAlertKind::EngineerInvited { id } => {
                    let name = engineer(*id).map_or_else(|| id.to_string(), |d| d.name.clone());
                    (
                        format!("invited:{id}"),
                        "engineer_invited",
                        Alert::new(Severity::Info, "engineering-alert-engineer-invited")
                            .arg("engineer", name),
                        json!({"engineer_id": id}),
                    )
                }
                EngAlertKind::ScarceObtained { key, goal } => {
                    let name = self.material_name(key);
                    (
                        format!("scarce:{key}:{goal}"),
                        "scarce_obtained",
                        Alert::new(Severity::Info, "engineering-alert-scarce-obtained")
                            .arg("material", name),
                        json!({"key": key, "goal": goal}),
                    )
                }
            };
            // Sin duplicados por id mientras siga en la lista.
            if self.alerts.iter().any(|x| x["id"] == json!(id)) {
                decide!("engineering.alert.duplicate", {});
                continue;
            }
            ctx.alert(alert);
            if let Some(o) = extra.as_object_mut() {
                o.insert("id".into(), json!(id));
                o.insert("kind".into(), json!(kind));
                o.insert("at".into(), json!(at));
            }
            decide!("engineering.alert.raised", {"kind": kind});
            self.alerts.push(extra);
            if self.alerts.len() > MAX_ALERTS {
                self.alerts.remove(0);
            }
        }
    }

    // ----- Render -----

    fn stamp(&self, mut v: Value) -> Value {
        if let Some(o) = v.as_object_mut() {
            o.insert("as_of".into(), json!(self.now));
        }
        v
    }

    fn render_inventory(&self) -> Value {
        let mut cats = serde_json::Map::new();
        for (cat, grades) in self.inventory.by_category() {
            let mut gm = serde_json::Map::new();
            for (grade, lines) in grades {
                let lines: Vec<Value> = lines
                    .iter()
                    .map(|l| {
                        let mut o = serde_json::Map::new();
                        o.insert("key".into(), json!(l.key));
                        // Sin nulos: solo el nombre del diario o, si falta, el del catálogo.
                        match (&l.name_localised, &l.name_en) {
                            (Some(n), _) => {
                                o.insert("name_localised".into(), json!(n));
                            }
                            (None, Some(n)) => {
                                o.insert("name_en".into(), json!(n));
                            }
                            (None, None) => {}
                        }
                        o.insert("count".into(), json!(l.count));
                        if !l.known {
                            o.insert("known".into(), json!(false));
                        }
                        Value::Object(o)
                    })
                    .collect();
                gm.insert(grade.to_string(), Value::Array(lines));
            }
            let name = match cat {
                tmc_model::events::MaterialCategory::Raw => "Raw",
                tmc_model::events::MaterialCategory::Manufactured => "Manufactured",
                tmc_model::events::MaterialCategory::Encoded => "Encoded",
            };
            cats.insert(name.into(), Value::Object(gm));
        }
        let caps: BTreeMap<String, u32> =
            (1..=5u8).map(|g| (g.to_string(), grade_cap(g))).collect();
        self.stamp(json!({
            "snapshot": self.inventory.has_snapshot(),
            "caps": caps,
            "by_category": cats,
            "at_max": self.inventory.at_max(),
        }))
    }

    fn render_session(&self) -> Value {
        let totals: BTreeMap<String, Value> = self
            .session
            .totals()
            .into_iter()
            .map(|(o, (i, out))| (format!("{o:?}"), json!({"in": i, "out": out})))
            .collect();
        self.stamp(json!({
            "since": self.session.since(),
            "totals": totals,
            "recent": self.session.recent(),
        }))
    }

    fn render_engineers(&self, state: &GameState) -> Value {
        let selected = self.selected_engineer.and_then(|id| {
            let def = engineer(id)?;
            Some(json!({
                "id": id,
                "next_step": self.next_step_of(id, state),
                "blueprints": def.blueprints.iter().map(|b| json!({
                    "module_type": b.module_type,
                    "blueprint": b.blueprint,
                    "grades": b.grades,
                    "max_grade": b.max_grade(),
                })).collect::<Vec<_>>(),
            }))
        });
        self.stamp(json!({
            "engineers": self.book.rows(),
            "selected": selected,
        }))
    }

    fn render_flow(&self, state: &GameState, all: &ShoppingView, cost: Option<&PlanCost>) -> Value {
        let idle = json!({
            "status": "idle",
            "slot": null,
            "module_type": null,
            "item": null,
            "current": null,
            "options": {"blueprints": [], "experimentals": []},
            "request": null,
            "cost": null,
            "lines": [],
            "percent": 0,
            "engineer": null,
            "blockers": [],
            "materials_ready": false,
            "merc_coins": null,
        });
        let Some(flow) = &self.flow else {
            return self.stamp(idle);
        };
        let row = self.row(&flow.slot);
        let module_type = row.and_then(|r| r.module_type.clone());
        let cat = blueprints();
        let options = match &module_type {
            Some(t) => json!({
                "blueprints": cat.blueprints_for(t).iter().map(|b| b.key.clone()).collect::<Vec<_>>(),
                "experimentals": cat.experimentals_for(t).iter().map(|e| e.key.clone()).collect::<Vec<_>>(),
            }),
            None => json!({"blueprints": [], "experimentals": []}),
        };
        let mut v = json!({
            "status": "idle",
            "slot": flow.slot,
            "module_type": module_type,
            "item": row.map(|r| r.item.clone()),
            "current": row.and_then(|r| r.current.clone()),
            "options": options,
            "request": flow.request,
            "cost": null,
            "lines": [],
            "percent": 0,
            "engineer": null,
            "blockers": [],
            "materials_ready": false,
            "merc_coins": null,
        });
        let (Some(req), Some(o)) = (&flow.request, v.as_object_mut()) else {
            return self.stamp(v);
        };
        let Some(cost) = cost else {
            o.insert("status".into(), json!("no_data"));
            return self.stamp(v);
        };
        let gv = all.goals.iter().find(|g| g.id == FLOW_ID);
        o.insert("blockers".into(), json!(cost.blockers));
        if cost.has_resolution_blocker() {
            // El motivo del bloqueo viaja en `blockers`; sin coste ni líneas que mostrar.
            o.insert("status".into(), json!("no_data"));
            return self.stamp(v);
        }
        let mut lines = Vec::new();
        for (key, need) in &cost.total {
            let short = gv.and_then(|g| g.missing.get(key)).copied().unwrap_or(0);
            let trade = gv.and_then(|g| g.trades.iter().find(|t| &t.get_key == key).cloned());
            let (st, missing) = match (&trade, short) {
                (_, 0) => ("ok", 0),
                (Some(t), s) if t.get >= s => ("trade", 0),
                (Some(t), s) => ("missing", s - t.get),
                (None, s) => ("missing", s),
            };
            let def = materials().get(key);
            lines.push(FlowLine {
                key: key.clone(),
                name_en: def.map(|d| d.name_en.clone()),
                grade: def.map_or(0, |d| d.grade),
                need: *need,
                have: self.inventory.count(key),
                state: st,
                missing,
                trade,
            });
        }
        o.insert("status".into(), json!("ok"));
        o.insert("cost".into(), json!(cost));
        o.insert("lines".into(), json!(lines));
        o.insert("percent".into(), json!(gv.map_or(0, |g| g.percent)));
        o.insert(
            "materials_ready".into(),
            json!(gv.is_some_and(|g| g.materials_ready)),
        );
        o.insert("merc_coins".into(), json!(gv.map(|g| g.merc_coins)));
        o.insert(
            "engineer".into(),
            json!(self.pick(
                row.map_or("", |r| r.item.as_str()),
                req,
                row.and_then(|r| r.current.as_ref()),
                state
            )),
        );
        self.stamp(v)
    }

    fn render_goals(&self, shopping: &ShoppingView, verified: &BTreeMap<String, bool>) -> Value {
        let goals: Vec<Value> = shopping
            .goals
            .iter()
            .filter_map(|gv| {
                let g = self.goals.iter().find(|g| g.id == gv.id)?;
                Some(json!({
                    "id": g.id,
                    "slot": g.slot,
                    "item": g.item,
                    "request": g.request,
                    "percent": gv.percent,
                    "missing": gv.missing,
                    "trades": gv.trades,
                    "completable": gv.completable,
                    "materials_ready": gv.materials_ready,
                    "merc_coins": gv.merc_coins,
                    "blockers": gv.blockers,
                    "verified": verified.get(&g.id).copied().unwrap_or(false),
                    "engineer": self.engineer_for_goal(&g.id),
                }))
            })
            .collect();
        json!({
            "status": shopping.status,
            "goals": goals,
            "needs": shopping.needs,
            "missing": shopping.missing,
            "surplus": shopping.surplus,
        })
    }

    fn set(&mut self, topic: &'static str, v: Value, dirty: &mut TopicSet) {
        crate::topicx::set_topic(&mut self.values, topic, v, dirty);
    }

    fn render_all(
        &mut self,
        state: &GameState,
        shopping: &ShoppingView,
        all: &ShoppingView,
        flow_cost: Option<&PlanCost>,
        verified: &BTreeMap<String, bool>,
    ) -> TopicSet {
        let mut dirty = TopicSet::new();
        let inventory = self.render_inventory();
        let mut engineers = self.render_engineers(state);
        let semantic_changed = |topic: &str, value: &Value| {
            let strip = |v: &Value| {
                v.as_object().map(|o| {
                    o.iter()
                        .filter(|(k, _)| !matches!(k.as_str(), "as_of" | "revision"))
                        .collect::<BTreeMap<_, _>>()
                        .into_iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<BTreeMap<_, _>>()
                })
            };
            self.values
                .get(topic)
                .is_none_or(|old| strip(old) != strip(value))
        };
        let mut flow = self.render_flow(state, all, flow_cost);
        let flow_pages = {
            let row = self.flow.as_ref().and_then(|f| self.row(&f.slot)).map(|r| {
                (
                    r.item.as_str(),
                    r.module_type.as_deref(),
                    r.current.as_ref(),
                )
            });
            pages::flow_pages(blueprints(), row, flow_cost, &flow["lines"])
        };
        let engineer_rows: Vec<Value> = self
            .book
            .rows()
            .iter()
            .map(|r| serde_json::to_value(r).unwrap_or_default())
            .collect();
        if let Some(o) = engineers.as_object_mut() {
            o.insert("total".into(), json!(engineer_rows.len()));
            o.insert("selected_id".into(), json!(self.selected_engineer));
        }
        let dependencies_changed = semantic_changed(TOPIC_INVENTORY, &inventory)
            || semantic_changed(TOPIC_ENGINEERS, &engineers);
        let mut details = self.render_goals(shopping, verified);
        let mut goal_pages = BTreeMap::new();
        goal_pages.insert(
            "goals".to_owned(),
            details["goals"]
                .as_array_mut()
                .map(std::mem::take)
                .unwrap_or_default(),
        );
        for section in ["needs", "missing", "surplus"] {
            let rows = details[section]
                .as_object_mut()
                .map(std::mem::take)
                .unwrap_or_default()
                .into_iter()
                .map(|(key, count)| json!({"key":key,"count":count}))
                .collect();
            goal_pages.insert(section.to_owned(), rows);
        }
        let locker = serde_json::to_value(self.locker.view()).unwrap_or_default();
        let locker_pages: BTreeMap<String, Vec<Value>> =
            serde_json::from_value(locker).unwrap_or_default();
        if dependencies_changed
            || self
                .values
                .get(TOPIC_GOALS)
                .is_none_or(|old| old["status"] != shopping.status)
            || self.goal_pages != goal_pages
            || self.locker_pages != locker_pages
            || self.engineer_rows != engineer_rows
            || self.flow_pages != flow_pages
        {
            self.revision += 1;
        }
        self.goal_pages = goal_pages;
        self.locker_pages = locker_pages;
        self.engineer_rows = engineer_rows;
        self.flow_pages = flow_pages;
        let counts_of = |pages: &pages::Pages, names: &[&str]| -> BTreeMap<String, usize> {
            names
                .iter()
                .map(|n| ((*n).to_owned(), pages.get(*n).map_or(0, Vec::len)))
                .collect()
        };
        let catalogue_counts = counts_of(
            pages::catalogue_pages(),
            &[
                "module_types",
                "blueprints",
                "recipes",
                "experimentals",
                "variants",
            ],
        );
        let v = self.stamp(json!({"revision": self.revision, "counts": catalogue_counts}));
        self.set(TOPIC_CATALOGUE, v, &mut dirty);
        if let Some(o) = engineers.as_object_mut() {
            o.insert("revision".into(), json!(self.revision));
        }
        if let Some(o) = flow.as_object_mut() {
            o.insert("revision".into(), json!(self.revision));
            o.insert(
                "counts".into(),
                json!(counts_of(&self.flow_pages, &pages::FLOW_SECTIONS)),
            );
        }
        self.set(TOPIC_INVENTORY, inventory, &mut dirty);
        let counts: BTreeMap<_, _> = self
            .locker_pages
            .iter()
            .map(|(k, v)| (k, v.len()))
            .collect();
        let v = self.stamp(json!({"revision":self.revision,"counts":counts}));
        self.set(TOPIC_LOCKER, v, &mut dirty);
        let v = self.render_session();
        self.set(TOPIC_SESSION, v, &mut dirty);
        self.set(TOPIC_ENGINEERS, engineers, &mut dirty);
        let v = self.stamp(serde_json::to_value(&self.ship).unwrap_or_default());
        self.set(TOPIC_SHIP, v, &mut dirty);
        self.set(TOPIC_FLOW, flow, &mut dirty);
        let goals: Vec<Value> = self.goal_pages["goals"].iter().map(|g| json!({
            "id":g["id"], "percent":g["percent"], "completable":g["completable"], "engineer":g["engineer"],
            "materials_ready":g["materials_ready"], "merc_coins":g["merc_coins"], "blockers":g["blockers"],
        })).collect();
        let v = self.stamp(json!({"revision":self.revision,"status":shopping.status,"total":goals.len(),"goals":goals}));
        self.set(TOPIC_GOALS, v, &mut dirty);
        let v = self.stamp(serde_json::to_value(self.sources.view()).unwrap_or_default());
        self.set(TOPIC_SOURCES, v, &mut dirty);
        let v = self.stamp(json!({"alerts": self.alerts}));
        self.set(TOPIC_ALERTS, v, &mut dirty);
        dirty
    }

    // ----- Acciones -----

    fn on_action(&mut self, name: &str, args: &Value, state: &GameState, ctx: &ModuleCtx) -> bool {
        match name {
            ACTION_SELECT_MODULE => {
                let Some(slot) = args.get("slot").and_then(Value::as_str) else {
                    decide!("engineering.select_module.rejected_args", {});
                    return false;
                };
                let Some(row) = self
                    .row(slot)
                    .filter(|r| r.engineerable && r.module_type.is_some())
                else {
                    decide!("engineering.select_module.rejected_slot", {});
                    return false;
                };
                self.flow = Some(Flow {
                    ship_id: self.ship.ship_id,
                    slot: row.slot.clone(),
                    request: None,
                });
                true
            }
            ACTION_PREVIEW => self.on_preview(args),
            ACTION_CLOSE_FLOW => {
                self.workshop_activity = false;
                self.flow.take().is_some()
            }
            ACTION_PIN_GOAL => self.on_pin(state, ctx),
            ACTION_UNPIN_GOAL => {
                let Some(id) = args.get("id").and_then(Value::as_str) else {
                    decide!("engineering.unpin.rejected_args", {});
                    return false;
                };
                let Some(pos) = self.goals.iter().position(|g| g.id == id) else {
                    decide!("engineering.unpin.rejected_unknown", {});
                    return false;
                };
                self.goals.remove(pos);
                if !self.goals_loaded {
                    self.pending_goals
                        .push(PendingGoalAction::Unpin(id.to_owned()));
                }
                self.goals_status = STATUS_OK;
                true
            }
            ACTION_MOVE_GOAL => {
                let id = args.get("id").and_then(Value::as_str);
                let to = args
                    .get("to")
                    .and_then(Value::as_u64)
                    .and_then(|t| usize::try_from(t).ok());
                let (Some(id), Some(to)) = (id, to) else {
                    decide!("engineering.move.rejected_args", {});
                    return false;
                };
                let from = self.goals.iter().position(|g| g.id == id);
                let (Some(from), true) = (from, to < self.goals.len()) else {
                    decide!("engineering.move.rejected_range", {});
                    return false;
                };
                if from == to {
                    return false;
                }
                let g = self.goals.remove(from);
                self.goals.insert(to, g);
                if !self.goals_loaded {
                    self.pending_goals
                        .push(PendingGoalAction::Move(id.to_owned(), to));
                }
                true
            }
            ACTION_SELECT_ENGINEER => {
                let id = args.get("id").and_then(Value::as_u64);
                let Some(id) = id.filter(|i| engineer(*i).is_some()) else {
                    decide!("engineering.select_engineer.rejected", {});
                    return false;
                };
                self.selected_engineer = Some(id);
                true
            }
            ACTION_DISMISS_ALERT => {
                let idx = args
                    .get("index")
                    .and_then(Value::as_u64)
                    .and_then(|i| usize::try_from(i).ok())
                    .filter(|i| *i < self.alerts.len());
                let Some(idx) = idx else {
                    decide!("engineering.dismiss.rejected", {});
                    return false;
                };
                self.alerts.remove(idx);
                true
            }
            ACTION_RESET_SESSION => {
                let now = self.clock();
                self.session.reset(now);
                decide!("engineering.session.reset.applied", {});
                true
            }
            _ => false,
        }
    }

    fn on_preview(&mut self, args: &Value) -> bool {
        let Some(flow) = &self.flow else {
            decide!("engineering.preview.rejected_no_flow", {});
            return false;
        };
        let Some(module_type) = self.row(&flow.slot).and_then(|r| r.module_type.clone()) else {
            return false;
        };
        let Some(blueprint) = args
            .get("blueprint")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|b| !b.is_empty())
        else {
            decide!("engineering.preview.rejected_blueprint", {});
            return false;
        };
        let Some(grade) = args
            .get("grade")
            .and_then(Value::as_u64)
            .filter(|g| (1..=5).contains(g))
        else {
            decide!("engineering.preview.rejected_grade", {});
            return false;
        };
        let cat = blueprints();
        let raw = blueprint.to_ascii_lowercase();
        let known_elsewhere =
            cat.blueprint(&raw).is_some() || cat.aliases.iter().any(|a| a.alias == raw);
        if known_elsewhere && cat.canonical_blueprint(&module_type, &raw).is_none() {
            decide!("engineering.preview.rejected_incompatible", {});
            return false;
        }
        let experimental = match args.get("experimental") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) if s.trim().is_empty() => None,
            Some(Value::String(s)) => match cat.canonical_experimental(&module_type, s) {
                Some(key) => Some(key.to_owned()),
                None => {
                    decide!("engineering.preview.rejected_experimental", {});
                    return false;
                }
            },
            Some(_) => {
                decide!("engineering.preview.rejected_experimental", {});
                return false;
            }
        };
        let rolls = match args.get("rolls") {
            None | Some(Value::Null) => self.settings.engineering_rolls,
            Some(Value::Array(a)) if a.len() == 5 => {
                let mut r = [0u8; 5];
                for (slot, v) in r.iter_mut().zip(a) {
                    match v.as_u64().filter(|n| (1..=10).contains(n)) {
                        Some(n) => *slot = n as u8,
                        None => {
                            decide!("engineering.preview.rejected_rolls", {});
                            return false;
                        }
                    }
                }
                r
            }
            Some(_) => {
                decide!("engineering.preview.rejected_rolls", {});
                return false;
            }
        };
        // Un blueprint no catalogado se guarda tal cual: el flujo queda en `no_data`.
        let known = cat
            .canonical_blueprint(&module_type, blueprint)
            .and_then(|key| cat.blueprint(key));
        let request = PlanRequest {
            module_type,
            blueprint: known.map_or_else(|| blueprint.to_owned(), |b| b.key.clone()),
            experimental: experimental.filter(|_| known.is_some()),
            target_grade: grade as u8,
            rolls,
        };
        if let Some(f) = self.flow.as_mut() {
            f.request = Some(request);
        }
        true
    }

    fn on_pin(&mut self, state: &GameState, ctx: &ModuleCtx) -> bool {
        let Some(flow) = &self.flow else {
            decide!("engineering.pin.rejected_no_flow", {});
            return false;
        };
        let Some(req) = flow.request.clone() else {
            decide!("engineering.pin.rejected_no_request", {});
            return false;
        };
        let Some(row) = self.row(&flow.slot) else {
            return false;
        };
        let cost = self.cost_for(&row.item, &req, row.current.as_ref());
        match pin_rejection(&cost) {
            Some("incompatible") => {
                decide!("engineering.pin.rejected_incompatible", {});
                return false;
            }
            Some(_) => {
                decide!("engineering.pin.rejected_no_data", {});
                return false;
            }
            None => {}
        }
        if row.module_type.as_deref() != Some(req.module_type.as_str()) || !row.engineerable {
            decide!("engineering.pin.rejected_incompatible", {});
            return false;
        }
        let goal = Goal::new(
            flow.ship_id,
            row.slot.clone(),
            row.item.clone(),
            req,
            row.current.clone(),
        );
        let (goals, status) = pin_goal(&self.goals, goal.clone());
        if !self.goals_loaded && status == STATUS_OK {
            self.pending_goals.push(PendingGoalAction::Pin(goal));
        }
        let status = if status == STATUS_TOO_MANY {
            STATUS_TOO_MANY
        } else {
            STATUS_OK
        };
        let changed = goals != self.goals || status != self.goals_status;
        self.goals = goals;
        self.goals_status = status;
        let _ = (state, ctx);
        changed
    }

    /// `true` si la acción cambió la lista de objetivos (hay que escribirla).
    fn is_goal_action(name: &str) -> bool {
        matches!(name, ACTION_PIN_GOAL | ACTION_UNPIN_GOAL | ACTION_MOVE_GOAL)
    }

    fn at_workshop(state: &GameState) -> bool {
        let l = &state.location;
        if l.mode != FlightMode::Docked {
            return false;
        }
        engineer_defs().iter().any(|d| {
            (d.market_id.is_some() && d.market_id == l.market_id)
                || (l
                    .station
                    .as_deref()
                    .is_some_and(|s| s.eq_ignore_ascii_case(&d.workshop))
                    && l.system.eq_ignore_ascii_case(&d.system))
        })
    }
}

impl Default for Engineering {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Engineering {
    fn id(&self) -> &'static str {
        "engineering"
    }

    fn on_event(&mut self, ev: &SourceEvent, state: &GameState, ctx: &mut ModuleCtx) -> TopicSet {
        let mut delta = EventDelta::default();
        match ev {
            SourceEvent::Settings(s) => {
                let enabling = s.online_services && !self.settings.online_services;
                self.settings = s.clone();
                if enabling {
                    // Los «sin datos» de cuando no había red se olvidan.
                    self.sources.set_current_system(None);
                }
            }
            SourceEvent::Journal { ev: je, .. } => {
                delta = self.on_journal(je, state, ctx);
                if !delta.relevant {
                    return TopicSet::new();
                }
            }
            SourceEvent::Companion(CompanionFile::Raw(CompanionKind::ShipLocker, v)) => {
                self.locker.apply_file(v);
            }
            // La carga cuenta para el «siguiente paso» de un ingeniero (contribución en mercancía).
            SourceEvent::Companion(CompanionFile::Cargo(_)) => {}
            SourceEvent::Stored { module, tag, value } if module == "engineering" => {
                if ctx.replaying {
                    return TopicSet::new();
                }
                let applied = match tag.as_str() {
                    TAG_GOALS_READ => self.on_goals_stored(value, state, ctx),
                    _ => false,
                };
                if !applied {
                    return TopicSet::new();
                }
            }
            SourceEvent::Net(r) if r.module == "engineering" => {
                if !self.on_net(r) {
                    return TopicSet::new();
                }
            }
            SourceEvent::Action { name, args } if name.starts_with("engineering.") => {
                if !self.on_action(name, args, state, ctx) {
                    return TopicSet::new();
                }
                if Self::is_goal_action(name) {
                    self.write_goals(state, ctx);
                }
            }
            _ => return TopicSet::new(),
        }
        self.refresh(state, ctx, &delta)
    }

    fn query(&self, resource: &str, query: &ModulePageQuery) -> Result<Value, ModuleQueryError> {
        let section = query.section.as_deref();
        let (default, valid): (&str, &[&str]) = match resource {
            "goals" => ("goals", &["goals", "needs", "missing", "surplus"]),
            "locker" => ("items", &["items", "components", "consumables", "data"]),
            "catalogue" => ("blueprints", &pages::CATALOGUE_SECTIONS),
            "engineers" => ("engineers", &pages::ENGINEER_SECTIONS),
            "flow" => ("lines", &pages::FLOW_SECTIONS),
            _ => return Err(ModuleQueryError::NotFound),
        };
        let section = section.unwrap_or(default);
        if !valid.contains(&section) {
            return Err(ModuleQueryError::InvalidQuery);
        }
        let rows: &[Value] = match (resource, section) {
            ("goals", s) => self.goal_pages.get(s),
            ("locker", s) => self.locker_pages.get(s),
            ("catalogue", s) => pages::catalogue_pages().get(s),
            ("engineers", "engineers") => Some(&self.engineer_rows),
            ("engineers", s) => pages::engineer_static_pages().get(s),
            (_, s) => self.flow_pages.get(s),
        }
        .ok_or(ModuleQueryError::Unavailable)?;
        pagination::page(self.revision, section, rows, query)
    }

    fn topic_value(&self, topic: &str) -> Option<Value> {
        self.values.get(topic).cloned()
    }

    fn topics(&self) -> Vec<String> {
        TOPICS.iter().map(|t| (*t).to_owned()).collect()
    }

    fn relevance(&self, state: &GameState) -> Relevance {
        if self.flow.is_some() || self.workshop_activity || Self::at_workshop(state) {
            Relevance::Active
        } else {
            Relevance::Normal
        }
    }

    fn migrations(&self) -> &'static [Migration] {
        &MIGRATIONS
    }
}

#[cfg(test)]
mod tests;
