//! Cliente HTTP puro para la API de Inara (`InAPI v1`).

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use serde_json::Value;
use tmc_model::accounts::{InaraBatchEnvelope, InaraEvent, InaraHeader};
use tmc_model::sync::ErrorClass;

use crate::KeyCheck;
use crate::transport::{HttpReplyEx, TransportError};

pub const INARA_API_URL: &str = "https://inara.cz/inapi/v1/";
pub const INARA_APP_NAME: &str = "The Multipurpose Companion (TMC)";
pub const MAX_EVENTS_PER_BATCH: usize = 100;

/// Entrada de un lote a enviar a InAPI v1.
pub struct InaraBatch<'a> {
    pub api_key: &'a str,
    pub commander_name: &'a str,
    pub commander_fid: &'a str,
    /// Pares `(sync_outbox.id, evento JSON sin cabecera)`.
    pub events: &'a [(i64, Value)],
}

/// Error local al construir un lote para InAPI v1.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InaraLocalError {
    #[error("la lista de eventos está vacía")]
    Empty,
    #[error("el lote excede el máximo de {MAX_EVENTS_PER_BATCH} eventos ({0})")]
    TooMany(usize),
    #[error("clave API vacía")]
    MissingApiKey,
    #[error("evento inválido en el lote")]
    InvalidEvent,
}

/// Veredicto normalizado por evento o cabecera tras una petición a InAPI v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InaraVerdict {
    Sent {
        code: u16,
    },
    Rejected {
        code: u16,
    },
    Retry {
        class: ErrorClass,
        code: String,
        after: Option<Duration>,
    },
    AuthError,
}

/// Respuesta clasificada de InAPI v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InaraClassified {
    pub header: Option<InaraVerdict>,
    pub per_event: Vec<(i64, InaraVerdict)>,
}

