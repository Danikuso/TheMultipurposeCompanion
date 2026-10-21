//! Datos estáticos del juego: tabla de FSD y clases de estrella.

use crate::exobiology::predict::BioRules;
use crate::exobiology::species::SpeciesTable;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;
use tmc_model::events::{AsteroidContent, MaterialCategory};
use tmc_model::net::PadSize;

/// Species and colony distances embedded once, without runtime file access.
pub fn species_table() -> &'static SpeciesTable {
    static TABLE: OnceLock<SpeciesTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        SpeciesTable::from_json(include_str!("../data/species.json")).unwrap_or_else(|error| {
            tracing::error!(%error, "invalid embedded species.json; using empty table");
            SpeciesTable::default()
        })
    })
}

/// Prediction rules written by TMC, embedded once.
pub fn bio_rules() -> &'static BioRules {
    static RULES: OnceLock<BioRules> = OnceLock::new();
    RULES.get_or_init(|| {
        BioRules::from_json(include_str!("../data/bio_rules.json")).unwrap_or_else(|error| {
            tracing::error!(%error, "invalid embedded bio_rules.json; using no rules");
            BioRules::default()
        })
    })
}

/// Tipo de FSD (determina los multiplicadores de supercarga).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FsdKind {
    Standard,
    Sco,
    ScoMk2,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FsdEntry {
    pub item_prefix: String,
    pub optimal_mass: f64,
    pub max_fuel_per_jump: f64,
    pub fuel_mul: f64,
    pub fuel_power: f64,
    pub kind: FsdKind,
    #[serde(default)]
    pub provenance: String,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BoosterEntry {
    pub item_prefix: String,
    pub bonus_ly: f64,
    #[serde(default)]
    pub provenance: String,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FsdTable {
    pub source: String,
    pub license: String,
    pub version: String,
    pub fsd: Vec<FsdEntry>,
    pub boosters: Vec<BoosterEntry>,
}

fn matches_prefix(item: &str, prefix: &str) -> bool {
    let item = item.to_ascii_lowercase();
    item == prefix
        || item
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('_'))
}

impl FsdTable {
    pub fn find_fsd(&self, item: &str) -> Option<&FsdEntry> {
        self.fsd
            .iter()
            .find(|e| matches_prefix(item, &e.item_prefix))
    }

    pub fn find_booster(&self, item: &str) -> Option<&BoosterEntry> {
        self.boosters
            .iter()
            .find(|e| matches_prefix(item, &e.item_prefix))
    }
}

/// Tabla embebida, cargada una sola vez.
pub fn fsd_table() -> &'static FsdTable {
    static TABLE: OnceLock<FsdTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str(include_str!("../data/fsd.json")).unwrap_or_else(|e| {
            tracing::error!(error = %e, "fsd.json embebido inválido; tabla vacía");
            FsdTable {
                source: String::new(),
                license: String::new(),
                version: String::new(),
                fsd: Vec::new(),
                boosters: Vec::new(),
            }
        })
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShipPadEntry {
    pub ship: String,
    pub name: String,
    pub pad: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub verified: bool,
}

/// Landing pad size per ship (journal `Ship` symbol, lowercase).
#[derive(Debug, Clone, Default)]
pub struct ShipPads {
    pub entries: Vec<ShipPadEntry>,
}

fn pad_from_str(s: &str) -> Option<PadSize> {
    match s {
        "S" => Some(PadSize::Small),
        "M" => Some(PadSize::Medium),
        "L" => Some(PadSize::Large),
        _ => None,
    }
}

impl ShipPads {
    fn find(&self, ship: &str) -> Option<&ShipPadEntry> {
        let ship = ship.to_ascii_lowercase();
        self.entries
            .iter()
            .find(|e| e.ship == ship || e.aliases.iter().any(|a| a.eq_ignore_ascii_case(&ship)))
    }

    pub fn pad_for(&self, ship: &str) -> Option<PadSize> {
        self.find(ship).and_then(|e| pad_from_str(&e.pad))
    }

    /// Fallback for `Ship_Localised`.
    pub fn display_name(&self, ship: &str) -> Option<&str> {
        self.find(ship).map(|e| e.name.as_str())
    }
}

/// Tolerant parser: invalid input yields an empty table; invalid entries are skipped.
fn parse_ship_pads(json: &str) -> ShipPads {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return ShipPads::default();
    };
    let entries = value
        .get("entries")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| serde_json::from_value::<ShipPadEntry>(v.clone()).ok())
                .filter(|e| pad_from_str(&e.pad).is_some())
                .map(|mut e| {
                    e.ship = e.ship.to_ascii_lowercase();
                    e
                })
                .collect()
        })
        .unwrap_or_default();
    ShipPads { entries }
}

