//! Servidor HTTP y WebSocket para estáticos, protocolo en vivo y emparejamiento LAN.
#![forbid(unsafe_code)]

pub mod assets;
pub mod pairing;
pub mod rest;
pub mod security;
pub mod ws;

use axum::Router;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use tmc_core::{CoreHandle, Store};
use tokio::sync::oneshot;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActionError {
    #[error("action not found")]
    NotFound,
    #[error("action failed: {0}")]
    Failed(String),
}

pub trait ActionHost: Send + Sync + 'static {
    fn run_action(
        &self,
        name: &str,
        snapshot: &BTreeMap<String, Value>,
    ) -> Result<Value, ActionError>;
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub lan: bool,
    pub token: String,
    pub ui_dir: Option<PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 47320,
            lan: false,
            token: String::new(),
            ui_dir: None,
        }
    }
}

pub struct ServerHandle {
    pub local_url: String,
    pub lan_url: Option<String>,
    pub port: u16,
    shutdown: oneshot::Sender<()>,
}

impl ServerHandle {
    pub fn shutdown(self) {
        let _ = self.shutdown.send(());
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("no available port in range {start}..={end}")]
    NoAvailablePort { start: u16, end: u16 },
}

#[derive(Clone)]
pub struct AppState {
    pub cfg: ServerConfig,
    pub port: u16,
    pub lan_ip: String,
    pub core: CoreHandle,
    pub store: Store,
    pub host: Arc<dyn ActionHost>,
}

pub fn detect_lan_ip() -> String {
    if let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0")
        && sock.connect("192.0.2.1:80").is_ok()
        && let Ok(local) = sock.local_addr()
        && !local.ip().is_loopback()
        && !local.ip().is_unspecified()
    {
        return local.ip().to_string();
    }
    "127.0.0.1".to_owned()
}

pub fn build_router(
    cfg: ServerConfig,
    lan_ip: String,
    core: CoreHandle,
    store: Store,
    host: Arc<dyn ActionHost>,
) -> Router {
    let port = cfg.port;
    let state = AppState {
        cfg,
        port,
        lan_ip,
        core,
        store,
        host,
    };
    Router::new()
        .route("/ws", axum::routing::get(ws::ws_handler))
        .route("/api/health", axum::routing::get(rest::get_health))
        .route(
            "/api/history/events",
            axum::routing::get(rest::get_history_events),
        )
        .route(
            "/api/actions/{name}",
            axum::routing::post(rest::post_action),
        )
        .route("/api/pairing", axum::routing::get(pairing::get_pairing))
        .fallback(axum::routing::get(assets::serve_asset))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            security::enforce_security,
        ))
        .with_state(state)
}

async fn bind_with_fallback(
    bind_ip: &str,
    start_port: u16,
) -> Result<(tokio::net::TcpListener, u16), ServerError> {
    if start_port == 0 {
        let listener = tokio::net::TcpListener::bind(format!("{bind_ip}:0")).await?;
        let actual_port = listener.local_addr()?.port();
        return Ok((listener, actual_port));
    }
    let end_port = start_port.saturating_add(20);
    for port in start_port..=end_port {
        if let Ok(listener) = tokio::net::TcpListener::bind(format!("{bind_ip}:{port}")).await {
            let actual_port = listener.local_addr()?.port();
            return Ok((listener, actual_port));
        }
    }
    Err(ServerError::NoAvailablePort {
        start: start_port,
        end: end_port,
    })
}

