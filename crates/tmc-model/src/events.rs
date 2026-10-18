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
    #[serde(rename = "BodyID")]
    pub body_id: Option<u32>,
    pub system_allegiance: Option<String>,
    #[serde(rename = "SystemGovernment_Localised")]
    pub system_government_localised: Option<String>,
    #[serde(rename = "SystemEconomy_Localised")]
    pub system_economy_localised: Option<String>,
    #[serde(rename = "SystemSecondEconomy_Localised")]
    pub system_second_economy_localised: Option<String>,
    #[serde(rename = "SystemSecurity_Localised")]
    pub system_security_localised: Option<String>,
    pub population: Option<u64>,
    pub system_faction: Option<SystemFaction>,
    pub powers: Vec<String>,
    pub powerplay_state: Option<String>,
    pub boost_used: Option<f64>,
}

/// Facción controladora del sistema (`SystemFaction` de `FSDJump`/`Location`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct SystemFaction {
    pub name: Option<String>,
    pub faction_state: Option<String>,
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
    pub system_address: Option<u64>,
    #[serde(rename = "BodyID")]
    pub body_id: Option<u32>,
}

/// Datos de `Touchdown` y `Liftoff`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct SurfaceEvent {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub body: Option<String>,
    #[serde(rename = "BodyID")]
    pub body_id: Option<u32>,
    pub system_address: Option<u64>,
    pub on_planet: Option<bool>,
    pub player_controlled: Option<bool>,
}

/// Datos de `Embark` y `Disembark`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct EmbarkEvent {
    #[serde(rename = "SRV")]
    pub srv: bool,
    pub taxi: bool,
    pub multicrew: bool,
    pub on_planet: bool,
    pub on_station: bool,
    pub body: Option<String>,
    #[serde(rename = "BodyID")]
    pub body_id: Option<u32>,
    pub system_address: Option<u64>,
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
    pub engineering: Option<Engineering>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Engineering {
    pub blueprint_name: Option<String>,
    pub level: Option<u32>,
    pub quality: Option<f64>,
    pub experimental_effect: Option<String>,
    pub modifiers: Vec<Modifier>,
}

/// Modificador de ingeniería. `value` es `None` si el diario trae texto (`ValueStr`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Modifier {
    pub label: String,
    #[serde(deserialize_with = "lenient_f64")]
    pub value: Option<f64>,
    #[serde(deserialize_with = "lenient_f64")]
    pub original_value: Option<f64>,
    pub less_is_good: Option<u8>,
}

/// Acepta un número; cualquier otro tipo JSON da `None` sin error.
fn lenient_f64<'de, D>(d: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<serde_json::Value>::deserialize(d)?.and_then(|v| v.as_f64()))
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
    pub inventory: Vec<CargoItem>,
}

