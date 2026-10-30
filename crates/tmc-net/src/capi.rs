//! Cliente de Frontier Companion API (CAPI) con autenticación OAuth2 PKCE y telemetría de flota remota.

use crate::transport::{HttpReply, Transport};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tmc_model::accounts::RemoteShip;

pub const FRONTIER_AUTH_URL: &str = "https://auth.frontierstore.net/auth";
pub const FRONTIER_TOKEN_URL: &str = "https://auth.frontierstore.net/token";
pub const FRONTIER_CAPI_BASE: &str = "https://companion.orerve.net";
pub const DEFAULT_REDIRECT_URI: &str = "https://localhost:47320/api/auth/frontier/callback";
pub const OFFICIAL_REDIRECT_URI: &str =
    "https://danikuso.github.io/TheMultipurposeCompanion/oauth/callback.html";
pub const DEFAULT_SCOPE: &str = "auth capi";

const BASE64URL_CHARS: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Codificación Base64URL sin relleno según RFC 7636.
pub fn base64url_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() * 4).div_ceil(3));
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i];
        let b1 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b2 = if i + 2 < input.len() { input[i + 2] } else { 0 };

        let idx0 = (b0 >> 2) as usize;
        let idx1 = (((b0 & 0x03) << 4) | (b1 >> 4)) as usize;
        let idx2 = (((b1 & 0x0f) << 2) | (b2 >> 6)) as usize;
        let idx3 = (b2 & 0x3f) as usize;

        out.push(BASE64URL_CHARS[idx0] as char);
        out.push(BASE64URL_CHARS[idx1] as char);
        if i + 1 < input.len() {
            out.push(BASE64URL_CHARS[idx2] as char);
        }
        if i + 2 < input.len() {
            out.push(BASE64URL_CHARS[idx3] as char);
        }
        i += 3;
    }
    out
}

/// Genera el par PKCE `(code_verifier, code_challenge)` conforme a RFC 7636 (S256).
pub fn generate_pkce_pair(seed: &str) -> (String, String) {
    let mut v_hasher = Sha256::new();
    v_hasher.update(seed.as_bytes());
    v_hasher.update(b":tmc_capi_verifier:");
    v_hasher.update(format!("{:?}", std::time::Instant::now()).as_bytes());
    let hex = format!("{:x}{:x}", v_hasher.clone().finalize(), v_hasher.finalize());
    let verifier = hex[0..64].to_string();

    let mut ch_hasher = Sha256::new();
    ch_hasher.update(verifier.as_bytes());
    let challenge = base64url_encode(&ch_hasher.finalize());

    (verifier, challenge)
}

/// Construye la URL de inicio de sesión de Frontier Store con OAuth2 + PKCE.
pub fn build_auth_url(
    client_id: &str,
    redirect_uri: &str,
    code_challenge: &str,
    state: &str,
) -> String {
    format!(
        "{FRONTIER_AUTH_URL}?response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&state={}",
        crate::transport::url_encode(client_id),
        crate::transport::url_encode(redirect_uri),
        crate::transport::url_encode(DEFAULT_SCOPE),
        crate::transport::url_encode(code_challenge),
        crate::transport::url_encode(state),
    )
}

/// Tokens de sesión OAuth2 devueltos por Frontier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapiToken {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

/// Datos del perfil del comandante extraídos de `/profile`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CapiProfile {
    pub commander_name: String,
    pub credits: u64,
    pub debt: u64,
    pub current_ship_id: Option<u64>,
    pub remote_ships: Vec<RemoteShip>,
}

/// Errores durante la comunicación con Frontier CAPI.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CapiError {
    #[error("falta el Client ID de Frontier CAPI")]
    MissingClientId,
    #[error("código de autorización inválido o expirado (HTTP {0}): {1}")]
    InvalidCode(u16, String),
    #[error("sesión no autorizada o token caducado (HTTP {0}): {1}")]
    Unauthorized(u16, String),
    #[error("respuesta malformada de la CAPI: {0}")]
    Malformed(String),
    #[error("error de transporte de red: {0}")]
    Transport(String),
}

