//! Módulo `missions`: libro de misiones activas, masacre, avisos, recompensas,
//! pasajeros, turismo y objetivos comunitarios.
//!
//! El evento `Missions` manda sobre la lista activa; los detalles de cada misión
//! salen de `MissionAccepted` (en vivo o desde el historial). Las bajas de las
//! masacres no se cuentan aquí (las lleva `MassacreStack`).
//!
//! Topics (todos con `as_of`, el último instante del diario; se publican solo si
//! cambian y sin campos que dependan del reloj: las cuentas atrás las calcula la UI
//! con `core.clock` y las caducidades publicadas): `missions.active`,
//! `missions.massacre`, `missions.alerts`, `missions.rewards`, `missions.passengers`,
//! `missions.tourism` y `missions.community_goals`.
//!
//! Reconstrucción encadenada: `Missions` → (1) lectura `missions.accepted`; solo al
//! aplicar su resultado (comandante comprobado) → (2) `missions.bounties` desde la
//! aceptación de masacre más antigua y (3) `missions.passenger_stats`.
//!
//! Turismo: el diario no indica si la misión exige volver al origen, así que el plan
//! no incluye tramo de regreso (`return_to = None`); tras `MissionRedirected` se
//! recalcula desde la posición actual al nuevo destino y la distancia se acumula.
//!
//! «A la ruta» (`missions.to_route`): copia el sistema al portapapeles (`copy_target`
//! y `copy_seq` en `missions.active`); un módulo no puede invocar a otro, así que la
//! UI puede enviar además `navigation.plan` para abrir el planificador de F1.
//!
//! Red y almacén: nada en replay. Coordenadas por red (`SystemCoordsByName`) nunca
//! en Legacy ni sin «servicios online». Las escrituras llevan la etiqueta `*.write`
//! y tampoco ocurren en Legacy.

mod alerts;
mod book;
mod coords;
mod massacre;
mod passengers;
mod rewards;
mod tourism;

pub use alerts::{AlertKind, Here, MissionAlert, alert_from_change, evaluate_alerts};
pub use book::{
    BookChange, Mission, MissionBook, MissionKind, MissionProgress, accepted_from_result,
    accepted_rebuild_task, classify_kind,
};
pub use coords::{
    CoordResolver, MAX_COORDS_INFLIGHT, TAG_COORDS, coord_key, coords_from_result, coords_task,
};
pub use massacre::{IssuerBar, MASSACRE_COUNTS_KILL_BONDS, MassacrePile, MassacreStack};
pub use passengers::{CabinOccupancy, Cabins, PassengerGroup, groups, wanted_aboard};
pub use rewards::{CgView, CommunityGoals, FactionDelta, SessionRewards};
pub use tourism::{
    EXACT_MAX_STOPS, PASSENGER_STATS_MIGRATION, PlanInput, TAG_PASSENGER_STATS,
    TAG_STATS_COMPLETE_WRITE, TAG_STATS_WRITE, TourismPlan, Verdict, own_reference,
    plan as tourism_plan, rows_from_result, stats_complete_task, stats_read_task, stats_write_task,
};

use crate::history::{Position, for_each_after};
use crate::inflight::Inflight;
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashSet};
use tmc_core::{Alert, Migration, Module, ModuleCtx, Relevance, Severity, StoreTask, TopicSet};
use tmc_model::companion::CompanionFile;
use tmc_model::events::{CargoItem, EventKind, Passengers, parse_line};
use tmc_model::net::{NetOutcome, NetQuery, NetResponse};
use tmc_model::settings::ModuleSettings;
use tmc_model::source::SourceEvent;
use tmc_model::state::{FlightMode, Galaxy, GameState};

const TOPIC_ACTIVE: &str = "missions.active";
const TOPIC_MASSACRE: &str = "missions.massacre";
const TOPIC_ALERTS: &str = "missions.alerts";
const TOPIC_REWARDS: &str = "missions.rewards";
const TOPIC_PASSENGERS: &str = "missions.passengers";
const TOPIC_TOURISM: &str = "missions.tourism";
const TOPIC_CG: &str = "missions.community_goals";
const TOPICS: [&str; 7] = [
    TOPIC_ACTIVE,
    TOPIC_MASSACRE,
    TOPIC_ALERTS,
    TOPIC_REWARDS,
    TOPIC_PASSENGERS,
    TOPIC_TOURISM,
    TOPIC_CG,
];

pub const TAG_ACCEPTED: &str = "missions.accepted";
pub const TAG_BOUNTIES: &str = "missions.bounties";

const ACTION_TO_ROUTE: &str = "missions.to_route";
const ACTION_DISMISS: &str = "missions.dismiss_alert";
const ACTION_RESET: &str = "missions.reset_session";

