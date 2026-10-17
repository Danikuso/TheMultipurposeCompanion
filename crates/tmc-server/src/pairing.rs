//! Emparejamiento LAN por código QR SVG (/api/pairing).

use crate::{AppState, security::ClientTrust};
use axum::{Extension, Json, extract::State, http::StatusCode};
use qrcode::{QrCode, render::svg};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PairingResponse {
    pub url: String,
    pub qr_svg: String,
}

pub async fn get_pairing(
    State(state): State<AppState>,
    Extension(trust): Extension<ClientTrust>,
) -> Result<Json<PairingResponse>, StatusCode> {
    if !trust.is_loopback {
        return Err(StatusCode::FORBIDDEN);
    }
    let url = format!(
        "http://{}:{}/?token={}",
        state.lan_ip, state.port, state.cfg.token
    );
    let code = QrCode::new(url.as_bytes()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let qr_svg = code
        .render::<svg::Color<'_>>()
        .min_dimensions(200, 200)
        .build();
    Ok(Json(PairingResponse { url, qr_svg }))
}
