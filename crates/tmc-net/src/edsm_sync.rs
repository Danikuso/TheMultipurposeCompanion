//! Cliente de sincronización con la API de registros de EDSM (`https://www.edsm.net/api-logs-v1/`).

use crate::transport::{HttpReply, Transport};
use serde_json::Value;
use std::sync::Arc;
use tmc_model::state::Galaxy;

pub const EDSM_LOGS_BASE: &str = "https://www.edsm.net/api-logs-v1";

/// Error de sincronización con EDSM.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EdsmSyncError {
    #[error("sincronización bloqueada: galaxia Legacy no permitida")]
    LegacyBlocked,
    #[error("sincronización bloqueada: servicios en línea desactivados")]
    OfflineBlocked,
    #[error("sincronización bloqueada: servicio EDSM desactivado")]
    DisabledBlocked,
    #[error("faltan credenciales de EDSM (nombre de CMDR o API key)")]
    MissingCredentials,
    #[error("credenciales inválidas en EDSM (código {0}): {1}")]
    InvalidCredentials(u16, String),
    #[error("error de respuesta EDSM (código {0}): {1}")]
    ApiError(u16, String),
    #[error("error de transporte HTTP: {0}")]
    Transport(String),
}

/// Credenciales de autenticación para EDSM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdsmAuth<'a> {
    pub cmdr_name: &'a str,
    pub api_key: &'a str,
}

/// Parámetros para registrar un salto en EDSM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdsmJumpLog<'a> {
    pub system_name: &'a str,
    pub date_utc: &'a str,
    pub first_discovery: bool,
}

/// Rangos del comandante para sincronizar con EDSM.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EdsmCommanderRanks {
    pub combat: Option<u8>,
    pub trade: Option<u8>,
    pub explore: Option<u8>,
    pub cqc: Option<u8>,
    pub soldier: Option<u8>,
    pub exobiologist: Option<u8>,
}

/// Saldo del comandante para sincronizar con EDSM.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EdsmCommanderCredits {
    pub balance: u64,
    pub loan: Option<u64>,
    pub ship_value: Option<u64>,
}

/// Cliente de sincronización con EDSM.
pub struct EdsmSyncClient {
    transport: Arc<dyn Transport>,
}

