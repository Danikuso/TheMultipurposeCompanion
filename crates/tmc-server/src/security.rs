//! Middleware de seguridad anti DNS-rebinding, token LAN y Origin de WebSocket.

use crate::AppState;
use axum::{
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::Response,
};
use std::net::{IpAddr, SocketAddr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientTrust {
    pub is_loopback: bool,
}

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn is_loopback_host(host: &str, port: u16) -> bool {
    let expected_v4 = format!("127.0.0.1:{port}");
    let expected_local = format!("localhost:{port}");
    let expected_v6 = format!("[::1]:{port}");
    host.eq_ignore_ascii_case(&expected_v4)
        || host.eq_ignore_ascii_case(&expected_local)
        || host.eq_ignore_ascii_case(&expected_v6)
}

fn is_lan_host(host: &str, lan_ip: &str, port: u16) -> bool {
    let expected = format!("{lan_ip}:{port}");
    if host.eq_ignore_ascii_case(&expected) {
        return true;
    }
    // Si el host es un literal IPv4:puerto privado (por ejemplo múltiples NICs), nunca un dominio DNS.
    if let Some((ip_part, port_part)) = host.rsplit_once(':')
        && port_part.parse::<u16>() == Ok(port)
        && let Ok(IpAddr::V4(v4)) = ip_part.parse::<IpAddr>()
    {
        return v4.is_private() || v4.is_link_local();
    }
    false
}

fn origin_matches_host(headers: &HeaderMap, host: &str, require_present: bool) -> bool {
    let Some(origin_val) = headers.get(header::ORIGIN) else {
        return !require_present;
    };
    let Ok(origin_str) = origin_val.to_str() else {
        return false;
    };
    let authority = origin_str
        .strip_prefix("http://")
        .or_else(|| origin_str.strip_prefix("https://"));
    let Some(authority) = authority else {
        return false;
    };
    let authority = authority.trim_end_matches('/');
    authority.eq_ignore_ascii_case(host)
}

/// Defensa CSRF para peticiones que cambian estado (no GET/HEAD/OPTIONS).
/// `Err` lleva el código HTTP de rechazo.
fn guard_state_change(req: &Request, host: &str) -> Result<(), StatusCode> {
    use axum::http::Method;
    if matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS) {
        return Ok(());
    }
    let headers = req.headers();
    if headers.contains_key(header::ORIGIN) {
        if !origin_matches_host(headers, host, true) {
            return Err(StatusCode::FORBIDDEN);
        }
    } else if let Some(site) = headers.get("sec-fetch-site") {
        match site.to_str() {
            Ok("same-origin") | Ok("none") => {}
            _ => return Err(StatusCode::FORBIDDEN),
        }
    }
    let has_body = headers.contains_key(header::TRANSFER_ENCODING)
        || headers
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.trim() != "0");
    if has_body {
        let is_json = headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| {
                v.split(';')
                    .next()
                    .is_some_and(|m| m.trim().eq_ignore_ascii_case("application/json"))
            });
        if !is_json {
            return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
        }
    }
    Ok(())
}

fn extract_query_token(query: Option<&str>) -> Option<String> {
    let query = query?;
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=')
            && k == "token"
        {
            return Some(v.to_owned());
        }
    }
    None
}

fn extract_cookie_token(headers: &HeaderMap) -> Option<String> {
    for value in headers.get_all(header::COOKIE) {
        let Ok(raw) = value.to_str() else {
            continue;
        };
        for part in raw.split(';') {
            let trimmed = part.trim();
            if let Some(val) = trimmed.strip_prefix("tmc_token=") {
                return Some(val.to_owned());
            }
        }
    }
    None
}

