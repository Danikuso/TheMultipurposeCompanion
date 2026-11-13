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

fn reply_ex(
    status: u16,
    body: &str,
) -> Result<tmc_net::transport::HttpReplyEx, tmc_net::transport::TransportError> {
    Ok(tmc_net::transport::HttpReplyEx {
        status,
        body: body.to_string(),
        retry_after_s: None,
        rate_limit_remaining: None,
        rate_limit_reset_s: None,
    })
}

fn setup_worker_store(now: DateTime<Utc>) -> (Store, Arc<TestClock>) {
    let store = Store::open_in_memory().unwrap();
    let clock = TestClock::new(now);
    store.upsert_known_cmdr(F1, "CMDR Ficticio", now).unwrap();
    store
        .put_sync_credential(&SyncCredential {
            service: SyncService::Edsm,
            cmdr_fid: F1.into(),
            cmdr_name: "CMDR Ficticio".into(),
            api_key: "EDSM-KEY-1".into(),
            created_at: now.to_rfc3339(),
            verified_at: Some(now.to_rfc3339()),
            assignment: KeyAssignment::Confirmed,
        })
        .unwrap();
    (store, clock)
}

fn push_edsm_row(
    store: &Store,
    key: &str,
    kind: &str,
    event_ts: DateTime<Utc>,
    class: Option<tmc_model::sync::ErrorClass>,
) {
    let payload = serde_json::json!({
        "fromGameVersion": "4.0.0.1903",
        "fromGameBuild": "r310123/r0 ",
        "message": {
            "timestamp": event_ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "event": kind,
            "StarSystem": "Sol",
            "_systemAddress": 10477373803u64,
            "_systemName": "Sol",
            "_systemCoordinates": [0.0, 0.0, 0.0],
            "_marketId": null,
            "_stationName": null,
            "_shipId": 1u64
        }
    })
    .to_string();
    store
        .enqueue_batch_with_cursor(
            &[tmc_store::NewOutboxRow {
                service: SyncService::Edsm,
                cmdr_fid: F1.into(),
                idem_key: key.into(),
                kind: kind.into(),
                coalesce_key: None,
                event_ts,
                payload,
                origin: tmc_model::sync::RowOrigin::Live,
                state: tmc_model::sync::OutboxState::Pending,
                last_class: class,
            }],
            &[],
            event_ts,
        )
        .unwrap();
}

fn ok_events_body(count: usize) -> String {
    let events: Vec<Value> = (0..count)
        .map(|_| serde_json::json!({ "msgnum": 100, "msg": "OK" }))
        .collect();
    serde_json::json!({ "msgnum": 100, "msg": "OK", "events": events }).to_string()
}

type WorkerRig = (
    Arc<tokio::sync::Notify>,
    Arc<tokio::sync::Notify>,
    tokio::sync::mpsc::Receiver<crate::worker::WorkerEvent>,
    tokio::sync::watch::Sender<bool>,
    tokio::task::JoinHandle<()>,
);

fn setup_worker(
    store: &Store,
    clock: Arc<TestClock>,
    transport: Arc<ScriptedTransport>,
    discard: Arc<DiscardList>,
) -> WorkerRig {
    let refresh_wake = Arc::new(tokio::sync::Notify::new());
    let client: Arc<dyn crate::worker::ServiceClient> = Arc::new(crate::edsm::EdsmClient::new(
        transport,
        store.clone(),
        clock.clone(),
        discard,
        refresh_wake.clone(),
    ));
    let gates = Arc::new(crate::router::SendGates::new(
        Arc::new(AtomicBool::new(true)),
        Arc::new(AtomicBool::new(true)),
    ));
    let wake = Arc::new(tokio::sync::Notify::new());
    let (ev_tx, ev_rx) = tokio::sync::mpsc::channel(32);
    let (shut_tx, shut_rx) = tokio::sync::watch::channel(false);
    let worker =
        crate::worker::Worker::new(client, store.clone(), clock, gates, wake.clone(), ev_tx);
    let handle = tokio::spawn(worker.run(shut_rx));
    (wake, refresh_wake, ev_rx, shut_tx, handle)
}