fn non_empty(s: &str) -> Option<String> {
    let trimmed = s.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Construye el cuerpo JSON para `POST https://inara.cz/inapi/v1/`.
/// Nunca trunca lotes de más de 100 eventos; devuelve `Err(InaraLocalError::TooMany)`.
pub fn build_request(b: &InaraBatch<'_>) -> Result<Value, InaraLocalError> {
    if b.events.is_empty() {
        return Err(InaraLocalError::Empty);
    }
    if b.events.len() > MAX_EVENTS_PER_BATCH {
        return Err(InaraLocalError::TooMany(b.events.len()));
    }
    let Some(api_key) = non_empty(b.api_key) else {
        return Err(InaraLocalError::MissingApiKey);
    };

    let mut events = Vec::with_capacity(b.events.len());
    for (row_id, raw) in b.events {
        let Some(obj) = raw.as_object() else {
            return Err(InaraLocalError::InvalidEvent);
        };
        let Some(event_name) = obj.get("eventName").and_then(Value::as_str) else {
            return Err(InaraLocalError::InvalidEvent);
        };
        let Some(event_timestamp) = obj.get("eventTimestamp").and_then(Value::as_str) else {
            return Err(InaraLocalError::InvalidEvent);
        };
        let Some(event_data) = obj.get("eventData") else {
            return Err(InaraLocalError::InvalidEvent);
        };
        events.push(InaraEvent {
            event_name: event_name.to_string(),
            event_timestamp: event_timestamp.to_string(),
            event_custom_id: Some(*row_id),
            event_data: event_data.clone(),
        });
    }

    let envelope = InaraBatchEnvelope {
        header: InaraHeader {
            app_name: INARA_APP_NAME.to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            is_being_developed: cfg!(debug_assertions),
            api_key,
            commander_name: non_empty(b.commander_name),
            commander_frontier_id: non_empty(b.commander_fid),
        },
        events,
    };

    serde_json::to_value(envelope).map_err(|_| InaraLocalError::InvalidEvent)
}

/// Construye una petición `getCommanderProfile` para verificar una clave personal.
pub fn build_profile_request(
    api_key: &str,
    commander_name: &str,
    commander_fid: Option<&str>,
) -> Value {
    let event_data = non_empty(commander_name)
        .map(|name| serde_json::json!({ "searchName": name }))
        .unwrap_or_else(|| serde_json::json!({}));
    let envelope = InaraBatchEnvelope {
        header: InaraHeader {
            app_name: INARA_APP_NAME.to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            is_being_developed: cfg!(debug_assertions),
            api_key: api_key.trim().to_string(),
            commander_name: non_empty(commander_name),
            commander_frontier_id: commander_fid.and_then(non_empty),
        },
        events: vec![InaraEvent {
            event_name: "getCommanderProfile".to_string(),
            event_timestamp: chrono::Utc::now().to_rfc3339(),
            event_custom_id: Some(1),
            event_data,
        }],
    };
    serde_json::to_value(envelope).unwrap_or(Value::Null)
}

fn retry_header(
    class: ErrorClass,
    code: impl Into<String>,
    after: Option<Duration>,
) -> InaraClassified {
    InaraClassified {
        header: Some(InaraVerdict::Retry {
            class,
            code: code.into(),
            after,
        }),
        per_event: Vec::new(),
    }
}

fn classify_event_entry(ev: &Value) -> InaraVerdict {
    let Some(code_u64) = ev.get("eventStatus").and_then(Value::as_u64) else {
        return InaraVerdict::Retry {
            class: ErrorClass::ServerError,
            code: "inara_missing_event_status".into(),
            after: None,
        };
    };
    let Ok(code) = u16::try_from(code_u64) else {
        return InaraVerdict::Retry {
            class: ErrorClass::ServerError,
            code: format!("inara_event_{code_u64}"),
            after: None,
        };
    };
    match code {
        200 | 202 | 204 => InaraVerdict::Sent { code },
        400 => InaraVerdict::Rejected { code },
        _ => InaraVerdict::Retry {
            class: ErrorClass::ServerError,
            code: format!("inara_event_{code}"),
            after: None,
        },
    }
}

/// Clasifica la respuesta completa (cabecera y por evento) de InAPI v1.
pub fn classify(reply: &Result<HttpReplyEx, TransportError>, sent_ids: &[i64]) -> InaraClassified {
    let r = match reply {
        Ok(r) => r,
        Err(TransportError::Timeout) => {
            return retry_header(ErrorClass::Timeout, "timeout", None);
        }
        Err(TransportError::Connect(_)) => {
            return retry_header(ErrorClass::Network, "network", None);
        }
    };

    match r.status {
        429 => {
            let after = Duration::from_secs(r.retry_after_s.unwrap_or(120));
            return retry_header(ErrorClass::RateLimited, "inara_429", Some(after));
        }
        403 => {
            return retry_header(
                ErrorClass::Blocked,
                "inara_403",
                Some(Duration::from_secs(15 * 60)),
            );
        }
        408 => return retry_header(ErrorClass::Timeout, "inara_408", None),
        s if !(200..300).contains(&s) => {
            return retry_header(ErrorClass::ServerError, format!("inara_http_{s}"), None);
        }
        _ => {}
    }

    let Ok(json) = serde_json::from_str::<Value>(&r.body) else {
        return retry_header(ErrorClass::ServerError, "inara_non_json", None);
    };

    let Some(header_code) = json
        .pointer("/header/eventStatus")
        .and_then(Value::as_u64)
        .and_then(|n| u16::try_from(n).ok())
    else {
        return retry_header(ErrorClass::ServerError, "inara_missing_header_status", None);
    };

    match header_code {
        400 => {
            return InaraClassified {
                header: Some(InaraVerdict::AuthError),
                per_event: Vec::new(),
            };
        }
        200 | 202 => {}
        other => {
            return retry_header(
                ErrorClass::ServerError,
                format!("inara_header_{other}"),
                None,
            );
        }
    }

    let resp_events = json
        .get("events")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let sent_set: HashSet<i64> = sent_ids.iter().copied().collect();
    let mut matched: HashMap<i64, InaraVerdict> = HashMap::new();

    for ev in resp_events {
        if let Some(cid) = ev.get("eventCustomID").and_then(Value::as_i64)
            && sent_set.contains(&cid)
        {
            matched
                .entry(cid)
                .or_insert_with(|| classify_event_entry(ev));
        }
    }

    if resp_events.len() == sent_ids.len() {
        for (idx, &id) in sent_ids.iter().enumerate() {
            if let std::collections::hash_map::Entry::Vacant(slot) = matched.entry(id) {
                let ev = &resp_events[idx];
                if ev.get("eventCustomID").is_none() {
                    slot.insert(classify_event_entry(ev));
                }
            }
        }
    }

    let per_event = sent_ids
        .iter()
        .map(|&id| {
            let verdict = matched.remove(&id).unwrap_or_else(|| InaraVerdict::Retry {
                class: ErrorClass::ServerError,
                code: "inara_unmatched_event".into(),
                after: None,
            });
            (id, verdict)
        })
        .collect();

    InaraClassified {
        header: None,
        per_event,
    }
}

/// Clasifica la respuesta de verificación de clave (`getCommanderProfile`).
pub fn classify_profile(reply: &Result<HttpReplyEx, TransportError>) -> KeyCheck {
    let classified = classify(reply, &[1]);
    match classified.header {
        Some(InaraVerdict::AuthError) => return KeyCheck::Rejected,
        Some(_) => return KeyCheck::Unreachable,
        None => {}
    }
    match classified.per_event.first().map(|(_, v)| v) {
        Some(InaraVerdict::Sent { .. }) => KeyCheck::Valid,
        Some(InaraVerdict::Rejected { .. } | InaraVerdict::AuthError) => KeyCheck::Rejected,
        Some(InaraVerdict::Retry { .. }) | None => KeyCheck::Unreachable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_event(name: &str) -> Value {
        json!({
            "eventName": name,
            "eventTimestamp": "2026-11-10T18:00:00Z",
            "eventData": { "starsystemName": "Sol" }
        })
    }

    fn ok_reply(
        status: u16,
        body: &str,
        retry_after_s: Option<u64>,
    ) -> Result<HttpReplyEx, TransportError> {
        Ok(HttpReplyEx {
            status,
            body: body.to_string(),
            retry_after_s,
            rate_limit_remaining: None,
            rate_limit_reset_s: None,
        })
    }

    #[test]
    fn header_has_exact_app_name_and_is_being_developed() {
        let events = [(10, sample_event("setCommanderTravelLocation"))];
        let req = build_request(&InaraBatch {
            api_key: "fake-key",
            commander_name: "ExampleCmdr",
            commander_fid: "F000001",
            events: &events,
        })
        .unwrap();
        let h = &req["header"];
        assert_eq!(h["appName"], INARA_APP_NAME);
        assert_eq!(h["appVersion"], env!("CARGO_PKG_VERSION"));
        assert_eq!(h["isBeingDeveloped"], cfg!(debug_assertions));
        assert_eq!(h["APIkey"], "fake-key");
        assert_eq!(h["commanderName"], "ExampleCmdr");
        assert_eq!(h["commanderFrontierID"], "F000001");
    }

    #[test]
    fn event_custom_id_is_row_id() {
        let events = [
            (41, sample_event("addCommanderTravelFSDJump")),
            (42, sample_event("addCommanderTravelDock")),
        ];
        let req = build_request(&InaraBatch {
            api_key: "fake-key",
            commander_name: "ExampleCmdr",
            commander_fid: "F000001",
            events: &events,
        })
        .unwrap();
        let arr = req["events"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["eventCustomID"], 41);
        assert_eq!(arr[1]["eventCustomID"], 42);
    }

    #[test]
    fn more_than_100_is_error_not_truncation() {
        let events: Vec<(i64, Value)> = (1..=101)
            .map(|id| (id, sample_event("setCommanderTravelLocation")))
            .collect();
        let err = build_request(&InaraBatch {
            api_key: "fake-key",
            commander_name: "ExampleCmdr",
            commander_fid: "F000001",
            events: &events,
        })
        .unwrap_err();
        assert_eq!(err, InaraLocalError::TooMany(101));

        let exact_100 = &events[..MAX_EVENTS_PER_BATCH];
        let ok = build_request(&InaraBatch {
            api_key: "fake-key",
            commander_name: "ExampleCmdr",
            commander_fid: "F000001",
            events: exact_100,
        })
        .unwrap();
        assert_eq!(ok["events"].as_array().unwrap().len(), 100);
    }

    #[test]
    fn event_status_table() {
        let body_ok = include_str!("../../../tests/fixtures/sync/inara/response_ok.json");
        let c = classify(&ok_reply(200, body_ok, None), &[101, 102, 103]);
        assert!(c.header.is_none());
        assert_eq!(
            c.per_event,
            vec![
                (101, InaraVerdict::Sent { code: 200 }),
                (102, InaraVerdict::Sent { code: 202 }),
                (103, InaraVerdict::Sent { code: 204 }),
            ]
        );

        let body_mixed = include_str!("../../../tests/fixtures/sync/inara/response_mixed.json");
        let cm = classify(&ok_reply(200, body_mixed, None), &[201, 202, 203]);
        assert!(cm.header.is_none());
        assert_eq!(
            cm.per_event,
            vec![
                (201, InaraVerdict::Sent { code: 200 }),
                (202, InaraVerdict::Rejected { code: 400 }),
                (203, InaraVerdict::Sent { code: 204 }),
            ]
        );

        let body_h400 = include_str!("../../../tests/fixtures/sync/inara/response_header_400.json");
        let ch = classify(&ok_reply(200, body_h400, None), &[1, 2]);
        assert_eq!(ch.header, Some(InaraVerdict::AuthError));
        assert!(ch.per_event.is_empty());

        let body_nostat =
            include_str!("../../../tests/fixtures/sync/inara/response_no_status.json");
        let cn = classify(&ok_reply(200, body_nostat, None), &[301]);
        assert!(matches!(
            cn.header,
            Some(InaraVerdict::Retry {
                class: ErrorClass::ServerError,
                ..
            })
        ));
    }

    #[test]
    fn pairs_by_custom_id_then_by_position_only_if_same_count() {
        // Reordenado por eventCustomID
        let body = json!({
            "header": { "eventStatus": 200 },
            "events": [
                { "eventCustomID": 20, "eventStatus": 400 },
                { "eventCustomID": 10, "eventStatus": 200 }
            ]
        })
        .to_string();
        let c = classify(&ok_reply(200, &body, None), &[10, 20]);
        assert_eq!(
            c.per_event,
            vec![
                (10, InaraVerdict::Sent { code: 200 }),
                (20, InaraVerdict::Rejected { code: 400 }),
            ]
        );

        // Sin eventCustomID pero misma longitud: empareja por posición
        let pos_body = json!({
            "header": { "eventStatus": 200 },
            "events": [
                { "eventStatus": 200 },
                { "eventStatus": 204 }
            ]
        })
        .to_string();
        let cp = classify(&ok_reply(200, &pos_body, None), &[50, 60]);
        assert_eq!(
            cp.per_event,
            vec![
                (50, InaraVerdict::Sent { code: 200 }),
                (60, InaraVerdict::Sent { code: 204 }),
            ]
        );
    }

    #[test]
    fn unmatched_event_is_retry() {
        // Sin eventCustomID y longitud distinta: no empareja posicionalmente; todo queda Retry
        let body = json!({
            "header": { "eventStatus": 200 },
            "events": [{ "eventStatus": 200 }]
        })
        .to_string();
        let c = classify(&ok_reply(200, &body, None), &[1, 2]);
        assert_eq!(c.per_event.len(), 2);
        assert!(matches!(c.per_event[0].1, InaraVerdict::Retry { .. }));
        assert!(matches!(c.per_event[1].1, InaraVerdict::Retry { .. }));

        // Uno emparejado por custom ID y el otro ausente: el ausente queda Retry
        let partial = json!({
            "header": { "eventStatus": 200 },
            "events": [{ "eventCustomID": 2, "eventStatus": 200 }]
        })
        .to_string();
        let cp = classify(&ok_reply(200, &partial, None), &[1, 2]);
        assert!(matches!(cp.per_event[0], (1, InaraVerdict::Retry { .. })));
        assert_eq!(cp.per_event[1], (2, InaraVerdict::Sent { code: 200 }));
    }

    #[test]
    fn http_429_retry_after_or_120s() {
        let with_header = classify(&ok_reply(429, "slow down", Some(45)), &[1]);
        assert_eq!(
            with_header.header,
            Some(InaraVerdict::Retry {
                class: ErrorClass::RateLimited,
                code: "inara_429".into(),
                after: Some(Duration::from_secs(45)),
            })
        );

        let fallback = classify(&ok_reply(429, "slow down", None), &[1]);
        assert_eq!(
            fallback.header,
            Some(InaraVerdict::Retry {
                class: ErrorClass::RateLimited,
                code: "inara_429".into(),
                after: Some(Duration::from_secs(120)),
            })
        );
    }

    #[test]
    fn html_200_body_is_retry_not_success() {
        let html = include_str!("../../../tests/fixtures/sync/inara/response_html_200.html");
        let c = classify(&ok_reply(200, html, None), &[1]);
        assert!(matches!(
            c.header,
            Some(InaraVerdict::Retry {
                class: ErrorClass::ServerError,
                ..
            })
        ));
    }

    #[test]
    fn profile_classification() {
        let ok = include_str!("../../../tests/fixtures/sync/inara/profile_ok.json");
        assert_eq!(classify_profile(&ok_reply(200, ok, None)), KeyCheck::Valid);

        let bad = include_str!("../../../tests/fixtures/sync/inara/response_header_400.json");
        assert_eq!(
            classify_profile(&ok_reply(200, bad, None)),
            KeyCheck::Rejected
        );

        let html = include_str!("../../../tests/fixtures/sync/inara/response_html_200.html");
        assert_eq!(
            classify_profile(&ok_reply(200, html, None)),
            KeyCheck::Unreachable
        );
    }
}
