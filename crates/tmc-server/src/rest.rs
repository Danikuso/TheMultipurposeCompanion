//! Endpoints REST (/api/health, /api/history/events, /api/actions/{name}).

use crate::{ActionError, AppState};
use axum::{
    Json,
    body::Bytes,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tmc_core::EventQuery;

#[derive(Debug, Deserialize, Default)]
pub struct HistoryQueryParams {
    pub name: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub cmdr: Option<String>,
    pub limit: Option<u32>,
    pub before_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct HistoryEventDto {
    pub id: i64,
    pub ts: DateTime<Utc>,
    pub name: String,
    pub cmdr: Option<String>,
    pub json: String,
}

pub async fn get_health() -> Json<Value> {
    Json(json!({
        "ok": true,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

pub async fn get_history_events(
    State(state): State<AppState>,
    Query(params): Query<HistoryQueryParams>,
) -> Result<Json<Vec<HistoryEventDto>>, StatusCode> {
    let query = EventQuery {
        name: params.name,
        from: params.from,
        to: params.to,
        cmdr: params.cmdr,
        limit: params.limit.unwrap_or(100),
        before_id: params.before_id,
    };
    let store = state.store.clone();
    let rows = tokio::task::spawn_blocking(move || store.query_events(&query))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let dtos = rows
        .into_iter()
        .map(|r| HistoryEventDto {
            id: r.id,
            ts: r.ts,
            name: r.name,
            cmdr: r.cmdr,
            json: r.json,
        })
        .collect();
    Ok(Json(dtos))
}

pub async fn post_action(
    State(state): State<AppState>,
    Path(name): Path<String>,
    peer: Option<axum::Extension<axum::extract::ConnectInfo<std::net::SocketAddr>>>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Response {
    let origin = headers
        .get(axum::http::header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let client = format!(
        "rest:{}:{origin}",
        peer.map_or_else(|| "?".to_owned(), |p| p.0.0.ip().to_string())
    );
    let args: Value = if body.iter().all(u8::is_ascii_whitespace) {
        Value::Null
    } else {
        match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(_) => {
                tmc_model::diagnostic!(
                    tmc_model::diagnostics::DiagnosticContext::Http,
                    "http.action.invalid_json",
                    tmc_model::diagnostics::json!({"body_bytes": body.len()})
                );
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "invalid-json" })),
                )
                    .into_response();
            }
        }
    };
    let snapshot = state.core.snapshot().await;
    match crate::diag::run_action(state.host.as_ref(), &name, &args, &snapshot, false, &client) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(ActionError::NotFound) => {
            (StatusCode::NOT_FOUND, Json(json!({ "error": "not-found" }))).into_response()
        }
        Err(ActionError::Failed(reason)) => {
            (StatusCode::BAD_REQUEST, Json(json!({ "error": reason }))).into_response()
        }
    }
}

const AUTOCOMPLETE_PAGE_SIZE: usize = 20;

#[derive(Debug, Deserialize, Default)]
pub struct AutocompleteParams {
    pub q: Option<String>,
    pub page: Option<usize>,
}

pub async fn get_autocomplete(
    State(state): State<AppState>,
    Query(params): Query<AutocompleteParams>,
) -> Response {
    let snapshot = state.core.snapshot().await;
    let q = params.q.unwrap_or_default();
    let raw = match state.host.autocomplete(q, &snapshot).await {
        Ok(v) => v,
        Err(ActionError::NotFound) => {
            return (StatusCode::NOT_FOUND, Json(json!({ "error": "not-found" }))).into_response();
        }
        Err(ActionError::Failed(reason)) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": reason }))).into_response();
        }
    };
    let items: Vec<Value> = match raw {
        Value::Array(a) => a,
        Value::Object(mut o) => match o.remove("items") {
            Some(Value::Array(a)) => a,
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };
    let page = params.page.unwrap_or(0);
    let start = page.saturating_mul(AUTOCOMPLETE_PAGE_SIZE);
    let more = items.len() > start.saturating_add(AUTOCOMPLETE_PAGE_SIZE);
    let slice: Vec<Value> = items
        .into_iter()
        .skip(start)
        .take(AUTOCOMPLETE_PAGE_SIZE)
        .collect();
    (
        StatusCode::OK,
        Json(json!({ "items": slice, "page": page, "more": more })),
    )
        .into_response()
}

pub async fn get_config(State(state): State<AppState>) -> Json<Value> {
    Json(state.host.get_config())
}

pub async fn put_config(State(state): State<AppState>, Json(payload): Json<Value>) -> Response {
    match state.host.put_config(payload) {
        Ok(value) => {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Http,
                "http.config.updated",
                tmc_model::diagnostics::json!({"ok": true})
            );
            (StatusCode::OK, Json(value)).into_response()
        }
        Err(ActionError::NotFound) => {
            (StatusCode::NOT_FOUND, Json(json!({ "error": "not-found" }))).into_response()
        }
        Err(ActionError::Failed(reason)) => {
            tmc_model::diagnostic!(
                tmc_model::diagnostics::DiagnosticContext::Http,
                "http.config.rejected",
                tmc_model::diagnostics::json!({"reason_bytes": reason.len()})
            );
            (StatusCode::BAD_REQUEST, Json(json!({ "error": reason }))).into_response()
        }
    }
}

/// Detalles de ingenieria consultados al actor; nunca se reconstruyen desde SQLite.
pub async fn get_engineering_goals(
    State(state): State<AppState>,
    query: Result<Query<tmc_core::ModulePageQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    engineering_page(&state, "goals", query).await
}
pub async fn get_engineering_locker(
    State(state): State<AppState>,
    query: Result<Query<tmc_core::ModulePageQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    engineering_page(&state, "locker", query).await
}
pub async fn get_engineering_catalogue(
    State(state): State<AppState>,
    query: Result<Query<tmc_core::ModulePageQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    engineering_page(&state, "catalogue", query).await
}
pub async fn get_engineering_engineers(
    State(state): State<AppState>,
    query: Result<Query<tmc_core::ModulePageQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    engineering_page(&state, "engineers", query).await
}
pub async fn get_engineering_flow(
    State(state): State<AppState>,
    query: Result<Query<tmc_core::ModulePageQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    engineering_page(&state, "flow", query).await
}
async fn engineering_page(
    state: &AppState,
    resource: &str,
    query: Result<Query<tmc_core::ModulePageQuery>, axum::extract::rejection::QueryRejection>,
) -> Response {
    use tmc_core::ModuleQueryError;
    let result = match query {
        Ok(Query(query)) => {
            state
                .core
                .query_module("engineering", resource, query)
                .await
        }
        Err(_) => Err(ModuleQueryError::InvalidQuery),
    };
    let mut response = match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => {
            let (status, code) = match error {
                ModuleQueryError::NotFound => (StatusCode::NOT_FOUND, "not-found"),
                ModuleQueryError::InvalidQuery => (StatusCode::BAD_REQUEST, "invalid-query"),
                ModuleQueryError::StaleRevision => (StatusCode::CONFLICT, "stale-revision"),
                ModuleQueryError::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
                ModuleQueryError::ItemTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "item-too-large"),
            };
            (status, Json(json!({"error":code}))).into_response()
        }
    };
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}