impl EdsmSyncClient {
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self { transport }
    }

    /// Envía un registro de salto a EDSM (`set-logs`).
    pub async fn send_jump_log(
        &self,
        auth: &EdsmAuth<'_>,
        jump: &EdsmJumpLog<'_>,
        galaxy: Galaxy,
        online: bool,
        enabled: bool,
    ) -> Result<(), EdsmSyncError> {
        self.check_guards(auth, galaxy, online, enabled)?;

        let mut form = vec![
            (
                "commanderName".to_string(),
                auth.cmdr_name.trim().to_string(),
            ),
            ("apiKey".to_string(), auth.api_key.trim().to_string()),
            ("system".to_string(), jump.system_name.trim().to_string()),
            ("date".to_string(), jump.date_utc.trim().to_string()),
        ];
        if jump.first_discovery {
            form.push(("firstDiscover".to_string(), "1".to_string()));
        }

        let url = format!("{EDSM_LOGS_BASE}/set-logs");
        self.post_and_parse(&url, &form).await
    }

    /// Envía los rangos actualizados del comandante (`set-commander-ranks`).
    pub async fn send_ranks(
        &self,
        auth: &EdsmAuth<'_>,
        ranks: &EdsmCommanderRanks,
        galaxy: Galaxy,
        online: bool,
        enabled: bool,
    ) -> Result<(), EdsmSyncError> {
        self.check_guards(auth, galaxy, online, enabled)?;

        let mut form = vec![
            (
                "commanderName".to_string(),
                auth.cmdr_name.trim().to_string(),
            ),
            ("apiKey".to_string(), auth.api_key.trim().to_string()),
        ];
        if let Some(r) = ranks.combat {
            form.push(("combat".to_string(), r.to_string()));
        }
        if let Some(r) = ranks.trade {
            form.push(("trade".to_string(), r.to_string()));
        }
        if let Some(r) = ranks.explore {
            form.push(("explore".to_string(), r.to_string()));
        }
        if let Some(r) = ranks.cqc {
            form.push(("cqc".to_string(), r.to_string()));
        }
        if let Some(r) = ranks.soldier {
            form.push(("soldier".to_string(), r.to_string()));
        }
        if let Some(r) = ranks.exobiologist {
            form.push(("exobiologist".to_string(), r.to_string()));
        }

        let url = format!("{EDSM_LOGS_BASE}/set-commander-ranks");
        self.post_and_parse(&url, &form).await
    }

    /// Envía el estado financiero del comandante (`set-commander-credits`).
    pub async fn send_credits(
        &self,
        auth: &EdsmAuth<'_>,
        credits: &EdsmCommanderCredits,
        galaxy: Galaxy,
        online: bool,
        enabled: bool,
    ) -> Result<(), EdsmSyncError> {
        self.check_guards(auth, galaxy, online, enabled)?;

        let mut form = vec![
            (
                "commanderName".to_string(),
                auth.cmdr_name.trim().to_string(),
            ),
            ("apiKey".to_string(), auth.api_key.trim().to_string()),
            ("credits".to_string(), credits.balance.to_string()),
        ];
        if let Some(loan) = credits.loan {
            form.push(("loan".to_string(), loan.to_string()));
        }
        if let Some(val) = credits.ship_value {
            form.push(("shipValue".to_string(), val.to_string()));
        }

        let url = format!("{EDSM_LOGS_BASE}/set-commander-credits");
        self.post_and_parse(&url, &form).await
    }

    fn check_guards(
        &self,
        auth: &EdsmAuth<'_>,
        galaxy: Galaxy,
        online: bool,
        enabled: bool,
    ) -> Result<(), EdsmSyncError> {
        if galaxy != Galaxy::Live {
            return Err(EdsmSyncError::LegacyBlocked);
        }
        if !online {
            return Err(EdsmSyncError::OfflineBlocked);
        }
        if !enabled {
            return Err(EdsmSyncError::DisabledBlocked);
        }
        if auth.cmdr_name.trim().is_empty() || auth.api_key.trim().is_empty() {
            return Err(EdsmSyncError::MissingCredentials);
        }
        Ok(())
    }

    async fn post_and_parse(
        &self,
        url: &str,
        form: &[(String, String)],
    ) -> Result<(), EdsmSyncError> {
        let reply = self
            .transport
            .post_form(url, form)
            .await
            .map_err(|e| EdsmSyncError::Transport(format!("{e:?}")))?;

        parse_edsm_reply(&reply)
    }
}

