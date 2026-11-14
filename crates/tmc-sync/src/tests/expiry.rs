//! Tests de caducidad por antigüedad, límite de cola y mantenimiento (C9, E0-12).

use crate::clock::TestClock;
use crate::housekeeping::run_housekeeping;
use crate::status::StatusAggregator;
use chrono::{Duration as ChronoDuration, TimeZone, Utc};
use std::time::Duration;
use tmc_model::sync::{OutboxState, RowOrigin, SyncReport, SyncService};
use tmc_store::{NewOutboxRow, SYNC_PENDING_CAP, Store};
use tokio::sync::watch;

const F1: &str = "F0000001";

fn row(service: SyncService, key: &str, event_ts: chrono::DateTime<Utc>) -> NewOutboxRow {
    NewOutboxRow {
        service,
        cmdr_fid: F1.into(),
        idem_key: key.into(),
        kind: "FSDJump".into(),
        coalesce_key: None,
        event_ts,
        payload: "{}".into(),
        origin: RowOrigin::Live,
        state: OutboxState::Pending,
        last_class: None,
    }
}

fn status_for(reports: &[SyncReport], service: SyncService) -> tmc_model::sync::SyncServiceStatus {
    for r in reports {
        match (service, r) {
            (SyncService::Inara, SyncReport::Inara(s)) => return s.status.clone(),
            (SyncService::Edsm, SyncReport::Edsm(s)) => return s.status.clone(),
            (SyncService::Eddn, SyncReport::Eddn(s)) => return s.status.clone(),
            _ => {}
        }
    }
    panic!("report ausente para {service:?}");
}

#[tokio::test(start_paused = true)]
async fn rows_older_than_service_limit_expire_and_count() {
    let store = Store::open_in_memory().unwrap();
    let now = Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap();
    let clock = TestClock::new(now);

    store
        .enqueue_batch_with_cursor(
            &[
                row(
                    SyncService::Eddn,
                    "eddn-25h",
                    now - ChronoDuration::hours(25),
                ),
                row(SyncService::Edsm, "edsm-8d", now - ChronoDuration::days(8)),
                row(
                    SyncService::Inara,
                    "inara-8d",
                    now - ChronoDuration::days(8),
                ),
            ],
            &[],
            now,
        )
        .unwrap();

    let (tx, rx) = watch::channel(false);
    let task = tokio::spawn(run_housekeeping(store.clone(), clock.clone(), rx));
    tokio::task::yield_now().await;

    let agg = StatusAggregator::new(store.clone(), clock, "salt".into());
    let reports = agg.build(Some(F1), &|_, _| None);
    for service in SyncService::ALL {
        let st = status_for(&reports, service);
        assert_eq!(st.expired_total, 1, "{service:?} expired_total");
        assert_eq!(st.pending, 0, "{service:?} pending");
    }

    let _ = tx.send(true);
    task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn future_event_ts_is_neither_expired_nor_negative_age() {
    let store = Store::open_in_memory().unwrap();
    let now = Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap();
    let clock = TestClock::new(now);

    store
        .enqueue_batch_with_cursor(
            &[row(
                SyncService::Eddn,
                "future-2h",
                now + ChronoDuration::hours(2),
            )],
            &[],
            now,
        )
        .unwrap();

    let (tx, rx) = watch::channel(false);
    let task = tokio::spawn(run_housekeeping(store.clone(), clock, rx));
    tokio::task::yield_now().await;

    let counts = store.sync_counts(SyncService::Eddn, F1).unwrap();
    assert_eq!(counts.pending, 1);
    assert_eq!(counts.expired_total, 0);

    let _ = tx.send(true);
    task.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn cap_expires_oldest_and_is_visible() {
    let store = Store::open_in_memory().unwrap();
    let now = Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap();
    let clock = TestClock::new(now);

    let extra = 3usize;
    let total = SYNC_PENDING_CAP as usize + extra;
    let base_ts = now - ChronoDuration::hours(2);
    let rows: Vec<NewOutboxRow> = (0..total)
        .map(|i| {
            row(
                SyncService::Eddn,
                &format!("cap-{i}"),
                base_ts + ChronoDuration::milliseconds(i as i64),
            )
        })
        .collect();
    store.enqueue_batch_with_cursor(&rows, &[], now).unwrap();

    let (tx, rx) = watch::channel(false);
    let task = tokio::spawn(run_housekeeping(store.clone(), clock.clone(), rx));
    tokio::task::yield_now().await;

    let agg = StatusAggregator::new(store.clone(), clock, "salt".into());
    let reports = agg.build(Some(F1), &|_, _| None);
    let st = status_for(&reports, SyncService::Eddn);
    assert_eq!(st.expired_total, extra as u64);
    assert_eq!(st.pending, SYNC_PENDING_CAP);

    // La siguiente pasada periódica de 24 h funciona y se apaga limpio al pedirlo.
    tokio::time::advance(Duration::from_secs(3600)).await;
    let _ = tx.send(true);
    task.await.unwrap();
}
