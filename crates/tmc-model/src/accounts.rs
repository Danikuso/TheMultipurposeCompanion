//! Modelos de datos y topics para cuentas y sincronizaciones externas (EDDN, EDSM, Inara, Frontier CAPI).

use crate::sync::{
    CmdrRef, CommanderRow, EddnServiceStatus, KeyedServiceStatus, PendingByService, SyncService,
};
use serde::{Deserialize, Serialize};

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
    pub current_cmdr: Option<CmdrRef>,
    pub inara: KeyedServiceStatus,
    pub edsm: KeyedServiceStatus,
    pub eddn: EddnServiceStatus,
    pub commanders: Vec<CommanderRow>,
    pub other_cmdrs_pending: PendingByService,
    pub unassigned_keys: Vec<SyncService>,
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
    #[serde(rename = "isBeingDeveloped")]
    pub is_being_developed: bool,
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
    fn inara_header_uses_is_being_developed() {
        let header = InaraHeader {
            app_name: "TMC".into(),
            app_version: "1".into(),
            is_being_developed: false,
            api_key: "k".into(),
            commander_name: None,
        };
        let json = serde_json::to_string(&header).unwrap();
        assert!(json.contains("\"isBeingDeveloped\":false"));
        assert!(!json.contains("isDeveloped"));
    }

    #[test]
    fn accounts_status_topic_defaults() {
        let topic = AccountsStatusTopic::default();
        assert!(!topic.eddn.enabled);
        assert_eq!(topic.eddn.status.state, crate::sync::SyncState::Disabled);
        assert_eq!(
            topic.eddn.status.reason,
            Some(crate::sync::SyncReason::SwitchOff)
        );
        assert!(!topic.edsm.has_key);
        assert_eq!(
            topic.edsm.status.reason,
            Some(crate::sync::SyncReason::NoKey)
        );
        assert!(!topic.inara.has_key);
        assert_eq!(topic.inara.status.pending, 0);
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
                is_being_developed: false,
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