pub async fn serve(
    mut cfg: ServerConfig,
    core: CoreHandle,
    store: Store,
    host: Arc<dyn ActionHost>,
) -> Result<ServerHandle, ServerError> {
    let bind_ip = if cfg.lan { "0.0.0.0" } else { "127.0.0.1" };
    let (listener, actual_port) = bind_with_fallback(bind_ip, cfg.port).await?;
    cfg.port = actual_port;
    let lan_ip = detect_lan_ip();
    let local_url = format!("http://127.0.0.1:{actual_port}/");
    let lan_url = cfg
        .lan
        .then(|| format!("http://{lan_ip}:{actual_port}/?token={}", cfg.token));

    let router = build_router(cfg, lan_ip, core, store, host);
    let make_svc = router.into_make_service_with_connect_info::<std::net::SocketAddr>();
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        let _ = axum::serve(listener, make_svc)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await;
    });

    Ok(ServerHandle {
        local_url,
        lan_url,
        port: actual_port,
        shutdown: shutdown_tx,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::ConnectInfo;
    use axum::http::{Request, StatusCode, header};
    use futures_util::{SinkExt, StreamExt};
    use std::net::SocketAddr;
    use std::time::Duration;
    use tmc_core::{EventWriter, StoredEvent, spawn_core};
    use tmc_model::{events::parse_line, source::SourceEvent, status::Status};
    use tokio::net::TcpListener;
    use tokio::time::timeout;
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{Message as WsMessage, client::IntoClientRequest},
    };
    use tower::ServiceExt;

    struct DummyHost;
    impl ActionHost for DummyHost {
        fn run_action(
            &self,
            name: &str,
            snapshot: &BTreeMap<String, Value>,
        ) -> Result<Value, ActionError> {
            match name {
                "clipboard.copy-next-system" => {
                    let next = snapshot
                        .get("core.route")
                        .and_then(|r| r.get("next_system"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Sol");
                    Ok(serde_json::json!({ "system": next }))
                }
                "fail" => Err(ActionError::Failed("boom".into())),
                _ => Err(ActionError::NotFound),
            }
        }
    }

    fn test_router(lan: bool, token: &str) -> (Router, CoreHandle, Store) {
        let store = Store::open_in_memory().unwrap();
        let core = spawn_core(vec![], None, Duration::from_millis(100));
        let cfg = ServerConfig {
            port: 47320,
            lan,
            token: token.into(),
            ui_dir: None,
        };
        let router = build_router(
            cfg,
            "192.168.1.50".into(),
            core.clone(),
            store.clone(),
            Arc::new(DummyHost),
        );
        (router, core, store)
    }

    fn with_peer(mut req: Request<Body>, addr: &str) -> Request<Body> {
        let sock: SocketAddr = addr.parse().unwrap();
        req.extensions_mut().insert(ConnectInfo(sock));
        req
    }

    #[tokio::test]
    async fn health_ok_from_loopback() {
        let (app, _core, _store) = test_router(false, "");
        let req = with_peer(
            Request::builder()
                .uri("/api/health")
                .header(header::HOST, "127.0.0.1:47320")
                .body(Body::empty())
                .unwrap(),
            "127.0.0.1:55000",
        );
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 16)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["ok"], true);
        assert!(json["version"].is_string());
    }

    #[tokio::test]
    async fn rejects_foreign_host_header() {
        let (app, _core, _store) = test_router(false, "");
        let req = with_peer(
            Request::builder()
                .uri("/api/health")
                .header(header::HOST, "evil.com")
                .body(Body::empty())
                .unwrap(),
            "127.0.0.1:55000",
        );
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn lan_request_without_token_is_rejected() {
        let (app, _core, _store) = test_router(true, "secret-token-123456");
        let req = with_peer(
            Request::builder()
                .uri("/api/health")
                .header(header::HOST, "192.168.1.50:47320")
                .body(Body::empty())
                .unwrap(),
            "192.168.1.99:55000",
        );
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn lan_request_with_token_sets_cookie() {
        let (app, _core, _store) = test_router(true, "secret-token-123456");
        let req = with_peer(
            Request::builder()
                .uri("/api/health?token=secret-token-123456")
                .header(header::HOST, "192.168.1.50:47320")
                .body(Body::empty())
                .unwrap(),
            "192.168.1.99:55000",
        );
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let cookie = resp
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        assert!(cookie.contains("tmc_token=secret-token-123456"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));

        // Subsequent request with Cookie header (no query token) succeeds.
        let req2 = with_peer(
            Request::builder()
                .uri("/api/health")
                .header(header::HOST, "192.168.1.50:47320")
                .header(header::COOKIE, "tmc_token=secret-token-123456")
                .body(Body::empty())
                .unwrap(),
            "192.168.1.99:55001",
        );
        let resp2 = app.oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::OK);
    }

    enum TestWsStream {
        Tcp(
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        ),
        Duplex(tokio_tungstenite::WebSocketStream<tokio::io::DuplexStream>),
    }

    impl TestWsStream {
        async fn next_text(&mut self) -> String {
            let msg = match self {
                Self::Tcp(s) => s.next().await.unwrap().unwrap(),
                Self::Duplex(s) => s.next().await.unwrap().unwrap(),
            };
            msg.into_text().unwrap().to_string()
        }

        async fn send_text(&mut self, text: &str) {
            let msg = WsMessage::Text(text.to_owned().into());
            match self {
                Self::Tcp(s) => s.send(msg).await.unwrap(),
                Self::Duplex(s) => s.send(msg).await.unwrap(),
            }
        }
    }

    async fn connect_ws_or_duplex(
        url: String,
        router: Router,
    ) -> Result<TestWsStream, tokio_tungstenite::tungstenite::Error> {
        let req = url.clone().into_client_request().unwrap();
        match connect_async(req).await {
            Ok((ws, _)) => Ok(TestWsStream::Tcp(ws)),
            Err(tokio_tungstenite::tungstenite::Error::Io(err))
                if err.kind() == std::io::ErrorKind::PermissionDenied =>
            {
                let (client_io, server_io) = tokio::io::duplex(65536);
                tokio::spawn(async move {
                    let io = hyper_util::rt::TokioIo::new(server_io);
                    let svc = hyper_util::service::TowerToHyperService::new(router);
                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(io, svc)
                        .with_upgrades()
                        .await;
                });
                let req2 = url.into_client_request().unwrap();
                let (ws, _) = tokio_tungstenite::client_async(req2, client_io).await?;
                Ok(TestWsStream::Duplex(ws))
            }
            Err(other) => Err(other),
        }
    }

    #[tokio::test]
    async fn ws_rejects_mismatched_origin() {
        let (app, _core, _store) = test_router(false, "");
        let req = with_peer(
            Request::builder()
                .uri("/ws")
                .header(header::HOST, "127.0.0.1:47320")
                .header(header::ORIGIN, "http://evil.example")
                .header(header::CONNECTION, "Upgrade")
                .header(header::UPGRADE, "websocket")
                .header("Sec-WebSocket-Version", "13")
                .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
                .body(Body::empty())
                .unwrap(),
            "127.0.0.1:55000",
        );
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn ws_sends_hello_then_snapshot_then_updates() {
        let store = Store::open_in_memory().unwrap();
        let core = spawn_core(vec![], None, Duration::from_millis(100));
        let srv = serve(
            ServerConfig {
                port: 0,
                lan: false,
                token: String::new(),
                ui_dir: None,
            },
            core.clone(),
            store.clone(),
            Arc::new(DummyHost),
        )
        .await
        .unwrap();
        let fallback_router = build_router(
            ServerConfig {
                port: srv.port,
                lan: false,
                token: String::new(),
                ui_dir: None,
            },
            "127.0.0.1".into(),
            core.clone(),
            store,
            Arc::new(DummyHost),
        );
        let mut ws =
            connect_ws_or_duplex(format!("ws://127.0.0.1:{}/ws", srv.port), fallback_router)
                .await
                .unwrap();

        let hello: Value = serde_json::from_str(&ws.next_text().await).unwrap();
        assert_eq!(hello["type"], "hello");
        assert_eq!(hello["version"], 1);
        assert_eq!(hello["locale"], "es");
        assert!(hello["topics"].as_array().unwrap().len() >= 8);

        let snap: Value = serde_json::from_str(&ws.next_text().await).unwrap();
        assert_eq!(snap["type"], "snapshot");
        assert!(snap["topics"]["core.location"].is_object());

        core.events
            .send(SourceEvent::Status(
                Status::parse(r#"{"timestamp":"2026-10-17T18:00:00Z","Flags":1}"#).unwrap(),
            ))
            .await
            .unwrap();

        let upd: Value = serde_json::from_str(
            &timeout(Duration::from_secs(2), ws.next_text())
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(upd["type"], "update");
        assert_eq!(upd["topics"]["core.status"]["Flags"], 1);
        srv.shutdown();
    }

    #[tokio::test]
    async fn ws_subscription_filters_topics() {
        let store = Store::open_in_memory().unwrap();
        let core = spawn_core(vec![], None, Duration::from_millis(100));
        let srv = serve(
            ServerConfig {
                port: 0,
                lan: false,
                token: String::new(),
                ui_dir: None,
            },
            core.clone(),
            store.clone(),
            Arc::new(DummyHost),
        )
        .await
        .unwrap();
        let fallback_router = build_router(
            ServerConfig {
                port: srv.port,
                lan: false,
                token: String::new(),
                ui_dir: None,
            },
            "127.0.0.1".into(),
            core.clone(),
            store,
            Arc::new(DummyHost),
        );
        let mut ws =
            connect_ws_or_duplex(format!("ws://127.0.0.1:{}/ws", srv.port), fallback_router)
                .await
                .unwrap();
        let _hello = ws.next_text().await;
        let _snap = ws.next_text().await;

        ws.send_text(r#"{"type":"subscribe","topics":["core.status"]}"#)
            .await;

        let sub_snap: Value = serde_json::from_str(
            &timeout(Duration::from_secs(2), ws.next_text())
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(sub_snap["type"], "snapshot");
        let topics = sub_snap["topics"].as_object().unwrap();
        assert_eq!(topics.len(), 1);
        assert!(topics.contains_key("core.status"));

        // Send a location change (not subscribed) followed by a status change (subscribed).
        core.events
            .send(SourceEvent::Journal {
                ev: parse_line(
                    r#"{"timestamp":"2026-10-17T18:00:00Z","event":"FSDJump","StarSystem":"Sol","JumpDist":5.0}"#,
                )
                .unwrap(),
                file: "Journal.test.log".into(),
                line: 1,
                replaying: false,
            })
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(150)).await;
        core.events
            .send(SourceEvent::Status(
                Status::parse(r#"{"timestamp":"2026-10-17T18:00:01Z","Flags":9}"#).unwrap(),
            ))
            .await
            .unwrap();

        let msg: Value = serde_json::from_str(
            &timeout(Duration::from_secs(2), ws.next_text())
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(msg["type"], "update");
        let updated = msg["topics"].as_object().unwrap();
        assert!(!updated.contains_key("core.location"));
        assert_eq!(updated["core.status"]["Flags"], 9);
        srv.shutdown();
    }

    #[tokio::test]
    async fn action_endpoint_calls_host() {
        let (app, _core, _store) = test_router(false, "");
        let req = with_peer(
            Request::builder()
                .method("POST")
                .uri("/api/actions/clipboard.copy-next-system")
                .header(header::HOST, "127.0.0.1:47320")
                .body(Body::empty())
                .unwrap(),
            "127.0.0.1:55000",
        );
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 16)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["system"], "Sol");

        let req_missing = with_peer(
            Request::builder()
                .method("POST")
                .uri("/api/actions/unknown.action")
                .header(header::HOST, "127.0.0.1:47320")
                .body(Body::empty())
                .unwrap(),
            "127.0.0.1:55000",
        );
        let resp_missing = app.oneshot(req_missing).await.unwrap();
        assert_eq!(resp_missing.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn history_endpoint_paginates() {
        let (app, _core, store) = test_router(false, "");
        let writer = EventWriter::spawn(&store);
        for line in 1..=5 {
            writer.send(StoredEvent {
                file_name: "Journal.test.log".into(),
                line,
                ts: "2026-10-17T18:00:00Z".parse().unwrap(),
                name: "FSDJump".into(),
                cmdr: Some("CMDR TESTER".into()),
                json: format!(r#"{{"event":"FSDJump","line":{line}}}"#),
            });
        }
        writer.flush_blocking().unwrap();

        let req = with_peer(
            Request::builder()
                .uri("/api/history/events?name=FSDJump&limit=2")
                .header(header::HOST, "127.0.0.1:47320")
                .body(Body::empty())
                .unwrap(),
            "127.0.0.1:55000",
        );
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 16)
            .await
            .unwrap();
        let page1: Vec<Value> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(page1.len(), 2);
        let before_id = page1[1]["id"].as_i64().unwrap();

        let req2 = with_peer(
            Request::builder()
                .uri(format!(
                    "/api/history/events?name=FSDJump&limit=2&before_id={before_id}"
                ))
                .header(header::HOST, "127.0.0.1:47320")
                .body(Body::empty())
                .unwrap(),
            "127.0.0.1:55000",
        );
        let resp2 = app.oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::OK);
        let bytes2 = axum::body::to_bytes(resp2.into_body(), 1024 * 16)
            .await
            .unwrap();
        let page2: Vec<Value> = serde_json::from_slice(&bytes2).unwrap();
        assert_eq!(page2.len(), 2);
        assert!(page2[0]["id"].as_i64().unwrap() < before_id);
    }

    #[tokio::test]
    async fn falls_back_to_next_free_port() {
        let occupied = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_port = occupied.local_addr().unwrap().port();
        let store = Store::open_in_memory().unwrap();
        let core = spawn_core(vec![], None, Duration::from_millis(100));
        let srv = serve(
            ServerConfig {
                port: base_port,
                lan: false,
                token: String::new(),
                ui_dir: None,
            },
            core,
            store,
            Arc::new(DummyHost),
        )
        .await
        .unwrap();
        assert!(srv.port > base_port && srv.port <= base_port + 20);
        assert_eq!(srv.local_url, format!("http://127.0.0.1:{}/", srv.port));
        srv.shutdown();
        drop(occupied);
    }

    #[tokio::test]
    async fn pairing_endpoint_is_loopback_only_and_renders_svg_qr() {
        let (app, _core, _store) = test_router(true, "token-abc");
        let loop_req = with_peer(
            Request::builder()
                .uri("/api/pairing")
                .header(header::HOST, "127.0.0.1:47320")
                .body(Body::empty())
                .unwrap(),
            "127.0.0.1:55000",
        );
        let resp = app.clone().oneshot(loop_req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 64)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["url"], "http://192.168.1.50:47320/?token=token-abc");
        assert!(json["qr_svg"].as_str().unwrap().contains("<svg"));

        let lan_req = with_peer(
            Request::builder()
                .uri("/api/pairing?token=token-abc")
                .header(header::HOST, "192.168.1.50:47320")
                .body(Body::empty())
                .unwrap(),
            "192.168.1.99:55000",
        );
        let lan_resp = app.oneshot(lan_req).await.unwrap();
        assert_eq!(lan_resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn serves_static_ui_dir_with_spa_fallback() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "<h1>TMC UI</h1>").unwrap();
        std::fs::write(dir.path().join("app.js"), "console.log('ok');").unwrap();

        let store = Store::open_in_memory().unwrap();
        let core = spawn_core(vec![], None, Duration::from_millis(100));
        let app = build_router(
            ServerConfig {
                port: 47320,
                lan: false,
                token: String::new(),
                ui_dir: Some(dir.path().to_path_buf()),
            },
            "127.0.0.1".into(),
            core,
            store,
            Arc::new(DummyHost),
        );

        let js_req = with_peer(
            Request::builder()
                .uri("/app.js")
                .header(header::HOST, "127.0.0.1:47320")
                .body(Body::empty())
                .unwrap(),
            "127.0.0.1:55000",
        );
        let js_resp = app.clone().oneshot(js_req).await.unwrap();
        assert_eq!(js_resp.status(), StatusCode::OK);
        assert_eq!(
            js_resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/javascript; charset=utf-8"
        );

        let root_req = with_peer(
            Request::builder()
                .uri("/")
                .header(header::HOST, "127.0.0.1:47320")
                .body(Body::empty())
                .unwrap(),
            "127.0.0.1:55000",
        );
        let root_resp = app.oneshot(root_req).await.unwrap();
        assert_eq!(root_resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(root_resp.into_body(), 4096)
            .await
            .unwrap();
        assert_eq!(&body[..], b"<h1>TMC UI</h1>");
    }
}
