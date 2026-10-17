//! Endpoints REST (/api/health, /api/history/events, /api/actions/{name}).

use crate::{ActionError, AppState};
use axum::{
    Json,
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

pub async fn post_action(State(state): State<AppState>, Path(name): Path<String>) -> Response {
    let snapshot = state.core.snapshot().await;
    match state.host.run_action(&name, &snapshot) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(ActionError::NotFound) => {
            (StatusCode::NOT_FOUND, Json(json!({ "error": "not-found" }))).into_response()
        }
        Err(ActionError::Failed(reason)) => {
            (StatusCode::BAD_REQUEST, Json(json!({ "error": reason }))).into_response()
        }
    }
}

pub async fn get_config(State(state): State<AppState>) -> Json<Value> {
    Json(state.host.get_config())
}

pub async fn put_config(State(state): State<AppState>, Json(payload): Json<Value>) -> Response {
    match state.host.put_config(payload) {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(ActionError::NotFound) => {
            (StatusCode::NOT_FOUND, Json(json!({ "error": "not-found" }))).into_response()
        }
        Err(ActionError::Failed(reason)) => {
            (StatusCode::BAD_REQUEST, Json(json!({ "error": reason }))).into_response()
        }
    }
}
