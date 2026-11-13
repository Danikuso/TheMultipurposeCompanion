//! Tests de arranque seguro, reintentos y backoff del trabajador genérico (C3, E0-11).

use crate::clock::{Clock, TestClock};
use crate::router::SendGates;
use crate::worker::{
    BatchPolicy, BoxFut, MAX_WORKER_SLEEP, RowVerdict, SendCtx, SendOutcome, ServiceClient,
    ServiceEffect, Worker, WorkerEvent, backoff,
};
use chrono::{TimeZone, Utc};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tmc_model::sync::{ErrorClass, KeyAssignment, OutboxState, RowOrigin, SyncService};
use tmc_net::Transport;
use tmc_net::limiter::{RateLimiter, Service};
use tmc_net::testing::ScriptedTransport;
use tmc_net::transport::{HttpPost, HttpReplyEx};
use tmc_store::{NewOutboxRow, OutboxRow, Store, SyncCredential};
use tokio::sync::{Notify, mpsc, watch};

const F1: &str = "F0000001";

#[derive(Clone)]
enum ScriptAction {
    Verdict {
        verdict: RowVerdict,
        effect: ServiceEffect,
        before_reply: Option<Arc<dyn Fn() + Send + Sync>>,
    },
    AbortAfterHttp,
}

struct TestClient {
    service: SyncService,
    needs_cred: bool,
    per_cmdr: bool,
    max_rows: usize,
    transport: Arc<ScriptedTransport>,
    actions: Mutex<VecDeque<ScriptAction>>,
}

impl TestClient {
    fn new(service: SyncService, transport: Arc<ScriptedTransport>) -> Self {
        Self {
            service,
            needs_cred: false,
            per_cmdr: false,
            max_rows: 10,
            transport,
            actions: Mutex::new(VecDeque::new()),
        }
    }

    fn push(&self, action: ScriptAction) {
        self.actions.lock().unwrap().push_back(action);
    }
}

impl ServiceClient for TestClient {
    fn service(&self) -> SyncService {
        self.service
    }

    fn policy(&self) -> BatchPolicy {
        BatchPolicy {
            max_rows: self.max_rows,
            per_cmdr: self.per_cmdr,
            min_interval: Duration::ZERO,
            limiter: RateLimiter::for_service(Service::Edsm),
            flush_after: None,
        }
    }

    fn needs_credential(&self) -> bool {
        self.needs_cred
    }

    fn send<'a>(&'a self, _ctx: SendCtx<'a>, rows: &'a [OutboxRow]) -> BoxFut<'a, SendOutcome> {
        Box::pin(async move {
            let action = self
                .actions
                .lock()
                .unwrap()
                .pop_front()
                .expect("acción de test disponible");
            match action {
                ScriptAction::Verdict {
                    verdict,
                    effect,
                    before_reply,
                } => {
                    if let Some(hook) = before_reply {
                        hook();
                    }
                    let _ = self
                        .transport
                        .post(HttpPost {
                            url: "https://mock.test/submit",
                            body: &[],
                            content_type: "application/json",
                            headers: &[],
                            http1_only: false,
                        })
                        .await;
                    let verdicts = rows.iter().map(|r| (r.id, verdict.clone())).collect();
                    SendOutcome { verdicts, effect }
                }
                ScriptAction::AbortAfterHttp => {
                    let _ = self
                        .transport
                        .post(HttpPost {
                            url: "https://mock.test/submit",
                            body: &[],
                            content_type: "application/json",
                            headers: &[],
                            http1_only: false,
                        })
                        .await;
                    // Simula corte abrupto tras recibir el 200 pero antes de confirmar en BD:
                    // suspende eternamente hasta cancelación del task.
                    std::future::pending::<SendOutcome>().await
                }
            }
        })
    }
}

