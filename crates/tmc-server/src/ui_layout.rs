//! API REST de perfiles de disposición de tarjetas por dispositivo (`/api/ui/…`).

use crate::{AppState, security::ClientTrust};
use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use tmc_core::StoreError;

const NAME_MAX: usize = 32;
const DOC_VERSION: u64 = 1;

/// Solo la forma canónica (minúsculas con guiones): evita perfiles duplicados por variantes del mismo UUID.
pub(crate) fn valid_device_id(s: &str) -> bool {
    uuid::Uuid::parse_str(s).is_ok_and(|u| u.hyphenated().to_string() == s)
}

pub(crate) fn normalize_name(s: &str) -> Option<String> {
    let t = s.trim();
    let n = t.chars().count();
    (1..=NAME_MAX).contains(&n).then(|| t.to_string())
}

pub(crate) fn valid_doc(v: &Value) -> bool {
    v.as_object()
        .and_then(|o| o.get("version"))
        .and_then(Value::as_u64)
        == Some(DOC_VERSION)
}

fn err(status: StatusCode, code: &str) -> Response {
    (status, Json(json!({ "error": code }))).into_response()
}

fn bad(code: &str) -> Response {
    err(StatusCode::BAD_REQUEST, code)
}

fn not_found() -> Response {
    err(StatusCode::NOT_FOUND, "not_found")
}

fn store_failed(_e: StoreError) -> Response {
    tmc_model::diagnostic!(
        tmc_model::diagnostics::DiagnosticContext::Http,
        "store.ui_layout.failed",
        tmc_model::diagnostics::json!({})
    );
    err(StatusCode::INTERNAL_SERVER_ERROR, "store")
}

fn parse_body(body: &Bytes) -> Result<Value, &'static str> {
    serde_json::from_slice::<Value>(body).map_err(|_| "invalid_json")
}

fn body_name(v: &Value) -> Result<String, &'static str> {
    v.get("name")
        .and_then(Value::as_str)
        .and_then(normalize_name)
        .ok_or("invalid_name")
}

pub async fn list(State(state): State<AppState>) -> Response {
    match state.store.ui_layout_list() {
        Ok(items) => Json(items).into_response(),
        Err(e) => store_failed(e),
    }
}

pub async fn get(State(state): State<AppState>, Path(device_id): Path<String>) -> Response {
    if !valid_device_id(&device_id) {
        return bad("invalid_device");
    }
    match state.store.ui_layout_get(&device_id) {
        Ok(Some(p)) => Json(p).into_response(),
        Ok(None) => not_found(),
        Err(e) => store_failed(e),
    }
}

pub async fn put(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    body: Bytes,
) -> Response {
    if !valid_device_id(&device_id) {
        return bad("invalid_device");
    }
    let v = match parse_body(&body) {
        Ok(v) => v,
        Err(code) => return bad(code),
    };
    let name = match body_name(&v) {
        Ok(n) => n,
        Err(code) => return bad(code),
    };
    let doc = match v.get("doc") {
        Some(d) if valid_doc(d) => d,
        _ => return bad("invalid_doc"),
    };
    match state.store.ui_layout_put(&device_id, &name, doc) {
        Ok(p) => Json(p).into_response(),
        Err(e) => store_failed(e),
    }
}

pub async fn patch(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    body: Bytes,
) -> Response {
    if !valid_device_id(&device_id) {
        return bad("invalid_device");
    }
    let v = match parse_body(&body) {
        Ok(v) => v,
        Err(code) => return bad(code),
    };
    let name = match body_name(&v) {
        Ok(n) => n,
        Err(code) => return bad(code),
    };
    match state.store.ui_layout_rename(&device_id, &name) {
        Ok(true) => match state.store.ui_layout_get(&device_id) {
            Ok(Some(p)) => Json(p).into_response(),
            Ok(None) => not_found(),
            Err(e) => store_failed(e),
        },
        Ok(false) => not_found(),
        Err(e) => store_failed(e),
    }
}

pub async fn copy_from(
    State(state): State<AppState>,
    Path((device_id, source_id)): Path<(String, String)>,
    body: Bytes,
) -> Response {
    if !valid_device_id(&device_id) || !valid_device_id(&source_id) {
        return bad("invalid_device");
    }
    let source = match state.store.ui_layout_get(&source_id) {
        Ok(Some(p)) => p,
        Ok(None) => return not_found(),
        Err(e) => return store_failed(e),
    };
    let name = match state.store.ui_layout_get(&device_id) {
        Ok(Some(existing)) => existing.name,
        Ok(None) => match parse_body(&body).and_then(|v| body_name(&v)) {
            Ok(n) => n,
            Err(code) => return bad(code),
        },
        Err(e) => return store_failed(e),
    };
    match state.store.ui_layout_put(&device_id, &name, &source.doc) {
        Ok(p) => Json(p).into_response(),
        Err(e) => store_failed(e),
    }
}

