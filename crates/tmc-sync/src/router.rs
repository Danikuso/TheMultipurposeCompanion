//! Router de sincronización: decide qué líneas del diario se encolan para cada servicio.
//!
//! Por cada línea: tracker → registro del comandante → guardas globales → origen → por servicio
//! (activo y `timestamp ≥ activated_at`) → `map`. El cursor del comandante dueño de la línea avanza
//! siempre que la línea se evalúa (con filas o sin ellas) y en la misma transacción que las filas.
//! Ante cualquier duda la línea no se encola (falla cerrado).
//!
//! Modo recuperable (§5.4): las líneas `CatchUp` del archivo vigente al arrancar y del anterior de la
//! misma sesión (leído por [`Recovery`] al construir el router) se evalúan con las guardas normales,
//! salvo la del juego en marcha, y sus filas se retienen hasta `CatchUpDone`; si llega una línea en
//! vivo antes, la recuperación se abandona sin confirmar nada (ruling E0-6).

use crate::clock::Clock;
use crate::ids::idem_key;
use crate::recovery::{Recovery, RecoveryPlan, same_session};
use crate::tracker::{TransientSnapshot, TransientTracker};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::time::Duration;
use tmc_model::companion::CompanionFile;
use tmc_model::events::{EventKind, JournalEvent};
use tmc_model::state::Galaxy;
use tmc_model::sync::{
    ErrorClass, FeedOrigin, KeyAssignment, OutboxState, RowOrigin, SyncFeed, SyncFeedEvent,
    SyncReason, SyncService,
};
use tmc_store::{CursorAdvance, EnqueueOutcome, NewOutboxRow, Store, StoreError, UnassignedReason};
use tokio::sync::Notify;
use tokio::time::Instant;

pub use tmc_model::sync::REPLAY_MARKER;

/// Clave `kv` del interruptor EDDN: instante de encendido (RFC 3339); ausente = apagado (PD-3).
pub const EDDN_ENABLED_KEY: &str = "sync.eddn.enabled_at";

/// Guardas del entorno de ejecución. El marcador `.tmc-replay` lo comprueba el router (PD-9).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EnvGates {
    /// `TMC_E2E_GAME_RUNNING` definida (en cualquier compilación).
    pub e2e_simulation: bool,
}

/// Cómo se sabe que el juego escribe el diario en vivo, por plataforma (ruling T0-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveSignalPolicy {
    /// Windows: proceso del juego (`GameProcess`).
    ProcessCheck,
    /// Linux: líneas `Live` nuevas + ausencia de `.tmc-replay` + resto de guardas.
    JournalActivity,
    /// Plataforma sin señal fiable: no se sincroniza nada.
    Unavailable,
}

#[derive(Debug, Clone)]
pub struct RouterConfig {
    pub journal_dir: PathBuf,
    pub env: EnvGates,
    pub live_signal: LiveSignalPolicy,
    /// Vaciado del lote por tiempo (250 ms).
    pub flush_every: Duration,
    /// Vaciado del lote por tamaño (200 filas o líneas).
    pub flush_rows: usize,
    /// Retención de líneas `Live` antes de la primera muestra de proceso: (200 líneas, 10 s) (PD-2).
    pub hold_before_process: (usize, Duration),
}

/// Entrada de un mapeador de servicio para una línea permitida.
pub struct MappedInput<'a> {
    pub ev: &'a JournalEvent,
    pub file: &'a str,
    pub line: u64,
    pub snap: &'a TransientSnapshot,
    /// Compañero emparejado (E1-3); en E0 siempre `None`.
    pub companion: Option<&'a CompanionFile>,
    pub tracker: &'a TransientTracker,
}

/// Fila producida por un mapeador; el router añade servicio, comandante, clave y origen.
#[derive(Debug, Clone)]
pub struct MappedRow {
    pub kind: String,
    pub sub: u32,
    pub payload: serde_json::Value,
    pub coalesce_key: Option<String>,
    pub state: OutboxState,
    pub class: Option<ErrorClass>,
}