/// Includes security and router rejections for these live resources only.
pub(crate) async fn no_store_engineering_pages(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let no_store = matches!(
        request.uri().path(),
        "/api/engineering/goals"
            | "/api/engineering/locker"
            | "/api/engineering/catalogue"
            | "/api/engineering/engineers"
            | "/api/engineering/flow"
    );
    let mut response = next.run(request).await;
    if no_store {
        response.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-store"),
        );
    }
    response
}

#[derive(Debug, Deserialize, Default)]
pub struct FrontierLoginQuery {
    pub redirect: Option<bool>,
    pub open: Option<bool>,
}

pub fn open_url_in_browser(url: &str) -> bool {
    #[cfg(windows)]
    {
        std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
            .is_ok()
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn().is_ok()
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .is_ok()
    }
}

pub async fn get_frontier_login(
    State(state): State<AppState>,
    Query(query): Query<FrontierLoginQuery>,
    headers: axum::http::HeaderMap,
) -> Response {
    let port = state.port;
    let redirect_uri = format!("https://localhost:{port}/api/auth/frontier/callback");
    match state.host.frontier_login_url(&redirect_uri).await {
        Ok(v) => {
            let auth_url = v
                .get("url")
                .and_then(|u| u.as_str())
                .unwrap_or_default()
                .to_owned();
            let should_open = query.open.unwrap_or(false);
            let mut opened = false;
            if should_open && !auth_url.is_empty() {
                opened = open_url_in_browser(&auth_url);
            }
            let wants_json = query.redirect == Some(false)
                || should_open
                || headers
                    .get(axum::http::header::ACCEPT)
                    .and_then(|a| a.to_str().ok())
                    .is_some_and(|a| a.contains("application/json"));
            if wants_json {
                let mut res = v;
                if let Some(obj) = res.as_object_mut() {
                    obj.insert("opened".to_owned(), serde_json::Value::Bool(opened));
                }
                Json(res).into_response()
            } else {
                axum::response::Redirect::temporary(&auth_url).into_response()
            }
        }
        Err(e) => {
            let (status, msg) = match e {
                ActionError::NotFound => (StatusCode::NOT_FOUND, "frontier-auth-not-supported"),
                ActionError::Failed(ref m) => (StatusCode::BAD_REQUEST, m.as_str()),
            };
            (status, Json(json!({ "error": msg }))).into_response()
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct FrontierCallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

pub async fn get_frontier_callback(
    State(state): State<AppState>,
    Query(params): Query<FrontierCallbackParams>,
    headers: axum::http::HeaderMap,
) -> Response {
    let wants_json = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|a| a.to_str().ok())
        .is_some_and(|a| a.contains("application/json"));

    if let Some(err) = params.error {
        let desc = params.error_description.unwrap_or(err);
        if wants_json {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": desc }))).into_response();
        }
        return render_oauth_html_response(false, &format!("Error de autenticación: {desc}"));
    }

    let (Some(code), Some(auth_state)) = (params.code, params.state) else {
        if wants_json {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "missing-code-or-state" })),
            )
                .into_response();
        }
        return render_oauth_html_response(
            false,
            "Parámetros requeridos ausentes en la respuesta de Frontier.",
        );
    };

    let redirect_uri = format!(
        "https://localhost:{}/api/auth/frontier/callback",
        state.port
    );
    match state
        .host
        .frontier_exchange_code(&code, &auth_state, &redirect_uri)
        .await
    {
        Ok(res) => {
            if wants_json {
                Json(res).into_response()
            } else {
                render_oauth_html_response(
                    true,
                    "Tu cuenta de Frontier Store ha sido vinculada con éxito. Ya puedes cerrar esta ventana y regresar a TMC.",
                )
            }
        }
        Err(e) => {
            let msg = match e {
                ActionError::Failed(m) => m,
                ActionError::NotFound => "Servicio CAPI no configurado".to_string(),
            };
            if wants_json {
                (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response()
            } else {
                render_oauth_html_response(
                    false,
                    &format!("No se pudo completar la vinculación: {msg}"),
                )
            }
        }
    }
}