const ACCEPTED_KEY: &str = "accepted";
const COORDS_PREFIX: &str = "coords:";
/// Avisos conservados como máximo (los más antiguos se descartan).
const MAX_ALERTS: usize = 20;
/// Eventos en vivo guardados mientras se reconstruye la masacre.
const LIVE_BUFFER_CAP: usize = 500;

static MIGRATIONS: [Migration; 1] = [PASSENGER_STATS_MIGRATION];

/// Lectura (`missions.bounties`) de las bajas y redirecciones posteriores a `since`.
pub fn bounty_rebuild_task(cmdr: String, since: DateTime<Utc>) -> StoreTask {
    Box::new(move |store| {
        let mut names = vec!["Bounty", "MissionRedirected"];
        if MASSACRE_COUNTS_KILL_BONDS {
            names.push("FactionKillBond");
        }
        let cursor = Position {
            id: 0,
            ts: since.to_rfc3339_opts(SecondsFormat::Nanos, true),
            file: String::new(),
            line: 0,
            raw: String::new(),
        };
        let mut events = Vec::new();
        for_each_after(store, &cmdr, i64::MAX, &names, Some(cursor), |p| {
            events.push(json!({"ts": p.ts, "json": p.raw}));
        })?;
        Ok(json!({"cmdr": cmdr, "since": since, "events": events}))
    })
}

fn journal_cargo(items: &[tmc_model::companion::CargoItem]) -> Vec<CargoItem> {
    items
        .iter()
        .map(|i| CargoItem {
            name: i.name.clone(),
            name_localised: i.name_localised.clone(),
            count: i.count,
            stolen: Some(i.stolen),
            mission_id: i.mission_id,
        })
        .collect()
}