/// Traduce eventos del diario a filas de un servicio.
pub trait ServiceMapper: Send {
    fn service(&self) -> SyncService;
    fn wants_companion(&self, _ev: &JournalEvent) -> bool {
        false
    }
    fn map(&mut self, input: &MappedInput<'_>, out: &mut Vec<MappedRow>);
}

const R_NONE: u8 = 0;
const R_REPLAY: u8 = 1;
const R_PLATFORM: u8 = 2;
const R_LEGACY: u8 = 3;
const R_BETA: u8 = 4;

fn decode_reason(code: u8) -> Option<SyncReason> {
    match code {
        R_REPLAY => Some(SyncReason::Replay),
        R_PLATFORM => Some(SyncReason::PlatformUnsupported),
        R_LEGACY => Some(SyncReason::Legacy),
        R_BETA => Some(SyncReason::Beta),
        _ => None,
    }
}

/// Estado de guardas compartido: lo produce el router y lo leen trabajadores y agregador (PF-4).
#[derive(Debug)]
pub struct SendGates {
    pub online: Arc<AtomicBool>,
    pub lease_held: Arc<AtomicBool>,
    /// Galaxia actual del actor ≠ Legacy (`Unknown` permitido, PD-7).
    pub galaxy_not_legacy: Arc<AtomicBool>,
    /// Replay, simulación e2e o plataforma sin soporte.
    env_reason: AtomicU8,
    /// Galaxia o versión del último archivo evaluado (Legacy/beta).
    content_reason: AtomicU8,
    /// Política de proceso sin juego en marcha (o sin muestra aún).
    game_not_running: AtomicBool,
}

impl SendGates {
    pub fn new(online: Arc<AtomicBool>, lease_held: Arc<AtomicBool>) -> Self {
        Self {
            online,
            lease_held,
            galaxy_not_legacy: Arc::new(AtomicBool::new(true)),
            env_reason: AtomicU8::new(R_NONE),
            content_reason: AtomicU8::new(R_NONE),
            game_not_running: AtomicBool::new(false),
        }
    }

    /// Se puede enviar lo encolado: en línea, con lease, fuera de Legacy y sin replay/e2e/plataforma.
    pub fn send_allowed(&self) -> bool {
        self.online.load(Ordering::SeqCst)
            && self.lease_held.load(Ordering::SeqCst)
            && self.galaxy_not_legacy.load(Ordering::SeqCst)
            && self.env_reason.load(Ordering::SeqCst) == R_NONE
    }

    /// Motivo global que impide encolar, en el orden de evaluación de las guardas.
    pub fn global_reason(&self) -> Option<SyncReason> {
        if !self.lease_held.load(Ordering::SeqCst) {
            return Some(SyncReason::OtherInstance);
        }
        if let Some(reason) = decode_reason(self.env_reason.load(Ordering::SeqCst)) {
            return Some(reason);
        }
        if !self.online.load(Ordering::SeqCst) {
            return Some(SyncReason::OnlineOff);
        }
        if !self.galaxy_not_legacy.load(Ordering::SeqCst) {
            return Some(SyncReason::Legacy);
        }
        decode_reason(self.content_reason.load(Ordering::SeqCst))
    }

    pub fn game_not_running(&self) -> bool {
        self.game_not_running.load(Ordering::SeqCst)
    }

    /// Motivo actual de bloqueo (global o juego parado) para el agregador.
    pub fn blocked_reason(&self) -> Option<SyncReason> {
        self.global_reason().or_else(|| {
            self.game_not_running()
                .then_some(SyncReason::GameNotRunning)
        })
    }
}

#[derive(Default)]
struct Batch {
    rows: Vec<NewOutboxRow>,
    cursors: Vec<CursorAdvance>,
    lines: usize,
    since: Option<Instant>,
}

impl Batch {
    fn is_empty(&self) -> bool {
        self.rows.is_empty() && self.cursors.is_empty()
    }

    fn touch(&mut self) {
        if self.since.is_none() {
            self.since = Some(Instant::now());
        }
    }

    /// Añade un avance; fusiona con el anterior si es del mismo (comandante, archivo) y no es un reinicio.
    fn advance(&mut self, fid: &str, file: &str, line: u64) {
        self.touch();
        if line != 0
            && let Some(last) = self.cursors.last_mut()
            && last.last_line != 0
            && last.cmdr_fid == fid
            && last.journal_file == file
        {
            last.last_line = last.last_line.max(line);
            return;
        }
        self.cursors.push(CursorAdvance {
            cmdr_fid: fid.to_string(),
            journal_file: file.to_string(),
            last_line: line,
        });
    }