fn render_oauth_html_response(success: bool, message: &str) -> Response {
    let title = if success {
        "TMC — Frontier Store Conectado"
    } else {
        "TMC — Error de Vinculación"
    };
    let accent_color = if success { "#00d4b2" } else { "#ff6644" };
    let icon = if success { "✓" } else { "✕" };
    let subtitle = if success {
        "VINCULACIÓN EXITOSA"
    } else {
        "ERROR DE AUTENTICACIÓN"
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{title}</title>
    <style>
        :root {{
            --bg: #080c10;
            --surface: rgba(14, 22, 33, 0.85);
            --border: rgba(0, 212, 178, 0.25);
            --text-main: #e6f1ff;
            --text-dim: #7e9bb5;
            --accent: {accent_color};
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            background: var(--bg);
            color: var(--text-main);
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", sans-serif;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            padding: 20px;
        }}
        .card {{
            background: var(--surface);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 32px 40px;
            max-width: 480px;
            text-align: center;
            box-shadow: 0 12px 32px rgba(0, 0, 0, 0.6);
            backdrop-filter: blur(12px);
        }}
        .icon {{
            font-size: 48px;
            color: var(--accent);
            margin-bottom: 16px;
            line-height: 1;
        }}
        .badge {{
            display: inline-block;
            font-size: 11px;
            letter-spacing: 0.1em;
            text-transform: uppercase;
            color: var(--accent);
            border: 1px solid var(--accent);
            padding: 3px 8px;
            border-radius: 4px;
            margin-bottom: 12px;
            font-weight: 600;
        }}
        h1 {{
            font-size: 20px;
            font-weight: 600;
            margin-bottom: 12px;
            color: var(--text-main);
        }}
        p {{
            font-size: 14px;
            color: var(--text-dim);
            line-height: 1.5;
            margin-bottom: 24px;
        }}
        .footer {{
            font-size: 12px;
            color: #4f6b85;
            border-top: 1px solid rgba(255, 255, 255, 0.08);
            padding-top: 16px;
        }}
    </style>
</head>
<body>
    <div class="card">
        <div class="icon">{icon}</div>
        <div class="badge">{subtitle}</div>
        <h1>{title}</h1>
        <p>{message}</p>
        <div class="footer">The Multipurpose Companion · Elite Dangerous</div>
    </div>
</body>
</html>"#
    );

    let status = if success {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    (
        status,
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
        .into_response()
}

/// Consulta de voces disponibles según el motor TTS activo.
pub async fn get_tts_voices(State(state): State<AppState>) -> Response {
    let snapshot = state.core.snapshot().await;
    match state
        .host
        .run_action_with("tts.voices", &Value::Null, &snapshot)
    {
        Ok(val) => (StatusCode::OK, Json(val)).into_response(),
        Err(ActionError::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "not-found" })),
        )
            .into_response(),
        Err(ActionError::Failed(err)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err })),
        )
            .into_response(),
    }
}

