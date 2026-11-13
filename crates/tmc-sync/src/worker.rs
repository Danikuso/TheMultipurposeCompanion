//! Trabajador genérico de envío con limitador, backoff y reinicio sin duplicados (C3, C11).

use crate::clock::Clock;
use crate::router::SendGates;
use chrono::{DateTime, SecondsFormat, Utc};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tmc_model::sync::{
    ErrorClass, OutboxState, RecentRejection, SERVER_HINT_MAX_CHARS, SyncService,
};
use tmc_net::limiter::{RateLimiter, SharedRateLimiter};
pub use tmc_net::transport::BoxFut;
use tmc_store::{OutboxRow, OutboxVerdict, Store, StoreError, SyncCredential};
use tokio::sync::{Notify, mpsc, watch};

/// Tope máximo de espera del trabajador entre evaluaciones (seguro ante saltos de reloj).
pub const MAX_WORKER_SLEEP: Duration = Duration::from_secs(1800);

/// Retroceso exponencial desde 60 s hasta 30 min para los tres servicios (PD-8).
pub fn backoff(_service: SyncService, attempts: u32) -> Duration {
    let shift = attempts.saturating_sub(1).min(5);
    Duration::from_secs((60u64 << shift).min(1800))
}

/// Política de agrupación y ritmo declarada por cada cliente de servicio.
#[derive(Debug, Clone)]
pub struct BatchPolicy {
    pub max_rows: usize,
    pub per_cmdr: bool,
    pub min_interval: Duration,
    pub limiter: RateLimiter,
    pub flush_after: Option<Duration>,
}

/// Contexto temporal y de credenciales del envío de un lote.
pub struct SendCtx<'a> {
    pub cmdr_fid: &'a str,
    pub credential: Option<&'a SyncCredential>,
    pub now: DateTime<Utc>,
}

/// Decisión individual del cliente sobre cada fila reclamada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowVerdict {
    Sent {
        code: String,
    },
    Rejected {
        code: String,
        class: ErrorClass,
        hint: Option<String>,
    },
    Discarded {
        code: String,
    },
    Superseded,
    Retry {
        code: String,
        class: ErrorClass,
        after: Option<Duration>,
    },
    Hold {
        code: String,
        class: ErrorClass,
    },
}

/// Efecto a nivel de servicio/comandante derivado de la respuesta del servidor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceEffect {
    None,
    Connected,
    CmdrError(ErrorClass),
    ServiceError {
        class: ErrorClass,
        until: Option<DateTime<Utc>>,
    },
    PauseSchema(String),
}

/// Resultado completo del envío de un lote.
pub struct SendOutcome {
    pub verdicts: Vec<(i64, RowVerdict)>,
    pub effect: ServiceEffect,
}

/// Contrato de cliente específico de servicio consumido por [`Worker`].
pub trait ServiceClient: Send + Sync {
    fn service(&self) -> SyncService;
    fn policy(&self) -> BatchPolicy;
    fn needs_credential(&self) -> bool;
    fn send<'a>(&'a self, ctx: SendCtx<'a>, rows: &'a [OutboxRow]) -> BoxFut<'a, SendOutcome>;
    fn filter_claimed(&self, rows: Vec<OutboxRow>) -> (Vec<OutboxRow>, Vec<(i64, RowVerdict)>) {
        (rows, Vec::new())
    }
    /// Claves `kind` actualmente en pausa (no reclamables por ahora, p. ej. HTTP 426 EDDN).
    fn excluded_kinds(&self) -> Vec<String> {
        Vec::new()
    }
    /// Limitador compartido entre trabajadores y verificaciones (p. ej. Inara 2/60 s).
    fn shared_limiter(&self) -> Option<SharedRateLimiter> {
        None
    }
}

/// Eventos publicados por los trabajadores hacia [`crate::status::StatusAggregator`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerEvent {
    Outcome {
        service: SyncService,
        cmdr_fid: String,
        effect: ServiceEffect,
        sent: u64,
        rejected: u64,
    },
    Rejection {
        service: SyncService,
        cmdr_fid: String,
        rejection: RecentRejection,
    },
}

