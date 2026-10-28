//! Planificador de rutas Spansh (neutrones / galaxia) y ruta activa.

use super::Navigation;
use super::jump::{RangeSource, ShipMass, supercharge_multiplier};
use super::persist;
use super::route::{ActivePlan, PlanMode};
use crate::data::StarSpecial;
use serde::Serialize;
use serde_json::{Value, json};
use tmc_core::ModuleCtx;
use tmc_model::net::{
    GalaxyRouteParams, NetOutcome, NetQuery, NetResponse, NeutronRouteParams, PlannedHop,
    PlannedRoute, ShipParams,
};
use tmc_model::state::{Galaxy, GameState};

pub(super) const ROUTE_KEY: &str = "route";
pub(super) const PLAN_TAG: &str = "plan";
pub(super) const PLAN_TAG_SYNC: &str = "plan-sync";

const ERR_TOO_LONG: &str = "nav-plan-error-route-too-long";
const MAX_HOPS: usize = 2000;
const TOPIC_HOPS: usize = 50;
const MAX_NAME_CHARS: usize = 128;
const ERR_NO_ORIGIN: &str = "nav-plan-error-no-origin";
const ERR_BAD_REQUEST: &str = "nav-plan-error-bad-request";
const ERR_NO_RANGE: &str = "nav-plan-error-no-range";
const ERR_NO_FSD: &str = "nav-plan-error-no-fsd";
const ERR_NO_PLAN: &str = "nav-plan-error-no-plan";
const ERR_NO_ROUTE: &str = "nav-plan-error-no-route";
const ERR_SERVICE: &str = "nav-plan-error-service";
const ERR_UNKNOWN_ORIGIN: &str = "nav-plan-error-unknown-origin";
const ERR_UNKNOWN_DESTINATION: &str = "nav-plan-error-unknown-destination";
const DISABLED_LEGACY: &str = "nav-plan-disabled-legacy";
const DISABLED_OFFLINE: &str = "nav-plan-disabled-offline";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum PlanStatus {
    #[default]
    Idle,
    Computing,
    Ready,
    Active,
    Failed,
    Disabled,
}

/// Petición normalizada de `navigation.plan`.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Request {
    mode: PlanMode,
    to: String,
    from: String,
    to_id64: Option<u64>,
    from_id64: Option<u64>,
    efficiency: u8,
    use_ship: bool,
    range_ly: Option<f64>,
    refuel_scoopable: bool,
    use_injections: bool,
}

impl Request {
    fn to_json(&self) -> Value {
        json!({
            "mode": match self.mode { PlanMode::Neutron => "neutron", PlanMode::Galaxy => "galaxy" },
            "to": self.to,
            "from": self.from,
            "to_id64": self.to_id64,
            "from_id64": self.from_id64,
            "efficiency": self.efficiency,
            "use_ship": self.use_ship,
            "range_ly": self.range_ly,
            "refuel_scoopable": self.refuel_scoopable,
            "use_injections": self.use_injections,
        })
    }
}

#[derive(Default)]
pub(super) struct Planner {
    status: PlanStatus,
    request: Option<Request>,
    result: Option<PlannedRoute>,
    error_key: Option<&'static str>,
    /// Id de la última petición enviada; las respuestas con otro id son obsoletas.
    pub(super) req_id: Option<u64>,
}

#[derive(Serialize)]
struct ActiveTopic {
    destination: String,
    current_index: usize,
    total: usize,
    off_route: bool,
}

/// Resumen de la ruta calculada: totales y los primeros tramos.
#[derive(Serialize)]
struct ResultTopic {
    total_ly: f64,
    hops_total: usize,
    hops: Vec<PlannedHop>,
}

