//! Servicio de autenticación y sincronización con Frontier Companion API (CAPI).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tmc_model::accounts::RemoteShip;
use tmc_net::Transport;
pub use tmc_net::capi::{CapiClient, CapiError, CapiToken, build_auth_url, generate_pkce_pair};
use tmc_store::{AccountCredentialRecord, Store, StoreError};

const PENDING_STATE_TTL: Duration = Duration::from_secs(600);
const DEFAULT_CLIENT_ID: &str = "12fbec0d-b5c9-4911-af18-0e7a4ede266e";
const CAPI_SERVICE_KEY: &str = "frontier_capi";

/// Servicio de integración Frontier CAPI con gestión de ciclo de vida OAuth2 PKCE y flota remota.
pub struct CapiService {
    client: CapiClient,
    store: Store,
    client_id: String,
    pending: Arc<Mutex<HashMap<String, (String, Instant)>>>,
}

impl CapiService {
    pub fn new(store: Store, transport: Arc<dyn Transport>, client_id: Option<String>) -> Self {
        let client_id = client_id
            .or_else(|| std::env::var("TMC_CAPI_CLIENT_ID").ok())
            .unwrap_or_else(|| DEFAULT_CLIENT_ID.to_string());
        Self {
            client: CapiClient::new(transport),
            store,
            client_id,
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Inicia el flujo de autenticación PKCE, registrando el verificador temporal y generando la URL de Frontier.
    pub fn start_login(&self, redirect_uri: &str) -> (String, String) {
        let state = uuid::Uuid::new_v4().to_string();
        let (verifier, challenge) = generate_pkce_pair(&state);

        let now = Instant::now();
        if let Ok(mut pending) = self.pending.lock() {
            pending.retain(|_, (_, created)| now.duration_since(*created) < PENDING_STATE_TTL);
            pending.insert(state.clone(), (verifier, now));
        }

        let auth_url = build_auth_url(&self.client_id, redirect_uri, &challenge, &state);
        (auth_url, state)
    }

    /// Procesa el callback de OAuth2: valida el estado, canjea el código por tokens y los persiste de forma segura.
    pub async fn handle_callback(
        &self,
        code: &str,
        state: &str,
        redirect_uri: &str,
    ) -> Result<CapiToken, CapiError> {
        let verifier = {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| CapiError::Malformed("lock-poisoned".into()))?;
            let (v, created) = pending
                .remove(state)
                .ok_or_else(|| CapiError::InvalidCode(400, "state-not-found-or-expired".into()))?;
            if Instant::now().duration_since(created) >= PENDING_STATE_TTL {
                return Err(CapiError::InvalidCode(400, "state-expired".into()));
            }
            v
        };

        let token = self
            .client
            .exchange_code(&self.client_id, code, &verifier, redirect_uri)
            .await?;

        self.save_token(&token)?;

        Ok(token)
    }

    /// Obtiene el token de sesión almacenado si existe.
    pub fn get_token(&self) -> Result<Option<CapiToken>, CapiError> {
        let rec = self
            .store
            .get_account_credential(CAPI_SERVICE_KEY)
            .map_err(|e| CapiError::Malformed(e.to_string()))?;
        let Some(json) = rec.and_then(|r| r.token_json) else {
            return Ok(None);
        };
        let token = serde_json::from_str(&json).map_err(|e| CapiError::Malformed(e.to_string()))?;
        Ok(Some(token))
    }

    /// Guarda el token OAuth2 en SQLite.
    pub fn save_token(&self, token: &CapiToken) -> Result<(), CapiError> {
        let json = serde_json::to_string(token).map_err(|e| CapiError::Malformed(e.to_string()))?;
        let rec = AccountCredentialRecord {
            service: CAPI_SERVICE_KEY.to_string(),
            api_key: None,
            cmdr_name: None,
            token_json: Some(json),
            salt: None,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        self.store
            .set_account_credential(&rec)
            .map_err(|e| CapiError::Malformed(e.to_string()))
    }

    /// Refresca los tokens con Frontier Store y actualiza la persistencia.
    pub async fn refresh_session(&self) -> Result<CapiToken, CapiError> {
        let token = self
            .get_token()?
            .ok_or_else(|| CapiError::Unauthorized(401, "no-session".into()))?;

        let refreshed = self
            .client
            .refresh_token(&self.client_id, &token.refresh_token)
            .await?;

        self.save_token(&refreshed)?;

        Ok(refreshed)
    }

    /// Consulta la flota remota de naves del comandante vía `/profile`. Refresca automáticamente si el token caducó.
    pub async fn fetch_fleet(&self) -> Result<Vec<RemoteShip>, CapiError> {
        let token = self
            .get_token()?
            .ok_or_else(|| CapiError::Unauthorized(401, "no-session".into()))?;

        match self.client.fetch_profile(&token.access_token).await {
            Ok(profile) => Ok(profile.remote_ships),
            Err(CapiError::Unauthorized(_, _)) => {
                let refreshed = self.refresh_session().await?;
                let profile = self.client.fetch_profile(&refreshed.access_token).await?;
                Ok(profile.remote_ships)
            }
            Err(e) => Err(e),
        }
    }

    /// Elimina la sesión de Frontier de la base de datos (desvincular cuenta).
    pub fn clear_session(&self) -> Result<bool, StoreError> {
        self.store.delete_account_credential(CAPI_SERVICE_KEY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;
    use tmc_net::transport::{BoxFut, HttpReply, TransportError};

    struct MockTransport {
        reply: StdMutex<Option<Result<HttpReply, TransportError>>>,
    }

    impl MockTransport {
        fn new(reply: Result<HttpReply, TransportError>) -> Self {
            Self {
                reply: StdMutex::new(Some(reply)),
            }
        }
    }

    impl Transport for MockTransport {
        fn get(&self, _url: &str) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            let res = self
                .reply
                .lock()
                .unwrap()
                .take()
                .unwrap_or_else(|| Err(TransportError::Connect("no-reply".into())));
            Box::pin(async move { res })
        }

        fn post_form(
            &self,
            _url: &str,
            _params: &[(String, String)],
        ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            let res = self
                .reply
                .lock()
                .unwrap()
                .take()
                .unwrap_or_else(|| Err(TransportError::Connect("no-reply".into())));
            Box::pin(async move { res })
        }

        fn get_with_auth(
            &self,
            _url: &str,
            _token: &str,
        ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            let res = self
                .reply
                .lock()
                .unwrap()
                .take()
                .unwrap_or_else(|| Err(TransportError::Connect("no-reply".into())));
            Box::pin(async move { res })
        }
    }

    fn in_memory_store() -> Store {
        Store::open_in_memory().unwrap()
    }

    #[tokio::test]
    async fn login_flow_generates_valid_auth_url_and_handles_callback() {
        let store = in_memory_store();
        let token_payload = r#"{
            "access_token": "mock_access_tok_123",
            "refresh_token": "mock_refresh_tok_456",
            "token_type": "Bearer",
            "expires_in": 3600
        }"#;

        let transport = Arc::new(MockTransport::new(Ok(HttpReply {
            status: 200,
            body: token_payload.to_string(),
        })));

        let service = CapiService::new(store.clone(), transport, Some("test_client".to_string()));

        let redirect_uri = "https://localhost:47320/api/auth/frontier/callback";
        let (auth_url, state) = service.start_login(redirect_uri);

        assert!(auth_url.contains("https://auth.frontierstore.net/auth"));
        assert!(auth_url.contains("client_id=test_client"));
        assert!(auth_url.contains(&format!("state={state}")));
        assert!(auth_url.contains("code_challenge="));

        // Callback con estado desconocido debe fallar
        let err = service
            .handle_callback("code_xyz", "invalid_state", redirect_uri)
            .await
            .unwrap_err();
        assert!(matches!(err, CapiError::InvalidCode(400, _)));

        // Callback con estado legítimo canjea y persiste
        let token = service
            .handle_callback("valid_code", &state, redirect_uri)
            .await
            .unwrap();

        assert_eq!(token.access_token, "mock_access_tok_123");
        assert_eq!(token.refresh_token, "mock_refresh_tok_456");

        // Verificamos persistencia
        let persisted = service.get_token().unwrap().unwrap();
        assert_eq!(persisted.access_token, "mock_access_tok_123");

        // Desvincular
        assert!(service.clear_session().unwrap());
        assert_eq!(service.get_token().unwrap(), None);
    }
}
