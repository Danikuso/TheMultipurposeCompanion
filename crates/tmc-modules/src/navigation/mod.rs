//! Módulo `navigation`: tira de ruta (ficha local + remota) y alcance de salto.

pub mod jump;
pub mod persist;
pub mod planner;
pub mod route;
pub mod strip;

use crate::data::{fsd_table, star_flags};
use chrono::{DateTime, SecondsFormat, Utc};
use jump::{JumpModel, RangeSource, ShipMass, build_jump_model, fuel_for_jump};
use planner::Planner;
use route::ActivePlan;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use strip::{
    Entry, LocalPolitics, RemoteState, RemoteStatus, RouteTopic, STRIP_AHEAD, Strip, SystemCard,
    build_strip, system_key,
};
use tmc_core::{Alert, Module, ModuleCtx, Relevance, Severity, TopicSet};
use tmc_model::events::EventKind;
use tmc_model::net::{NetOutcome, NetQuery, NetResponse};
use tmc_model::settings::ModuleSettings;
use tmc_model::source::SourceEvent;
use tmc_model::state::{FlightMode, Galaxy, GameState};

const TOPIC_ROUTE: &str = "navigation.route";
const TOPIC_JUMP: &str = "navigation.jump";
const TOPIC_PLAN: &str = "navigation.plan";
const VISITS_TAG: &str = "visits";
pub const VISITS_SQL: &str = "SELECT json_extract(json,'$.SystemAddress'), COUNT(*) FROM event WHERE name='FSDJump' AND cmdr=?1 AND ts<=?2 GROUP BY 1";
/// Una petición sin respuesta pasado este tiempo se da por perdida y puede repetirse.
const INFLIGHT_TTL_S: i64 = 60;
/// Un `Failed` se reintenta pasado este tiempo (la red ya hace su propio backoff).
const FAILED_COOLDOWN_S: i64 = 300;

#[derive(Debug, Serialize)]
struct JumpTopic {
    range_ly: Option<f64>,
    range_full_ly: Option<f64>,
    source: RangeSource,
    fuel_main: Option<f64>,
    fuel_capacity: Option<f64>,
    max_fuel_per_jump: Option<f64>,
    supercharged: Option<f64>,
}

/// Visitas por sistema: histórico del almacén más los saltos posteriores al corte.
#[derive(Default)]
struct Visits {
    cmdr: Option<String>,
    cut: Option<DateTime<Utc>>,
    loaded: bool,
    counts: HashMap<u64, u32>,
    pending: HashMap<u64, u32>,
}

impl Visits {
    fn reset(&mut self, cmdr: String, cut: DateTime<Utc>) {
        *self = Self {
            cmdr: Some(cmdr),
            cut: Some(cut),
            ..Self::default()
        };
    }

    /// Un `FSDJump` hasta el corte ya lo cuenta la consulta; solo suman los posteriores.
    fn on_jump(&mut self, id64: u64, at: DateTime<Utc>) {
        if self.cut.is_none_or(|c| at <= c) {
            return;
        }
        let map = if self.loaded {
            &mut self.counts
        } else {
            &mut self.pending
        };
        let n = map.entry(id64).or_insert(0);
        *n = n.saturating_add(1);
    }

    fn load(&mut self, rows: &Value) {
        self.counts.clear();
        for row in rows.as_array().into_iter().flatten() {
            let id = row.get(0).and_then(Value::as_u64);
            let n = row.get(1).and_then(Value::as_u64);
            if let (Some(id), Some(n)) = (id, n) {
                self.counts.insert(id, u32::try_from(n).unwrap_or(u32::MAX));
            }
        }
        for (id, n) in self.pending.drain() {
            let c = self.counts.entry(id).or_insert(0);
            *c = c.saturating_add(n);
        }
        self.loaded = true;
    }

    fn get(&self, id64: u64) -> Option<u32> {
        self.loaded
            .then(|| self.counts.get(&id64).copied().unwrap_or(0))
    }
}

