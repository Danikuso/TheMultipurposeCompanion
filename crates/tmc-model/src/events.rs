//! Eventos tipados del diario de Elite Dangerous.

use crate::ParseError;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// Evento del diario ya envuelto: marca de tiempo, nombre, variante tipada y JSON original.
#[derive(Debug, Clone)]
pub struct JournalEvent {
    pub timestamp: DateTime<Utc>,
    pub name: String,
    pub kind: EventKind,
    pub raw: Box<RawValue>,
}

#[derive(Deserialize)]
struct Envelope {
    timestamp: Option<DateTime<Utc>>,
    event: Option<String>,
}

/// Datos de posición compartidos por `Location`, `FSDJump` y `CarrierJump`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct LocationInfo {
    pub star_system: String,
    pub system_address: u64,
    pub star_pos: [f64; 3],
    pub body: Option<String>,
    pub body_type: Option<String>,
    pub docked: bool,
    pub station_name: Option<String>,
    pub station_type: Option<String>,
    #[serde(rename = "MarketID")]
    pub market_id: Option<u64>,
    pub jump_dist: Option<f64>,
    pub fuel_used: Option<f64>,
    pub fuel_level: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fileheader {
    pub part: Option<u32>,
    pub language: Option<String>,
    pub gameversion: Option<String>,
    pub build: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Commander {
    #[serde(rename = "FID")]
    pub fid: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct LoadGame {
    #[serde(rename = "FID")]
    pub fid: Option<String>,
    pub commander: Option<String>,
    pub odyssey: Option<bool>,
    pub ship: Option<String>,
    #[serde(rename = "Ship_Localised")]
    pub ship_localised: Option<String>,
    #[serde(rename = "ShipID")]
    pub ship_id: Option<u64>,
    pub ship_name: Option<String>,
    pub ship_ident: Option<String>,
    pub fuel_level: Option<f64>,
    pub fuel_capacity: Option<f64>,
    pub game_mode: Option<String>,
    pub credits: Option<u64>,
    pub loan: Option<u64>,
    #[serde(rename = "gameversion")]
    pub gameversion: Option<String>,
    #[serde(rename = "build")]
    pub build: Option<String>,
}

/// Rangos (`Rank`) o porcentajes de progreso (`Progress`): mismos campos.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Rank {
    pub combat: Option<u32>,
    pub trade: Option<u32>,
    pub explore: Option<u32>,
    pub soldier: Option<u32>,
    pub exobiologist: Option<u32>,
    pub empire: Option<u32>,
    pub federation: Option<u32>,
    #[serde(rename = "CQC")]
    pub cqc: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Docked {
    pub station_name: Option<String>,
    pub station_type: Option<String>,
    #[serde(rename = "MarketID")]
    pub market_id: Option<u64>,
    pub star_system: Option<String>,
    pub system_address: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Undocked {
    pub station_name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct SupercruiseEntry {
    pub star_system: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct SupercruiseExit {
    pub star_system: Option<String>,
    pub body: Option<String>,
    pub body_type: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct BodyRef {
    pub body: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct StartJump {
    pub jump_type: Option<String>,
    pub star_system: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct FsdTarget {
    pub name: Option<String>,
    pub system_address: Option<u64>,
    pub remaining_jumps_in_route: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct FuelCapacity {
    pub main: Option<f64>,
    pub reserve: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct LoadoutModule {
    pub slot: Option<String>,
    pub item: Option<String>,
    pub health: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Loadout {
    pub ship: Option<String>,
    #[serde(rename = "ShipID")]
    pub ship_id: Option<u64>,
    pub ship_name: Option<String>,
    pub ship_ident: Option<String>,
    pub hull_health: Option<f64>,
    pub unladen_mass: Option<f64>,
    pub cargo_capacity: Option<u32>,
    pub max_jump_range: Option<f64>,
    pub fuel_capacity: Option<FuelCapacity>,
    pub rebuy: Option<u64>,
    pub modules: Vec<LoadoutModule>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Cargo {
    pub vessel: Option<String>,
    pub count: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct HullDamage {
    pub health: Option<f64>,
    pub player_pilot: Option<bool>,
    pub fighter: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct UnderAttack {
    pub target: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Interdicted {
    pub submitted: Option<bool>,
    pub interdictor: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct FuelScoop {
    pub scooped: Option<f64>,
    pub total: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Refuel {
    pub amount: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct ShipyardSwap {
    pub ship_type: Option<String>,
    #[serde(rename = "ShipID")]
    pub ship_id: Option<u64>,
}

/// Variante tipada de un evento. Lo desconocido o malformado cae en `Other`.
#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
    Fileheader(Fileheader),
    Commander(Commander),
    LoadGame(LoadGame),
    Rank(Rank),
    Progress(Rank),
    Location(LocationInfo),
    FsdJump(LocationInfo),
    CarrierJump(LocationInfo),
    Docked(Docked),
    Undocked(Undocked),
    SupercruiseEntry(SupercruiseEntry),
    SupercruiseExit(SupercruiseExit),
    ApproachBody(BodyRef),
    LeaveBody(BodyRef),
    Touchdown,
    Liftoff,
    StartJump(StartJump),
    FsdTarget(FsdTarget),
    NavRoute,
    NavRouteClear,
    Loadout(Loadout),
    Cargo(Cargo),
    HullDamage(HullDamage),
    UnderAttack(UnderAttack),
    Interdicted(Interdicted),
    FuelScoop(FuelScoop),
    RefuelAll(Refuel),
    RefuelPartial(Refuel),
    ShipyardSwap(ShipyardSwap),
    Died,
    Resurrect,
    Shutdown,
    Market,
    Outfitting,
    Shipyard,
    ModuleInfo,
    Backpack,
    ShipLocker,
    FcMaterials,
    Other,
}

fn de<T: DeserializeOwned>(name: &str, raw: &RawValue) -> Option<T> {
    match serde_json::from_str(raw.get()) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::debug!(event = name, error = %e, "evento tipado no deserializable, se usa Other");
            None
        }
    }
}

fn classify(name: &str, raw: &RawValue) -> EventKind {
    let kind = match name {
        "Fileheader" => de(name, raw).map(EventKind::Fileheader),
        "Commander" => de(name, raw).map(EventKind::Commander),
        "LoadGame" => de(name, raw).map(EventKind::LoadGame),
        "Rank" => de(name, raw).map(EventKind::Rank),
        "Progress" => de(name, raw).map(EventKind::Progress),
        "Location" => de(name, raw).map(EventKind::Location),
        "FSDJump" => de(name, raw).map(EventKind::FsdJump),
        "CarrierJump" => de(name, raw).map(EventKind::CarrierJump),
        "Docked" => de(name, raw).map(EventKind::Docked),
        "Undocked" => de(name, raw).map(EventKind::Undocked),
        "SupercruiseEntry" => de(name, raw).map(EventKind::SupercruiseEntry),
        "SupercruiseExit" => de(name, raw).map(EventKind::SupercruiseExit),
        "ApproachBody" => de(name, raw).map(EventKind::ApproachBody),
        "LeaveBody" => de(name, raw).map(EventKind::LeaveBody),
        "Touchdown" => Some(EventKind::Touchdown),
        "Liftoff" => Some(EventKind::Liftoff),
        "StartJump" => de(name, raw).map(EventKind::StartJump),
        "FSDTarget" => de(name, raw).map(EventKind::FsdTarget),
        "NavRoute" => Some(EventKind::NavRoute),
        "NavRouteClear" => Some(EventKind::NavRouteClear),
        "Loadout" => de(name, raw).map(EventKind::Loadout),
        "Cargo" => de(name, raw).map(EventKind::Cargo),
        "HullDamage" => de(name, raw).map(EventKind::HullDamage),
        "UnderAttack" => de(name, raw).map(EventKind::UnderAttack),
        "Interdicted" => de(name, raw).map(EventKind::Interdicted),
        "FuelScoop" => de(name, raw).map(EventKind::FuelScoop),
        "RefuelAll" => de(name, raw).map(EventKind::RefuelAll),
        "RefuelPartial" => de(name, raw).map(EventKind::RefuelPartial),
        "ShipyardSwap" => de(name, raw).map(EventKind::ShipyardSwap),
        "Died" => Some(EventKind::Died),
        "Resurrect" => Some(EventKind::Resurrect),
        "Shutdown" => Some(EventKind::Shutdown),
        "Market" => Some(EventKind::Market),
        "Outfitting" => Some(EventKind::Outfitting),
        "Shipyard" => Some(EventKind::Shipyard),
        "ModuleInfo" => Some(EventKind::ModuleInfo),
        "Backpack" => Some(EventKind::Backpack),
        "ShipLocker" => Some(EventKind::ShipLocker),
        "FCMaterials" => Some(EventKind::FcMaterials),
        _ => None,
    };
    kind.unwrap_or(EventKind::Other)
}

/// Analiza una línea del diario (acepta `\r`/`\n` finales).
pub fn parse_line(line: &str) -> Result<JournalEvent, ParseError> {
    let line = line.trim_end_matches(['\r', '\n']);
    if line.trim().is_empty() {
        return Err(ParseError::Empty);
    }
    let env: Envelope = serde_json::from_str(line).map_err(|e| ParseError::Json(e.to_string()))?;
    let raw: Box<RawValue> =
        serde_json::from_str(line).map_err(|e| ParseError::Json(e.to_string()))?;
    let timestamp = env.timestamp.ok_or(ParseError::MissingField("timestamp"))?;
    let name = env.event.ok_or(ParseError::MissingField("event"))?;
    let kind = classify(&name, &raw);
    Ok(JournalEvent {
        timestamp,
        name,
        kind,
        raw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ParseError;

    const FSD: &str = r#"{ "timestamp":"2026-03-05T18:03:22Z", "event":"FSDJump", "StarSystem":"Exampla", "SystemAddress":5068732442345, "StarPos":[14.0,-2.5,40.75], "JumpDist":9.873, "FuelUsed":1.2, "FuelLevel":18.7 }"#;

    #[test]
    fn parses_fsdjump_into_location_info() {
        let ev = parse_line(FSD).unwrap();
        assert_eq!(ev.name, "FSDJump");
        match ev.kind {
            EventKind::FsdJump(l) => {
                assert_eq!(l.star_system, "Exampla");
                assert_eq!(l.system_address, 5068732442345);
                assert_eq!(l.star_pos, [14.0, -2.5, 40.75]);
                assert_eq!(l.jump_dist, Some(9.873));
            }
            other => panic!("inesperado: {other:?}"),
        }
    }

    #[test]
    fn unknown_event_is_other_not_error() {
        let ev = parse_line(r#"{"timestamp":"2026-03-05T18:00:00Z","event":"FutureThing","X":1}"#)
            .unwrap();
        assert_eq!(ev.kind, EventKind::Other);
        assert_eq!(ev.name, "FutureThing");
    }

    #[test]
    fn malformed_typed_event_falls_back_to_other() {
        let ev = parse_line(r#"{"timestamp":"2026-03-05T18:00:00Z","event":"Rank","Combat":"x"}"#)
            .unwrap();
        assert_eq!(ev.kind, EventKind::Other);
    }

    #[test]
    fn crlf_line_is_accepted() {
        let line = format!("{FSD}\r\n");
        assert!(parse_line(&line).is_ok());
    }

    #[test]
    fn empty_line_is_empty_error() {
        assert_eq!(parse_line("\r\n").unwrap_err(), ParseError::Empty);
    }

    #[test]
    fn missing_event_field_is_error() {
        let r = parse_line(r#"{"timestamp":"2026-03-05T18:00:00Z"}"#);
        assert_eq!(r.unwrap_err(), ParseError::MissingField("event"));
    }

    #[test]
    fn missing_timestamp_is_error() {
        let r = parse_line(r#"{"event":"Died"}"#);
        assert_eq!(r.unwrap_err(), ParseError::MissingField("timestamp"));
    }

    #[test]
    fn raw_json_is_preserved_verbatim() {
        let ev = parse_line(&format!("{FSD}\r")).unwrap();
        assert_eq!(ev.raw.get(), FSD);
    }

    #[test]
    fn every_line_of_basic_session_fixture_parses() {
        let text = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/journal/basic_session.log"
        ));
        let mut n = 0;
        for line in text.lines() {
            let ev = parse_line(line).unwrap_or_else(|e| panic!("{line}: {e}"));
            assert_ne!(
                ev.kind,
                EventKind::Other,
                "{} debería estar tipado",
                ev.name
            );
            n += 1;
        }
        assert!(n >= 20);
    }
}