    fn extend_from(&mut self, mut other: Batch) {
        if other.is_empty() && other.lines == 0 {
            return;
        }
        self.touch();
        self.lines += other.lines;
        self.rows.append(&mut other.rows);
        for c in other.cursors.drain(..) {
            self.advance(&c.cmdr_fid, &c.journal_file, c.last_line);
        }
    }
}

struct PendingCompanion {
    ev: JournalEvent,
    file: String,
    line: u64,
    fid: String,
    snap: TransientSnapshot,
    row_origin: RowOrigin,
}

pub struct Router {
    store: Store,
    clock: Arc<dyn Clock>,
    gates: Arc<SendGates>,
    cfg: RouterConfig,
    mappers: Vec<Box<dyn ServiceMapper>>,
    tracker: TransientTracker,
    replay_marker: bool,
    /// Última muestra de proceso (`None` = aún ninguna).
    process: Option<bool>,
    /// Archivo de la última línea evaluada.
    last_file: Option<String>,
    /// Archivos cuyas líneas `CatchUp` son recuperables: vigente al arrancar y anterior de la sesión.
    recoverable_files: Vec<String>,
    /// La relectura sigue abierta: ni `CatchUpDone` ni línea en vivo todavía.
    recovery_open: bool,
    /// Filas y cursores recuperados, retenidos hasta `CatchUpDone`.
    staged: Batch,
    /// Última línea vista por archivo en esta ejecución (detección de truncado).
    last_line: HashMap<String, u64>,
    /// Cursor conocido por (comandante, archivo): del almacén o del lote en curso.
    cursors: HashMap<(String, String), u64>,
    batch: Batch,
    held: VecDeque<SyncFeed>,
    held_since: Option<Instant>,
    pending_companion: Option<PendingCompanion>,
    enqueued_wakers: Vec<Arc<Notify>>,
    recovered_totals: Option<Arc<[AtomicU64; 3]>>,
}

impl Router {
    pub fn new(
        store: Store,
        clock: Arc<dyn Clock>,
        online: Arc<AtomicBool>,
        lease_held: Arc<AtomicBool>,
        cfg: RouterConfig,
        mappers: Vec<Box<dyn ServiceMapper>>,
    ) -> Self {
        let mut router = Self {
            store,
            clock,
            gates: Arc::new(SendGates::new(online, lease_held)),
            cfg,
            mappers,
            tracker: TransientTracker::new(),
            replay_marker: true,
            process: None,
            last_file: None,
            recoverable_files: Vec::new(),
            recovery_open: true,
            staged: Batch::default(),
            last_line: HashMap::new(),
            cursors: HashMap::new(),
            batch: Batch::default(),
            held: VecDeque::new(),
            held_since: None,
            pending_companion: None,
            enqueued_wakers: Vec::new(),
            recovered_totals: None,
        };
        router.refresh_replay_marker();
        router.publish();
        router
    }

    /// Archivo vigente al arrancar (el último de `list_journals`): sus líneas `CatchUp` son recuperables.
    pub fn set_current_file(&mut self, file: &str) {
        if !self.recoverable_files.iter().any(|f| f == file) {
            self.recoverable_files.push(file.to_string());
        }
    }

    /// Recuperación de arranque (ruling T0-5): llamar justo tras `new`, antes de cualquier `handle`.
    /// Si la carpeta no se puede listar, no se recupera nada (falla cerrado).
    pub fn recover_previous_session(&mut self) -> Result<(), StoreError> {
        match Recovery::new(self.cfg.journal_dir.clone()).plan() {
            Ok(Some(plan)) => self.recover_with(&plan),
            Ok(None) => Ok(()),
            Err(error) => {
                tracing::warn!(failure_class = ?error.kind(), "sync recovery: cannot list journals");
                Ok(())
            }
        }
    }

