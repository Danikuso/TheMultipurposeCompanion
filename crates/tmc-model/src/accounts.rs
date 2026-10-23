//! Modelos de datos y topics para cuentas y sincronizaciones externas (EDDN, EDSM, Inara, Frontier CAPI).

use serde::{Deserialize, Serialize};

/// Ajustes de cuentas de servicios de sincronización.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AccountSettings {
    /// Envío a EDDN activado (opt-in/opt-out).
    pub eddn_enabled: bool,
    /// Sal criptográfica para anonimizar el uploaderID de EDDN.
    pub eddn_uploader_salt: Option<String>,
    /// Sincronización con EDSM activada.
    pub edsm_enabled: bool,
    /// Nombre del comandante en EDSM.
    pub edsm_cmdr_name: Option<String>,
    /// Sincronización con Inara activada.
    pub inara_enabled: bool,
    /// Nombre del comandante en Inara.
    pub inara_cmdr_name: Option<String>,
    /// Integración con Frontier CAPI activada.
    pub capi_enabled: bool,
    /// Client ID opcional de Frontier CAPI.
    pub capi_client_id: Option<String>,
}

impl Default for AccountSettings {
    fn default() -> Self {
        Self {
            eddn_enabled: true,
            eddn_uploader_salt: None,
            edsm_enabled: false,
            edsm_cmdr_name: None,
            inara_enabled: false,
            inara_cmdr_name: None,
            capi_enabled: false,
            capi_client_id: None,
        }
    }
}

/// Estado de la transmisión a EDDN.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EddnStatus {
    pub enabled: bool,
    pub connected: bool,
    pub uploader_id: String,
    pub messages_sent_session: u32,
    pub last_upload_time: Option<String>,
    pub last_error: Option<String>,
}

impl Default for EddnStatus {
    fn default() -> Self {
        Self {
            enabled: true,
            connected: false,
            uploader_id: String::new(),
            messages_sent_session: 0,
            last_upload_time: None,
            last_error: None,
        }
    }
}

/// Estado de la sincronización con EDSM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EdsmStatus {
    pub enabled: bool,
    pub has_key: bool,
    pub commander_name: Option<String>,
    pub last_sync_time: Option<String>,
    pub last_error: Option<String>,
    pub syncing: bool,
}

/// Estado de la sincronización con Inara.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InaraStatus {
    pub enabled: bool,
    pub has_key: bool,
    pub commander_name: Option<String>,
    pub last_sync_time: Option<String>,
    pub last_error: Option<String>,
    pub pending_events: u32,
    pub next_slot_secs: u32,
    pub syncing: bool,
}

/// Estado de la conexión con Frontier CAPI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CapiStatus {
    pub configured: bool,
    pub connected: bool,
    pub commander_name: Option<String>,
    pub token_expires_at: Option<String>,
    pub remote_ships_count: u32,
    pub last_sync_time: Option<String>,
    pub last_error: Option<String>,
}

/// Topic WebSocket `accounts.status` servido a la interfaz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AccountsStatusTopic {
    pub eddn: EddnStatus,
    pub edsm: EdsmStatus,
    pub inara: InaraStatus,
    pub capi: CapiStatus,
}

/// Nave remota almacenada en un astillero galáctico (recuperada vía CAPI).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteShip {
    pub id: u64,
    pub name: Option<String>,
    pub ship_type: String,
    pub station: String,
    pub system: String,
    pub value: u64,
    pub distance_ly: Option<f64>,
    pub transfer_time_secs: Option<u64>,
}

/// Cabecera obligatoria de un mensaje EDDN.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EddnHeader {
    #[serde(rename = "uploaderID")]
    pub uploader_id: String,
    #[serde(rename = "softwareName")]
    pub software_name: String,
    #[serde(rename = "softwareVersion")]
    pub software_version: String,
    #[serde(rename = "gatewayTimestamp")]
    pub gateway_timestamp: String,
}

