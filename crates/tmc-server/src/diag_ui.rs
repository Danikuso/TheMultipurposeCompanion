//! Acción `app.diagnostics`: lotes de registros de la UI. Solo existe en depuración.
//!
//! Validación estricta (lista cerrada de códigos, tamaño y número acotados), tasa
//! máxima de un lote por segundo y redacción con `DiagnosticContext::Ui`.

use crate::ActionError;
use serde_json::{Value, json};
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

#[derive(Default)]
pub(crate) struct Limiter(Mutex<Option<Instant>>);

impl Limiter {
    fn admit(&self, now: Instant) -> bool {
        let mut last = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if last.is_some_and(|prev| now.saturating_duration_since(prev) < MIN_INTERVAL) {
            return false;
        }
        *last = Some(now);
        true
    }
}

static LIMITER: Limiter = Limiter(Mutex::new(None));

pub(crate) fn handle(args: &Value) -> Result<Value, ActionError> {
    handle_with(&LIMITER, args, Instant::now())
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
    args: &Value,
    now: Instant,
) -> Result<Value, ActionError> {
    // Valida antes de consumir el turno: un lote inválido no bloquea al siguiente válido.
    let records = validate(args)?;
    if !limiter.admit(now) {
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
                "view": "ok-id", "topic": ["core.status", "CANARYTOPIC"]}}
        ]);
        let (result, records) = capture(|| handle_with(&limiter, &batch, Instant::now()));
        assert_eq!(result.unwrap()["accepted"], 2);
        assert_eq!(records.len(), 2);
        assert!(records.iter().all(|r| r.context == "ui"));
        assert_eq!(records[0].code, "ui.app.layout");
        let all: String = records.iter().map(|r| r.payload.clone()).collect();
        assert!(!all.contains("CANARY"), "{all}");
        assert!(all.contains("layout:combat"));
        assert!(all.contains("ok-id"));
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
                    handle_with(&limiter, &case, now),
                    Err(ActionError::Failed(_))
                ),
                "{case}"
            );
        }
        // Los lotes inválidos no consumen el turno de tasa.
        assert!(handle_with(&limiter, &json!([rec("ui.ws.update")]), now).is_ok());
    }

    #[test]
    fn rate_limit_allows_one_batch_per_interval() {
        let limiter = Limiter::default();
        let t0 = Instant::now();
        let batch = json!([rec("ui.ws.update")]);
        assert!(handle_with(&limiter, &batch, t0).is_ok());
        assert!(matches!(
            handle_with(&limiter, &batch, t0 + Duration::from_millis(500)),
            Err(ActionError::Failed(reason)) if reason == "rate-limited"
        ));
        assert!(handle_with(&limiter, &batch, t0 + Duration::from_millis(1000)).is_ok());
    }

    #[test]
    fn every_code_has_ui_prefix() {
        assert!(CODES.iter().all(|c| c.starts_with("ui.")));
    }
}
