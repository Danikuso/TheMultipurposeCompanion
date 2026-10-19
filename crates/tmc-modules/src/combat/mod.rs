//! Módulo `combat`: objetivo, mi nave, sesión de caza con vales, zona de conflicto y AX.
//!
//! Topics (siempre presentes tras el primer evento; `null` cuando no aplica):
//! `combat.target`, `combat.ship`, `combat.session`, `combat.conflict_zone`, `combat.ax`.
//!
//! Tiempo: las caducidades (objetivo perdido 10 s, `HeatDamage` 10 s, `UnderAttack`
//! 30 s, objetivo AX 60 s) solo se ven en `view(now)`. El módulo vuelve a renderizar en
//! cada evento y en cada `Status` (≈1 Hz si algo cambia) y solo marca sucio el topic
//! cuyo valor cambió. El temporizador de sobrecalentamiento lo cuenta la UI en local a
//! partir de `combat.ship.overheating_since`, sin `requestAnimationFrame` continuo.
//! Sin reloj propio: «ahora» es el último instante visto (diario o `Status`).
//!
//! Vales: el libro se reconstruye desde el almacén una vez por comandante, en la
//! primera ocasión en vivo (en replay no hay `StoreTask`; si no llega el resultado en
//! 60 s se repite). Los eventos de vales que llegan antes del resultado se guardan con su
//! posición `(ts, archivo, línea)` y, al llegar, se reaplican solo los estrictamente
//! posteriores a `cut_pos` (última posición cronológica cubierta por el corte, mismo
//! orden que `history`), sin perder ni duplicar eventos del mismo segundo.
//!
//! Render: Status solo recalcula `combat.ship` (y `target`/`ax` si hay una caducidad en
//! curso); sesión, zona y AX se recalculan con eventos, respuestas o acciones.

pub mod ax;
pub mod cz;
pub mod session;
pub mod ship;
pub mod target;
pub mod vouchers;

use crate::data::ship_pads;
use crate::inflight::Inflight;
use ax::AxTracker;
use chrono::{DateTime, SecondsFormat, Utc};
use cz::CzTracker;
use serde::Serialize;
use serde_json::Value;
use session::HuntingSession;
use ship::{ShipAlert, ShipTracker};
use std::collections::{HashMap, VecDeque};
use target::TargetTracker;
use tmc_core::{Alert, Module, ModuleCtx, Relevance, Severity, TopicSet};
use tmc_model::events::{EventKind, JournalEvent};
use tmc_model::net::{NearbyService, NetOutcome, NetQuery, NetResponse, PadSize, RemoteStation};
use tmc_model::settings::ModuleSettings;
use tmc_model::source::SourceEvent;
use tmc_model::state::{Galaxy, GameState};
use vouchers::{VoucherKind, VoucherLedger};

const TOPIC_TARGET: &str = "combat.target";
const TOPIC_SHIP: &str = "combat.ship";
const TOPIC_SESSION: &str = "combat.session";
const TOPIC_ZONE: &str = "combat.conflict_zone";
const TOPIC_AX: &str = "combat.ax";
const TOPICS: [&str; 5] = [
    TOPIC_TARGET,
    TOPIC_SHIP,
    TOPIC_SESSION,
    TOPIC_ZONE,
    TOPIC_AX,
];

const TAG_VOUCHERS: &str = "combat.vouchers";
const LEDGER_KEY: &str = "ledger";
const ACTION_RESET: &str = "combat.reset_session";
/// Filas de vales visibles; el resto se agrega en una fila «otras».
const MAX_VOUCHER_ROWS: usize = 32;
const MAX_ROW_SYSTEMS: usize = 8;
/// Comisión de Interstellar Factors (porcentaje).
pub const IF_FEE_PCT: u8 = 25;
/// Un `Failed` de la búsqueda de Interstellar Factors se reintenta pasado este tiempo.
const FAILED_COOLDOWN_S: i64 = 300;
/// Tope del búfer de eventos de vales pendientes de reconciliar con el almacén.
const PENDING_CAP: usize = 4096;

/// Posición cronológica de un evento: `(ts normalizado, archivo, línea)`.
type Pos = (String, String, u64);

struct Pending {
    kind: EventKind,
    system: String,
    ts: DateTime<Utc>,
    pos: Pos,
}

#[derive(Debug, Clone)]
enum IfState {
    Found(RemoteStation),
    NoData,
    Failed(DateTime<Utc>),
}

