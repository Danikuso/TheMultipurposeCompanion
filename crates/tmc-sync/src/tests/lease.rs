//! Tests de instancia emisora única mediante lease en SQLite (C3c, E0-10).

use super::support::router_bench::{F1, TWO_CMDRS, TestMapper, early_activation};
use super::support::{ev, fixture_lines};
use crate::clock::{Clock, TestClock};
use crate::lease::{LEASE_RENEW_EVERY, LEASE_TTL_S, Lease};
use crate::router::{EnvGates, LiveSignalPolicy, Router, RouterConfig};
use chrono::{TimeZone, Utc};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tmc_model::state::Galaxy;
use tmc_model::sync::{
    FeedOrigin, KeyAssignment, SyncFeed, SyncFeedEvent, SyncReason, SyncService,
};
use tmc_store::{Store, SyncCredential};
use tokio::sync::{Notify, watch};

fn open_pair() -> (tempfile::TempDir, Store, Store) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("tmc.db");
    let store_a = Store::open(&db_path).unwrap();
    let store_b = Store::open(&db_path).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 3, 1, 0, 0, 0).unwrap();
    store_a
        .put_sync_credential(&SyncCredential {
            service: SyncService::Inara,
            cmdr_fid: F1.into(),
            cmdr_name: "CMDR TESTER".into(),
            api_key: "TEST-KEY".into(),
            created_at: now.to_rfc3339(),
            verified_at: None,
            assignment: KeyAssignment::Confirmed,
        })
        .unwrap();
    store_a
        .set_activation(SyncService::Inara, F1, early_activation())
        .unwrap();
    (dir, store_a, store_b)
}

fn make_router(
    dir: &std::path::Path,
    store: Store,
    clock: Arc<TestClock>,
    lease_held: Arc<AtomicBool>,
) -> Router {
    let calls = Arc::new(AtomicUsize::new(0));
    Router::new(
        store,
        clock,
        Arc::new(AtomicBool::new(true)),
        lease_held,
        RouterConfig {
            journal_dir: dir.to_path_buf(),
            env: EnvGates::default(),
            live_signal: LiveSignalPolicy::ProcessCheck,
            flush_every: Duration::from_millis(250),
            flush_rows: 200,
            hold_before_process: (200, Duration::from_secs(10)),
        },
        vec![Box::new(TestMapper {
            service: SyncService::Inara,
            calls,
        })],
    )
}

fn feed_lines(router: &mut Router, lines: &[String], range: std::ops::RangeInclusive<usize>) {
    router
        .handle(SyncFeed {
            event: SyncFeedEvent::GameProcess { running: true },
            origin: FeedOrigin::Live,
            galaxy: Galaxy::Live,
            game_running: true,
        })
        .unwrap();
    for line in range {
        router
            .handle(SyncFeed {
                event: SyncFeedEvent::Journal {
                    ev: ev(&lines[line - 1]),
                    file: TWO_CMDRS.into(),
                    line: line as u64,
                },
                origin: FeedOrigin::Live,
                galaxy: Galaxy::Live,
                game_running: true,
            })
            .unwrap();
    }
    router.flush().unwrap();
}

fn outbox_count(store: &Store) -> i64 {
    store
        .query_json("SELECT COUNT(*) FROM sync_outbox", &[])
        .unwrap()[0][0]
        .as_i64()
        .unwrap()
}

