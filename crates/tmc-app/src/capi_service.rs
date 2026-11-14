//! Servicio de autenticación y sincronización con Frontier Companion API (CAPI).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tmc_model::accounts::RemoteShip;
use tmc_net::Transport;
pub use tmc_net::capi::{
    CapiClient, CapiError, CapiProfile, CapiToken, build_auth_url, generate_pkce_pair,
    generate_random_pkce_pair,
};
use tmc_store::{AccountCredentialRecord, Store, StoreError};

const PENDING_STATE_TTL: Duration = Duration::from_secs(600);
const DEFAULT_CLIENT_ID: &str = "12fbec0d-b5c9-4911-af18-0e7a4ede266e";
const CAPI_SERVICE_KEY: &str = "frontier_capi";

#[derive(Clone)]
struct PendingAuth {
    verifier: String,
    fid: Option<String>,
    created: Instant,
}

fn service_key_for(fid: Option<&str>) -> String {
    match fid.map(str::trim).filter(|f| !f.is_empty()) {
        Some(f) => format!("{CAPI_SERVICE_KEY}:{f}"),
        None => CAPI_SERVICE_KEY.to_string(),
    }
}

/// Servicio de integración Frontier CAPI con gestión de ciclo de vida OAuth2 PKCE y flota remota.
pub struct CapiService {
    client: CapiClient,
    store: Store,
    client_id: String,
    active_fid: Arc<Mutex<Option<String>>>,
    pending: Arc<Mutex<HashMap<String, PendingAuth>>>,
}