/// Fila del topic de sesión. `other = true` agrega las facciones que no caben.
#[derive(Debug, Serialize)]
struct VoucherRow {
    kind: Option<VoucherKind>,
    faction: String,
    amount: u64,
    systems: Vec<String>,
    ax: bool,
    other: bool,
    count: u32,
}

#[derive(Debug, Serialize)]
struct VoucherTotals {
    bounty: u64,
    combat_bond: u64,
}

#[derive(Serialize)]
struct SessionTopic<'a> {
    #[serde(flatten)]
    session: &'a HuntingSession,
    vouchers: Vec<VoucherRow>,
    vouchers_total: VoucherTotals,
    nearest_if: Option<&'a RemoteStation>,
    if_fee_pct: u8,
}

pub struct Combat {
    settings: ModuleSettings,
    now: Option<DateTime<Utc>>,
    cmdr: Option<String>,
    system: String,
    target: TargetTracker,
    ship: ShipTracker,
    session: HuntingSession,
    ledger: VoucherLedger,
    cz: CzTracker,
    ax: AxTracker,
    /// El libro ya viene del almacén.
    ledger_loaded: bool,
    /// Eventos de vales vistos antes de recibir el libro del almacén.
    pending: VecDeque<Pending>,
    inflight: Inflight,
    if_state: Option<(u64, IfState)>,
    prev_alerts: Vec<ShipAlert>,
    /// Ya se vio un `Status`. Decisión: el primer `Status` fija la línea base de avisos
    /// sin avisar, porque un `Status` suelto sin `SHIELDS_UP` (p. ej. al arrancar con la
    /// nave sin escudos) no es una caída de escudos real; solo avisan las transiciones.
    status_seen: bool,
    #[cfg(test)]
    session_renders: u32,
    #[cfg(test)]
    emitted: Vec<&'static str>,
    values: HashMap<&'static str, Value>,
}

impl Combat {
    pub fn new() -> Self {
        Self {
            settings: ModuleSettings::default(),
            now: None,
            cmdr: None,
            system: String::new(),
            target: TargetTracker::default(),
            ship: ShipTracker::default(),
            session: HuntingSession::new(DateTime::UNIX_EPOCH),
            ledger: VoucherLedger::default(),
            cz: CzTracker::default(),
            ax: AxTracker::default(),
            ledger_loaded: false,
            pending: VecDeque::new(),
            inflight: Inflight::default(),
            if_state: None,
            prev_alerts: Vec::new(),
            status_seen: false,
            #[cfg(test)]
            session_renders: 0,
            #[cfg(test)]
            emitted: Vec::new(),
            values: HashMap::new(),
        }
    }