#[derive(Serialize)]
struct PlanTopic {
    status: PlanStatus,
    request: Option<Value>,
    result: Option<ResultTopic>,
    active: Option<ActiveTopic>,
    error_key: Option<&'static str>,
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn parse_request(
    args: &Value,
    state: &GameState,
    default_efficiency: u8,
) -> Result<Request, &'static str> {
    let name = |key: &str| -> Result<Option<&str>, &'static str> {
        match args.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(v)) if v.chars().count() <= MAX_NAME_CHARS => {
                Ok(Some(v.trim()).filter(|v| !v.is_empty()))
            }
            Some(_) => Err(ERR_BAD_REQUEST),
        }
    };
    let to = name("to")?.ok_or(ERR_BAD_REQUEST)?.to_owned();
    let from_arg = name("from")?;
    let mode = match str_arg(args, "mode") {
        Some("neutron") => PlanMode::Neutron,
        Some("galaxy") => PlanMode::Galaxy,
        _ => return Err(ERR_BAD_REQUEST),
    };
    let here = &state.location;
    let from = from_arg
        .or_else(|| Some(here.system.as_str()).filter(|s| !s.is_empty()))
        .ok_or(ERR_NO_ORIGIN)?
        .to_owned();
    let from_id64 = args.get("from_id64").and_then(Value::as_u64).or_else(|| {
        (here.system_address != 0 && from.eq_ignore_ascii_case(&here.system))
            .then_some(here.system_address)
    });
    let efficiency = match args.get("efficiency") {
        None | Some(Value::Null) => default_efficiency,
        Some(v) => match v.as_u64() {
            Some(e @ 0..=100) => e as u8,
            _ => return Err(ERR_BAD_REQUEST),
        },
    };
    let flag = |key: &str, default: bool| args.get(key).and_then(Value::as_bool).unwrap_or(default);
    Ok(Request {
        mode,
        to,
        from,
        to_id64: args.get("to_id64").and_then(Value::as_u64),
        from_id64,
        efficiency,
        use_ship: flag("use_ship", true),
        range_ly: args.get("range_ly").and_then(Value::as_f64),
        refuel_scoopable: flag("refuel_scoopable", true),
        use_injections: flag("use_injections", false),
    })
}

impl Navigation {
    /// Alcance con el depósito principal lleno y la carga actual.
    fn planning_range(&self, state: &GameState) -> Option<f64> {
        let model = self.model.as_ref()?;
        if model.source == RangeSource::Unknown {
            // FSD desconocido: la cifra del propio juego es la única fuente (nunca bloquea el planificador).
            return model.game_max_range.filter(|r| r.is_finite() && *r > 0.0);
        }
        let ship = state.ship.as_ref()?;
        let mass = ShipMass {
            unladen: self.unladen_mass?,
            fuel_main: ship.fuel_capacity_main?,
            fuel_reserve: 0.0,
            cargo: f64::from(ship.cargo_count.unwrap_or(0)),
        };
        model.range_now(&mass).filter(|r| r.is_finite() && *r > 0.0)
    }

