//! Acción `app.diagnostics`: lotes de registros de la UI. Solo existe en depuración.
//!
//! Validación estricta (lista cerrada de códigos, tamaño y número acotados), tasa
//! máxima de un lote por segundo y redacción con `DiagnosticContext::Ui`.

use crate::ActionError;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub(crate) const ACTION: &str = "app.diagnostics";
pub(crate) const MAX_RECORDS: usize = 100;
pub(crate) const MAX_BYTES: usize = 8 * 1024;
pub(crate) const MAX_PAYLOAD_BYTES: usize = 2 * 1024;
/// Intervalo mínimo entre lotes (la UI envía como máximo uno por segundo).
const MIN_INTERVAL: Duration = Duration::from_millis(900);

/// Códigos admitidos (lista cerrada; el prefijo `ui.` los enruta a `diag_ui`).
pub(crate) const CODES: &[&str] = &[
    "ui.diagnostics.enabled",
    "ui.queue.dropped",
    "ui.ws.hello",
    "ui.ws.snapshot",
    "ui.ws.update",
    "ui.ws.gap",
    "ui.ws.alert",
    "ui.ws.import_progress",
    "ui.ws.connection",
    "ui.ws.fault",
    "ui.ws.subscribe",
    "ui.topic.applied",
    "ui.action.sent",
    "ui.action.result",
    "ui.app.layout",
    "ui.panel.mounted",
    "ui.panel.unmounted",
    "ui.list.render",
    "ui.card.system",
    "ui.card.route_strip",
    "ui.topbar.state",
    "ui.toast.shown",
    "ui.toast.dismissed",
    "ui.import.progress",
    "ui.fetch.result",
    "ui.window.fault",
];

/// Máximo de clientes recordados a la vez (acota la memoria ante claves hostiles).
const MAX_CLIENTS: usize = 64;

/// Límite de tasa independiente por cliente (conexión WS o par IP+origen en REST).
#[derive(Default)]
pub(crate) struct Limiter(Mutex<HashMap<String, Instant>>);

impl Limiter {
    fn admit(&self, client: &str, now: Instant) -> bool {
        let mut map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if map
            .get(client)
            .is_some_and(|prev| now.saturating_duration_since(*prev) < MIN_INTERVAL)
        {
            return false;
        }
        if map.len() >= MAX_CLIENTS && !map.contains_key(client) {
            map.retain(|_, prev| now.saturating_duration_since(*prev) < MIN_INTERVAL);
            if map.len() >= MAX_CLIENTS {
                return false;
            }
        }
        map.insert(client.to_owned(), now);
        true
    }
}

static LIMITER: std::sync::LazyLock<Limiter> = std::sync::LazyLock::new(Limiter::default);

pub(crate) fn handle(client: &str, args: &Value) -> Result<Value, ActionError> {
    handle_with(&LIMITER, client, args, Instant::now())
}

fn invalid() -> ActionError {
    ActionError::Failed("invalid-batch".into())
}

fn validate(args: &Value) -> Result<Vec<(&'static str, Value)>, ActionError> {
    let records = args.as_array().ok_or_else(invalid)?;
    if records.is_empty() || records.len() > MAX_RECORDS {
        return Err(invalid());
    }
    if serde_json::to_vec(args).map_or(usize::MAX, |b| b.len()) > MAX_BYTES {
        return Err(invalid());
    }
    let mut out = Vec::with_capacity(records.len());
    for record in records {
        let obj = record.as_object().ok_or_else(invalid)?;
        if obj.len() != 3 {
            return Err(invalid());
        }
        let code = obj
            .get("code")
            .and_then(Value::as_str)
            .ok_or_else(invalid)?;
        let code = CODES
            .iter()
            .copied()
            .find(|c| *c == code)
            .ok_or_else(invalid)?;
        let ts = obj.get("ts").and_then(Value::as_f64).ok_or_else(invalid)?;
        let payload = obj
            .get("payload")
            .filter(|p| p.is_object())
            .ok_or_else(invalid)?;
        if serde_json::to_vec(payload).map_or(usize::MAX, |b| b.len()) > MAX_PAYLOAD_BYTES {
            return Err(invalid());
        }
        let mut payload = payload.clone();
        payload["client_ts"] = json!(ts);
        out.push((code, payload));
    }
    Ok(out)
}