pub struct Navigation {
    settings: ModuleSettings,
    plan: Option<ActivePlan>,
    planner: Planner,
    /// Comandante al que pertenece el plan en memoria.
    plan_cmdr: Option<String>,
    /// Hay cambios del plan pendientes de volcar al almacén.
    plan_dirty: bool,
    /// Espera la carga del plan del almacen (un `clear-plan` la invalida).
    load_pending: bool,
    model: Option<JumpModel>,
    unladen_mass: Option<f64>,
    supercharged: Option<f64>,
    politics: Option<(u64, LocalPolitics)>,
    remote: HashMap<String, RemoteState>,
    /// Clave -> (id de petición, instante de envío).
    inflight: HashMap<String, (u64, DateTime<Utc>)>,
    /// Último instante conocido (eventos del diario y Status).
    now: Option<DateTime<Utc>>,
    visits: Visits,
    copy_seq: u64,
    route_value: Value,
    jump_value: Value,
    plan_value: Value,
}

impl Navigation {
    pub fn new() -> Self {
        Self {
            settings: ModuleSettings::default(),
            plan: None,
            planner: Planner::default(),
            plan_cmdr: None,
            plan_dirty: false,
            load_pending: false,
            model: None,
            unladen_mass: None,
            supercharged: None,
            politics: None,
            remote: HashMap::new(),
            inflight: HashMap::new(),
            now: None,
            visits: Visits::default(),
            copy_seq: 0,
            route_value: Value::Null,
            jump_value: Value::Null,
            plan_value: Value::Null,
        }
    }

    /// Plan activo (lo alimenta el planificador). `None` vuelve a `NavRoute`.
    pub fn set_plan(&mut self, plan: Option<ActivePlan>) {
        self.plan = plan;
    }

    fn tick(&mut self, at: DateTime<Utc>) {
        if self.now.is_none_or(|n| at > n) {
            self.now = Some(at);
        }
    }

    fn online(&self, state: &GameState) -> bool {
        self.settings.online_services && state.galaxy != Galaxy::Legacy
    }

    fn mass_now(&self, state: &GameState) -> Option<ShipMass> {
        let ship = state.ship.as_ref()?;
        Some(ShipMass {
            unladen: self.unladen_mass?,
            fuel_main: ship.fuel_main?,
            fuel_reserve: ship.fuel_reservoir.unwrap_or(0.0),
            cargo: f64::from(ship.cargo_count.unwrap_or(0)),
        })
    }

    fn note_visits_source(&mut self, state: &GameState, at: DateTime<Utc>, ctx: &mut ModuleCtx) {
        let Some(name) = state
            .cmdr
            .as_ref()
            .map(|c| c.name.clone())
            .filter(|n| !n.is_empty())
        else {
            return;
        };
        if self.visits.cmdr.as_deref() == Some(name.as_str()) {
            return;
        }
        self.visits.reset(name.clone(), at);
        let cut = at.to_rfc3339_opts(SecondsFormat::Nanos, true);
        ctx.store_task(
            VISITS_TAG,
            Box::new(move |store: &tmc_core::Store| {
                let rows = store
                    .query_json(VISITS_SQL, &[Value::from(name.as_str()), Value::from(cut)])?;
                Ok(serde_json::json!({"cmdr": name, "rows": rows}))
            }),
        );
    }

