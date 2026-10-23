//! Servidor HTTP y WebSocket para estáticos, protocolo en vivo y emparejamiento LAN.
#![forbid(unsafe_code)]

pub mod assets;
mod diag;
#[cfg(debug_assertions)]
mod diag_ui;
pub mod pairing;
pub mod rest;
pub mod security;
pub mod ws;

use axum::Router;
use serde_json::Value;
use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
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

/// Futuro de una consulta asíncrona del anfitrión (autocompletado).
pub type ActionFut = Pin<Box<dyn Future<Output = Result<Value, ActionError>> + Send + 'static>>;

pub trait ConfigHost: Send + Sync + 'static {
    fn get_config(&self) -> Value;
    fn put_config(&self, v: Value) -> Result<Value, ActionError>;
}

pub trait ActionHost: Send + Sync + 'static {
    fn run_action(
        &self,
        name: &str,
        snapshot: &BTreeMap<String, Value>,
    ) -> Result<Value, ActionError>;

    /// Acción con argumentos JSON (`null` si no hay cuerpo).
    fn run_action_with(
        &self,
        name: &str,
        args: &Value,
        snapshot: &BTreeMap<String, Value>,
    ) -> Result<Value, ActionError> {
        let _ = args;
        self.run_action(name, snapshot)
    }

    /// Sugerencias de sistemas: `{ "items": [...] }` o un array. El servidor pagina.
    fn autocomplete(&self, _q: String, _snapshot: &BTreeMap<String, Value>) -> ActionFut {
        Box::pin(async { Err(ActionError::NotFound) })
    }

    fn get_config(&self) -> Value {
        Value::Null
    }

    fn put_config(&self, _v: Value) -> Result<Value, ActionError> {
        Err(ActionError::NotFound)
    }

    /// Inicia el flujo OAuth2 PKCE devolviendo la URL de login y el estado generado.
    fn frontier_login_url(&self, _redirect_uri: &str) -> ActionFut {
        Box::pin(async { Err(ActionError::NotFound) })
    }

    /// Canjea el código de autorización de Frontier OAuth2 por credenciales persistentes.
    fn frontier_exchange_code(&self, _code: &str, _state: &str, _redirect_uri: &str) -> ActionFut {
        Box::pin(async { Err(ActionError::NotFound) })
    }
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
    let router = Router::new()
        .route("/ws", axum::routing::get(ws::ws_handler))
        .route("/api/health", axum::routing::get(rest::get_health))
        .route(
            "/api/config",
            axum::routing::get(rest::get_config).put(rest::put_config),
        )
        .route(
            "/api/history/events",
            axum::routing::get(rest::get_history_events),
        )
        .route(
            "/api/actions/{name}",
            axum::routing::post(rest::post_action),
        )
        .route(
            "/api/navigation/autocomplete",
            axum::routing::get(rest::get_autocomplete),
        )
        .route(
            "/api/engineering/goals",
            axum::routing::get(rest::get_engineering_goals),
        )
        .route(
            "/api/engineering/locker",
            axum::routing::get(rest::get_engineering_locker),
        )
        .route(
            "/api/engineering/catalogue",
            axum::routing::get(rest::get_engineering_catalogue),
        )
        .route(
            "/api/engineering/engineers",
            axum::routing::get(rest::get_engineering_engineers),
        )
        .route(
            "/api/engineering/flow",
            axum::routing::get(rest::get_engineering_flow),
        )
        .route(
            "/api/auth/frontier/login",
            axum::routing::get(rest::get_frontier_login),
        )
        .route(
            "/api/auth/frontier/callback",
            axum::routing::get(rest::get_frontier_callback),
        )
        .route("/api/pairing", axum::routing::get(pairing::get_pairing))
        .fallback(axum::routing::get(assets::serve_asset))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            security::enforce_security,
        ))
        .with_state(state)
        .layer(axum::middleware::from_fn(rest::no_store_engineering_pages));
    #[cfg(debug_assertions)]
    let router = router.layer(axum::middleware::from_fn(diag::trace_http));
    router
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

    struct PageModule;
    impl tmc_core::Module for PageModule {
        fn id(&self) -> &'static str {
            "engineering"
        }
        fn on_event(
            &mut self,
            _: &tmc_model::source::SourceEvent,
            _: &tmc_model::state::GameState,
            _: &mut tmc_core::ModuleCtx,
        ) -> tmc_core::TopicSet {
            tmc_core::TopicSet::new()
        }
        fn topics(&self) -> Vec<String> {
            vec![]
        }
        fn topic_value(&self, _: &str) -> Option<Value> {
            None
        }
        fn query(
            &self,
            resource: &str,
            query: &tmc_core::ModulePageQuery,
        ) -> Result<Value, tmc_core::ModuleQueryError> {
            use tmc_core::ModuleQueryError as E;
            if query.revision != 7 {
                return Err(E::StaleRevision);
            }
            if query.section.as_deref() == Some("oversize") {
                return Err(E::ItemTooLarge);
            }
            if query.section.as_deref() == Some("panic") {
                panic!("synthetic page panic");
            }
            if query.cursor > 1 || query.section.as_deref() == Some("invalid") {
                return Err(E::InvalidQuery);
            }
            Ok(
                serde_json::json!({"revision":7,"section":query.section.as_deref().unwrap_or(if resource=="goals" {"goals"} else {"items"}),"cursor":query.cursor,"next_cursor":null,"total":1,"items":[{"count":8}]}),
            )
        }
    }
    #[tokio::test]
    async fn engineering_pages_validate_queries_headers_and_auth() {
        let store = Store::open_in_memory().unwrap();
        let core = spawn_core(vec![Box::new(PageModule)], None, Duration::from_millis(100));
        let app = build_router(
            ServerConfig {
                port: 47320,
                lan: true,
                token: "test-secret".into(),
                ui_dir: None,
            },
            "192.168.1.50".into(),
            core,
            store,
            Arc::new(DummyHost),
        );
        for resource in ["goals", "locker", "catalogue", "engineers", "flow"] {
            for (suffix, expected) in [
                ("", 400),
                ("?revision=bad", 400),
                ("?revision=-1", 400),
                ("?revision=18446744073709551616", 400),
                ("?revision=7&cursor=bad", 400),
                ("?revision=7&cursor=-1", 400),
                ("?revision=7&cursor=2", 400),
                ("?revision=7&section=invalid", 400),
                ("?revision=6", 409),
                ("?revision=7&section=oversize", 413),
                ("?revision=7&section=panic", 503),
                ("?revision=7", 200),
            ] {
                let req = with_peer(
                    Request::builder()
                        .uri(format!("/api/engineering/{resource}{suffix}"))
                        .header(header::HOST, "127.0.0.1:47320")
                        .body(Body::empty())
                        .unwrap(),
                    "127.0.0.1:55000",
                );
                let response = app.clone().oneshot(req).await.unwrap();
                assert_eq!(response.status().as_u16(), expected, "{resource}{suffix}");
                assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
                if expected == 200 {
                    let body = axum::body::to_bytes(response.into_body(), 16384)
                        .await
                        .unwrap();
                    let page: Value = serde_json::from_slice(&body).unwrap();
                    assert_eq!(page["cursor"], 0);
                    assert_eq!(page["revision"], 7);
                    assert_eq!(
                        page["section"],
                        if resource == "goals" {
                            "goals"
                        } else {
                            "items"
                        }
                    );
                }
            }
            for cookie in [None, Some("tmc_token=test-secret")] {
                let mut req = Request::builder()
                    .uri(format!("/api/engineering/{resource}?revision=7"))
                    .header(header::HOST, "192.168.1.50:47320");
                if let Some(cookie) = cookie {
                    req = req.header(header::COOKIE, cookie);
                }
                let response = app
                    .clone()
                    .oneshot(with_peer(
                        req.body(Body::empty()).unwrap(),
                        "192.168.1.99:55000",
                    ))
                    .await
                    .unwrap();
                assert_eq!(
                    response.status(),
                    if cookie.is_some() {
                        StatusCode::OK
                    } else {
                        StatusCode::FORBIDDEN
                    }
                );
            }
        }
        let (empty, _, _) = test_router(false, "");
        let req = with_peer(
            Request::builder()
                .uri("/api/engineering/goals?revision=0")
                .header(header::HOST, "127.0.0.1:47320")
                .body(Body::empty())
                .unwrap(),
            "127.0.0.1:55000",
        );
        let response = empty.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }

    #[tokio::test]
    async fn engineering_page_rejections_are_not_cached_without_changing_other_routes() {
        let (app, _, _) = test_router(true, "test-secret");
        for resource in ["goals", "locker", "catalogue", "engineers", "flow"] {
            for (method, host, peer, expected) in [
                (
                    "GET",
                    "192.168.1.50:47320",
                    "192.168.1.99:55000",
                    StatusCode::FORBIDDEN,
                ),
                (
                    "POST",
                    "127.0.0.1:47320",
                    "127.0.0.1:55000",
                    StatusCode::METHOD_NOT_ALLOWED,
                ),
            ] {
                let req = with_peer(
                    Request::builder()
                        .method(method)
                        .uri(format!("/api/engineering/{resource}?revision=7"))
                        .header(header::HOST, host)
                        .body(Body::empty())
                        .unwrap(),
                    peer,
                );
                let response = app.clone().oneshot(req).await.unwrap();
                assert_eq!(response.status(), expected);
                assert_eq!(
                    response
                        .headers()
                        .get(header::CACHE_CONTROL)
                        .and_then(|v| v.to_str().ok()),
                    Some("no-store")
                );
            }
        }
        let req = with_peer(
            Request::builder()
                .uri("/api/health")
                .header(header::HOST, "127.0.0.1:47320")
                .body(Body::empty())
                .unwrap(),
            "127.0.0.1:55000",
        );
        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!response.headers().contains_key(header::CACHE_CONTROL));
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
        #[cfg(debug_assertions)]
        assert_eq!(hello["capabilities"]["diagnostics"], true);
        #[cfg(not(debug_assertions))]
        assert!(hello["capabilities"]["diagnostics"].is_null());
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

    #[cfg(debug_assertions)]
    #[tokio::test]
    async fn diagnostics_action_checks_origin_and_accepts_valid_batch() {
        let (app, _core, _store) = test_router(false, "");
        let body = r#"[{"code":"ui.ws.update","ts":1,"payload":{"seq":1}}]"#;
        let post = |origin: Option<&str>| {
            let mut b = Request::builder()
                .method("POST")
                .uri("/api/actions/app.diagnostics")
                .header(header::HOST, "127.0.0.1:47320")
                .header(header::CONTENT_TYPE, "application/json");
            if let Some(o) = origin {
                b = b.header(header::ORIGIN, o);
            }
            with_peer(b.body(Body::from(body)).unwrap(), "127.0.0.77:55000")
        };
        let resp = app
            .clone()
            .oneshot(post(Some("http://evil.example")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        let resp = app
            .clone()
            .oneshot(post(Some("http://127.0.0.1:47320")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["accepted"], 1);
        // Mismo cliente dentro del intervalo: 400 rate-limited.
        let resp = app
            .oneshot(post(Some("http://127.0.0.1:47320")))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
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

    /// Host que registra las acciones recibidas y sirve nombres de autocompletado.
    struct RecHost {
        actions: std::sync::Mutex<Vec<(String, Value)>>,
        names: Vec<String>,
        online: bool,
        transport_calls: std::sync::atomic::AtomicUsize,
    }

    impl RecHost {
        fn new(count: usize, online: bool) -> Arc<Self> {
            Arc::new(Self {
                actions: std::sync::Mutex::new(Vec::new()),
                names: (0..count).map(|i| format!("Sys {i:02}")).collect(),
                online,
                transport_calls: std::sync::atomic::AtomicUsize::new(0),
            })
        }
    }

    impl ActionHost for RecHost {
        fn run_action(
            &self,
            _name: &str,
            _snapshot: &BTreeMap<String, Value>,
        ) -> Result<Value, ActionError> {
            Err(ActionError::NotFound)
        }

        fn run_action_with(
            &self,
            name: &str,
            args: &Value,
            _snapshot: &BTreeMap<String, Value>,
        ) -> Result<Value, ActionError> {
            self.actions
                .lock()
                .unwrap()
                .push((name.to_owned(), args.clone()));
            Ok(serde_json::json!({ "accepted": true }))
        }

        fn autocomplete(&self, _q: String, snapshot: &BTreeMap<String, Value>) -> ActionFut {
            let legacy = snapshot
                .get("core.game")
                .and_then(|g| g.get("galaxy"))
                .and_then(Value::as_str)
                == Some("Legacy");
            if legacy || !self.online {
                return Box::pin(async { Ok(serde_json::json!({ "items": [] })) });
            }
            self.transport_calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let items: Vec<Value> = self
                .names
                .iter()
                .enumerate()
                .map(|(i, n)| serde_json::json!({ "name": n, "id64": i as u64 }))
                .collect();
            Box::pin(async move { Ok(serde_json::json!({ "items": items })) })
        }
    }

    fn router_with_host(host: Arc<dyn ActionHost>, lan: bool, token: &str) -> (Router, CoreHandle) {
        let store = Store::open_in_memory().unwrap();
        let core = spawn_core(vec![], None, Duration::from_millis(100));
        let router = build_router(
            ServerConfig {
                port: 47320,
                lan,
                token: token.into(),
                ui_dir: None,
            },
            "192.168.1.50".into(),
            core.clone(),
            store,
            host,
        );
        (router, core)
    }

    async fn get_json(app: Router, uri: &str) -> (StatusCode, Value) {
        let req = with_peer(
            Request::builder()
                .uri(uri)
                .header(header::HOST, "127.0.0.1:47320")
                .body(Body::empty())
                .unwrap(),
            "127.0.0.1:55000",
        );
        let resp = app.oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 64)
            .await
            .unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    #[tokio::test]
    async fn action_body_args_reach_host() {
        let host = RecHost::new(0, true);
        let (app, _core) = router_with_host(host.clone(), false, "");
        let req = with_peer(
            Request::builder()
                .method("POST")
                .uri("/api/actions/navigation.plan")
                .header(header::HOST, "127.0.0.1:47320")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"to":"Colonia"}"#))
                .unwrap(),
            "127.0.0.1:55000",
        );
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        // Sin cuerpo: args es null.
        let req2 = with_peer(
            Request::builder()
                .method("POST")
                .uri("/api/actions/navigation.clear")
                .header(header::HOST, "127.0.0.1:47320")
                .body(Body::empty())
                .unwrap(),
            "127.0.0.1:55000",
        );
        assert_eq!(app.oneshot(req2).await.unwrap().status(), StatusCode::OK);
        let got = host.actions.lock().unwrap().clone();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].0, "navigation.plan");
        assert_eq!(got[0].1["to"], "Colonia");
        assert_eq!(got[1].0, "navigation.clear");
        assert_eq!(got[1].1, Value::Null);
    }

    #[tokio::test]
    async fn ws_action_with_args_reaches_host() {
        let host = RecHost::new(0, true);
        let store = Store::open_in_memory().unwrap();
        let core = spawn_core(vec![], None, Duration::from_millis(100));
        let cfg = ServerConfig {
            port: 0,
            lan: false,
            token: String::new(),
            ui_dir: None,
        };
        let srv = serve(cfg.clone(), core.clone(), store.clone(), host.clone())
            .await
            .unwrap();
        let fallback = build_router(
            ServerConfig {
                port: srv.port,
                ..cfg
            },
            "127.0.0.1".into(),
            core,
            store,
            host.clone(),
        );
        let mut ws = connect_ws_or_duplex(format!("ws://127.0.0.1:{}/ws", srv.port), fallback)
            .await
            .unwrap();
        let _hello = ws.next_text().await;
        let _snap = ws.next_text().await;
        ws.send_text(r#"{"type":"action","name":"navigation.plan","args":{"to":"Colonia"}}"#)
            .await;
        for _ in 0..40 {
            if !host.actions.lock().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let got = host.actions.lock().unwrap().clone();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0, "navigation.plan");
        assert_eq!(got[0].1["to"], "Colonia");
        srv.shutdown();
    }

    #[tokio::test]
    async fn autocomplete_route_paginates() {
        let host = RecHost::new(45, true);
        let (app, _core) = router_with_host(host, false, "");
        let (st, p0) = get_json(app.clone(), "/api/navigation/autocomplete?q=Sys&page=0").await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(p0["items"].as_array().unwrap().len(), 20);
        assert_eq!(p0["items"][0]["name"], "Sys 00");
        assert_eq!(p0["page"], 0);
        assert_eq!(p0["more"], true);
        let (_, p2) = get_json(app.clone(), "/api/navigation/autocomplete?q=Sys&page=2").await;
        assert_eq!(p2["items"].as_array().unwrap().len(), 5);
        assert_eq!(p2["items"][0]["name"], "Sys 40");
        assert_eq!(p2["items"][0]["id64"], 40);
        assert_eq!(p2["page"], 2);
        assert_eq!(p2["more"], false);
        let (_, p9) = get_json(app, "/api/navigation/autocomplete?q=Sys&page=9").await;
        assert_eq!(p9["items"].as_array().unwrap().len(), 0);
        assert_eq!(p9["more"], false);
    }

    #[tokio::test]
    async fn autocomplete_returns_empty_for_legacy_or_offline() {
        let offline = RecHost::new(45, false);
        let (app, _core) = router_with_host(offline.clone(), false, "");
        let (_, out) = get_json(app, "/api/navigation/autocomplete?q=Sys").await;
        assert_eq!(
            out,
            serde_json::json!({ "items": [], "page": 0, "more": false })
        );
        assert_eq!(
            offline
                .transport_calls
                .load(std::sync::atomic::Ordering::SeqCst),
            0
        );

        let online = RecHost::new(45, true);
        let (app2, core2) = router_with_host(online.clone(), false, "");
        core2
            .events
            .send(SourceEvent::Journal {
                ev: parse_line(
                    r#"{"timestamp":"2026-10-17T18:00:00Z","event":"Fileheader","part":1,"language":"English","gameversion":"3.8.0.407","build":"r1"}"#,
                )
                .unwrap(),
                file: "Journal.test.log".into(),
                line: 1,
                replaying: false,
            })
            .await
            .unwrap();
        for _ in 0..40 {
            if core2.snapshot().await["core.game"]["galaxy"] == "Legacy" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(core2.snapshot().await["core.game"]["galaxy"], "Legacy");
        let (_, out2) = get_json(app2, "/api/navigation/autocomplete?q=Sys").await;
        assert_eq!(
            out2,
            serde_json::json!({ "items": [], "page": 0, "more": false })
        );
        assert_eq!(
            online
                .transport_calls
                .load(std::sync::atomic::Ordering::SeqCst),
            0
        );
    }

    async fn send_req(
        app: &Router,
        method: &str,
        uri: &str,
        extra: &[(&str, &str)],
        body: &str,
    ) -> StatusCode {
        let mut b = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::HOST, "127.0.0.1:47320");
        for (k, v) in extra {
            b = b.header(*k, *v);
        }
        if !body.is_empty() {
            b = b.header(header::CONTENT_LENGTH, body.len());
        }
        let req = with_peer(
            b.body(Body::from(body.to_owned())).unwrap(),
            "127.0.0.1:55000",
        );
        app.clone().oneshot(req).await.unwrap().status()
    }

    #[tokio::test]
    async fn state_changes_are_protected_against_csrf() {
        let host = RecHost::new(0, true);
        let (app, _core) = router_with_host(host.clone(), false, "");
        let a = "/api/actions/navigation.plan";
        let json = ("content-type", "application/json");
        // Origin ajeno
        let evil = [("origin", "https://evil.example"), json];
        assert_eq!(
            send_req(&app, "POST", a, &evil, "{}").await,
            StatusCode::FORBIDDEN
        );
        // Sin Origin, Sec-Fetch-Site cross-site
        let cross = [("sec-fetch-site", "cross-site"), json];
        assert_eq!(
            send_req(&app, "POST", a, &cross, "{}").await,
            StatusCode::FORBIDDEN
        );
        // Config PUT cross-origin
        assert_eq!(
            send_req(&app, "PUT", "/api/config", &evil, "{}").await,
            StatusCode::FORBIDDEN
        );
        assert!(host.actions.lock().unwrap().is_empty());
        // Origin coincidente
        let same = [("origin", "http://127.0.0.1:47320"), json];
        assert_eq!(send_req(&app, "POST", a, &same, "{}").await, StatusCode::OK);
        // Sec-Fetch-Site same-origin sin Origin
        let sfs = [("sec-fetch-site", "same-origin"), json];
        assert_eq!(send_req(&app, "POST", a, &sfs, "{}").await, StatusCode::OK);
        // Cliente no navegador (sin cabeceras)
        assert_eq!(send_req(&app, "POST", a, &[], "").await, StatusCode::OK);
        assert_eq!(
            send_req(&app, "POST", a, &[json], r#"{"to":"X"}"#).await,
            StatusCode::OK
        );
        // Cuerpo no JSON
        let text = [("content-type", "text/plain")];
        assert_eq!(
            send_req(&app, "POST", a, &text, "{}").await,
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
        assert_eq!(host.actions.lock().unwrap().len(), 4);
        // GET de solo lectura con Origin ajeno: permitido (no cambia estado)
        assert_eq!(
            send_req(
                &app,
                "GET",
                "/api/health",
                &[("origin", "https://evil.example")],
                ""
            )
            .await,
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn autocomplete_lan_without_token_is_rejected() {
        let (app, _core) = router_with_host(RecHost::new(3, true), true, "secret-token-123456");
        let req = with_peer(
            Request::builder()
                .uri("/api/navigation/autocomplete?q=Sys")
                .header(header::HOST, "192.168.1.50:47320")
                .body(Body::empty())
                .unwrap(),
            "192.168.1.99:55000",
        );
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
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

    #[cfg(debug_assertions)]
    #[test]
    fn diagnostics_use_templates_and_never_credentials() {
        use tmc_model::diagnostics::capture::capture;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (_, records) = capture(|| {
            rt.block_on(async {
                let (app, _core, _store) = test_router(true, "secret-token-123456");
                let lan = |uri: &str| {
                    with_peer(
                        Request::builder()
                            .uri(uri)
                            .header(header::HOST, "192.168.1.50:47320")
                            .header(header::COOKIE, "tmc_token=CANARYCOOKIE")
                            .header(header::AUTHORIZATION, "Bearer CANARYBEARER")
                            .body(Body::empty())
                            .unwrap(),
                        "192.168.1.99:55000",
                    )
                };
                let r = app
                    .clone()
                    .oneshot(lan("/api/health?token=CANARYTOKEN"))
                    .await;
                assert_eq!(r.unwrap().status(), StatusCode::FORBIDDEN);
                let r = app
                    .clone()
                    .oneshot(lan("/api/health?token=secret-token-123456"))
                    .await;
                assert_eq!(r.unwrap().status(), StatusCode::OK);
                let local = |method: &str, uri: &str| {
                    with_peer(
                        Request::builder()
                            .method(method)
                            .uri(uri)
                            .header(header::HOST, "127.0.0.1:47320")
                            .body(Body::empty())
                            .unwrap(),
                        "127.0.0.1:55000",
                    )
                };
                for req in [
                    local("POST", "/api/actions/clipboard.copy-next-system"),
                    local("POST", "/api/actions/fail"),
                    local("GET", "/api/health?x=CANARYQUERY"),
                    local("GET", "/api/zzz?q=CANARYQUERY"),
                ] {
                    let _ = app.clone().oneshot(req).await.unwrap();
                }
            })
        });
        let all: String = records
            .iter()
            .filter(|r| !r.code.starts_with("actor.") && !r.code.starts_with("core."))
            .map(|r| {
                format!(
                    "{} {} {}
",
                    r.code, r.context, r.payload
                )
            })
            .collect();
        for canary in [
            "CANARYTOKEN",
            "secret-token-123456",
            "CANARYCOOKIE",
            "CANARYBEARER",
            "CANARYQUERY",
            "boom",
            "?token",
        ] {
            assert!(!all.contains(canary), "{canary} leaked: {all}");
        }
        let codes: Vec<&str> = records.iter().map(|r| r.code.as_str()).collect();
        for code in [
            "auth.lan.token_rejected",
            "auth.lan.accepted",
            "auth.guard.loopback_accepted",
            "action.requested",
            "action.succeeded",
            "action.failed",
            "http.request",
        ] {
            assert!(codes.contains(&code), "falta {code}: {codes:?}");
        }
        for route in ["/api/actions/{name}", "/api/health", "<unmatched>"] {
            assert!(all.contains(route), "falta ruta {route}: {all}");
        }
    }

    #[cfg(debug_assertions)]
    fn dump(records: &[tmc_model::diagnostics::capture::Record]) -> String {
        records
            .iter()
            .filter(|r| !r.code.starts_with("actor.") && !r.code.starts_with("core."))
            .map(|r| format!("{} {} {}\n", r.code, r.context, r.payload))
            .collect()
    }

    #[cfg(debug_assertions)]
    fn codes(records: &[tmc_model::diagnostics::capture::Record]) -> Vec<&str> {
        records.iter().map(|r| r.code.as_str()).collect()
    }

    #[cfg(debug_assertions)]
    fn paused_rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .start_paused(true)
            .build()
            .unwrap()
    }

    #[cfg(debug_assertions)]
    #[test]
    fn pairing_diagnostics_never_include_url_qr_or_token() {
        use tmc_model::diagnostics::capture::capture;
        let (_, records) = capture(|| {
            paused_rt().block_on(async {
                let (app, _core, _store) = test_router(true, "CANARYLANTOKEN987");
                let local = with_peer(
                    Request::builder()
                        .uri("/api/pairing")
                        .header(header::HOST, "127.0.0.1:47320")
                        .body(Body::empty())
                        .unwrap(),
                    "127.0.0.1:55000",
                );
                let resp = app.clone().oneshot(local).await.unwrap();
                assert_eq!(resp.status(), StatusCode::OK);
                let lan = with_peer(
                    Request::builder()
                        .uri("/api/pairing?token=CANARYLANTOKEN987")
                        .header(header::HOST, "192.168.1.50:47320")
                        .body(Body::empty())
                        .unwrap(),
                    "192.168.1.99:55000",
                );
                let resp = app.oneshot(lan).await.unwrap();
                assert_eq!(resp.status(), StatusCode::FORBIDDEN);
            });
        });
        let all = dump(&records);
        let c = codes(&records);
        assert!(c.contains(&"auth.pairing.issued"), "{all}");
        assert!(c.contains(&"auth.pairing.rejected"), "{all}");
        for canary in [
            "CANARYLANTOKEN987",
            "token=",
            "<svg",
            "192.168.1.50",
            "http://",
        ] {
            assert!(!all.contains(canary), "{canary} leaked: {all}");
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn origin_and_csrf_rejections_are_logged_without_headers() {
        use tmc_model::diagnostics::capture::capture;
        let (_, records) = capture(|| {
            paused_rt().block_on(async {
                let (app, _core, _store) = test_router(true, "CANARYLANTOKEN987");
                for (host, peer, query) in [
                    ("127.0.0.1:47320", "127.0.0.1:55000", ""),
                    (
                        "192.168.1.50:47320",
                        "192.168.1.99:55000",
                        "?token=CANARYLANTOKEN987",
                    ),
                ] {
                    let ws = with_peer(
                        Request::builder()
                            .uri(format!("/ws{query}"))
                            .header(header::HOST, host)
                            .header(header::ORIGIN, "http://CANARYORIGIN.example")
                            .header(header::COOKIE, "tmc_token=CANARYCOOKIE")
                            .header(header::CONNECTION, "Upgrade")
                            .header(header::UPGRADE, "websocket")
                            .header("Sec-WebSocket-Version", "13")
                            .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
                            .body(Body::empty())
                            .unwrap(),
                        peer,
                    );
                    let resp = app.clone().oneshot(ws).await.unwrap();
                    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
                    let post = with_peer(
                        Request::builder()
                            .method("POST")
                            .uri(format!("/api/actions/x{query}"))
                            .header(header::HOST, host)
                            .header(header::ORIGIN, "http://CANARYORIGIN.example")
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Body::from("{}"))
                            .unwrap(),
                        peer,
                    );
                    let resp = app.clone().oneshot(post).await.unwrap();
                    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
                }
            });
        });
        let all = dump(&records);
        let c = codes(&records);
        for code in ["auth.guard.origin_rejected", "auth.guard.csrf_rejected"] {
            assert_eq!(c.iter().filter(|x| **x == code).count(), 2, "{code}: {all}");
        }
        for canary in ["CANARYORIGIN", "CANARYCOOKIE", "CANARYLANTOKEN987"] {
            assert!(!all.contains(canary), "{canary} leaked: {all}");
        }
    }

    #[cfg(debug_assertions)]
    struct FailingConfigHost;

    #[cfg(debug_assertions)]
    impl ActionHost for FailingConfigHost {
        fn run_action(
            &self,
            _name: &str,
            _snapshot: &BTreeMap<String, Value>,
        ) -> Result<Value, ActionError> {
            Err(ActionError::NotFound)
        }

        fn put_config(&self, _v: Value) -> Result<Value, ActionError> {
            Err(ActionError::Failed("CANARYREASON".into()))
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn invalid_json_and_config_failures_log_only_sizes() {
        use tmc_model::diagnostics::capture::capture;
        let (_, records) = capture(|| {
            paused_rt().block_on(async {
                let (app, _core) = router_with_host(Arc::new(FailingConfigHost), false, "");
                let json = [("content-type", "application/json")];
                let status =
                    send_req(&app, "POST", "/api/actions/x", &json, r#"{"CANARYJSON":"#).await;
                assert_eq!(status, StatusCode::BAD_REQUEST);
                let status = send_req(&app, "PUT", "/api/config", &json, "{}").await;
                assert_eq!(status, StatusCode::BAD_REQUEST);
            });
        });
        let all = dump(&records);
        assert!(!all.contains("CANARYJSON"), "{all}");
        assert!(!all.contains("CANARYREASON"), "{all}");
        let invalid = records
            .iter()
            .find(|r| r.code == "http.action.invalid_json")
            .expect("invalid_json");
        assert!(invalid.payload.contains("body_bytes"), "{all}");
        assert!(invalid.payload.contains("14"), "{all}");
        let rejected = records
            .iter()
            .find(|r| r.code == "http.config.rejected")
            .expect("config rejected");
        assert!(rejected.payload.contains("reason_bytes"), "{all}");
        assert!(rejected.payload.contains("12"), "{all}");
    }

    #[cfg(debug_assertions)]
    async fn duplex_ws(
        router: Router,
        buffer: usize,
    ) -> tokio_tungstenite::WebSocketStream<tokio::io::DuplexStream> {
        let (client_io, server_io) = tokio::io::duplex(buffer);
        tokio::spawn(async move {
            let io = hyper_util::rt::TokioIo::new(server_io);
            let svc = hyper_util::service::TowerToHyperService::new(router);
            let _ = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, svc)
                .with_upgrades()
                .await;
        });
        let req = "ws://127.0.0.1:47320/ws".into_client_request().unwrap();
        tokio_tungstenite::client_async(req, client_io)
            .await
            .unwrap()
            .0
    }

    #[cfg(debug_assertions)]
    async fn push_status(core: &CoreHandle, flags: u32) {
        let line = format!(r#"{{"timestamp":"2026-10-17T18:00:00Z","Flags":{flags}}}"#);
        core.events
            .send(SourceEvent::Status(Status::parse(&line).unwrap()))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(150)).await;
    }

    #[cfg(debug_assertions)]
    fn summaries(records: &[tmc_model::diagnostics::capture::Record]) -> Vec<Value> {
        records
            .iter()
            .filter(|r| r.code == "ws.broadcast.summary")
            .map(|r| serde_json::from_str(&r.payload).unwrap())
            .collect()
    }

    #[cfg(debug_assertions)]
    #[test]
    fn ws_summary_is_flushed_on_timer_and_resets_window() {
        use tmc_model::diagnostics::capture::capture;
        let (_, records) = capture(|| {
            paused_rt().block_on(async {
                let (app, core, _store) = test_router(false, "");
                let mut ws = duplex_ws(app, 65536).await;
                let _ = ws.next().await; // hello
                let _ = ws.next().await; // snapshot
                for flags in 1..=3 {
                    push_status(&core, flags).await;
                    let _ = ws.next().await.unwrap().unwrap();
                }
                // Sin mas eventos: el temporizador vuelca la ventana (<= 1/s).
                tokio::time::sleep(Duration::from_millis(1500)).await;
                push_status(&core, 4).await;
                let _ = ws.next().await.unwrap().unwrap();
                ws.send(WsMessage::Text("CANARYWS not json".into()))
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(50)).await;
                ws.close(None).await.unwrap();
                tokio::time::sleep(Duration::from_millis(50)).await;
            });
        });
        let all = dump(&records);
        let sums = summaries(&records);
        assert_eq!(sums.len(), 2, "{all}");
        assert_eq!(sums[0]["updates"], 3, "{all}");
        assert_eq!(sums[0]["alerts"], 0);
        assert_eq!(sums[0]["filtered_out"], 0);
        assert!(sums[0]["bytes"].as_u64().unwrap() > 0);
        assert_eq!(sums[1]["updates"], 1, "ventana reiniciada: {all}");
        let c = codes(&records);
        let last_summary = c
            .iter()
            .rposition(|x| *x == "ws.broadcast.summary")
            .unwrap();
        let closed = c.iter().position(|x| *x == "ws.closed").expect("ws.closed");
        assert!(last_summary < closed, "{c:?}");
        let invalid = records
            .iter()
            .find(|r| r.code == "ws.client.invalid_message")
            .expect("invalid_message");
        assert!(invalid.payload.contains("bytes"));
        assert!(!all.contains("CANARYWS"), "{all}");
    }

    #[cfg(debug_assertions)]
    #[test]
    fn ws_summary_is_flushed_when_the_socket_closes_inside_the_window() {
        use tmc_model::diagnostics::capture::capture;
        let (_, records) = capture(|| {
            paused_rt().block_on(async {
                let (app, core, _store) = test_router(false, "");
                let mut ws = duplex_ws(app, 65536).await;
                let _ = ws.next().await;
                let _ = ws.next().await;
                for flags in 1..=3 {
                    push_status(&core, flags).await;
                    let _ = ws.next().await.unwrap().unwrap();
                }
                ws.close(None).await.unwrap();
                tokio::time::sleep(Duration::from_millis(50)).await;
            });
        });
        let all = dump(&records);
        let sums = summaries(&records);
        assert_eq!(sums.len(), 1, "{all}");
        assert_eq!(sums[0]["updates"], 3, "{all}");
        let c = codes(&records);
        let summary = c.iter().position(|x| *x == "ws.broadcast.summary").unwrap();
        assert!(summary < c.iter().position(|x| *x == "ws.closed").unwrap());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn ws_lagging_client_is_resynchronised_and_logged() {
        use tmc_model::diagnostics::capture::capture;
        let (_, records) = capture(|| {
            paused_rt().block_on(async {
                let (app, core, _store) = test_router(false, "");
                // Buffer minusculo: el servidor se bloquea enviando y el canal se desborda.
                let mut ws = duplex_ws(app, 64).await;
                for flags in 1..=300 {
                    push_status(&core, flags).await;
                }
                let drain = async { while let Some(Ok(_)) = ws.next().await {} };
                let _ = timeout(Duration::from_secs(5), drain).await;
            });
        });
        let all = dump(&records);
        let lagged = records
            .iter()
            .find(|r| r.code == "ws.backpressure.lagged")
            .unwrap_or_else(|| panic!("sin lagged: {all}"));
        assert!(lagged.payload.contains("skipped"));
    }
}
