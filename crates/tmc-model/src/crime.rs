//! Modelos de datos para el módulo de Crimen, Notoriedad y Piratería (Fase 9).

use serde::{Deserialize, Serialize};

/// Tipos de delito reconocidos en Elite Dangerous.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrimeType {
    Assault,
    Murder,
    Piracy,
    Interdiction,
    RecklessWeaponsDischarge,
    IllegalCargo,
    DisobeyPolice,
    DockingMajorBlockingAvenue,
    DockingMinorBlockingAvenue,
    DockingMajorTresspass,
    DockingMinorTresspass,
    FireInNoFireZone,
    FireInStation,
    CollidedAtSpeedInStation,
    #[serde(untagged)]
    Other(String),
}

impl CrimeType {
    /// Indica si el delito incrementa la notoriedad del piloto (crímenes de sangre).
    pub fn increases_notoriety(&self) -> bool {
        matches!(self, Self::Murder)
    }

    /// Devuelve el identificador de cadena para serialización y UI.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Assault => "assault",
            Self::Murder => "murder",
            Self::Piracy => "piracy",
            Self::Interdiction => "interdiction",
            Self::RecklessWeaponsDischarge => "reckless_weapons_discharge",
            Self::IllegalCargo => "illegal_cargo",
            Self::DisobeyPolice => "disobey_police",
            Self::DockingMajorBlockingAvenue => "docking_major_blocking_avenue",
            Self::DockingMinorBlockingAvenue => "docking_minor_blocking_avenue",
            Self::DockingMajorTresspass => "docking_major_tresspass",
            Self::DockingMinorTresspass => "docking_minor_tresspass",
            Self::FireInNoFireZone => "fire_in_no_fire_zone",
            Self::FireInStation => "fire_in_station",
            Self::CollidedAtSpeedInStation => "collided_at_speed_in_station",
            Self::Other(s) => s.as_str(),
        }
    }
}

/// Evento `CommitCrime` del diario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CommitCrimeEvent {
    pub crime_type: CrimeType,
    pub faction: String,
    pub victim: Option<String>,
    pub fine: Option<u64>,
    pub bounty: Option<u64>,
}

/// Evento `CrimeVictim` del diario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CrimeVictimEvent {
    pub offender: String,
    pub crime_type: CrimeType,
    pub bounty: Option<u64>,
}

/// Evento `PayFines` del diario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PayFinesEvent {
    pub amount: u64,
    #[serde(rename = "BrokerPercentage")]
    pub broker_percentage: Option<f64>,
    #[serde(rename = "AllFines", default)]
    pub all_fines: bool,
    pub faction: Option<String>,
    #[serde(rename = "ShipID")]
    pub ship_id: Option<u64>,
}

/// Evento `PayBounties` o `PayBounty` del diario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PayBountiesEvent {
    pub amount: u64,
    #[serde(rename = "BrokerPercentage")]
    pub broker_percentage: Option<f64>,
    #[serde(rename = "AllBounties", default)]
    pub all_bounties: bool,
    pub faction: Option<String>,
    #[serde(rename = "ShipID")]
    pub ship_id: Option<u64>,
}

/// Resumen de multas y recompensas emitidas por una facción.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FactionFineBounty {
    pub faction: String,
    pub system: Option<String>,
    pub fines: u64,
    pub bounties: u64,
}

/// Mercancía robada o ilícita en la bodega de carga.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StolenCargoItem {
    pub name: String,
    pub name_localised: Option<String>,
    pub count: u32,
    pub stolen: u32,
    pub is_mission: bool,
}

/// Información de una estación o puerto cercano (Interstellar Factor o Black Market).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NearbyFacility {
    pub station_name: String,
    pub system_name: String,
    pub distance_ly: f64,
    pub pad_size: String,
    pub is_planetary: bool,
}

/// Nivel de seguridad del sistema estelar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SystemSecurityLevel {
    High,
    Medium,
    Low,
    Anarchy,
    #[default]
    Unknown,
}

impl SystemSecurityLevel {
    /// Tiempo estimado de respuesta policial en segundos según nivel de seguridad.
    pub fn estimated_police_response_seconds(&self) -> Option<u32> {
        match self {
            Self::High => Some(20),
            Self::Medium => Some(60),
            Self::Low => Some(180),
            Self::Anarchy | Self::Unknown => None,
        }
    }
}

impl From<&str> for SystemSecurityLevel {
    fn from(s: &str) -> Self {
        let lower = s.to_lowercase();
        if lower.contains("high") {
            Self::High
        } else if lower.contains("medium") {
            Self::Medium
        } else if lower.contains("low") {
            Self::Low
        } else if lower.contains("anarchy") {
            Self::Anarchy
        } else {
            Self::Unknown
        }
    }
}

