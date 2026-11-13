//! Pruebas de credenciales, interruptor EDDN y visibilidad de descarte (C7).

use crate::clock::TestClock;
use crate::eddn::EddnMapper;
use crate::router::{EDDN_ENABLED_KEY, EnvGates, LiveSignalPolicy, Router, RouterConfig};
use crate::status::StatusAggregator;
use chrono::{SecondsFormat, TimeZone, Utc};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tempfile::tempdir;
use tmc_model::events::{JournalEvent, parse_line};
use tmc_model::state::Galaxy;
use tmc_model::sync::{
    FeedOrigin, SyncFeed, SyncFeedEvent, SyncReason, SyncReport, SyncService, SyncState,
};
use tmc_net::eddn_upload::SchemaMode;
use tmc_store::Store;

const F1: &str = "F0000001";

fn parse(raw: &str) -> JournalEvent {
    parse_line(raw).unwrap()
}

fn live_feed(event: SyncFeedEvent) -> SyncFeed {
    SyncFeed {
        origin: FeedOrigin::Live,
        galaxy: Galaxy::Live,
        game_running: true,
        event,
    }
}

fn setup_router(
    dir: &std::path::Path,
    store: &Store,
    base: chrono::DateTime<Utc>,
) -> (Router, Arc<TestClock>) {
    let clock = TestClock::new(base);
    let mut router = Router::new(
        store.clone(),
        clock.clone(),
        Arc::new(AtomicBool::new(true)),
        Arc::new(AtomicBool::new(true)),
        RouterConfig {
            env: EnvGates {
                e2e_simulation: false,
            },
            live_signal: LiveSignalPolicy::ProcessCheck,
            journal_dir: dir.to_path_buf(),
            flush_every: Duration::from_secs(60),
            flush_rows: 200,
            hold_before_process: (200, Duration::from_secs(10)),
        },
        vec![Box::new(EddnMapper::new(SchemaMode::Live, "salt-test"))],
    );
    router
        .handle(live_feed(SyncFeedEvent::CatchUpDone))
        .unwrap();
    router
        .handle(live_feed(SyncFeedEvent::GameProcess { running: true }))
        .unwrap();
    (router, clock)
}

fn seed_session(router: &mut Router, file: &str, t0: chrono::DateTime<Utc>) {
    let ts = |secs: i64| {
        (t0 + chrono::Duration::seconds(secs)).to_rfc3339_opts(SecondsFormat::Secs, true)
    };
    for (line, ev_json) in [
        (
            1u64,
            format!(
                r#"{{"timestamp":"{}","event":"Fileheader","part":1,"gameversion":"4.1.0.0","build":"r1","Odyssey":true}}"#,
                ts(0)
            ),
        ),
        (
            2u64,
            format!(
                r#"{{"timestamp":"{}","event":"LoadGame","FID":"{F1}","Commander":"CMDR ONE","Ship":"SideWinder","ShipID":1,"Horizons":true,"Odyssey":true}}"#,
                ts(1)
            ),
        ),
    ] {
        router
            .handle(live_feed(SyncFeedEvent::Journal {
                ev: parse(&ev_json),
                file: file.into(),
                line,
            }))
            .unwrap();
    }
}

#[test]
fn eddn_default_off_enqueues_nothing() {
    let dir = tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let base = Utc.with_ymd_and_hms(2026, 3, 10, 10, 0, 0).unwrap();
    let (mut router, clock) = setup_router(dir.path(), &store, base);
    let file = "Journal.2026-03-10T100000.01.log";

    seed_session(&mut router, file, base);
    router
        .handle(live_feed(SyncFeedEvent::Journal {
            ev: parse(
                r#"{"timestamp":"2026-03-10T10:00:05Z","event":"Location","StarSystem":"Sol","SystemAddress":10477373803,"StarPos":[0.0,0.0,0.0],"Docked":false}"#,
            ),
            file: file.into(),
            line: 3,
        }))
        .unwrap();
    router
        .handle(live_feed(SyncFeedEvent::Journal {
            ev: parse(
                r#"{"timestamp":"2026-03-10T10:00:10Z","event":"FSDJump","StarSystem":"Luhman 16","SystemAddress":10477373899,"StarPos":[1.0,2.0,3.0],"JumpDist":6.5}"#,
            ),
            file: file.into(),
            line: 4,
        }))
        .unwrap();
    router.flush().unwrap();

    let counts = store.sync_counts(SyncService::Eddn, F1).unwrap();
    assert_eq!(counts.pending, 0);
    assert_eq!(counts.sent_total, 0);

    let agg = StatusAggregator::new(store.clone(), clock, "salt-test".into());
    let reports = agg.build(Some(F1), &|svc, fid| {
        crate::router::blocked_with(&router.send_gates(), &store, svc, fid)
    });
    let eddn = reports
        .into_iter()
        .find_map(|r| match r {
            SyncReport::Eddn(s) => Some(s),
            _ => None,
        })
        .unwrap();
    assert!(!eddn.enabled);
    assert_eq!(eddn.status.state, SyncState::Disabled);
    assert_eq!(eddn.status.reason, Some(SyncReason::SwitchOff));
}