pub async fn enforce_security(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let Some(host_val) = req.headers().get(header::HOST) else {
        crate::diag::auth_event!("auth.guard.host_missing", {"attempts": crate::diag::rejected_attempts()});
        return Err(StatusCode::FORBIDDEN);
    };
    let Ok(host) = host_val.to_str() else {
        crate::diag::auth_event!("auth.guard.host_invalid", {"attempts": crate::diag::rejected_attempts()});
        return Err(StatusCode::FORBIDDEN);
    };

    let is_loopback =
        if let Some(ConnectInfo(addr)) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
            addr.ip().is_loopback()
        } else {
            is_loopback_host(host, state.port)
        };

    let is_ws = req.uri().path() == "/ws";

    if is_loopback {
        if !is_loopback_host(host, state.port) {
            crate::diag::auth_event!("auth.guard.rebinding_rejected",
                {"attempts": crate::diag::rejected_attempts()});
            return Err(StatusCode::FORBIDDEN);
        }
        if is_ws && !origin_matches_host(req.headers(), host, false) {
            crate::diag::auth_event!("auth.guard.origin_rejected",
                {"loopback": true, "attempts": crate::diag::rejected_attempts()});
            return Err(StatusCode::FORBIDDEN);
        }
        if let Err(status) = guard_state_change(&req, host) {
            crate::diag::auth_event!("auth.guard.csrf_rejected", {"loopback": true,
                "status": status.as_u16(), "attempts": crate::diag::rejected_attempts()});
            return Err(status);
        }
        crate::diag::auth_event!("auth.guard.loopback_accepted", {"ws": is_ws});
        req.extensions_mut()
            .insert(ClientTrust { is_loopback: true });
        return Ok(next.run(req).await);
    }

    // Petición desde otra IP (LAN)
    if !state.cfg.lan || state.cfg.token.is_empty() {
        crate::diag::auth_event!("auth.lan.disabled_rejected", {"lan_enabled": state.cfg.lan,
            "attempts": crate::diag::rejected_attempts()});
        return Err(StatusCode::FORBIDDEN);
    }
    if !is_lan_host(host, &state.lan_ip, state.port) {
        crate::diag::auth_event!("auth.lan.host_rejected",
            {"attempts": crate::diag::rejected_attempts()});
        return Err(StatusCode::FORBIDDEN);
    }
    if is_ws && !origin_matches_host(req.headers(), host, true) {
        crate::diag::auth_event!("auth.guard.origin_rejected",
            {"loopback": false, "attempts": crate::diag::rejected_attempts()});
        return Err(StatusCode::FORBIDDEN);
    }

    if let Err(status) = guard_state_change(&req, host) {
        crate::diag::auth_event!("auth.guard.csrf_rejected", {"loopback": false,
            "status": status.as_u16(), "attempts": crate::diag::rejected_attempts()});
        return Err(status);
    }

    let expected = state.cfg.token.as_bytes();
    let query_tok = extract_query_token(req.uri().query());
    let cookie_tok = extract_cookie_token(req.headers());

    let matched_query = query_tok
        .as_deref()
        .is_some_and(|t| constant_time_eq(t.as_bytes(), expected));
    let matched_cookie = cookie_tok
        .as_deref()
        .is_some_and(|t| constant_time_eq(t.as_bytes(), expected));

    if !matched_query && !matched_cookie {
        crate::diag::auth_event!("auth.lan.token_rejected", {"presented_query": query_tok.is_some(),
            "presented_cookie": cookie_tok.is_some(),
            "attempts": crate::diag::rejected_attempts()});
        return Err(StatusCode::FORBIDDEN);
    }
    // Los estáticos no se registran: solo la API y el WebSocket.
    if is_ws || req.uri().path().starts_with("/api/") {
        crate::diag::auth_event!("auth.lan.accepted", {"via_query": matched_query,
            "via_cookie": matched_cookie, "ws": is_ws});
    }

    req.extensions_mut()
        .insert(ClientTrust { is_loopback: false });
    let mut response = next.run(req).await;
    if matched_query {
        let cookie_header = format!(
            "tmc_token={}; Path=/; HttpOnly; SameSite=Strict",
            state.cfg.token
        );
        if let Ok(val) = HeaderValue::from_str(&cookie_header) {
            response.headers_mut().insert(header::SET_COOKIE, val);
        }
    }
    Ok(response)
}