    fn tick(&mut self, at: DateTime<Utc>) {
        if self.now.is_none() {
            self.session.started_at = at;
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

    /// Un cambio de comandante descarta el libro y la búsqueda de cobro.
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
        if switched {
            // Otro comandante: sesión, rango, objetivo, avisos, CZ y AX no sirven.
            self.session = HuntingSession::new(self.clock());
            self.target = TargetTracker::default();
            self.prev_alerts.clear();
            self.cz = CzTracker::default();
            self.ax = AxTracker::default();
        }
        self.ledger = VoucherLedger::default();
        self.ledger_loaded = false;
        self.inflight.finish(LEDGER_KEY);
        self.pending.clear();
        self.if_state = None;
    }

    /// Pide la reconstrucción del libro (solo en vivo). Una vez por comandante; si el
    /// resultado no llega en `INFLIGHT_TTL_S` se repite.
    fn ensure_ledger_task(&mut self, ctx: &mut ModuleCtx) {
        if ctx.replaying || self.ledger_loaded {
            return;
        }
        let Some(cmdr) = self.cmdr.clone() else {
            return;
        };
        if !self.inflight.begin(LEDGER_KEY, self.clock()) {
            return;
        }
        ctx.store_task(TAG_VOUCHERS, session::rebuild_task(cmdr));
        decide!(
            "combat.ledger.rebuild.requested",
            tmc_model::diagnostics::json!({})
        );
    }

    fn on_journal(&mut self, je: &JournalEvent, file: &str, line: u64, state: &GameState) {
        let now = je.timestamp;
        self.check_cmdr(state);
        self.system = state.location.system.clone();
        if let EventKind::Location(l) | EventKind::FsdJump(l) | EventKind::CarrierJump(l) = &je.kind
        {
            self.ax.on_location(l);
        }
        match &je.kind {
            EventKind::ShipTargeted(t) => {
                self.target.apply(t, now);
            }
            EventKind::Died => self.target = TargetTracker::default(),
            _ => {}
        }
        self.ax.apply_event(&je.kind, now);
        self.cz.apply_event(&je.kind, &self.system, now);
        self.session.apply_event(&je.kind, now);
        self.ship.on_event(&je.kind, now);
        let relevant = matches!(
            je.kind,
            EventKind::Bounty(_)
                | EventKind::FactionKillBond(_)
                | EventKind::CapShipBond(_)
                | EventKind::RedeemVoucher(_)
                | EventKind::Died
        );
        if relevant {
            self.ledger.apply_event(&je.kind, &self.system, now);
            if !self.ledger_loaded {
                if self.pending.len() >= PENDING_CAP {
                    self.pending.pop_front();
                }
                self.pending.push_back(Pending {
                    kind: je.kind.clone(),
                    system: self.system.clone(),
                    ts: now,
                    pos: (
                        now.to_rfc3339_opts(SecondsFormat::Nanos, true),
                        file.to_owned(),
                        line,
                    ),
                });
            }
        }
    }

    fn on_stored(&mut self, value: &Value) {
        if self.cmdr.is_none()
            || value.get("cmdr").and_then(Value::as_str) != self.cmdr.as_deref()
            || self.ledger_loaded
        {
            return;
        }
        let Ok(ledger) = serde_json::from_value::<VoucherLedger>(value["ledger"].clone()) else {
            tracing::warn!("libro de vales del almacén ilegible");
            return;
        };
        let cut_pos: Option<Pos> = value.get("cut_pos").and_then(|p| {
            Some((
                p.get("ts")?.as_str()?.to_owned(),
                p.get("file")?.as_str()?.to_owned(),
                p.get("line")?.as_u64()?,
            ))
        });
        self.ledger = ledger;
        for p in std::mem::take(&mut self.pending) {
            if cut_pos.as_ref().is_none_or(|c| p.pos > *c) {
                self.ledger.apply_event(&p.kind, &p.system, p.ts);
            }
        }
        self.inflight.finish(LEDGER_KEY);
        self.ledger_loaded = true;
    }

    fn unredeemed_total(&self) -> u64 {
        self.ledger
            .total(VoucherKind::Bounty)
            .saturating_add(self.ledger.total(VoucherKind::CombatBond))
    }

    /// Busca el Interstellar Factors más cercano al sistema actual si hay vales.
    fn maybe_request_if(&mut self, state: &GameState, ctx: &mut ModuleCtx) {
        let addr = state.location.system_address;
        if ctx.replaying || addr == 0 || self.unredeemed_total() == 0 {
            return;
        }
        if !self.online(state) {
            decide!(
                "combat.if.request.skipped_offline",
                tmc_model::diagnostics::json!({"unredeemed": self.unredeemed_total()})
            );
            return;
        }
        match &self.if_state {
            Some((a, IfState::Found(_) | IfState::NoData)) if *a == addr => return,
            Some((a, IfState::Failed(at)))
                if *a == addr && (self.clock() - *at).num_seconds() < FAILED_COOLDOWN_S =>
            {
                return;
            }
            _ => {}
        }
        let key = format!("if:{addr}");
        if !self.inflight.begin(&key, self.clock()) {
            return;
        }
        // Nave desconocida: plataforma Large, para no ofrecer una estación que la
        // nave no pueda usar (mismo criterio conservador que `ship_pads`).
        let min_pad = state
            .ship
            .as_ref()
            .and_then(|s| ship_pads().pad_for(&s.ship_type))
            .unwrap_or(PadSize::Large);
        let query = NetQuery::NearestService {
            system_address: addr,
            service: NearbyService::InterstellarFactors,
            min_pad,
        };
        let accepted = ctx.request(key.clone(), query).is_some();
        if !accepted {
            self.inflight.finish(&key);
        }
        decide!(
            "combat.if.request.sent",
            tmc_model::diagnostics::json!({"accepted": accepted})
        );
    }

    fn apply_response(&mut self, r: &NetResponse, state: &GameState) {
        let Some(addr) = r
            .key
            .strip_prefix("if:")
            .and_then(|a| a.parse::<u64>().ok())
            .filter(|_| r.module == "combat")
        else {
            return;
        };
        self.inflight.finish(&r.key);
        if addr != state.location.system_address {
            return;
        }
        let st = match &r.outcome {
            NetOutcome::Stations(v) => v
                .iter()
                .min_by(|a, b| a.distance_ly.total_cmp(&b.distance_ly))
                .cloned()
                .map_or(IfState::NoData, IfState::Found),
            NetOutcome::NoData => IfState::NoData,
            NetOutcome::Failed(_) => IfState::Failed(self.clock()),
            _ => return,
        };
        self.if_state = Some((addr, st));
    }

    fn render_session(&self, state: &GameState) -> Value {
        let mut rows = self.ledger.unredeemed();
        rows.sort_by_key(|t| std::cmp::Reverse(t.amount));
        let rest = rows.split_off(rows.len().min(MAX_VOUCHER_ROWS));
        let mut vouchers: Vec<VoucherRow> = rows
            .into_iter()
            .map(|t| VoucherRow {
                kind: Some(t.kind),
                faction: t.faction,
                amount: t.amount,
                systems: t.systems.into_iter().take(MAX_ROW_SYSTEMS).collect(),
                ax: t.ax,
                other: false,
                count: 1,
            })
            .collect();
        if !rest.is_empty() {
            vouchers.push(VoucherRow {
                kind: None,
                faction: String::new(),
                amount: rest.iter().fold(0u64, |a, t| a.saturating_add(t.amount)),
                systems: Vec::new(),
                ax: false,
                other: true,
                count: u32::try_from(rest.len()).unwrap_or(u32::MAX),
            });
        }
        let nearest_if = match &self.if_state {
            Some((a, IfState::Found(s))) if *a == state.location.system_address => Some(s),
            _ => None,
        };
        let topic = SessionTopic {
            session: &self.session,
            vouchers,
            vouchers_total: VoucherTotals {
                bounty: self.ledger.total(VoucherKind::Bounty),
                combat_bond: self.ledger.total(VoucherKind::CombatBond),
            },
            nearest_if,
            if_fee_pct: IF_FEE_PCT,
        };
        serde_json::to_value(topic).unwrap_or_default()
    }

    fn render_ax(&self, now: DateTime<Utc>) -> Value {
        serde_json::json!({
            "active": self.ax.active(now),
            "presence": self.ax.presence,
            "kills_by_type": self.ax.kills_by_type,
            "bonds": self.ax.bonds,
            "target": self.ax.pending_target(now),
        })
    }

    /// Avisos nuevos (transiciones). Con `baseline` (primer `Status`) solo se fija la
    /// línea base. Se actualiza también en replay: al pasar a vivo no se repiten avisos.
    fn emit_alerts(&mut self, ctx: &mut ModuleCtx, view: &ship::ShipView, baseline: bool) {
        for a in &view.alerts {
            if baseline || self.prev_alerts.contains(a) || !self.settings.alerts_enabled {
                continue;
            }
            let (sev, key) = match a {
                ShipAlert::HullLow => (Severity::Critical, "combat-alert-hull-low"),
                ShipAlert::ShieldsDown => (Severity::Warn, "combat-alert-shields-down"),
                ShipAlert::Overheating => (Severity::Warn, "combat-alert-overheating"),
                ShipAlert::UnderAttack => (Severity::Warn, "combat-alert-under-attack"),
                ShipAlert::Interdiction => (Severity::Critical, "combat-alert-interdiction"),
            };
            let mut alert = Alert::new(sev, key);
            if *a == ShipAlert::HullLow {
                alert = alert.arg("pct", view.hull_pct.map_or(0, u32::from));
            }
            #[cfg(test)]
            if !ctx.replaying {
                self.emitted.push(key);
            }
            decide!(
                "combat.alert.emitted",
                tmc_model::diagnostics::json!({"replaying": ctx.replaying})
            );
            ctx.alert(alert);
        }
        self.prev_alerts = view.alerts.clone();
    }

    /// Valor cacheado de un campo de un topic (para decidir si una caducidad corre).
    fn cached(&self, topic: &str, field: &str) -> Option<&Value> {
        self.values.get(topic)?.get(field)
    }

    fn set(&mut self, topic: &'static str, v: Value, dirty: &mut TopicSet) {
        // `as_of` avanza con cada evento: no cuenta para detectar cambios.
        if strip_as_of(self.values.get(topic)) != strip_as_of(Some(&v)) {
            self.values.insert(topic, v);
            dirty.insert(topic.to_owned());
        }
    }
}

fn strip_as_of(v: Option<&Value>) -> Option<Value> {
    v.map(|v| {
        let mut v = v.clone();
        if let Some(o) = v.as_object_mut() {
            o.remove("as_of");
        }
        v
    })
}

impl Default for Combat {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Combat {
    fn id(&self) -> &'static str {
        "combat"
    }