pub fn ship_pads() -> &'static ShipPads {
    static TABLE: OnceLock<ShipPads> = OnceLock::new();
    TABLE.get_or_init(|| {
        let t = parse_ship_pads(include_str!("../data/ship_pads.json"));
        if t.entries.is_empty() {
            tracing::error!("ship_pads.json embebido inválido o vacío; tabla vacía");
        }
        t
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct MiningYieldEntry {
    pub content: String,
    pub tons_per_asteroid: f64,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub verified: bool,
}

/// Typical tonnes per asteroid by content level (project estimates).
#[derive(Debug, Clone, Default)]
pub struct MiningYield {
    pub entries: Vec<MiningYieldEntry>,
}

impl MiningYield {
    pub fn tons_for(&self, c: AsteroidContent) -> Option<f64> {
        let key = match c {
            AsteroidContent::Low => "Low",
            AsteroidContent::Medium => "Medium",
            AsteroidContent::High => "High",
            AsteroidContent::Unknown => return None,
        };
        self.entries
            .iter()
            .find(|e| e.content == key)
            .map(|e| e.tons_per_asteroid)
    }
}

fn parse_mining_yield(json: &str) -> MiningYield {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return MiningYield::default();
    };
    let entries = value
        .get("entries")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| serde_json::from_value::<MiningYieldEntry>(v.clone()).ok())
                .filter(|e| matches!(e.content.as_str(), "Low" | "Medium" | "High"))
                .filter(|e| e.tons_per_asteroid.is_finite() && e.tons_per_asteroid > 0.0)
                .collect()
        })
        .unwrap_or_default();
    MiningYield { entries }
}

