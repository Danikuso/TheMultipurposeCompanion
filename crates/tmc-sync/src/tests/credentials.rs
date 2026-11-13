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