    fn on_journal(
        &mut self,
        je: &tmc_model::events::JournalEvent,
        state: &GameState,
        ctx: &mut ModuleCtx,
    ) {
        self.note_visits_source(state, je.timestamp, ctx);
        self.note_plan_cmdr(state, ctx);
        match &je.kind {
            EventKind::Loadout(l) => {
                self.model = Some(build_jump_model(l, fsd_table()));
                self.unladen_mass = l.unladen_mass;
            }
            EventKind::FsdJump(l) => {
                self.supercharged = None;
                self.politics = LocalPolitics::from_location(l).map(|p| (l.system_address, p));
                self.visits.on_jump(l.system_address, je.timestamp);
                if let Some(plan) = &mut self.plan {
                    plan.on_jump(&l.star_system, l.system_address);
                    self.plan_dirty = true;
                }
                // Navigation decide: solo salto en vivo con la copia automática activa.
                if !ctx.replaying && self.settings.auto_copy_next_system {
                    self.copy_seq += 1;
                    // El plan ya avanzó: el destino es el siguiente de la tira.
                    if let Some(next) = build_strip(self.plan.as_ref(), state).upcoming.first() {
                        ctx.alert(
                            Alert::new(Severity::Info, "nav-auto-copied")
                                .arg("system", next.name.clone()),
                        );
                    }
                }
            }
            EventKind::Location(l) | EventKind::CarrierJump(l) => {
                self.politics = LocalPolitics::from_location(l).map(|p| (l.system_address, p));
                if let Some(plan) = &mut self.plan {
                    let before = (plan.current_index, plan.off_route);
                    plan.reconcile(
                        &l.star_system,
                        (l.system_address != 0).then_some(l.system_address),
                    );
                    self.plan_dirty |= before != (plan.current_index, plan.off_route);
                }
            }
            EventKind::JetConeBoost(j) => self.supercharged = j.boost_value,
            _ => {}
        }
    }

    fn apply_response(&mut self, r: &NetResponse, strip: &Strip) {
        if r.module != "navigation" || !r.key.starts_with("system:") {
            return;
        }
        self.inflight.remove(&r.key);
        if !strip
            .visible()
            .any(|e| e.id64.is_some_and(|id| system_key(id) == r.key))
        {
            return;
        }
        if self
            .remote
            .get(&r.key)
            .and_then(|s| s.fetched_at)
            .is_some_and(|applied| r.fetched_at < applied)
        {
            return;
        }
        let (status, data) = match &r.outcome {
            NetOutcome::System(s) => (RemoteStatus::Ok, Some(s.clone())),
            NetOutcome::NoData => (RemoteStatus::NoData, None),
            NetOutcome::Disabled => (RemoteStatus::Disabled, None),
            NetOutcome::Failed(_) => (RemoteStatus::Failed, None),
            _ => return,
        };
        self.remote.insert(
            r.key.clone(),
            RemoteState {
                status,
                source: Some(r.source),
                fetched_at: Some(r.fetched_at),
                data,
            },
        );
    }

    fn request_missing(&mut self, strip: &Strip, state: &GameState, ctx: &mut ModuleCtx) {
        let keep: HashSet<String> = strip
            .visible()
            .filter_map(|e| e.id64.map(system_key))
            .collect();
        self.remote.retain(|k, _| keep.contains(k));
        self.inflight.retain(|k, _| keep.contains(k));
        if let Some(now) = self.now {
            self.remote.retain(|_, r| {
                r.status != RemoteStatus::Failed
                    || r.fetched_at
                        .is_none_or(|t| (now - t).num_seconds() < FAILED_COOLDOWN_S)
            });
            self.inflight
                .retain(|_, (_, sent)| (now - *sent).num_seconds() <= INFLIGHT_TTL_S);
        }
        if !self.online(state) {
            return;
        }
        for e in strip.visible() {
            let Some(id64) = e.id64 else { continue };
            let key = system_key(id64);
            if self.remote.contains_key(&key) || self.inflight.contains_key(&key) {
                continue;
            }
            let query = NetQuery::System {
                id64,
                name: e.name.clone(),
            };
            if let Some(id) = ctx.request(key.clone(), query) {
                self.inflight
                    .insert(key, (id, self.now.unwrap_or(DateTime::UNIX_EPOCH)));
            }
        }
    }

    /// Combustible restante (%) y alcance tras cada salto de los sistemas visibles
    /// por delante, sin contar repostajes intermedios.
    fn fuel_chain(
        &self,
        state: &GameState,
        upcoming: &[Entry],
    ) -> Vec<(Option<f64>, Option<bool>)> {
        let mut out = vec![(None, None); upcoming.len().min(STRIP_AHEAD)];
        let Some(model) = self
            .model
            .as_ref()
            .filter(|m| m.source == RangeSource::Formula)
        else {
            return out;
        };
        let (Some(fsd), Some(mut mass)) = (model.fsd.as_ref(), self.mass_now(state)) else {
            return out;
        };
        let Some(capacity) = state
            .ship
            .as_ref()
            .and_then(|s| s.fuel_capacity_main)
            .filter(|c| c.is_finite() && *c > 0.0)
        else {
            return out;
        };
        for (slot, e) in out.iter_mut().zip(upcoming) {
            let Some(leg) = e.leg_ly.filter(|_| e.jumps <= 1) else {
                break;
            };
            match fuel_for_jump(fsd, mass.total(), leg) {
                Some(burn) if burn <= mass.fuel_main => {
                    mass.fuel_main -= burn;
                    *slot = (Some(mass.fuel_main / capacity * 100.0), Some(true));
                }
                _ => {
                    *slot = (None, Some(false));
                    break;
                }
            }
        }
        out
    }

