//! Servicio de integración con Inara InAPI v1.

use serde_json::{Value, json};
use std::sync::Arc;
use tmc_model::accounts::InaraEvent;
use tmc_model::state::Galaxy;
use tmc_net::Transport;
use tmc_net::inara::InaraClient;
use tmc_store::{AccountCredentialRecord, Store};

pub const INARA_SERVICE_KEY: &str = "inara";

/// Servicio de integración y sincronización con Inara InAPI v1.
pub struct InaraService {
    client: InaraClient,
    store: Store,
}

impl InaraService {
    pub fn new(store: Store, transport: Arc<dyn Transport>) -> Self {
        Self {
            client: InaraClient::new(transport),
            store,
        }
    }

    /// Obtiene la clave API guardada en SQLite si existe.
    pub fn get_saved_key(&self) -> Option<String> {
        self.store
            .get_account_credential(INARA_SERVICE_KEY)
            .ok()
            .flatten()
            .and_then(|r| r.api_key)
            .filter(|k| !k.trim().is_empty())
    }

    /// Guarda o actualiza la clave API de Inara en SQLite.
    pub fn save_key(&self, key: &str, cmdr_name: Option<&str>) -> Result<(), String> {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return Err("La clave API de Inara no puede estar vacía".into());
        }
        let record = AccountCredentialRecord {
            service: INARA_SERVICE_KEY.to_string(),
            api_key: Some(trimmed.to_string()),
            cmdr_name: cmdr_name.map(|n| n.trim().to_string()),
            token_json: None,
            salt: None,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        self.store
            .set_account_credential(&record)
            .map_err(|e| e.to_string())
    }

    /// Elimina las credenciales de Inara guardadas en SQLite.
    pub fn clear_key(&self) -> Result<(), String> {
        self.store
            .delete_account_credential(INARA_SERVICE_KEY)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Prueba la conexión contra InAPI v1 (consultando el perfil del comandante o CGs).
    pub async fn test_connection(
        &self,
        api_key: Option<&str>,
        commander_name: Option<&str>,
    ) -> Result<Value, String> {
        let key = match api_key.filter(|k| !k.trim().is_empty()) {
            Some(k) => k.trim().to_string(),
            None => self
                .get_saved_key()
                .ok_or_else(|| "Falta la clave API personal de Inara".to_string())?,
        };

        let cmdr = commander_name.unwrap_or("").trim();

        let profile_opt = if !cmdr.is_empty() {
            self.client
                .get_commander_profile(&key, cmdr, Galaxy::Live, true, true)
                .await
                .map_err(|e| format!("{e}"))?
        } else {
            self.client
                .get_community_goals(&key, Galaxy::Live, true, true)
                .await
                .map_err(|e| format!("{e}"))?
        };

        // Si se proporcionó una clave explícita y la llamada tuvo éxito, persistirla
        if let Some(new_k) = api_key {
            let _ = self.save_key(new_k, commander_name);
        }

        let inara_url = profile_opt
            .as_ref()
            .and_then(|p| p.get("inaraURL"))
            .and_then(|u| u.as_str())
            .map(ToString::to_string);

        let user_name = profile_opt
            .as_ref()
            .and_then(|p| p.get("userName"))
            .and_then(|u| u.as_str())
            .map(ToString::to_string);

        Ok(json!({
            "ok": true,
            "commander_name": cmdr,
            "user_name": user_name,
            "inara_url": inara_url,
            "data": profile_opt,
        }))
    }

    /// Sincroniza un lote de eventos con Inara InAPI v1.
    pub async fn sync_batch(
        &self,
        commander_name: Option<&str>,
        events: Vec<InaraEvent<Value>>,
    ) -> Result<usize, String> {
        let key = self
            .get_saved_key()
            .ok_or_else(|| "Falta la clave API personal de Inara".to_string())?;

        self.client
            .send_batch(&key, commander_name, events, Galaxy::Live, true, true)
            .await
            .map_err(|e| format!("{e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tmc_net::transport::{BoxFut, HttpReply, TransportError};

    struct MockTransport {
        replies: Mutex<Vec<Result<HttpReply, TransportError>>>,
    }

    impl MockTransport {
        fn new(replies: Vec<Result<HttpReply, TransportError>>) -> Self {
            Self {
                replies: Mutex::new(replies),
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
            _url: &str,
            _body: &str,
        ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
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
    async fn save_get_and_clear_key() {
        let store = Store::open_in_memory().unwrap();
        let transport = Arc::new(MockTransport::new(vec![]));
        let service = InaraService::new(store, transport);

        assert_eq!(service.get_saved_key(), None);

        service
            .save_key("test_inara_key_xyz", Some("Danikuso"))
            .unwrap();
        assert_eq!(
            service.get_saved_key().as_deref(),
            Some("test_inara_key_xyz")
        );

        service.clear_key().unwrap();
        assert_eq!(service.get_saved_key(), None);
    }

    #[tokio::test]
    async fn test_connection_fetches_profile_and_saves_key() {
        let store = Store::open_in_memory().unwrap();
        let reply_body = json!({
            "header": { "eventStatus": 200, "eventStatusText": "OK" },
            "events": [
                {
                    "eventStatus": 200,
                    "eventData": {
                        "userName": "Danikuso",
                        "inaraURL": "https://inara.cz/elite/cmdr/99999/"
                    }
                }
            ]
        });

        let transport = Arc::new(MockTransport::new(vec![Ok(HttpReply {
            status: 200,
            body: reply_body.to_string(),
        })]));
        let service = InaraService::new(store, transport);

        let res = service
            .test_connection(Some("new_key_123"), Some("Danikuso"))
            .await
            .unwrap();

        assert_eq!(res["ok"], true);
        assert_eq!(res["user_name"], "Danikuso");
        assert_eq!(res["inara_url"], "https://inara.cz/elite/cmdr/99999/");
        assert_eq!(service.get_saved_key().as_deref(), Some("new_key_123"));
    }
}