    fn on_event(&mut self, ev: &SourceEvent, state: &GameState, ctx: &mut ModuleCtx) -> TopicSet {
        // La búsqueda de cobro se lanza al llegar a un sistema, al cargar el libro, al
        // pasar el total de vales de 0 a >0 y al activar `online_services`.
        let mut search_if = false;
        let mut baseline = false;
        // `false`: solo `Status` (ship y caducidades); `true`: todo.
        let mut full = true;
        match ev {
            SourceEvent::Settings(s) => {
                search_if = s.online_services && !self.settings.online_services;
                self.settings = s.clone();
                if !search_if {
                    return TopicSet::new();
                }
            }
            SourceEvent::Status(st) => {
                self.tick(st.timestamp);
                baseline = !self.status_seen;
                self.status_seen = true;
                self.ship.on_status(st, st.timestamp);
                full = self.values.is_empty();
            }
            SourceEvent::Journal { ev, file, line, .. } => {
                self.tick(ev.timestamp);
                let before = self.unredeemed_total();
                self.on_journal(ev, file, *line, state);
                self.ensure_ledger_task(ctx);
                search_if = matches!(
                    ev.kind,
                    EventKind::Location(_) | EventKind::FsdJump(_) | EventKind::CarrierJump(_)
                ) || (before == 0 && self.unredeemed_total() > 0);
            }
            SourceEvent::Stored { module, tag, value } if module == "combat" => {
                if tag != TAG_VOUCHERS {
                    return TopicSet::new();
                }
                self.on_stored(value);
                search_if = true;
            }
            SourceEvent::Net(r) if r.module == "combat" => self.apply_response(r, state),
            SourceEvent::Action { name, .. } if name == ACTION_RESET => {
                let now = self.clock();
                self.session.reset(now);
                decide!(
                    "combat.session.reset.applied",
                    tmc_model::diagnostics::json!({})
                );
            }
            _ => return TopicSet::new(),
        }
        if search_if {
            self.maybe_request_if(state, ctx);
        }
        let now = self.clock();
        let ship_view = self.ship.view(now);
        self.emit_alerts(ctx, &ship_view, baseline);

        let mut dirty = TopicSet::new();
        let ship = serde_json::to_value(&ship_view).unwrap_or_default();
        self.set(TOPIC_SHIP, ship, &mut dirty);
        // Un objetivo perdido caduca a los 10 s: solo entonces hay que mirar el reloj.
        let target_expiring = self
            .cached(TOPIC_TARGET, "lost_at")
            .is_some_and(|v| !v.is_null());
        if full || target_expiring {
            let target = self
                .target
                .view(now)
                .and_then(|t| serde_json::to_value(t).ok())
                .unwrap_or(Value::Null);
            self.set(TOPIC_TARGET, target, &mut dirty);
        }
        // AX depende del reloj solo mientras está activo o con objetivo pendiente (60 s).
        let ax_timed = self
            .cached(TOPIC_AX, "active")
            .is_some_and(|v| v.as_bool() == Some(true))
            || self
                .cached(TOPIC_AX, "target")
                .is_some_and(|v| !v.is_null());
        if full || ax_timed {
            let ax = self.render_ax(now);
            self.set(TOPIC_AX, ax, &mut dirty);
        }
        if full {
            #[cfg(test)]
            {
                self.session_renders += 1;
            }
            let mut session = self.render_session(state);
            if let (Some(o), Some(at)) = (session.as_object_mut(), self.now) {
                o.insert("as_of".into(), serde_json::json!(at));
            }
            self.set(TOPIC_SESSION, session, &mut dirty);
            let zone = self
                .cz
                .view()
                .and_then(|z| serde_json::to_value(z).ok())
                .unwrap_or(Value::Null);
            self.set(TOPIC_ZONE, zone, &mut dirty);
        }
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
        let locked = self.target.view(now).is_some_and(|t| t.lost_at.is_none());
        let ship = self.ship.view(now);
        let under_attack = ship.alerts.contains(&ShipAlert::UnderAttack);
        let in_zone = ship.hardpoints && self.cz.view().is_some();
        if locked || under_attack || in_zone || self.ax.active(now) {
            Relevance::Active
        } else {
            Relevance::Normal
        }
    }
}

#[cfg(test)]
mod tests;