/// Cliente para la API oficial Frontier CAPI.
pub struct CapiClient {
    transport: Arc<dyn Transport>,
}

impl CapiClient {
    pub fn new(transport: Arc<dyn Transport>) -> Self {
        Self { transport }
    }

    /// Canjea el código de autorización temporal por tokens OAuth2 mediante PKCE.
    pub async fn exchange_code(
        &self,
        client_id: &str,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<CapiToken, CapiError> {
        if client_id.trim().is_empty() {
            return Err(CapiError::MissingClientId);
        }

        let form = vec![
            ("grant_type".to_string(), "authorization_code".to_string()),
            ("client_id".to_string(), client_id.trim().to_string()),
            ("code".to_string(), code.trim().to_string()),
            (
                "code_verifier".to_string(),
                code_verifier.trim().to_string(),
            ),
            ("redirect_uri".to_string(), redirect_uri.trim().to_string()),
        ];

        let reply = self
            .transport
            .post_form(FRONTIER_TOKEN_URL, &form)
            .await
            .map_err(|e| CapiError::Transport(format!("{e:?}")))?;

        parse_token_reply(&reply)
    }

    /// Renueva el token de acceso usando el token de refresco.
    pub async fn refresh_token(
        &self,
        client_id: &str,
        refresh_token: &str,
    ) -> Result<CapiToken, CapiError> {
        if client_id.trim().is_empty() {
            return Err(CapiError::MissingClientId);
        }

        let form = vec![
            ("grant_type".to_string(), "refresh_token".to_string()),
            ("client_id".to_string(), client_id.trim().to_string()),
            (
                "refresh_token".to_string(),
                refresh_token.trim().to_string(),
            ),
        ];

        let reply = self
            .transport
            .post_form(FRONTIER_TOKEN_URL, &form)
            .await
            .map_err(|e| CapiError::Transport(format!("{e:?}")))?;

        parse_token_reply(&reply)
    }

    /// Obtiene el perfil completo y las naves almacenadas en toda la galaxia (`/profile`).
    pub async fn fetch_profile(&self, access_token: &str) -> Result<CapiProfile, CapiError> {
        let url = format!("{FRONTIER_CAPI_BASE}/profile");
        let reply = self
            .transport
            .get_with_auth(&url, access_token)
            .await
            .map_err(|e| CapiError::Transport(format!("{e:?}")))?;

        if reply.status == 401 || reply.status == 403 {
            return Err(CapiError::Unauthorized(reply.status, reply.body));
        }
        if !(200..=299).contains(&reply.status) {
            return Err(CapiError::Transport(format!("HTTP {}", reply.status)));
        }

        parse_profile(&reply.body)
    }
}

fn parse_token_reply(reply: &HttpReply) -> Result<CapiToken, CapiError> {
    if reply.status == 400 || reply.status == 401 {
        return Err(CapiError::InvalidCode(reply.status, reply.body.clone()));
    }
    if !(200..=299).contains(&reply.status) {
        return Err(CapiError::Transport(format!("HTTP {}", reply.status)));
    }

    let val: Value = serde_json::from_str(&reply.body)
        .map_err(|e| CapiError::Malformed(format!("JSON inválido: {e}")))?;

    let access_token = val
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CapiError::Malformed("Falta access_token".into()))?
        .to_string();

    let refresh_token = val
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CapiError::Malformed("Falta refresh_token".into()))?
        .to_string();

    let token_type = val
        .get("token_type")
        .and_then(|v| v.as_str())
        .unwrap_or("Bearer")
        .to_string();

    let expires_in = val
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .unwrap_or(86400);

    Ok(CapiToken {
        access_token,
        refresh_token,
        token_type,
        expires_in,
    })
}