#[tokio::test(start_paused = true)]
async fn only_holder_enqueues_and_sends() {
    let (dir, store_a, store_b) = open_pair();
    let clock = TestClock::new(Utc.with_ymd_and_hms(2026, 3, 9, 10, 0, 0).unwrap());

    let lease_a = Lease::new("desktop");
    let held_a = lease_a.held.clone();
    let (tx_a, rx_a) = watch::channel(false);
    let notify_a = Arc::new(Notify::new());
    let task_a = tokio::spawn(lease_a.run(store_a.clone(), clock.clone(), rx_a, notify_a));
    tokio::task::yield_now().await;
    assert!(held_a.load(Ordering::SeqCst));

    let lease_b = Lease::new("headless");
    let held_b = lease_b.held.clone();
    let (tx_b, rx_b) = watch::channel(false);
    let notify_b = Arc::new(Notify::new());
    let task_b = tokio::spawn(lease_b.run(store_b.clone(), clock.clone(), rx_b, notify_b));
    tokio::task::yield_now().await;
    assert!(!held_b.load(Ordering::SeqCst));

    let lines = fixture_lines("two_cmdrs.01.log");
    let mut router_b = make_router(dir.path(), store_b.clone(), clock.clone(), held_b.clone());
    feed_lines(&mut router_b, &lines, 1..=4);
    assert_eq!(outbox_count(&store_b), 0);
    assert!(!router_b.send_gates().send_allowed());
    assert_eq!(
        router_b.blocked(SyncService::Inara, Some(F1)),
        Some(SyncReason::OtherInstance)
    );

    let mut router_a = make_router(dir.path(), store_a.clone(), clock, held_a);
    feed_lines(&mut router_a, &lines, 1..=4);
    assert!(router_a.send_gates().send_allowed());
    assert_eq!(outbox_count(&store_a), 3);

    let _ = tx_b.send(true);
    let _ = tx_a.send(true);
    task_b.await.unwrap();
    task_a.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn takeover_after_expiry_without_duplicates() {
    let (dir, store_a, store_b) = open_pair();
    let clock = TestClock::new(Utc.with_ymd_and_hms(2026, 3, 9, 10, 0, 0).unwrap());

    // A adquiere y deja de renovar (sin shutdown, simulando cuelgue).
    let lease_a = Lease::new("desktop");
    assert!(
        store_a
            .try_acquire_lease(
                &lease_a.owner,
                clock.now_utc(),
                chrono::Duration::seconds(LEASE_TTL_S),
            )
            .unwrap()
    );
    let mut router_a = make_router(
        dir.path(),
        store_a.clone(),
        clock.clone(),
        Arc::new(AtomicBool::new(true)),
    );
    let lines = fixture_lines("two_cmdrs.01.log");
    feed_lines(&mut router_a, &lines, 1..=4);
    assert_eq!(outbox_count(&store_a), 3);

    // Avanzar 46 s (> TTL 45 s): B toma el lease; reinyectar las mismas líneas no duplica.
    tokio::time::advance(Duration::from_secs(LEASE_TTL_S as u64 + 1)).await;
    let lease_b = Lease::new("desktop");
    let held_b = lease_b.held.clone();
    let (tx_b, rx_b) = watch::channel(false);
    let task_b = tokio::spawn(lease_b.run(
        store_b.clone(),
        clock.clone(),
        rx_b,
        Arc::new(Notify::new()),
    ));
    tokio::task::yield_now().await;
    assert!(held_b.load(Ordering::SeqCst));

    let mut router_b = make_router(dir.path(), store_b.clone(), clock, held_b);
    feed_lines(&mut router_b, &lines, 1..=4);
    assert_eq!(outbox_count(&store_b), 3);

    let _ = tx_b.send(true);
    task_b.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn orderly_release_lets_other_take_immediately() {
    let (_dir, store_a, store_b) = open_pair();
    let clock = TestClock::new(Utc.with_ymd_and_hms(2026, 3, 9, 10, 0, 0).unwrap());

    let lease_a = Lease::new("desktop");
    let owner_a = lease_a.owner.clone();
    let held_a = lease_a.held.clone();
    let (tx_a, rx_a) = watch::channel(false);
    let task_a = tokio::spawn(lease_a.run(
        store_a.clone(),
        clock.clone(),
        rx_a,
        Arc::new(Notify::new()),
    ));
    tokio::task::yield_now().await;
    assert!(held_a.load(Ordering::SeqCst));
    assert_eq!(
        store_a.lease_owner().unwrap().map(|(o, _)| o),
        Some(owner_a)
    );

    // Cierre ordenado de A libera de inmediato sin esperar los 45 s de TTL.
    let _ = tx_a.send(true);
    task_a.await.unwrap();
    assert!(!held_a.load(Ordering::SeqCst));
    assert_eq!(store_a.lease_owner().unwrap(), None);

    let lease_b = Lease::new("headless");
    let held_b = lease_b.held.clone();
    let (tx_b, rx_b) = watch::channel(false);
    let task_b = tokio::spawn(lease_b.run(store_b.clone(), clock, rx_b, Arc::new(Notify::new())));
    tokio::task::yield_now().await;
    assert!(held_b.load(Ordering::SeqCst));

    let _ = tx_b.send(true);
    task_b.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn holder_that_loses_lease_stops_enqueuing_and_sending() {
    let (dir, store_a, store_b) = open_pair();
    let clock_a = TestClock::new(Utc.with_ymd_and_hms(2026, 3, 9, 10, 0, 0).unwrap());

    let lease_a = Lease::new("desktop");
    let held_a = lease_a.held.clone();
    let (tx_a, rx_a) = watch::channel(false);
    let notify_a = Arc::new(Notify::new());
    let notified = notify_a.notified();
    let task_a =
        tokio::spawn(lease_a.run(store_a.clone(), clock_a.clone(), rx_a, notify_a.clone()));
    tokio::pin!(notified);
    notified.as_mut().await;
    assert!(held_a.load(Ordering::SeqCst));

    let mut router_a = make_router(dir.path(), store_a.clone(), clock_a.clone(), held_a.clone());
    assert!(router_a.send_gates().send_allowed());

    // B fuerza expiración pasada en BD (simula salto de reloj / suspensión) y toma el lease.
    store_b
        .exec_json(
            "UPDATE sync_lease SET expires_at='2026-03-09T09:00:00Z' WHERE id=1",
            &[],
        )
        .unwrap();
    let owner_b = "b-instance:desktop";
    assert!(
        store_b
            .try_acquire_lease(
                owner_b,
                clock_a.now_utc(),
                chrono::Duration::seconds(LEASE_TTL_S),
            )
            .unwrap()
    );

    // En la siguiente renovación (15 s), A detecta que el dueño ya no es A → held false y notifica.
    let notified_loss = notify_a.notified();
    tokio::time::advance(LEASE_RENEW_EVERY).await;
    notified_loss.await;
    assert!(!held_a.load(Ordering::SeqCst));
    assert!(!router_a.send_gates().send_allowed());

    let lines = fixture_lines("two_cmdrs.01.log");
    feed_lines(&mut router_a, &lines, 1..=4);
    assert_eq!(outbox_count(&store_a), 0);
    assert_eq!(
        router_a.blocked(SyncService::Inara, Some(F1)),
        Some(SyncReason::OtherInstance)
    );

    let _ = tx_a.send(true);
    task_a.await.unwrap();
    // El cierre de A no borra el lease que ahora pertenece a B.
    assert_eq!(
        store_b.lease_owner().unwrap().map(|(o, _)| o),
        Some(owner_b.to_string())
    );
}