fn finite(p: &[f64; 3]) -> bool {
    p.iter().all(|v| v.is_finite())
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Destino de una misión: el único o la primera parada.
fn destination(m: &Mission) -> Option<String> {
    m.destination_system
        .clone()
        .or_else(|| m.stops.first().cloned())
        .filter(|s| !s.trim().is_empty())
}

/// Paradas para el cálculo de turismo: las del tablón o el destino único.
fn trip_stops(m: &Mission) -> Vec<String> {
    if m.stops.is_empty() {
        m.destination_system.clone().into_iter().collect()
    } else {
        m.stops.clone()
    }
}

/// Misión de pasajeros seguida para el turismo y las estadísticas propias.
#[derive(Debug, Clone)]
struct Trip {
    start: Option<[f64; 3]>,
    start_market: Option<u64>,
    reward: u64,
    accepted_at: DateTime<Utc>,
    plan: Option<TourismPlan>,
    written: bool,
    /// Reconstruida del historial: la fila solo se inserta si aún no existe.
    rebuilt: bool,
    /// Distancia ya cubierta por planes anteriores (antes de una redirección).
    base_ly: f64,
    /// La fila escrita debe actualizarse con la nueva distancia total.
    rewrite: bool,
}

impl Trip {
    fn total_ly(&self) -> Option<f64> {
        self.plan.as_ref().map(|p| self.base_ly + p.optimal_ly)
    }
}

/// Escritura (`missions.stats.write`) que no pisa una fila existente (viajes reconstruidos).
fn stats_insert_missing_task(
    cmdr: String,
    mission_id: u64,
    accepted_at: String,
    reward: u64,
    route_ly: f64,
) -> StoreTask {
    Box::new(move |store| {
        let bad = |e: std::num::TryFromIntError| tmc_core::StoreError::Unsupported(e.to_string());
        let id = i64::try_from(mission_id).map_err(bad)?;
        let r = i64::try_from(reward).map_err(bad)?;
        store.exec_json(
            "INSERT INTO missions_passenger_stats (mission_id, cmdr, accepted_at, reward, route_ly) \
             VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(mission_id) DO NOTHING",
            &[json!(id), json!(cmdr), json!(accepted_at), json!(r), json!(route_ly)],
        )?;
        Ok(json!({"mission_id": mission_id}))
    })
}

/// Un fallo de red al pedir coordenadas se reintenta pasado este tiempo.
const COORDS_RETRY_S: i64 = 300;

pub struct Missions {
    settings: ModuleSettings,
    now: Option<DateTime<Utc>>,
    cmdr: Option<String>,
    book: MissionBook,
    stack: MassacreStack,
    rewards: SessionRewards,
    cgs: CommunityGoals,
    cabins: Cabins,
    manifest: Option<Passengers>,
    inventory: Vec<CargoItem>,
    alerts: Vec<MissionAlert>,
    already: HashSet<(u64, u8, i64)>,
    resolver: CoordResolver,
    coords_pending: HashSet<String>,
    trips: BTreeMap<u64, Trip>,
    stats_rows: Option<Vec<(u64, f64)>>,
    needs_rebuild: bool,
    /// Bajas/redirecciones en vivo vistas desde que empezó la reconstrucción.
    rebuilding: bool,
    live_buffer: Vec<(EventKind, DateTime<Utc>)>,
    inflight: Inflight,
    copy_target: Option<String>,
    copy_seq: u64,
    /// Última redirección aplicada en vivo por misión (gana a las del historial).
    live_redirects: std::collections::HashMap<u64, DateTime<Utc>>,
    /// Reintento de coordenadas tras un fallo de red.
    coords_retry: std::collections::HashMap<String, DateTime<Utc>>,
    /// Ya se registró que la red está desactivada (un aviso por cambio de estado).
    offline_logged: bool,
    values: std::collections::HashMap<&'static str, Value>,
}

impl Missions {
    pub fn new() -> Self {
        Self {
            settings: ModuleSettings::default(),
            now: None,
            cmdr: None,
            book: MissionBook::default(),
            stack: MassacreStack::default(),
            rewards: SessionRewards::default(),
            cgs: CommunityGoals::default(),
            cabins: Cabins::default(),
            manifest: None,
            inventory: Vec::new(),
            alerts: Vec::new(),
            already: HashSet::new(),
            resolver: CoordResolver::default(),
            coords_pending: HashSet::new(),
            trips: BTreeMap::new(),
            stats_rows: None,
            needs_rebuild: false,
            rebuilding: false,
            live_buffer: Vec::new(),
            inflight: Inflight::default(),
            copy_target: None,
            copy_seq: 0,
            live_redirects: std::collections::HashMap::new(),
            coords_retry: std::collections::HashMap::new(),
            offline_logged: false,
            values: std::collections::HashMap::new(),
        }
    }

    pub fn migrations() -> &'static [Migration] {
        &MIGRATIONS
    }

    fn tick(&mut self, at: DateTime<Utc>) {
        if self.now.is_none() {
            self.rewards.reset(at);
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

    fn can_write(&self, state: &GameState, ctx: &ModuleCtx) -> bool {
        !ctx.replaying && state.galaxy != Galaxy::Legacy && self.cmdr.is_some()
    }

    fn position(state: &GameState) -> Option<[f64; 3]> {
        let l = &state.location;
        (!l.system.trim().is_empty() && finite(&l.star_pos)).then_some(l.star_pos)
    }

    fn stored_cmdr_ok(&self, value: &Value) -> bool {
        self.cmdr.is_some() && value.get("cmdr").and_then(Value::as_str) == self.cmdr.as_deref()
    }

    /// Un cambio de comandante lo descarta todo.
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
        decide!("missions.cmdr.changed", {"switched": switched});
        if !switched {
            return;
        }
        self.book.clear();
        self.stack.clear();
        self.rewards.reset(self.clock());
        self.cgs = CommunityGoals::default();
        self.cabins.reset_occupancy();
        self.manifest = None;
        self.inventory.clear();
        self.alerts.clear();
        self.already.clear();
        self.resolver.reset_requests();
        self.coords_pending.clear();
        self.trips.clear();
        self.stats_rows = None;
        self.needs_rebuild = false;
        self.rebuilding = false;
        self.live_buffer.clear();
        self.inflight.finish(ACCEPTED_KEY);
        self.inflight.finish_prefix(COORDS_PREFIX);
        self.copy_target = None;
        self.live_redirects.clear();
        self.coords_retry.clear();
    }

    // ----- Reconstrucción -----

    fn ensure_rebuild(&mut self, ctx: &mut ModuleCtx) {
        if ctx.replaying || !self.needs_rebuild {
            return;
        }
        let Some(cmdr) = self.cmdr.clone() else {
            return;
        };
        if !self.inflight.begin(ACCEPTED_KEY, self.clock()) {
            return;
        }
        let ids: Vec<u64> = self.book.active().map(|m| m.id).collect();
        decide!("missions.accepted.requested", {"ids": ids.len()});
        self.rebuilding = true;
        self.live_buffer.clear();
        ctx.store_task(TAG_ACCEPTED, accepted_rebuild_task(cmdr, ids));
    }

    fn on_accepted_stored(
        &mut self,
        value: &Value,
        state: &GameState,
        ctx: &mut ModuleCtx,
    ) -> bool {
        if !self.stored_cmdr_ok(value) {
            decide!("missions.accepted.discarded", {});
            return false;
        }
        let cmdr = self.cmdr.clone().unwrap_or_default();
        self.inflight.finish(ACCEPTED_KEY);
        self.needs_rebuild = false;
        self.book.on_history_result(&cmdr, value);
        // Viajes de pasajeros activos tras un reinicio: el plan sale de la posición actual.
        let rebuilt: Vec<(u64, u64, DateTime<Utc>)> = self
            .book
            .active()
            .filter(|m| matches!(m.kind, MissionKind::Passenger | MissionKind::Sightseeing))
            .filter(|m| !self.trips.contains_key(&m.id))
            .filter_map(|m| Some((m.id, m.reward.unwrap_or(0), m.accepted_at?)))
            .collect();
        decide!("missions.trips.rebuilt", {"trips": rebuilt.len()});
        for (id, reward, at) in rebuilt {
            self.trips.insert(
                id,
                Trip {
                    start: Self::position(state),
                    start_market: None,
                    reward,
                    accepted_at: at,
                    plan: None,
                    written: false,
                    rebuilt: true,
                    base_ly: 0.0,
                    rewrite: false,
                },
            );
        }
        let since = self
            .book
            .active()
            .filter(|m| m.kind == MissionKind::Massacre)
            .filter_map(|m| m.accepted_at)
            .min();
        decide!("missions.accepted.applied", {"massacre": since.is_some()});
        match since {
            Some(since) => ctx.store_task(TAG_BOUNTIES, bounty_rebuild_task(cmdr.clone(), since)),
            None => {
                self.rebuilding = false;
                self.live_buffer.clear();
            }
        }
        ctx.store_task(TAG_PASSENGER_STATS, stats_read_task(cmdr));
        true
    }

    fn on_bounties_stored(&mut self, value: &Value) -> bool {
        if !self.stored_cmdr_ok(value) {
            decide!("missions.bounties.discarded", {});
            return false;
        }
        self.stack.clear();
        let mut last: Option<DateTime<Utc>> = None;
        let mut applied = 0usize;
        for e in value["events"].as_array().into_iter().flatten() {
            let Some(ev) = e["json"].as_str().and_then(|j| parse_line(j).ok()) else {
                continue;
            };
            self.stack.apply_event(&ev.kind, ev.timestamp, &self.book);
            // Una redirección del historial no pisa otra igual o más reciente vista en vivo.
            if let EventKind::MissionRedirected(r) = &ev.kind {
                let stale = self
                    .live_redirects
                    .get(&r.mission_id)
                    .is_some_and(|live| *live >= ev.timestamp);
                if stale {
                    decide!("missions.bounties.redirect_skipped", {"mission": r.mission_id});
                } else {
                    self.book.apply_event(&ev.kind, ev.timestamp);
                }
            }
            last = Some(last.map_or(ev.timestamp, |l| l.max(ev.timestamp)));
            applied += 1;
        }
        // Lo visto en vivo después del último evento guardado se vuelve a aplicar.
        let buffered = std::mem::take(&mut self.live_buffer);
        for (kind, at) in buffered {
            // Las redirecciones son idempotentes en la pila: se aplican siempre.
            if last.is_none_or(|l| at > l) || matches!(kind, EventKind::MissionRedirected(_)) {
                self.stack.apply_event(&kind, at, &self.book);
            }
        }
        self.rebuilding = false;
        let _ = applied; // solo lo lee el diagnóstico
        decide!("missions.bounties.applied", {"events": applied});
        true
    }

    fn on_stats_stored(&mut self, value: &Value) -> bool {
        if !self.stored_cmdr_ok(value) {
            return false;
        }
        self.stats_rows = Some(rows_from_result(value));
        // La referencia cambió: los planes se recalculan.
        for t in self.trips.values_mut() {
            t.plan = None;
        }
        decide!("missions.stats.applied", {"rows": self.stats_rows.as_ref().map_or(0, Vec::len)});
        true
    }

    // ----- Coordenadas -----

    fn wanted_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.book.active().filter_map(destination).collect();
        for id in self.trips.keys() {
            if let Some(m) = self.book.get(*id) {
                names.extend(trip_stops(m));
            }
        }
        names.sort();
        names.dedup();
        names
    }

    fn request_coords(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        if ctx.replaying {
            return;
        }
        let Some(cmdr) = self.cmdr.clone() else {
            return;
        };
        let names = self.wanted_names();
        let local = self.resolver.need(&names);
        if !local.is_empty() {
            decide!("missions.coords.local_requested", {"names": local.len()});
            ctx.store_task(TAG_COORDS, coords_task(cmdr, local));
        }
        let candidates = self.resolver.remote_candidates();
        if candidates.is_empty() {
            return;
        }
        if !self.online(state) {
            if !self.offline_logged {
                self.offline_logged = true;
                decide!("missions.coords.skipped_offline", {"names": candidates.len()});
            }
            return;
        }
        self.offline_logged = false;
        let now = self.clock();
        for name in candidates {
            if self.coords_pending.len() >= MAX_COORDS_INFLIGHT {
                break;
            }
            let key = format!("{COORDS_PREFIX}{}", coord_key(&name));
            if self.coords_retry.get(&key).is_some_and(|at| now < *at) {
                continue;
            }
            if self.coords_pending.contains(&key) || !self.inflight.begin(&key, now) {
                continue;
            }
            let accepted = ctx
                .request(key.clone(), NetQuery::SystemCoordsByName { name })
                .is_some();
            if accepted {
                self.coords_pending.insert(key);
            } else {
                self.inflight.finish(&key);
            }
            decide!("missions.coords.remote_sent", {"accepted": accepted});
        }
    }

    fn on_coords_stored(&mut self, value: &Value) -> bool {
        let (names, map) = coords_from_result(value);
        if !self.stored_cmdr_ok(value) {
            // Descartado: esos nombres se volverán a preguntar al almacén.
            self.resolver.forget_asked(&names);
            decide!("missions.coords.local_discarded", {"asked": names.len()});
            return false;
        }
        decide!("missions.coords.local_applied", {"asked": names.len(), "found": map.len()});
        self.resolver.on_local_for(&names, map);
        true
    }

    fn on_net(&mut self, r: &NetResponse) -> bool {
        let Some(k) = r.key.strip_prefix(COORDS_PREFIX) else {
            return false;
        };
        self.inflight.finish(&r.key);
        self.coords_pending.remove(&r.key);
        match &r.outcome {
            NetOutcome::Coords(c) => {
                self.coords_retry.remove(&r.key);
                self.resolver.learn(k, c.star_pos);
                self.resolver.on_remote(c);
                decide!("missions.coords.remote_applied", {});
            }
            NetOutcome::NoData => {
                self.resolver.mark_unresolvable(k);
                decide!("missions.coords.remote_missing", {});
            }
            _ => {
                // Fallo, límite o servicio desactivado: se reintenta más tarde.
                let retry = self.clock() + chrono::Duration::seconds(COORDS_RETRY_S);
                self.coords_retry.insert(r.key.clone(), retry);
                decide!("missions.coords.remote_failed", {"retry_s": COORDS_RETRY_S});
            }
        }
        true
    }

    // ----- Turismo -----

    fn start_trip(&mut self, id: u64, at: DateTime<Utc>, state: &GameState) {
        let Some(m) = self.book.get(id) else {
            return;
        };
        if !matches!(m.kind, MissionKind::Passenger | MissionKind::Sightseeing) {
            return;
        }
        self.trips.insert(
            id,
            Trip {
                start: Self::position(state),
                start_market: state.location.market_id,
                reward: m.reward.unwrap_or(0),
                accepted_at: at,
                plan: None,
                written: false,
                rebuilt: false,
                base_ly: 0.0,
                rewrite: false,
            },
        );
    }

    fn compute_trips(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        let own = self.stats_rows.as_deref().and_then(own_reference);
        let setting = self.settings.tourism_reference_cr_ly as f64;
        let jump_range = state.ship.as_ref().and_then(|s| s.max_jump_range);
        let write = self.can_write(state, ctx);
        let cmdr = self.cmdr.clone().unwrap_or_default();
        let ids: Vec<u64> = self.trips.keys().copied().collect();
        for id in ids {
            let Some(m) = self.book.get(id) else {
                continue;
            };
            let names = trip_stops(m);
            let trip = self.trips.get_mut(&id).expect("trip");
            if trip.plan.is_some() || names.is_empty() {
                continue;
            }
            let Some(start) = trip.start else {
                continue;
            };
            let stops: Option<Vec<(String, [f64; 3])>> = names
                .iter()
                .map(|n| self.resolver.get(n).map(|p| (n.clone(), p)))
                .collect();
            let Some(stops) = stops else {
                continue;
            };
            let input = PlanInput {
                mission_id: id,
                reward: trip.reward,
                start,
                stops: &stops,
                return_to: None,
                jump_range,
                own_reference: own,
                setting_reference: setting,
                can_abandon_now: false,
            };
            let Ok(plan) = tourism_plan(&input) else {
                continue;
            };
            let total = trip.base_ly + plan.optimal_ly;
            let accepted_at = trip.accepted_at.to_rfc3339_opts(SecondsFormat::Secs, true);
            if write && (!trip.written || trip.rewrite) {
                let task = if trip.rebuilt && !trip.written {
                    stats_insert_missing_task(cmdr.clone(), id, accepted_at, trip.reward, total)
                } else {
                    stats_write_task(cmdr.clone(), id, accepted_at, trip.reward, total)
                };
                ctx.store_task(TAG_STATS_WRITE, task);
                trip.written = true;
                trip.rewrite = false;
            }
            trip.plan = Some(plan);
        }
    }

    fn end_trip(&mut self, id: u64, completed: bool, state: &GameState, ctx: &mut ModuleCtx) {
        let Some(trip) = self.trips.remove(&id) else {
            return;
        };
        if !completed {
            return;
        }
        let ly = trip.total_ly();
        if let (Some(rows), Some(ly)) = (self.stats_rows.as_mut(), ly)
            && ly > 0.0
        {
            rows.push((trip.reward, ly));
        }
        if trip.written && self.can_write(state, ctx) {
            let cmdr = self.cmdr.clone().unwrap_or_default();
            let at = self.clock().to_rfc3339_opts(SecondsFormat::Secs, true);
            ctx.store_task(TAG_STATS_COMPLETE_WRITE, stats_complete_task(cmdr, id, at));
        }
    }

    // ----- Avisos -----

    fn raise(&mut self, a: MissionAlert, ctx: &mut ModuleCtx) {
        if !self.already.insert(a.key()) {
            return;
        }
        let name = self
            .book
            .get(a.mission_id)
            .map(|m| m.name_localised.clone())
            .unwrap_or_default();
        let alert = match &a.kind {
            AlertKind::ExpiringSoon => {
                Alert::new(Severity::Warn, "missions-alert-expiring").arg("name", name)
            }
            AlertKind::Redirected { system, station } => {
                let dest = match station {
                    Some(s) => format!("{system} · {s}"),
                    None => system.clone(),
                };
                Alert::new(Severity::Info, "missions-alert-redirected").arg("destination", dest)
            }
            AlertKind::Failed => {
                Alert::new(Severity::Warn, "missions-alert-failed").arg("name", name)
            }
            AlertKind::CargoMissing { missing, .. } => {
                Alert::new(Severity::Warn, "missions-alert-cargo-missing").arg("tons", *missing)
            }
        };
        ctx.alert(alert);
        decide!("missions.alert.raised", {"code": a.kind.code()});
        self.alerts.push(a);
        if self.alerts.len() > MAX_ALERTS {
            self.alerts.remove(0);
        }
    }

    fn evaluate(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        let station = (state.location.mode == FlightMode::Docked)
            .then_some(state.location.station.as_deref())
            .flatten();
        let here = Here {
            system: Some(state.location.system.as_str()).filter(|s| !s.is_empty()),
            station,
        };
        let found = evaluate_alerts(
            &self.book,
            &self.inventory,
            self.clock(),
            here,
            &self.already,
        );
        for a in found {
            self.raise(a, ctx);
        }
        // Los avisos de misiones que ya no están activas desaparecen (salvo «fallida»).
        let book = &self.book;
        self.alerts
            .retain(|a| a.kind == AlertKind::Failed || book.get(a.mission_id).is_some());
    }

    // ----- Diario -----

    fn on_journal(
        &mut self,
        kind: &EventKind,
        at: DateTime<Utc>,
        state: &GameState,
        ctx: &mut ModuleCtx,
    ) {
        self.tick(at);
        self.check_cmdr(state);
        match kind {
            EventKind::LoadGame(_) => {
                self.cabins.reset_occupancy();
                self.manifest = None;
                self.inventory.clear();
            }
            EventKind::Missions(_) => {
                self.needs_rebuild = true;
                self.inflight.finish(ACCEPTED_KEY);
            }
            EventKind::Loadout(l) => {
                self.cabins = Cabins::from_loadout(&l.modules);
                if let Some(p) = &self.manifest {
                    self.cabins.apply_manifest(p);
                }
            }
            EventKind::Passengers(p) => {
                self.cabins.apply_manifest(p);
                self.manifest = Some(p.clone());
            }
            EventKind::Cargo(c) => {
                if c.vessel.as_deref().is_none_or(|v| v == "Ship")
                    && let Some(inv) = &c.inventory
                {
                    self.inventory = inv.clone();
                }
            }
            EventKind::CommunityGoal(cg) => self.cgs.apply(cg),
            EventKind::FsdJump(l) | EventKind::Location(l) | EventKind::CarrierJump(l) => {
                self.resolver.learn(&l.star_system, l.star_pos);
            }
            _ => {}
        }
        if self.rebuilding
            && matches!(
                kind,
                EventKind::Bounty(_)
                    | EventKind::MissionRedirected(_)
                    | EventKind::FactionKillBond(_)
            )
            && self.live_buffer.len() < LIVE_BUFFER_CAP
        {
            self.live_buffer.push((kind.clone(), at));
        }
        // La pila se aplica con el libro anterior al evento.
        self.stack.apply_event(kind, at, &self.book);
        let changes = self.book.apply_event(kind, at);
        self.rewards.apply_event(kind, at);
        for c in &changes {
            if let Some(a) = alert_from_change(c, at) {
                self.raise(a, ctx);
            }
            match c {
                BookChange::Added(id) => self.start_trip(*id, at, state),
                BookChange::Completed(id) => self.end_trip(*id, true, state, ctx),
                BookChange::Failed(id) | BookChange::Abandoned(id) => {
                    self.end_trip(*id, false, state, ctx)
                }
                BookChange::Redirected { id, .. } => {
                    self.live_redirects.insert(*id, at);
                    if let Some(t) = self.trips.get_mut(id) {
                        if let Some(p) = t.plan.take() {
                            t.base_ly += p.optimal_ly;
                            t.rewrite = t.written;
                        }
                        t.start = Self::position(state).or(t.start);
                    }
                }
                BookChange::Progress(_) => {}
            }
        }
        // Misiones que la lista activa descartó.
        let book = &self.book;
        self.trips.retain(|id, _| book.get(*id).is_some());
        self.evaluate(state, ctx);
    }

    // ----- Render -----

    fn stamp(&self, mut v: Value) -> Value {
        if let Some(o) = v.as_object_mut() {
            o.insert("as_of".into(), json!(self.now));
        }
        v
    }

    fn render_active(&self, state: &GameState) -> Value {
        struct Group<'a> {
            system: Option<String>,
            station: Option<String>,
            distance: Option<f64>,
            expiry: Option<DateTime<Utc>>,
            missions: Vec<&'a Mission>,
        }
        let pos = Self::position(state);
        let here_key = coord_key(&state.location.system);
        let mut groups: BTreeMap<(String, String), Group> = BTreeMap::new();
        for m in self.book.active() {
            let system = destination(m);
            let station = m
                .destination_station
                .clone()
                .filter(|s| !s.trim().is_empty());
            let key = (
                system.as_deref().map(coord_key).unwrap_or_default(),
                station.as_deref().map(coord_key).unwrap_or_default(),
            );
            let distance = match (&system, pos) {
                (Some(s), Some(p)) if coord_key(s) == here_key => finite(&p).then_some(0.0),
                (Some(s), Some(p)) => self.resolver.get(s).map(|d| dist(p, d)),
                _ => None,
            };
            let g = groups.entry(key).or_insert_with(|| Group {
                system: system.clone(),
                station: station.clone(),
                distance,
                expiry: None,
                missions: Vec::new(),
            });
            if let Some(e) = m.expiry {
                g.expiry = Some(g.expiry.map_or(e, |x| x.min(e)));
            }
            g.missions.push(m);
        }
        let mut groups: Vec<Group> = groups.into_values().collect();
        let exp_key = |g: &Group| g.expiry.map_or(i64::MAX, |e| e.timestamp());
        groups.sort_by(|a, b| {
            let by_dist = match (a.distance, b.distance) {
                (Some(x), Some(y)) if (x - y).abs() > 1e-6 => x.total_cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            };
            by_dist
                .then(exp_key(a).cmp(&exp_key(b)))
                .then(a.system.cmp(&b.system))
                .then(a.station.cmp(&b.station))
        });
        let next = groups.iter().position(|g| g.system.is_some());
        let next_system = next.and_then(|i| groups[i].system.clone());
        let out: Vec<Value> = groups
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let mut ms = g.missions.clone();
                ms.sort_by_key(|m| (m.expiry.map_or(i64::MAX, |e| e.timestamp()), m.id));
                json!({
                    "system": g.system,
                    "station": g.station,
                    "distance_ly": g.distance,
                    "expiry": g.expiry,
                    "next": Some(i) == next,
                    "missions": ms,
                })
            })
            .collect();
        self.stamp(json!({
            "count": self.book.active().count(),
            "groups": out,
            "next_system": next_system,
            "copy_target": self.copy_target,
            "copy_seq": self.copy_seq,
        }))
    }

    fn render_tourism(&self, state: &GameState) -> Value {
        let online = self.online(state);
        let docked = state.location.mode == FlightMode::Docked;
        let mut out = Vec::new();
        for (id, t) in &self.trips {
            let Some(m) = self.book.get(*id) else {
                continue;
            };
            if m.kind != MissionKind::Sightseeing && m.stops.len() < 2 {
                continue;
            }
            let stops = trip_stops(m);
            let (status, plan) = if stops.is_empty() {
                ("stops_unavailable", None)
            } else if let Some(p) = &t.plan {
                let mut p = p.clone();
                p.can_abandon_now = docked
                    && t.start_market.is_some()
                    && state.location.market_id == t.start_market;
                ("ok", Some(p))
            } else if !online
                || t.start.is_none()
                || stops.iter().any(|s| self.resolver.is_unresolvable(s))
            {
                ("no_coords", None)
            } else {
                ("loading", None)
            };
            out.push(json!({
                "mission_id": id,
                "name": m.name_localised,
                "stops": stops,
                "status": status,
                "plan": plan,
            }));
        }
        self.stamp(json!({"trips": out}))
    }

    fn render_passengers(&self) -> Value {
        let (groups, wanted) = match &self.manifest {
            Some(p) => (groups(&self.book, p), wanted_aboard(p)),
            None => (Vec::new(), false),
        };
        self.stamp(json!({"cabins": self.cabins, "groups": groups, "wanted": wanted}))
    }

    fn render_alerts(&self) -> Value {
        let items: Vec<Value> = self
            .alerts
            .iter()
            .map(|a| {
                let mut v = serde_json::to_value(a).unwrap_or_default();
                if let Some(o) = v.as_object_mut() {
                    let m = self.book.get(a.mission_id);
                    o.insert("name".into(), json!(m.map(|m| &m.name_localised)));
                    o.insert("mission_kind".into(), json!(m.map(|m| m.kind)));
                }
                v
            })
            .collect();
        self.stamp(json!({"alerts": items}))
    }

    fn render_all(&mut self, state: &GameState) -> TopicSet {
        let mut dirty = TopicSet::new();
        let v = self.render_active(state);
        self.set(TOPIC_ACTIVE, v, &mut dirty);
        let v = self.stamp(json!({"piles": self.stack.view(&self.book)}));
        self.set(TOPIC_MASSACRE, v, &mut dirty);
        let v = self.render_alerts();
        self.set(TOPIC_ALERTS, v, &mut dirty);
        let v = self.stamp(serde_json::to_value(&self.rewards).unwrap_or_default());
        self.set(TOPIC_REWARDS, v, &mut dirty);
        let v = self.render_passengers();
        self.set(TOPIC_PASSENGERS, v, &mut dirty);
        let v = self.render_tourism(state);
        self.set(TOPIC_TOURISM, v, &mut dirty);
        let v = self.stamp(json!({"goals": self.cgs.goals}));
        self.set(TOPIC_CG, v, &mut dirty);
        dirty
    }

    fn set(&mut self, topic: &'static str, v: Value, dirty: &mut TopicSet) {
        crate::topicx::set_topic(&mut self.values, topic, v, dirty);
    }

    fn on_action(&mut self, name: &str, args: &Value, ctx: &ModuleCtx) -> bool {
        match name {
            ACTION_TO_ROUTE => {
                let Some(system) = args
                    .get("system")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                else {
                    decide!("missions.to_route.rejected", {});
                    return false;
                };
                self.copy_target = Some(system.to_owned());
                if !ctx.replaying {
                    self.copy_seq += 1;
                }
                decide!("missions.to_route.applied", {"copy_seq": self.copy_seq});
                true
            }
            ACTION_DISMISS => {
                let Some(id) = args.get("mission_id").and_then(Value::as_u64) else {
                    return false;
                };
                let before = self.alerts.len();
                let _ = before; // solo lo lee el diagnóstico
                self.alerts.retain(|a| a.mission_id != id);
                decide!("missions.alert.dismissed", {"removed": before - self.alerts.len()});
                true
            }
            ACTION_RESET => {
                let now = self.clock();
                self.rewards.reset(now);
                decide!("missions.session.reset.applied", {});
                true
            }
            _ => false,
        }
    }
}