/// Extrae el perfil del comandante y sus naves remotas desde el volcado JSON de `/profile`.
pub fn parse_profile(body: &str) -> Result<CapiProfile, CapiError> {
    let val: Value = serde_json::from_str(body)
        .map_err(|e| CapiError::Malformed(format!("JSON inválido: {e}")))?;

    let commander = val.get("commander").unwrap_or(&Value::Null);
    let commander_name = commander
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("CMDR")
        .to_string();
    let credits = commander
        .get("credits")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let debt = commander.get("debt").and_then(|v| v.as_u64()).unwrap_or(0);
    let current_ship_id = commander.get("currentShipId").and_then(|v| v.as_u64());

    let mut remote_ships = Vec::new();
    if let Some(ships_obj) = val.get("ships").and_then(|s| s.as_object()) {
        for (ship_id_str, ship_val) in ships_obj {
            let id = ship_id_str
                .parse::<u64>()
                .unwrap_or_else(|_| ship_val.get("id").and_then(|v| v.as_u64()).unwrap_or(0));

            // En Frontier CAPI:
            // - "name" suele ser el identificador o modelo de la nave (ej: "Mandalay", "Python_NX", "MediumTransport01", "Viper").
            // - "shipName" es el nombre personalizado asignado por el comandante.
            // - Si "type" está presente (en mocks o endpoints secundarios), se toma como modelo y "name" como nombre propio.
            let (ship_type, name) = if let Some(custom) = ship_val
                .get("shipName")
                .or_else(|| ship_val.get("ship_name"))
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
            {
                let model = ship_val
                    .get("name")
                    .or_else(|| ship_val.get("type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                (model.to_string(), Some(custom.to_string()))
            } else if let Some(model) = ship_val.get("type").and_then(|v| v.as_str()) {
                let custom = ship_val
                    .get("name")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.to_string());
                (model.to_string(), custom)
            } else {
                let model = ship_val
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                (model.to_string(), None)
            };

            let station = ship_val
                .get("station")
                .and_then(|s| s.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("En Tránsito")
                .to_string();

            let system = ship_val
                .get("starsystem")
                .and_then(|s| s.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("Espacio Profundo")
                .to_string();

            let value = ship_val
                .get("value")
                .and_then(|v| v.get("total"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            remote_ships.push(RemoteShip {
                id,
                name,
                ship_type,
                station,
                system,
                value,
                distance_ly: None,
                transfer_time_secs: None,
            });
        }
    }

    Ok(CapiProfile {
        commander_name,
        credits,
        debt,
        current_ship_id,
        remote_ships,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{BoxFut, TransportError};
    use std::sync::Mutex;

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
            let mut g = self.replies.lock().unwrap();
            let r = if g.is_empty() {
                Ok(HttpReply {
                    status: 200,
                    body: r#"{"access_token":"at_123","refresh_token":"rt_456","token_type":"Bearer","expires_in":86400}"#.to_string(),
                })
            } else {
                g.remove(0)
            };
            Box::pin(async move { r })
        }

        fn get_with_auth(
            &self,
            _url: &str,
            _token: &str,
        ) -> BoxFut<'_, Result<HttpReply, TransportError>> {
            let mut g = self.replies.lock().unwrap();
            let r = if g.is_empty() {
                Ok(HttpReply {
                    status: 200,
                    body: r#"{
                        "commander": { "name": "Danikuso", "credits": 500000000, "debt": 0, "currentShipId": 1 },
                        "ships": {
                            "1": { "id": 1, "name": "Invictus", "type": "cutter", "starsystem": { "name": "Achenar" }, "station": { "name": "Dawes Hub" }, "value": { "total": 1200000000 } },
                            "2": { "id": 2, "name": "Explorer", "type": "anaconda", "starsystem": { "name": "Colonia" }, "station": { "name": "Jaques Station" }, "value": { "total": 600000000 } }
                        }
                    }"#.to_string(),
                })
            } else {
                g.remove(0)
            };
            Box::pin(async move { r })
        }
    }

    #[test]
    fn base64url_encoding_matches_standard() {
        assert_eq!(base64url_encode(b""), "");
        assert_eq!(base64url_encode(b"f"), "Zg");
        assert_eq!(base64url_encode(b"fo"), "Zm8");
        assert_eq!(base64url_encode(b"foo"), "Zm9v");
    }

    #[test]
    fn pkce_generation_and_auth_url() {
        let (verifier, challenge) = generate_pkce_pair("test_seed");
        assert_eq!(verifier.len(), 64);
        assert!(!challenge.is_empty());
        assert!(!challenge.contains('+'));
        assert!(!challenge.contains('/'));
        assert!(!challenge.contains('='));

        let url = build_auth_url("my_client_id", DEFAULT_REDIRECT_URI, &challenge, "state123");
        assert!(url.contains("https://auth.frontierstore.net/auth?response_type=code"));
        assert!(url.contains("client_id=my_client_id"));
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
    }

    #[tokio::test]
    async fn exchanges_code_successfully() {
        let transport = Arc::new(MockTransport::new(vec![Ok(HttpReply {
            status: 200,
            body: r#"{"access_token":"acc_abc","refresh_token":"ref_xyz","token_type":"Bearer","expires_in":7200}"#.to_string(),
        })]));
        let client = CapiClient::new(transport);

        let token = client
            .exchange_code("cli_1", "code_1", "ver_1", DEFAULT_REDIRECT_URI)
            .await
            .unwrap();

        assert_eq!(token.access_token, "acc_abc");
        assert_eq!(token.refresh_token, "ref_xyz");
        assert_eq!(token.expires_in, 7200);
    }

    #[tokio::test]
    async fn refreshes_token_successfully() {
        let transport = Arc::new(MockTransport::new(vec![Ok(HttpReply {
            status: 200,
            body: r#"{"access_token":"acc_new","refresh_token":"ref_new","token_type":"Bearer","expires_in":86400}"#.to_string(),
        })]));
        let client = CapiClient::new(transport);

        let token = client.refresh_token("cli_1", "ref_old").await.unwrap();

        assert_eq!(token.access_token, "acc_new");
        assert_eq!(token.refresh_token, "ref_new");
    }

    #[tokio::test]
    async fn fetches_profile_and_parses_remote_ships() {
        let transport = Arc::new(MockTransport::new(vec![]));
        let client = CapiClient::new(transport);

        let profile = client.fetch_profile("valid_token").await.unwrap();
        assert_eq!(profile.commander_name, "Danikuso");
        assert_eq!(profile.credits, 500_000_000);
        assert_eq!(profile.remote_ships.len(), 2);

        let cutter = profile
            .remote_ships
            .iter()
            .find(|s| s.ship_type == "cutter")
            .unwrap();
        assert_eq!(cutter.name.as_deref(), Some("Invictus"));
        assert_eq!(cutter.station, "Dawes Hub");
        assert_eq!(cutter.system, "Achenar");
        assert_eq!(cutter.value, 1_200_000_000);
    }

    #[test]
    fn parses_real_capi_ships_without_type_field() {
        let json = r#"{
            "commander": { "name": "Danikuso", "credits": 500000000, "debt": 0 },
            "ships": {
                "10": { "name": "Mandalay", "starsystem": { "name": "PSR J1752-2806" }, "station": { "name": "The Plaza" }, "value": { "total": 29131160 } },
                "11": { "name": "Python_NX", "shipName": "Vortex", "starsystem": { "name": "Sol" }, "station": { "name": "Li Qing Jao" }, "value": { "total": 117248604 } }
            }
        }"#;
        let profile = parse_profile(json).unwrap();
        assert_eq!(profile.remote_ships.len(), 2);
        let m = profile.remote_ships.iter().find(|s| s.id == 10).unwrap();
        assert_eq!(m.ship_type, "Mandalay");
        assert_eq!(m.name, None);

        let p = profile.remote_ships.iter().find(|s| s.id == 11).unwrap();
        assert_eq!(p.ship_type, "Python_NX");
        assert_eq!(p.name.as_deref(), Some("Vortex"));
    }
}