#[test]
fn eddn_on_sends_only_after_switch_time() {
    let dir = tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let base = Utc.with_ymd_and_hms(2026, 3, 10, 10, 0, 0).unwrap();
    let (mut router, _clock) = setup_router(dir.path(), &store, base);
    let file = "Journal.2026-03-10T100000.01.log";

    seed_session(&mut router, file, base);
    // Evento previo al encendido: no se encola.
    router
        .handle(live_feed(SyncFeedEvent::Journal {
            ev: parse(
                r#"{"timestamp":"2026-03-10T10:00:05Z","event":"Location","StarSystem":"Sol","SystemAddress":10477373803,"StarPos":[0.0,0.0,0.0],"Docked":false}"#,
            ),
            file: file.into(),
            line: 3,
        }))
        .unwrap();
    router.flush().unwrap();
    assert_eq!(store.sync_counts(SyncService::Eddn, F1).unwrap().pending, 0);

    // Encender EDDN en mitad de sesión en t = 10:01:00Z.
    store.kv_set(EDDN_ENABLED_KEY, "2026-03-10T10:01:00Z");

    // Evento anterior al instante de encendido -> no entra.
    router
        .handle(live_feed(SyncFeedEvent::Journal {
            ev: parse(
                r#"{"timestamp":"2026-03-10T10:00:50Z","event":"FSDJump","StarSystem":"Alpha Centauri","SystemAddress":10477373804,"StarPos":[1.0,1.0,1.0],"JumpDist":4.3}"#,
            ),
            file: file.into(),
            line: 4,
        }))
        .unwrap();
    router.flush().unwrap();
    assert_eq!(store.sync_counts(SyncService::Eddn, F1).unwrap().pending, 0);

    // Evento posterior al instante de encendido -> sí se encola aunque no haya habido otro LoadGame.
    router
        .handle(live_feed(SyncFeedEvent::Journal {
            ev: parse(
                r#"{"timestamp":"2026-03-10T10:01:05Z","event":"FSDJump","StarSystem":"Sirius","SystemAddress":10477373805,"StarPos":[2.0,2.0,2.0],"JumpDist":8.5}"#,
            ),
            file: file.into(),
            line: 5,
        }))
        .unwrap();
    router.flush().unwrap();
    assert_eq!(store.sync_counts(SyncService::Eddn, F1).unwrap().pending, 1);
}

