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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
#[cfg(debug_assertions)]
use tmc_model::diagnostic;
#[cfg(debug_assertions)]
use tmc_model::diagnostics::{DiagnosticContext, json};
use tmc_model::source::SourceEvent;
use tmc_model::sync::{SyncFeed, SyncService};
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
    verify_wake: Arc<Notify>,
    router_wake: Arc<Notify>,
    verify_tx: Option<mpsc::Sender<crate::verify::VerifyUpdate>>,
}

impl SyncWaker {
    pub fn wake_all(&self) {
        for wake in &self.wakers {
            wake.notify_one();
        }
        self.verify_wake.notify_one();
        self.router_wake.notify_one();
        self.status_wake.notify_one();
    }

    pub fn wake_verify(&self) {
        self.verify_wake.notify_one();
        self.status_wake.notify_one();
    }

    pub fn report_verify(&self, update: crate::verify::VerifyUpdate) {
        if let Some(tx) = &self.verify_tx {
            let _ = tx.try_send(update);
        }
        self.verify_wake.notify_one();
        self.status_wake.notify_one();
    }
}

/// Control en ejecución de la sincronización; propietario del feed y del cierre ordenado.
pub struct SyncHandle {
    pub feed: mpsc::UnboundedSender<SyncFeed>,
    wakers: Vec<Arc<Notify>>,
    status_wake: Arc<Notify>,
    verify_wake: Arc<Notify>,
    router_wake: Arc<Notify>,
    verify_tx: mpsc::Sender<crate::verify::VerifyUpdate>,
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
        self.verify_wake.notify_one();
        self.status_wake.notify_one();
    }

    pub fn client_wake(&self, index: usize) -> Option<Arc<Notify>> {
        self.wakers.get(index).cloned()
    }

    pub fn spawn_aux<F, Fut>(&mut self, build: F)
    where
        F: FnOnce(watch::Receiver<bool>) -> Fut,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let rx = self.aux_shutdown.subscribe();
        self.aux_tasks.push(tokio::spawn(build(rx)));
    }

    pub fn waker(&self) -> SyncWaker {
        SyncWaker {
            wakers: self.wakers.clone(),
            status_wake: self.status_wake.clone(),
            verify_wake: self.verify_wake.clone(),
            router_wake: self.router_wake.clone(),
            verify_tx: Some(self.verify_tx.clone()),
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
    let verify_transport = deps.transport.clone();
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

    let verify_online = deps.online.clone();
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
    let verify_wake = Arc::new(Notify::new());
    let router_wake = Arc::new(Notify::new());
    let (verify_tx, mut verify_rx) = mpsc::channel::<crate::verify::VerifyUpdate>(64);

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
    let recovered_totals = Arc::new([AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)]);
    router.set_recovered_totals(recovered_totals.clone());

    #[cfg(debug_assertions)]
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
    aux_tasks.push(tokio::spawn(crate::verify::run_key_verifier(
        deps.store.clone(),
        verify_transport,
        deps.clock.clone(),
        verify_online,
        verify_wake.clone(),
        wakers.clone(),
        status_wake.clone(),
        verify_tx.clone(),
        aux_shutdown_rx.clone(),
    )));

    let router_status_wake = status_wake.clone();
    let router_wake_rx = router_wake.clone();
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
                _ = router_wake_rx.notified() => {
                    if let Err(error) = router.tick() {
                        tracing::warn!(
                            failure_class = error.diagnostic_class(),
                            "sync router: tick failed"
                        );
                    }
                    if let Err(error) = router.flush() {
                        tracing::warn!(
                            failure_class = error.diagnostic_class(),
                            "sync router: wake flush failed"
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
        type VerifyErrorMap = std::collections::HashMap<
            (SyncService, String),
            (
                tmc_model::sync::ErrorClass,
                Option<chrono::DateTime<chrono::Utc>>,
            ),
        >;
        let mut verify_errors: VerifyErrorMap = std::collections::HashMap::new();
        let mut dirty = true;
        let mut retry_at: Option<Instant> = None;
        let mut worker_events_open = true;
        let mut verify_open = true;

        loop {
            if *agg_shutdown.borrow() {
                break;
            }
            for (svc, slot) in [
                (SyncService::Inara, &recovered_totals[0]),
                (SyncService::Edsm, &recovered_totals[1]),
                (SyncService::Eddn, &recovered_totals[2]),
            ] {
                let delta = slot.swap(0, Ordering::AcqRel);
                if delta > 0 {
                    agg.on_recovered(svc, delta);
                    dirty = true;
                }
            }
            if dirty {
                let fid = current_fid.lock().unwrap().clone();
                let mut reports = agg.build(fid.as_deref(), &|svc, f| {
                    blocked_with(&gates, &status_store, svc, f)
                });
                if let Some(active_fid) = fid.as_deref() {
                    for report in &mut reports {
                        let (svc, keyed) = match report {
                            tmc_model::sync::SyncReport::Inara(k) => (SyncService::Inara, k),
                            tmc_model::sync::SyncReport::Edsm(k) => (SyncService::Edsm, k),
                            _ => continue,
                        };
                        if keyed.has_key
                            && let Some((class, until)) =
                                verify_errors.get(&(svc, active_fid.to_string()))
                            && !matches!(
                                keyed.status.reason,
                                Some(
                                    tmc_model::sync::SyncReason::OnlineOff
                                        | tmc_model::sync::SyncReason::Legacy
                                        | tmc_model::sync::SyncReason::Beta
                                        | tmc_model::sync::SyncReason::Replay
                                        | tmc_model::sync::SyncReason::OtherInstance
                                        | tmc_model::sync::SyncReason::PlatformUnsupported
                                        | tmc_model::sync::SyncReason::KeyUnassigned
                                )
                            )
                        {
                            keyed.status.state = tmc_model::sync::SyncState::Error;
                            keyed.status.reason = Some(match class {
                                tmc_model::sync::ErrorClass::Auth => {
                                    tmc_model::sync::SyncReason::Auth
                                }
                                _ => tmc_model::sync::SyncReason::Network,
                            });
                            keyed.status.next_attempt_at = until
                                .map(|ts| ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
                        }
                    }
                }
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
                upd = verify_rx.recv(), if verify_open => {
                    match upd {
                        Some(u) => {
                            match u.check {
                                Some(tmc_net::KeyCheck::Unreachable) => {
                                    verify_errors.insert(
                                        (u.service, u.fid),
                                        (tmc_model::sync::ErrorClass::Network, u.next_retry),
                                    );
                                }
                                Some(tmc_net::KeyCheck::Rejected) => {
                                    verify_errors.insert(
                                        (u.service, u.fid),
                                        (tmc_model::sync::ErrorClass::Auth, None),
                                    );
                                }
                                Some(tmc_net::KeyCheck::Valid) | None => {
                                    verify_errors.remove(&(u.service, u.fid));
                                }
                            }
                            dirty = true;
                        }
                        None => verify_open = false,
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
        verify_wake,
        router_wake,
        verify_tx,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::TestClock;
    use crate::router::{EDDN_ENABLED_KEY, MappedInput, MappedRow, ServiceMapper};
    use chrono::TimeZone;
    use tmc_model::events::{EventKind, parse_line};
    use tmc_model::state::Galaxy;
    use tmc_model::sync::{FeedOrigin, OutboxState, SyncFeedEvent, SyncReport, SyncService};
    use tmc_net::testing::NoNetworkTransport;

    struct DummyEddnMapper;
    impl ServiceMapper for DummyEddnMapper {
        fn service(&self) -> SyncService {
            SyncService::Eddn
        }
        fn map(&mut self, input: &MappedInput<'_>, out: &mut Vec<MappedRow>) {
            if matches!(input.ev.kind, EventKind::FsdJump(_)) {
                out.push(MappedRow {
                    sub: 0,
                    kind: "journal/1".into(),
                    coalesce_key: None,
                    payload: serde_json::json!({"event": "FSDJump"}),
                    state: OutboxState::Pending,
                    class: None,
                });
            }
        }
    }

    #[tokio::test]
    async fn recovered_session_is_reported_via_status_aggregator() {
        let dir =
            std::env::temp_dir().join(format!("tmc-sync-recovered-runtime-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = "Journal.2026-11-10T100000.01.log";
        std::fs::write(
            dir.join(file),
            "{\"timestamp\":\"2026-11-10T10:00:00Z\",\"event\":\"Fileheader\",\"part\":1,\"language\":\"English/UK\",\"gameversion\":\"4.0.0.1903\",\"build\":\"r310123/r0 \"}\n",
        )
        .unwrap();

        let store = Store::open_in_memory().unwrap();
        let now = chrono::Utc
            .with_ymd_and_hms(2026, 11, 10, 10, 5, 0)
            .unwrap();
        let enabled_at = (now - chrono::Duration::hours(1)).to_rfc3339();
        store.kv_set(EDDN_ENABLED_KEY, &enabled_at);

        let (core_tx, mut core_rx) = mpsc::channel(32);
        let handle = spawn_sync(SyncDeps {
            store,
            transport: Arc::new(NoNetworkTransport::default()),
            online: Arc::new(AtomicBool::new(true)),
            core_events: core_tx,
            journal_dir: dir.clone(),
            env: EnvGates::default(),
            live_signal: LiveSignalPolicy::JournalActivity,
            clock: TestClock::new(now),
            process_kind: "desktop",
            clients: Vec::new(),
            mappers: vec![Box::new(DummyEddnMapper)],
        });

        let load = parse_line(
            r#"{"timestamp":"2026-11-10T10:01:00Z","event":"LoadGame","FID":"F1000001","Commander":"CMDR REC","Horizons":true,"Odyssey":true,"gameversion":"4.0.0.1903","build":"r310123/r0 "}"#,
        )
        .unwrap();
        let jump = parse_line(
            r#"{"timestamp":"2026-11-10T10:02:00Z","event":"FSDJump","StarSystem":"Sol","SystemAddress":10477373803,"StarPos":[0.0,0.0,0.0]}"#,
        )
        .unwrap();

        for (line, ev) in [(2u64, load), (3u64, jump)] {
            handle
                .feed
                .send(SyncFeed {
                    event: SyncFeedEvent::Journal {
                        ev,
                        file: file.to_string(),
                        line,
                    },
                    origin: FeedOrigin::CatchUp,
                    galaxy: Galaxy::Live,
                    game_running: true,
                })
                .unwrap();
        }
        handle
            .feed
            .send(SyncFeed {
                event: SyncFeedEvent::CatchUpDone,
                origin: FeedOrigin::CatchUp,
                galaxy: Galaxy::Live,
                game_running: true,
            })
            .unwrap();

        let mut seen = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout_at(deadline, core_rx.recv()).await {
                Ok(Some(SourceEvent::SyncReport(rep))) => {
                    if let SyncReport::Eddn(st) = *rep
                        && st.status.recovered_session == 1
                    {
                        seen = true;
                        break;
                    }
                }
                Ok(Some(_)) => {}
                Ok(None) | Err(_) => break,
            }
        }
        handle.shutdown().await;
        let _ = std::fs::remove_dir_all(&dir);
        assert!(seen, "expected Eddn SyncReport with recovered_session == 1");
    }
}