    /// Aplica un plan de recuperación: fija el vigente y evalúa el anterior de la misma sesión.
    /// Solo actúa mientras la relectura sigue abierta. Un error de lectura detiene la recuperación
    /// del anterior en esa línea (lo ya evaluado se conserva; el cursor no pasa de ahí).
    pub fn recover_with(&mut self, plan: &RecoveryPlan) -> Result<(), StoreError> {
        if !self.recovery_open {
            return Ok(());
        }
        self.set_current_file(&plan.current);
        let Some(previous) = &plan.previous else {
            return Ok(());
        };
        if let Some(name) = previous.file_name() {
            self.set_current_file(&name.to_string_lossy());
        }
        match Recovery::stream_feeds(previous, |feed| self.handle(feed)) {
            Err(StoreError::Io(error)) => {
                tracing::warn!(failure_class = ?error.kind(), "sync recovery: previous journal unreadable");
                Ok(())
            }
            other => other,
        }
    }

    /// Estado de guardas compartido con trabajadores y agregador.
    pub fn send_gates(&self) -> Arc<SendGates> {
        self.gates.clone()
    }

    /// Registra notificadores que se despiertan cuando `flush` inserta filas nuevas.
    pub fn set_enqueued_wakers(&mut self, wakers: Vec<Arc<Notify>>) {
        self.enqueued_wakers = wakers;
    }

    /// Registra acumuladores atómicos para notificar filas recuperadas al agregador de estado.
    pub(crate) fn set_recovered_totals(&mut self, totals: Arc<[AtomicU64; 3]>) {
        self.recovered_totals = Some(totals);
    }

    /// Estado transitorio tras la última línea procesada.
    pub fn tracker(&self) -> &TransientTracker {
        &self.tracker
    }

    /// Procesa un evento del feed; vacía el lote al llegar a `flush_rows`. Si falla, el evento no se
    /// pierde: queda retenido en orden y se reintenta en la siguiente llamada a `handle` o `tick`.
    pub fn handle(&mut self, feed: SyncFeed) -> Result<(), StoreError> {
        self.gates
            .galaxy_not_legacy
            .store(feed.galaxy != Galaxy::Legacy, Ordering::SeqCst);
        let result = self.route(feed);
        self.publish();
        result
    }

    fn route(&mut self, feed: SyncFeed) -> Result<(), StoreError> {
        if let SyncFeedEvent::GameProcess { running } = feed.event {
            self.process = Some(running);
            self.release_held()?;
        } else {
            if let Err(error) = self.release_held_if_expired() {
                self.held.push_back(feed);
                return Err(error);
            }
            if !self.held.is_empty() || self.must_hold(&feed) {
                if self.held.is_empty() {
                    self.held_since = Some(Instant::now());
                }
                self.held.push_back(feed);
                if self.held.len() >= self.cfg.hold_before_process.0 {
                    self.release_held()?;
                }
            } else if let Err(error) = self.process_feed(&feed) {
                self.requeue(feed);
                return Err(error);
            }
        }
        self.flush_if_full()
    }

    /// Confirma el lote (filas y cursores) en una única transacción.
    /// Si una guarda global cayó tras evaluar la línea, solo se confirman los cursores (C2).
    pub fn flush(&mut self) -> Result<EnqueueOutcome, StoreError> {
        if self.batch.is_empty() {
            self.batch = Batch::default();
            return Ok(EnqueueOutcome::default());
        }
        let rows: &[NewOutboxRow] = if self.gates.global_reason().is_some() {
            &[]
        } else {
            &self.batch.rows
        };
        let outcome = self.store.enqueue_batch_with_cursor(
            rows,
            &self.batch.cursors,
            self.clock.now_utc(),
        )?;
        self.batch = Batch::default();
        if let Some(totals) = &self.recovered_totals {
            for (slot, count) in totals.iter().zip(outcome.recovered_by_service) {
                if count > 0 {
                    slot.fetch_add(count, Ordering::AcqRel);
                }
            }
        }
        if outcome.inserted > 0 {
            for wake in &self.enqueued_wakers {
                wake.notify_one();
            }
        }
        Ok(outcome)
    }