/// Topic WebSocket / REST `crime.status`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrimeStatusTopic {
    /// Estado legal actual de la nave ("Clean", "IllegalCargo", "Speeding", "Wanted", "Hostile", "Warrant")
    pub legal_state: String,
    /// Nivel de notoriedad del comandante (0 a 10)
    pub notoriety: u8,
    /// Segundos estimados de vuelo necesarios para que la notoriedad decaiga a 0 (1 punto = 7.200s)
    pub notoriety_decay_seconds: u32,
    /// Suma total de multas activas en créditos
    pub total_fines: u64,
    /// Suma total de recompensas por cabeza en créditos
    pub total_bounties: u64,
    /// Desglose por facción
    pub factions: Vec<FactionFineBounty>,
    /// Mercancías robadas detectadas en la bodega
    pub stolen_items: Vec<StolenCargoItem>,
    /// Interstellar Factor más cercano (si notoriedad es 0)
    pub nearest_interstellar_factor: Option<NearbyFacility>,
    /// Mercado negro más cercano
    pub nearest_black_market: Option<NearbyFacility>,
    /// Clasificación de seguridad del sistema actual
    pub system_security: SystemSecurityLevel,
    /// Tiempo estimado de respuesta policial en segundos (None en Anarquía)
    pub police_response_estimate_seconds: Option<u32>,
}

impl Default for CrimeStatusTopic {
    fn default() -> Self {
        Self {
            legal_state: "Clean".to_string(),
            notoriety: 0,
            notoriety_decay_seconds: 0,
            total_fines: 0,
            total_bounties: 0,
            factions: Vec::new(),
            stolen_items: Vec::new(),
            nearest_interstellar_factor: None,
            nearest_black_market: None,
            system_security: SystemSecurityLevel::Unknown,
            police_response_estimate_seconds: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_crime_fine_deserializes() {
        let json = r#"{
            "timestamp": "2026-10-24T12:00:00Z",
            "event": "CommitCrime",
            "CrimeType": "reckless_weapons_discharge",
            "Faction": "Federation Security",
            "Fine": 500
        }"#;
        let ev: CommitCrimeEvent = serde_json::from_str(json).unwrap();
        assert_eq!(ev.crime_type, CrimeType::RecklessWeaponsDischarge);
        assert_eq!(ev.faction, "Federation Security");
        assert_eq!(ev.fine, Some(500));
        assert_eq!(ev.bounty, None);
        assert!(!ev.crime_type.increases_notoriety());
    }

    #[test]
    fn commit_crime_bounty_and_murder_notoriety() {
        let json = r#"{
            "timestamp": "2026-10-24T12:05:00Z",
            "event": "CommitCrime",
            "CrimeType": "murder",
            "Faction": "The Dark Wheel",
            "Victim": "CMDR James",
            "Bounty": 10000
        }"#;
        let ev: CommitCrimeEvent = serde_json::from_str(json).unwrap();
        assert_eq!(ev.crime_type, CrimeType::Murder);
        assert_eq!(ev.victim.as_deref(), Some("CMDR James"));
        assert_eq!(ev.bounty, Some(10000));
        assert!(ev.crime_type.increases_notoriety());
    }

    #[test]
    fn pay_fines_and_bounties_deserialize() {
        let json_fines = r#"{
            "timestamp": "2026-10-24T12:30:00Z",
            "event": "PayFines",
            "Amount": 1250,
            "BrokerPercentage": 25.0,
            "AllFines": true,
            "Faction": "Omega Syndicate",
            "ShipID": 4
        }"#;
        let pf: PayFinesEvent = serde_json::from_str(json_fines).unwrap();
        assert_eq!(pf.amount, 1250);
        assert_eq!(pf.broker_percentage, Some(25.0));
        assert!(pf.all_fines);
        assert_eq!(pf.faction.as_deref(), Some("Omega Syndicate"));

        let json_bounties = r#"{
            "timestamp": "2026-10-24T12:35:00Z",
            "event": "PayBounties",
            "Amount": 15000,
            "AllBounties": true
        }"#;
        let pb: PayBountiesEvent = serde_json::from_str(json_bounties).unwrap();
        assert_eq!(pb.amount, 15000);
        assert!(pb.all_bounties);
    }

    #[test]
    fn system_security_parsing_and_response_times() {
        assert_eq!(
            SystemSecurityLevel::from("$SYSTEM_SECURITY_high;"),
            SystemSecurityLevel::High
        );
        assert_eq!(
            SystemSecurityLevel::High.estimated_police_response_seconds(),
            Some(20)
        );
        assert_eq!(
            SystemSecurityLevel::from("$SYSTEM_SECURITY_anarchy;"),
            SystemSecurityLevel::Anarchy
        );
        assert_eq!(
            SystemSecurityLevel::Anarchy.estimated_police_response_seconds(),
            None
        );
    }

    #[test]
    fn crime_status_topic_roundtrip() {
        let topic = CrimeStatusTopic {
            legal_state: "Wanted".to_string(),
            notoriety: 2,
            notoriety_decay_seconds: 14400,
            total_bounties: 25000,
            factions: vec![FactionFineBounty {
                faction: "Federal Navy".to_string(),
                system: Some("Sol".to_string()),
                fines: 0,
                bounties: 25000,
            }],
            nearest_interstellar_factor: Some(NearbyFacility {
                station_name: "Chun Station".to_string(),
                system_name: "Epsilon Indi".to_string(),
                distance_ly: 8.4,
                pad_size: "L".to_string(),
                is_planetary: false,
            }),
            ..Default::default()
        };

        let json = serde_json::to_string(&topic).unwrap();
        let de: CrimeStatusTopic = serde_json::from_str(&json).unwrap();
        assert_eq!(de.legal_state, "Wanted");
        assert_eq!(de.notoriety, 2);
        assert_eq!(de.notoriety_decay_seconds, 14400);
        assert_eq!(
            de.nearest_interstellar_factor.unwrap().station_name,
            "Chun Station"
        );
    }
}