impl Default for Missions {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Missions {
    fn id(&self) -> &'static str {
        "missions"
    }

    fn on_event(&mut self, ev: &SourceEvent, state: &GameState, ctx: &mut ModuleCtx) -> TopicSet {
        match ev {
            SourceEvent::Settings(s) => {
                let enabling = s.online_services && !self.settings.online_services;
                let reference = s.tourism_reference_cr_ly != self.settings.tourism_reference_cr_ly;
                self.settings = s.clone();
                if reference {
                    for t in self.trips.values_mut() {
                        t.plan = None;
                    }
                }
                if !(enabling || reference) {
                    return TopicSet::new();
                }
            }
            SourceEvent::Journal { ev: je, .. } => {
                self.on_journal(&je.kind, je.timestamp, state, ctx);
                self.ensure_rebuild(ctx);
            }
            SourceEvent::Companion(CompanionFile::Cargo(f)) => {
                if f.vessel.as_deref().is_none_or(|v| v == "Ship") {
                    self.inventory = journal_cargo(&f.inventory);
                }
                self.evaluate(state, ctx);
            }
            SourceEvent::Stored { module, tag, value } if module == "missions" => {
                if ctx.replaying {
                    return TopicSet::new();
                }
                let applied = match tag.as_str() {
                    TAG_ACCEPTED => self.on_accepted_stored(value, state, ctx),
                    TAG_BOUNTIES => self.on_bounties_stored(value),
                    TAG_PASSENGER_STATS => self.on_stats_stored(value),
                    TAG_COORDS => self.on_coords_stored(value),
                    _ => false,
                };
                if !applied {
                    return TopicSet::new();
                }
            }
            SourceEvent::Net(r) if r.module == "missions" => {
                if !self.on_net(r) {
                    return TopicSet::new();
                }
            }
            SourceEvent::Action { name, args } => {
                if !self.on_action(name, args, ctx) {
                    return TopicSet::new();
                }
            }
            _ => return TopicSet::new(),
        }
        self.request_coords(state, ctx);
        self.compute_trips(state, ctx);
        self.render_all(state)
    }

    fn topic_value(&self, topic: &str) -> Option<Value> {
        self.values.get(topic).cloned()
    }

    fn topics(&self) -> Vec<String> {
        TOPICS.iter().map(|t| (*t).to_owned()).collect()
    }

    fn relevance(&self, state: &GameState) -> Relevance {
        let mode = state.location.mode;
        if self.book.active().next().is_some()
            && matches!(mode, FlightMode::Docked | FlightMode::Supercruise)
        {
            Relevance::Active
        } else {
            Relevance::Normal
        }
    }

    fn migrations(&self) -> &'static [Migration] {
        Missions::migrations()
    }
}

#[cfg(test)]
mod tests;