    /// Próximo instante en que `tick` tiene trabajo (vaciado por tiempo o fin de retención).
    pub fn next_deadline(&self) -> Option<Instant> {
        let flush = self.batch.since.map(|t| t + self.cfg.flush_every);
        let hold = self.held_since.map(|t| t + self.cfg.hold_before_process.1);
        match (flush, hold) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// Vence retenciones y vacía el lote si han pasado `flush_every` desde su primera línea.
    pub fn tick(&mut self) -> Result<(), StoreError> {
        self.release_held_if_expired()?;
        if self
            .batch
            .since
            .is_some_and(|t| Instant::now() >= t + self.cfg.flush_every)
        {
            self.flush()?;
        }
        self.publish();
        Ok(())
    }

    /// Motivo por el que un servicio no encola para un comandante (para el agregador).
    pub fn blocked(&self, service: SyncService, cmdr_fid: Option<&str>) -> Option<SyncReason> {
        blocked_with(&self.gates, &self.store, service, cmdr_fid)
    }
}

/// Evalúa el bloqueo efectivo con las guardas compartidas y el estado en almacén.
pub(crate) fn blocked_with(
    gates: &SendGates,
    store: &Store,
    service: SyncService,
    cmdr_fid: Option<&str>,
) -> Option<SyncReason> {
    if let Some(reason) = gates.global_reason() {
        return Some(reason);
    }
    if let Some(reason) = service_reason_in(store, service, cmdr_fid) {
        return Some(reason);
    }
    gates
        .game_not_running()
        .then_some(SyncReason::GameNotRunning)
}

fn service_reason_in(
    store: &Store,
    service: SyncService,
    cmdr_fid: Option<&str>,
) -> Option<SyncReason> {
    match service {
        SyncService::Eddn => match eddn_enabled_at(store) {
            Some(_) => None,
            None => Some(SyncReason::SwitchOff),
        },
        SyncService::Inara | SyncService::Edsm => {
            if let Some(fid) = cmdr_fid {
                match store.sync_credential(service, fid) {
                    Ok(Some(_)) => return None,
                    Ok(None) => {}
                    Err(_) => return Some(SyncReason::NoKey),
                }
            }
            let ambiguous = store.unassigned_keys().is_ok_and(|keys| {
                keys.iter()
                    .any(|k| k.service == service && k.reason == UnassignedReason::Ambiguous)
            });
            Some(if ambiguous {
                SyncReason::KeyUnassigned
            } else {
                SyncReason::NoKey
            })
        }
    }
}

impl Router {
    fn must_hold(&self, feed: &SyncFeed) -> bool {
        self.cfg.live_signal == LiveSignalPolicy::ProcessCheck
            && self.process.is_none()
            && feed.origin == FeedOrigin::Live
            && matches!(
                feed.event,
                SyncFeedEvent::Journal { .. } | SyncFeedEvent::Companion(_)
            )
    }

    fn release_held_if_expired(&mut self) -> Result<(), StoreError> {
        if self
            .held_since
            .is_some_and(|t| Instant::now() >= t + self.cfg.hold_before_process.1)
        {
            self.release_held()?;
        }
        Ok(())
    }

    /// Evalúa en orden las líneas retenidas con la muestra de proceso disponible (o sin ella).
    fn release_held(&mut self) -> Result<(), StoreError> {
        while let Some(feed) = self.held.pop_front() {
            if let Err(error) = self.process_feed(&feed) {
                self.requeue(feed);
                return Err(error);
            }
            self.flush_if_full()?;
        }
        self.held_since = None;
        Ok(())
    }

    /// Devuelve a la cabeza de la retención un evento que falló, con reintento inmediato.
    fn requeue(&mut self, feed: SyncFeed) {
        self.held.push_front(feed);
        let now = Instant::now();
        self.held_since = Some(
            now.checked_sub(self.cfg.hold_before_process.1)
                .unwrap_or(now),
        );
    }

    fn flush_if_full(&mut self) -> Result<(), StoreError> {
        if self.batch.rows.len() >= self.cfg.flush_rows || self.batch.lines >= self.cfg.flush_rows {
            self.flush()?;
        }
        Ok(())
    }

    fn process_feed(&mut self, feed: &SyncFeed) -> Result<(), StoreError> {
        match &feed.event {
            SyncFeedEvent::Journal { ev, file, line } => {
                if feed.origin == FeedOrigin::Live && self.recovery_open {
                    self.abort_recovery();
                }
                self.resolve_pending_companion(None)?;
                self.process_line(ev, file, *line, feed.origin, feed.galaxy)
            }
            SyncFeedEvent::Companion(comp) => {
                if self.pending_companion.as_ref().is_some_and(|p| {
                    crate::eddn::companion::companion_matches_event(&p.ev.name, comp)
                }) {
                    self.resolve_pending_companion(Some(comp))?;
                }
                Ok(())
            }
            SyncFeedEvent::CatchUpDone => {
                self.resolve_pending_companion(None)?;
                self.finish_recovery();
                Ok(())
            }
            SyncFeedEvent::GameProcess { running } => {
                self.process = Some(*running);
                Ok(())
            }
        }
    }