/// Envelope oficial de envío a EDDN.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EddnEnvelope<T> {
    #[serde(rename = "$schemaRef")]
    pub schema_ref: String,
    pub header: EddnHeader,
    pub message: T,
}

/// Cabecera del envelope Inara InAPI v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InaraHeader {
    #[serde(rename = "appName")]
    pub app_name: String,
    #[serde(rename = "appVersion")]
    pub app_version: String,
    #[serde(rename = "isDeveloped")]
    pub is_developed: bool,
    #[serde(rename = "APIkey")]
    pub api_key: String,
    #[serde(rename = "commanderName", skip_serializing_if = "Option::is_none")]
    pub commander_name: Option<String>,
}

/// Evento individual dentro de un lote de Inara InAPI v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InaraEvent<T> {
    #[serde(rename = "eventName")]
    pub event_name: String,
    #[serde(rename = "eventTimestamp")]
    pub event_timestamp: String,
    #[serde(rename = "eventData")]
    pub event_data: T,
}

/// Lote oficial de Inara InAPI v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InaraBatchEnvelope<T> {
    pub header: InaraHeader,
    pub events: Vec<InaraEvent<T>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_settings_defaults() {
        let s = AccountSettings::default();
        assert!(s.eddn_enabled);
        assert!(!s.edsm_enabled);
        assert!(!s.inara_enabled);
        assert!(!s.capi_enabled);
        assert_eq!(s.eddn_uploader_salt, None);
    }

    #[test]
    fn accounts_status_topic_defaults() {
        let topic = AccountsStatusTopic::default();
        assert!(topic.eddn.enabled);
        assert!(!topic.edsm.enabled);
        assert_eq!(topic.inara.pending_events, 0);
        assert_eq!(topic.capi.remote_ships_count, 0);
    }

    #[test]
    fn eddn_envelope_serialization() {
        let env = EddnEnvelope {
            schema_ref: "https://eddn.edcd.io/schemas/commodity/3".to_string(),
            header: EddnHeader {
                uploader_id: "test_salt_id".to_string(),
                software_name: "TMC".to_string(),
                software_version: "0.1.0".to_string(),
                gateway_timestamp: "2026-10-23T12:00:00Z".to_string(),
            },
            message: serde_json::json!({
                "systemName": "Sol",
                "stationName": "Li Qing Jao"
            }),
        };

        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains("\"$schemaRef\":\"https://eddn.edcd.io/schemas/commodity/3\""));
        assert!(json.contains("\"uploaderID\":\"test_salt_id\""));
        assert!(json.contains("\"softwareName\":\"TMC\""));
    }

    #[test]
    fn inara_batch_envelope_serialization() {
        let batch = InaraBatchEnvelope {
            header: InaraHeader {
                app_name: "TMC".to_string(),
                app_version: "0.1.0".to_string(),
                is_developed: false,
                api_key: "key123".to_string(),
                commander_name: Some("Danikuso".to_string()),
            },
            events: vec![InaraEvent {
                event_name: "setCommanderProfile".to_string(),
                event_timestamp: "2026-10-23T12:00:00Z".to_string(),
                event_data: serde_json::json!({ "credits": 500000000 }),
            }],
        };

        let json = serde_json::to_string(&batch).unwrap();
        assert!(json.contains("\"appName\":\"TMC\""));
        assert!(json.contains("\"APIkey\":\"key123\""));
        assert!(json.contains("\"eventName\":\"setCommanderProfile\""));
    }

    #[test]
    fn remote_ship_serialization() {
        let ship = RemoteShip {
            id: 42,
            name: Some("Invictus".to_string()),
            ship_type: "cutter".to_string(),
            station: "Dawes Hub".to_string(),
            system: "Achenar".to_string(),
            value: 1_200_000_000,
            distance_ly: Some(74.5),
            transfer_time_secs: Some(360),
        };
        let json = serde_json::to_string(&ship).unwrap();
        assert!(json.contains("\"name\":\"Invictus\""));
        assert!(json.contains("\"cutter\""));
        assert!(json.contains("1200000000"));
    }
}
