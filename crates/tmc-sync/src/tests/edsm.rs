//! Tests de mapeo y envío de EDSM (E2-2 y E2-3).

use crate::clock::TestClock;
use crate::edsm::{
    DiscardList, DiscardStatus, EDSM_DISCARD_CACHE_KEY, EdsmMapper, TMC_PRIVACY_LIST,
    refresh_discard_if_needed,
};
use crate::router::{EnvGates, LiveSignalPolicy, Router, RouterConfig, ServiceMapper};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tmc_model::events::parse_line;
use tmc_model::state::Galaxy;
use tmc_model::sync::{FeedOrigin, KeyAssignment, SyncFeed, SyncFeedEvent, SyncService};
use tmc_net::edsm_journal::{EDSM_DISCARD_URL, parse_discard};
use tmc_net::testing::ScriptedTransport;
use tmc_net::transport::HttpReply;
use tmc_store::{Store, SyncCredential};

const F1: &str = "F0000001";
const LOG_FILE: &str = "Journal.2026-01-01T120000.01.log";

fn edsm_fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/sync/edsm")
}

fn eddn_journal_fsdjump_input() -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/sync/eddn/journal_1/fsdjump/input.log"),
    )
    .expect("fixture input.log")
}

fn fixture_discard_json() -> String {
    std::fs::read_to_string(edsm_fixtures_root().join("discard.json"))
        .expect("fixture discard.json")
}

fn fixture_discard_set() -> HashSet<String> {
    parse_discard(&fixture_discard_json()).expect("discard.json válido")
}

fn early_activation() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()
}

fn setup_edsm_router(
    dir: &std::path::Path,
    store: &Store,
    discard: Arc<DiscardList>,
    now: DateTime<Utc>,
) -> Router {
    store.upsert_known_cmdr(F1, "CMDR Ficticio", now).unwrap();
    store
        .put_sync_credential(&SyncCredential {
            service: SyncService::Edsm,
            cmdr_fid: F1.into(),
            cmdr_name: "CMDR Ficticio".into(),
            api_key: "EXAMPLE-EDSM-API-KEY".into(),
            created_at: early_activation().to_rfc3339(),
            verified_at: Some(early_activation().to_rfc3339()),
            assignment: KeyAssignment::Confirmed,
        })
        .unwrap();
    store
        .set_activation(SyncService::Edsm, F1, early_activation())
        .unwrap();
    let cfg = RouterConfig {
        journal_dir: dir.to_path_buf(),
        env: EnvGates::default(),
        live_signal: LiveSignalPolicy::JournalActivity,
        flush_every: Duration::from_millis(250),
        flush_rows: 200,
        hold_before_process: (200, Duration::from_secs(10)),
    };
    let clock = TestClock::new(now);
    let mappers: Vec<Box<dyn ServiceMapper>> = vec![Box::new(EdsmMapper::new(discard))];
    Router::new(
        store.clone(),
        clock,
        Arc::new(AtomicBool::new(true)),
        Arc::new(AtomicBool::new(true)),
        cfg,
        mappers,
    )
}

fn feed_line(router: &mut Router, line: u64, raw: &str) {
    router
        .handle(SyncFeed {
            event: SyncFeedEvent::Journal {
                ev: parse_line(raw).unwrap(),
                file: LOG_FILE.into(),
                line,
            },
            origin: FeedOrigin::Live,
            galaxy: Galaxy::Live,
            game_running: true,
        })
        .unwrap();
}

fn claim_all_edsm(store: &Store, now: DateTime<Utc>) -> Vec<tmc_store::OutboxRow> {
    store
        .claim_ready(SyncService::Edsm, Some(F1), 100, now)
        .unwrap()
}

fn last_class_of(store: &Store, kind: &str) -> Option<String> {
    let rows = store
        .query_json(
            "SELECT last_class FROM sync_outbox WHERE service='edsm' AND kind=?1",
            &[kind.into()],
        )
        .unwrap();
    rows.first().and_then(|r| r[0].as_str().map(str::to_owned))
}