    fn finish_recovery(&mut self) {
        if !self.recovery_open {
            return;
        }
        self.recovery_open = false;
        let staged = std::mem::take(&mut self.staged);
        self.batch.extend_from(staged);
    }

    fn abort_recovery(&mut self) {
        if !self.recovery_open {
            return;
        }
        self.recovery_open = false;
        if self
            .pending_companion
            .as_ref()
            .is_some_and(|p| p.row_origin == RowOrigin::Recovered)
        {
            self.pending_companion = None;
        }
        for c in &self.staged.cursors {
            self.cursors
                .remove(&(c.cmdr_fid.clone(), c.journal_file.clone()));
        }
        self.staged = Batch::default();
    }

    fn process_line(
        &mut self,
        ev: &JournalEvent,
        file: &str,
        line: u64,
        origin: FeedOrigin,
        feed_galaxy: Galaxy,
    ) -> Result<(), StoreError> {
        if self.last_file.as_deref() != Some(file) {
            let inherited = self.last_file.as_deref().is_some_and(|prev| {
                same_session(
                    &self.cfg.journal_dir.join(prev),
                    &self.cfg.journal_dir.join(file),
                )
            });
            if !inherited {
                // Archivo nuevo de otra sesión: estado propio del archivo (PD-9).
                self.tracker.reset_for_new_file();
            }
            self.refresh_replay_marker();
            self.last_file = Some(file.to_string());
        } else if self.last_line.get(file).is_some_and(|&last| line <= last) {
            self.on_truncated(file);
        }

        if origin == FeedOrigin::Live && self.recovery_open {
            self.abort_recovery();
        }

        self.tracker.apply(ev);
        let snap = self.tracker.snapshot();
        self.content_state(&snap, feed_galaxy);
        if !self.gates.lease_held.load(Ordering::SeqCst) {
            // Sin lease esta instancia es solo observadora: no escribe comandantes ni cursores en BD.
            self.last_line.insert(file.to_string(), line);
            return Ok(());
        }
        // Datos de replay o simulación nunca registran comandantes ni reciben claves en espera.
        if !(self.replay_marker || self.cfg.env.e2e_simulation) {
            self.register_cmdr(ev)?;
        }

        if let Some(fid) = snap.cmdr_fid.clone() {
            let recoverable_catchup = origin == FeedOrigin::CatchUp
                && self.recovery_open
                && self.recoverable_files.iter().any(|f| f == file);
            if (origin != FeedOrigin::CatchUp || recoverable_catchup)
                && line > self.cursor(&fid, file)?
            {
                let is_live = origin == FeedOrigin::Live;
                let mut defer_cursor = false;
                if self.line_block(&snap, feed_galaxy, is_live).is_none() {
                    let row_origin = if is_live {
                        RowOrigin::Live
                    } else {
                        RowOrigin::Recovered
                    };
                    if self.line_wants_companion(ev, &fid)? {
                        self.pending_companion = Some(PendingCompanion {
                            ev: ev.clone(),
                            file: file.to_string(),
                            line,
                            fid: fid.clone(),
                            snap,
                            row_origin,
                        });
                        defer_cursor = true;
                    } else {
                        self.map_line(ev, file, line, &fid, &snap, row_origin, None)?;
                    }
                }
                if !defer_cursor {
                    let target = if is_live {
                        &mut self.batch
                    } else {
                        &mut self.staged
                    };
                    target.lines += 1;
                    target.advance(&fid, file, line);
                    self.cursors.insert((fid, file.to_string()), line);
                }
            }
        }
        // Solo tras completar la evaluación sin error para no falsar truncado al reintentar.
        self.last_line.insert(file.to_string(), line);
        Ok(())
    }

