//! Cliente HTTP puro para la API de diario de EDSM (`api-journal-v1` y `api-commander-v1/get-ranks`).

use std::collections::HashSet;
use std::time::Duration;

use serde_json::{Value, json};
use tmc_model::sync::ErrorClass;

use crate::KeyCheck;
use crate::transport::{HttpReplyEx, TransportError, url_encode};

pub const EDSM_JOURNAL_URL: &str = "https://www.edsm.net/api-journal-v1";
pub const EDSM_DISCARD_URL: &str = "https://www.edsm.net/api-journal-v1/discard";
pub const EDSM_GET_RANKS_URL: &str = "https://www.edsm.net/api-commander-v1/get-ranks";
pub const EDSM_SOFTWARE_NAME: &str = "The Multipurpose Companion (TMC)";

pub struct EdsmBatch<'a> {
    pub commander_name: &'a str,
    pub api_key: &'a str,
    pub game_version: &'a str,
    pub game_build: &'a str,
    pub messages: &'a [Value],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdsmVerdict {
    Sent {
        code: u32,
    },
    Rejected {
        code: u32,
        class: ErrorClass,
    },
    Discarded {
        code: u32,
    },
    BenignRejected {
        code: u32,
    },
    CredentialError {
        code: u32,
    },
    SoftwareBlocked {
        code: u32,
    },
    Retry {
        class: ErrorClass,
        code: String,
        after: Option<Duration>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdsmClassified {
    pub global: Option<EdsmVerdict>,
    pub per_event: Vec<EdsmVerdict>,
}

pub fn get_ranks_url(commander_name: &str, api_key: &str) -> String {
    format!(
        "{EDSM_GET_RANKS_URL}?commanderName={}&apiKey={}",
        url_encode(commander_name),
        url_encode(api_key)
    )
}

pub fn build_request(b: &EdsmBatch<'_>) -> Value {
    json!({
        "commanderName": b.commander_name,
        "apiKey": b.api_key,
        "fromSoftware": EDSM_SOFTWARE_NAME,
        "fromSoftwareVersion": env!("CARGO_PKG_VERSION"),
        "fromGameVersion": b.game_version,
        "fromGameBuild": b.game_build,
        "message": b.messages,
    })
}

pub fn parse_discard(body: &str) -> Result<HashSet<String>, String> {
    let entries: Vec<String> =
        serde_json::from_str(body).map_err(|e| format!("discard JSON inválido: {e}"))?;
    Ok(entries.into_iter().filter(|s| !s.is_empty()).collect())
}

fn classify_msgnum(code: u32) -> EdsmVerdict {
    match code {
        100..=104 => EdsmVerdict::Sent { code },
        201..=203 => EdsmVerdict::CredentialError { code },
        204 | 205 => EdsmVerdict::SoftwareBlocked { code },
        206..=208 | 301..=303 | 401 | 402 => EdsmVerdict::Rejected {
            code,
            class: ErrorClass::InvalidEvent,
        },
        304 => EdsmVerdict::Discarded { code },
        451..=453 => EdsmVerdict::BenignRejected { code },
        _ => EdsmVerdict::Retry {
            class: ErrorClass::ServerError,
            code: format!("edsm_{code}"),
            after: None,
        },
    }
}

fn read_msgnum(v: &Value) -> Option<u32> {
    v.get("msgnum")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
}

fn retry_global(
    class: ErrorClass,
    code: impl Into<String>,
    after: Option<Duration>,
) -> EdsmClassified {
    EdsmClassified {
        global: Some(EdsmVerdict::Retry {
            class,
            code: code.into(),
            after,
        }),
        per_event: Vec::new(),
    }
}

pub fn classify(reply: &Result<HttpReplyEx, TransportError>, n_events: usize) -> EdsmClassified {
    let r = match reply {
        Ok(r) => r,
        Err(TransportError::Timeout) => {
            return retry_global(ErrorClass::Timeout, "timeout", None);
        }
        Err(TransportError::Connect(_)) => {
            return retry_global(ErrorClass::Network, "network", None);
        }
    };

    match r.status {
        429 => {
            let after = r
                .retry_after_s
                .or(r.rate_limit_reset_s)
                .map(Duration::from_secs);
            return retry_global(ErrorClass::RateLimited, "edsm_429", after);
        }
        403 => {
            return retry_global(
                ErrorClass::Blocked,
                "edsm_403",
                Some(Duration::from_secs(15 * 60)),
            );
        }
        408 => {
            return retry_global(ErrorClass::Timeout, "edsm_408", None);
        }
        s if !(200..300).contains(&s) => {
            return retry_global(ErrorClass::ServerError, format!("edsm_{s}"), None);
        }
        _ => {}
    }

    let Ok(json) = serde_json::from_str::<Value>(&r.body) else {
        return retry_global(ErrorClass::ServerError, "edsm_non_json", None);
    };

    let Some(root_code) = read_msgnum(&json) else {
        return retry_global(ErrorClass::ServerError, "edsm_missing_msgnum", None);
    };

    let root_verdict = classify_msgnum(root_code);
    if !matches!(root_verdict, EdsmVerdict::Sent { .. }) {
        return EdsmClassified {
            global: Some(root_verdict),
            per_event: Vec::new(),
        };
    }

    match json.get("events") {
        Some(Value::Array(events)) => {
            if events.len() != n_events {
                return retry_global(ErrorClass::ServerError, "edsm_length_mismatch", None);
            }
            let per_event = events
                .iter()
                .map(|ev| match read_msgnum(ev) {
                    Some(c) => classify_msgnum(c),
                    None => EdsmVerdict::Retry {
                        class: ErrorClass::ServerError,
                        code: "edsm_missing_event_msgnum".into(),
                        after: None,
                    },
                })
                .collect();
            EdsmClassified {
                global: None,
                per_event,
            }
        }
        None if n_events <= 1 => EdsmClassified {
            global: Some(root_verdict),
            per_event: Vec::new(),
        },
        None => retry_global(ErrorClass::ServerError, "edsm_missing_events", None),
        Some(_) => retry_global(ErrorClass::ServerError, "edsm_invalid_events", None),
    }
}

pub fn classify_get_ranks(reply: &Result<HttpReplyEx, TransportError>) -> KeyCheck {
    let Ok(r) = reply else {
        return KeyCheck::Unreachable;
    };
    if !(200..300).contains(&r.status) {
        return KeyCheck::Unreachable;
    }
    let Ok(json) = serde_json::from_str::<Value>(&r.body) else {
        return KeyCheck::Unreachable;
    };
    match read_msgnum(&json) {
        Some(100 | 207) => KeyCheck::Valid,
        Some(201..=203) => KeyCheck::Rejected,
        _ => KeyCheck::Unreachable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_reply(status: u16, body: &str) -> Result<HttpReplyEx, TransportError> {
        Ok(HttpReplyEx {
            status,
            body: body.to_string(),
            retry_after_s: None,
            rate_limit_remaining: None,
            rate_limit_reset_s: None,
        })
    }

    #[test]
    fn request_matches_fixture() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/sync/edsm/request_batch.json"
        ))
        .expect("fixture válida");
        let messages = fixture["message"]
            .as_array()
            .expect("message debe ser un array")
            .clone();
        let req = build_request(&EdsmBatch {
            commander_name: "CMDR Ficticio",
            api_key: "EXAMPLE-EDSM-API-KEY",
            game_version: "4.0.0.1903",
            game_build: "r310123/r0 ",
            messages: &messages,
        });
        assert_eq!(
            serde_json::to_string(&req).unwrap(),
            serde_json::to_string(&fixture).unwrap()
        );
        assert_eq!(req["fromSoftware"], "The Multipurpose Companion (TMC)");
        assert_eq!(req["fromGameBuild"], "r310123/r0 ");
    }

    #[test]
    fn msgnum_table() {
        let cases = [
            (100, EdsmVerdict::Sent { code: 100 }),
            (101, EdsmVerdict::Sent { code: 101 }),
            (102, EdsmVerdict::Sent { code: 102 }),
            (103, EdsmVerdict::Sent { code: 103 }),
            (104, EdsmVerdict::Sent { code: 104 }),
            (201, EdsmVerdict::CredentialError { code: 201 }),
            (202, EdsmVerdict::CredentialError { code: 202 }),
            (203, EdsmVerdict::CredentialError { code: 203 }),
            (204, EdsmVerdict::SoftwareBlocked { code: 204 }),
            (205, EdsmVerdict::SoftwareBlocked { code: 205 }),
            (
                206,
                EdsmVerdict::Rejected {
                    code: 206,
                    class: ErrorClass::InvalidEvent,
                },
            ),
            (
                207,
                EdsmVerdict::Rejected {
                    code: 207,
                    class: ErrorClass::InvalidEvent,
                },
            ),
            (
                208,
                EdsmVerdict::Rejected {
                    code: 208,
                    class: ErrorClass::InvalidEvent,
                },
            ),
            (
                301,
                EdsmVerdict::Rejected {
                    code: 301,
                    class: ErrorClass::InvalidEvent,
                },
            ),
            (
                302,
                EdsmVerdict::Rejected {
                    code: 302,
                    class: ErrorClass::InvalidEvent,
                },
            ),
            (
                303,
                EdsmVerdict::Rejected {
                    code: 303,
                    class: ErrorClass::InvalidEvent,
                },
            ),
            (304, EdsmVerdict::Discarded { code: 304 }),
            (
                401,
                EdsmVerdict::Rejected {
                    code: 401,
                    class: ErrorClass::InvalidEvent,
                },
            ),
            (
                402,
                EdsmVerdict::Rejected {
                    code: 402,
                    class: ErrorClass::InvalidEvent,
                },
            ),
            (451, EdsmVerdict::BenignRejected { code: 451 }),
            (452, EdsmVerdict::BenignRejected { code: 452 }),
            (453, EdsmVerdict::BenignRejected { code: 453 }),
            (
                500,
                EdsmVerdict::Retry {
                    class: ErrorClass::ServerError,
                    code: "edsm_500".into(),
                    after: None,
                },
            ),
            (
                501,
                EdsmVerdict::Retry {
                    class: ErrorClass::ServerError,
                    code: "edsm_501".into(),
                    after: None,
                },
            ),
        ];

        for (code, expected) in cases {
            let body = json!({ "msgnum": code, "msg": "test" }).to_string();
            let got = classify(&ok_reply(200, &body), 1);
            assert_eq!(got.global, Some(expected), "msgnum {code}");
            assert!(got.per_event.is_empty(), "msgnum {code}");
        }
    }

    #[test]
    fn http_429_uses_retry_after_then_rate_limit_reset() {
        let with_retry_after = Ok(HttpReplyEx {
            status: 429,
            body: "rate limit".into(),
            retry_after_s: Some(15),
            rate_limit_remaining: Some(0),
            rate_limit_reset_s: Some(45),
        });
        let got1 = classify(&with_retry_after, 2);
        assert_eq!(
            got1.global,
            Some(EdsmVerdict::Retry {
                class: ErrorClass::RateLimited,
                code: "edsm_429".into(),
                after: Some(Duration::from_secs(15)),
            })
        );

        let with_reset_only = Ok(HttpReplyEx {
            status: 429,
            body: "rate limit".into(),
            retry_after_s: None,
            rate_limit_remaining: Some(0),
            rate_limit_reset_s: Some(45),
        });
        let got2 = classify(&with_reset_only, 2);
        assert_eq!(
            got2.global,
            Some(EdsmVerdict::Retry {
                class: ErrorClass::RateLimited,
                code: "edsm_429".into(),
                after: Some(Duration::from_secs(45)),
            })
        );
    }

    #[test]
    fn cloudflare_html_403_is_blocked_retry_15min() {
        let html = include_str!("../../../tests/fixtures/sync/edsm/response_html_403.html");
        let got = classify(&ok_reply(403, html), 1);
        assert_eq!(
            got.global,
            Some(EdsmVerdict::Retry {
                class: ErrorClass::Blocked,
                code: "edsm_403".into(),
                after: Some(Duration::from_secs(15 * 60)),
            })
        );
    }

    #[test]
    fn html_200_is_unknown_retry_never_sent() {
        let html = include_str!("../../../tests/fixtures/sync/edsm/response_html_200.html");
        let got = classify(&ok_reply(200, html), 1);
        assert!(got.per_event.is_empty());
        assert!(matches!(got.global, Some(EdsmVerdict::Retry { .. })));
    }

    #[test]
    fn per_event_length_mismatch_retries_all() {
        let body = include_str!("../../../tests/fixtures/sync/edsm/response_ok.json");
        // response_ok.json trae 2 eventos; si enviamos 3 debe reintentar todo el lote.
        let got = classify(&ok_reply(200, body), 3);
        assert!(got.per_event.is_empty());
        assert!(matches!(
            got.global,
            Some(EdsmVerdict::Retry {
                class: ErrorClass::ServerError,
                ..
            })
        ));
    }

    #[test]
    fn discard_list_parses_fixture() {
        let body = include_str!("../../../tests/fixtures/sync/edsm/discard.json");
        let set = parse_discard(body).expect("parse_discard válido");
        assert!(set.len() >= 140);
        assert!(set.contains("ShutDown"));
        assert!(set.contains("ScanOrganic"));
        assert!(!set.contains("FSDJump"));
        assert!(parse_discard("{}").is_err());
    }

    #[test]
    fn get_ranks_valid_and_bad_key() {
        let ok_body = include_str!("../../../tests/fixtures/sync/edsm/get_ranks_ok.json");
        assert_eq!(classify_get_ranks(&ok_reply(200, ok_body)), KeyCheck::Valid);

        let no_ranks = json!({ "msgnum": 207, "msg": "No ranks stored" }).to_string();
        assert_eq!(
            classify_get_ranks(&ok_reply(200, &no_ranks)),
            KeyCheck::Valid
        );

        let bad_body = include_str!("../../../tests/fixtures/sync/edsm/get_ranks_bad_key.json");
        assert_eq!(
            classify_get_ranks(&ok_reply(200, bad_body)),
            KeyCheck::Rejected
        );

        let html = include_str!("../../../tests/fixtures/sync/edsm/response_html_403.html");
        assert_eq!(
            classify_get_ranks(&ok_reply(403, html)),
            KeyCheck::Unreachable
        );
        assert_eq!(
            classify_get_ranks(&Err(TransportError::Timeout)),
            KeyCheck::Unreachable
        );
    }
}