impl CapiService {
    pub fn new(store: Store, transport: Arc<dyn Transport>, client_id: Option<String>) -> Self {
        let client_id = client_id
            .or_else(|| crate::env::env_var_or_file("TMC_CAPI_CLIENT_ID"))
            .unwrap_or_else(|| DEFAULT_CLIENT_ID.to_string());
        Self {
            client: CapiClient::new(transport),
            store,
            client_id,
            active_fid: Arc::new(Mutex::new(None)),
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Actualiza el FID del comandante activo para aislar sesiones CAPI multicomandante.
    pub fn set_active_fid(&self, fid: Option<&str>) {
        if let Ok(mut guard) = self.active_fid.lock() {
            *guard = fid
                .map(str::trim)
                .filter(|f| !f.is_empty())
                .map(str::to_owned);
        }
    }

    pub fn active_fid(&self) -> Option<String> {
        self.active_fid.lock().ok().and_then(|g| g.clone())
    }

    fn known_cmdrs_count(&self) -> usize {
        self.store.known_cmdrs().map(|v| v.len()).unwrap_or(0)
    }

    fn resolve_effective_fid(&self) -> Option<String> {
        if let Some(fid) = self.active_fid() {
            return Some(fid);
        }
        let known = self.store.known_cmdrs().ok()?;
        if known.len() == 1 {
            return Some(known[0].0.clone());
        }
        None
    }

    /// Inicia el flujo de autenticación PKCE, registrando el verificador temporal y generando la URL de Frontier.
    pub fn start_login(&self, redirect_uri: &str) -> (String, String) {
        let state = uuid::Uuid::new_v4().to_string();
        let (verifier, challenge) = generate_random_pkce_pair();
        let fid = self.resolve_effective_fid();

        let now = Instant::now();
        if let Ok(mut pending) = self.pending.lock() {
            pending.retain(|_, entry| now.duration_since(entry.created) < PENDING_STATE_TTL);
            pending.insert(
                state.clone(),
                PendingAuth {
                    verifier,
                    fid,
                    created: now,
                },
            );
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
        let (verifier, login_fid) = {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| CapiError::Malformed("lock-poisoned".into()))?;
            let entry = pending
                .remove(state)
                .ok_or_else(|| CapiError::InvalidCode(400, "state-not-found-or-expired".into()))?;
            if Instant::now().duration_since(entry.created) >= PENDING_STATE_TTL {
                return Err(CapiError::InvalidCode(400, "state-expired".into()));
            }
            (entry.verifier, entry.fid)
        };

        let token = self
            .client
            .exchange_code(&self.client_id, code, &verifier, redirect_uri)
            .await?;

        let target_fid = login_fid.or_else(|| self.resolve_effective_fid());
        self.save_token_for(target_fid.as_deref(), &token)?;

        Ok(token)
    }

    /// Obtiene el token de sesión del comandante activo (con respaldo transparente a clave heredada en cuentas únicas).
    pub fn get_token(&self) -> Result<Option<CapiToken>, CapiError> {
        self.get_token_for(self.resolve_effective_fid().as_deref())
    }

    /// Obtiene el token de sesión de un comandante específico (o clave heredada si `fid` es `None` o cuenta única).
    pub fn get_token_for(&self, fid: Option<&str>) -> Result<Option<CapiToken>, CapiError> {
        let scoped_key = service_key_for(fid);
        let rec = self
            .store
            .get_account_credential(&scoped_key)
            .map_err(|e| CapiError::Malformed(e.to_string()))?;
        let json = match rec.and_then(|r| r.token_json) {
            Some(j) => j,
            None if fid.is_some() && self.known_cmdrs_count() <= 1 => {
                let Some(legacy) = self
                    .store
                    .get_account_credential(CAPI_SERVICE_KEY)
                    .map_err(|e| CapiError::Malformed(e.to_string()))?
                    .and_then(|r| r.token_json)
                else {
                    return Ok(None);
                };
                legacy
            }
            None => return Ok(None),
        };
        let token = serde_json::from_str(&json).map_err(|e| CapiError::Malformed(e.to_string()))?;
        Ok(Some(token))
    }

    /// Guarda el token OAuth2 en SQLite para el comandante activo.
    pub fn save_token(&self, token: &CapiToken) -> Result<(), CapiError> {
        self.save_token_for(self.resolve_effective_fid().as_deref(), token)
    }

    /// Guarda el token OAuth2 en SQLite aislado bajo `frontier_capi:<fid>` cuando se conoce el comandante.
    pub fn save_token_for(&self, fid: Option<&str>, token: &CapiToken) -> Result<(), CapiError> {
        let key = service_key_for(fid);
        let json = serde_json::to_string(token).map_err(|e| CapiError::Malformed(e.to_string()))?;
        let rec = AccountCredentialRecord {
            service: key,
            api_key: None,
            cmdr_name: None,
            token_json: Some(json),
            salt: None,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        self.store
            .set_account_credential(&rec)
            .map_err(|e| CapiError::Malformed(e.to_string()))?;
        // Limpiar clave global heredada cuando se migra a almacenamiento por FID en cuenta única.
        if fid.is_some() && self.known_cmdrs_count() <= 1 {
            let _ = self.store.delete_account_credential(CAPI_SERVICE_KEY);
        }
        Ok(())
    }

    /// Refresca los tokens con Frontier Store y actualiza la persistencia.
    pub async fn refresh_session(&self) -> Result<CapiToken, CapiError> {
        let fid = self.resolve_effective_fid();
        let token = self
            .get_token_for(fid.as_deref())?
            .ok_or_else(|| CapiError::Unauthorized(401, "no-session".into()))?;

        let refreshed = self
            .client
            .refresh_token(&self.client_id, &token.refresh_token)
            .await?;

        self.save_token_for(fid.as_deref(), &refreshed)?;

        Ok(refreshed)
    }

    /// Consulta el perfil completo del comandante vía `/profile`. Refresca automáticamente si el token caducó.
    pub async fn fetch_profile(&self) -> Result<CapiProfile, CapiError> {
        let token = self
            .get_token()?
            .ok_or_else(|| CapiError::Unauthorized(401, "no-session".into()))?;

        match self.client.fetch_profile(&token.access_token).await {
            Ok(profile) => Ok(profile),
            Err(CapiError::Unauthorized(_, _)) => {
                let refreshed = self.refresh_session().await?;
                let profile = self.client.fetch_profile(&refreshed.access_token).await?;
                Ok(profile)
            }
            Err(e) => Err(e),
        }
    }

    /// Consulta la flota remota de naves del comandante vía `/profile`. Refresca automáticamente si el token caducó.
    pub async fn fetch_fleet(&self) -> Result<Vec<RemoteShip>, CapiError> {
        self.fetch_profile().await.map(|p| p.remote_ships)
    }

    /// Elimina la sesión de Frontier del comandante activo en la base de datos (desvincular cuenta).
    pub fn clear_session(&self) -> Result<bool, StoreError> {
        self.clear_session_for(self.resolve_effective_fid().as_deref())
    }

    /// Elimina únicamente la sesión de Frontier del comandante especificado (`frontier_capi:<fid>`).
    pub fn clear_session_for(&self, fid: Option<&str>) -> Result<bool, StoreError> {
        let key = service_key_for(fid);
        let removed_scoped = self.store.delete_account_credential(&key)?;
        let removed_legacy = if fid.is_some() && self.known_cmdrs_count() <= 1 {
            self.store.delete_account_credential(CAPI_SERVICE_KEY)?
        } else {
            false
        };
        Ok(removed_scoped || removed_legacy)
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

    #[test]
    fn test_multi_commander_token_isolation_and_legacy_fallback() {
        let store = in_memory_store();
        let now = chrono::Utc::now();
        store.upsert_known_cmdr("F111", "Alpha", now).unwrap();

        let service = CapiService::new(
            store.clone(),
            Arc::new(MockTransport::new(Err(TransportError::Connect(
                "unused".into(),
            )))),
            Some("test_client".to_string()),
        );

        let legacy_tok = CapiToken {
            access_token: "legacy_access".into(),
            refresh_token: "legacy_refresh".into(),
            token_type: "Bearer".into(),
            expires_in: 3600,
        };
        service.save_token_for(None, &legacy_tok).unwrap();
        assert_eq!(
            service.get_token_for(Some("F111")).unwrap(),
            Some(legacy_tok.clone())
        );

        // Registrar segundo comandante: los tokens quedan aislados por FID y sin fallback cruzado.
        store.upsert_known_cmdr("F222", "Beta", now).unwrap();
        assert_eq!(service.get_token_for(Some("F111")).unwrap(), None);
        assert_eq!(service.get_token_for(Some("F222")).unwrap(), None);

        let tok_alpha = CapiToken {
            access_token: "alpha_access".into(),
            refresh_token: "alpha_refresh".into(),
            token_type: "Bearer".into(),
            expires_in: 3600,
        };
        let tok_beta = CapiToken {
            access_token: "beta_access".into(),
            refresh_token: "beta_refresh".into(),
            token_type: "Bearer".into(),
            expires_in: 7200,
        };

        service.set_active_fid(Some("F111"));
        service.save_token(&tok_alpha).unwrap();
        service.set_active_fid(Some("F222"));
        service.save_token(&tok_beta).unwrap();

        service.set_active_fid(Some("F111"));
        assert_eq!(service.get_token().unwrap().as_ref(), Some(&tok_alpha));
        assert!(service.clear_session().unwrap());
        assert_eq!(service.get_token().unwrap(), None);

        service.set_active_fid(Some("F222"));
        assert_eq!(service.get_token().unwrap().as_ref(), Some(&tok_beta));
    }
}
