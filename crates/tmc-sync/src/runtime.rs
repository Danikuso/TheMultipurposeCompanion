//! Composición y ciclo de vida en ejecución de la sincronización (C3, C11).

use crate::clock::Clock;
use crate::housekeeping::run_housekeeping;
use crate::lease::Lease;
use crate::router::{
    EnvGates, LiveSignalPolicy, Router, RouterConfig, ServiceMapper, blocked_with,
};
use crate::status::StatusAggregator;
use crate::worker::{ServiceClient, Worker, WorkerEvent};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tmc_model::diagnostic;
use tmc_model::diagnostics::{DiagnosticContext, json};
use tmc_model::source::SourceEvent;
use tmc_model::sync::SyncFeed;
use tmc_net::Transport;
use tmc_store::Store;
use tokio::sync::{Notify, mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::Instant;

/// Dependencias necesarias para iniciar la sincronización de servicios.
pub struct SyncDeps {
    pub store: Store,
    pub transport: Arc<dyn Transport>,
    pub online: Arc<AtomicBool>,
    pub core_events: mpsc::Sender<SourceEvent>,
    pub journal_dir: PathBuf,
    pub env: EnvGates,
    pub live_signal: LiveSignalPolicy,
    pub clock: Arc<dyn Clock>,
    pub process_kind: &'static str,
    pub clients: Vec<Arc<dyn ServiceClient>>,
    pub mappers: Vec<Box<dyn ServiceMapper>>,
}

/// Notificador compartible para despertar trabajadores y agregador (`accounts.sync_now`).
#[derive(Clone, Default)]
pub struct SyncWaker {
    wakers: Vec<Arc<Notify>>,
    status_wake: Arc<Notify>,
}

impl SyncWaker {
    pub fn wake_all(&self) {
        for wake in &self.wakers {
            wake.notify_one();
        }
        self.status_wake.notify_one();
    }
}

/// Control en ejecución de la sincronización; propietario del feed y del cierre ordenado.
pub struct SyncHandle {
    pub feed: mpsc::UnboundedSender<SyncFeed>,
    wakers: Vec<Arc<Notify>>,
    status_wake: Arc<Notify>,
    router_shutdown: watch::Sender<bool>,
    aux_shutdown: watch::Sender<bool>,
    router_and_workers: Vec<JoinHandle<()>>,
    aux_tasks: Vec<JoinHandle<()>>,
}

impl SyncHandle {
    pub fn wake_all(&self) {
        for wake in &self.wakers {
            wake.notify_one();
        }
        self.status_wake.notify_one();
    }

    pub fn waker(&self) -> SyncWaker {
        SyncWaker {
            wakers: self.wakers.clone(),
            status_wake: self.status_wake.clone(),
        }
    }

    /// Señala parada, vacía el lote del router en curso y libera el lease antes de volver.
    pub async fn shutdown(mut self) {
        // 1. Parar primero trabajadores y router mientras el lease sigue activo para confirmar el lote final.
        let _ = self.router_shutdown.send(true);
        self.wake_all();
        for handle in std::mem::take(&mut self.router_and_workers) {
            let _ = handle.await;
        }
        // 2. Ahora detener agregador, housekeeping y lease (que libera el registro en SQLite).
        let _ = self.aux_shutdown.send(true);
        self.wake_all();
        for handle in std::mem::take(&mut self.aux_tasks) {
            let _ = handle.await;
        }
    }
}

impl Drop for SyncHandle {
    fn drop(&mut self) {
        let _ = self.router_shutdown.send(true);
        let _ = self.aux_shutdown.send(true);
        self.wake_all();
    }
}

/// Inicia el subsistema de sincronización en el runtime de Tokio actual.
///
/// Orden: `release_inflight` → lease → `Recovery` → router → trabajadores → agregador → housekeeping.
pub fn spawn_sync(deps: SyncDeps) -> SyncHandle {
    let _ = &deps.transport;
    if let Err(error) = deps.store.release_inflight() {
        tracing::warn!(
            failure_class = error.diagnostic_class(),
            "sync runtime: release_inflight failed"
        );
    }

    let (router_shutdown_tx, router_shutdown_rx) = watch::channel(false);
    let (aux_shutdown_tx, aux_shutdown_rx) = watch::channel(false);
    let lease = Lease::new(deps.process_kind);
    let lease_held = lease.held.clone();
    let _ = lease.try_step(&deps.store, deps.clock.as_ref());

    let cfg = RouterConfig {
        journal_dir: deps.journal_dir,
        env: deps.env,
        live_signal: deps.live_signal,
        flush_every: Duration::from_millis(250),
        flush_rows: 200,
        hold_before_process: (200, Duration::from_secs(10)),
    };
    let mut router = Router::new(
        deps.store.clone(),
        deps.clock.clone(),
        deps.online,
        lease_held,
        cfg,
        deps.mappers,
    );
    if let Err(error) = router.recover_previous_session() {
        tracing::warn!(
            failure_class = error.diagnostic_class(),
            "sync runtime: initial recovery failed"
        );
    }

    let gates = router.send_gates();
    let current_fid: Arc<Mutex<Option<String>>> =
        Arc::new(Mutex::new(router.tracker().snapshot().cmdr_fid.clone()));
    let (feed_tx, mut feed_rx) = mpsc::unbounded_channel();
    let (worker_events_tx, mut worker_events_rx) = mpsc::channel::<WorkerEvent>(128);
    let status_wake = Arc::new(Notify::new());

    let mut wakers = Vec::with_capacity(deps.clients.len());
    let mut router_and_workers = Vec::new();
    let mut aux_tasks = Vec::new();

    for client in deps.clients {
        let wake = Arc::new(Notify::new());
        wakers.push(wake.clone());
        let worker = Worker::new(
            client,
            deps.store.clone(),
            deps.clock.clone(),
            gates.clone(),
            wake,
            worker_events_tx.clone(),
        );
        router_and_workers.push(tokio::spawn(worker.run(router_shutdown_rx.clone())));
    }
    drop(worker_events_tx);
    router.set_enqueued_wakers(wakers.clone());

    diagnostic!(
        DiagnosticContext::Sync,
        "sync.runtime.started",
        json!({
            "decision": if deps.env.e2e_simulation { "e2e_disabled" } else { "started" },
            "process_kind": deps.process_kind,
        })
    );

    aux_tasks.push(tokio::spawn(lease.run(
        deps.store.clone(),
        deps.clock.clone(),
        aux_shutdown_rx.clone(),
        status_wake.clone(),
    )));

    let router_status_wake = status_wake.clone();
    let router_fid = current_fid.clone();
    let mut router_shutdown = router_shutdown_rx;
    router_and_workers.push(tokio::spawn(async move {
        loop {
            if *router_shutdown.borrow() {
                break;
            }
            let deadline = router.next_deadline();
            tokio::select! {
                biased;
                changed = router_shutdown.changed() => {
                    if changed.is_err() || *router_shutdown.borrow() {
                        break;
                    }
                }
                maybe_feed = feed_rx.recv() => {
                    match maybe_feed {
                        Some(feed) => {
                            if let Err(error) = router.handle(feed) {
                                tracing::warn!(
                                    failure_class = error.diagnostic_class(),
                                    "sync router: handle failed"
                                );
                            }
                            while let Ok(more) = feed_rx.try_recv() {
                                if let Err(error) = router.handle(more) {
                                    tracing::warn!(
                                        failure_class = error.diagnostic_class(),
                                        "sync router: handle failed"
                                    );
                                }
                            }
                            *router_fid.lock().unwrap() = router.tracker().snapshot().cmdr_fid.clone();
                            router_status_wake.notify_one();
                        }
                        None => break,
                    }
                }
                _ = sleep_until_opt(deadline), if deadline.is_some() => {
                    if let Err(error) = router.tick() {
                        tracing::warn!(
                            failure_class = error.diagnostic_class(),
                            "sync router: tick failed"
                        );
                    }
                    *router_fid.lock().unwrap() = router.tracker().snapshot().cmdr_fid.clone();
                    router_status_wake.notify_one();
                }
            }
        }
        while let Ok(feed) = feed_rx.try_recv() {
            if let Err(error) = router.handle(feed) {
                tracing::warn!(
                    failure_class = error.diagnostic_class(),
                    "sync router: shutdown handle failed"
                );
            }
        }
        if let Err(error) = router.flush() {
            tracing::warn!(
                failure_class = error.diagnostic_class(),
                "sync router: shutdown flush failed"
            );
        }
        *router_fid.lock().unwrap() = router.tracker().snapshot().cmdr_fid.clone();
        router_status_wake.notify_one();
    }));

    let salt = deps.store.get_or_create_eddn_salt().unwrap_or_default();
    let status_store = deps.store.clone();
    let status_clock = deps.clock.clone();
    let core_events = deps.core_events;
    let status_notify = status_wake.clone();
    let mut agg_shutdown = aux_shutdown_rx.clone();
    aux_tasks.push(tokio::spawn(async move {
        let mut agg = StatusAggregator::new(status_store.clone(), status_clock, salt);
        let mut dirty = true;
        let mut retry_at: Option<Instant> = None;
        let mut worker_events_open = true;

        loop {
            if *agg_shutdown.borrow() {
                break;
            }
            if dirty {
                let fid = current_fid.lock().unwrap().clone();
                let reports = agg.build(fid.as_deref(), &|svc, f| {
                    blocked_with(&gates, &status_store, svc, f)
                });
                let now = Instant::now();
                let (to_send, throttled_wait) = agg.changed_reports_with_pending(reports, now);
                for report in to_send {
                    tokio::select! {
                        changed = agg_shutdown.changed() => {
                            if changed.is_err() || *agg_shutdown.borrow() {
                                return;
                            }
                        }
                        res = core_events.send(SourceEvent::SyncReport(Box::new(report))) => {
                            if res.is_err() {
                                return;
                            }
                        }
                    }
                }
                dirty = false;
                retry_at = throttled_wait.map(|delay| now + delay);
            }

            tokio::select! {
                changed = agg_shutdown.changed() => {
                    if changed.is_err() || *agg_shutdown.borrow() {
                        break;
                    }
                }
                ev = worker_events_rx.recv(), if worker_events_open => {
                    if let Some(event) = ev {
                        agg.on_worker_event(&event);
                        dirty = true;
                    } else {
                        worker_events_open = false;
                    }
                }
                _ = status_notify.notified() => {
                    dirty = true;
                }
                _ = sleep_until_opt(retry_at), if retry_at.is_some() => {
                    retry_at = None;
                    dirty = true;
                }
            }
        }
    }));

    aux_tasks.push(tokio::spawn(run_housekeeping(
        deps.store,
        deps.clock,
        aux_shutdown_rx,
    )));

    SyncHandle {
        feed: feed_tx,
        wakers,
        status_wake,
        router_shutdown: router_shutdown_tx,
        aux_shutdown: aux_shutdown_tx,
        router_and_workers,
        aux_tasks,
    }
}

async fn sleep_until_opt(deadline: Option<Instant>) {
    match deadline {
        Some(at) => tokio::time::sleep_until(at).await,
        None => std::future::pending().await,
    }
}