#[test]
fn eddn_off_discards_pending_visibly() {
    let dir = tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let base = Utc.with_ymd_and_hms(2026, 3, 10, 10, 0, 0).unwrap();
    store.kv_set(EDDN_ENABLED_KEY, "2026-03-10T10:00:00Z");
    let (mut router, clock) = setup_router(dir.path(), &store, base);
    let file = "Journal.2026-03-10T100000.01.log";

    seed_session(&mut router, file, base);
    for (line, ev_json) in [
        (
            3u64,
            r#"{"timestamp":"2026-03-10T10:00:05Z","event":"Location","StarSystem":"Sol","SystemAddress":10477373803,"StarPos":[0.0,0.0,0.0],"Docked":false}"#,
        ),
        (
            4u64,
            r#"{"timestamp":"2026-03-10T10:00:10Z","event":"FSDJump","StarSystem":"Alpha Centauri","SystemAddress":10477373804,"StarPos":[1.0,1.0,1.0],"JumpDist":4.3}"#,
        ),
        (
            5u64,
            r#"{"timestamp":"2026-03-10T10:00:15Z","event":"FSDJump","StarSystem":"Sirius","SystemAddress":10477373805,"StarPos":[2.0,2.0,2.0],"JumpDist":8.5}"#,
        ),
    ] {
        router
            .handle(live_feed(SyncFeedEvent::Journal {
                ev: parse(ev_json),
                file: file.into(),
                line,
            }))
            .unwrap();
    }
    router.flush().unwrap();
    assert_eq!(store.sync_counts(SyncService::Eddn, F1).unwrap().pending, 3);

    // Apagar EDDN: borra activación, descarta los 3 pendientes visiblemente.
    store.kv_delete(EDDN_ENABLED_KEY);
    let discarded = store
        .discard_pending(
            SyncService::Eddn,
            None,
            base + chrono::Duration::seconds(30),
        )
        .unwrap();
    store.clear_sync_activations(SyncService::Eddn).unwrap();
    assert_eq!(discarded, 3);

    let counts = store.sync_counts(SyncService::Eddn, F1).unwrap();
    assert_eq!(counts.pending, 0);
    assert_eq!(counts.discarded, 3);
    assert!(store.activation(SyncService::Eddn, F1).unwrap().is_none());

    let agg = StatusAggregator::new(store.clone(), clock, "salt-test".into());
    let reports = agg.build(Some(F1), &|svc, fid| {
        crate::router::blocked_with(&router.send_gates(), &store, svc, fid)
    });
    let eddn = reports
        .into_iter()
        .find_map(|r| match r {
            SyncReport::Eddn(s) => Some(s),
            _ => None,
        })
        .unwrap();
    assert!(!eddn.enabled);
    assert_eq!(eddn.status.pending, 0);
    assert_eq!(eddn.status.state, SyncState::Disabled);
    assert_eq!(eddn.status.reason, Some(SyncReason::SwitchOff));
}

#[test]
fn two_cmdrs_key_only_for_one() {
    use crate::edsm::{DiscardList, EdsmMapper};
    use tmc_model::sync::KeyAssignment;
    use tmc_net::edsm_journal::parse_discard;
    use tmc_store::SyncCredential;

    const F2: &str = "F0000002";
    let dir = tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let base = Utc.with_ymd_and_hms(2026, 3, 9, 9, 0, 0).unwrap();
    let discard_json = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/sync/edsm/discard.json"),
    )
    .unwrap();
    let discard = DiscardList::with_set(parse_discard(&discard_json).unwrap());

    // Solo F1 tiene clave EDSM; F2 no tiene ninguna clave.
    store.upsert_known_cmdr(F1, "CMDR TESTER", base).unwrap();
    store.upsert_known_cmdr(F2, "CMDR TESTER", base).unwrap();
    store
        .put_sync_credential(&SyncCredential {
            service: SyncService::Edsm,
            cmdr_fid: F1.into(),
            cmdr_name: "CMDR TESTER".into(),
            api_key: "EDSM-KEY-F1".into(),
            created_at: base.to_rfc3339(),
            verified_at: Some(base.to_rfc3339()),
            assignment: KeyAssignment::Confirmed,
        })
        .unwrap();
    store
        .set_activation(SyncService::Edsm, F1, base - chrono::Duration::hours(1))
        .unwrap();

    let clock = TestClock::new(base + chrono::Duration::minutes(5));
    let mut router = Router::new(
        store.clone(),
        clock,
        Arc::new(AtomicBool::new(true)),
        Arc::new(AtomicBool::new(true)),
        RouterConfig {
            env: EnvGates::default(),
            live_signal: LiveSignalPolicy::ProcessCheck,
            journal_dir: dir.path().to_path_buf(),
            flush_every: Duration::from_secs(60),
            flush_rows: 200,
            hold_before_process: (200, Duration::from_secs(10)),
        },
        vec![Box::new(EdsmMapper::new(discard))],
    );
    router
        .handle(live_feed(SyncFeedEvent::CatchUpDone))
        .unwrap();
    router
        .handle(live_feed(SyncFeedEvent::GameProcess { running: true }))
        .unwrap();

    let two_cmdrs_log = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/sync/core/two_cmdrs.01.log"),
    )
    .unwrap();
    let file = "Journal.2026-03-09T090001.01.log";
    for (idx, line) in two_cmdrs_log.lines().enumerate() {
        router
            .handle(live_feed(SyncFeedEvent::Journal {
                ev: parse(line),
                file: file.into(),
                line: (idx + 1) as u64,
            }))
            .unwrap();
    }
    router.flush().unwrap();

    // F1 genera filas EDSM; F2 sin clave produce exactamente 0 filas EDSM.
    assert!(store.sync_counts(SyncService::Edsm, F1).unwrap().pending > 0);
    assert_eq!(store.sync_counts(SyncService::Edsm, F2).unwrap().pending, 0);
    assert_eq!(
        store.sync_counts(SyncService::Edsm, F2).unwrap().sent_total,
        0
    );
}

