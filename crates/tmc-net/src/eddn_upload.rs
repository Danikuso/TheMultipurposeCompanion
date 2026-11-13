//! Cliente HTTP de subida al gateway EDDN (protocolo puro conforme a C6).

use std::io::Write;

use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tmc_model::sync::ErrorClass;

use crate::transport::{HttpPost, HttpReplyEx, Transport, TransportError};

pub const EDDN_UPLOAD_URL: &str = "https://eddn.edcd.io:4430/upload/";
pub const EDDN_SOFTWARE_NAME: &str = "The Multipurpose Companion (TMC)";
pub const GZIP_THRESHOLD: usize = 16 * 1024;
pub const MAX_COMPRESSED: usize = 1024 * 1024;

/// Modo de esquema EDDN (`/test` para pruebas manuales, producción por defecto).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaMode {
    Live,
    Test,
}

/// Error local previo al envío HTTP.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EddnLocalError {
    #[error("error serializando envelope EDDN: {0}")]
    Serialize(String),
    #[error("el envelope EDDN supera 1 MiB ({0} bytes)")]
    TooLarge(usize),
}

/// Cuerpo codificado listo para el POST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedBody {
    pub bytes: Vec<u8>,
    pub gzip: bool,
}

/// Clasificación de la respuesta del gateway EDDN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EddnVerdict {
    Sent,
    Rejected {
        class: ErrorClass,
        code: String,
        hint: Option<String>,
    },
    SchemaOutdated {
        code: String,
    },
    Retry {
        class: ErrorClass,
        code: String,
    },
}

pub fn uploader_id(salt: &str, fid: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(fid.as_bytes());
    let hex = format!("{:x}", hasher.finalize());
    hex.chars().take(32).collect()
}

pub fn schema_ref(schema: &str, mode: SchemaMode) -> String {
    let base = schema.trim_start_matches('/').trim_end_matches('#');
    match mode {
        SchemaMode::Live => format!("https://eddn.edcd.io/schemas/{base}"),
        SchemaMode::Test => format!("https://eddn.edcd.io/schemas/{base}/test"),
    }
}

pub fn build_envelope(
    schema_ref: &str,
    uploader_id: &str,
    gameversion: &str,
    gamebuild: &str,
    message: &Value,
) -> Value {
    json!({
        "$schemaRef": schema_ref,
        "header": {
            "uploaderID": uploader_id,
            "softwareName": EDDN_SOFTWARE_NAME,
            "softwareVersion": env!("CARGO_PKG_VERSION"),
            "gameversion": gameversion,
            "gamebuild": gamebuild,
        },
        "message": message,
    })
}

pub fn encode_body(envelope: &Value) -> Result<EncodedBody, EddnLocalError> {
    let raw = serde_json::to_vec(envelope).map_err(|e| EddnLocalError::Serialize(e.to_string()))?;
    if raw.len() > GZIP_THRESHOLD {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&raw)
            .map_err(|e| EddnLocalError::Serialize(e.to_string()))?;
        let gz = encoder
            .finish()
            .map_err(|e| EddnLocalError::Serialize(e.to_string()))?;
        if gz.len() > MAX_COMPRESSED {
            return Err(EddnLocalError::TooLarge(gz.len()));
        }
        Ok(EncodedBody {
            bytes: gz,
            gzip: true,
        })
    } else {
        if raw.len() > MAX_COMPRESSED {
            return Err(EddnLocalError::TooLarge(raw.len()));
        }
        Ok(EncodedBody {
            bytes: raw,
            gzip: false,
        })
    }
}

