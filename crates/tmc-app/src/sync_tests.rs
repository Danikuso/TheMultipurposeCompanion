//! Tests de composición de sincronización, prueba canario (C8 común) y vaciado al cierre.

use crate::config::ServerSection;
use crate::logging::diagnostics::{Limits, start_writer};
use crate::sync_wiring::{env_gates, env_gates_from, live_signal_for_platform, start_sync};
use crate::{Config, start_with_transport};
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use chrono::{TimeZone, Utc};
use rusqlite::Connection;
use serde_json::Value;
use std::ffi::OsString;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::tempdir;
use tmc_model::events::parse_line;
use tmc_model::source::SourceEvent;
use tmc_model::state::Galaxy;
use tmc_model::sync::{
    FeedOrigin, KeyAssignment, SyncFeed, SyncFeedEvent, SyncReason, SyncService,
};
use tmc_net::testing::NoNetworkTransport;
use tmc_server::{ServerConfig, build_router};
use tmc_store::{Store, SyncCredential};
use tmc_sync::{EnvGates, LiveSignalPolicy, SyncDeps, SystemClock, spawn_sync};
use tokio::sync::mpsc;
use tower::ServiceExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::FilterExt;
use tracing_subscriber::prelude::*;

const CANARY_KEY: &str = "CANARYKEY1234";
const CANARY_FID: &str = "F9999999";
const CANARY_NAME: &str = "CMDR CANARY";

#[derive(Clone)]
struct MemSink(Arc<Mutex<Vec<u8>>>);

impl Write for MemSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn read_diagnostic_rows(diag_dir: &Path) -> Vec<String> {
    let db = diag_dir.join("diagnostics.sqlite");
    if !db.exists() {
        return Vec::new();
    }
    let conn = Connection::open(db).unwrap();
    let mut stmt = conn
        .prepare("SELECT code, context, schema, COALESCE(payload, '') FROM diag_timeline")
        .unwrap();
    stmt.query_map([], |r| {
        Ok(format!(
            "{}|{}|{}|{}",
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?.unwrap_or_default(),
            r.get::<_, String>(3)?,
        ))
    })
    .unwrap()
    .map(Result::unwrap)
    .collect()
}

fn collect_name_paths(val: &Value, path: &str, needle: &str, hits: &mut Vec<String>) {
    match val {
        Value::String(s) => {
            if s.contains(needle) {
                hits.push(path.to_string());
            }
        }
        Value::Array(items) => {
            for (idx, item) in items.iter().enumerate() {
                let next = format!("{path}[{idx}]");
                collect_name_paths(item, &next, needle, hits);
            }
        }
        Value::Object(map) => {
            for (k, v) in map {
                let next = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                collect_name_paths(v, &next, needle, hits);
            }
        }
        _ => {}
    }
}