pub fn mining_yield() -> &'static MiningYield {
    static TABLE: OnceLock<MiningYield> = OnceLock::new();
    TABLE.get_or_init(|| {
        let t = parse_mining_yield(include_str!("../data/mining_yield.json"));
        if t.entries.is_empty() {
            tracing::error!("mining_yield.json embebido inválido o vacío; tabla vacía");
        }
        t
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct RareGood {
    pub key: String,
    pub origin_system: String,
    pub origin_station: String,
    pub sell_distance_ly: u32,
    pub source: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CabinClass {
    Economy,
    Business,
    First,
    Luxury,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CabinSpec {
    pub symbol: String,
    pub class: CabinClass,
    pub seats: u32,
    pub source: String,
    pub verified: bool,
}

fn parse_static_table<T: serde::de::DeserializeOwned>(
    json: &str,
    file: &str,
    key: fn(&T) -> &str,
    valid: fn(&T) -> bool,
) -> HashMap<String, T> {
    let value = match serde_json::from_str::<serde_json::Value>(json) {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(file, %error, "invalid embedded static data; using empty table");
            return HashMap::new();
        }
    };
    let Some(entries) = value.get("entries").and_then(serde_json::Value::as_array) else {
        tracing::warn!(file, "missing static data entries array; using empty table");
        return HashMap::new();
    };
    let mut table = HashMap::new();
    for (index, value) in entries.iter().enumerate() {
        match serde_json::from_value::<T>(value.clone()) {
            Ok(entry) if valid(&entry) => {
                table.insert(key(&entry).to_owned(), entry);
            }
            Ok(_) => tracing::warn!(file, index, "invalid static data entry; skipping row"),
            Err(error) => {
                tracing::warn!(file, index, %error, "invalid static data entry; skipping row")
            }
        }
    }
    table
}

fn parse_rare_goods(json: &str) -> HashMap<String, RareGood> {
    parse_static_table(
        json,
        "rare_goods.json",
        |entry| &entry.key,
        |entry| {
            !entry.key.trim().is_empty()
                && !entry.origin_system.trim().is_empty()
                && !entry.origin_station.trim().is_empty()
                && entry.sell_distance_ly > 0
                && !entry.source.trim().is_empty()
        },
    )
}

fn parse_cabins(json: &str) -> HashMap<String, CabinSpec> {
    parse_static_table(
        json,
        "cabins.json",
        |entry| &entry.symbol,
        |entry| {
            !entry.symbol.trim().is_empty() && entry.seats > 0 && !entry.source.trim().is_empty()
        },
    )
}

/// Project-authored rare goods, embedded once without runtime file access.
pub fn rare_goods() -> &'static HashMap<String, RareGood> {
    static TABLE: OnceLock<HashMap<String, RareGood>> = OnceLock::new();
    TABLE.get_or_init(|| parse_rare_goods(include_str!("../data/rare_goods.json")))
}

/// Passenger cabin facts indexed by the lowercase journal module symbol.
pub fn cabin_spec(symbol: &str) -> Option<&'static CabinSpec> {
    static TABLE: OnceLock<HashMap<String, CabinSpec>> = OnceLock::new();
    TABLE
        .get_or_init(|| parse_cabins(include_str!("../data/cabins.json")))
        .get(symbol)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HintKind {
    Mining,
    SurfaceProspecting,
    Hge,
    SignalSource,
    DataScan,
    MissionReward,
    Salvage,
    Broker,
    Site,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hint {
    pub kind: HintKind,
    /// Fluent key with extra detail, if any.
    #[serde(default)]
    pub detail_key: Option<String>,
}

/// Material facts (journal key lowercase), with provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDef {
    pub key: String,
    pub name_en: String,
    pub category: MaterialCategory,
    pub grade: u8,
    pub family: String,
    pub tradeable: bool,
    #[serde(default)]
    pub hints: Vec<Hint>,
    pub source: String,
    pub verified: bool,
}

fn parse_materials(json: &str) -> HashMap<String, MaterialDef> {
    parse_static_table(
        json,
        "materials.json",
        |entry: &MaterialDef| &entry.key,
        |entry| {
            !entry.key.trim().is_empty()
                && !entry.name_en.trim().is_empty()
                && !entry.family.trim().is_empty()
                && (1..=5).contains(&entry.grade)
                && !entry.source.trim().is_empty()
        },
    )
}

/// Material catalogue indexed by lowercase journal key.
pub fn materials() -> &'static HashMap<String, MaterialDef> {
    static TABLE: OnceLock<HashMap<String, MaterialDef>> = OnceLock::new();
    TABLE.get_or_init(|| parse_materials(include_str!("../data/materials.json")))
}

/// Storage cap per material grade (game constant `GRADE_CAPS`); out-of-range grades give 0.
pub fn grade_cap(grade: u8) -> u32 {
    const CAPS: [u32; 5] = [300, 250, 200, 150, 100];
    usize::from(grade)
        .checked_sub(1)
        .and_then(|i| CAPS.get(i))
        .copied()
        .unwrap_or(0)
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TraderRules {
    pub same_family_up: u32,
    pub same_family_down: u32,
    pub cross_family_factor: u32,
    pub source: String,
    pub verified: bool,
}

impl Default for TraderRules {
    fn default() -> Self {
        Self {
            same_family_up: 6,
            same_family_down: 3,
            cross_family_factor: 6,
            source: String::new(),
            verified: false,
        }
    }
}

fn parse_trader_rules(json: &str) -> TraderRules {
    serde_json::from_str(json).unwrap_or_else(|error| {
        tracing::warn!(file = "trader_rules.json", %error, "invalid embedded static data; using defaults");
        TraderRules::default()
    })
}

/// Material trader exchange rules (provisional until verified).
pub fn trader_rules() -> &'static TraderRules {
    static RULES: OnceLock<TraderRules> = OnceLock::new();
    RULES.get_or_init(|| parse_trader_rules(include_str!("../data/trader_rules.json")))
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Ingredient {
    pub material: String,
    pub count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModuleTypeDef {
    pub id: String,
    pub item_prefixes: Vec<String>,
}

/// Engineer blueprint (journal `BlueprintName`, lowercase) with a recipe per grade roll.
#[derive(Debug, Clone, Deserialize)]
pub struct Blueprint {
    pub key: String,
    pub module_types: Vec<String>,
    pub grades: BTreeMap<u8, Vec<Ingredient>>,
    pub source: String,
    pub verified: bool,
}

/// Experimental effect (journal `ExperimentalEffect`, lowercase).
#[derive(Debug, Clone, Deserialize)]
pub struct Experimental {
    pub key: String,
    pub module_types: Vec<String>,
    pub ingredients: Vec<Ingredient>,
    pub source: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Default)]
pub struct BlueprintCatalog {
    pub module_types: Vec<ModuleTypeDef>,
    pub blueprints: Vec<Blueprint>,
    pub experimentals: Vec<Experimental>,
}

fn valid_ingredients(list: &[Ingredient]) -> bool {
    !list.is_empty()
        && list
            .iter()
            .all(|i| !i.material.trim().is_empty() && i.count > 0)
}

fn parse_section<T: serde::de::DeserializeOwned>(
    value: &serde_json::Value,
    section: &str,
    valid: fn(&T) -> bool,
) -> Vec<T> {
    let Some(rows) = value.get(section).and_then(serde_json::Value::as_array) else {
        tracing::warn!(
            file = "blueprints.json",
            section,
            "missing section; using empty"
        );
        return Vec::new();
    };
    let mut out = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        match serde_json::from_value::<T>(row.clone()) {
            Ok(entry) if valid(&entry) => out.push(entry),
            Ok(_) => {
                tracing::warn!(
                    file = "blueprints.json",
                    section,
                    index,
                    "invalid row; skipping"
                )
            }
            Err(error) => tracing::warn!(
                file = "blueprints.json", section, index, %error, "invalid row; skipping"
            ),
        }
    }
    out
}

fn parse_blueprints(json: &str) -> BlueprintCatalog {
    let value = match serde_json::from_str::<serde_json::Value>(json) {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(file = "blueprints.json", %error, "invalid embedded static data; using empty catalogue");
            return BlueprintCatalog::default();
        }
    };
    let module_types = parse_section::<ModuleTypeDef>(&value, "module_types", |t| {
        !t.id.trim().is_empty() && !t.item_prefixes.is_empty()
    });
    let mut blueprints = parse_section::<Blueprint>(&value, "blueprints", |b| {
        !b.key.trim().is_empty()
            && !b.module_types.is_empty()
            && !b.source.trim().is_empty()
            && !b.grades.is_empty()
            && b.grades
                .iter()
                .all(|(g, list)| (1..=5).contains(g) && valid_ingredients(list))
    });
    let mut experimentals = parse_section::<Experimental>(&value, "experimentals", |e| {
        !e.key.trim().is_empty()
            && !e.module_types.is_empty()
            && !e.source.trim().is_empty()
            && valid_ingredients(&e.ingredients)
    });
    for b in &mut blueprints {
        b.key = b.key.to_ascii_lowercase();
    }
    for e in &mut experimentals {
        e.key = e.key.to_ascii_lowercase();
    }
    BlueprintCatalog {
        module_types,
        blueprints,
        experimentals,
    }
}