/// Analiza la respuesta JSON de EDSM logs.
fn parse_edsm_reply(reply: &HttpReply) -> Result<(), EdsmSyncError> {
    if !(200..=299).contains(&reply.status) {
        return Err(EdsmSyncError::Transport(format!(
            "HTTP {} status",
            reply.status
        )));
    }

    let val: Value = serde_json::from_str(&reply.body)
        .map_err(|e| EdsmSyncError::Transport(format!("malformed json: {e}")))?;

    let msgnum = val.get("msgnum").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
    let msg = val
        .get("msg")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown response")
        .to_string();

    match msgnum {
        100 => Ok(()), // 100 = OK
        201 | 202 => Err(EdsmSyncError::InvalidCredentials(msgnum, msg)),
        other => Err(EdsmSyncError::ApiError(other, msg)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{BoxFut, TransportError};
    use std::sync::Mutex;

    type PostedEntry = (String, Vec<(String, String)>);

    struct MockTransport {
        replies: Mutex<Vec<Result<HttpReply, TransportError>>>,
        posted_forms: Mutex<Vec<PostedEntry>>,
    }

    impl MockTransport {
        fn new(replies: Vec<Result<HttpReply, TransportError>>) -> Self {
            Self {
                replies: Mutex::new(replies),
                posted_forms: Mutex::new(Vec::new()),
            }
        }
    }

    impl Transport for MockTransport {
        fn get(&self, _url: &str) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            Box::pin(async { Err(TransportError::Connect("no get".into())) })
        }

        fn post_form(
            &self,
            url: &str,
            form: &[(String, String)],
        ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            self.posted_forms
                .lock()
                .unwrap()
                .push((url.to_string(), form.to_vec()));
            let mut g = self.replies.lock().unwrap();
            let r = if g.is_empty() {
                Ok(HttpReply {
                    status: 200,
                    body: r#"{"msgnum":100,"msg":"OK"}"#.to_string(),
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
        let client = EdsmSyncClient::new(transport.clone());
        let auth = EdsmAuth {
            cmdr_name: "Danikuso",
            api_key: "key123",
        };
        let jump = EdsmJumpLog {
            system_name: "Sol",
            date_utc: "2026-10-23 12:00:00",
            first_discovery: false,
        };

        let res_leg = client
            .send_jump_log(&auth, &jump, Galaxy::Legacy, true, true)
            .await;
        assert_eq!(res_leg, Err(EdsmSyncError::LegacyBlocked));

        let res_off = client
            .send_jump_log(&auth, &jump, Galaxy::Live, false, true)
            .await;
        assert_eq!(res_off, Err(EdsmSyncError::OfflineBlocked));
        assert!(transport.posted_forms.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_missing_credentials() {
        let transport = Arc::new(MockTransport::new(vec![]));
        let client = EdsmSyncClient::new(transport);
        let auth = EdsmAuth {
            cmdr_name: "",
            api_key: "key123",
        };
        let jump = EdsmJumpLog {
            system_name: "Sol",
            date_utc: "2026-10-23 12:00:00",
            first_discovery: false,
        };

        let res = client
            .send_jump_log(&auth, &jump, Galaxy::Live, true, true)
            .await;
        assert_eq!(res, Err(EdsmSyncError::MissingCredentials));
    }

    #[tokio::test]
    async fn successful_jump_log_sends_form() {
        let transport = Arc::new(MockTransport::new(vec![Ok(HttpReply {
            status: 200,
            body: r#"{"msgnum":100,"msg":"OK"}"#.to_string(),
        })]));
        let client = EdsmSyncClient::new(transport.clone());
        let auth = EdsmAuth {
            cmdr_name: "Danikuso",
            api_key: "key123",
        };
        let jump = EdsmJumpLog {
            system_name: "Shinrarta Dezhra",
            date_utc: "2026-10-23 12:00:00",
            first_discovery: true,
        };

        let res = client
            .send_jump_log(&auth, &jump, Galaxy::Live, true, true)
            .await;

        assert_eq!(res, Ok(()));
        let forms = transport.posted_forms.lock().unwrap();
        assert_eq!(forms.len(), 1);
        assert_eq!(forms[0].0, "https://www.edsm.net/api-logs-v1/set-logs");
        assert!(
            forms[0]
                .1
                .iter()
                .any(|(k, v)| k == "system" && v == "Shinrarta Dezhra")
        );
        assert!(
            forms[0]
                .1
                .iter()
                .any(|(k, v)| k == "firstDiscover" && v == "1")
        );
    }

    #[tokio::test]
    async fn handles_invalid_api_key_response() {
        let transport = Arc::new(MockTransport::new(vec![Ok(HttpReply {
            status: 200,
            body: r#"{"msgnum":202,"msg":"Invalid API Key"}"#.to_string(),
        })]));
        let client = EdsmSyncClient::new(transport);
        let auth = EdsmAuth {
            cmdr_name: "Danikuso",
            api_key: "badkey",
        };

        let res = client
            .send_ranks(
                &auth,
                &EdsmCommanderRanks {
                    combat: Some(5),
                    trade: Some(8),
                    explore: Some(9),
                    ..Default::default()
                },
                Galaxy::Live,
                true,
                true,
            )
            .await;

        assert_eq!(
            res,
            Err(EdsmSyncError::InvalidCredentials(
                202,
                "Invalid API Key".to_string()
            ))
        );
    }
}