    fn card(
        &self,
        state: &GameState,
        e: &Entry,
        fuel: (Option<f64>, Option<bool>),
        is_current: bool,
    ) -> SystemCard {
        let flags = e.star_class.as_deref().map(star_flags);
        let remote = match e.id64 {
            _ if !self.online(state) => RemoteState::disabled(),
            Some(id) => self
                .remote
                .get(&system_key(id))
                .cloned()
                .unwrap_or_else(RemoteState::pending),
            None => RemoteState::pending(),
        };
        SystemCard {
            name: e.name.clone(),
            id64: e.id64,
            jump_ly: e.leg_ly,
            star_class: e.star_class.clone(),
            scoopable: flags.map(|f| f.scoopable),
            special: flags.and_then(|f| f.special),
            fuel_after_pct: fuel.0,
            reachable: fuel.1,
            visits: e.id64.and_then(|id| self.visits.get(id)),
            remote,
            local_politics: self
                .politics
                .as_ref()
                .filter(|(id, _)| is_current && Some(*id) == e.id64)
                .map(|(_, p)| p.clone()),
        }
    }

    fn render_route(&self, strip: &Strip, state: &GameState) -> Value {
        let fuel = self.fuel_chain(state, &strip.upcoming);
        let mut cards = strip
            .upcoming
            .iter()
            .take(STRIP_AHEAD)
            .zip(fuel)
            .map(|(e, f)| self.card(state, e, f, false));
        let next = cards.next();
        let topic = RouteTopic {
            source: strip.source,
            current: self.card(state, &strip.current, (None, None), true),
            copy_target: next.as_ref().map(|c| c.name.clone()),
            next,
            following: cards.collect(),
            remaining_jumps: strip.remaining_jumps,
            total_ly: strip.total_ly,
            off_route: strip.off_route,
            copy_seq: self.copy_seq,
        };
        serde_json::to_value(topic).unwrap_or_default()
    }

    fn render_jump(&self, state: &GameState) -> Value {
        let ship = state.ship.as_ref();
        let capacity = ship.and_then(|s| s.fuel_capacity_main);
        let range = |mass: Option<ShipMass>| {
            self.model
                .as_ref()
                .zip(mass)
                .and_then(|(m, mass)| m.range_now(&mass))
        };
        let full = self
            .unladen_mass
            .zip(capacity)
            .map(|(unladen, main)| ShipMass {
                unladen,
                fuel_main: main,
                fuel_reserve: 0.0,
                cargo: 0.0,
            });
        let topic = JumpTopic {
            range_ly: range(self.mass_now(state)),
            range_full_ly: range(full),
            source: self
                .model
                .as_ref()
                .map_or(RangeSource::Unknown, |m| m.source),
            fuel_main: ship.and_then(|s| s.fuel_main),
            fuel_capacity: capacity,
            max_fuel_per_jump: self
                .model
                .as_ref()
                .and_then(|m| m.fsd.as_ref())
                .map(|f| f.max_fuel_per_jump),
            supercharged: self.supercharged,
        };
        serde_json::to_value(topic).unwrap_or_default()
    }
}

impl Default for Navigation {
    fn default() -> Self {
        Self::new()
    }
}