#[test]
fn delete_key_discards_pending_visibly() {
    use crate::edsm::{DiscardList, EdsmMapper};
    use tmc_model::sync::KeyAssignment;
    use tmc_net::edsm_journal::parse_discard;
    use tmc_store::SyncCredential;

    let dir = tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let base = Utc.with_ymd_and_hms(2026, 3, 10, 10, 0, 0).unwrap();
    let discard_json = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/sync/edsm/discard.json"),
    )
    .unwrap();
    let discard = DiscardList::with_set(parse_discard(&discard_json).unwrap());

    store.upsert_known_cmdr(F1, "CMDR ONE", base).unwrap();
    store
        .put_sync_credential(&SyncCredential {
            service: SyncService::Edsm,
            cmdr_fid: F1.into(),
            cmdr_name: "CMDR ONE".into(),
            api_key: "EDSM-KEY-DEL".into(),
            created_at: base.to_rfc3339(),
            verified_at: Some(base.to_rfc3339()),
            assignment: KeyAssignment::Confirmed,
        })
        .unwrap();
    store
        .set_activation(SyncService::Edsm, F1, base - chrono::Duration::minutes(5))
        .unwrap();

    let clock = TestClock::new(base + chrono::Duration::seconds(30));
    let mut router = Router::new(
        store.clone(),
        clock.clone(),
        Arc::new(AtomicBool::new(true)),
        Arc::new(AtomicBool::new(true)),
        RouterConfig {
            env: EnvGates::default(),
            live_signal: LiveSignalPolicy::ProcessCheck,
            journal_dir: dir.path().to_path_buf(),
            flush_every: Duration::from_secs(60),
            flush_rows: 200,
            hold_before_process: (200, Duration::from_secs(10)),
        },
        vec![Box::new(EdsmMapper::new(discard))],
    );
    router
        .handle(live_feed(SyncFeedEvent::CatchUpDone))
        .unwrap();
    router
        .handle(live_feed(SyncFeedEvent::GameProcess { running: true }))
        .unwrap();

    let file = "Journal.2026-03-10T100000.01.log";
    seed_session(&mut router, file, base);
    router
        .handle(live_feed(SyncFeedEvent::Journal {
            ev: parse(
                r#"{"timestamp":"2026-03-10T10:00:10Z","event":"FSDJump","StarSystem":"Sol","SystemAddress":10477373803,"StarPos":[0.0,0.0,0.0],"JumpDist":4.3}"#,
            ),
            file: file.into(),
            line: 3,
        }))
        .unwrap();
    router.flush().unwrap();
    let before = store.sync_counts(SyncService::Edsm, F1).unwrap().pending;
    assert!(before >= 1);

    let discarded = store
        .delete_sync_credential(SyncService::Edsm, F1, base + chrono::Duration::minutes(1))
        .unwrap();
    assert_eq!(discarded, before);

    let after = store.sync_counts(SyncService::Edsm, F1).unwrap();
    assert_eq!(after.pending, 0);
    assert_eq!(after.discarded, discarded);

    let agg = StatusAggregator::new(store.clone(), clock, "salt-test".into());
    let reports = agg.build(Some(F1), &|svc, fid| {
        crate::router::blocked_with(&router.send_gates(), &store, svc, fid)
    });
    let edsm = reports
        .into_iter()
        .find_map(|r| match r {
            SyncReport::Edsm(s) => Some(s),
            _ => None,
        })
        .unwrap();
    assert!(!edsm.has_key);
    assert_eq!(edsm.status.pending, 0);
    assert_eq!(edsm.status.state, SyncState::Disabled);
    assert_eq!(edsm.status.reason, Some(SyncReason::NoKey));
}