pub(crate) fn handle_with(
    limiter: &Limiter,
    client: &str,
    args: &Value,
    now: Instant,
) -> Result<Value, ActionError> {
    // Valida antes de consumir el turno: un lote inválido no bloquea al siguiente válido.
    let records = validate(args)?;
    if !limiter.admit(client, now) {
        return Err(ActionError::Failed("rate-limited".into()));
    }
    let accepted = records.len();
    for (code, payload) in records {
        tmc_model::diagnostics::emit(
            tmc_model::diagnostics::DiagnosticContext::Ui,
            code,
            &payload,
        );
    }
    Ok(json!({"accepted": accepted}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmc_model::diagnostics::capture::capture;

    fn rec(code: &str) -> Value {
        json!({"code": code, "ts": 12.5, "payload": {"tab": "layout:combat", "count": 3}})
    }

    #[test]
    fn accepts_valid_batch_and_redacts() {
        let limiter = Limiter::default();
        let batch = json!([
            rec("ui.app.layout"),
            {"code": "ui.ws.hello", "ts": 1, "payload": {
                "lan_token": "CANARYTOKEN", "pairing_url": "http://x/CANARYURL",
                "qr_svg": "<svg>CANARYSVG</svg>", "state": "CANARYTEXT",
                "view": "navigation", "topic": ["core.status", "CANARYTOPIC"]}}
        ]);
        let (result, records) = capture(|| handle_with(&limiter, "a", &batch, Instant::now()));
        assert_eq!(result.unwrap()["accepted"], 2);
        assert_eq!(records.len(), 2);
        assert!(records.iter().all(|r| r.context == "ui"));
        assert_eq!(records[0].code, "ui.app.layout");
        let all: String = records.iter().map(|r| r.payload.clone()).collect();
        assert!(!all.contains("CANARY"), "{all}");
        assert!(all.contains("layout:combat"));
        assert!(all.contains("navigation"));
        assert!(all.contains("core.status"));
    }

    #[test]
    fn rejects_invalid_shapes_and_unknown_codes() {
        let limiter = Limiter::default();
        let now = Instant::now();
        let long = "x".repeat(MAX_PAYLOAD_BYTES + 1);
        let too_many: Vec<Value> = (0..=MAX_RECORDS).map(|_| rec("ui.ws.update")).collect();
        let big: Vec<Value> = (0..10)
            .map(|_| json!({"code": "ui.ws.update", "ts": 1, "payload": {"a": "y".repeat(1000)}}))
            .collect();
        let cases = [
            json!({"code": "ui.ws.update"}),
            json!([]),
            json!([rec("http.request")]),
            json!([rec("ui.not.listed")]),
            json!([{"code": "ui.ws.update", "ts": "1", "payload": {}}]),
            json!([{"code": "ui.ws.update", "ts": 1, "payload": []}]),
            json!([{"code": "ui.ws.update", "ts": 1, "payload": {}, "extra": 1}]),
            json!([{"code": "ui.ws.update", "ts": 1, "payload": {"a": long}}]),
            json!(too_many),
            json!(big),
            Value::Null,
        ];
        for case in cases {
            assert!(
                matches!(
                    handle_with(&limiter, "a", &case, now),
                    Err(ActionError::Failed(_))
                ),
                "{case}"
            );
        }
        // Los lotes inválidos no consumen el turno de tasa.
        assert!(handle_with(&limiter, "a", &json!([rec("ui.ws.update")]), now).is_ok());
    }

    #[test]
    fn rate_limit_allows_one_batch_per_interval() {
        let limiter = Limiter::default();
        let t0 = Instant::now();
        let batch = json!([rec("ui.ws.update")]);
        assert!(handle_with(&limiter, "a", &batch, t0).is_ok());
        assert!(matches!(
            handle_with(&limiter, "a", &batch, t0 + Duration::from_millis(500)),
            Err(ActionError::Failed(reason)) if reason == "rate-limited"
        ));
        assert!(handle_with(&limiter, "a", &batch, t0 + Duration::from_millis(1000)).is_ok());
    }

    #[test]
    fn clients_do_not_block_each_other() {
        let limiter = Limiter::default();
        let t0 = Instant::now();
        let batch = json!([rec("ui.ws.update")]);
        assert!(handle_with(&limiter, "ws:1", &batch, t0).is_ok());
        assert!(handle_with(&limiter, "ws:2", &batch, t0).is_ok());
        assert!(handle_with(&limiter, "ws:1", &batch, t0).is_err());
        assert!(handle_with(&limiter, "rest:x", &batch, t0).is_ok());
    }

    #[test]
    fn client_table_is_bounded() {
        let limiter = Limiter::default();
        let t0 = Instant::now();
        let batch = json!([rec("ui.ws.update")]);
        for i in 0..MAX_CLIENTS {
            assert!(handle_with(&limiter, &format!("c{i}"), &batch, t0).is_ok());
        }
        assert!(handle_with(&limiter, "extra", &batch, t0).is_err());
        let later = t0 + Duration::from_secs(2);
        assert!(handle_with(&limiter, "extra", &batch, later).is_ok());
    }

    #[test]
    fn every_code_has_ui_prefix() {
        assert!(CODES.iter().all(|c| c.starts_with("ui.")));
    }

    /// Una carga representativa por código, con la forma que emite la UI (`ui/src`).
    fn reference_payloads() -> Vec<(&'static str, Value)> {
        vec![
            ("ui.diagnostics.enabled", json!({"max_queue": 200})),
            ("ui.queue.dropped", json!({"count": 4})),
            ("ui.ws.hello", json!({"topics": 3})),
            ("ui.ws.snapshot", json!({"has_seq": true, "seq": 7})),
            ("ui.ws.update", json!({"seq": 8})),
            ("ui.ws.gap", json!({"expected": 9, "got": 11})),
            ("ui.ws.alert", json!({"level": "warning", "listeners": 1})),
            ("ui.ws.import_progress", json!({"done": 1, "total": 5})),
            (
                "ui.ws.connection",
                json!({"state": "open", "hidden": false}),
            ),
            (
                "ui.ws.connection",
                json!({"state": "reconnect", "attempt": 2, "delay_ms": 500}),
            ),
            ("ui.ws.connection", json!({"state": "closed", "code": 1006})),
            ("ui.ws.fault", json!({"kind": "parse", "size": 12})),
            ("ui.ws.subscribe", json!({"hidden": true, "patterns": 1})),
            (
                "ui.topic.applied",
                json!({"snapshot": false, "count": 2,
                "topic": ["core.status", "core.location"]}),
            ),
            (
                "ui.action.sent",
                json!({"action": "navigation.plan", "has_args": true}),
            ),
            (
                "ui.action.result",
                json!({"action": "navigation.plan", "ok": true,
                "status": 200, "duration_ms": 12}),
            ),
            (
                "ui.app.layout",
                json!({"tab": "layout:combat", "layout": "combat",
                "pinned": false, "slot": ["target", "ax"], "defined_slots": 5,
                "module": ["combat", "mining"], "relevance": ["active", "normal"],
                "context": "exploration", "manual_context": "none",
                "ax_active": false, "ax_presence": true, "cz_present": false}),
            ),
            (
                "ui.panel.mounted",
                json!({"panel": "panel-route-strip", "span": 0,
                "large": false, "has_children": true}),
            ),
            ("ui.panel.unmounted", json!({"panel": "panel-route-strip"})),
            (
                "ui.list.render",
                json!({"items": 50, "row_height": 24, "height": 300,
                "start": 0, "end": 12}),
            ),
            (
                "ui.card.system",
                json!({"system": "Sol", "big": true, "state": "ok",
                "remote_ok": true, "cache": false, "stale_known": false,
                "has_politics": true, "danger": false, "kind": "neutron",
                "scoopable": true, "reachable": true, "has_label": false,
                "has_star_classes": true}),
            ),
            (
                "ui.card.route_strip",
                json!({"has_route": true, "has_main": true,
                "has_next": true, "kind": "plan", "off_route": false, "following": 3,
                "remaining_jumps": 9, "current_system_matches": true}),
            ),
            (
                "ui.topbar.state",
                json!({"connected": true, "has_location": true,
                "has_ship": true, "has_cmdr": true, "game_running": true,
                "galaxy": "Live", "mode": "Docked", "fuel_ratio": 0.5}),
            ),
            (
                "ui.toast.shown",
                json!({"level": "info", "replaced": false, "count": 1}),
            ),
            ("ui.toast.dismissed", json!({"found": true})),
            (
                "ui.import.progress",
                json!({"visible": true, "done": 1, "total": 5,
                "width_pct": 20}),
            ),
            (
                "ui.fetch.result",
                json!({"kind": "config-put", "ok": true,
                "status": 200, "lan": false}),
            ),
            (
                "ui.window.fault",
                json!({"kind": "error", "line": 3, "col": 4}),
            ),
        ]
    }

    #[test]
    fn reference_payloads_cover_every_code_without_redaction() {
        let limiter = Limiter::default();
        let refs = reference_payloads();
        for code in CODES {
            assert!(
                refs.iter().any(|(c, _)| c == code),
                "sin referencia: {code}"
            );
        }
        for (i, (code, payload)) in refs.iter().enumerate() {
            let batch = json!([{"code": code, "ts": 1, "payload": payload}]);
            let (result, records) =
                capture(|| handle_with(&limiter, &format!("c{i}"), &batch, Instant::now()));
            assert!(result.is_ok(), "{code}");
            assert_eq!(records.len(), 1);
            assert!(
                !records[0].payload.contains("[redacted]"),
                "{code}: {}",
                records[0].payload
            );
        }
    }
}
