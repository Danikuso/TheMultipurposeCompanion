//! Eventos tipados del diario de Elite Dangerous.

use crate::ParseError;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::BTreeMap;

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

/// Elemento `{ Name, Percent }` (materiales, composición atmosférica).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Composition {
    pub name: String,
    pub percent: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Scan {
    pub scan_type: Option<String>,
    pub body_name: String,
    #[serde(rename = "BodyID")]
    pub body_id: Option<u32>,
    pub parents: Vec<BTreeMap<String, u32>>,
    pub star_system: Option<String>,
    pub system_address: Option<u64>,
    #[serde(rename = "DistanceFromArrivalLS")]
    pub distance_from_arrival_ls: Option<f64>,
    pub star_type: Option<String>,
    pub subclass: Option<u32>,
    pub stellar_mass: Option<f64>,
    pub radius: Option<f64>,
    pub planet_class: Option<String>,
    pub terraform_state: Option<String>,
    pub atmosphere: Option<String>,
    pub atmosphere_type: Option<String>,
    pub atmosphere_composition: Vec<Composition>,
    pub volcanism: Option<String>,
    #[serde(rename = "MassEM")]
    pub mass_em: Option<f64>,
    pub surface_gravity: Option<f64>,
    pub surface_temperature: Option<f64>,
    pub surface_pressure: Option<f64>,
    pub landable: Option<bool>,
    pub materials: Vec<Composition>,
    pub was_discovered: Option<bool>,
    pub was_mapped: Option<bool>,
    pub was_footfalled: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct FssDiscoveryScan {
    pub progress: Option<f64>,
    pub body_count: Option<u32>,
    pub non_body_count: Option<u32>,
    pub system_name: Option<String>,
    pub system_address: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct FssAllBodiesFound {
    pub system_name: Option<String>,
    pub system_address: Option<u64>,
    pub count: Option<u32>,
}

const SIGNAL_BIOLOGICAL: &str = "$SAA_SignalType_Biological;";
const SIGNAL_GEOLOGICAL: &str = "$SAA_SignalType_Geological;";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Signal {
    #[serde(rename = "Type")]
    pub type_: String,
    #[serde(rename = "Type_Localised")]
    pub type_localised: Option<String>,
    pub count: u32,
}

impl Signal {
    pub fn is_biological(&self) -> bool {
        self.type_ == SIGNAL_BIOLOGICAL
    }

    pub fn is_geological(&self) -> bool {
        self.type_ == SIGNAL_GEOLOGICAL
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct GenusRef {
    pub genus: String,
    #[serde(rename = "Genus_Localised")]
    pub genus_localised: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct FssBodySignals {
    pub body_name: String,
    #[serde(rename = "BodyID")]
    pub body_id: Option<u32>,
    pub system_address: Option<u64>,
    pub signals: Vec<Signal>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct SaaScanComplete {
    pub body_name: String,
    #[serde(rename = "BodyID")]
    pub body_id: Option<u32>,
    pub system_address: Option<u64>,
    pub probes_used: Option<u32>,
    pub efficiency_target: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct SaaSignalsFound {
    pub body_name: String,
    #[serde(rename = "BodyID")]
    pub body_id: Option<u32>,
    pub system_address: Option<u64>,
    pub signals: Vec<Signal>,
    pub genuses: Vec<GenusRef>,
}

/// Fase de un `ScanOrganic`; cualquier valor desconocido es `Unknown`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganicScanType {
    Log,
    Sample,
    Analyse,
    #[default]
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct ScanOrganic {
    pub scan_type: OrganicScanType,
    pub genus: Option<String>,
    #[serde(rename = "Genus_Localised")]
    pub genus_localised: Option<String>,
    pub species: Option<String>,
    #[serde(rename = "Species_Localised")]
    pub species_localised: Option<String>,
    pub variant: Option<String>,
    #[serde(rename = "Variant_Localised")]
    pub variant_localised: Option<String>,
    pub system_address: Option<u64>,
    pub body: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct BioSale {
    pub genus: Option<String>,
    pub species: Option<String>,
    pub variant: Option<String>,
    pub value: u64,
    pub bonus: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct SellOrganicData {
    #[serde(rename = "MarketID")]
    pub market_id: Option<u64>,
    pub bio_data: Vec<BioSale>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct SellExplorationData {
    pub systems: Vec<String>,
    pub discovered: Vec<String>,
    pub base_value: Option<u64>,
    pub bonus: Option<u64>,
    pub total_earnings: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct SoldSystem {
    pub system_name: String,
    pub num_bodies: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct MultiSellExplorationData {
    pub discovered: Vec<SoldSystem>,
    pub base_value: Option<u64>,
    pub bonus: Option<u64>,
    pub total_earnings: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CodexEntry {
    #[serde(rename = "EntryID")]
    pub entry_id: Option<u64>,
    pub name: Option<String>,
    #[serde(rename = "Name_Localised")]
    pub name_localised: Option<String>,
    pub category: Option<String>,
    pub sub_category: Option<String>,
    pub system_address: Option<u64>,
    #[serde(rename = "BodyID")]
    pub body_id: Option<u32>,
    pub is_new_entry: Option<bool>,
    pub voucher_amount: Option<u64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct JetConeBoost {
    pub boost_value: Option<f64>,
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
    Scan(Box<Scan>),
    FssDiscoveryScan(FssDiscoveryScan),
    FssAllBodiesFound(FssAllBodiesFound),
    FssBodySignals(FssBodySignals),
    SaaScanComplete(SaaScanComplete),
    SaaSignalsFound(SaaSignalsFound),
    ScanOrganic(ScanOrganic),
    SellOrganicData(SellOrganicData),
    SellExplorationData(SellExplorationData),
    MultiSellExplorationData(MultiSellExplorationData),
    CodexEntry(CodexEntry),
    JetConeBoost(JetConeBoost),
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
        "Scan" => de::<Scan>(name, raw).map(|s| EventKind::Scan(Box::new(s))),
        "FSSDiscoveryScan" => de(name, raw).map(EventKind::FssDiscoveryScan),
        "FSSAllBodiesFound" => de(name, raw).map(EventKind::FssAllBodiesFound),
        "FSSBodySignals" => de(name, raw).map(EventKind::FssBodySignals),
        "SAAScanComplete" => de(name, raw).map(EventKind::SaaScanComplete),
        "SAASignalsFound" => de(name, raw).map(EventKind::SaaSignalsFound),
        "ScanOrganic" => de(name, raw).map(EventKind::ScanOrganic),
        "SellOrganicData" => de(name, raw).map(EventKind::SellOrganicData),
        "SellExplorationData" => de(name, raw).map(EventKind::SellExplorationData),
        "MultiSellExplorationData" => de(name, raw).map(EventKind::MultiSellExplorationData),
        "CodexEntry" => de(name, raw).map(EventKind::CodexEntry),
        "JetConeBoost" => de(name, raw).map(EventKind::JetConeBoost),
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

    fn ev(json: &str) -> EventKind {
        parse_line(json).unwrap().kind
    }

    #[test]
    fn parses_detailed_scan_of_landable_planet() {
        let k = ev(
            r#"{"timestamp":"2026-04-10T20:07:00Z","event":"Scan","ScanType":"Detailed","BodyName":"Kappa Testis A 2","BodyID":7,"Parents":[{"Star":0},{"Null":1}],"StarSystem":"Kappa Testis","SystemAddress":3932277478106,"DistanceFromArrivalLS":455.9,"PlanetClass":"High metal content body","MassEM":0.12,"SurfaceGravity":2.1,"Landable":true,"Materials":[{"Name":"iron","Percent":19.8}],"WasDiscovered":false,"WasMapped":false,"WasFootfalled":false}"#,
        );
        let EventKind::Scan(s) = k else {
            panic!("inesperado: {k:?}")
        };
        assert_eq!(s.scan_type.as_deref(), Some("Detailed"));
        assert_eq!(s.body_name, "Kappa Testis A 2");
        assert_eq!(s.body_id, Some(7));
        assert_eq!(s.parents.len(), 2);
        assert_eq!(s.parents[0].get("Star"), Some(&0));
        assert_eq!(s.system_address, Some(3932277478106));
        assert_eq!(s.distance_from_arrival_ls, Some(455.9));
        assert_eq!(s.planet_class.as_deref(), Some("High metal content body"));
        assert_eq!(s.mass_em, Some(0.12));
        assert_eq!(s.surface_gravity, Some(2.1));
        assert_eq!(s.landable, Some(true));
        assert_eq!(s.materials[0].name, "iron");
        assert_eq!(s.materials[0].percent, 19.8);
        assert_eq!(s.was_discovered, Some(false));
        assert_eq!(s.was_mapped, Some(false));
        assert_eq!(s.was_footfalled, Some(false));
    }

    #[test]
    fn parses_star_scan() {
        let k = ev(
            r#"{"timestamp":"2026-04-10T20:06:00Z","event":"Scan","ScanType":"AutoScan","BodyName":"Pulsar X","BodyID":0,"StarType":"N","StellarMass":1.4}"#,
        );
        let EventKind::Scan(s) = k else {
            panic!("inesperado: {k:?}")
        };
        assert_eq!(s.star_type.as_deref(), Some("N"));
        assert_eq!(s.stellar_mass, Some(1.4));
        assert_eq!(s.planet_class, None);
        assert_eq!(s.landable, None);
    }

    #[test]
    fn fss_body_signals_detects_biological_count() {
        let k = ev(
            r#"{"timestamp":"2026-04-10T20:08:30Z","event":"FSSBodySignals","BodyName":"B 3","BodyID":7,"SystemAddress":1,"Signals":[{"Type":"$SAA_SignalType_Biological;","Type_Localised":"Biological","Count":3}]}"#,
        );
        let EventKind::FssBodySignals(f) = k else {
            panic!("inesperado: {k:?}")
        };
        assert_eq!(f.body_id, Some(7));
        assert_eq!(f.signals.len(), 1);
        assert!(f.signals[0].is_biological());
        assert!(!f.signals[0].is_geological());
        assert_eq!(f.signals[0].count, 3);
    }

    #[test]
    fn saa_signals_found_lists_genuses() {
        let k = ev(
            r#"{"timestamp":"2026-04-10T20:12:31Z","event":"SAASignalsFound","BodyName":"B 3","BodyID":7,"SystemAddress":1,"Signals":[{"Type":"$SAA_SignalType_Geological;","Count":1}],"Genuses":[{"Genus":"$Codex_Ent_Bacterial_Genus_Name;","Genus_Localised":"Bacterium"}]}"#,
        );
        let EventKind::SaaSignalsFound(f) = k else {
            panic!("inesperado: {k:?}")
        };
        assert_eq!(f.genuses.len(), 1);
        assert_eq!(f.genuses[0].genus, "$Codex_Ent_Bacterial_Genus_Name;");
        assert_eq!(f.genuses[0].genus_localised.as_deref(), Some("Bacterium"));
        assert!(f.signals[0].is_geological());
    }

    #[test]
    fn saa_scan_complete_parses() {
        let k = ev(
            r#"{"timestamp":"2026-04-10T20:12:30Z","event":"SAAScanComplete","BodyName":"B 3","BodyID":7,"SystemAddress":1,"ProbesUsed":5,"EfficiencyTarget":6}"#,
        );
        let EventKind::SaaScanComplete(s) = k else {
            panic!("inesperado: {k:?}")
        };
        assert_eq!(s.probes_used, Some(5));
        assert_eq!(s.efficiency_target, Some(6));
    }

    #[test]
    fn scan_organic_types_and_unknown() {
        let mk = |t: &str| {
            ev(&format!(
                r#"{{"timestamp":"2026-04-10T20:20:10Z","event":"ScanOrganic","ScanType":"{t}","Genus":"$G;","Species":"$S;","Variant":"$V;","SystemAddress":9,"Body":7}}"#
            ))
        };
        let EventKind::ScanOrganic(a) = mk("Log") else {
            panic!("Log")
        };
        assert_eq!(a.scan_type, OrganicScanType::Log);
        assert_eq!(a.body, Some(7));
        assert_eq!(a.genus.as_deref(), Some("$G;"));
        let EventKind::ScanOrganic(b) = mk("Analyse") else {
            panic!("Analyse")
        };
        assert_eq!(b.scan_type, OrganicScanType::Analyse);
        let EventKind::ScanOrganic(c) = mk("Sample") else {
            panic!("Sample")
        };
        assert_eq!(c.scan_type, OrganicScanType::Sample);
        let EventKind::ScanOrganic(d) = mk("Foo") else {
            panic!("Foo")
        };
        assert_eq!(d.scan_type, OrganicScanType::Unknown);
    }

    #[test]
    fn sell_organic_data_values_and_bonus() {
        let k = ev(
            r#"{"timestamp":"2026-04-10T20:45:30Z","event":"SellOrganicData","MarketID":3700000001,"BioData":[{"Genus":"$G;","Species":"$S;","Variant":"$V;","Value":1000000,"Bonus":4000000}]}"#,
        );
        let EventKind::SellOrganicData(s) = k else {
            panic!("inesperado: {k:?}")
        };
        assert_eq!(s.market_id, Some(3700000001));
        assert_eq!(s.bio_data[0].value, 1_000_000);
        assert_eq!(s.bio_data[0].bonus, 4_000_000);
    }

    #[test]
    fn sell_exploration_data_parses() {
        let k = ev(
            r#"{"timestamp":"2026-04-10T20:45:10Z","event":"SellExplorationData","Systems":["A"],"Discovered":["A"],"BaseValue":85000,"Bonus":12000,"TotalEarnings":97000}"#,
        );
        let EventKind::SellExplorationData(s) = k else {
            panic!("inesperado: {k:?}")
        };
        assert_eq!(s.systems, vec!["A".to_string()]);
        assert_eq!(s.total_earnings, Some(97000));
    }

    #[test]
    fn multi_sell_exploration_data_parses() {
        let k = ev(
            r#"{"timestamp":"2026-04-10T20:45:20Z","event":"MultiSellExplorationData","Discovered":[{"SystemName":"Sys A","NumBodies":7},{"SystemName":"Sys B","NumBodies":3}],"BaseValue":100000,"Bonus":20000,"TotalEarnings":120000}"#,
        );
        let EventKind::MultiSellExplorationData(m) = k else {
            panic!("inesperado: {k:?}")
        };
        assert_eq!(m.discovered.len(), 2);
        assert_eq!(m.discovered[0].system_name, "Sys A");
        assert_eq!(m.discovered[0].num_bodies, 7);
        assert_eq!(m.base_value, Some(100000));
        assert_eq!(m.bonus, Some(20000));
        assert_eq!(m.total_earnings, Some(120000));
    }

    #[test]
    fn codex_entry_and_jet_cone_parse() {
        let EventKind::CodexEntry(c) = ev(
            r#"{"timestamp":"2026-04-10T20:20:00Z","event":"CodexEntry","EntryID":2100101,"Name":"$N;","Name_Localised":"Bacterium Aurasus","Category":"$Codex_Category_Biology;","SubCategory":"$Sub;","SystemAddress":5,"BodyID":7,"IsNewEntry":true,"VoucherAmount":50000,"Latitude":12.6,"Longitude":-45.3}"#,
        ) else {
            panic!("CodexEntry")
        };
        assert_eq!(c.entry_id, Some(2100101));
        assert_eq!(c.name_localised.as_deref(), Some("Bacterium Aurasus"));
        assert_eq!(c.is_new_entry, Some(true));
        assert_eq!(c.voucher_amount, Some(50000));
        assert_eq!(c.latitude, Some(12.6));
        let EventKind::JetConeBoost(j) =
            ev(r#"{"timestamp":"2026-04-10T20:05:00Z","event":"JetConeBoost","BoostValue":1.5}"#)
        else {
            panic!("JetConeBoost")
        };
        assert_eq!(j.boost_value, Some(1.5));
    }

    #[test]
    fn fss_discovery_and_all_bodies_found_parse() {
        let EventKind::FssDiscoveryScan(d) = ev(
            r#"{"timestamp":"2026-04-10T20:05:40Z","event":"FSSDiscoveryScan","Progress":0.25,"BodyCount":6,"NonBodyCount":3,"SystemName":"K","SystemAddress":8}"#,
        ) else {
            panic!("FSSDiscoveryScan")
        };
        assert_eq!(d.body_count, Some(6));
        assert_eq!(d.non_body_count, Some(3));
        assert_eq!(d.progress, Some(0.25));
        let EventKind::FssAllBodiesFound(a) = ev(
            r#"{"timestamp":"2026-04-10T20:09:00Z","event":"FSSAllBodiesFound","SystemName":"K","SystemAddress":8,"Count":6}"#,
        ) else {
            panic!("FSSAllBodiesFound")
        };
        assert_eq!(a.count, Some(6));
        assert_eq!(a.system_address, Some(8));
    }

    #[test]
    fn every_line_of_exploration_session_fixture_is_typed() {
        const EXPECTED_OTHER: [&str; 2] = ["Music", "ReceiveText"];
        let text = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/journal/exploration_session.log"
        ));
        let mut n = 0;
        let mut others = std::collections::BTreeSet::new();
        for line in text.lines() {
            let ev = parse_line(line).unwrap_or_else(|e| panic!("{line}: {e}"));
            if ev.kind == EventKind::Other {
                others.insert(ev.name.clone());
            }
            n += 1;
        }
        assert!(n >= 30, "líneas: {n}");
        let expected: std::collections::BTreeSet<String> =
            EXPECTED_OTHER.iter().map(|s| s.to_string()).collect();
        assert_eq!(others, expected);
    }
}