fn store_with_rows(count: usize, age_offset_secs: i64) -> (Store, Arc<TestClock>) {
    let store = Store::open_in_memory().unwrap();
    let base = Utc.with_ymd_and_hms(2026, 3, 9, 10, 0, 0).unwrap();
    let clock = TestClock::new(base);
    let ts = base + chrono::Duration::seconds(age_offset_secs);
    let rows: Vec<NewOutboxRow> = (0..count)
        .map(|i| NewOutboxRow {
            service: SyncService::Edsm,
            cmdr_fid: F1.into(),
            idem_key: format!("k-{i}"),
            kind: "FSDJump".into(),
            coalesce_key: None,
            event_ts: ts + chrono::Duration::seconds(i as i64),
            payload: format!(r#"{{"i":{i}}}"#),
            origin: RowOrigin::Live,
            state: OutboxState::Pending,
            last_class: None,
        })
        .collect();
    store.enqueue_batch_with_cursor(&rows, &[], base).unwrap();
    (store, clock)
}

fn ok_replies(n: usize) -> Arc<ScriptedTransport> {
    let replies = (0..n)
        .map(|_| {
            Ok(HttpReplyEx {
                status: 200,
                body: "OK".into(),
                retry_after_s: None,
                rate_limit_remaining: None,
                rate_limit_reset_s: None,
            })
        })
        .collect();
    ScriptedTransport::new(replies)
}

#[tokio::test(start_paused = true)]
async fn inflight_rows_return_to_pending_on_restart() {
    let (store, clock) = store_with_rows(2, 0);
    let claimed = store
        .claim_ready(SyncService::Edsm, None, 10, clock.now_utc())
        .unwrap();
    assert_eq!(claimed.len(), 2);
    // Simula reinicio tras cierre abrupto con 2 filas inflight.
    assert_eq!(store.release_inflight().unwrap(), 2);

    let transport = ok_replies(1);
    let client = Arc::new(TestClient::new(SyncService::Edsm, transport.clone()));
    client.push(ScriptAction::Verdict {
        verdict: RowVerdict::Sent { code: "200".into() },
        effect: ServiceEffect::Connected,
        before_reply: None,
    });

    let gates = Arc::new(SendGates::new(
        Arc::new(AtomicBool::new(true)),
        Arc::new(AtomicBool::new(true)),
    ));
    let wake = Arc::new(Notify::new());
    let (ev_tx, mut ev_rx) = mpsc::channel::<WorkerEvent>(16);
    let (shut_tx, shut_rx) = watch::channel(false);
    let worker = Worker::new(client, store.clone(), clock, gates, wake, ev_tx);
    let task = tokio::spawn(worker.run(shut_rx));

    let ev = ev_rx.recv().await.unwrap();
    match ev {
        WorkerEvent::Outcome { sent, .. } => assert_eq!(sent, 2),
        _ => panic!("se esperaba Outcome"),
    }
    assert_eq!(transport.calls(), 1);
    assert_eq!(
        store.sync_counts(SyncService::Edsm, F1).unwrap().sent_total,
        2
    );

    let _ = shut_tx.send(true);
    task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn reply_received_but_not_marked_is_resent_once() {
    let (store, clock) = store_with_rows(1, 0);
    let transport = ok_replies(2);
    let gates = Arc::new(SendGates::new(
        Arc::new(AtomicBool::new(true)),
        Arc::new(AtomicBool::new(true)),
    ));

    // Ejecución 1: llega respuesta HTTP 200 pero el proceso muere antes de aplicar veredictos.
    let client1 = Arc::new(TestClient::new(SyncService::Edsm, transport.clone()));
    client1.push(ScriptAction::AbortAfterHttp);
    let (ev_tx1, _ev_rx1) = mpsc::channel(8);
    let (_shut_tx1, shut_rx1) = watch::channel(false);
    let worker1 = Worker::new(
        client1,
        store.clone(),
        clock.clone(),
        gates.clone(),
        Arc::new(Notify::new()),
        ev_tx1,
    );
    let task1 = tokio::spawn(worker1.run(shut_rx1));
    tokio::task::yield_now().await;
    assert_eq!(transport.calls(), 1);
    task1.abort();
    let _ = task1.await;
    assert_eq!(
        store.sync_counts(SyncService::Edsm, F1).unwrap().sent_total,
        0
    );

    // Reapertura: release_inflight devuelve a pending y reenvía exactamente una vez.
    assert_eq!(store.release_inflight().unwrap(), 1);
    let client2 = Arc::new(TestClient::new(SyncService::Edsm, transport.clone()));
    client2.push(ScriptAction::Verdict {
        verdict: RowVerdict::Sent { code: "200".into() },
        effect: ServiceEffect::Connected,
        before_reply: None,
    });
    let (ev_tx2, mut ev_rx2) = mpsc::channel(8);
    let (shut_tx2, shut_rx2) = watch::channel(false);
    let worker2 = Worker::new(
        client2,
        store.clone(),
        clock,
        gates,
        Arc::new(Notify::new()),
        ev_tx2,
    );
    let task2 = tokio::spawn(worker2.run(shut_rx2));
    let _ = ev_rx2.recv().await.unwrap();
    tokio::time::advance(Duration::from_secs(60)).await;
    assert_eq!(transport.calls(), 2);
    assert_eq!(
        store.sync_counts(SyncService::Edsm, F1).unwrap().sent_total,
        1
    );

    let _ = shut_tx2.send(true);
    task2.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn retry_respects_next_attempt_at() {
    let (store, clock) = store_with_rows(1, 0);
    let transport = ok_replies(2);
    let client = Arc::new(TestClient::new(SyncService::Edsm, transport.clone()));
    client.push(ScriptAction::Verdict {
        verdict: RowVerdict::Retry {
            code: "503".into(),
            class: ErrorClass::ServerError,
            after: Some(Duration::from_secs(60)),
        },
        effect: ServiceEffect::ServiceError {
            class: ErrorClass::ServerError,
            until: None,
        },
        before_reply: None,
    });
    client.push(ScriptAction::Verdict {
        verdict: RowVerdict::Sent { code: "200".into() },
        effect: ServiceEffect::Connected,
        before_reply: None,
    });

    let gates = Arc::new(SendGates::new(
        Arc::new(AtomicBool::new(true)),
        Arc::new(AtomicBool::new(true)),
    ));
    let (ev_tx, mut ev_rx) = mpsc::channel(8);
    let (shut_tx, shut_rx) = watch::channel(false);
    let worker = Worker::new(
        client,
        store.clone(),
        clock,
        gates,
        Arc::new(Notify::new()),
        ev_tx,
    );
    let task = tokio::spawn(worker.run(shut_rx));
    let _ = ev_rx.recv().await.unwrap();
    assert_eq!(transport.calls(), 1);

    // +59 s: todavía no debe reintentar.
    tokio::time::advance(Duration::from_secs(59)).await;
    tokio::task::yield_now().await;
    assert_eq!(transport.calls(), 1);

    // +1 s (total 60 s): se ejecuta el segundo intento.
    tokio::time::advance(Duration::from_secs(1)).await;
    let _ = ev_rx.recv().await.unwrap();
    assert_eq!(transport.calls(), 2);
    assert_eq!(
        store.sync_counts(SyncService::Edsm, F1).unwrap().sent_total,
        1
    );

    let _ = shut_tx.send(true);
    task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn hold_keeps_rows_pending_until_wake() {
    let (store, clock) = store_with_rows(1, 0);
    store
        .put_sync_credential(&SyncCredential {
            service: SyncService::Edsm,
            cmdr_fid: F1.into(),
            cmdr_name: "CMDR TESTER".into(),
            api_key: "OLD-KEY".into(),
            created_at: "2026-03-09T09:00:00Z".into(),
            verified_at: None,
            assignment: KeyAssignment::Confirmed,
        })
        .unwrap();

    let transport = ok_replies(2);
    let mut client = TestClient::new(SyncService::Edsm, transport.clone());
    client.needs_cred = true;
    client.per_cmdr = true;
    let client = Arc::new(client);
    client.push(ScriptAction::Verdict {
        verdict: RowVerdict::Hold {
            code: "401".into(),
            class: ErrorClass::Auth,
        },
        effect: ServiceEffect::CmdrError(ErrorClass::Auth),
        before_reply: None,
    });
    client.push(ScriptAction::Verdict {
        verdict: RowVerdict::Sent { code: "200".into() },
        effect: ServiceEffect::Connected,
        before_reply: None,
    });

    let gates = Arc::new(SendGates::new(
        Arc::new(AtomicBool::new(true)),
        Arc::new(AtomicBool::new(true)),
    ));
    let wake = Arc::new(Notify::new());
    let (ev_tx, mut ev_rx) = mpsc::channel(8);
    let (shut_tx, shut_rx) = watch::channel(false);
    let worker = Worker::new(client, store.clone(), clock, gates, wake.clone(), ev_tx);
    let task = tokio::spawn(worker.run(shut_rx));

    let _ = ev_rx.recv().await.unwrap();
    assert_eq!(transport.calls(), 1);
    assert_eq!(store.sync_counts(SyncService::Edsm, F1).unwrap().pending, 1);

    // 1 h virtual en espera por Hold(auth): 0 reintentos automáticos.
    tokio::time::advance(Duration::from_secs(3600)).await;
    tokio::task::yield_now().await;
    assert_eq!(transport.calls(), 1);

    // El piloto corrige la clave y despierta al trabajador → envía.
    store
        .put_sync_credential(&SyncCredential {
            service: SyncService::Edsm,
            cmdr_fid: F1.into(),
            cmdr_name: "CMDR TESTER".into(),
            api_key: "NEW-KEY".into(),
            created_at: "2026-03-09T11:00:00Z".into(),
            verified_at: None,
            assignment: KeyAssignment::Confirmed,
        })
        .unwrap();
    wake.notify_one();
    let _ = ev_rx.recv().await.unwrap();
    assert_eq!(transport.calls(), 2);
    assert_eq!(
        store.sync_counts(SyncService::Edsm, F1).unwrap().sent_total,
        1
    );

    let _ = shut_tx.send(true);
    task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn clock_jump_backwards_never_sleeps_beyond_max_wait() {
    let (store, clock) = store_with_rows(1, 0);
    let claimed = store
        .claim_ready(SyncService::Edsm, None, 1, clock.now_utc())
        .unwrap();
    let future = clock.now_utc() + chrono::Duration::minutes(10);
    store
        .schedule_retry(&[claimed[0].id], future, "503", ErrorClass::ServerError)
        .unwrap();

    // Salto de reloj hacia atrás de 2 h: `next_attempt_at - now` sería 2 h 10 min.
    clock.shift(chrono::Duration::hours(-2));

    let transport = ok_replies(1);
    let client = Arc::new(TestClient::new(SyncService::Edsm, transport.clone()));
    let gates = Arc::new(SendGates::new(
        Arc::new(AtomicBool::new(true)),
        Arc::new(AtomicBool::new(true)),
    ));
    let (ev_tx, _ev_rx) = mpsc::channel(8);
    let (shut_tx, shut_rx) = watch::channel(false);
    let worker = Worker::new(client, store, clock, gates, Arc::new(Notify::new()), ev_tx);
    let iters = worker.iterations();
    let task = tokio::spawn(worker.run(shut_rx));
    tokio::task::yield_now().await;
    let first = iters.load(Ordering::SeqCst);
    assert_eq!(first, 1);

    // Justo antes de MAX_WORKER_SLEEP (30 min) sigue dormido; a los 30 min despierta a reevaluar.
    tokio::time::advance(MAX_WORKER_SLEEP - Duration::from_secs(1)).await;
    tokio::task::yield_now().await;
    assert_eq!(iters.load(Ordering::SeqCst), 1);

    tokio::time::advance(Duration::from_secs(1)).await;
    tokio::task::yield_now().await;
    assert!(iters.load(Ordering::SeqCst) >= 2);
    assert_eq!(transport.calls(), 0);

    let _ = shut_tx.send(true);
    task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn closed_gates_send_nothing() {
    let (store, clock) = store_with_rows(1, 0);
    let transport = ok_replies(1);
    let client = Arc::new(TestClient::new(SyncService::Edsm, transport.clone()));
    let gates = Arc::new(SendGates::new(
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(true)),
    ));
    let (ev_tx, _ev_rx) = mpsc::channel(8);
    let (shut_tx, shut_rx) = watch::channel(false);
    let worker = Worker::new(
        client,
        store.clone(),
        clock,
        gates,
        Arc::new(Notify::new()),
        ev_tx,
    );
    let task = tokio::spawn(worker.run(shut_rx));
    tokio::time::advance(Duration::from_secs(600)).await;
    tokio::task::yield_now().await;

    assert_eq!(transport.calls(), 0);
    assert_eq!(store.sync_counts(SyncService::Edsm, F1).unwrap().pending, 1);

    let _ = shut_tx.send(true);
    task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn online_off_mid_batch_keeps_rows_pending() {
    let (store, clock) = store_with_rows(1, 0);
    let transport = ok_replies(2);
    let online = Arc::new(AtomicBool::new(true));
    let gates = Arc::new(SendGates::new(
        online.clone(),
        Arc::new(AtomicBool::new(true)),
    ));
    let wake = Arc::new(Notify::new());

    let client = Arc::new(TestClient::new(SyncService::Edsm, transport.clone()));
    let online_hook = online.clone();
    client.push(ScriptAction::Verdict {
        verdict: RowVerdict::Sent { code: "200".into() },
        effect: ServiceEffect::Connected,
        before_reply: Some(Arc::new(move || {
            online_hook.store(false, Ordering::SeqCst);
        })),
    });
    client.push(ScriptAction::Verdict {
        verdict: RowVerdict::Sent { code: "200".into() },
        effect: ServiceEffect::Connected,
        before_reply: None,
    });

    let (ev_tx, mut ev_rx) = mpsc::channel(8);
    let (shut_tx, shut_rx) = watch::channel(false);
    let worker = Worker::new(client, store.clone(), clock, gates, wake.clone(), ev_tx);
    let task = tokio::spawn(worker.run(shut_rx));
    tokio::task::yield_now().await;

    let counts = store.sync_counts(SyncService::Edsm, F1).unwrap();
    assert_eq!(counts.sent_total, 0);
    assert_eq!(counts.pending, 1);
    assert!(ev_rx.try_recv().is_err());

    // Al volver en línea y despertar, la fila se envía con normalidad.
    online.store(true, Ordering::SeqCst);
    wake.notify_one();
    let _ = ev_rx.recv().await.unwrap();
    assert_eq!(
        store.sync_counts(SyncService::Edsm, F1).unwrap().sent_total,
        1
    );

    let _ = shut_tx.send(true);
    task.await.unwrap();
}

#[test]
fn backoff_values() {
    for service in SyncService::ALL {
        assert_eq!(backoff(service, 1), Duration::from_secs(60));
        assert_eq!(backoff(service, 2), Duration::from_secs(120));
        assert_eq!(backoff(service, 7), Duration::from_secs(1800));
        assert_eq!(backoff(service, 12), Duration::from_secs(1800));
    }
}