    /// La numeración del archivo vuelve a empezar: cursores a 0 antes de las líneas nuevas (T0-2).
    fn on_truncated(&mut self, file: &str) {
        tracing::debug!("sync router: journal file truncated, cursors reset");
        if self
            .pending_companion
            .as_ref()
            .is_some_and(|p| p.file == file)
        {
            self.pending_companion = None;
        }
        self.tracker.reset_for_new_file();
        self.refresh_replay_marker();
        let mut owners: Vec<String> = self
            .cursors
            .keys()
            .filter(|(_, f)| f == file)
            .map(|(fid, _)| fid.clone())
            .collect();
        owners.sort();
        for fid in owners {
            self.batch.advance(&fid, file, 0);
            self.cursors.insert((fid, file.to_string()), 0);
        }
    }

    fn cursor(&mut self, fid: &str, file: &str) -> Result<u64, StoreError> {
        let key = (fid.to_string(), file.to_string());
        if let Some(&line) = self.cursors.get(&key) {
            return Ok(line);
        }
        let line = self.store.sync_cursor(fid, file)?.unwrap_or(0);
        self.cursors.insert(key, line);
        Ok(line)
    }

    fn line_wants_companion(&self, ev: &JournalEvent, fid: &str) -> Result<bool, StoreError> {
        for mapper in &self.mappers {
            if !mapper.wants_companion(ev) {
                continue;
            }
            let Some(activated_at) = active_since(&self.store, mapper.service(), fid)? else {
                continue;
            };
            if ev.timestamp >= activated_at {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn resolve_pending_companion(
        &mut self,
        companion: Option<&CompanionFile>,
    ) -> Result<(), StoreError> {
        let Some(pending) = self.pending_companion.take() else {
            return Ok(());
        };
        if let Err(err) = self.map_line(
            &pending.ev,
            &pending.file,
            pending.line,
            &pending.fid,
            &pending.snap,
            pending.row_origin,
            companion,
        ) {
            self.pending_companion = Some(pending);
            return Err(err);
        }
        let target = if pending.row_origin == RowOrigin::Live {
            &mut self.batch
        } else {
            &mut self.staged
        };
        target.lines += 1;
        target.advance(&pending.fid, &pending.file, pending.line);
        self.cursors
            .insert((pending.fid, pending.file), pending.line);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn map_line(
        &mut self,
        ev: &JournalEvent,
        file: &str,
        line: u64,
        fid: &str,
        snap: &TransientSnapshot,
        row_origin: RowOrigin,
        companion: Option<&CompanionFile>,
    ) -> Result<(), StoreError> {
        let input = MappedInput {
            ev,
            file,
            line,
            snap,
            companion,
            tracker: &self.tracker,
        };
        let now = self.clock.now_utc();
        let mut out = Vec::new();
        for mapper in &mut self.mappers {
            let service = mapper.service();
            let Some(activated_at) = active_since(&self.store, service, fid)? else {
                continue;
            };
            if ev.timestamp < activated_at {
                continue;
            }
            out.clear();
            mapper.map(&input, &mut out);
            let target = if row_origin == RowOrigin::Recovered {
                &mut self.staged.rows
            } else {
                &mut self.batch.rows
            };
            for row in out.drain(..) {
                let state = if row_origin == RowOrigin::Recovered
                    && now - ev.timestamp > service.max_age()
                {
                    OutboxState::Expired
                } else {
                    row.state
                };
                target.push(NewOutboxRow {
                    service,
                    cmdr_fid: fid.to_string(),
                    idem_key: idem_key(service, fid, file, line, row.sub),
                    kind: row.kind,
                    coalesce_key: row.coalesce_key,
                    event_ts: ev.timestamp,
                    payload: row.payload.to_string(),
                    origin: row_origin,
                    state,
                    last_class: row.class,
                });
            }
        }
        Ok(())
    }

    /// Guardas de una línea con FID, en orden; `None` = se puede mapear.
    fn line_block(
        &self,
        snap: &TransientSnapshot,
        feed_galaxy: Galaxy,
        check_process: bool,
    ) -> Option<SyncReason> {
        if !self.gates.lease_held.load(Ordering::SeqCst) {
            return Some(SyncReason::OtherInstance);
        }
        if let Some(reason) = self.env_reason() {
            return Some(reason);
        }
        if !self.gates.online.load(Ordering::SeqCst) {
            return Some(SyncReason::OnlineOff);
        }
        // Galaxia del propio archivo: Legacy o desconocida bloquean; el actor en Legacy también.
        if snap.galaxy != Galaxy::Live || feed_galaxy == Galaxy::Legacy {
            return Some(SyncReason::Legacy);
        }
        if snap.is_beta {
            return Some(SyncReason::Beta);
        }
        if check_process
            && self.cfg.live_signal == LiveSignalPolicy::ProcessCheck
            && self.process != Some(true)
        {
            return Some(SyncReason::GameNotRunning);
        }
        None
    }

    /// Replay (marcador o e2e) o plataforma sin soporte.
    fn env_reason(&self) -> Option<SyncReason> {
        if self.replay_marker || self.cfg.env.e2e_simulation {
            Some(SyncReason::Replay)
        } else if self.cfg.live_signal == LiveSignalPolicy::Unavailable {
            Some(SyncReason::PlatformUnsupported)
        } else {
            None
        }
    }

    fn content_state(&self, snap: &TransientSnapshot, feed_galaxy: Galaxy) {
        let code = if snap.galaxy != Galaxy::Live || feed_galaxy == Galaxy::Legacy {
            R_LEGACY
        } else if snap.is_beta {
            R_BETA
        } else {
            R_NONE
        };
        self.gates.content_reason.store(code, Ordering::SeqCst);
    }

    /// Comprueba el marcador; si no se puede comprobar, se considera presente (falla cerrado).
    fn refresh_replay_marker(&mut self) {
        self.replay_marker = self
            .cfg
            .journal_dir
            .join(REPLAY_MARKER)
            .try_exists()
            .unwrap_or(true);
    }

    fn publish(&self) {
        let env = match self.env_reason() {
            Some(SyncReason::Replay) => R_REPLAY,
            Some(SyncReason::PlatformUnsupported) => R_PLATFORM,
            _ => R_NONE,
        };
        self.gates.env_reason.store(env, Ordering::SeqCst);
        let not_running =
            self.cfg.live_signal == LiveSignalPolicy::ProcessCheck && self.process != Some(true);
        self.gates
            .game_not_running
            .store(not_running, Ordering::SeqCst);
    }

    /// Registra el comandante visto y le asigna una clave heredada en espera (caso b, §6.2).
    fn register_cmdr(&self, ev: &JournalEvent) -> Result<(), StoreError> {
        let (fid, name) = match &ev.kind {
            EventKind::Commander(c) => (c.fid.as_deref(), c.name.as_deref()),
            EventKind::LoadGame(l) => (l.fid.as_deref(), l.commander.as_deref()),
            _ => return Ok(()),
        };
        let (Some(fid), Some(name)) = (fid, name) else {
            return Ok(());
        };
        let now = self.clock.now_utc();
        self.store.upsert_known_cmdr(fid, name, now)?;
        for key in self.store.unassigned_keys()? {
            if key.reason == UnassignedReason::WaitingFirstCmdr {
                self.store.assign_unassigned_key(
                    key.service,
                    fid,
                    name,
                    KeyAssignment::AutoPendingConfirmation,
                    now,
                )?;
            }
        }
        // EDDN encendido: la activación del comandante es el instante de encendido (PD-3).
        if let Some(enabled_at) = eddn_enabled_at(&self.store)
            && self.store.activation(SyncService::Eddn, fid)?.is_none()
        {
            self.store
                .set_activation(SyncService::Eddn, fid, enabled_at)?;
        }
        Ok(())
    }
}

pub(crate) fn eddn_enabled_at(store: &Store) -> Option<DateTime<Utc>> {
    let value = store.kv_get(EDDN_ENABLED_KEY)?;
    DateTime::parse_from_rfc3339(&value)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

/// Instante desde el que el servicio está activo para el comandante (`None` = inactivo).
fn active_since(
    store: &Store,
    service: SyncService,
    fid: &str,
) -> Result<Option<DateTime<Utc>>, StoreError> {
    let active = match service {
        SyncService::Eddn => eddn_enabled_at(store).is_some(),
        SyncService::Inara | SyncService::Edsm => store.sync_credential(service, fid)?.is_some(),
    };
    if !active {
        return Ok(None);
    }
    store.activation(service, fid)
}
