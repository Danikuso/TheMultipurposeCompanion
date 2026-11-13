//! Pruebas de credenciales por comandante EDSM, retirada de `edsm.*` y canario C8 (E2-4).

use crate::{AppActions, Config, ServerSection, start_custom};
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use chrono::Utc;
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use tempfile::tempdir;
use tmc_core::CoreMessage;
use tmc_model::diagnostics::capture::capture;
use tmc_model::state::Galaxy;
use tmc_model::sync::{SyncFeed, SyncFeedEvent, SyncReason, SyncService, SyncState};
use tmc_net::edsm_journal::{EDSM_DISCARD_URL, EDSM_JOURNAL_URL, get_ranks_url};
use tmc_net::testing::ScriptedTransport;
use tmc_net::transport::{HttpReply, HttpReplyEx, TransportError};
use tmc_server::{ActionHost, ServerConfig, build_router};
use tmc_store::Store;
use tower::ServiceExt;

fn edsm_fixture(name: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/sync/edsm")
            .join(name),
    )
    .expect("edsm fixture")
}

async fn post_action(router: &axum::Router, action_name: &str, body: Value) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method("POST")
        .uri(format!("/api/actions/{action_name}"))
        .header(header::HOST, "127.0.0.1:47320")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    req.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:55000".parse::<SocketAddr>().unwrap(),
    ));
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json = serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null);
    (status, json)
}