/// Trabajador asíncrono para un [`ServiceClient`].
pub struct Worker {
    client: Arc<dyn ServiceClient>,
    store: Store,
    clock: Arc<dyn Clock>,
    gates: Arc<SendGates>,
    wake: Arc<Notify>,
    events: mpsc::Sender<WorkerEvent>,
    iterations: Arc<AtomicU64>,
}

struct WorkerState {
    policy: BatchPolicy,
    shared_limiter: Option<SharedRateLimiter>,
    held_cmdrs: BTreeSet<String>,
    last_send: Option<tokio::time::Instant>,
}

impl WorkerState {
    fn new(policy: BatchPolicy, shared_limiter: Option<SharedRateLimiter>) -> Self {
        Self {
            policy,
            shared_limiter,
            held_cmdrs: BTreeSet::new(),
            last_send: None,
        }
    }

    fn now_std(&self) -> std::time::Instant {
        tokio::time::Instant::now().into_std()
    }
}

impl Worker {
    pub fn new(
        client: Arc<dyn ServiceClient>,
        store: Store,
        clock: Arc<dyn Clock>,
        gates: Arc<SendGates>,
        wake: Arc<Notify>,
        events: mpsc::Sender<WorkerEvent>,
    ) -> Self {
        Self {
            client,
            store,
            clock,
            gates,
            wake,
            events,
            iterations: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Contador observable de iteraciones del bucle (tests C11 y saltos de reloj).
    pub fn iterations(&self) -> Arc<AtomicU64> {
        self.iterations.clone()
    }

    /// Bucle principal hasta que `shutdown` sea `true`.
    pub async fn run(self, mut shutdown: watch::Receiver<bool>) {
        let mut st = WorkerState::new(self.client.policy(), self.client.shared_limiter());
        loop {
            if *shutdown.borrow() {
                break;
            }
            self.iterations.fetch_add(1, Ordering::SeqCst);

            let wait = match self.step(&mut st, &mut shutdown).await {
                StepControl::Continue => continue,
                StepControl::Sleep(wait) => wait,
                StepControl::Stop => break,
            };

            if !self
                .wait_or_wake(wait, &mut shutdown, &mut st.held_cmdrs)
                .await
            {
                break;
            }
        }
    }

    async fn step(
        &self,
        st: &mut WorkerState,
        shutdown: &mut watch::Receiver<bool>,
    ) -> StepControl {
        if !self.gates.send_allowed() {
            return StepControl::Sleep(None);
        }

        let service = self.client.service();
        let now = self.clock.now_utc();
        let eligible = match self.eligible_cmdrs(service, &st.held_cmdrs) {
            Ok(list) => list,
            Err(error) => {
                tracing::warn!(
                    failure_class = error.diagnostic_class(),
                    "sync worker: pending query failed"
                );
                return StepControl::Sleep(Some(Duration::from_secs(5)));
            }
        };

        if eligible.is_empty() {
            // Cola vacía o todos en Hold: ningún temporizador en reposo (C11).
            return StepControl::Sleep(None);
        }

        if let Some(wait) = self.flush_wait(&st.policy, &eligible, now) {
            return StepControl::Sleep(Some(wait.min(MAX_WORKER_SLEEP)));
        }

        if let Some(wait) = self.throttle_wait(st) {
            return StepControl::Sleep(Some(wait.min(MAX_WORKER_SLEEP)));
        }

        let excluded = self.client.excluded_kinds();
        let rows = match self.claim_batch(
            service,
            &st.policy,
            &st.held_cmdrs,
            &eligible,
            &excluded,
            now,
        ) {
            Ok(rows) => rows,
            Err(error) => {
                tracing::warn!(
                    failure_class = error.diagnostic_class(),
                    "sync worker: claim failed"
                );
                return StepControl::Sleep(Some(Duration::from_secs(5)));
            }
        };

        if rows.is_empty() {
            let next = self.next_eligible_wait(service, &eligible, &excluded, now);
            return StepControl::Sleep(next);
        }

        self.process_batch(st, rows, shutdown).await
    }

    async fn process_batch(
        &self,
        st: &mut WorkerState,
        claimed: Vec<OutboxRow>,
        shutdown: &mut watch::Receiver<bool>,
    ) -> StepControl {
        let service = self.client.service();
        let all_ids: Vec<i64> = claimed.iter().map(|r| r.id).collect();
        let cmdr_fid = claimed[0].cmdr_fid.clone();

        if !self.gates.send_allowed() {
            let _ = self.store.release_rows(&all_ids);
            return StepControl::Sleep(None);
        }

        // Guarda de credencial justo antes de enviar.
        let credential = if self.client.needs_credential() {
            match self.store.sync_credential(service, &cmdr_fid) {
                Ok(Some(cred)) => Some(cred),
                Ok(None) => {
                    let _ = self.store.release_rows(&all_ids);
                    return StepControl::Continue;
                }
                Err(error) => {
                    tracing::warn!(
                        failure_class = error.diagnostic_class(),
                        "sync worker: credential check failed"
                    );
                    let _ = self.store.release_rows(&all_ids);
                    return StepControl::Sleep(Some(Duration::from_secs(5)));
                }
            }
        } else {
            None
        };

        let all_rows = claimed.clone();
        let (to_send, mut verdicts) = self.client.filter_claimed(claimed);
        if to_send.is_empty() {
            let effect = filter_only_effect(&verdicts);
            let halt = matches!(
                effect,
                ServiceEffect::ServiceError {
                    class: ErrorClass::SoftwareBlocked,
                    ..
                }
            );
            self.commit_verdicts(st, &cmdr_fid, &all_rows, verdicts, effect)
                .await;
            return if halt {
                StepControl::Stop
            } else {
                StepControl::Continue
            };
        }

        // Espera el limitador si toca justo antes de lanzar el POST.
        loop {
            let now_lim = st.now_std();
            let slot = match &st.shared_limiter {
                Some(shared) => shared.try_acquire(now_lim),
                None => st.policy.limiter.try_acquire(now_lim),
            };
            match slot {
                Ok(()) => break,
                Err(wait) => {
                    if !self
                        .wait_or_wake(
                            Some(wait.min(MAX_WORKER_SLEEP)),
                            shutdown,
                            &mut st.held_cmdrs,
                        )
                        .await
                    {
                        let _ = self.store.release_rows(&all_ids);
                        return StepControl::Stop;
                    }
                    if !self.gates.send_allowed() {
                        let _ = self.store.release_rows(&all_ids);
                        return StepControl::Sleep(None);
                    }
                }
            }
        }

        if !self.gates.send_allowed() {
            let _ = self.store.release_rows(&all_ids);
            return StepControl::Sleep(None);
        }

        let send_now = self.clock.now_utc();
        let outcome = self
            .client
            .send(
                SendCtx {
                    cmdr_fid: &cmdr_fid,
                    credential: credential.as_ref(),
                    now: send_now,
                },
                &to_send,
            )
            .await;
        st.last_send = Some(tokio::time::Instant::now());

        // Si alguna guarda cayó durante el envío (p. ej. modo desconectado a mitad de lote),
        // no aplicamos veredictos `sent`: devolvemos el lote a `pending`.
        if !self.gates.send_allowed() {
            let _ = self.store.release_rows(&all_ids);
            return StepControl::Sleep(None);
        }

        let halt = matches!(
            outcome.effect,
            ServiceEffect::ServiceError {
                class: ErrorClass::SoftwareBlocked,
                ..
            }
        );
        verdicts.extend(outcome.verdicts);
        self.commit_verdicts(st, &cmdr_fid, &all_rows, verdicts, outcome.effect)
            .await;
        if halt {
            StepControl::Stop
        } else {
            StepControl::Continue
        }
    }

    async fn commit_verdicts(
        &self,
        st: &mut WorkerState,
        cmdr_fid: &str,
        sent_rows: &[OutboxRow],
        verdicts: Vec<(i64, RowVerdict)>,
        effect: ServiceEffect,
    ) {
        let service = self.client.service();
        let now = self.clock.now_utc();
        let now_rfc = now.to_rfc3339_opts(SecondsFormat::Secs, true);
        let row_map: HashMap<i64, &OutboxRow> = sent_rows.iter().map(|r| (r.id, r)).collect();

        let mut seen_ids = BTreeSet::new();
        let mut updates = Vec::with_capacity(sent_rows.len() + verdicts.len());
        let mut sent_count = 0u64;
        let mut rejected_count = 0u64;
        let mut rejections = Vec::new();

        for (id, verdict) in verdicts {
            seen_ids.insert(id);
            let row_opt = row_map.get(&id).copied();
            match verdict {
                RowVerdict::Sent { code } => {
                    sent_count += 1;
                    updates.push(OutboxVerdict::Sent { id, code });
                }
                RowVerdict::Rejected { code, class, hint } => {
                    rejected_count += 1;
                    let kind = row_opt.map(|r| r.kind.clone()).unwrap_or_default();
                    let trimmed = hint.map(|h| trim_chars(&h, SERVER_HINT_MAX_CHARS));
                    rejections.push((
                        row_opt
                            .map(|r| r.cmdr_fid.clone())
                            .unwrap_or_else(|| cmdr_fid.to_string()),
                        RecentRejection {
                            kind,
                            class,
                            code: code.clone(),
                            at: now_rfc.clone(),
                            hint: trimmed,
                        },
                    ));
                    updates.push(OutboxVerdict::Rejected { id, code, class });
                }
                RowVerdict::Discarded { code } => {
                    updates.push(OutboxVerdict::Closed {
                        id,
                        state: OutboxState::Discarded,
                        code: Some(code),
                        class: None,
                    });
                }
                RowVerdict::Superseded => {
                    updates.push(OutboxVerdict::Closed {
                        id,
                        state: OutboxState::Superseded,
                        code: None,
                        class: None,
                    });
                }
                RowVerdict::Retry { code, class, after } => {
                    let attempts = row_opt.map(|r| r.attempts + 1).unwrap_or(1);
                    let delay = after.unwrap_or_else(|| backoff(service, attempts));
                    let next_at = now
                        + chrono::Duration::from_std(delay)
                            .unwrap_or_else(|_| chrono::Duration::seconds(1800));
                    updates.push(OutboxVerdict::Retry {
                        id,
                        next_attempt_at: next_at,
                        code,
                        class,
                    });
                }
                RowVerdict::Hold { code, class } => {
                    let fid = row_opt
                        .map(|r| r.cmdr_fid.clone())
                        .unwrap_or_else(|| cmdr_fid.to_string());
                    st.held_cmdrs.insert(fid);
                    updates.push(OutboxVerdict::Hold { id, code, class });
                }
            }
        }

        // Cualquier fila reclamada sin veredicto vuelve a `pending`.
        for row in sent_rows {
            if !seen_ids.contains(&row.id) {
                updates.push(OutboxVerdict::Release { id: row.id });
            }
        }

        if let Err(error) = self.store.apply_verdicts(&updates, now) {
            tracing::warn!(
                failure_class = error.diagnostic_class(),
                "sync worker: verdict commit failed"
            );
            return;
        }

        for (fid, rejection) in rejections {
            let _ = self
                .events
                .send(WorkerEvent::Rejection {
                    service,
                    cmdr_fid: fid,
                    rejection,
                })
                .await;
        }
        let _ = self
            .events
            .send(WorkerEvent::Outcome {
                service,
                cmdr_fid: cmdr_fid.to_string(),
                effect,
                sent: sent_count,
                rejected: rejected_count,
            })
            .await;
    }

    fn eligible_cmdrs(
        &self,
        service: SyncService,
        held: &BTreeSet<String>,
    ) -> Result<Vec<(String, u64, DateTime<Utc>)>, StoreError> {
        let rows = self.store.pending_by_cmdr(service)?;
        let mut eligible = Vec::new();
        for (fid, count, oldest_ts) in rows {
            if held.contains(&fid) {
                continue;
            }
            if self.client.needs_credential()
                && self.store.sync_credential(service, &fid)?.is_none()
            {
                continue;
            }
            eligible.push((fid, count, oldest_ts));
        }
        eligible.sort_by_key(|(_, _, oldest)| *oldest);
        Ok(eligible)
    }

    fn flush_wait(
        &self,
        policy: &BatchPolicy,
        eligible: &[(String, u64, DateTime<Utc>)],
        now: DateTime<Utc>,
    ) -> Option<Duration> {
        let max_wait = policy.flush_after?;
        let max_wait_chrono = chrono::Duration::from_std(max_wait).ok()?;
        let mut earliest: Option<Duration> = None;
        for (_, count, oldest_ts) in eligible {
            if *count as usize >= policy.max_rows || now - *oldest_ts >= max_wait_chrono {
                return None;
            }
            let remaining = (*oldest_ts + max_wait_chrono - now)
                .to_std()
                .unwrap_or(Duration::ZERO);
            earliest = Some(match earliest {
                Some(cur) => cur.min(remaining),
                None => remaining,
            });
        }
        earliest
    }

    fn throttle_wait(&self, st: &WorkerState) -> Option<Duration> {
        if st.policy.min_interval.is_zero() {
            return None;
        }
        let last = st.last_send?;
        let elapsed = tokio::time::Instant::now().saturating_duration_since(last);
        st.policy.min_interval.checked_sub(elapsed)
    }

    fn claim_batch(
        &self,
        service: SyncService,
        policy: &BatchPolicy,
        held: &BTreeSet<String>,
        eligible: &[(String, u64, DateTime<Utc>)],
        excluded_kinds: &[String],
        now: DateTime<Utc>,
    ) -> Result<Vec<OutboxRow>, StoreError> {
        if !policy.per_cmdr && held.is_empty() && !self.client.needs_credential() {
            return self.store.claim_ready_excluding(
                service,
                None,
                excluded_kinds,
                policy.max_rows,
                now,
            );
        }
        for (fid, _, _) in eligible {
            let rows = self.store.claim_ready_excluding(
                service,
                Some(fid),
                excluded_kinds,
                policy.max_rows,
                now,
            )?;
            if !rows.is_empty() {
                return Ok(rows);
            }
        }
        Ok(Vec::new())
    }

    fn next_eligible_wait(
        &self,
        service: SyncService,
        eligible: &[(String, u64, DateTime<Utc>)],
        excluded_kinds: &[String],
        now: DateTime<Utc>,
    ) -> Option<Duration> {
        let mut earliest: Option<DateTime<Utc>> = None;
        for (fid, _, _) in eligible {
            if let Ok(Some(next_at)) =
                self.store
                    .next_attempt_at_excluding(service, Some(fid), excluded_kinds)
            {
                earliest = Some(match earliest {
                    Some(cur) => cur.min(next_at),
                    None => next_at,
                });
            }
        }
        earliest.map(|next_at| {
            if next_at <= now {
                Duration::ZERO
            } else {
                (next_at - now)
                    .to_std()
                    .unwrap_or(MAX_WORKER_SLEEP)
                    .min(MAX_WORKER_SLEEP)
            }
        })
    }

    async fn wait_or_wake(
        &self,
        sleep: Option<Duration>,
        shutdown: &mut watch::Receiver<bool>,
        held_cmdrs: &mut BTreeSet<String>,
    ) -> bool {
        if *shutdown.borrow() {
            return false;
        }
        match sleep {
            None => {
                tokio::select! {
                    _ = self.wake.notified() => {
                        held_cmdrs.clear();
                        true
                    }
                    changed = shutdown.changed() => !(changed.is_err() || *shutdown.borrow()),
                }
            }
            Some(dur) => {
                tokio::select! {
                    _ = self.wake.notified() => {
                        held_cmdrs.clear();
                        true
                    }
                    changed = shutdown.changed() => !(changed.is_err() || *shutdown.borrow()),
                    _ = tokio::time::sleep(dur) => true,
                }
            }
        }
    }
}

fn filter_only_effect(verdicts: &[(i64, RowVerdict)]) -> ServiceEffect {
    for (_, verdict) in verdicts {
        if let RowVerdict::Hold { class, .. } = verdict
            && matches!(
                class,
                ErrorClass::DiscardListMissing | ErrorClass::SoftwareBlocked
            )
        {
            return ServiceEffect::ServiceError {
                class: *class,
                until: None,
            };
        }
    }
    ServiceEffect::None
}

enum StepControl {
    Continue,
    Sleep(Option<Duration>),
    Stop,
}

fn trim_chars(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}
