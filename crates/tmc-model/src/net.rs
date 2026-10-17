//! Tipos de consulta de red compartidos entre módulos, núcleo y capa de red.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Consulta de red emitida por un módulo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NetQuery {
    System {
        id64: u64,
        name: String,
    },
    NeutronRoute(NeutronRouteParams),
    GalaxyRoute(GalaxyRouteParams),
    Autocomplete {
        prefix: String,
    },
    BodyBio {
        system_address: u64,
        body_id: u32,
        body_name: String,
        profile: BodyProfile,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NeutronRouteParams {
    pub from: String,
    pub to: String,
    pub range_ly: f64,
    pub efficiency: u8,
    pub supercharge_multiplier: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GalaxyRouteParams {
    pub from: String,
    pub to: String,
    pub ship: ShipParams,
    pub refuel_scoopable: bool,
    pub use_injections: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShipParams {
    pub unladen_mass: f64,
    pub fuel_main_capacity: f64,
    pub fuel_reserve: f64,
    pub cargo: f64,
    pub optimal_mass: f64,
    pub max_fuel_per_jump: f64,
    pub fuel_mul: f64,
    pub fuel_power: f64,
    pub guardian_bonus_ly: f64,
    pub supercharge_multiplier: f64,
}

/// Petición con clave de relevancia (`system:<id64>`, `route`, `bio:<addr>:<body>`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetRequest {
    pub id: u64,
    pub module: String,
    pub key: String,
    pub query: NetQuery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetSource {
    Spansh,
    Edsm,
    Canonn,
    Cache,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetResponse {
    pub id: u64,
    pub module: String,
    pub key: String,
    pub source: NetSource,
    pub fetched_at: DateTime<Utc>,
    pub outcome: NetOutcome,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum NetOutcome {
    System(RemoteSystem),
    Route(PlannedRoute),
    Autocomplete(Vec<String>),
    BodyBio(RemoteBodyBio),
    NoData,
    Disabled,
    Failed(NetFailure),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetFailure {
    Timeout,
    Http(u16),
    Malformed,
    Down,
    RateLimited,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteSystem {
    pub id64: u64,
    pub name: String,
    pub discovered: Option<bool>,
    pub body_count: Option<u32>,
    pub primary_star: Option<String>,
    pub allegiance: Option<String>,
    pub government: Option<String>,
    pub controlling_faction: Option<String>,
    pub faction_state: Option<String>,
    pub economy: Option<String>,
    pub second_economy: Option<String>,
    pub security: Option<String>,
    pub population: Option<u64>,
    pub powers: Vec<String>,
    pub powerplay_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedRoute {
    pub hops: Vec<PlannedHop>,
    pub total_ly: f64,
}

/// Tramo de ruta. En rutas de neutrones un tramo puede agrupar varios saltos
/// (`jumps`) y `distance_ly` es la distancia del tramo; `refuel` es `None`
/// cuando la fuente no lo informa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedHop {
    pub name: String,
    pub id64: Option<u64>,
    pub star_pos: Option<[f64; 3]>,
    pub distance_ly: f64,
    pub jumps: u32,
    pub neutron: bool,
    pub refuel: Option<bool>,
    pub star_class: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyProfile {
    pub planet_class: Option<String>,
    pub atmosphere_type: Option<String>,
    pub gravity_g: Option<f64>,
    pub temperature_k: Option<f64>,
    pub pressure_atm: Option<f64>,
    pub volcanism: Option<String>,
    pub star_class: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteBodyBio {
    pub hints: Vec<RemoteSpeciesHint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteSpeciesHint {
    pub genus: String,
    pub species: String,
    pub weight: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sol() -> RemoteSystem {
        RemoteSystem {
            id64: 1,
            name: "Sol".into(),
            discovered: None,
            body_count: Some(40),
            primary_star: Some("G".into()),
            allegiance: None,
            government: None,
            controlling_faction: None,
            faction_state: None,
            economy: None,
            second_economy: None,
            security: None,
            population: Some(22_780_919_531),
            powers: vec!["Jerome Archer".into()],
            powerplay_state: None,
        }
    }

    fn response(outcome: NetOutcome) -> NetResponse {
        NetResponse {
            id: 3,
            module: "navigation".into(),
            key: "system:1".into(),
            source: NetSource::Spansh,
            fetched_at: "2026-10-17T12:00:00Z".parse().unwrap(),
            outcome,
        }
    }

    #[test]
    fn net_response_roundtrips_json() {
        for x in [
            response(NetOutcome::System(sol())),
            response(NetOutcome::Failed(NetFailure::Http(503))),
            response(NetOutcome::Route(PlannedRoute {
                hops: vec![PlannedHop {
                    name: "Colonia".into(),
                    id64: Some(7),
                    star_pos: Some([1.0, 2.0, 3.0]),
                    distance_ly: 12.5,
                    jumps: 2,
                    neutron: true,
                    refuel: None,
                    star_class: Some("N".into()),
                }],
                total_ly: 12.5,
            })),
        ] {
            let text = serde_json::to_string(&x).unwrap();
            let back: NetResponse = serde_json::from_str(&text).unwrap();
            assert_eq!(back, x);
        }
        assert_eq!(response(NetOutcome::NoData).id, 3);
    }

    #[test]
    fn net_failure_http_serializes_stably() {
        let text = serde_json::to_string(&NetFailure::Http(503)).unwrap();
        assert_eq!(text, r#"{"Http":503}"#);
        assert_eq!(
            serde_json::from_str::<NetFailure>(&text).unwrap(),
            NetFailure::Http(503)
        );
    }
}