impl BlueprintCatalog {
    /// Module type id for a journal item symbol (case-insensitive prefix match).
    pub fn module_type_of(&self, item: &str) -> Option<&str> {
        let item = item.to_ascii_lowercase();
        self.module_types
            .iter()
            .find(|t| t.item_prefixes.iter().any(|p| item.starts_with(p.as_str())))
            .map(|t| t.id.as_str())
    }

    pub fn blueprints_for(&self, module_type: &str) -> Vec<&Blueprint> {
        self.blueprints
            .iter()
            .filter(|b| b.module_types.iter().any(|t| t == module_type))
            .collect()
    }

    pub fn experimentals_for(&self, module_type: &str) -> Vec<&Experimental> {
        self.experimentals
            .iter()
            .filter(|e| e.module_types.iter().any(|t| t == module_type))
            .collect()
    }

    pub fn blueprint(&self, key: &str) -> Option<&Blueprint> {
        let key = key.to_ascii_lowercase();
        self.blueprints.iter().find(|b| b.key == key)
    }

    pub fn experimental(&self, key: &str) -> Option<&Experimental> {
        let key = key.to_ascii_lowercase();
        self.experimentals.iter().find(|e| e.key == key)
    }
}

/// Blueprint and experimental catalogue (provisional minimal set, unverified).
pub fn blueprints() -> &'static BlueprintCatalog {
    static CATALOG: OnceLock<BlueprintCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| parse_blueprints(include_str!("../data/blueprints.json")))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StarSpecial {
    Neutron,
    WhiteDwarf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarFlags {
    pub scoopable: bool,
    pub special: Option<StarSpecial>,
}

const WHITE_DWARF_CLASSES: [&str; 15] = [
    "D", "DA", "DAB", "DAO", "DAZ", "DAV", "DB", "DBZ", "DBV", "DO", "DOV", "DQ", "DC", "DCV", "DX",
];

/// Propiedades de una clase de estrella del diario (`StarType`).
pub fn star_flags(class: &str) -> StarFlags {
    let base = class.split('_').next().unwrap_or("");
    let special = if base == "N" {
        Some(StarSpecial::Neutron)
    } else if WHITE_DWARF_CLASSES.contains(&base) {
        Some(StarSpecial::WhiteDwarf)
    } else {
        None
    };
    StarFlags {
        scoopable: matches!(base, "O" | "B" | "A" | "F" | "G" | "K" | "M"),
        special,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rare_goods_load_and_have_provenance() {
        let table = rare_goods();
        assert!(!table.is_empty());
        for (key, entry) in table {
            assert_eq!(key, &entry.key);
            assert!(!entry.source.trim().is_empty());
            assert!(!entry.origin_station.trim().is_empty());
            assert!(!entry.verified);
        }
        let brandy = &table["lavianbrandy"];
        assert_eq!(brandy.origin_system, "Lave");
        assert!(brandy.sell_distance_ly >= 150);
    }

    #[test]
    fn cabin_specs_known_sizes() {
        for (symbol, class, seats) in [
            ("int_passengercabin_size2_class1", CabinClass::Economy, 2),
            ("int_passengercabin_size4_class1", CabinClass::Economy, 8),
            ("int_passengercabin_size5_class4", CabinClass::Luxury, 4),
            ("int_passengercabin_size6_class3", CabinClass::First, 12),
        ] {
            let entry = cabin_spec(symbol).unwrap();
            assert_eq!(entry.symbol, symbol);
            assert_eq!((entry.class, entry.seats), (class, seats));
            assert!(!entry.source.trim().is_empty());
            assert!(!entry.verified);
        }
        assert!(cabin_spec("unknown_cabin").is_none());
        assert!(CabinClass::Economy < CabinClass::Business);
        assert!(CabinClass::Business < CabinClass::First);
        assert!(CabinClass::First < CabinClass::Luxury);
    }

    struct WarningCounter(std::sync::Arc<std::sync::atomic::AtomicUsize>);

    impl tracing::Subscriber for WarningCounter {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            if *event.metadata().level() == tracing::Level::WARN {
                self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
    }

    #[test]
    fn corrupt_static_data_is_ignored_with_warning() {
        let warnings = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        tracing::subscriber::with_default(WarningCounter(warnings.clone()), || {
            for json in ["{oops", "null", r#"{"entries":{}}"#] {
                assert!(parse_rare_goods(json).is_empty());
                assert!(parse_cabins(json).is_empty());
            }
        });
        assert_eq!(warnings.load(std::sync::atomic::Ordering::Relaxed), 6);
    }

    #[test]
    fn invalid_static_rows_are_skipped_between_valid_rows() {
        let rare = serde_json::json!({"key":"lavianbrandy", "origin_system":"Lave",
            "origin_station":"Lave Station", "sell_distance_ly":150, "source":"test", "verified":false});
        let cabin = serde_json::json!({"symbol":"int_passengercabin_size2_class1", "class":"Economy",
            "seats":2, "source":"test", "verified":false});
        let mut rare2 = rare.clone();
        rare2["key"] = "other".into();
        let mut cabin2 = cabin.clone();
        cabin2["symbol"] = "int_passengercabin_size4_class1".into();
        for bad in [
            serde_json::Value::Null,
            serde_json::json!({}),
            {
                let mut entry = rare.clone();
                entry["source"] = " ".into();
                entry
            },
            {
                let mut entry = rare.clone();
                entry["sell_distance_ly"] = 0.into();
                entry
            },
        ] {
            assert_eq!(
                parse_rare_goods(&serde_json::json!({"entries":[rare, bad, rare2]}).to_string())
                    .len(),
                2
            );
        }
        for bad in [
            serde_json::Value::Null,
            serde_json::json!({}),
            {
                let mut entry = cabin.clone();
                entry["source"] = " ".into();
                entry
            },
            {
                let mut entry = cabin.clone();
                entry["seats"] = 0.into();
                entry
            },
            {
                let mut entry = cabin.clone();
                entry["class"] = "Unknown".into();
                entry
            },
        ] {
            assert_eq!(
                parse_cabins(&serde_json::json!({"entries":[cabin, bad, cabin2]}).to_string())
                    .len(),
                2
            );
        }
    }

    #[test]
    fn star_flags_classes() {
        for c in [
            "K",
            "K_OrangeGiant",
            "A_BlueWhiteSuperGiant",
            "M_RedSuperGiant",
        ] {
            assert!(star_flags(c).scoopable, "{c}");
        }
        for c in [
            "AeBe",
            "TTS",
            "MS",
            "S",
            "L",
            "T",
            "Y",
            "W",
            "WN",
            "C",
            "CN",
            "DAB",
            "N",
            "H",
            "SupermassiveBlackHole",
            "",
        ] {
            assert!(!star_flags(c).scoopable, "{c}");
        }
        assert_eq!(star_flags("N").special, Some(StarSpecial::Neutron));
        assert_eq!(star_flags("DAB").special, Some(StarSpecial::WhiteDwarf));
        assert_eq!(star_flags("DA").special, Some(StarSpecial::WhiteDwarf));
        assert_eq!(star_flags("DXY").special, None);
        assert_eq!(star_flags("M").special, None);
        assert_eq!(star_flags("N_x").special, Some(StarSpecial::Neutron));
    }

    #[test]
    fn fsd_table_loads_and_has_sco_and_mk2_entries() {
        let _: FsdTable = serde_json::from_str(include_str!("../data/fsd.json")).unwrap();
        let t = fsd_table();
        let mut seen = std::collections::HashSet::new();
        for e in &t.fsd {
            assert!(e.fuel_mul > 0.0 && e.fuel_power > 0.0, "{}", e.item_prefix);
            assert!(seen.insert(e.item_prefix.clone()), "dup {}", e.item_prefix);
        }
        for b in &t.boosters {
            assert!(b.bonus_ly > 0.0, "{}", b.item_prefix);
            assert!(seen.insert(b.item_prefix.clone()), "dup {}", b.item_prefix);
        }
        assert_eq!(t.version, "2026-10-18");
        assert!(t.fsd.iter().any(|e| e.kind == FsdKind::Sco));
        assert!(t.fsd.iter().any(|e| e.kind == FsdKind::ScoMk2));
        for e in &t.fsd {
            assert!(
                e.optimal_mass > 0.0 && e.max_fuel_per_jump > 0.0,
                "{}",
                e.item_prefix
            );
            assert!(!e.provenance.is_empty());
        }
        assert_eq!(
            t.find_booster("int_guardianfsdbooster_size5")
                .unwrap()
                .bonus_ly,
            10.5
        );
        let a5 = t.find_fsd("int_hyperdrive_size5_class5").unwrap();
        assert_eq!(
            (
                a5.optimal_mass,
                a5.max_fuel_per_jump,
                a5.fuel_mul,
                a5.fuel_power
            ),
            (1050.0, 5.0, 0.012, 2.45)
        );
        assert!(t.find_fsd("int_hyperdrive_size9_class9_future").is_none());
    }

    #[test]
    fn ship_pads_known_sizes() {
        let p = ship_pads();
        assert_eq!(p.pad_for("anaconda"), Some(PadSize::Large));
        assert_eq!(p.pad_for("python"), Some(PadSize::Medium));
        assert_eq!(p.pad_for("sidewinder"), Some(PadSize::Small));
        assert_eq!(p.pad_for("type9"), Some(PadSize::Large));
        assert_eq!(p.pad_for("Python"), Some(PadSize::Medium));
        assert_eq!(p.display_name("krait_mkii"), Some("Krait Mk II"));
        assert_eq!(p.pad_for("nave_inexistente"), None);
    }

    #[test]
    fn every_ship_pad_entry_has_source_and_valid_pad() {
        let p = ship_pads();
        assert!(p.entries.len() >= 40, "{}", p.entries.len());
        assert_eq!(p.entries.len(), 47);
        for e in &p.entries {
            assert!(!e.source.is_empty(), "{}", e.ship);
            assert!(!e.name.is_empty(), "{}", e.ship);
        }
        let mut seen = std::collections::HashSet::new();
        for e in &p.entries {
            assert!(seen.insert(e.ship.clone()), "dup {}", e.ship);
        }
    }

    #[test]
    fn mining_yield_has_three_levels_unverified() {
        let m = mining_yield();
        let (l, md, h) = (
            m.tons_for(AsteroidContent::Low).unwrap(),
            m.tons_for(AsteroidContent::Medium).unwrap(),
            m.tons_for(AsteroidContent::High).unwrap(),
        );
        assert_eq!((l, md, h), (10.0, 20.0, 30.0));
        assert_eq!(m.tons_for(AsteroidContent::Unknown), None);
        assert_eq!(m.entries.len(), 3);
        assert!(m.entries.iter().all(|e| !e.verified));
    }

    #[test]
    fn corrupt_data_file_loads_empty_without_panic() {
        assert_eq!(parse_ship_pads("{oops").pad_for("anaconda"), None);
        let p = parse_ship_pads(
            r#"{"entries":[{"ship":"bad","name":"B","pad":"X","source":"s"},
            {"ship":"anaconda","name":"Anaconda","pad":"L","source":"s"}]}"#,
        );
        assert_eq!(p.pad_for("bad"), None);
        assert_eq!(p.pad_for("anaconda"), Some(PadSize::Large));
        assert_eq!(
            parse_mining_yield("{oops").tons_for(AsteroidContent::High),
            None
        );
        let m = parse_mining_yield(
            r#"{"entries":[{"content":"Nope","tons_per_asteroid":5.0,"source":"s"},
            {"content":"High","tons_per_asteroid":7.5,"source":"s"}]}"#,
        );
        assert_eq!(m.tons_for(AsteroidContent::High), Some(7.5));
        assert_eq!(m.tons_for(AsteroidContent::Low), None);
    }

    #[test]
    fn ship_pads_table_values_aliases_and_vulture() {
        let p = ship_pads();
        for (ship, pad) in [
            ("type6", PadSize::Medium),
            ("independant_trader", PadSize::Medium),
            ("cutter", PadSize::Large),
            ("empire_trader", PadSize::Large),
            ("typex", PadSize::Medium),
            ("anaconda", PadSize::Large),
            ("python", PadSize::Medium),
            ("cobramkiii", PadSize::Small),
            ("vulture", PadSize::Small),
            ("type11", PadSize::Large),
            ("explorer_nx", PadSize::Large),
        ] {
            assert_eq!(p.pad_for(ship), Some(pad), "{ship}");
        }
        assert_eq!(p.display_name("independant_trader"), Some("Keelback"));
        assert_eq!(p.pad_for("Kestrel_MkII"), Some(PadSize::Small));
        assert_eq!(p.display_name("kestrel_mkii"), Some("Kestrel Mk II"));
    }

    #[test]
    fn mining_yield_rejects_non_positive_or_non_finite() {
        let m = parse_mining_yield(
            r#"{"entries":[{"content":"Low","tons_per_asteroid":0.0},
            {"content":"Medium","tons_per_asteroid":-3.0},
            {"content":"High","tons_per_asteroid":1e999}]}"#,
        );
        assert_eq!(m.entries.len(), 0);
    }

    #[test]
    fn materials_load_with_provenance_and_valid_grades() {
        let table = materials();
        assert_eq!(table.len(), 29);
        for (key, m) in table {
            assert_eq!(key, &m.key);
            assert!(!m.source.trim().is_empty());
            assert!((1..=5).contains(&m.grade), "{key}");
            assert!(!m.verified);
        }
        let iron = &table["iron"];
        assert_eq!(
            (iron.category, iron.grade, iron.name_en.as_str()),
            (MaterialCategory::Raw, 1, "Iron")
        );
        let ceramics = &table["conductiveceramics"];
        assert_eq!(
            (ceramics.category, ceramics.grade),
            (MaterialCategory::Manufactured, 3)
        );
    }

    #[test]
    fn grade_caps_are_game_values() {
        let caps: Vec<u32> = (1..=5).map(grade_cap).collect();
        assert_eq!(caps, vec![300, 250, 200, 150, 100]);
        assert_eq!((grade_cap(0), grade_cap(6)), (0, 0));
    }

    #[test]
    fn families_have_one_material_per_grade_where_tradeable() {
        let mut seen = std::collections::HashSet::new();
        for m in materials().values().filter(|m| m.tradeable) {
            assert!(
                seen.insert((m.family.clone(), m.grade)),
                "{} grade {}",
                m.family,
                m.grade
            );
        }
        for family in ["raw-1", "heat", "conductive", "wake-scans"] {
            let mut grades: Vec<u8> = materials()
                .values()
                .filter(|m| m.family == family)
                .map(|m| m.grade)
                .collect();
            grades.sort_unstable();
            assert_eq!(grades, vec![1, 2, 3, 4, 5], "{family}");
        }
    }

    #[test]
    fn trader_rules_default_rates() {
        let rules = trader_rules();
        assert_eq!(
            (
                rules.same_family_up,
                rules.same_family_down,
                rules.cross_family_factor
            ),
            (6, 3, 6)
        );
        assert!(!rules.verified);
        assert!(!rules.source.trim().is_empty());
    }

    #[test]
    fn corrupt_material_row_skipped_with_warning() {
        let good = serde_json::json!({"key":"iron","name_en":"Iron","category":"Raw","grade":1,
            "family":"raw-4","tradeable":true,"hints":[],"source":"t","verified":false});
        let mut bad = good.clone();
        bad["grade"] = 9.into();
        let mut good2 = good.clone();
        good2["key"] = "carbon".into();
        let warnings = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        tracing::subscriber::with_default(WarningCounter(warnings.clone()), || {
            let json = serde_json::json!({"entries":[good, bad, good2]}).to_string();
            assert_eq!(parse_materials(&json).len(), 2);
        });
        assert_eq!(warnings.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[test]
    fn every_ingredient_is_a_catalogued_material() {
        let cat = blueprints();
        let all = cat
            .blueprints
            .iter()
            .flat_map(|b| b.grades.values().flatten())
            .chain(cat.experimentals.iter().flat_map(|e| e.ingredients.iter()));
        for i in all {
            assert!(materials().contains_key(&i.material), "{}", i.material);
            assert!(i.count > 0);
        }
    }

    #[test]
    fn every_blueprint_has_contiguous_grades_from_1() {
        let cat = blueprints();
        assert!(!cat.blueprints.is_empty());
        for b in &cat.blueprints {
            let grades: Vec<u8> = b.grades.keys().copied().collect();
            let max = *grades.last().unwrap();
            assert!(max <= 5, "{}", b.key);
            assert_eq!(grades, (1..=max).collect::<Vec<_>>(), "{}", b.key);
            assert!(!b.verified && !b.source.trim().is_empty());
            for t in &b.module_types {
                assert!(cat.module_types.iter().any(|m| &m.id == t), "{t}");
            }
        }
        for e in &cat.experimentals {
            assert!(!e.verified && !e.source.trim().is_empty());
        }
    }

    #[test]
    fn thrusters_have_dirty_drive_and_drag_drives() {
        let cat = blueprints();
        assert_eq!(
            cat.module_type_of("int_engine_size3_class5"),
            Some("engine")
        );
        assert_eq!(
            cat.module_type_of("Int_Hyperdrive_Size5_Class5"),
            Some("fsd")
        );
        assert_eq!(cat.module_type_of("int_shieldgenerator_size3_class5"), None);
        let dirty = cat.blueprint("Engine_Dirty").unwrap();
        assert_eq!(
            dirty.grades.keys().copied().collect::<Vec<_>>(),
            [1, 2, 3, 4, 5]
        );
        assert!(
            cat.blueprints_for("engine")
                .iter()
                .any(|b| b.key == "engine_dirty")
        );
        assert!(cat.blueprint("fsd_longrange").is_some());
        let drag = cat.experimental("special_engine_overloaded").unwrap();
        assert!(
            cat.experimentals_for("engine")
                .iter()
                .any(|e| e.key == drag.key)
        );
        assert!(cat.experimentals_for("fsd").is_empty());
        assert!(cat.blueprint("nope").is_none());
    }

    #[test]
    fn corrupt_blueprint_row_skipped_with_warning() {
        let good = serde_json::json!({"key":"engine_dirty","module_types":["engine"],
            "grades":{"1":[{"material":"carbon","count":1}]},"source":"t","verified":false});
        let mut zero_count = good.clone();
        zero_count["grades"] = serde_json::json!({"1":[{"material":"carbon","count":0}]});
        let mut zero_grade = good.clone();
        zero_grade["grades"] = serde_json::json!({"0":[{"material":"carbon","count":1}]});
        let mut good2 = good.clone();
        good2["key"] = "fsd_longrange".into();
        let warnings = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        tracing::subscriber::with_default(WarningCounter(warnings.clone()), || {
            let json = serde_json::json!({"module_types":[],"experimentals":[],
                "blueprints":[good, zero_count, zero_grade, good2]})
            .to_string();
            assert_eq!(parse_blueprints(&json).blueprints.len(), 2);
        });
        assert_eq!(warnings.load(std::sync::atomic::Ordering::Relaxed), 2);
        assert!(parse_blueprints("{oops").blueprints.is_empty());
    }

    #[test]
    fn fixture_material_keys_exist_in_catalogue() {
        for (key, category, grade) in [
            ("carbon", MaterialCategory::Raw, 1),
            ("iron", MaterialCategory::Raw, 1),
            ("nickel", MaterialCategory::Raw, 1),
            ("tellurium", MaterialCategory::Raw, 4),
            ("conductiveceramics", MaterialCategory::Manufactured, 3),
            ("pharmaceuticalisolators", MaterialCategory::Manufactured, 5),
            ("shieldpatternanalysis", MaterialCategory::Encoded, 4),
        ] {
            let m = &materials()[key];
            assert_eq!((m.category, m.grade), (category, grade), "{key}");
        }
    }
}