    fn build_query(&self, req: &Request, state: &GameState) -> Result<NetQuery, &'static str> {
        let fsd = self.model.as_ref().and_then(|m| m.fsd.as_ref());
        match req.mode {
            PlanMode::Neutron => {
                let range_ly = if req.use_ship {
                    self.planning_range(state)
                } else {
                    req.range_ly.filter(|r| r.is_finite() && *r > 0.0)
                }
                .ok_or(ERR_NO_RANGE)?;
                let supercharge_multiplier = fsd.map_or(4.0, |f| {
                    supercharge_multiplier(f.kind, StarSpecial::Neutron)
                });
                let from_coords = if state.location.system.eq_ignore_ascii_case(&req.from)
                    || (state.location.system_address != 0
                        && req.from_id64 == Some(state.location.system_address))
                {
                    let p = state.location.star_pos;
                    (p[0] != 0.0 || p[1] != 0.0 || p[2] != 0.0).then_some(p)
                } else {
                    None
                };
                Ok(NetQuery::NeutronRoute(NeutronRouteParams {
                    from: req.from.clone(),
                    to: req.to.clone(),
                    range_ly,
                    efficiency: req.efficiency,
                    supercharge_multiplier,
                    from_coords,
                }))
            }
            PlanMode::Galaxy => {
                let fsd = fsd.ok_or(ERR_NO_FSD)?;
                let to = req
                    .to_id64
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| req.to.clone());
                let from = req
                    .from_id64
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| req.from.clone());
                let ship = state.ship.as_ref().ok_or(ERR_NO_RANGE)?;
                let (Some(unladen_mass), Some(fuel_main_capacity)) =
                    (self.unladen_mass, ship.fuel_capacity_main)
                else {
                    return Err(ERR_NO_RANGE);
                };
                let from_coords = if state.location.system.eq_ignore_ascii_case(&req.from)
                    || (state.location.system_address != 0
                        && req.from_id64 == Some(state.location.system_address))
                {
                    let p = state.location.star_pos;
                    (p[0] != 0.0 || p[1] != 0.0 || p[2] != 0.0).then_some(p)
                } else {
                    None
                };
                Ok(NetQuery::GalaxyRoute(GalaxyRouteParams {
                    from,
                    to,
                    ship: ShipParams {
                        unladen_mass,
                        fuel_main_capacity,
                        fuel_reserve: ship.fuel_capacity_reserve.unwrap_or(0.0),
                        cargo: f64::from(ship.cargo_count.unwrap_or(0)),
                        optimal_mass: fsd.optimal_mass,
                        max_fuel_per_jump: fsd.max_fuel_per_jump,
                        fuel_mul: fsd.fuel_mul,
                        fuel_power: fsd.fuel_power,
                        guardian_bonus_ly: fsd.guardian_bonus_ly,
                        supercharge_multiplier: supercharge_multiplier(
                            fsd.kind,
                            StarSpecial::Neutron,
                        ),
                    },
                    refuel_scoopable: req.refuel_scoopable,
                    use_injections: req.use_injections,
                    algorithm: Some("optimistic".into()),
                    from_coords,
                }))
            }
        }
    }

    fn plan_fail(&mut self, key: &'static str) {
        let p = &mut self.planner;
        p.status = PlanStatus::Failed;
        p.error_key = Some(key);
        p.result = None;
        p.req_id = None;
    }

    fn disabled_key(&self, state: &GameState) -> Option<&'static str> {
        if state.galaxy == Galaxy::Legacy {
            Some(DISABLED_LEGACY)
        } else if !self.settings.online_services {
            Some(DISABLED_OFFLINE)
        } else {
            None
        }
    }

    fn start_plan(&mut self, args: &Value, state: &GameState, ctx: &mut ModuleCtx) {
        if let Some(key) = self.disabled_key(state) {
            let p = &mut self.planner;
            *p = Planner::default();
            p.status = PlanStatus::Disabled;
            p.error_key = Some(key);
            return;
        }
        let req = match parse_request(args, state, self.settings.default_neutron_efficiency) {
            Ok(r) => r,
            Err(key) => return self.plan_fail(key),
        };
        let query = match self.build_query(&req, state) {
            Ok(q) => q,
            Err(key) => {
                self.planner.request = Some(req);
                return self.plan_fail(key);
            }
        };
        match ctx.request(ROUTE_KEY, query) {
            Some(id) => {
                decide!(
                    "navigation.planner.request.accepted",
                    tmc_model::diagnostics::json!({"request_id": id})
                );
                self.planner = Planner {
                    status: PlanStatus::Computing,
                    request: Some(req),
                    result: None,
                    error_key: None,
                    req_id: Some(id),
                };
            }
            None => {
                decide!(
                    "navigation.planner.request.rejected",
                    tmc_model::diagnostics::json!({})
                );
                self.planner.request = Some(req);
                self.plan_fail(ERR_SERVICE);
            }
        }
    }

    fn recalculate(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        let Some(plan) = &self.plan else {
            return self.plan_fail(ERR_NO_PLAN);
        };
        let mut args = match &plan.params {
            Value::Object(_) => plan.params.clone(),
            _ => json!({}),
        };
        args["mode"] = json!(match plan.mode {
            PlanMode::Neutron => "neutron",
            PlanMode::Galaxy => "galaxy",
        });
        let here = &state.location;
        args["from"] = json!(here.system);
        args["from_id64"] = json!((here.system_address != 0).then_some(here.system_address));
        args["to"] = json!(plan.destination);
        if args.get("to_id64").is_none_or(Value::is_null) {
            args["to_id64"] = json!(plan.hops.last().and_then(|h| h.id64));
        }
        self.start_plan(&args, state, ctx);
    }

    fn accept_plan(&mut self, state: &GameState) {
        let (Some(req), Some(route)) = (&self.planner.request, &self.planner.result) else {
            return;
        };
        if self.planner.status != PlanStatus::Ready || route.hops.len() < 2 {
            return;
        }
        let mut plan = ActivePlan {
            created_at: self.now.unwrap_or_else(chrono::Utc::now),
            origin: req.from.clone(),
            destination: req.to.clone(),
            mode: req.mode,
            params: req.to_json(),
            hops: route.hops.clone(),
            current_index: 0,
            off_route: false,
        };
        let here = &state.location;
        if !here.system.is_empty() {
            plan.reconcile(
                &here.system,
                (here.system_address != 0).then_some(here.system_address),
            );
        }
        decide!("navigation.plan.accepted", {"hops": plan.hops.len(), "index": plan.current_index});
        self.plan = Some(plan);
        self.plan_dirty = true;
        self.load_pending = false;
        self.planner = Planner::default();
    }

    /// Un cambio de ajustes puede activar o desactivar el planificador.
    pub(super) fn on_planner_settings(&mut self, state: &GameState) {
        let key = self.disabled_key(state);
        let p = &mut self.planner;
        match (p.status, key) {
            (PlanStatus::Disabled, None) => {
                p.status = PlanStatus::Idle;
                p.error_key = None;
            }
            (PlanStatus::Computing, Some(key)) => {
                p.status = PlanStatus::Disabled;
                p.error_key = Some(key);
                p.req_id = None;
            }
            _ => {}
        }
    }

    pub(super) fn on_plan_action(
        &mut self,
        name: &str,
        args: &Value,
        state: &GameState,
        ctx: &mut ModuleCtx,
    ) {
        match name {
            "navigation.plan" => self.start_plan(args, state, ctx),
            "navigation.recalculate" => self.recalculate(state, ctx),
            "navigation.accept-plan" => self.accept_plan(state),
            "navigation.clear-plan" => {
                self.plan = None;
                self.plan_dirty = true;
                self.load_pending = false;
                self.planner = Planner::default();
            }
            _ => {}
        }
    }

    /// Aplica la respuesta de ruta; idempotente por id (los reintentos de red
    /// pueden repetir `Failed`) e ignora respuestas obsoletas.
    pub(super) fn apply_route_response(&mut self, r: &NetResponse) {
        if r.module != "navigation" || r.key != ROUTE_KEY || self.planner.req_id != Some(r.id) {
            return;
        }
        let waiting = matches!(
            self.planner.status,
            PlanStatus::Computing | PlanStatus::Failed
        );
        if !waiting {
            return;
        }
        let keep = self.planner.req_id;
        match &r.outcome {
            NetOutcome::Route(route) if route.hops.len() > MAX_HOPS => self.plan_fail(ERR_TOO_LONG),
            NetOutcome::Route(route) if !route.hops.is_empty() => {
                self.planner.status = PlanStatus::Ready;
                self.planner.result = Some(route.clone());
                self.planner.error_key = None;
            }
            NetOutcome::Route(_) | NetOutcome::NoData => self.plan_fail(ERR_NO_ROUTE),
            NetOutcome::Disabled => {
                self.planner.status = PlanStatus::Disabled;
                self.planner.error_key = Some(DISABLED_OFFLINE);
            }
            NetOutcome::Failed(tmc_model::net::NetFailure::UnknownOrigin) => {
                self.plan_fail(ERR_UNKNOWN_ORIGIN)
            }
            NetOutcome::Failed(tmc_model::net::NetFailure::UnknownDestination) => {
                self.plan_fail(ERR_UNKNOWN_DESTINATION)
            }
            NetOutcome::Failed(tmc_model::net::NetFailure::Http(400)) => {
                self.plan_fail(ERR_BAD_REQUEST)
            }
            NetOutcome::Failed(_) => self.plan_fail(ERR_SERVICE),
            _ => return,
        }
        decide!("navigation.planner.result", {"ok": self.planner.status == PlanStatus::Ready,
            "disabled": self.planner.status == PlanStatus::Disabled,
            "jumps": self.planner.result.as_ref().map(|r| r.hops.len())});
        self.planner.req_id = keep;
    }

    /// Persistencia de la ruta activa. No escribe durante el replay: los
    /// cambios se acumulan y se vuelcan en el primer evento en vivo.
    pub(super) fn sync_plan(&mut self, ctx: &mut ModuleCtx) {
        if !self.plan_dirty || ctx.replaying {
            return;
        }
        self.plan_dirty = false;
        if self.plan.as_ref().is_some_and(ActivePlan::arrived) {
            self.plan = None;
        }
        let Some(cmdr) = self.plan_cmdr.clone() else {
            return;
        };
        let task = match self.plan.clone() {
            Some(plan) => persist::save_plan(cmdr, plan),
            None => persist::delete_plan(cmdr),
        };
        decide!(
            "navigation.plan.sync.queued",
            tmc_model::diagnostics::json!({"has_plan": self.plan.is_some()})
        );
        ctx.store_task(PLAN_TAG_SYNC, task);
    }

    /// Cambio de comandante: se descarta el plan en memoria y se carga el suyo.
    pub(super) fn note_plan_cmdr(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        let Some(name) = state
            .cmdr
            .as_ref()
            .map(|c| c.name.clone())
            .filter(|n| !n.is_empty())
        else {
            return;
        };
        if self.plan_cmdr.as_deref() == Some(name.as_str()) {
            return;
        }
        self.plan_cmdr = Some(name.clone());
        self.plan = None;
        self.plan_dirty = false;
        self.planner = Planner::default();
        self.load_pending = true;
        ctx.store_task(PLAN_TAG, persist::load_plan(name));
    }

    /// Plan restaurado del almacén; se concilia con el sistema actual.
    pub(super) fn apply_loaded_plan(&mut self, value: &Value, state: &GameState) {
        if value.get("cmdr").and_then(Value::as_str) != self.plan_cmdr.as_deref()
            || self.plan.is_some()
            || !self.load_pending
        {
            decide!("navigation.plan.load.discarded", {"pending": self.load_pending,
                "has_plan": self.plan.is_some()});
            return;
        }
        self.load_pending = false;
        let Some(mut plan) = value
            .get("plan")
            .filter(|p| !p.is_null())
            .and_then(|p| serde_json::from_value::<ActivePlan>(p.clone()).ok())
        else {
            decide!("navigation.plan.load.none", {});
            return;
        };
        if !plan.params.is_object() {
            plan.params = json!({});
        }
        let here = &state.location;
        if !here.system.is_empty() {
            plan.reconcile(
                &here.system,
                (here.system_address != 0).then_some(here.system_address),
            );
            // Puede haber llegado o cambiado de índice: se vuelca al sincronizar.
            self.plan_dirty = true;
        }
        decide!("navigation.plan.loaded", {"hops": plan.hops.len(), "index": plan.current_index,
            "off_route": plan.off_route});
        self.plan = Some(plan);
    }

    pub(super) fn render_plan(&self, state: &GameState) -> Value {
        let p = &self.planner;
        let mut status = p.status;
        let mut error_key = p.error_key;
        if status == PlanStatus::Idle {
            if self.plan.is_some() {
                status = PlanStatus::Active;
            } else if let Some(key) = self.disabled_key(state) {
                status = PlanStatus::Disabled;
                error_key = Some(key);
            }
        }
        let topic = PlanTopic {
            status,
            request: p.request.as_ref().map(Request::to_json),
            result: p.result.as_ref().map(|r| ResultTopic {
                total_ly: r.total_ly,
                hops_total: r.hops.len(),
                hops: r.hops.iter().take(TOPIC_HOPS).cloned().collect(),
            }),
            active: self.plan.as_ref().map(|a| ActiveTopic {
                destination: a.destination.clone(),
                current_index: a.current_index,
                total: a.hops.len(),
                off_route: a.off_route,
            }),
            error_key,
        };
        serde_json::to_value(topic).unwrap_or_default()
    }
}