#[test]
fn payload_is_raw_line_plus_transients() {
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 5, 0).unwrap();
    let discard = DiscardList::with_set(fixture_discard_set());
    let mut router = setup_edsm_router(tmp.path(), &store, discard, now);

    let input = eddn_journal_fsdjump_input();
    for (idx, raw) in input.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        feed_line(&mut router, (idx + 1) as u64, raw);
    }
    router.flush().unwrap();

    let rows = claim_all_edsm(&store, now);
    let fsd = rows
        .iter()
        .find(|r| r.kind == "FSDJump")
        .expect("fila FSDJump encolada");
    let saved: Value = serde_json::from_str(&fsd.payload).unwrap();
    let fixture_req: Value = serde_json::from_str(
        &std::fs::read_to_string(edsm_fixtures_root().join("request_batch.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(saved["fromGameVersion"], "4.0.0.1903");
    assert_eq!(saved["fromGameBuild"], "r310123/r0 ");
    assert_eq!(saved["message"], fixture_req["message"][0]);
}

#[test]
fn privacy_list_never_enqueued() {
    assert_eq!(TMC_PRIVACY_LIST.len(), 17);
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 5, 0).unwrap();
    let mut router = setup_edsm_router(
        tmp.path(),
        &store,
        DiscardList::with_set(HashSet::new()),
        now,
    );

    feed_line(
        &mut router,
        1,
        r#"{"timestamp":"2026-01-01T12:00:00Z","event":"Fileheader","part":1,"gameversion":"4.0.0.1903","build":"r310123/r0 "}"#,
    );
    feed_line(
        &mut router,
        2,
        r#"{"timestamp":"2026-01-01T12:00:01Z","event":"LoadGame","FID":"F0000001","Commander":"CMDR Ficticio","Horizons":true,"Odyssey":true}"#,
    );
    router.flush().unwrap();

    let counts_before = store.sync_counts(SyncService::Edsm, F1).unwrap();
    for (idx, event_name) in TMC_PRIVACY_LIST.iter().enumerate() {
        let raw = format!(
            r#"{{"timestamp":"2026-01-01T12:01:{:02}Z","event":"{event_name}","Message":"privado"}}"#,
            idx
        );
        feed_line(&mut router, (idx + 3) as u64, &raw);
    }
    router.flush().unwrap();

    let counts_after = store.sync_counts(SyncService::Edsm, F1).unwrap();
    assert_eq!(counts_after.pending, counts_before.pending);
    assert_eq!(counts_after.discarded, 0);
    assert_eq!(counts_after.omitted_total, 0);
}

#[test]
fn discarded_event_counted_as_discarded() {
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 5, 0).unwrap();
    let mut router = setup_edsm_router(
        tmp.path(),
        &store,
        DiscardList::with_set(fixture_discard_set()),
        now,
    );

    feed_line(
        &mut router,
        1,
        r#"{"timestamp":"2026-01-01T12:00:00Z","event":"Fileheader","part":1,"gameversion":"4.0.0.1903","build":"r310123/r0 "}"#,
    );
    feed_line(
        &mut router,
        2,
        r#"{"timestamp":"2026-01-01T12:00:01Z","event":"LoadGame","FID":"F0000001","Commander":"CMDR Ficticio","Horizons":true,"Odyssey":true}"#,
    );
    feed_line(
        &mut router,
        3,
        r#"{"timestamp":"2026-01-01T12:01:00Z","event":"Market","MarketID":128000001}"#,
    );
    feed_line(
        &mut router,
        4,
        r#"{"timestamp":"2026-01-01T12:02:00Z","event":"ShutDown"}"#,
    );
    router.flush().unwrap();

    let counts = store.sync_counts(SyncService::Edsm, F1).unwrap();
    assert_eq!(counts.discarded, 2);
    assert_eq!(counts.omitted_total, 0);
}

#[test]
fn missing_discard_list_enqueues_pending_with_reason() {
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let now = Utc.with_ymd_and_hms(2026, 1, 1, 12, 5, 0).unwrap();
    let mut router = setup_edsm_router(tmp.path(), &store, DiscardList::empty(), now);

    feed_line(
        &mut router,
        1,
        r#"{"timestamp":"2026-01-01T12:00:00Z","event":"Fileheader","part":1,"gameversion":"4.0.0.1903","build":"r310123/r0 "}"#,
    );
    feed_line(
        &mut router,
        2,
        r#"{"timestamp":"2026-01-01T12:00:01Z","event":"LoadGame","FID":"F0000001","Commander":"CMDR Ficticio","Horizons":true,"Odyssey":true}"#,
    );
    feed_line(
        &mut router,
        3,
        r#"{"timestamp":"2026-01-01T12:01:00Z","event":"FSDJump","StarSystem":"Sol","SystemAddress":10477373803,"StarPos":[0.0,0.0,0.0]}"#,
    );
    router.flush().unwrap();

    let counts = store.sync_counts(SyncService::Edsm, F1).unwrap();
    assert_eq!(counts.pending, 2);
    assert_eq!(
        last_class_of(&store, "FSDJump").as_deref(),
        Some("discard_list_missing")
    );
}

#[tokio::test]
async fn discard_list_cached_24h_in_api_cache() {
    let store = Store::open_in_memory().unwrap();
    let t0 = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
    let clock = TestClock::new(t0);
    let body = fixture_discard_json();

    let transport = ScriptedTransport::new(vec![]);
    transport.push_get(Ok(HttpReply {
        status: 200,
        body: body.clone(),
    }));
    transport.push_get(Ok(HttpReply { status: 200, body }));

    // Arranque 1: descarga y persiste en api_cache.
    let list1 = DiscardList::from_store(&store);
    assert!(!list1.is_ready());
    assert!(
        refresh_discard_if_needed(&store, transport.as_ref(), clock.as_ref(), &list1, false).await
    );
    assert_eq!(transport.calls(), 1);
    assert_eq!(transport.get_urls(), vec![EDSM_DISCARD_URL.to_string()]);
    assert_eq!(list1.status("ShutDown"), DiscardStatus::Discarded);
    assert_eq!(list1.status("FSDJump"), DiscardStatus::Keep);
    assert!(
        store
            .api_cache_get(EDSM_DISCARD_CACHE_KEY)
            .unwrap()
            .is_some()
    );

    // Arranque 2 (< 24 h): reutiliza api_cache sin peticiones HTTP.
    clock.shift(chrono::Duration::hours(23));
    let list2 = DiscardList::from_store(&store);
    assert!(list2.is_ready());
    assert_eq!(list2.status("ShutDown"), DiscardStatus::Discarded);
    assert!(
        !refresh_discard_if_needed(&store, transport.as_ref(), clock.as_ref(), &list2, false).await
    );
    assert_eq!(transport.calls(), 1);

    // Arranque 3 (>= 24 h): caducada → vuelve a consultar HTTP.
    clock.shift(chrono::Duration::hours(1));
    assert!(
        refresh_discard_if_needed(&store, transport.as_ref(), clock.as_ref(), &list2, false).await
    );
    assert_eq!(transport.calls(), 2);
}
