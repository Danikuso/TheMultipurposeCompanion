//! Emisor de esquemas oficiales a gateway EDDN con seudonimización y protección de galaxia Legacy.

use crate::transport::Transport;
use chrono::Utc;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tmc_model::accounts::{EddnEnvelope, EddnHeader};
use tmc_model::state::Galaxy;

pub const EDDN_GATEWAY_URL: &str = "https://eddn.edcd.io:4430/upload/";

pub const SCHEMA_COMMODITY_3: &str = "https://eddn.edcd.io/schemas/commodity/3";
pub const SCHEMA_JOURNAL_1: &str = "https://eddn.edcd.io/schemas/journal/1";
pub const SCHEMA_OUTFITTING_2: &str = "https://eddn.edcd.io/schemas/outfitting/2";
pub const SCHEMA_SHIPYARD_2: &str = "https://eddn.edcd.io/schemas/shipyard/2";
pub const SCHEMA_NAVROUTE_1: &str = "https://eddn.edcd.io/schemas/navroute/1";

/// Error al emitir un mensaje al gateway de EDDN.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EddnSendError {
    #[error("envío bloqueado: galaxia Legacy no permitida")]
    LegacyBlocked,
    #[error("envío bloqueado: servicios en línea desactivados")]
    OfflineBlocked,
    #[error("envío bloqueado: emisor EDDN desactivado en ajustes")]
    DisabledBlocked,
    #[error("error de transporte HTTP: {0}")]
    Transport(String),
    #[error("gateway EDDN rechazó el mensaje con HTTP {0}: {1}")]
    GatewayRejected(u16, String),
}