#[tokio::test(start_paused = true)]
async fn batches_up_to_50_or_15s() {
    let base = Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap();
    let (store, clock) = setup_worker_store(base);
    for i in 0..60 {
        push_edsm_row(&store, &format!("edsm:batch:{i}"), "FSDJump", base, None);
    }
    let transport = ScriptedTransport::new(vec![
        reply_ex(200, &ok_events_body(50)),
        reply_ex(200, &ok_events_body(10)),
    ]);
    let (_wake, _rw, _rx, shut_tx, handle) = setup_worker(
        &store,
        clock,
        transport.clone(),
        DiscardList::with_set(fixture_discard_set()),
    );

    // >= 50 filas → primer lote de 50 inmediato sin esperar 15 s.
    tokio::time::advance(Duration::from_millis(50)).await;
    tokio::task::yield_now().await;
    assert_eq!(transport.calls(), 1);
    let p0: Value = serde_json::from_slice(&transport.posts()[0].body).unwrap();
    assert_eq!(p0["message"].as_array().unwrap().len(), 50);

    // Con < 50 filas restantes (10), antes de 15 s no envía todavía.
    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    assert_eq!(transport.calls(), 1);

    // Cumplidos 15 s desde la primera fila del lote, vacía las 10 restantes.
    tokio::time::advance(Duration::from_secs(6)).await;
    tokio::task::yield_now().await;
    assert_eq!(transport.calls(), 2);
    let p1: Value = serde_json::from_slice(&transport.posts()[1].body).unwrap();
    assert_eq!(p1["message"].as_array().unwrap().len(), 10);
    assert_eq!(
        store.sync_counts(SyncService::Edsm, F1).unwrap().sent_total,
        60
    );

    let _ = shut_tx.send(true);
    handle.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn never_more_than_10_in_10s_and_target_one_per_5s() {
    let base = Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap();
    let (store, clock) = setup_worker_store(base);
    let old_ts = base - chrono::Duration::seconds(60);
    for i in 0..150 {
        push_edsm_row(&store, &format!("edsm:rate:{i}"), "FSDJump", old_ts, None);
    }
    let transport = ScriptedTransport::new(vec![
        reply_ex(200, &ok_events_body(50)),
        reply_ex(200, &ok_events_body(50)),
        reply_ex(200, &ok_events_body(50)),
    ]);
    let (_wake, _rw, _rx, shut_tx, handle) = setup_worker(
        &store,
        clock,
        transport.clone(),
        DiscardList::with_set(fixture_discard_set()),
    );

    tokio::time::advance(Duration::from_millis(50)).await;
    tokio::task::yield_now().await;
    assert_eq!(transport.calls(), 1);

    // A los +4 s todavía debe respetar el objetivo de 1 petición / 5 s.
    tokio::time::advance(Duration::from_secs(4)).await;
    tokio::task::yield_now().await;
    assert_eq!(transport.calls(), 1);

    // A los +5 s y +10 s salen el 2.º y 3.er lote.
    tokio::time::advance(Duration::from_secs(2)).await;
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    assert_eq!(transport.calls(), 2);

    tokio::time::advance(Duration::from_secs(6)).await;
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    assert_eq!(transport.calls(), 3);
    assert_eq!(
        store.sync_counts(SyncService::Edsm, F1).unwrap().sent_total,
        150
    );

    let _ = shut_tx.send(true);
    handle.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn credential_error_holds_rows_until_key_changes() {
    let base = Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap();
    let (store, clock) = setup_worker_store(base);
    push_edsm_row(
        &store,
        "edsm:auth:1",
        "FSDJump",
        base - chrono::Duration::seconds(20),
        None,
    );
    let bad_key =
        std::fs::read_to_string(edsm_fixtures_root().join("get_ranks_bad_key.json")).unwrap();
    let transport = ScriptedTransport::new(vec![
        reply_ex(200, &bad_key),
        reply_ex(200, &ok_events_body(1)),
    ]);
    let (wake, _rw, mut ev_rx, shut_tx, handle) = setup_worker(
        &store,
        clock,
        transport.clone(),
        DiscardList::with_set(fixture_discard_set()),
    );

    let ev = ev_rx.recv().await.unwrap();
    assert_eq!(transport.calls(), 1);
    assert_eq!(store.sync_counts(SyncService::Edsm, F1).unwrap().pending, 1);
    assert!(matches!(
        ev,
        crate::worker::WorkerEvent::Outcome {
            effect: crate::worker::ServiceEffect::CmdrError(tmc_model::sync::ErrorClass::Auth),
            ..
        }
    ));

    // 1 h en espera sin reintentos mientras la clave sea inválida.
    tokio::time::advance(Duration::from_secs(3600)).await;
    tokio::task::yield_now().await;
    assert_eq!(transport.calls(), 1);

    // El piloto actualiza su clave y despierta al trabajador.
    store
        .put_sync_credential(&SyncCredential {
            service: SyncService::Edsm,
            cmdr_fid: F1.into(),
            cmdr_name: "CMDR Ficticio".into(),
            api_key: "EDSM-KEY-NEW".into(),
            created_at: base.to_rfc3339(),
            verified_at: Some(base.to_rfc3339()),
            assignment: KeyAssignment::Confirmed,
        })
        .unwrap();
    wake.notify_one();

    let ev2 = ev_rx.recv().await.unwrap();
    assert_eq!(transport.calls(), 2);
    assert!(matches!(
        ev2,
        crate::worker::WorkerEvent::Outcome {
            effect: crate::worker::ServiceEffect::Connected,
            sent: 1,
            ..
        }
    ));
    let sent_body: Value = serde_json::from_slice(&transport.posts()[1].body).unwrap();
    assert_eq!(sent_body["apiKey"], "EDSM-KEY-NEW");

    let _ = shut_tx.send(true);
    handle.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn software_blocked_stops_service_without_retries() {
    let base = Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap();
    let (store, clock) = setup_worker_store(base);
    push_edsm_row(
        &store,
        "edsm:blk:1",
        "FSDJump",
        base - chrono::Duration::seconds(20),
        None,
    );
    let transport = ScriptedTransport::new(vec![
        reply_ex(200, r#"{"msgnum":204,"msg":"Software banned"}"#),
        reply_ex(200, &ok_events_body(1)),
    ]);
    let (wake, _rw, mut ev_rx, shut_tx, handle) = setup_worker(
        &store,
        clock,
        transport.clone(),
        DiscardList::with_set(fixture_discard_set()),
    );

    let ev = ev_rx.recv().await.unwrap();
    assert_eq!(transport.calls(), 1);
    assert!(matches!(
        ev,
        crate::worker::WorkerEvent::Outcome {
            effect: crate::worker::ServiceEffect::ServiceError {
                class: tmc_model::sync::ErrorClass::SoftwareBlocked,
                until: None,
            },
            ..
        }
    ));

    // Nuevas filas, despertares y avance de reloj no reintentan hasta el siguiente arranque.
    push_edsm_row(&store, "edsm:blk:2", "FSDJump", base, None);
    wake.notify_one();
    tokio::time::advance(Duration::from_secs(3600)).await;
    tokio::task::yield_now().await;
    assert_eq!(transport.calls(), 1);

    let _ = shut_tx.send(true);
    handle.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn msgnum_304_discards_and_refreshes_list() {
    let base = Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap();
    let (store, clock) = setup_worker_store(base);
    let old = base - chrono::Duration::seconds(20);
    push_edsm_row(&store, "edsm:304:1", "NewDiscardableEvent", old, None);
    push_edsm_row(&store, "edsm:304:2", "FSDJump", old, None);

    let transport = ScriptedTransport::new(vec![reply_ex(
        200,
        r#"{"msgnum":100,"msg":"OK","events":[{"msgnum":304,"msg":"Discarded"},{"msgnum":100,"msg":"OK"}]}"#,
    )]);
    let mut updated_list: Vec<String> = fixture_discard_set().into_iter().collect();
    updated_list.push("NewDiscardableEvent".into());
    transport.push_get(Ok(HttpReply {
        status: 200,
        body: serde_json::to_string(&updated_list).unwrap(),
    }));

    let discard = DiscardList::with_set(fixture_discard_set());
    let (_wake, _rw, mut ev_rx, shut_tx, handle) =
        setup_worker(&store, clock, transport.clone(), discard.clone());

    let _ = ev_rx.recv().await.unwrap();
    let counts = store.sync_counts(SyncService::Edsm, F1).unwrap();
    assert_eq!(counts.discarded, 1);
    assert_eq!(counts.sent_total, 1);
    assert_eq!(counts.rejected_total, 0);
    assert_eq!(transport.get_urls(), vec![EDSM_DISCARD_URL.to_string()]);
    assert_eq!(
        discard.status("NewDiscardableEvent"),
        DiscardStatus::Discarded
    );

    let _ = shut_tx.send(true);
    handle.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn no_discard_list_no_send() {
    let base = Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap();
    let (store, clock) = setup_worker_store(base);
    push_edsm_row(
        &store,
        "edsm:nodl:1",
        "FSDJump",
        base - chrono::Duration::seconds(20),
        Some(tmc_model::sync::ErrorClass::DiscardListMissing),
    );
    let transport = ScriptedTransport::new(vec![reply_ex(200, &ok_events_body(1))]);
    let discard = DiscardList::empty();
    let (wake, _rw, mut ev_rx, shut_tx, handle) =
        setup_worker(&store, clock, transport.clone(), discard.clone());

    tokio::time::advance(Duration::from_secs(60)).await;
    tokio::task::yield_now().await;
    assert_eq!(transport.calls(), 0);
    assert_eq!(store.sync_counts(SyncService::Edsm, F1).unwrap().pending, 1);
    let ev = ev_rx.recv().await.unwrap();
    assert!(matches!(
        ev,
        crate::worker::WorkerEvent::Outcome {
            effect: crate::worker::ServiceEffect::ServiceError {
                class: tmc_model::sync::ErrorClass::DiscardListMissing,
                until: None,
            },
            ..
        }
    ));

    // Una vez cargada la lista de descarte, el despertar envía las filas retenidas.
    discard.set(fixture_discard_set());
    wake.notify_one();
    let ev2 = ev_rx.recv().await.unwrap();
    assert_eq!(transport.calls(), 1);
    assert!(matches!(
        ev2,
        crate::worker::WorkerEvent::Outcome {
            effect: crate::worker::ServiceEffect::Connected,
            sent: 1,
            ..
        }
    ));

    let _ = shut_tx.send(true);
    handle.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn per_event_results_applied_individually() {
    let base = Utc.with_ymd_and_hms(2026, 3, 10, 12, 0, 0).unwrap();
    let (store, clock) = setup_worker_store(base);
    let old = base - chrono::Duration::seconds(20);
    for (idx, kind) in ["FSDJump", "CustomOld", "Scan", "Docked"]
        .iter()
        .enumerate()
    {
        push_edsm_row(&store, &format!("edsm:mix:{idx}"), kind, old, None);
    }

    let mixed = std::fs::read_to_string(edsm_fixtures_root().join("response_mixed.json")).unwrap();
    let transport = ScriptedTransport::new(vec![reply_ex(200, &mixed)]);
    transport.push_get(Ok(HttpReply {
        status: 200,
        body: fixture_discard_json(),
    }));

    let (_wake, _rw, mut ev_rx, shut_tx, handle) = setup_worker(
        &store,
        clock,
        transport.clone(),
        DiscardList::with_set(fixture_discard_set()),
    );

    let mut outcome = None;
    let mut rejection_events = 0;
    while let Some(ev) = ev_rx.recv().await {
        match ev {
            crate::worker::WorkerEvent::Rejection { .. } => rejection_events += 1,
            ev @ crate::worker::WorkerEvent::Outcome { .. } => {
                outcome = Some(ev);
                break;
            }
        }
    }
    assert_eq!(rejection_events, 2);
    assert!(matches!(
        outcome,
        Some(crate::worker::WorkerEvent::Outcome {
            effect: crate::worker::ServiceEffect::Connected,
            sent: 1,
            rejected: 2,
            ..
        })
    ));
    let counts = store.sync_counts(SyncService::Edsm, F1).unwrap();
    assert_eq!(counts.sent_total, 1);
    assert_eq!(counts.discarded, 1);
    assert_eq!(counts.rejected_total, 2);
    assert_eq!(counts.pending, 0);

    let states = store
        .query_json(
            "SELECT kind, state, last_code FROM sync_outbox WHERE service='edsm' ORDER BY id",
            &[],
        )
        .unwrap();
    assert_eq!(states[0], serde_json::json!(["FSDJump", "sent", "100"]));
    assert_eq!(
        states[1],
        serde_json::json!(["CustomOld", "discarded", "304"])
    );
    assert_eq!(states[2], serde_json::json!(["Scan", "rejected", "452"]));
    assert_eq!(states[3], serde_json::json!(["Docked", "rejected", "302"]));

    let _ = shut_tx.send(true);
    handle.await.unwrap();
}
