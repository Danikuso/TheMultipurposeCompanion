//! Tests de cero consumo en reposo cuando la cola está vacía (C11, E0-11).

use crate::clock::TestClock;
use crate::router::SendGates;
use crate::worker::{
    BatchPolicy, BoxFut, SendCtx, SendOutcome, ServiceClient, ServiceEffect, Worker,
};
use chrono::{TimeZone, Utc};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tmc_model::sync::SyncService;
use tmc_net::limiter::{RateLimiter, Service};
use tmc_store::{OutboxRow, Store};
use tokio::sync::{Notify, mpsc, watch};

struct DummyClient;

impl ServiceClient for DummyClient {
    fn service(&self) -> SyncService {
        SyncService::Eddn
    }

    fn policy(&self) -> BatchPolicy {
        BatchPolicy {
            max_rows: 1,
            per_cmdr: false,
            min_interval: Duration::ZERO,
            limiter: RateLimiter::for_service(Service::Eddn),
            flush_after: None,
        }
    }

    fn needs_credential(&self) -> bool {
        false
    }

    fn send<'a>(&'a self, _ctx: SendCtx<'a>, _rows: &'a [OutboxRow]) -> BoxFut<'a, SendOutcome> {
        Box::pin(async {
            SendOutcome {
                verdicts: vec![],
                effect: ServiceEffect::None,
            }
        })
    }
}

#[tokio::test(start_paused = true)]
async fn workers_have_zero_wakeups_with_empty_queue() {
    let store = Store::open_in_memory().unwrap();
    let clock = TestClock::new(Utc.with_ymd_and_hms(2026, 3, 9, 10, 0, 0).unwrap());
    let gates = Arc::new(SendGates::new(
        Arc::new(AtomicBool::new(true)),
        Arc::new(AtomicBool::new(true)),
    ));
    let (ev_tx, _ev_rx) = mpsc::channel(8);
    let (shut_tx, shut_rx) = watch::channel(false);

    let worker = Worker::new(
        Arc::new(DummyClient),
        store,
        clock,
        gates,
        Arc::new(Notify::new()),
        ev_tx,
    );
    let iters = worker.iterations();
    let task = tokio::spawn(worker.run(shut_rx));
    tokio::task::yield_now().await;
    assert_eq!(iters.load(Ordering::SeqCst), 1);

    // 60 s sin eventos con cola vacía: ningún wakeup adicional.
    tokio::time::advance(Duration::from_secs(60)).await;
    tokio::task::yield_now().await;
    assert_eq!(iters.load(Ordering::SeqCst), 1);

    let _ = shut_tx.send(true);
    task.await.unwrap();
}