#[test]
fn sync_canary_common() {
    let jdir = tempdir().unwrap();
    let ddir = tempdir().unwrap();
    let diag_dir = tempdir().unwrap();

    let journal_name = "Journal.2026-11-10T100000.01.log";
    let journal_lines = concat!(
        r#"{"timestamp":"2026-11-10T10:00:00Z","event":"Fileheader","part":1,"language":"English/UK","Odyssey":true,"gameversion":"4.1.0.0","build":"r1"}"#,
        "\n",
        r#"{"timestamp":"2026-11-10T10:00:01Z","event":"Commander","FID":"F9999999","Name":"CMDR CANARY"}"#,
        "\n",
        r#"{"timestamp":"2026-11-10T10:00:02Z","event":"LoadGame","FID":"F9999999","Commander":"CMDR CANARY","Ship":"SideWinder","ShipID":1,"FuelLevel":8.0,"FuelCapacity":8.0,"GameMode":"Solo","Credits":1000,"Loan":0}"#,
        "\n",
        r#"{"timestamp":"2026-11-10T10:00:03Z","event":"Location","StarSystem":"Sol","SystemAddress":10477373803,"StarPos":[0,0,0],"Docked":false}"#,
        "\n",
        r#"{"timestamp":"2026-11-10T10:00:04Z","event":"FSDJump","StarSystem":"Alpha Centauri","SystemAddress":10477373804,"StarPos":[1,1,1],"JumpDist":4.38}"#,
        "\n",
    );
    std::fs::write(jdir.path().join(journal_name), journal_lines).unwrap();

    let sink = MemSink(Arc::new(Mutex::new(Vec::new())));
    let log_buf = sink.0.clone();
    let (diag_layer, diag_guard) =
        start_writer(diag_dir.path().to_owned(), Limits::default()).unwrap();
    let general_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(move || sink.clone())
        .with_filter(
            EnvFilter::new("info").and(tracing_subscriber::filter::filter_fn(|m| {
                m.target() != "tmc::diagnostics"
            })),
        );
    let subscriber =
        tracing_subscriber::registry()
            .with(general_layer)
            .with(
                diag_layer.with_filter(tracing_subscriber::filter::filter_fn(|m| {
                    m.target() == "tmc::diagnostics"
                })),
            );

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let (snapshot, rest_bodies) = tracing::subscriber::with_default(subscriber, || {
        rt.block_on(async {
            let cfg = Config {
                journal_dir: Some(jdir.path().to_path_buf()),
                data_dir: Some(ddir.path().to_path_buf()),
                server: ServerSection {
                    port: 0,
                    ..Default::default()
                },
                online_services: true,
                auto_update_check: false,
                auto_copy_next_system: false,
                ..Default::default()
            };
            let running = start_with_transport(cfg, Arc::new(NoNetworkTransport::default()))
                .await
                .unwrap();

            running
                .store
                .put_sync_credential(&SyncCredential {
                    service: SyncService::Inara,
                    cmdr_fid: CANARY_FID.to_string(),
                    cmdr_name: CANARY_NAME.to_string(),
                    api_key: CANARY_KEY.to_string(),
                    created_at: "2026-11-10T10:00:00Z".to_string(),
                    verified_at: None,
                    assignment: KeyAssignment::Confirmed,
                })
                .unwrap();
            running
                .store
                .set_activation(
                    SyncService::Inara,
                    CANARY_FID,
                    Utc.with_ymd_and_hms(2026, 11, 10, 9, 59, 0).unwrap(),
                )
                .unwrap();

            // Esperar a que termine la relectura inicial antes de inyectar eventos en vivo.
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    let s = running.core.snapshot().await;
                    if s.get("core.location")
                        .and_then(|v| v.get("system"))
                        .and_then(Value::as_str)
                        == Some("Alpha Centauri")
                    {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await
            .unwrap();

            let _ = running
                .core
                .events
                .send(SourceEvent::GameProcess { running: true })
                .await;
            let _ = running
                .core
                .events
                .send(SourceEvent::Journal {
                    file: journal_name.to_string(),
                    line: 6,
                    replaying: false,
                    ev: parse_line(
                        r#"{"timestamp":"2026-11-10T10:00:05Z","event":"FSDJump","StarSystem":"Sirius","SystemAddress":10477373805,"StarPos":[2,2,2],"JumpDist":8.5}"#,
                    )
                    .unwrap(),
                })
                .await;

            let snapshot = tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    let s = running.core.snapshot().await;
                    let has_cmdr = s
                        .get("accounts.status")
                        .and_then(|v| v.get("current_cmdr"))
                        .and_then(|v| v.get("name"))
                        .and_then(Value::as_str)
                        == Some(CANARY_NAME);
                    let has_system = s
                        .get("core.location")
                        .and_then(|v| v.get("system"))
                        .and_then(Value::as_str)
                        == Some("Sirius");
                    if has_cmdr && has_system {
                        break s;
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
            })
            .await
            .unwrap();

            let router = build_router(
                ServerConfig {
                    port: running.server.port,
                    lan: false,
                    token: String::new(),
                    ui_dir: None,
                },
                "127.0.0.1".into(),
                running.core.clone(),
                running.store.clone(),
                running.actions.clone(),
            );

            let mut bodies = Vec::new();
            for (method, uri, body) in [
                ("GET", "/api/health", Body::empty()),
                ("GET", "/api/config", Body::empty()),
                ("GET", "/api/history/events?limit=20", Body::empty()),
                ("GET", "/api/update/status", Body::empty()),
                ("GET", "/api/ui/layouts", Body::empty()),
                (
                    "POST",
                    "/api/actions/accounts.sync_now",
                    Body::from(r#"{}"#),
                ),
            ] {
                let mut req = Request::builder()
                    .method(method)
                    .uri(uri)
                    .header(header::HOST, format!("127.0.0.1:{}", running.server.port))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(body)
                    .unwrap();
                req.extensions_mut().insert(ConnectInfo(
                    "127.0.0.1:55000".parse::<SocketAddr>().unwrap(),
                ));
                let resp = router.clone().oneshot(req).await.unwrap();
                assert_eq!(resp.status(), StatusCode::OK, "endpoint {method} {uri}");
                let bytes = axum::body::to_bytes(resp.into_body(), 1_048_576)
                    .await
                    .unwrap();
                bodies.push((uri.to_string(), String::from_utf8_lossy(&bytes).into_owned()));
            }

            running.shutdown().await;
            (snapshot, bodies)
        })
    });

    drop(diag_guard);

    // 1. Logs generales: ni clave, ni FID, ni nombre.
    let logs = String::from_utf8(log_buf.lock().unwrap().clone()).unwrap();
    for secret in [CANARY_KEY, CANARY_FID, CANARY_NAME] {
        assert!(
            !logs.contains(secret),
            "secreto `{secret}` filtrado en logs generales:\n{logs}"
        );
    }

    // 2. Filas de diagnóstico: ni clave, ni FID, ni nombre.
    let diag_rows = read_diagnostic_rows(diag_dir.path());
    assert!(
        !diag_rows.is_empty(),
        "se esperaban filas de diagnóstico capturadas en la prueba canario"
    );
    assert!(
        diag_rows
            .iter()
            .any(|r| r.starts_with("sync.runtime.started|")),
        "se esperaba evento diagnóstico sync.runtime.started: {diag_rows:?}"
    );
    for row in &diag_rows {
        for secret in [CANARY_KEY, CANARY_FID, CANARY_NAME] {
            assert!(
                !row.contains(secret),
                "secreto `{secret}` filtrado en fila de diagnóstico: {row}"
            );
        }
    }

    // 3. Snapshot de topics:
    // - La clave nunca aparece en ningún topic.
    // - En `accounts.status` (topic de sincronización, DR-21) nunca aparece el FID (usa `cmdr_ref` opaco),
    //   y el nombre solo aparece en `current_cmdr.name` y `commanders[].name` (PD-6).
    let snap_val = Value::Object(snapshot.into_iter().collect());
    let mut hits = Vec::new();
    collect_name_paths(&snap_val, "", CANARY_KEY, &mut hits);
    assert!(hits.is_empty(), "clave presente en snapshot: {hits:?}");

    collect_name_paths(&snap_val, "", CANARY_FID, &mut hits);
    let non_core_fid_hits: Vec<_> = hits
        .iter()
        .filter(|p| p.as_str() != "core.cmdr.fid")
        .cloned()
        .collect();
    assert!(
        non_core_fid_hits.is_empty(),
        "FID presente en topics fuera de core.cmdr: {non_core_fid_hits:?}"
    );
    hits.clear();

    collect_name_paths(&snap_val, "", CANARY_NAME, &mut hits);
    let sync_name_hits: Vec<_> = hits
        .iter()
        .filter(|p| p.as_str() != "core.cmdr.name")
        .cloned()
        .collect();
    assert!(
        !sync_name_hits.is_empty(),
        "se esperaba el nombre del comandante en accounts.status"
    );
    for path in &sync_name_hits {
        let allowed = path == "accounts.status.current_cmdr.name"
            || (path.starts_with("accounts.status.commanders[") && path.ends_with("].name"));
        assert!(
            allowed,
            "nombre filtrado fuera de las rutas permitidas de accounts.status: {path}"
        );
    }

    // 4. Respuestas REST de /api:
    // - La clave nunca aparece en ningún endpoint.
    // - El FID nunca aparece fuera del volcado bruto del histórico de diario (`/api/history/events`).
    for (uri, body) in &rest_bodies {
        assert!(
            !body.contains(CANARY_KEY),
            "clave filtrada en respuesta REST {uri}: {body}"
        );
        if !uri.starts_with("/api/history/events") {
            assert!(
                !body.contains(CANARY_FID),
                "FID filtrado en respuesta REST {uri}: {body}"
            );
        }
    }
}

#[tokio::test]
async fn sync_disabled_when_e2e_env_set() {
    let gates = env_gates_from(|k| (k == "TMC_E2E_GAME_RUNNING").then(|| OsString::from("1")));
    assert!(gates.e2e_simulation);
    assert!(!env_gates_from(|_| None).e2e_simulation);
    assert_eq!(
        env_gates().e2e_simulation,
        std::env::var_os("TMC_E2E_GAME_RUNNING").is_some()
    );

    let jdir = tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let (core_tx, mut core_rx) = mpsc::channel(16);
    let online = Arc::new(AtomicBool::new(true));
    let handle = spawn_sync(SyncDeps {
        store: store.clone(),
        transport: Arc::new(NoNetworkTransport::default()),
        online,
        core_events: core_tx,
        journal_dir: jdir.path().to_path_buf(),
        env: EnvGates {
            e2e_simulation: true,
        },
        live_signal: LiveSignalPolicy::JournalActivity,
        clock: Arc::new(SystemClock),
        process_kind: "desktop",
        clients: Vec::new(),
        mappers: Vec::new(),
    });

    let report = tokio::time::timeout(Duration::from_secs(2), core_rx.recv())
        .await
        .unwrap()
        .unwrap();
    match report {
        SourceEvent::SyncReport(boxed) => match *boxed {
            tmc_model::sync::SyncReport::Inara(s) => {
                assert_eq!(s.status.reason, Some(SyncReason::Replay));
            }
            other => panic!("primer reporte inesperado: {other:?}"),
        },
        other => panic!("primer evento inesperado: {other:?}"),
    }
    handle.shutdown().await;
    assert!(store.known_cmdrs().unwrap().is_empty());
}

#[tokio::test]
async fn shutdown_flushes_router_batch() {
    let jdir = tempdir().unwrap();
    let store = Store::open_in_memory().unwrap();
    let (core_tx, _core_rx) = mpsc::channel(32);
    let online = Arc::new(AtomicBool::new(true));
    let transport: Arc<dyn tmc_net::Transport> = Arc::new(NoNetworkTransport::default());
    let file = "Journal.2026-11-10T100000.01.log".to_string();

    let handle = start_sync(&store, transport, online, core_tx, jdir.path(), "desktop");

    handle
        .feed
        .send(SyncFeed {
            origin: FeedOrigin::Live,
            galaxy: Galaxy::Live,
            game_running: true,
            event: SyncFeedEvent::GameProcess { running: true },
        })
        .unwrap();

    for (line, raw) in [
        (
            1u64,
            r#"{"timestamp":"2026-11-10T10:00:00Z","event":"Fileheader","part":1,"language":"English/UK","Odyssey":true,"gameversion":"4.1.0.0","build":"r1"}"#,
        ),
        (
            2u64,
            r#"{"timestamp":"2026-11-10T10:00:01Z","event":"Commander","FID":"F1234567","Name":"CMDR BATCH"}"#,
        ),
        (
            3u64,
            r#"{"timestamp":"2026-11-10T10:00:02Z","event":"Location","StarSystem":"Sol","SystemAddress":10477373803,"StarPos":[0,0,0],"Docked":false}"#,
        ),
    ] {
        handle
            .feed
            .send(SyncFeed {
                origin: FeedOrigin::Live,
                galaxy: Galaxy::Live,
                game_running: true,
                event: SyncFeedEvent::Journal {
                    file: file.clone(),
                    line,
                    ev: parse_line(raw).unwrap(),
                },
            })
            .unwrap();
    }

    // Cerrar inmediatamente antes de que venza el lote de 250 ms.
    let started = std::time::Instant::now();
    handle.shutdown().await;
    assert!(
        started.elapsed() < Duration::from_millis(250),
        "shutdown tardó demasiado ({:?})",
        started.elapsed()
    );
    let _ = live_signal_for_platform();
    let _ = LiveSignalPolicy::Unavailable;
    assert_eq!(store.sync_cursor("F1234567", &file).unwrap(), Some(3));
}

#[test]
fn sync_canary_eddn() {
    use crate::sync_wiring::eddn_schema_mode_from;
    use flate2::read::GzDecoder;
    use std::io::Read;
    use tmc_net::HttpReplyEx;
    use tmc_net::eddn_upload::SchemaMode;
    use tmc_net::testing::ScriptedTransport;

    assert_eq!(
        eddn_schema_mode_from(|k| (k == "TMC_EDDN_TEST_SCHEMAS").then(|| OsString::from("1"))),
        SchemaMode::Test
    );
    assert_eq!(eddn_schema_mode_from(|_| None), SchemaMode::Live);

    let jdir = tempdir().unwrap();
    let ddir = tempdir().unwrap();
    let now = Utc::now();
    let t_hdr =
        (now - chrono::Duration::seconds(5)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let t_cmdr =
        (now - chrono::Duration::seconds(4)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let t_load =
        (now - chrono::Duration::seconds(3)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let t_jump =
        (now + chrono::Duration::seconds(2)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let journal_name = "Journal.2026-11-10T100000.01.log";
    let journal_lines = format!(
        concat!(
            r#"{{"timestamp":"{t_hdr}","event":"Fileheader","part":1,"language":"English/UK","Odyssey":true,"gameversion":"4.1.0.0","build":"r1"}}"#,
            "\n",
            r#"{{"timestamp":"{t_cmdr}","event":"Commander","FID":"{fid}","Name":"{name}"}}"#,
            "\n",
            r#"{{"timestamp":"{t_load}","event":"LoadGame","FID":"{fid}","Commander":"{name}","Ship":"SideWinder","ShipID":1,"Horizons":true,"Odyssey":true,"FuelLevel":8.0,"FuelCapacity":8.0,"GameMode":"Solo","Credits":1000,"Loan":0}}"#,
            "\n"
        ),
        t_hdr = t_hdr,
        t_cmdr = t_cmdr,
        t_load = t_load,
        fid = CANARY_FID,
        name = CANARY_NAME,
    );
    std::fs::write(jdir.path().join(journal_name), journal_lines).unwrap();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let transport = ScriptedTransport::new(vec![Ok(HttpReplyEx {
        status: 200,
        body: "OK".to_string(),
        retry_after_s: None,
        rate_limit_remaining: None,
        rate_limit_reset_s: None,
    })]);

    let (posts, snapshot) = rt.block_on(async {
        let cfg = Config {
            journal_dir: Some(jdir.path().to_path_buf()),
            data_dir: Some(ddir.path().to_path_buf()),
            server: ServerSection {
                port: 0,
                ..Default::default()
            },
            online_services: true,
            auto_update_check: false,
            auto_copy_next_system: false,
            ..Default::default()
        };
        let running = start_with_transport(cfg, transport.clone())
            .await
            .unwrap();

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let s = running.core.snapshot().await;
                if s.get("core.cmdr")
                    .and_then(|v| v.get("fid"))
                    .and_then(Value::as_str)
                    == Some(CANARY_FID)
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();

        let router = build_router(
            ServerConfig {
                port: running.server.port,
                lan: false,
                token: String::new(),
                ui_dir: None,
            },
            "127.0.0.1".into(),
            running.core.clone(),
            running.store.clone(),
            running.actions.clone(),
        );

        let mut req_on = Request::builder()
            .method("POST")
            .uri("/api/actions/accounts.eddn.set_enabled")
            .header(header::HOST, format!("127.0.0.1:{}", running.server.port))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"enabled":true}"#))
            .unwrap();
        req_on
            .extensions_mut()
            .insert(ConnectInfo("127.0.0.1:55001".parse::<SocketAddr>().unwrap()));
        let resp_on = router.clone().oneshot(req_on).await.unwrap();
        assert_eq!(resp_on.status(), StatusCode::OK);

        let _ = running
            .core
            .events
            .send(SourceEvent::GameProcess { running: true })
            .await;
        let _ = running
            .core
            .events
            .send(SourceEvent::Journal {
                file: journal_name.to_string(),
                line: 4,
                replaying: false,
                ev: parse_line(&format!(
                    r#"{{"timestamp":"{t_jump}","event":"FSDJump","StarSystem":"Sirius","SystemAddress":10477373805,"StarPos":[6.25,1.25,-1.25],"JumpDist":8.5}}"#
                ))
                .unwrap(),
            })
            .await;

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if !transport.posts().is_empty() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();

        let snap = running.core.snapshot().await;

        let mut req_off = Request::builder()
            .method("POST")
            .uri("/api/actions/accounts.eddn.set_enabled")
            .header(header::HOST, format!("127.0.0.1:{}", running.server.port))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"enabled":false}"#))
            .unwrap();
        req_off
            .extensions_mut()
            .insert(ConnectInfo("127.0.0.1:55001".parse::<SocketAddr>().unwrap()));
        let resp_off = router.oneshot(req_off).await.unwrap();
        assert_eq!(resp_off.status(), StatusCode::OK);
        assert!(running.store.kv_get("sync.eddn.enabled_at").is_none());

        running.shutdown().await;
        (transport.posts(), snap)
    });

    assert_eq!(posts.len(), 1);
    let raw = &posts[0].body;
    let body_text = if raw.starts_with(&[0x1f, 0x8b]) {
        let mut out = String::new();
        GzDecoder::new(raw.as_slice())
            .read_to_string(&mut out)
            .unwrap();
        out
    } else {
        String::from_utf8(raw.clone()).unwrap()
    };

    for secret in [CANARY_NAME, CANARY_FID, CANARY_KEY] {
        assert!(
            !body_text.contains(secret),
            "secreto `{secret}` filtrado en cuerpo EDDN: {body_text}"
        );
    }

    let payload: Value = serde_json::from_str(&body_text).unwrap();
    assert_eq!(
        payload["$schemaRef"],
        "https://eddn.edcd.io/schemas/journal/1"
    );
    assert_eq!(
        payload["header"]["softwareName"],
        tmc_net::eddn_upload::EDDN_SOFTWARE_NAME
    );
    assert!(snapshot.contains_key("accounts.status"));
}