/// Entrada del inventario del evento `Cargo` (distinta de `companion::CargoItem`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CargoItem {
    pub name: String,
    #[serde(rename = "Name_Localised")]
    pub name_localised: Option<String>,
    pub count: u32,
    pub stolen: Option<u32>,
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct ShipTargeted {
    pub target_locked: bool,
    pub ship: Option<String>,
    #[serde(rename = "Ship_Localised")]
    pub ship_localised: Option<String>,
    pub scan_stage: Option<u8>,
    pub pilot_name: Option<String>,
    #[serde(rename = "PilotName_Localised")]
    pub pilot_name_localised: Option<String>,
    pub pilot_rank: Option<String>,
    pub shield_health: Option<f64>,
    pub hull_health: Option<f64>,
    pub faction: Option<String>,
    pub legal_status: Option<String>,
    pub bounty: Option<u64>,
    pub subsystem: Option<String>,
    #[serde(rename = "Subsystem_Localised")]
    pub subsystem_localised: Option<String>,
    pub subsystem_health: Option<f64>,
    pub power: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct BountyReward {
    pub faction: String,
    pub reward: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Bounty {
    pub rewards: Vec<BountyReward>,
    pub total_reward: Option<u64>,
    pub reward: Option<u64>,
    pub faction: Option<String>,
    pub victim_faction: Option<String>,
    #[serde(rename = "VictimFaction_Localised")]
    pub victim_faction_localised: Option<String>,
    pub target: Option<String>,
    #[serde(rename = "Target_Localised")]
    pub target_localised: Option<String>,
    pub shared_with_others: Option<u32>,
}

impl Bounty {
    /// Unifica el formato con `Rewards` y el antiguo (`Reward` + `Faction`, patines).
    pub fn normalized_rewards(&self) -> Vec<BountyReward> {
        if !self.rewards.is_empty() {
            return self.rewards.clone();
        }
        match (self.reward, &self.faction) {
            (Some(reward), Some(faction)) => vec![BountyReward {
                faction: faction.clone(),
                reward,
            }],
            _ => Vec::new(),
        }
    }
}

/// Datos de `FactionKillBond` y `CapShipBond`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct FactionKillBond {
    pub reward: u64,
    pub awarding_faction: Option<String>,
    #[serde(rename = "AwardingFaction_Localised")]
    pub awarding_faction_localised: Option<String>,
    pub victim_faction: Option<String>,
    #[serde(rename = "VictimFaction_Localised")]
    pub victim_faction_localised: Option<String>,
}

pub type CapShipBond = FactionKillBond;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct VoucherFaction {
    pub faction: String,
    pub amount: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct RedeemVoucher {
    #[serde(rename = "Type")]
    pub type_: String,
    pub amount: u64,
    pub faction: Option<String>,
    pub factions: Vec<VoucherFaction>,
    pub broker_percentage: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct ShieldState {
    pub shields_up: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct EscapeInterdiction {
    pub interdictor: Option<String>,
    pub is_player: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct PvpKill {
    pub victim: Option<String>,
    pub combat_rank: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Promotion {
    pub combat: Option<u8>,
    pub trade: Option<u8>,
    pub explore: Option<u8>,
    pub cqc: Option<u8>,
    pub soldier: Option<u8>,
    pub exobiologist: Option<u8>,
    pub federation: Option<u8>,
    pub empire: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct ProspectedMaterial {
    pub name: String,
    #[serde(rename = "Name_Localised")]
    pub name_localised: Option<String>,
    pub proportion: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsteroidContent {
    Low,
    Medium,
    High,
    Unknown,
}

impl AsteroidContent {
    pub fn from_journal(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "$asteroidmaterialcontent_low;" => Self::Low,
            "$asteroidmaterialcontent_medium;" => Self::Medium,
            "$asteroidmaterialcontent_high;" => Self::High,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct ProspectedAsteroid {
    pub materials: Vec<ProspectedMaterial>,
    pub content: String,
    #[serde(rename = "Content_Localised")]
    pub content_localised: Option<String>,
    pub motherlode_material: Option<String>,
    #[serde(rename = "MotherlodeMaterial_Localised")]
    pub motherlode_material_localised: Option<String>,
    pub remaining: Option<f64>,
}

impl ProspectedAsteroid {
    pub fn content_level(&self) -> AsteroidContent {
        AsteroidContent::from_journal(&self.content)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct AsteroidCracked {
    pub body: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct MiningRefined {
    #[serde(rename = "Type")]
    pub type_: String,
    #[serde(rename = "Type_Localised")]
    pub type_localised: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct LaunchDrone {
    #[serde(rename = "Type")]
    pub type_: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct EjectCargo {
    #[serde(rename = "Type")]
    pub type_: String,
    #[serde(rename = "Type_Localised")]
    pub type_localised: Option<String>,
    pub count: u32,
    pub abandoned: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct MarketSell {
    #[serde(rename = "MarketID")]
    pub market_id: Option<u64>,
    #[serde(rename = "Type")]
    pub type_: String,
    #[serde(rename = "Type_Localised")]
    pub type_localised: Option<String>,
    pub count: u32,
    pub sell_price: u64,
    pub total_sale: u64,
    pub avg_price_paid: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct RepairAll {
    pub cost: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Repair {
    pub items: Vec<String>,
    pub item: Option<String>,
    pub cost: Option<u64>,
}

/// Normaliza el nombre de una mercancía: minúsculas, sin `$`, `_name;` ni espacios.
pub fn commodity_key(raw: &str) -> String {
    let lower = raw.trim().to_lowercase();
    let t = lower.strip_prefix('$').unwrap_or(&lower);
    let t = t.strip_suffix(';').unwrap_or(t);
    let t = t.strip_suffix("_name").unwrap_or(t);
    t.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Variante tipada de un evento. Lo desconocido o malformado cae en `Other`.
#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
    Fileheader(Fileheader),
    Commander(Commander),
    LoadGame(LoadGame),
    Rank(Rank),
    Progress(Rank),
    Location(Box<LocationInfo>),
    FsdJump(Box<LocationInfo>),
    CarrierJump(Box<LocationInfo>),
    Docked(Docked),
    Undocked(Undocked),
    SupercruiseEntry(SupercruiseEntry),
    SupercruiseExit(SupercruiseExit),
    ApproachBody(BodyRef),
    LeaveBody(BodyRef),
    Touchdown(SurfaceEvent),
    Liftoff(SurfaceEvent),
    Embark(EmbarkEvent),
    Disembark(EmbarkEvent),
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
    ShipTargeted(Box<ShipTargeted>),
    Bounty(Box<Bounty>),
    FactionKillBond(FactionKillBond),
    CapShipBond(CapShipBond),
    RedeemVoucher(RedeemVoucher),
    ShieldState(ShieldState),
    HeatWarning,
    HeatDamage,
    EscapeInterdiction(EscapeInterdiction),
    PvpKill(PvpKill),
    FighterDestroyed,
    Promotion(Promotion),
    ProspectedAsteroid(Box<ProspectedAsteroid>),
    AsteroidCracked(AsteroidCracked),
    MiningRefined(MiningRefined),
    LaunchDrone(LaunchDrone),
    EjectCargo(EjectCargo),
    MarketSell(MarketSell),
    RepairAll(RepairAll),
    Repair(Repair),
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
        "Location" => de::<LocationInfo>(name, raw).map(|l| EventKind::Location(Box::new(l))),
        "FSDJump" => de::<LocationInfo>(name, raw).map(|l| EventKind::FsdJump(Box::new(l))),
        "CarrierJump" => de::<LocationInfo>(name, raw).map(|l| EventKind::CarrierJump(Box::new(l))),
        "Docked" => de(name, raw).map(EventKind::Docked),
        "Undocked" => de(name, raw).map(EventKind::Undocked),
        "SupercruiseEntry" => de(name, raw).map(EventKind::SupercruiseEntry),
        "SupercruiseExit" => de(name, raw).map(EventKind::SupercruiseExit),
        "ApproachBody" => de(name, raw).map(EventKind::ApproachBody),
        "LeaveBody" => de(name, raw).map(EventKind::LeaveBody),
        "Touchdown" => de(name, raw).map(EventKind::Touchdown),
        "Liftoff" => de(name, raw).map(EventKind::Liftoff),
        "Embark" => de(name, raw).map(EventKind::Embark),
        "Disembark" => de(name, raw).map(EventKind::Disembark),
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
        "ShipTargeted" | "ShipTargetted" => {
            de::<ShipTargeted>(name, raw).map(|t| EventKind::ShipTargeted(Box::new(t)))
        }
        "Bounty" => de::<Bounty>(name, raw).map(|b| EventKind::Bounty(Box::new(b))),
        "FactionKillBond" => de(name, raw).map(EventKind::FactionKillBond),
        "CapShipBond" => de(name, raw).map(EventKind::CapShipBond),
        "RedeemVoucher" => de(name, raw).map(EventKind::RedeemVoucher),
        "ShieldState" => de(name, raw).map(EventKind::ShieldState),
        "HeatWarning" => Some(EventKind::HeatWarning),
        "HeatDamage" => Some(EventKind::HeatDamage),
        "EscapeInterdiction" => de(name, raw).map(EventKind::EscapeInterdiction),
        "PVPKill" => de(name, raw).map(EventKind::PvpKill),
        "FighterDestroyed" => Some(EventKind::FighterDestroyed),
        "Promotion" => de(name, raw).map(EventKind::Promotion),
        "ProspectedAsteroid" => {
            de::<ProspectedAsteroid>(name, raw).map(|p| EventKind::ProspectedAsteroid(Box::new(p)))
        }
        "AsteroidCracked" => de(name, raw).map(EventKind::AsteroidCracked),
        "MiningRefined" => de(name, raw).map(EventKind::MiningRefined),
        "LaunchDrone" => de(name, raw).map(EventKind::LaunchDrone),
        "EjectCargo" => de(name, raw).map(EventKind::EjectCargo),
        "MarketSell" => de(name, raw).map(EventKind::MarketSell),
        "RepairAll" => de(name, raw).map(EventKind::RepairAll),
        "Repair" => de(name, raw).map(EventKind::Repair),
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

    fn loadout_with_modifier(modifier: &str) -> EventKind {
        ev(&format!(
            r#"{{"timestamp":"2026-04-10T20:00:00Z","event":"Loadout","Ship":"python","Modules":[{{"Slot":"FrameShiftDrive","Item":"int_hyperdrive_size5_class5","Health":1.0,"Engineering":{{"BlueprintName":"FSD_LongRange","Level":5,"Quality":0.9,"ExperimentalEffect":"special_fsd_fuelcapacity","Modifiers":[{modifier}]}}}}]}}"#
        ))
    }

    #[test]
    fn loadout_fsd_engineering_modifiers_are_parsed() {
        let EventKind::Loadout(l) = loadout_with_modifier(
            r#"{"Label":"FSDOptimalMass","Value":1692.6,"OriginalValue":1050.0,"LessIsGood":0}"#,
        ) else {
            panic!("Loadout")
        };
        let m = &l.modules[0];
        assert_eq!(m.item.as_deref(), Some("int_hyperdrive_size5_class5"));
        let e = m.engineering.as_ref().unwrap();
        assert_eq!(e.blueprint_name.as_deref(), Some("FSD_LongRange"));
        assert_eq!(e.level, Some(5));
        assert_eq!(e.quality, Some(0.9));
        assert_eq!(
            e.experimental_effect.as_deref(),
            Some("special_fsd_fuelcapacity")
        );
        assert_eq!(e.modifiers.len(), 1);
        assert_eq!(e.modifiers[0].label, "FSDOptimalMass");
        assert_eq!(e.modifiers[0].value, Some(1692.6));
        assert_eq!(e.modifiers[0].original_value, Some(1050.0));
        assert_eq!(e.modifiers[0].less_is_good, Some(0));
    }

    #[test]
    fn non_numeric_modifier_value_does_not_break_loadout() {
        let EventKind::Loadout(l) = loadout_with_modifier(
            r#"{"Label":"WeaponMode","ValueStr":"$Weapon_Mode_Burst;","ValueStr_Localised":"Burst"}"#,
        ) else {
            panic!("Loadout")
        };
        let m = &l.modules[0].engineering.as_ref().unwrap().modifiers[0];
        assert_eq!(m.label, "WeaponMode");
        assert_eq!(m.value, None);
    }

    #[test]
    fn modifier_value_as_text_is_none() {
        let EventKind::Loadout(l) =
            loadout_with_modifier(r#"{"Label":"WeaponMode","Value":"Burst"}"#)
        else {
            panic!("Loadout")
        };
        let m = &l.modules[0].engineering.as_ref().unwrap().modifiers[0];
        assert_eq!(m.label, "WeaponMode");
        assert_eq!(m.value, None);
    }

    #[test]
    fn fsdjump_carries_politics_and_body_id() {
        let EventKind::FsdJump(l) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"FSDJump","StarSystem":"Kappa","SystemAddress":1,"StarPos":[1,2,3],"BodyID":3,"SystemAllegiance":"Federation","SystemGovernment_Localised":"Democracia","SystemEconomy_Localised":"Industrial","SystemSecondEconomy_Localised":"Refinería","SystemSecurity_Localised":"Seguridad alta","Population":4500000,"SystemFaction":{"Name":"Kappa Gov","FactionState":"War"},"Powers":["Zachary Hudson"],"PowerplayState":"Fortified","BoostUsed":1.5}"#,
        ) else {
            panic!("FSDJump")
        };
        assert_eq!(l.body_id, Some(3));
        assert_eq!(l.system_allegiance.as_deref(), Some("Federation"));
        assert_eq!(l.system_government_localised.as_deref(), Some("Democracia"));
        assert_eq!(l.system_economy_localised.as_deref(), Some("Industrial"));
        assert_eq!(
            l.system_second_economy_localised.as_deref(),
            Some("Refinería")
        );
        assert_eq!(
            l.system_security_localised.as_deref(),
            Some("Seguridad alta")
        );
        assert_eq!(l.population, Some(4_500_000));
        let f = l.system_faction.as_ref().unwrap();
        assert_eq!(f.name.as_deref(), Some("Kappa Gov"));
        assert_eq!(f.faction_state.as_deref(), Some("War"));
        assert_eq!(l.powers, vec!["Zachary Hudson".to_string()]);
        assert_eq!(l.powerplay_state.as_deref(), Some("Fortified"));
        assert_eq!(l.boost_used, Some(1.5));
    }

    #[test]
    fn touchdown_carries_position_and_body_id() {
        let EventKind::Touchdown(t) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"Touchdown","Latitude":-12.5,"Longitude":33.1,"Body":"Planeta 1","BodyID":7,"SystemAddress":99,"OnPlanet":true,"PlayerControlled":true}"#,
        ) else {
            panic!("Touchdown")
        };
        assert_eq!(t.latitude, Some(-12.5));
        assert_eq!(t.longitude, Some(33.1));
        assert_eq!(t.body.as_deref(), Some("Planeta 1"));
        assert_eq!(t.body_id, Some(7));
        assert_eq!(t.system_address, Some(99));
        assert_eq!(t.on_planet, Some(true));
        assert_eq!(t.player_controlled, Some(true));
        // Sin campos sigue siendo Touchdown, no Other.
        let EventKind::Liftoff(l) = ev(r#"{"timestamp":"2026-04-10T20:00:00Z","event":"Liftoff"}"#)
        else {
            panic!("Liftoff")
        };
        assert_eq!(l, SurfaceEvent::default());
    }

    #[test]
    fn disembark_on_planet_parsed() {
        let EventKind::Disembark(d) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"Disembark","SRV":true,"Taxi":false,"Multicrew":false,"OnPlanet":true,"OnStation":false,"Body":"Planeta 1","BodyID":7,"SystemAddress":99}"#,
        ) else {
            panic!("Disembark")
        };
        assert!(d.srv);
        assert!(!d.taxi);
        assert!(d.on_planet);
        assert!(!d.on_station);
        assert_eq!(d.body.as_deref(), Some("Planeta 1"));
        assert_eq!(d.body_id, Some(7));
        assert_eq!(d.system_address, Some(99));
        let EventKind::Embark(e) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"Embark","SRV":false,"OnStation":true}"#,
        ) else {
            panic!("Embark")
        };
        assert!(e.on_station);
        assert_eq!(e.body_id, None);
    }

    #[test]
    fn approach_body_carries_ids() {
        let EventKind::ApproachBody(b) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"ApproachBody","Body":"P1","BodyID":4,"SystemAddress":12}"#,
        ) else {
            panic!("ApproachBody")
        };
        assert_eq!(b.body_id, Some(4));
        assert_eq!(b.system_address, Some(12));
    }

    #[test]
    fn ship_targeted_full_scan_parses() {
        let EventKind::ShipTargeted(t) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"ShipTargeted","TargetLocked":true,"Ship":"viper_mkiv","Ship_Localised":"Viper Mk IV","ScanStage":3,"PilotName":"$cmdr;","PilotRank":"Dangerous","ShieldHealth":45.5,"HullHealth":80.0,"Faction":"F","LegalStatus":"Wanted","Bounty":125000,"Subsystem":"$int_powerplant_size3_class5_name;","SubsystemHealth":62.0}"#,
        ) else {
            panic!("ShipTargeted")
        };
        assert!(t.target_locked);
        assert_eq!(t.ship.as_deref(), Some("viper_mkiv"));
        assert_eq!(t.ship_localised.as_deref(), Some("Viper Mk IV"));
        assert_eq!(t.scan_stage, Some(3));
        assert_eq!(t.pilot_rank.as_deref(), Some("Dangerous"));
        assert_eq!(t.shield_health, Some(45.5));
        assert_eq!(t.hull_health, Some(80.0));
        assert_eq!(t.legal_status.as_deref(), Some("Wanted"));
        assert_eq!(t.bounty, Some(125000));
        assert_eq!(
            t.subsystem.as_deref(),
            Some("$int_powerplant_size3_class5_name;")
        );
        assert_eq!(t.subsystem_health, Some(62.0));
        let EventKind::ShipTargeted(h) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"ShipTargetted","TargetLocked":false}"#,
        ) else {
            panic!("ShipTargetted alias")
        };
        assert!(!h.target_locked);
    }

    #[test]
    fn ship_targeted_lost_target_parses_minimal() {
        let EventKind::ShipTargeted(t) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"ShipTargeted","TargetLocked":false}"#,
        ) else {
            panic!("ShipTargeted")
        };
        assert!(!t.target_locked);
        assert_eq!(t.ship, None);
        assert_eq!(t.scan_stage, None);
        assert_eq!(t.bounty, None);
    }

    #[test]
    fn bounty_rewards_and_legacy_skimmer_format_normalize() {
        let EventKind::Bounty(b) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"Bounty","Rewards":[{"Faction":"A","Reward":50000},{"Faction":"B","Reward":10000}],"TotalReward":60000,"VictimFaction":"V","Target":"viper"}"#,
        ) else {
            panic!("Bounty")
        };
        let n = b.normalized_rewards();
        assert_eq!(n.len(), 2);
        assert_eq!(n.iter().map(|r| r.reward).sum::<u64>(), 60000);
        assert_eq!(n[0].faction, "A");
        assert_eq!(b.total_reward, Some(60000));
        assert_eq!(b.victim_faction.as_deref(), Some("V"));
        let EventKind::Bounty(s) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"Bounty","Reward":5000,"Faction":"C"}"#,
        ) else {
            panic!("Bounty legacy")
        };
        assert_eq!(
            s.normalized_rewards(),
            vec![BountyReward {
                faction: "C".into(),
                reward: 5000
            }]
        );
    }

    #[test]
    fn redeem_voucher_with_factions_and_broker() {
        let EventKind::RedeemVoucher(r) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"RedeemVoucher","Type":"bounty","Amount":37500,"Factions":[{"Faction":"A","Amount":50000}],"BrokerPercentage":25.0}"#,
        ) else {
            panic!("RedeemVoucher")
        };
        assert_eq!(r.type_, "bounty");
        assert_eq!(r.amount, 37500);
        assert_eq!(r.factions[0].faction, "A");
        assert_eq!(r.factions[0].amount, 50000);
        assert_eq!(r.broker_percentage, Some(25.0));
    }

    #[test]
    fn faction_kill_bond_and_cap_ship_bond_parse() {
        let body = r#""Reward":20000,"AwardingFaction":"A","VictimFaction":"B""#;
        let EventKind::FactionKillBond(f) = ev(&format!(
            r#"{{"timestamp":"2026-04-10T20:00:00Z","event":"FactionKillBond",{body}}}"#
        )) else {
            panic!("FactionKillBond")
        };
        assert_eq!(f.reward, 20000);
        assert_eq!(f.awarding_faction.as_deref(), Some("A"));
        assert_eq!(f.victim_faction.as_deref(), Some("B"));
        let EventKind::CapShipBond(c) = ev(&format!(
            r#"{{"timestamp":"2026-04-10T20:00:00Z","event":"CapShipBond",{body}}}"#
        )) else {
            panic!("CapShipBond")
        };
        assert_eq!(c.reward, 20000);
    }

    #[test]
    fn heat_and_shield_events_parse() {
        assert_eq!(
            ev(r#"{"timestamp":"2026-04-10T20:00:00Z","event":"HeatWarning"}"#),
            EventKind::HeatWarning
        );
        assert_eq!(
            ev(r#"{"timestamp":"2026-04-10T20:00:00Z","event":"HeatDamage"}"#),
            EventKind::HeatDamage
        );
        let EventKind::ShieldState(s) =
            ev(r#"{"timestamp":"2026-04-10T20:00:00Z","event":"ShieldState","ShieldsUp":false}"#)
        else {
            panic!("ShieldState")
        };
        assert!(!s.shields_up);
    }

    #[test]
    fn promotion_combat_rank_parses() {
        let EventKind::Promotion(p) =
            ev(r#"{"timestamp":"2026-04-10T20:00:00Z","event":"Promotion","Combat":5}"#)
        else {
            panic!("Promotion")
        };
        assert_eq!(p.combat, Some(5));
        assert_eq!(p.trade, None);
    }

    #[test]
    fn misc_combat_events_parse() {
        let EventKind::PvpKill(k) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"PVPKill","Victim":"X","CombatRank":4}"#,
        ) else {
            panic!("PVPKill")
        };
        assert_eq!(k.victim.as_deref(), Some("X"));
        assert_eq!(k.combat_rank, Some(4));
        let EventKind::EscapeInterdiction(e) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"EscapeInterdiction","Interdictor":"Y","IsPlayer":true}"#,
        ) else {
            panic!("EscapeInterdiction")
        };
        assert_eq!(e.is_player, Some(true));
        assert_eq!(
            ev(r#"{"timestamp":"2026-04-10T20:00:00Z","event":"FighterDestroyed"}"#),
            EventKind::FighterDestroyed
        );
    }

    #[test]
    fn prospected_asteroid_parses_materials_content_and_motherlode() {
        let EventKind::ProspectedAsteroid(p) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"ProspectedAsteroid","Materials":[{"Name":"Platinum","Name_Localised":"Platino","Proportion":31.5},{"Name":"Painite","Proportion":12.0}],"Content":"$AsteroidMaterialContent_High;","Content_Localised":"Alto","MotherlodeMaterial":"Monazite","Remaining":100.0}"#,
        ) else {
            panic!("ProspectedAsteroid")
        };
        assert_eq!(p.materials.len(), 2);
        assert_eq!(p.materials[0].name, "Platinum");
        assert_eq!(p.materials[0].name_localised.as_deref(), Some("Platino"));
        assert_eq!(p.materials[0].proportion, 31.5);
        assert_eq!(p.materials[1].proportion, 12.0);
        assert_eq!(p.content_level(), AsteroidContent::High);
        assert_eq!(p.motherlode_material.as_deref(), Some("Monazite"));
        assert_eq!(p.remaining, Some(100.0));
        // Sin campos opcionales sigue siendo ProspectedAsteroid.
        let EventKind::ProspectedAsteroid(m) =
            ev(r#"{"timestamp":"2026-04-10T20:00:00Z","event":"ProspectedAsteroid"}"#)
        else {
            panic!("minimal")
        };
        assert!(m.materials.is_empty());
        assert_eq!(m.motherlode_material, None);
        assert_eq!(m.content_level(), AsteroidContent::Unknown);
    }

    #[test]
    fn unknown_asteroid_content_is_unknown() {
        assert_eq!(
            AsteroidContent::from_journal("$Foo;"),
            AsteroidContent::Unknown
        );
        assert_eq!(
            AsteroidContent::from_journal("$AsteroidMaterialContent_Low;"),
            AsteroidContent::Low
        );
        assert_eq!(
            AsteroidContent::from_journal("$AsteroidMaterialContent_Medium;"),
            AsteroidContent::Medium
        );
    }

    #[test]
    fn mining_refined_and_launch_drone_parse() {
        let EventKind::MiningRefined(r) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"MiningRefined","Type":"$platinum_name;","Type_Localised":"Platino"}"#,
        ) else {
            panic!("MiningRefined")
        };
        assert_eq!(r.type_, "$platinum_name;");
        assert_eq!(commodity_key(&r.type_), "platinum");
        let EventKind::LaunchDrone(d) =
            ev(r#"{"timestamp":"2026-04-10T20:00:00Z","event":"LaunchDrone","Type":"Prospector"}"#)
        else {
            panic!("LaunchDrone")
        };
        assert_eq!(d.type_, "Prospector");
        let EventKind::AsteroidCracked(a) =
            ev(r#"{"timestamp":"2026-04-10T20:00:00Z","event":"AsteroidCracked","Body":"Ring A"}"#)
        else {
            panic!("AsteroidCracked")
        };
        assert_eq!(a.body.as_deref(), Some("Ring A"));
    }

    #[test]
    fn eject_cargo_and_repairs_parse() {
        let EventKind::EjectCargo(e) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"EjectCargo","Type":"limpet","Count":3,"Abandoned":true}"#,
        ) else {
            panic!("EjectCargo")
        };
        assert_eq!(e.type_, "limpet");
        assert_eq!(e.count, 3);
        assert_eq!(e.abandoned, Some(true));
        let EventKind::RepairAll(r) =
            ev(r#"{"timestamp":"2026-04-10T20:00:00Z","event":"RepairAll","Cost":1200}"#)
        else {
            panic!("RepairAll")
        };
        assert_eq!(r.cost, Some(1200));
        let EventKind::Repair(p) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"Repair","Items":["Hull","Cockpit"],"Cost":300}"#,
        ) else {
            panic!("Repair")
        };
        assert_eq!(p.items, vec!["Hull".to_string(), "Cockpit".to_string()]);
        assert_eq!(p.cost, Some(300));
        let EventKind::Repair(q) =
            ev(r#"{"timestamp":"2026-04-10T20:00:00Z","event":"Repair","Item":"Hull"}"#)
        else {
            panic!("Repair single")
        };
        assert_eq!(q.item.as_deref(), Some("Hull"));
        assert_eq!(q.cost, None);
    }

    #[test]
    fn market_sell_parses_prices() {
        let EventKind::MarketSell(m) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"MarketSell","MarketID":3700000001,"Type":"platinum","Count":20,"SellPrice":220000,"TotalSale":4400000,"AvgPricePaid":0}"#,
        ) else {
            panic!("MarketSell")
        };
        assert_eq!(m.market_id, Some(3700000001));
        assert_eq!(m.type_, "platinum");
        assert_eq!(m.count, 20);
        assert_eq!(m.sell_price, 220000);
        assert_eq!(m.total_sale, 4400000);
        assert_eq!(m.avg_price_paid, Some(0));
    }

    #[test]
    fn cargo_event_with_inventory_parses_limpets() {
        let EventKind::Cargo(c) = ev(
            r#"{"timestamp":"2026-04-10T20:00:00Z","event":"Cargo","Vessel":"Ship","Count":60,"Inventory":[{"Name":"drones","Name_Localised":"Limpet","Count":18},{"Name":"platinum","Count":42,"Stolen":0}]}"#,
        ) else {
            panic!("Cargo")
        };
        assert_eq!(c.count, Some(60));
        assert_eq!(c.inventory.len(), 2);
        assert_eq!(c.inventory[0].name, "drones");
        assert_eq!(c.inventory[0].count, 18);
        assert_eq!(c.inventory[0].stolen, None);
        assert_eq!(c.inventory[1].name, "platinum");
        assert_eq!(c.inventory[1].count, 42);
        assert_eq!(c.inventory[1].stolen, Some(0));
        let EventKind::Cargo(n) =
            ev(r#"{"timestamp":"2026-04-10T20:00:00Z","event":"Cargo","Vessel":"Ship","Count":7}"#)
        else {
            panic!("Cargo no inventory")
        };
        assert!(n.inventory.is_empty());
    }

    #[test]
    fn commodity_key_normalizes_variants() {
        assert_eq!(
            commodity_key("$lowtemperaturediamond_name;"),
            "lowtemperaturediamond"
        );
        assert_eq!(commodity_key("Platinum"), "platinum");
        assert_eq!(
            commodity_key("Low Temperature Diamond"),
            "lowtemperaturediamond"
        );
    }
}
