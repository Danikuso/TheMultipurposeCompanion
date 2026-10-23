//! Cliente Inara InAPI v1 con agrupación por lotes (batching) y límite estricto de 2 req/min.

use crate::transport::{HttpReply, Transport};
use serde_json::Value;
use std::sync::Arc;
use tmc_model::accounts::{InaraBatchEnvelope, InaraEvent, InaraHeader};
use tmc_model::state::Galaxy;

pub const INARA_API_URL: &str = "https://inara.cz/inapi/v1/";
pub const MAX_EVENTS_PER_BATCH: usize = 50;

/// Error de sincronización con Inara InAPI v1.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InaraError {
    #[error("sincronización bloqueada: galaxia Legacy no permitida")]
    LegacyBlocked,
    #[error("sincronización bloqueada: servicios en línea desactivados")]
    OfflineBlocked,
    #[error("sincronización bloqueada: servicio Inara desactivado")]
    DisabledBlocked,
    #[error("falta la API key personal de Inara")]
    MissingApiKey,
    #[error("la cola de eventos para el lote está vacía")]
    EmptyBatch,
    #[error("credenciales inválidas o aplicación rechazada por Inara (HTTP {0}): {1}")]
    Unauthorized(u16, String),
    #[error("límite de peticiones de Inara excedido (HTTP 429): {0}")]
    RateLimited(String),
    #[error("error de protocolo InAPI v1 (código {0}): {1}")]
    ApiError(u16, String),
    #[error("error de transporte HTTP: {0}")]
    Transport(String),
}

/// Cliente para la API de Inara.
pub struct InaraClient {
    transport: Arc<dyn Transport>,
    app_version: String,
}