#[tokio::test(start_paused = true)]
async fn switch_cmdr_keeps_sending_previous_cmdr_pending() {
    use crate::edsm::{DiscardList, EdsmClient};
    use crate::worker::Worker;
    use tmc_model::sync::{KeyAssignment, RowOrigin};
    use tmc_net::edsm_journal::parse_discard;
    use tmc_net::testing::ScriptedTransport;
    use tmc_net::transport::HttpReplyEx;
    use tmc_store::{NewOutboxRow, SyncCredential};

    const F2: &str = "F0000002";
    let dir = tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let base = Utc.with_ymd_and_hms(2026, 3, 10, 10, 0, 0).unwrap();
    let discard_json = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/sync/edsm/discard.json"),
    )
    .unwrap();
    let discard = DiscardList::with_set(parse_discard(&discard_json).unwrap());

    store.upsert_known_cmdr(F1, "CMDR ONE", base).unwrap();
    store.upsert_known_cmdr(F2, "CMDR TWO", base).unwrap();
    store
        .put_sync_credential(&SyncCredential {
            service: SyncService::Edsm,
            cmdr_fid: F1.into(),
            cmdr_name: "CMDR ONE".into(),
            api_key: "EDSM-KEY-F1".into(),
            created_at: base.to_rfc3339(),
            verified_at: Some(base.to_rfc3339()),
            assignment: KeyAssignment::Confirmed,
        })
        .unwrap();
    store
        .set_activation(SyncService::Edsm, F1, base - chrono::Duration::minutes(5))
        .unwrap();

    let event_ts = base - chrono::Duration::seconds(30);
    store
        .enqueue_batch_with_cursor(
            &[NewOutboxRow {
                service: SyncService::Edsm,
                cmdr_fid: F1.into(),
                idem_key: "edsm:f1:switch:1".into(),
                kind: "FSDJump".into(),
                coalesce_key: None,
                event_ts,
                payload: serde_json::json!({
                    "fromGameVersion": "4.1.0.0",
                    "fromGameBuild": "r1",
                    "message": { "timestamp": event_ts.to_rfc3339(), "event": "FSDJump", "StarSystem": "Sol" }
                })
                .to_string(),
                origin: RowOrigin::Live,
                state: tmc_model::sync::OutboxState::Pending,
                last_class: None,
            }],
            &[],
            base,
        )
        .unwrap();

    let (mut router, clock) = setup_router(dir.path(), &store, base);
    // Sesión activa cambia a F2.
    router
        .handle(live_feed(SyncFeedEvent::Journal {
            ev: parse(&format!(
                r#"{{"timestamp":"{}","event":"LoadGame","FID":"{F2}","Commander":"CMDR TWO","Horizons":true,"Odyssey":true}}"#,
                base.to_rfc3339_opts(SecondsFormat::Secs, true)
            )),
            file: "Journal.2026-03-10T100000.01.log".into(),
            line: 2,
        }))
        .unwrap();
    assert_eq!(router.tracker().snapshot().cmdr_fid.as_deref(), Some(F2));

    let agg = StatusAggregator::new(store.clone(), clock.clone(), "salt-test".into());
    let reports = agg.build(Some(F2), &|svc, fid| {
        crate::router::blocked_with(&router.send_gates(), &store, svc, fid)
    });
    let roster = reports
        .into_iter()
        .find_map(|r| match r {
            SyncReport::Roster(r) => Some(r),
            _ => None,
        })
        .unwrap();
    assert_eq!(roster.other_cmdrs_pending.edsm, 1);

    let transport = ScriptedTransport::new(vec![Ok(HttpReplyEx {
        status: 200,
        body: r#"{"msgnum":100,"msg":"OK","events":[{"msgnum":100,"msg":"OK"}]}"#.into(),
        retry_after_s: None,
        rate_limit_remaining: None,
        rate_limit_reset_s: None,
    })]);
    let client: Arc<dyn crate::worker::ServiceClient> = Arc::new(EdsmClient::new(
        transport.clone(),
        store.clone(),
        clock.clone(),
        discard,
        Arc::new(tokio::sync::Notify::new()),
    ));
    let (ev_tx, mut ev_rx) = tokio::sync::mpsc::channel(16);
    let (_shut_tx, shut_rx) = tokio::sync::watch::channel(false);
    let worker = Worker::new(
        client,
        store.clone(),
        clock,
        router.send_gates(),
        Arc::new(tokio::sync::Notify::new()),
        ev_tx,
    );
    let handle = tokio::spawn(worker.run(shut_rx));
    let ev = ev_rx.recv().await.unwrap();
    assert!(matches!(
        ev,
        crate::worker::WorkerEvent::Outcome { sent: 1, .. }
    ));
    assert_eq!(transport.calls(), 1);
    assert_eq!(
        store.sync_counts(SyncService::Edsm, F1).unwrap().sent_total,
        1
    );
    drop(_shut_tx);
    let _ = handle.await;
}