/// Prueba inmediata de locución del copiloto de voz.
pub async fn post_tts_test(State(state): State<AppState>, Json(payload): Json<Value>) -> Response {
    let snapshot = state.core.snapshot().await;
    match state.host.run_action_with("tts.test", &payload, &snapshot) {
        Ok(val) => (StatusCode::OK, Json(val)).into_response(),
        Err(ActionError::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "not-found" })),
        )
            .into_response(),
        Err(ActionError::Failed(err)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": err })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct VoiceAttackQueryParams {
    pub download: Option<bool>,
    pub format: Option<String>,
}

/// Estado de la integración con VoiceAttack y disponibilidad de perfiles.
pub async fn get_voiceattack_status(
    State(state): State<AppState>,
    Query(params): Query<VoiceAttackQueryParams>,
) -> Response {
    let snapshot = state.core.snapshot().await;
    if params.download == Some(true) || params.format.as_deref() == Some("xml") {
        return serve_vap_profile(&state, &snapshot).await;
    }
    match state
        .host
        .run_action_with("voice.status", &Value::Null, &snapshot)
    {
        Ok(val) => (StatusCode::OK, Json(val)).into_response(),
        Err(ActionError::NotFound) => {
            let status = tmc_model::voice_intent::VoiceAttackStatus {
                installed: false,
                running: false,
                profile_available: true,
                install_path: None,
            };
            (
                StatusCode::OK,
                Json(serde_json::to_value(status).unwrap_or(Value::Null)),
            )
                .into_response()
        }
        Err(ActionError::Failed(err)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err })),
        )
            .into_response(),
    }
}

/// Descarga directa del perfil XML oficial `.vap` de TMC para VoiceAttack.
pub async fn get_voiceattack_profile(State(state): State<AppState>) -> Response {
    let snapshot = state.core.snapshot().await;
    serve_vap_profile(&state, &snapshot).await
}

async fn serve_vap_profile(
    state: &AppState,
    snapshot: &std::collections::BTreeMap<String, Value>,
) -> Response {
    let xml = match state.host.run_action_with(
        "voice.profile",
        &serde_json::json!({ "port": state.port }),
        snapshot,
    ) {
        Ok(v) => v.get("xml").and_then(|s| s.as_str()).map(String::from),
        Err(_) => None,
    }
    .unwrap_or_else(|| {
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<Profile><Name>TMC - The Multipurpose Companion</Name></Profile>".to_string()
    });

    (
        StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/xml; charset=utf-8",
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"TMC-Copiloto.vap\"",
            ),
        ],
        xml,
    )
        .into_response()
}

/// Intérprete y ejecutor del asistente de voz de cabina.
pub async fn post_voice_intent(
    State(state): State<AppState>,
    Json(payload): Json<tmc_model::voice_intent::VoiceCommandRequest>,
) -> Response {
    let query = payload.query.trim();
    if query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "empty-query" })),
        )
            .into_response();
    }

    let snapshot = state.core.snapshot().await;
    let game_state = state.core.game_state().await;

    let locale = payload
        .locale
        .filter(|l| matches!(l.as_str(), "es" | "en" | "fr" | "de" | "pt"))
        .unwrap_or_else(|| {
            state
                .store
                .kv_get("locale")
                .unwrap_or_else(|| "es".to_string())
        });

    let (notoriety, total_fines, total_bounties, nearest_factor, nearest_bm) = snapshot
        .get("crime.status")
        .map(|c| {
            let n = c.get("notoriety").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
            let f = c.get("total_fines").and_then(|v| v.as_u64()).unwrap_or(0);
            let b = c
                .get("total_bounties")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let if_fac = c
                .get("nearest_interstellar_factor")
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            let bm_fac = c
                .get("nearest_black_market")
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            (n, f, b, if_fac, bm_fac)
        })
        .unwrap_or_default();

    let bio_count = snapshot
        .get("exobiology.body")
        .and_then(|v| v.get("signals"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    let ctx = tmc_core::VoiceResolverContext {
        locale,
        nearest_interstellar_factor: nearest_factor,
        nearest_black_market: nearest_bm,
        total_fines,
        total_bounties,
        notoriety,
        bio_signal_bodies_count: bio_count,
    };

    let response = tmc_core::VoiceIntentResolver::resolve(query, &game_state, &ctx);

    if let Some(target) = &response.clipboard_copied {
        let _ = state.host.run_action_with(
            "clipboard.copy",
            &serde_json::json!({ "text": target }),
            &snapshot,
        );
    }

    if !response.spoken_text.is_empty() {
        let _ = state.host.run_action_with(
            "voice.speak",
            &serde_json::json!({ "text": response.spoken_text }),
            &snapshot,
        );
    }

    (StatusCode::OK, Json(response)).into_response()
}