#[tokio::test]
async fn key_save_rejected_does_not_activate() {
    let dir = tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let now = Utc::now();
    store
        .upsert_known_cmdr("F123456", "CMDR CANARY", now)
        .unwrap();
    let salt = store.get_or_create_eddn_salt().unwrap();
    let cmdr_ref = tmc_sync::cmdr_ref(&salt, "F123456");

    let transport = ScriptedTransport::new(Vec::new());
    transport.push_get(Ok(HttpReply {
        status: 200,
        body: edsm_fixture("get_ranks_bad_key.json"),
    }));

    let core = tmc_core::spawn_core(vec![], None, Duration::from_millis(100));
    let sync = tmc_sync::spawn_sync(tmc_sync::SyncDeps {
        store: store.clone(),
        transport: transport.clone(),
        online: Arc::new(AtomicBool::new(true)),
        core_events: core.events.clone(),
        journal_dir: dir.path().to_path_buf(),
        env: tmc_sync::EnvGates::default(),
        live_signal: tmc_sync::LiveSignalPolicy::JournalActivity,
        clock: Arc::new(tmc_sync::SystemClock),
        process_kind: "desktop",
        clients: Vec::new(),
        mappers: Vec::new(),
    });
    let actions = Arc::new(
        AppActions::new()
            .with_store(store.clone())
            .with_transport(transport.clone())
            .with_sync_waker(sync.waker()),
    );
    let router = build_router(
        ServerConfig {
            port: 47320,
            lan: false,
            token: String::new(),
            ui_dir: None,
        },
        "127.0.0.1".into(),
        core,
        store.clone(),
        actions.clone(),
    );

    let (status, body) = post_action(
        &router,
        "accounts.key.save",
        json!({
            "service": "edsm",
            "cmdr_ref": cmdr_ref,
            "api_key": "BAD-CANARY-SECRET"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], false);
    assert_eq!(body["verified"], false);
    assert_eq!(body["state"], "rejected");
    assert!(
        store
            .sync_credential(SyncService::Edsm, "F123456")
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .activation(SyncService::Edsm, "F123456")
            .unwrap()
            .is_none()
    );
    assert!(!transport.get_urls().is_empty());

    sync.shutdown().await;
}

#[tokio::test]
async fn key_save_unreachable_saves_with_error_state() {
    let dir = tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let now = Utc::now();
    store
        .upsert_known_cmdr("F123456", "CMDR RETRY", now)
        .unwrap();
    let salt = store.get_or_create_eddn_salt().unwrap();
    let cmdr_ref = tmc_sync::cmdr_ref(&salt, "F123456");

    let transport = ScriptedTransport::new(Vec::new());
    // 1er intento al guardar: red inalcanzable; 2º intento (reintento backoff): clave válida.
    transport.push_get(Err(TransportError::Connect("unreachable".into())));
    transport.push_get(Ok(HttpReply {
        status: 200,
        body: edsm_fixture("get_ranks_ok.json"),
    }));

    let core = tmc_core::spawn_core(tmc_modules::all_modules(), None, Duration::from_millis(100));
    let mut rx = core.subscribe();
    let sync = tmc_sync::spawn_sync(tmc_sync::SyncDeps {
        store: store.clone(),
        transport: transport.clone(),
        online: Arc::new(AtomicBool::new(true)),
        core_events: core.events.clone(),
        journal_dir: dir.path().to_path_buf(),
        env: tmc_sync::EnvGates::default(),
        live_signal: tmc_sync::LiveSignalPolicy::JournalActivity,
        clock: Arc::new(tmc_sync::SystemClock),
        process_kind: "desktop",
        clients: Vec::new(),
        mappers: Vec::new(),
    });
    // Fija galaxia Live + comandante activo en tracker para que accounts.status muestre su estado.
    sync.feed
        .send(SyncFeed {
            event: SyncFeedEvent::Journal {
                ev: tmc_model::events::parse_line(
                    r#"{"timestamp":"2026-11-10T10:00:59Z","event":"Fileheader","part":1,"language":"English/UK","Odyssey":true,"gameversion":"4.1.0.0","build":"r999999/r0 "}"#,
                )
                .unwrap(),
                file: "Journal.2026-11-10T100000.01.log".into(),
                line: 1,
            },
            origin: tmc_model::sync::FeedOrigin::Live,
            galaxy: Galaxy::Live,
            game_running: true,
        })
        .unwrap();
    sync.feed
        .send(SyncFeed {
            event: SyncFeedEvent::Journal {
                ev: tmc_model::events::parse_line(
                    r#"{"timestamp":"2026-11-10T10:01:00Z","event":"LoadGame","FID":"F123456","Commander":"CMDR RETRY","Horizons":true,"Odyssey":true}"#,
                )
                .unwrap(),
                file: "Journal.2026-11-10T100000.01.log".into(),
                line: 2,
            },
            origin: tmc_model::sync::FeedOrigin::Live,
            galaxy: Galaxy::Live,
            game_running: true,
        })
        .unwrap();
    sync.feed
        .send(SyncFeed {
            event: SyncFeedEvent::CatchUpDone,
            origin: tmc_model::sync::FeedOrigin::Live,
            galaxy: Galaxy::Live,
            game_running: true,
        })
        .unwrap();

    let actions = Arc::new(
        AppActions::new()
            .with_store(store.clone())
            .with_transport(transport.clone())
            .with_sync_waker(sync.waker()),
    );
    let router = build_router(
        ServerConfig {
            port: 47320,
            lan: false,
            token: String::new(),
            ui_dir: None,
        },
        "127.0.0.1".into(),
        core,
        store.clone(),
        actions.clone(),
    );

    let (status, body) = post_action(
        &router,
        "accounts.key.save",
        json!({
            "service": "edsm",
            "cmdr_ref": cmdr_ref,
            "api_key": "RETRY-EDSM-KEY"
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(body["verified"], false);
    assert_eq!(body["state"], "saved_unverified");

    let cred = store
        .sync_credential(SyncService::Edsm, "F123456")
        .unwrap()
        .expect("saved credential");
    assert_eq!(cred.api_key, "RETRY-EDSM-KEY");
    assert!(cred.verified_at.is_none());

    // Topic accounts.status debe pasar a estado error/network mientras no esté verificada.
    let mut saw_error_state = false;
    let mut last_edsm = Value::Null;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout_at(deadline, rx.recv()).await {
            Ok(Ok(CoreMessage::Update { topics: snap, .. })) => {
                if let Some(acc) = snap.get("accounts.status") {
                    last_edsm = acc["edsm"].clone();
                    if acc["edsm"]["state"] == SyncState::Error.as_str_json()
                        && acc["edsm"]["reason"] == json!(SyncReason::Network)
                        && acc["edsm"]["has_key"] == true
                    {
                        saw_error_state = true;
                        break;
                    }
                }
            }
            Ok(Ok(_)) => {}
            _ => break,
        }
    }
    assert!(
        saw_error_state,
        "expected accounts.status.edsm in error/network state; last seen: {last_edsm}"
    );

    sync.shutdown().await;
}

trait SyncStateJsonExt {
    fn as_str_json(&self) -> Value;
}
impl SyncStateJsonExt for SyncState {
    fn as_str_json(&self) -> Value {
        serde_json::to_value(self).unwrap()
    }
}

#[tokio::test]
async fn key_actions_never_echo_api_key() {
    let dir = tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let now = Utc::now();
    store
        .upsert_known_cmdr("F999CANARY", "CMDR SECRET", now)
        .unwrap();
    let salt = store.get_or_create_eddn_salt().unwrap();
    let cmdr_ref = tmc_sync::cmdr_ref(&salt, "F999CANARY");

    let transport = ScriptedTransport::new(Vec::new());
    transport.push_get(Ok(HttpReply {
        status: 200,
        body: edsm_fixture("get_ranks_ok.json"),
    }));
    let core = tmc_core::spawn_core(vec![], None, Duration::from_millis(100));
    let actions = Arc::new(
        AppActions::new()
            .with_store(store.clone())
            .with_transport(transport.clone()),
    );
    let router = build_router(
        ServerConfig {
            port: 47320,
            lan: false,
            token: String::new(),
            ui_dir: None,
        },
        "127.0.0.1".into(),
        core,
        store.clone(),
        actions.clone(),
    );

    let secret = "EDSM_SUPER_SECRET_CANARY_4242";
    let actions_for_capture = Arc::clone(&actions);
    let cmdr_ref_for_capture = cmdr_ref.clone();
    let (save_body, records) = std::thread::spawn(move || {
        capture(|| {
            actions_for_capture
                .run_action_with(
                    "accounts.key.save",
                    &json!({
                        "service": "edsm",
                        "cmdr_ref": cmdr_ref_for_capture,
                        "api_key": secret
                    }),
                    &Default::default(),
                )
                .unwrap()
        })
    })
    .join()
    .unwrap();
    for r in &records {
        let s = format!("{r:?}");
        assert!(!s.contains(secret), "diagnostics leaked api_key: {s}");
        assert!(!s.contains("F999CANARY"), "diagnostics leaked FID: {s}");
    }
    assert!(!save_body.to_string().contains(secret));
    assert!(!save_body.to_string().contains("F999CANARY"));

    let (_st2, conf_body) = post_action(
        &router,
        "accounts.key.confirm",
        json!({ "service": "edsm", "cmdr_ref": cmdr_ref }),
    )
    .await;
    assert!(!conf_body.to_string().contains(secret));

    let (_st3, del_body) = post_action(
        &router,
        "accounts.key.delete",
        json!({ "service": "edsm", "cmdr_ref": cmdr_ref }),
    )
    .await;
    assert_eq!(del_body["ok"], true);
    assert!(!del_body.to_string().contains(secret));

    let (_st4, forget_body) = post_action(
        &router,
        "accounts.cmdr.forget",
        json!({ "cmdr_ref": cmdr_ref }),
    )
    .await;
    assert_eq!(forget_body["ok"], true);
    assert!(!forget_body.to_string().contains("F999CANARY"));
    let _ = dir;
}

#[tokio::test]
async fn module_prefixes_without_edsm() {
    let actions = AppActions::new();
    let err = actions
        .run_action_with(
            "edsm.save_key",
            &json!({ "api_key": "abc" }),
            &Default::default(),
        )
        .unwrap_err();
    assert!(matches!(err, tmc_server::ActionError::NotFound));
    let err2 = actions
        .run_action_with("edsm.clear_key", &json!({}), &Default::default())
        .unwrap_err();
    assert!(matches!(err2, tmc_server::ActionError::NotFound));
    assert!(!crate::clipboard::has_module_prefix("edsm."));
}

#[tokio::test]
async fn sync_canary_edsm() {
    let dir = tempdir().unwrap();
    let journal_dir = dir.path().join("journals");
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&journal_dir).unwrap();
    std::fs::create_dir_all(&data_dir).unwrap();

    let log_path = journal_dir.join("Journal.2026-11-12T100000.01.log");
    std::fs::write(
        &log_path,
        concat!(
            "{\"timestamp\":\"2026-11-12T10:00:00Z\",\"event\":\"Fileheader\",\"part\":1,\"language\":\"English/UK\",\"Odyssey\":true,\"gameversion\":\"4.1.0.0\",\"build\":\"r999999/r0 \"}\n",
            "{\"timestamp\":\"2026-11-12T10:00:01Z\",\"event\":\"Commander\",\"FID\":\"F999CANARY\",\"Name\":\"CMDR CANARY\"}\n",
            "{\"timestamp\":\"2026-11-12T10:00:02Z\",\"event\":\"LoadGame\",\"FID\":\"F999CANARY\",\"Commander\":\"CMDR CANARY\",\"Horizons\":true,\"Odyssey\":true,\"Ship\":\"python\",\"ShipID\":7}\n",
        ),
    )
    .unwrap();

    let transport = ScriptedTransport::new(vec![Ok(HttpReplyEx {
        status: 200,
        body: edsm_fixture("response_ok.json"),
        retry_after_s: None,
        rate_limit_remaining: None,
        rate_limit_reset_s: None,
    })]);
    // Respuestas enrutadas por prefijo URL para que consultas de fondo (/ticks) no roben /discard o get-ranks.
    let get_ranks_sample = get_ranks_url("x", "y");
    let get_ranks_prefix = get_ranks_sample.split('?').next().unwrap();
    transport.on_get(
        EDSM_DISCARD_URL,
        Ok(HttpReply {
            status: 200,
            body: edsm_fixture("discard.json"),
        }),
    );
    transport.on_get(
        get_ranks_prefix,
        Ok(HttpReply {
            status: 200,
            body: edsm_fixture("get_ranks_ok.json"),
        }),
    );

    let cfg = Config {
        journal_dir: Some(journal_dir.clone()),
        data_dir: Some(data_dir),
        server: ServerSection {
            port: 0,
            lan: false,
            lan_token: None,
        },
        online_services: true,
        auto_update_check: false,
        ..Default::default()
    };

    let (_, records) = capture(|| ());
    drop(records);

    let running = start_custom(cfg, transport.clone(), None)
        .await
        .expect("start_custom");
    let mut rx = running.core.subscribe();

    // Espera a que el router registre a F999CANARY y /discard haya sido descargado antes de guardar clave.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline {
        let cmdr_ready = running
            .store
            .known_cmdrs()
            .is_ok_and(|list| list.iter().any(|(fid, _)| fid == "F999CANARY"));
        let discard_fetched = transport.get_urls().iter().any(|u| u == EDSM_DISCARD_URL);
        if cmdr_ready && discard_fetched {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    assert!(
        transport.get_urls().iter().any(|u| u == EDSM_DISCARD_URL),
        "discard not fetched before key save; get_urls={:?}",
        transport.get_urls()
    );

    let salt = running.store.get_or_create_eddn_salt().unwrap();
    let cmdr_ref = tmc_sync::cmdr_ref(&salt, "F999CANARY");
    let actions_cap = Arc::clone(&running.actions);
    let cmdr_ref_cap = cmdr_ref.clone();
    let (res, cap_records) = std::thread::spawn(move || {
        capture(|| {
            actions_cap
                .run_action_with(
                    "accounts.key.save",
                    &json!({
                        "service": "edsm",
                        "cmdr_ref": cmdr_ref_cap,
                        "api_key": "CANARY_EDSM_SECRET_9876"
                    }),
                    &Default::default(),
                )
                .unwrap()
        })
    })
    .join()
    .unwrap();
    for r in &cap_records {
        let s = format!("{r:?}");
        assert!(
            !s.contains("CANARY_EDSM_SECRET_9876"),
            "diagnostics leaked secret: {s}"
        );
        assert!(!s.contains("F999CANARY"), "diagnostics leaked FID: {s}");
    }
    assert_eq!(
        res["verified"],
        true,
        "key save response={res}, get_urls={:?}",
        transport.get_urls()
    );

    // Marcamos CatchUpDone + proceso en ejecución y enviamos el FSDJump por core.events igual que sync_canary_eddn.
    let _ = running.sync.feed.send(SyncFeed {
        origin: tmc_model::sync::FeedOrigin::Live,
        galaxy: Galaxy::Live,
        game_running: true,
        event: SyncFeedEvent::CatchUpDone,
    });
    let _ = running
        .core
        .events
        .send(tmc_model::source::SourceEvent::GameProcess { running: true })
        .await;
    let jump_ts = (Utc::now() + chrono::Duration::seconds(1))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let jump_line = format!(
        r#"{{"timestamp":"{jump_ts}","event":"FSDJump","StarSystem":"CanarySystem","SystemAddress":9990001,"StarPos":[1.0,2.0,3.0],"JumpDist":12.5,"FuelUsed":1.2,"FuelLevel":14.8}}"#
    );
    let _ = running
        .core
        .events
        .send(tmc_model::source::SourceEvent::Journal {
            file: "Journal.2026-11-12T100000.01.log".into(),
            line: 4,
            replaying: false,
            ev: tmc_model::events::parse_line(&jump_line).unwrap(),
        })
        .await;

    // En Windows (`ProcessCheck`), el monitor real de proceso emite `running: false` cada pocos segundos
    // si EliteDangerous64.exe no está abierto; mantenemos viva la señal `running: true` mientras EDSM agrupa (`flush_after`).
    let send_deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    while tokio::time::Instant::now() < send_deadline {
        if transport.posts().iter().any(|p| p.url == EDSM_JOURNAL_URL) {
            break;
        }
        let _ = running
            .core
            .events
            .send(tmc_model::source::SourceEvent::GameProcess { running: true })
            .await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let snap = running.core.snapshot().await;
    let posts = transport.posts();
    let edsm_post = posts
        .iter()
        .find(|p| p.url == EDSM_JOURNAL_URL)
        .unwrap_or_else(|| {
            panic!(
                "expected EDSM journal POST; accounts.status={:?}, posts={:?}",
                snap.get("accounts.status"),
                posts.iter().map(|p| &p.url).collect::<Vec<_>>()
            )
        });
    let req_str = String::from_utf8_lossy(&edsm_post.body);
    let req_json: Value = serde_json::from_str(&req_str).unwrap();
    assert_eq!(req_json["commanderName"], "CMDR CANARY");
    assert_eq!(req_json["apiKey"], "CANARY_EDSM_SECRET_9876");
    assert!(
        !req_str.contains("F999CANARY"),
        "FID must never be sent to EDSM"
    );
    assert!(
        transport.get_urls().iter().any(|u| u == EDSM_DISCARD_URL),
        "expected /discard GET"
    );

    // Verificar topic accounts.status (clave y FID nunca salen).
    let mut latest_accounts = Value::Null;
    while let Ok(msg) = rx.try_recv() {
        if let CoreMessage::Update { topics: s, .. } = msg
            && let Some(acc) = s.get("accounts.status")
        {
            latest_accounts = acc.clone();
        }
    }
    let acc_str = latest_accounts.to_string();
    assert!(!acc_str.contains("CANARY_EDSM_SECRET_9876"));
    assert!(!acc_str.contains("F999CANARY"));

    running.shutdown().await;
}
