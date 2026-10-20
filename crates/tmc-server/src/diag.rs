//! Diagnóstico de depuración del servidor: HTTP, autenticación y acciones.
//!
//! Solo plantillas de ruta, métodos, códigos de estado, tamaños y decisiones.
//! Nunca URLs, cadenas de consulta, cabeceras, cookies, tokens ni textos de error.

use crate::{ActionError, ActionHost};
use serde_json::Value;
use std::collections::BTreeMap;

/// Decisión de autenticación o emparejamiento (solo booleanos y contadores).
macro_rules! auth_event {
    ($code:literal, { $($payload:tt)* }) => {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Auth,
            $code,
            tmc_model::diagnostics::json!({ $($payload)* })
        )
    };
}
pub(crate) use auth_event;

/// Intentos rechazados desde el arranque (contador, no identifica a nadie).
#[cfg(debug_assertions)]
pub(crate) fn rejected_attempts() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static REJECTED: AtomicU64 = AtomicU64::new(0);
    REJECTED.fetch_add(1, Ordering::Relaxed) + 1
}

/// Registra la petición HTTP con su plantilla de ruta.
#[cfg(debug_assertions)]
pub(crate) async fn trace_http(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::extract::MatchedPath;
    let started = std::time::Instant::now();
    let method = req.method().as_str().to_owned();
    let route = match req.extensions().get::<MatchedPath>() {
        Some(path) => path.as_str().to_owned(),
        None if req.uri().path().starts_with("/api/") => "<unmatched>".to_owned(),
        None => "<asset>".to_owned(),
    };
    let request_bytes = body_len(req.headers());
    let response = next.run(req).await;
    let status = response.status().as_u16();
    tmc_model::diagnostic!(
        tmc_model::diagnostics::DiagnosticContext::Http,
        "http.request",
        tmc_model::diagnostics::json!({
            "method": method,
            "route": route,
            "status": status,
            "class": i64::from(status / 100) * 100,
            "duration_ms": started.elapsed().as_millis() as u64,
            "request_bytes": request_bytes,
            "response_bytes": body_len(response.headers()),
        })
    );
    response
}

#[cfg(debug_assertions)]
fn body_len(headers: &axum::http::HeaderMap) -> u64 {
    headers
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Ejecuta una acción registrando nombre (lista cerrada), resultado y duración.
/// Los argumentos pasan por la redacción del contexto `Action`; los motivos de
/// fallo (texto libre) no se registran.
#[cfg_attr(not(debug_assertions), allow(unused_variables))]
pub(crate) fn run_action(
    host: &dyn ActionHost,
    name: &str,
    args: &Value,
    snapshot: &BTreeMap<String, Value>,
    over_ws: bool,
) -> Result<Value, ActionError> {
    #[cfg(debug_assertions)]
    let started = std::time::Instant::now();
    tmc_model::diagnostic!(
        tmc_model::diagnostics::DiagnosticContext::Action { name },
        "action.requested",
        tmc_model::diagnostics::json!({"ws": over_ws, "args": args})
    );
    let result = host.run_action_with(name, args, snapshot);
    match &result {
        Ok(value) => tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Action { name },
            "action.succeeded",
            tmc_model::diagnostics::json!({"ws": over_ws, "result": value,
                "duration_ms": started.elapsed().as_millis() as u64})
        ),
        Err(ActionError::NotFound) => tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Action { name },
            "action.not_found",
            tmc_model::diagnostics::json!({"ws": over_ws,
                "duration_ms": started.elapsed().as_millis() as u64})
        ),
        Err(ActionError::Failed(_)) => tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::Action { name },
            "action.failed",
            tmc_model::diagnostics::json!({"ws": over_ws,
                "duration_ms": started.elapsed().as_millis() as u64})
        ),
    }
    result
}