/// Genera un `uploaderID` seudonimizado irreversible pero determinista para el comandante y la sal local.
pub fn generate_uploader_id(cmdr_name: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(cmdr_name.trim().as_bytes());
    hasher.update(b":");
    hasher.update(salt.trim().as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Construye el sobre envelope oficial de EDDN con la versión actual de TMC y marca de tiempo UTC.
pub fn build_eddn_envelope<T: Serialize>(
    schema_ref: &str,
    uploader_id: &str,
    message: T,
) -> EddnEnvelope<T> {
    EddnEnvelope {
        schema_ref: schema_ref.to_string(),
        header: EddnHeader {
            uploader_id: uploader_id.to_string(),
            software_name: "TMC".to_string(),
            software_version: env!("CARGO_PKG_VERSION").to_string(),
            gateway_timestamp: Utc::now().to_rfc3339(),
        },
        message,
    }
}

/// Emisor de datos al gateway de EDDN.
pub struct EddnSender {
    transport: Arc<dyn Transport>,
}

impl EddnSender {
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self { transport }
    }

    /// Envía un mensaje a EDDN respetando estrictamente las políticas de galaxia Live y opt-out.
    pub async fn send<T: Serialize>(
        &self,
        schema_ref: &str,
        uploader_id: &str,
        message: &T,
        galaxy: Galaxy,
        online: bool,
        enabled: bool,
    ) -> Result<(), EddnSendError> {
        if galaxy != Galaxy::Live {
            return Err(EddnSendError::LegacyBlocked);
        }
        if !online {
            return Err(EddnSendError::OfflineBlocked);
        }
        if !enabled {
            return Err(EddnSendError::DisabledBlocked);
        }

        let envelope = build_eddn_envelope(schema_ref, uploader_id, message);
        let body = serde_json::to_string(&envelope)
            .map_err(|e| EddnSendError::Transport(e.to_string()))?;

        let reply = self
            .transport
            .post_json(EDDN_GATEWAY_URL, &body)
            .await
            .map_err(|e| EddnSendError::Transport(format!("{e:?}")))?;

        if (200..=299).contains(&reply.status) {
            Ok(())
        } else {
            Err(EddnSendError::GatewayRejected(reply.status, reply.body))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{BoxFut, HttpReply, TransportError};
    use std::sync::Mutex;

    struct MockTransport {
        replies: Mutex<Vec<Result<HttpReply, TransportError>>>,
        posted: Mutex<Vec<(String, String)>>,
    }

    impl MockTransport {
        fn new(replies: Vec<Result<HttpReply, TransportError>>) -> Self {
            Self {
                replies: Mutex::new(replies),
                posted: Mutex::new(Vec::new()),
            }
        }
    }

    impl Transport for MockTransport {
        fn get(&self, _url: &str) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            Box::pin(async { Err(TransportError::Connect("get not implemented".into())) })
        }

        fn post_form(
            &self,
            _url: &str,
            _form: &[(String, String)],
        ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            Box::pin(async { Err(TransportError::Connect("post_form not implemented".into())) })
        }

        fn post_json(
            &self,
            url: &str,
            body: &str,
        ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            self.posted
                .lock()
                .unwrap()
                .push((url.to_string(), body.to_string()));
            let mut g = self.replies.lock().unwrap();
            let r = if g.is_empty() {
                Ok(HttpReply {
                    status: 200,
                    body: "OK".to_string(),
                })
            } else {
                g.remove(0)
            };
            Box::pin(async move { r })
        }
    }

    #[test]
    fn uploader_id_is_deterministic_and_pseudonymous() {
        let id1 = generate_uploader_id("CMDR Danikuso", "salt-1234");
        let id2 = generate_uploader_id("CMDR Danikuso", "salt-1234");
        let id_diff = generate_uploader_id("CMDR Other", "salt-1234");
        let id_diff_salt = generate_uploader_id("CMDR Danikuso", "salt-5678");

        assert_eq!(id1, id2);
        assert_ne!(id1, id_diff);
        assert_ne!(id1, id_diff_salt);
        assert_eq!(id1.len(), 64);
        assert!(!id1.contains("Danikuso"));
    }

    #[tokio::test]
    async fn rejects_legacy_galaxy_strictly() {
        let transport = Arc::new(MockTransport::new(vec![]));
        let sender = EddnSender::new(transport.clone());

        let res = sender
            .send(
                SCHEMA_JOURNAL_1,
                "uploader_123",
                &serde_json::json!({"event": "FSDJump"}),
                Galaxy::Legacy,
                true,
                true,
            )
            .await;

        assert_eq!(res, Err(EddnSendError::LegacyBlocked));
        assert!(transport.posted.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_offline_or_disabled() {
        let transport = Arc::new(MockTransport::new(vec![]));
        let sender = EddnSender::new(transport.clone());

        let res_off = sender
            .send(
                SCHEMA_JOURNAL_1,
                "uploader_123",
                &serde_json::json!({"event": "FSDJump"}),
                Galaxy::Live,
                false,
                true,
            )
            .await;
        assert_eq!(res_off, Err(EddnSendError::OfflineBlocked));

        let res_dis = sender
            .send(
                SCHEMA_JOURNAL_1,
                "uploader_123",
                &serde_json::json!({"event": "FSDJump"}),
                Galaxy::Live,
                true,
                false,
            )
            .await;
        assert_eq!(res_dis, Err(EddnSendError::DisabledBlocked));
        assert!(transport.posted.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn successful_send_posts_valid_envelope() {
        let transport = Arc::new(MockTransport::new(vec![Ok(HttpReply {
            status: 200,
            body: "OK".to_string(),
        })]));
        let sender = EddnSender::new(transport.clone());

        let res = sender
            .send(
                SCHEMA_COMMODITY_3,
                "uploader_hash",
                &serde_json::json!({
                    "systemName": "Sol",
                    "stationName": "Li Qing Jao"
                }),
                Galaxy::Live,
                true,
                true,
            )
            .await;

        assert_eq!(res, Ok(()));
        let posted = transport.posted.lock().unwrap();
        assert_eq!(posted.len(), 1);
        assert_eq!(posted[0].0, EDDN_GATEWAY_URL);
        assert!(
            posted[0]
                .1
                .contains("\"$schemaRef\":\"https://eddn.edcd.io/schemas/commodity/3\"")
        );
        assert!(posted[0].1.contains("\"uploaderID\":\"uploader_hash\""));
        assert!(posted[0].1.contains("\"softwareName\":\"TMC\""));
    }

    #[tokio::test]
    async fn handles_gateway_bad_request() {
        let transport = Arc::new(MockTransport::new(vec![Ok(HttpReply {
            status: 400,
            body: "FAIL: Schema validation failed".to_string(),
        })]));
        let sender = EddnSender::new(transport);

        let res = sender
            .send(
                SCHEMA_JOURNAL_1,
                "uploader_hash",
                &serde_json::json!({"invalid": true}),
                Galaxy::Live,
                true,
                true,
            )
            .await;

        assert_eq!(
            res,
            Err(EddnSendError::GatewayRejected(
                400,
                "FAIL: Schema validation failed".to_string()
            ))
        );
    }
}