pub async fn delete(State(state): State<AppState>, Path(device_id): Path<String>) -> Response {
    if !valid_device_id(&device_id) {
        return bad("invalid_device");
    }
    match state.store.ui_layout_delete(&device_id) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => not_found(),
        Err(e) => store_failed(e),
    }
}

pub async fn desktop_device(
    State(state): State<AppState>,
    Extension(trust): Extension<ClientTrust>,
) -> Response {
    if !trust.is_loopback {
        return err(StatusCode::FORBIDDEN, "forbidden");
    }
    match state.host.desktop_device_id() {
        Some(id) => Json(json!({ "device_id": id })).into_response(),
        None => not_found(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActionError, ActionHost, ServerConfig, build_router};
    use axum::{
        Router,
        body::Body,
        extract::ConnectInfo,
        http::{Request, header},
    };
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tmc_core::{Store, spawn_core};
    use tower::ServiceExt;

    struct Host(Option<&'static str>);
    impl ActionHost for Host {
        fn run_action(&self, _: &str, _: &BTreeMap<String, Value>) -> Result<Value, ActionError> {
            Err(ActionError::NotFound)
        }
        fn desktop_device_id(&self) -> Option<String> {
            self.0.map(str::to_owned)
        }
    }

    const A: &str = "11111111-1111-4111-8111-111111111111";
    const B: &str = "22222222-2222-4222-8222-222222222222";
    const TOKEN: &str = "secret-token-123456";

    fn app(host: Option<&'static str>, lan: bool) -> Router {
        let core = spawn_core(vec![], None, Duration::from_millis(100));
        build_router(
            ServerConfig {
                port: 47320,
                lan,
                token: if lan { TOKEN.into() } else { String::new() },
                ui_dir: None,
            },
            "192.168.1.50".into(),
            core,
            Store::open_in_memory().unwrap(),
            Arc::new(Host(host)),
        )
    }

    async fn call(
        app: &Router,
        method: &str,
        uri: &str,
        extra: &[(&str, &str)],
        body: &str,
        lan_peer: bool,
    ) -> (StatusCode, Value) {
        let (host, peer) = if lan_peer {
            ("192.168.1.50:47320", "192.168.1.99:55000")
        } else {
            ("127.0.0.1:47320", "127.0.0.1:55000")
        };
        let mut b = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::HOST, host);
        for (k, v) in extra {
            b = b.header(*k, *v);
        }
        if !body.is_empty() {
            b = b
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::CONTENT_LENGTH, body.len());
        }
        let mut req = b.body(Body::from(body.to_owned())).unwrap();
        req.extensions_mut()
            .insert(ConnectInfo(peer.parse::<std::net::SocketAddr>().unwrap()));
        let resp = app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
            .await
            .unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn local(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, Value) {
        call(app, method, uri, &[], body, false).await
    }

    fn put_body(name: &str) -> String {
        json!({"name": name, "doc": {"version": 1, "tabs": {}}}).to_string()
    }

    #[test]
    fn validators() {
        assert!(!valid_device_id("not-a-uuid"));
        assert!(valid_device_id(A));
        let h = "abcdefab-cdef-4bcd-8bcd-abcdefabcdef";
        assert!(valid_device_id(h));
        assert!(!valid_device_id(&h.to_uppercase()));
        assert!(!valid_device_id(&h.replace('-', "")));
        assert!(!valid_device_id(&format!("urn:uuid:{h}")));
        assert!(!valid_device_id(&format!("{{{h}}}")));
        assert_eq!(normalize_name("  PC  "), Some("PC".into()));
        assert_eq!(normalize_name(""), None);
        assert_eq!(normalize_name(&"x".repeat(33)), None);
        assert_eq!(normalize_name(&"x".repeat(32)), Some("x".repeat(32)));
        assert!(!valid_doc(&json!({"version": 99})));
        assert!(valid_doc(&json!({"version": 1})));
        assert!(!valid_doc(&json!({})));
    }

    #[tokio::test]
    async fn put_then_get_profile() {
        let app = app(None, false);
        let uri = format!("/api/ui/layouts/{A}");
        let (s, _) = local(&app, "PUT", &uri, &put_body("PC")).await;
        assert_eq!(s, StatusCode::OK);
        let (s, v) = local(&app, "GET", &uri, "").await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(v["doc"], json!({"version": 1, "tabs": {}}));
        assert_eq!(v["name"], "PC");
    }

    #[tokio::test]
    async fn put_rejects_bad_inputs() {
        let app = app(None, false);
        let uri = format!("/api/ui/layouts/{A}");
        let (s, v) = local(&app, "PUT", "/api/ui/layouts/nope", &put_body("PC")).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(v["error"], "invalid_device");
        let (s, v) = local(&app, "PUT", &uri, &put_body("")).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(v["error"], "invalid_name");
        let (s, v) = local(&app, "PUT", &uri, r#"{"name":"PC","doc":{}}"#).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(v["error"], "invalid_doc");
        let (s, v) = local(&app, "PUT", &uri, "{").await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(v["error"], "invalid_json");
        let (s, _) = local(&app, "PUT", &uri, &"x".repeat(262_145)).await;
        assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn list_omits_doc() {
        let app = app(None, false);
        local(
            &app,
            "PUT",
            &format!("/api/ui/layouts/{A}"),
            &put_body("PC"),
        )
        .await;
        let (s, v) = local(&app, "GET", "/api/ui/layouts", "").await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert!(v[0].get("doc").is_none());
        assert_eq!(v[0]["device_id"], A);
    }

    #[tokio::test]
    async fn patch_renames_and_404s() {
        let app = app(None, false);
        let uri = format!("/api/ui/layouts/{A}");
        let (s, v) = local(&app, "PATCH", &uri, r#"{"name":"X"}"#).await;
        assert_eq!(s, StatusCode::NOT_FOUND);
        assert_eq!(v["error"], "not_found");
        local(&app, "PUT", &uri, &put_body("PC")).await;
        let (s, v) = local(&app, "PATCH", &uri, r#"{"name":" Tablet "}"#).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(v["name"], "Tablet");
        let (s, v) = local(&app, "PATCH", &uri, r#"{"name":""}"#).await;
        assert_eq!(s, StatusCode::BAD_REQUEST);
        assert_eq!(v["error"], "invalid_name");
    }

    #[tokio::test]
    async fn delete_204_then_404() {
        let app = app(None, false);
        let uri = format!("/api/ui/layouts/{A}");
        local(&app, "PUT", &uri, &put_body("PC")).await;
        assert_eq!(
            local(&app, "DELETE", &uri, "").await.0,
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            local(&app, "DELETE", &uri, "").await.0,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn copy_from_copies_doc_and_creates_target() {
        let app = app(None, false);
        let doc_a = json!({"version": 1, "tabs": {"a": 1}});
        let body = json!({"name": "Src", "doc": doc_a}).to_string();
        local(&app, "PUT", &format!("/api/ui/layouts/{A}"), &body).await;
        let uri = format!("/api/ui/layouts/{B}/copy-from/{A}");
        let (s, v) = local(&app, "POST", &uri, r#"{"name":"Copia"}"#).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(v["doc"], doc_a);
        assert_eq!(v["name"], "Copia");
        // destino existente: conserva su nombre
        let (s, v) = local(&app, "POST", &uri, r#"{"name":"Otro"}"#).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(v["name"], "Copia");
        // origen inexistente
        let uri = format!("/api/ui/layouts/{A}/copy-from/33333333-3333-4333-8333-333333333333");
        assert_eq!(
            local(&app, "POST", &uri, r#"{"name":"x"}"#).await.0,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn lan_requests_without_token_are_rejected() {
        let app = app(None, true);
        let (s, _) = call(&app, "GET", "/api/ui/layouts", &[], "", true).await;
        assert_eq!(s, StatusCode::FORBIDDEN);
        let uri = format!("/api/ui/layouts/{A}");
        let (s, _) = call(&app, "PUT", &uri, &[], &put_body("PC"), true).await;
        assert_eq!(s, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn loopback_write_without_csrf_proof_is_rejected() {
        let app = app(None, false);
        let uri = format!("/api/ui/layouts/{A}");
        let evil = [("origin", "https://evil.example")];
        let (s, _) = call(&app, "PUT", &uri, &evil, &put_body("PC"), false).await;
        assert_eq!(s, StatusCode::FORBIDDEN);
        let (s, _) = local(&app, "GET", &uri, "").await;
        assert_eq!(s, StatusCode::NOT_FOUND);
        let ok = [("origin", "http://127.0.0.1:47320")];
        let (s, _) = call(&app, "PUT", &uri, &ok, &put_body("PC"), false).await;
        assert_eq!(s, StatusCode::OK);
    }

    #[tokio::test]
    async fn desktop_device_endpoint() {
        let id = "44444444-4444-4444-8444-444444444444";
        let app_ok = app(Some(id), true);
        let (s, v) = local(&app_ok, "GET", "/api/ui/device/desktop", "").await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(v["device_id"], id);
        let cookie = [("cookie", "tmc_token=secret-token-123456")];
        let (s, _) = call(&app_ok, "GET", "/api/ui/device/desktop", &cookie, "", true).await;
        assert_eq!(s, StatusCode::FORBIDDEN);
        let app_none = app(None, false);
        let (s, _) = local(&app_none, "GET", "/api/ui/device/desktop", "").await;
        assert_eq!(s, StatusCode::NOT_FOUND);
    }
}
