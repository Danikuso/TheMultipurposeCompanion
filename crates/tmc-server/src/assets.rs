//! Servidor de archivos estáticos de la UI.

use crate::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use std::path::{Component, Path, PathBuf};

#[cfg(feature = "embed-ui")]
#[derive(rust_embed::RustEmbed)]
#[folder = "../../ui/dist"]
struct EmbeddedUi;

fn mime_for_path(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        "json" => "application/json",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn sanitize_relative_path(uri_path: &str) -> Option<PathBuf> {
    let trimmed = uri_path.trim_start_matches('/');
    if trimmed.is_empty() {
        return Some(PathBuf::from("index.html"));
    }
    let mut out = PathBuf::new();
    for comp in Path::new(trimmed).components() {
        match comp {
            Component::Normal(part) => out.push(part),
            _ => return None,
        }
    }
    Some(out)
}

pub async fn serve_asset(State(state): State<AppState>, uri: Uri) -> Response {
    let Some(rel_path) = sanitize_relative_path(uri.path()) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let rel_str = rel_path.to_string_lossy().replace('\\', "/");

    if let Some(ui_dir) = &state.cfg.ui_dir {
        let candidate = ui_dir.join(&rel_path);
        if let Ok(bytes) = tokio::fs::read(&candidate).await {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime_for_path(&rel_str))],
                Body::from(bytes),
            )
                .into_response();
        }
        let index = ui_dir.join("index.html");
        if let Ok(bytes) = tokio::fs::read(&index).await {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                Body::from(bytes),
            )
                .into_response();
        }
        return StatusCode::NOT_FOUND.into_response();
    }

    #[cfg(feature = "embed-ui")]
    {
        if let Some(file) = EmbeddedUi::get(&rel_str) {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime_for_path(&rel_str))],
                Body::from(file.data.into_owned()),
            )
                .into_response();
        }
        if let Some(index) = EmbeddedUi::get("index.html") {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                Body::from(index.data.into_owned()),
            )
                .into_response();
        }
    }

    if rel_str == "index.html" {
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            Body::from("<!doctype html><html><head><meta charset=\"utf-8\"><title>TMC</title></head><body><div id=\"app\"></div></body></html>"),
        )
            .into_response();
    }

    StatusCode::NOT_FOUND.into_response()
}