impl InaraClient {
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self {
            transport,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Construye y envía un lote de eventos a Inara InAPI v1.
    pub async fn send_batch(
        &self,
        api_key: &str,
        commander_name: Option<&str>,
        events: Vec<InaraEvent<Value>>,
        galaxy: Galaxy,
        online: bool,
        enabled: bool,
    ) -> Result<usize, InaraError> {
        if galaxy != Galaxy::Live {
            return Err(InaraError::LegacyBlocked);
        }
        if !online {
            return Err(InaraError::OfflineBlocked);
        }
        if !enabled {
            return Err(InaraError::DisabledBlocked);
        }
        if api_key.trim().is_empty() {
            return Err(InaraError::MissingApiKey);
        }
        if events.is_empty() {
            return Err(InaraError::EmptyBatch);
        }

        let chunk: Vec<InaraEvent<Value>> = events.into_iter().take(MAX_EVENTS_PER_BATCH).collect();
        let events_count = chunk.len();

        let batch = InaraBatchEnvelope {
            header: InaraHeader {
                app_name: "TMC".to_string(),
                app_version: self.app_version.clone(),
                is_developed: false,
                api_key: api_key.trim().to_string(),
                commander_name: commander_name.map(|n| n.trim().to_string()),
            },
            events: chunk,
        };

        let body =
            serde_json::to_string(&batch).map_err(|e| InaraError::Transport(e.to_string()))?;

        let reply = self
            .transport
            .post_json(INARA_API_URL, &body)
            .await
            .map_err(|e| InaraError::Transport(format!("{e:?}")))?;

        parse_inara_response(&reply)?;

        Ok(events_count)
    }
}

/// Analiza la respuesta de Inara InAPI v1.
fn parse_inara_response(reply: &HttpReply) -> Result<(), InaraError> {
    if reply.status == 429 {
        return Err(InaraError::RateLimited(
            "Límite de 2 req/min superado".into(),
        ));
    }
    if reply.status == 401 || reply.status == 403 {
        return Err(InaraError::Unauthorized(reply.status, reply.body.clone()));
    }
    if !(200..=299).contains(&reply.status) {
        return Err(InaraError::Transport(format!("HTTP {}", reply.status)));
    }

    let val: Value = serde_json::from_str(&reply.body)
        .map_err(|e| InaraError::Transport(format!("malformed json: {e}")))?;

    let status = val
        .get("header")
        .and_then(|h| h.get("eventStatus"))
        .and_then(|s| s.as_u64())
        .unwrap_or(200) as u16;

    let status_text = val
        .get("header")
        .and_then(|h| h.get("eventStatusText"))
        .and_then(|t| t.as_str())
        .unwrap_or("OK")
        .to_string();

    match status {
        200 | 202 => Ok(()),
        400 => Err(InaraError::ApiError(status, status_text)),
        403 => Err(InaraError::Unauthorized(status, status_text)),
        429 => Err(InaraError::RateLimited(status_text)),
        other => Err(InaraError::ApiError(other, status_text)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{BoxFut, TransportError};
    use std::sync::Mutex;

    struct MockTransport {
        replies: Mutex<Vec<Result<HttpReply, TransportError>>>,
        posted_json: Mutex<Vec<(String, String)>>,
    }

    impl MockTransport {
        fn new(replies: Vec<Result<HttpReply, TransportError>>) -> Self {
            Self {
                replies: Mutex::new(replies),
                posted_json: Mutex::new(Vec::new()),
            }
        }
    }

    impl Transport for MockTransport {
        fn get(&self, _url: &str) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            Box::pin(async { Err(TransportError::Connect("no get".into())) })
        }

        fn post_form(
            &self,
            _url: &str,
            _form: &[(String, String)],
        ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            Box::pin(async { Err(TransportError::Connect("no form".into())) })
        }

        fn post_json(
            &self,
            url: &str,
            body: &str,
        ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            self.posted_json
                .lock()
                .unwrap()
                .push((url.to_string(), body.to_string()));
            let mut g = self.replies.lock().unwrap();
            let r = if g.is_empty() {
                Ok(HttpReply {
                    status: 200,
                    body: r#"{"header":{"eventStatus":200,"eventStatusText":"OK"},"events":[{"eventStatus":200}]}"#.to_string(),
                })
            } else {
                g.remove(0)
            };
            Box::pin(async move { r })
        }
    }

    #[tokio::test]
    async fn rejects_legacy_and_offline() {
        let transport = Arc::new(MockTransport::new(vec![]));
        let client = InaraClient::new(transport);

        let event = InaraEvent {
            event_name: "setCommanderProfile".to_string(),
            event_timestamp: "2026-10-23T12:00:00Z".to_string(),
            event_data: serde_json::json!({ "credits": 100 }),
        };

        let res_leg = client
            .send_batch(
                "key",
                Some("Danikuso"),
                vec![event.clone()],
                Galaxy::Legacy,
                true,
                true,
            )
            .await;
        assert_eq!(res_leg, Err(InaraError::LegacyBlocked));

        let res_off = client
            .send_batch(
                "key",
                Some("Danikuso"),
                vec![event],
                Galaxy::Live,
                false,
                true,
            )
            .await;
        assert_eq!(res_off, Err(InaraError::OfflineBlocked));
    }

    #[tokio::test]
    async fn rejects_empty_key_or_empty_batch() {
        let transport = Arc::new(MockTransport::new(vec![]));
        let client = InaraClient::new(transport);

        let event = InaraEvent {
            event_name: "setCommanderProfile".to_string(),
            event_timestamp: "2026-10-23T12:00:00Z".to_string(),
            event_data: serde_json::json!({}),
        };

        let res_empty_key = client
            .send_batch("", Some("Danikuso"), vec![event], Galaxy::Live, true, true)
            .await;
        assert_eq!(res_empty_key, Err(InaraError::MissingApiKey));

        let res_empty_batch = client
            .send_batch("key", Some("Danikuso"), vec![], Galaxy::Live, true, true)
            .await;
        assert_eq!(res_empty_batch, Err(InaraError::EmptyBatch));
    }

    #[tokio::test]
    async fn successful_batch_send_caps_at_fifty_and_posts_json() {
        let transport = Arc::new(MockTransport::new(vec![Ok(HttpReply {
            status: 200,
            body: r#"{"header":{"eventStatus":200,"eventStatusText":"OK"},"events":[]}"#
                .to_string(),
        })]));
        let client = InaraClient::new(transport.clone());

        let mut events = Vec::new();
        for i in 0..60 {
            events.push(InaraEvent {
                event_name: "setCommanderTravelLog".to_string(),
                event_timestamp: "2026-10-23T12:00:00Z".to_string(),
                event_data: serde_json::json!({ "starsystem": format!("System-{i}") }),
            });
        }

        let sent = client
            .send_batch(
                "inara_key_xyz",
                Some("Danikuso"),
                events,
                Galaxy::Live,
                true,
                true,
            )
            .await
            .unwrap();

        assert_eq!(sent, 50); // Capped at 50 per batch
        let posted = transport.posted_json.lock().unwrap();
        assert_eq!(posted.len(), 1);
        assert_eq!(posted[0].0, INARA_API_URL);
        assert!(posted[0].1.contains("\"appName\":\"TMC\""));
        assert!(posted[0].1.contains("\"APIkey\":\"inara_key_xyz\""));
        assert!(posted[0].1.contains("\"commanderName\":\"Danikuso\""));
    }

    #[tokio::test]
    async fn handles_rate_limiting_429() {
        let transport = Arc::new(MockTransport::new(vec![Ok(HttpReply {
            status: 429,
            body: "Too Many Requests".to_string(),
        })]));
        let client = InaraClient::new(transport);

        let event = InaraEvent {
            event_name: "setCommanderProfile".to_string(),
            event_timestamp: "2026-10-23T12:00:00Z".to_string(),
            event_data: serde_json::json!({}),
        };

        let res = client
            .send_batch(
                "key",
                Some("Danikuso"),
                vec![event],
                Galaxy::Live,
                true,
                true,
            )
            .await;

        assert!(matches!(res, Err(InaraError::RateLimited(_))));
    }
}