fn sanitize_hint(raw: &str) -> Option<String> {
    let mut stripped = String::with_capacity(raw.len().min(256));
    let mut in_tag = false;
    let mut last_space = true;
    for ch in raw.chars() {
        match ch {
            '<' => {
                in_tag = true;
                if !last_space {
                    stripped.push(' ');
                    last_space = true;
                }
            }
            '>' => {
                in_tag = false;
            }
            _ if in_tag => {}
            _ if ch.is_whitespace() || ch.is_control() => {
                if !last_space {
                    stripped.push(' ');
                    last_space = true;
                }
            }
            _ => {
                stripped.push(ch);
                last_space = false;
            }
        }
        if stripped.chars().count() >= 120 && !in_tag {
            break;
        }
    }
    let trimmed: String = stripped.trim().chars().take(120).collect();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

pub fn classify(reply: &Result<HttpReplyEx, TransportError>) -> EddnVerdict {
    match reply {
        Ok(r) => match r.status {
            200..=299 => EddnVerdict::Sent,
            400 => EddnVerdict::Rejected {
                class: ErrorClass::InvalidEvent,
                code: "eddn_400".into(),
                hint: sanitize_hint(&r.body),
            },
            413 => EddnVerdict::Rejected {
                class: ErrorClass::TooLarge,
                code: "eddn_413".into(),
                hint: sanitize_hint(&r.body),
            },
            426 => EddnVerdict::SchemaOutdated {
                code: "eddn_426".into(),
            },
            408 => EddnVerdict::Retry {
                class: ErrorClass::Timeout,
                code: "eddn_408".into(),
            },
            429 => EddnVerdict::Retry {
                class: ErrorClass::RateLimited,
                code: "eddn_429".into(),
            },
            403 => EddnVerdict::Retry {
                class: ErrorClass::Blocked,
                code: "eddn_403".into(),
            },
            500..=599 => EddnVerdict::Retry {
                class: ErrorClass::ServerError,
                code: format!("eddn_{}", r.status),
            },
            s => EddnVerdict::Rejected {
                class: ErrorClass::InvalidEvent,
                code: format!("eddn_{s}"),
                hint: sanitize_hint(&r.body),
            },
        },
        Err(TransportError::Timeout) => EddnVerdict::Retry {
            class: ErrorClass::Timeout,
            code: "timeout".into(),
        },
        Err(TransportError::Connect(_)) => EddnVerdict::Retry {
            class: ErrorClass::Network,
            code: "network".into(),
        },
    }
}

pub async fn upload(t: &dyn Transport, body: &EncodedBody) -> Result<HttpReplyEx, TransportError> {
    let headers: &[(&str, &str)] = if body.gzip {
        &[("Content-Encoding", "gzip")]
    } else {
        &[]
    };
    t.post(HttpPost {
        url: EDDN_UPLOAD_URL,
        body: &body.bytes,
        content_type: "application/json",
        headers,
        http1_only: true,
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::ScriptedTransport;
    use flate2::read::GzDecoder;
    use std::io::Read;

    fn fixture_text(rel: &str) -> String {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests/fixtures/sync/eddn");
        std::fs::read_to_string(root.join(rel)).unwrap()
    }

    #[test]
    fn envelope_matches_spec_header() {
        let msg = json!({ "StarSystem": "Sistema Ficticio" });
        let ref_url = schema_ref("journal/1", SchemaMode::Live);
        assert_eq!(ref_url, "https://eddn.edcd.io/schemas/journal/1");
        assert_eq!(
            schema_ref("journal/1", SchemaMode::Test),
            "https://eddn.edcd.io/schemas/journal/1/test"
        );
        let env = build_envelope(&ref_url, "abcd", "4.0.0.1903", "r310123/r0 ", &msg);
        assert_eq!(env["$schemaRef"], "https://eddn.edcd.io/schemas/journal/1");
        let h = env["header"].as_object().unwrap();
        assert_eq!(h["uploaderID"], "abcd");
        assert_eq!(h["softwareName"], "The Multipurpose Companion (TMC)");
        assert_eq!(h["softwareVersion"], env!("CARGO_PKG_VERSION"));
        assert_eq!(h["gameversion"], "4.0.0.1903");
        assert_eq!(h["gamebuild"], "r310123/r0 ");
        assert!(!h.contains_key("gatewayTimestamp"));
        assert_eq!(env["message"], msg);
    }

    #[test]
    fn uploader_id_is_32_hex_and_differs_by_fid() {
        let a = uploader_id("test-salt", "F0000001");
        let b = uploader_id("test-salt", "F0000002");
        assert_eq!(a.len(), 32);
        assert!(
            a.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        assert_ne!(a, b);
        assert_eq!(a, uploader_id("test-salt", "F0000001"));
    }

    #[test]
    fn gzip_only_above_16k() {
        // Ajustamos el campo relleno para obtener exactamente 16384 y 16385 bytes serializados.
        let base = json!({ "pad": "" });
        let base_len = serde_json::to_vec(&base).unwrap().len();

        let exact = json!({ "pad": "a".repeat(GZIP_THRESHOLD - base_len) });
        assert_eq!(serde_json::to_vec(&exact).unwrap().len(), GZIP_THRESHOLD);
        let enc_exact = encode_body(&exact).unwrap();
        assert!(!enc_exact.gzip);
        assert_eq!(enc_exact.bytes.len(), GZIP_THRESHOLD);

        let over = json!({ "pad": "a".repeat(GZIP_THRESHOLD + 1 - base_len) });
        let over_raw = serde_json::to_vec(&over).unwrap();
        assert_eq!(over_raw.len(), GZIP_THRESHOLD + 1);
        let enc_over = encode_body(&over).unwrap();
        assert!(enc_over.gzip);
        let mut dec = Vec::new();
        GzDecoder::new(enc_over.bytes.as_slice())
            .read_to_end(&mut dec)
            .unwrap();
        assert_eq!(dec, over_raw);
    }

    #[test]
    fn classify_table() {
        let mk = |status: u16, body: &str| {
            Ok(HttpReplyEx {
                status,
                body: body.to_owned(),
                retry_after_s: None,
                rate_limit_remaining: None,
                rate_limit_reset_s: None,
            })
        };
        assert_eq!(
            classify(&mk(200, &fixture_text("responses/ok.txt"))),
            EddnVerdict::Sent
        );
        assert!(matches!(
            classify(&mk(400, &fixture_text("responses/fail_400.txt"))),
            EddnVerdict::Rejected {
                class: ErrorClass::InvalidEvent,
                ..
            }
        ));
        assert!(matches!(
            classify(&mk(413, "Payload Too Large")),
            EddnVerdict::Rejected {
                class: ErrorClass::TooLarge,
                ..
            }
        ));
        assert!(matches!(
            classify(&mk(426, "FAIL: Outdated Schema")),
            EddnVerdict::SchemaOutdated { .. }
        ));
        for status in [408u16, 503, 500] {
            assert!(matches!(
                classify(&mk(status, "err")),
                EddnVerdict::Retry { .. }
            ));
        }
        assert!(matches!(
            classify(&Err(TransportError::Timeout)),
            EddnVerdict::Retry {
                class: ErrorClass::Timeout,
                ..
            }
        ));
    }

    #[test]
    fn html_503_is_retry_min_60s() {
        let html = fixture_text("responses/html_503.html");
        let reply = Ok(HttpReplyEx {
            status: 503,
            body: html,
            retry_after_s: None,
            rate_limit_remaining: None,
            rate_limit_reset_s: None,
        });
        assert!(matches!(
            classify(&reply),
            EddnVerdict::Retry {
                class: ErrorClass::ServerError,
                ..
            }
        ));
    }

    #[test]
    fn server_hint_trimmed_to_120_chars_without_markup() {
        let raw = format!("FAIL: <b>{}</b>", "x".repeat(300));
        let reply = Ok(HttpReplyEx {
            status: 400,
            body: raw,
            retry_after_s: None,
            rate_limit_remaining: None,
            rate_limit_reset_s: None,
        });
        match classify(&reply) {
            EddnVerdict::Rejected { hint: Some(h), .. } => {
                assert!(h.chars().count() <= 120, "hint len: {}", h.chars().count());
                assert!(!h.contains('<') && !h.contains('>'));
            }
            other => panic!("esperaba Rejected con hint, recibí {other:?}"),
        }
    }

    #[tokio::test]
    async fn upload_is_http1_and_sets_content_encoding() {
        let tr = ScriptedTransport::new(vec![
            Ok(HttpReplyEx {
                status: 200,
                body: "OK".into(),
                retry_after_s: None,
                rate_limit_remaining: None,
                rate_limit_reset_s: None,
            }),
            Ok(HttpReplyEx {
                status: 200,
                body: "OK".into(),
                retry_after_s: None,
                rate_limit_remaining: None,
                rate_limit_reset_s: None,
            }),
        ]);

        let plain = EncodedBody {
            bytes: b"{}".to_vec(),
            gzip: false,
        };
        upload(tr.as_ref(), &plain).await.unwrap();

        let gzipped = encode_body(&json!({ "pad": "a".repeat(GZIP_THRESHOLD + 512) })).unwrap();
        assert!(gzipped.gzip);
        upload(tr.as_ref(), &gzipped).await.unwrap();

        let posts = tr.posts();
        assert_eq!(posts.len(), 2);
        assert_eq!(posts[0].url, EDDN_UPLOAD_URL);
        assert_eq!(posts[0].content_type, "application/json");
        assert!(posts[0].http1_only);
        assert!(posts[0].headers.is_empty());

        assert!(posts[1].http1_only);
        assert_eq!(
            posts[1].headers,
            vec![("Content-Encoding".to_string(), "gzip".to_string())]
        );
    }
}