/// Eventos del diario que pueden cambiar la tira o el alcance de salto.
fn affects_topics(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Fileheader(_)
            | EventKind::Commander(_)
            | EventKind::LoadGame(_)
            | EventKind::Location(_)
            | EventKind::FsdJump(_)
            | EventKind::CarrierJump(_)
            | EventKind::Docked(_)
            | EventKind::SupercruiseExit(_)
            | EventKind::FsdTarget(_)
            | EventKind::NavRouteClear
            | EventKind::Loadout(_)
            | EventKind::Cargo(_)
            | EventKind::FuelScoop(_)
            | EventKind::RefuelAll(_)
            | EventKind::RefuelPartial(_)
            | EventKind::ShipyardSwap(_)
            | EventKind::JetConeBoost(_)
    )
}

impl Module for Navigation {
    fn id(&self) -> &'static str {
        "navigation"
    }

    fn on_event(&mut self, ev: &SourceEvent, state: &GameState, ctx: &mut ModuleCtx) -> TopicSet {
        match ev {
            SourceEvent::Settings(s) => {
                self.settings = s.clone();
                // Los estados no Ok (p. ej. Disabled) no sobreviven a un cambio de ajustes.
                self.remote.retain(|_, r| r.status == RemoteStatus::Ok);
                self.on_planner_settings(state);
            }
            SourceEvent::Status(st) => self.tick(st.timestamp),
            SourceEvent::Journal { ev, .. } => {
                self.tick(ev.timestamp);
                self.on_journal(ev, state, ctx);
                self.sync_plan(ctx);
                if !self.route_value.is_null() && !affects_topics(&ev.kind) {
                    return TopicSet::new();
                }
            }
            SourceEvent::Stored { module, tag, value }
                if module == "navigation"
                    && tag == VISITS_TAG
                    && value.get("cmdr").and_then(Value::as_str) == self.visits.cmdr.as_deref() =>
            {
                self.visits.load(&value["rows"]);
            }
            SourceEvent::Stored { module, tag, value }
                if module == "navigation" && tag == planner::PLAN_TAG =>
            {
                self.apply_loaded_plan(value, state);
            }
            SourceEvent::Action { name, args } if name.starts_with("navigation.") => {
                self.on_plan_action(name, args, state, ctx);
            }
            _ => {}
        }
        self.sync_plan(ctx);
        if let SourceEvent::Net(response) = ev {
            self.apply_route_response(response);
        }
        let strip = build_strip(self.plan.as_ref(), state);
        if let SourceEvent::Net(response) = ev {
            self.apply_response(response, &strip);
        }
        // Las peticiones hechas durante el replay se descartan; se repiten aquí
        // en cuanto llega un evento en vivo.
        self.request_missing(&strip, state, ctx);

        let mut dirty = TopicSet::new();
        let route = self.render_route(&strip, state);
        if self.route_value != route {
            self.route_value = route;
            dirty.insert(TOPIC_ROUTE.to_owned());
        }
        let plan = self.render_plan(state);
        if self.plan_value != plan {
            self.plan_value = plan;
            dirty.insert(TOPIC_PLAN.to_owned());
        }
        let jump = self.render_jump(state);
        if self.jump_value != jump {
            self.jump_value = jump;
            dirty.insert(TOPIC_JUMP.to_owned());
        }
        dirty
    }

    fn migrations(&self) -> &'static [tmc_core::Migration] {
        persist::MIGRATIONS
    }

    fn topic_value(&self, topic: &str) -> Option<Value> {
        let v = match topic {
            TOPIC_ROUTE => &self.route_value,
            TOPIC_JUMP => &self.jump_value,
            TOPIC_PLAN => &self.plan_value,
            _ => return None,
        };
        (!v.is_null()).then(|| v.clone())
    }

    fn topics(&self) -> Vec<String> {
        vec![TOPIC_ROUTE.into(), TOPIC_JUMP.into(), TOPIC_PLAN.into()]
    }

    fn relevance(&self, state: &GameState) -> Relevance {
        let routed = self.plan.is_some() || !state.route.hops.is_empty();
        let travelling = matches!(
            state.location.mode,
            FlightMode::Supercruise | FlightMode::Hyperspace
        );
        if routed && travelling {
            Relevance::Active
        } else {
            Relevance::Normal
        }
    }
}

#[cfg(test)]
#[path = "strip_tests.rs"]
mod tests;
