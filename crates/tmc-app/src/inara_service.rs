//! Servicio de integración con Inara InAPI v1 (legado, pendiente de retirada en E3-6).

use serde_json::{Value, json};
use std::sync::Arc;
use tmc_model::accounts::{InaraBatchEnvelope, InaraEvent, InaraHeader};
use tmc_net::Transport;
use tmc_net::inara::{INARA_API_URL, INARA_APP_NAME, MAX_EVENTS_PER_BATCH};
use tmc_store::{AccountCredentialRecord, Store};

pub const INARA_SERVICE_KEY: &str = "inara";

/// Servicio de integración y sincronización con Inara InAPI v1.
pub struct InaraService {
    transport: Arc<dyn Transport>,
    store: Store,
    app_key: Option<String>,
}

impl InaraService {
    pub fn new(store: Store, transport: Arc<dyn Transport>) -> Self {
        let app_key = crate::env::env_var_or_file("INARA_DEV_API_KEY");
        Self::with_app_key(store, transport, app_key)
    }

    pub fn with_app_key(
        store: Store,
        transport: Arc<dyn Transport>,
        app_key: Option<String>,
    ) -> Self {
        Self {
            transport,
            store,
            app_key: app_key
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty()),
        }
    }

    async fn post_events_legacy(
        &self,
        api_key: &str,
        commander_name: Option<&str>,
        events: Vec<InaraEvent<Value>>,
    ) -> Result<Value, String> {
        if events.is_empty() {
            return Err("la cola de eventos para el lote está vacía".into());
        }
        let batch = InaraBatchEnvelope {
            header: InaraHeader {
                app_name: INARA_APP_NAME.to_string(),
                app_version: env!("CARGO_PKG_VERSION").to_string(),
                is_being_developed: cfg!(debug_assertions),
                api_key: api_key.trim().to_string(),
                commander_name: commander_name.map(|n| n.trim().to_string()),
                commander_frontier_id: None,
            },
            events: events.into_iter().take(MAX_EVENTS_PER_BATCH).collect(),
        };
        let body = serde_json::to_string(&batch).map_err(|e| e.to_string())?;
        let reply = self
            .transport
            .post_json(INARA_API_URL, &body)
            .await
            .map_err(|e| format!("{e:?}"))?;
        if !(200..300).contains(&reply.status) {
            return Err(format!("HTTP {}", reply.status));
        }
        serde_json::from_str(&reply.body).map_err(|e| e.to_string())
    }

    /// Obtiene la clave API personal guardada en SQLite si existe.
    pub fn get_saved_key(&self) -> Option<String> {
        self.store
            .get_account_credential(INARA_SERVICE_KEY)
            .ok()
            .flatten()
            .and_then(|r| r.api_key)
            .filter(|k| !k.trim().is_empty())
    }

    /// Clave API de desarrollador/aplicación oficial configurada en el entorno o `.env`.
    pub fn app_key(&self) -> Option<&str> {
        self.app_key.as_deref()
    }

    /// Clave efectiva a utilizar: prioridad a la clave personal en SQLite,
    /// con fallback a la clave de desarrollador/aplicación (`INARA_DEV_API_KEY`).
    pub fn get_effective_key(&self) -> Option<String> {
        self.get_saved_key().or_else(|| self.app_key.clone())
    }

    /// Comprueba si existe alguna clave disponible para consultar o sincronizar con Inara.
    pub fn has_any_key(&self) -> bool {
        self.get_effective_key().is_some()
    }

    /// Comprueba si hay una clave personal de comandante guardada en SQLite.
    pub fn has_saved_key(&self) -> bool {
        self.get_saved_key().is_some()
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
        let explicit_key = api_key.filter(|k| !k.trim().is_empty());
        let key = match explicit_key {
            Some(k) => k.trim().to_string(),
            None => self.get_effective_key().ok_or_else(|| {
                "Falta la clave API de Inara (personal o de aplicación)".to_string()
            })?,
        };

        let cmdr = commander_name.unwrap_or("").trim();

        let resp = if !cmdr.is_empty() {
            self.post_events_legacy(
                &key,
                Some(cmdr),
                vec![InaraEvent {
                    event_name: "getCommanderProfile".to_string(),
                    event_timestamp: chrono::Utc::now().to_rfc3339(),
                    event_custom_id: None,
                    event_data: json!({ "searchName": cmdr }),
                }],
            )
            .await?
        } else {
            self.post_events_legacy(
                &key,
                None,
                vec![InaraEvent {
                    event_name: "getCommunityGoalsRecent".to_string(),
                    event_timestamp: chrono::Utc::now().to_rfc3339(),
                    event_custom_id: None,
                    event_data: json!([]),
                }],
            )
            .await?
        };
        let profile_opt = resp
            .get("events")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(|e| e.get("eventData"))
            .cloned();

        // Si se proporcionó una clave explícita y la llamada tuvo éxito, persistirla
        if let Some(new_k) = explicit_key {
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
            .get_effective_key()
            .ok_or_else(|| "Falta la clave API de Inara (personal o de aplicación)".to_string())?;

        let count = events.len().min(MAX_EVENTS_PER_BATCH);
        self.post_events_legacy(&key, commander_name, events)
            .await?;
        Ok(count)
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

    #[tokio::test]
    async fn app_key_fallback_and_personal_key_override() {
        let store = Store::open_in_memory().unwrap();
        let reply_body = json!({
            "header": { "eventStatus": 200, "eventStatusText": "OK" },
            "events": [
                {
                    "eventStatus": 200,
                    "eventData": {
                        "userName": "Danikuso",
                        "inaraURL": "https://inara.cz/elite/cmdr/11111/"
                    }
                }
            ]
        });

        let transport = Arc::new(MockTransport::new(vec![Ok(HttpReply {
            status: 200,
            body: reply_body.to_string(),
        })]));
        let service =
            InaraService::with_app_key(store, transport, Some("app_dev_inara_key".to_string()));

        assert_eq!(service.get_saved_key(), None);
        assert_eq!(service.app_key(), Some("app_dev_inara_key"));
        assert_eq!(
            service.get_effective_key().as_deref(),
            Some("app_dev_inara_key")
        );
        assert!(service.has_any_key());
        assert!(!service.has_saved_key());

        // Probar conexión sin pasar clave explícita: usa app_key y NO guarda en SQLite
        let res = service
            .test_connection(None, Some("Danikuso"))
            .await
            .unwrap();
        assert_eq!(res["ok"], true);
        assert_eq!(service.get_saved_key(), None);

        // Guardar clave personal: debe tener prioridad sobre app_key
        service
            .save_key("personal_cmd_key", Some("Danikuso"))
            .unwrap();
        assert_eq!(
            service.get_effective_key().as_deref(),
            Some("personal_cmd_key")
        );
        assert!(service.has_saved_key());

        // Borrar clave personal: vuelve a la app_key
        service.clear_key().unwrap();
        assert_eq!(
            service.get_effective_key().as_deref(),
            Some("app_dev_inara_key")
        );
        assert!(!service.has_saved_key());
    }
}
