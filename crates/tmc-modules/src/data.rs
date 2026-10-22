//! Datos estáticos del juego: tabla de FSD y clases de estrella.

use crate::exobiology::predict::BioRules;
use crate::exobiology::species::SpeciesTable;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;
use tmc_model::events::{AsteroidContent, MaterialCategory};
use tmc_model::net::PadSize;

use crate::engineering::ship::CurrentEng;

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

/// Cost of one roll: tradeable materials plus Merc Coins (not a material, never tradeable).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct RecipeCost {
    #[serde(default)]
    pub materials: Vec<Ingredient>,
    #[serde(default)]
    pub merc_coins: u32,
}

/// Item symbols a recipe/variant applies to. Both lists empty means "no evidence": it
/// matches nothing (the row stays visible as pending but is never offered for an item).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct ModuleSelector {
    #[serde(default)]
    pub items: Vec<String>,
    #[serde(default)]
    pub item_prefixes: Vec<String>,
}

impl ModuleSelector {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.item_prefixes.is_empty()
    }

    /// Case-insensitive exact symbol or prefix match.
    pub fn matches(&self, item: &str) -> bool {
        let item = item.to_ascii_lowercase();
        self.items.iter().any(|i| i.eq_ignore_ascii_case(&item))
            || self
                .item_prefixes
                .iter()
                .any(|p| !p.is_empty() && item.starts_with(&p.to_ascii_lowercase()))
    }

    fn normalised(mut self) -> Self {
        for s in self.items.iter_mut().chain(self.item_prefixes.iter_mut()) {
            *s = s.trim().to_ascii_lowercase();
        }
        self.items.retain(|s| !s.is_empty());
        self.item_prefixes.retain(|s| !s.is_empty());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaseIdentity {
    Blueprint { key: String },
    Variant { key: String },
}

/// Installed base that unlocks an `ExistingOnly` upgrade path.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BaseRequirement {
    pub identity: BaseIdentity,
    pub selector: ModuleSelector,
    pub completed_grade: u8,
    /// Evidence of the transition itself, not only of both identities.
    pub source_rows: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryPolicy {
    /// Chain starts at grade 1.
    Stock,
    /// Needs a proven installed base; never starts from an empty grade 1.
    ExistingOnly {
        #[serde(default)]
        bases: Vec<BaseRequirement>,
    },
}

/// Recipes of one blueprint for one module context.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ModuleBlueprint {
    pub module_type: String,
    #[serde(default)]
    pub selector: Option<ModuleSelector>,
    pub entry: EntryPolicy,
    pub grades: BTreeMap<u8, RecipeCost>,
}

impl ModuleBlueprint {
    fn applies_to(&self, module_type: &str, item: &str) -> bool {
        self.module_type == module_type && self.selector.as_ref().is_none_or(|s| s.matches(item))
    }
}

/// Engineer blueprint (journal `BlueprintName`, lowercase) with recipes per module context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blueprint {
    pub key: String,
    pub recipes: Vec<ModuleBlueprint>,
    pub source: String,
    pub verified: bool,
}

impl Blueprint {
    /// Distinct module types with at least one recipe context.
    pub fn module_types(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for r in &self.recipes {
            if !out.contains(&r.module_type.as_str()) {
                out.push(&r.module_type);
            }
        }
        out
    }

    pub fn has_module_type(&self, module_type: &str) -> bool {
        self.recipes.iter().any(|r| r.module_type == module_type)
    }

    /// Highest grade of any context.
    pub fn max_grade(&self) -> Option<u8> {
        self.recipes
            .iter()
            .filter_map(|r| r.grades.keys().next_back().copied())
            .max()
    }
}

/// Experimental effect (journal `ExperimentalEffect`, lowercase).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Experimental {
    pub key: String,
    pub module_types: Vec<String>,
    pub cost: RecipeCost,
    pub source: String,
    pub verified: bool,
}

/// Purchased/pre-engineered variant: visible identity, never a free recipe.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ShipVariant {
    pub key: String,
    pub module_type: String,
    pub grade: u8,
    #[serde(default)]
    pub selector: Option<ModuleSelector>,
    /// Verified journal blueprint names; empty means the identity cannot be resolved yet.
    #[serde(default)]
    pub journal_blueprints: Vec<String>,
    pub source: String,
    pub verified: bool,
}

/// Journal spelling of a blueprint, valid only inside one module type.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BlueprintAlias {
    pub module_type: String,
    pub alias: String,
    pub canonical: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeResolutionError {
    UnknownBlueprint,
    IncompatibleItem,
    MissingBaseMapping,
    BaseRequired,
    AmbiguousRecipe,
}

#[derive(Debug, Clone)]
pub struct BlueprintOption<'a> {
    pub blueprint: &'a Blueprint,
    pub availability: Result<&'a ModuleBlueprint, RecipeResolutionError>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModuleTypeDef {
    pub id: String,
    /// Exact journal symbols (win over any prefix).
    #[serde(default)]
    pub items: Vec<String>,
    /// Types without proven symbols stay listed but match no item.
    #[serde(default)]
    pub item_prefixes: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BlueprintCatalog {
    pub module_types: Vec<ModuleTypeDef>,
    pub blueprints: Vec<Blueprint>,
    pub experimentals: Vec<Experimental>,
    pub variants: Vec<ShipVariant>,
    pub aliases: Vec<BlueprintAlias>,
}

fn valid_ingredients(list: &[Ingredient]) -> bool {
    list.iter()
        .all(|i| !i.material.trim().is_empty() && i.count > 0)
}

fn valid_cost(cost: &RecipeCost) -> bool {
    valid_ingredients(&cost.materials) && (!cost.materials.is_empty() || cost.merc_coins > 0)
}

/// Stock chains start at G1; `ExistingOnly` starts at G2 or later; no gaps in either.
fn valid_grades(entry: &EntryPolicy, grades: &BTreeMap<u8, RecipeCost>) -> bool {
    let (Some(first), Some(last)) = (grades.keys().next(), grades.keys().next_back()) else {
        return false;
    };
    let start_ok = match entry {
        EntryPolicy::Stock => *first == 1,
        EntryPolicy::ExistingOnly { .. } => *first >= 2,
    };
    start_ok
        && *last <= 5
        && usize::from(*last - *first) + 1 == grades.len()
        && grades.values().all(valid_cost)
}

fn valid_base(base: &BaseRequirement) -> bool {
    let key = match &base.identity {
        BaseIdentity::Blueprint { key } | BaseIdentity::Variant { key } => key,
    };
    !key.trim().is_empty()
        && !base.selector.is_empty()
        && (1..=5).contains(&base.completed_grade)
        && !base.source_rows.is_empty()
        && base.source_rows.iter().all(|r| !r.trim().is_empty())
}

fn valid_recipe(rec: &ModuleBlueprint) -> bool {
    !rec.module_type.trim().is_empty()
        && valid_grades(&rec.entry, &rec.grades)
        && match &rec.entry {
            EntryPolicy::Stock => true,
            EntryPolicy::ExistingOnly { bases } => bases.iter().all(valid_base),
        }
}

fn normalise_recipe(mut rec: ModuleBlueprint) -> ModuleBlueprint {
    rec.module_type = rec.module_type.trim().to_ascii_lowercase();
    rec.selector = rec.selector.map(ModuleSelector::normalised);
    if let EntryPolicy::ExistingOnly { bases } = &mut rec.entry {
        for b in bases {
            b.selector = std::mem::take(&mut b.selector).normalised();
            match &mut b.identity {
                BaseIdentity::Blueprint { key } | BaseIdentity::Variant { key } => {
                    *key = key.trim().to_ascii_lowercase();
                }
            }
        }
    }
    rec
}

/// Blueprint row: new shape (`recipes`) or the provisional one (`module_types` + `grades`).
#[derive(Deserialize)]
struct BlueprintRow {
    key: String,
    #[serde(default)]
    recipes: Option<Vec<ModuleBlueprint>>,
    #[serde(default)]
    module_types: Option<Vec<String>>,
    #[serde(default)]
    grades: Option<BTreeMap<u8, Vec<Ingredient>>>,
    source: String,
    #[serde(default)]
    verified: bool,
}

fn blueprint_from_row(row: BlueprintRow) -> Option<Blueprint> {
    if row.key.trim().is_empty() || row.source.trim().is_empty() {
        return None;
    }
    let recipes = match (row.recipes, row.module_types, row.grades) {
        (Some(recipes), _, _) => recipes,
        (None, Some(types), Some(grades)) => {
            let mut seen: Vec<String> = Vec::new();
            for t in types {
                if !seen.contains(&t) {
                    seen.push(t);
                }
            }
            seen.into_iter()
                .map(|module_type| ModuleBlueprint {
                    module_type,
                    selector: None,
                    entry: EntryPolicy::Stock,
                    grades: grades
                        .iter()
                        .map(|(g, list)| {
                            (
                                *g,
                                RecipeCost {
                                    materials: list.clone(),
                                    merc_coins: 0,
                                },
                            )
                        })
                        .collect(),
                })
                .collect()
        }
        _ => return None,
    };
    let total = recipes.len();
    let recipes: Vec<ModuleBlueprint> = recipes
        .into_iter()
        .filter(valid_recipe)
        .map(normalise_recipe)
        .collect();
    if recipes.is_empty() {
        return None;
    }
    if recipes.len() < total {
        tracing::warn!(
            file = "blueprints.json",
            key = %row.key,
            skipped = total - recipes.len(),
            "invalid recipe contexts; skipping them"
        );
    }
    Some(Blueprint {
        key: row.key.trim().to_ascii_lowercase(),
        recipes,
        source: row.source,
        verified: row.verified,
    })
}

#[derive(Deserialize)]
struct ExperimentalRow {
    key: String,
    module_types: Vec<String>,
    #[serde(default)]
    cost: Option<RecipeCost>,
    #[serde(default)]
    ingredients: Option<Vec<Ingredient>>,
    source: String,
    #[serde(default)]
    verified: bool,
}

fn experimental_from_row(row: ExperimentalRow) -> Option<Experimental> {
    let cost = match (row.cost, row.ingredients) {
        (Some(cost), _) => cost,
        (None, Some(materials)) => RecipeCost {
            materials,
            merc_coins: 0,
        },
        _ => return None,
    };
    (!row.key.trim().is_empty()
        && !row.module_types.is_empty()
        && !row.source.trim().is_empty()
        && valid_cost(&cost))
    .then(|| Experimental {
        key: row.key.trim().to_ascii_lowercase(),
        module_types: row
            .module_types
            .into_iter()
            .map(|t| t.trim().to_ascii_lowercase())
            .collect(),
        cost,
        source: row.source,
        verified: row.verified,
    })
}

fn variant_from_row(mut v: ShipVariant) -> Option<ShipVariant> {
    (!v.key.trim().is_empty()
        && !v.module_type.trim().is_empty()
        && (1..=5).contains(&v.grade)
        && !v.source.trim().is_empty())
    .then(|| {
        v.key = v.key.trim().to_ascii_lowercase();
        v.module_type = v.module_type.trim().to_ascii_lowercase();
        v.selector = v.selector.map(ModuleSelector::normalised);
        v.journal_blueprints = v
            .journal_blueprints
            .iter()
            .map(|b| b.trim().to_ascii_lowercase())
            .filter(|b| !b.is_empty())
            .collect();
        v
    })
}

fn alias_from_row(mut a: BlueprintAlias) -> Option<BlueprintAlias> {
    a.module_type = a.module_type.trim().to_ascii_lowercase();
    a.alias = a.alias.trim().to_ascii_lowercase();
    a.canonical = a.canonical.trim().to_ascii_lowercase();
    (!a.module_type.is_empty() && !a.alias.is_empty() && !a.canonical.is_empty()).then_some(a)
}

fn module_type_from_row(mut t: ModuleTypeDef) -> Option<ModuleTypeDef> {
    t.id = t.id.trim().to_ascii_lowercase();
    for s in t.items.iter_mut().chain(t.item_prefixes.iter_mut()) {
        *s = s.trim().to_ascii_lowercase();
    }
    t.items.retain(|s| !s.is_empty());
    t.item_prefixes.retain(|s| !s.is_empty());
    (!t.id.is_empty()).then_some(t)
}

/// Rows of `section`; absent optional sections yield nothing without a warning.
fn parse_section<R: serde::de::DeserializeOwned, T>(
    value: &serde_json::Value,
    section: &str,
    required: bool,
    convert: fn(R) -> Option<T>,
) -> Vec<T> {
    let Some(rows) = value.get(section).and_then(serde_json::Value::as_array) else {
        if required {
            tracing::warn!(
                file = "blueprints.json",
                section,
                "missing section; using empty"
            );
        }
        return Vec::new();
    };
    let mut out = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        match serde_json::from_value::<R>(row.clone()) {
            Ok(entry) => match convert(entry) {
                Some(entry) => out.push(entry),
                None => tracing::warn!(
                    file = "blueprints.json",
                    section,
                    index,
                    "invalid row; skipping"
                ),
            },
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
    BlueprintCatalog {
        module_types: parse_section(&value, "module_types", true, module_type_from_row),
        blueprints: parse_section(&value, "blueprints", true, blueprint_from_row),
        experimentals: parse_section(&value, "experimentals", true, experimental_from_row),
        variants: parse_section(&value, "variants", false, variant_from_row),
        aliases: parse_section(&value, "aliases", false, alias_from_row),
    }
}

/// Alias table from a separate symbols document (`aliases` array), tolerant per row.
#[cfg(test)]
#[allow(dead_code)]
fn parse_aliases(json: &str) -> Vec<BlueprintAlias> {
    match serde_json::from_str::<serde_json::Value>(json) {
        Ok(value) => parse_section(&value, "aliases", true, alias_from_row),
        Err(error) => {
            tracing::warn!(file = "symbols.json", %error, "invalid alias data; using none");
            Vec::new()
        }
    }
}

enum Fit<'a> {
    Valid(&'a ModuleBlueprint),
    MissingMapping,
    BaseRequired,
}

/// `level > n` or a full-quality `level == n` proves grade `n` completed.
fn grade_completed(current: &CurrentEng, grade: u8) -> bool {
    current.level > grade
        || (current.level == grade && current.quality.is_finite() && current.quality >= 0.999)
}

impl BlueprintCatalog {
    /// Module type id for a journal item symbol. An exact symbol wins over any prefix, the
    /// longest prefix wins over shorter ones, and a tie between different types is `None`.
    pub fn module_type_of(&self, item: &str) -> Option<&str> {
        let item = item.to_ascii_lowercase();
        let mut exact: Vec<&str> = Vec::new();
        for t in self.module_types.iter().filter(|t| t.items.contains(&item)) {
            if !exact.contains(&t.id.as_str()) {
                exact.push(&t.id);
            }
        }
        match exact.as_slice() {
            [one] => return Some(one),
            [] => {}
            _ => return None,
        }
        let mut best = 0;
        let mut ids: Vec<&str> = Vec::new();
        for t in &self.module_types {
            for p in t
                .item_prefixes
                .iter()
                .filter(|p| item.starts_with(p.as_str()))
            {
                if p.len() > best {
                    best = p.len();
                    ids.clear();
                }
                if p.len() == best && !ids.contains(&t.id.as_str()) {
                    ids.push(&t.id);
                }
            }
        }
        match ids.as_slice() {
            [one] => Some(one),
            _ => None,
        }
    }

    /// Blueprints with at least one recipe context for a module type.
    pub fn blueprints_for(&self, module_type: &str) -> Vec<&Blueprint> {
        self.blueprints
            .iter()
            .filter(|b| b.has_module_type(module_type))
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

    pub fn variants_for(&self, module_type: &str) -> Vec<&ShipVariant> {
        self.variants
            .iter()
            .filter(|v| v.module_type == module_type)
            .collect()
    }

    /// Canonical blueprint key for a journal spelling inside a module type. A key with a
    /// recipe in that module wins; otherwise a single, unambiguous alias of that module.
    /// Unknown or ambiguous spellings give `None` (callers keep the raw text).
    pub fn canonical_blueprint(&self, module_type: &str, raw: &str) -> Option<&str> {
        let raw = raw.trim().to_ascii_lowercase();
        let find = |key: &str| {
            self.blueprints
                .iter()
                .find(|b| b.key == key && b.has_module_type(module_type))
                .map(|b| b.key.as_str())
        };
        find(&raw).or_else(|| self.unique_alias_target(module_type, &raw, find))
    }

    pub fn canonical_experimental(&self, module_type: &str, raw: &str) -> Option<&str> {
        let raw = raw.trim().to_ascii_lowercase();
        let find = |key: &str| {
            self.experimentals
                .iter()
                .find(|e| e.key == key && e.module_types.iter().any(|t| t == module_type))
                .map(|e| e.key.as_str())
        };
        find(&raw).or_else(|| self.unique_alias_target(module_type, &raw, find))
    }

    fn unique_alias_target<'a>(
        &'a self,
        module_type: &str,
        raw: &str,
        target: impl Fn(&str) -> Option<&'a str>,
    ) -> Option<&'a str> {
        let mut found: Option<&str> = None;
        for a in self
            .aliases
            .iter()
            .filter(|a| a.module_type == module_type && a.alias == raw)
        {
            // A dangling alias targets nothing here: it neither resolves nor blocks the lookup
            // (`audit` reports every dangling alias).
            let Some(key) = target(&a.canonical) else {
                continue;
            };
            if found.is_some_and(|f| f != key) {
                return None;
            }
            found = Some(key);
        }
        found
    }

    fn key_is_known(&self, raw: &str) -> bool {
        let raw = raw.trim().to_ascii_lowercase();
        self.blueprints.iter().any(|b| b.key == raw) || self.aliases.iter().any(|a| a.alias == raw)
    }

    /// Variant installed in a module, recognised only through verified journal names.
    pub fn variant_of(&self, item: &str, current: &CurrentEng) -> Option<&ShipVariant> {
        let module_type = self.module_type_of(item)?;
        let raw = current.blueprint.trim().to_ascii_lowercase();
        let mut found: Option<&ShipVariant> = None;
        for v in self.variants.iter().filter(|v| {
            v.module_type == module_type
                && v.grade == current.level
                && v.selector.as_ref().is_none_or(|s| s.matches(item))
                && v.journal_blueprints.contains(&raw)
        }) {
            if found.is_some() {
                return None;
            }
            found = Some(v);
        }
        found
    }

    fn base_satisfied(
        &self,
        base: &BaseRequirement,
        module_type: &str,
        item: &str,
        current: Option<&CurrentEng>,
    ) -> bool {
        let Some(cur) = current else {
            return false;
        };
        if !grade_completed(cur, base.completed_grade) || !base.selector.matches(item) {
            return false;
        }
        match &base.identity {
            BaseIdentity::Blueprint { key } => {
                self.canonical_blueprint(module_type, &cur.blueprint) == Some(key.as_str())
            }
            BaseIdentity::Variant { key } => {
                self.variant_of(item, cur).is_some_and(|v| &v.key == key)
            }
        }
    }

    /// Bases of an `ExistingOnly` context proven by the installed module (empty for `Stock`).
    pub fn satisfied_bases<'a>(
        &self,
        rec: &'a ModuleBlueprint,
        item: &str,
        current: Option<&CurrentEng>,
    ) -> Vec<&'a BaseRequirement> {
        match &rec.entry {
            EntryPolicy::Stock => Vec::new(),
            EntryPolicy::ExistingOnly { bases } => bases
                .iter()
                .filter(|b| self.base_satisfied(b, &rec.module_type, item, current))
                .collect(),
        }
    }

    fn fit<'a>(
        &self,
        rec: &'a ModuleBlueprint,
        item: &str,
        current: Option<&CurrentEng>,
    ) -> Fit<'a> {
        match &rec.entry {
            EntryPolicy::Stock => Fit::Valid(rec),
            EntryPolicy::ExistingOnly { bases } if bases.is_empty() => Fit::MissingMapping,
            EntryPolicy::ExistingOnly { bases } => {
                if bases
                    .iter()
                    .any(|b| self.base_satisfied(b, &rec.module_type, item, current))
                {
                    Fit::Valid(rec)
                } else {
                    Fit::BaseRequired
                }
            }
        }
    }

    /// Exactly one valid context wins; never the first match. A compatible context still
    /// lacking its base mapping blocks resolution because it could compete with the valid one.
    fn resolve_in<'a>(
        &self,
        bp: &'a Blueprint,
        module_type: &str,
        item: &str,
        current: Option<&CurrentEng>,
    ) -> Result<&'a ModuleBlueprint, RecipeResolutionError> {
        use RecipeResolutionError as E;
        let mut valid: Vec<&ModuleBlueprint> = Vec::new();
        let (mut missing, mut any) = (false, false);
        for rec in bp
            .recipes
            .iter()
            .filter(|r| r.applies_to(module_type, item))
        {
            any = true;
            match self.fit(rec, item, current) {
                Fit::Valid(r) => valid.push(r),
                Fit::MissingMapping => missing = true,
                Fit::BaseRequired => {}
            }
        }
        match valid.as_slice() {
            _ if !any => Err(E::IncompatibleItem),
            [one] if !missing => Ok(one),
            [] if missing => Err(E::MissingBaseMapping),
            [] => Err(E::BaseRequired),
            _ => Err(E::AmbiguousRecipe),
        }
    }

    /// Recipe context for an item and a journal blueprint spelling.
    pub fn blueprint_for_item(
        &self,
        item: &str,
        key: &str,
        current: Option<&CurrentEng>,
    ) -> Result<(&Blueprint, &ModuleBlueprint), RecipeResolutionError> {
        use RecipeResolutionError as E;
        if !self.key_is_known(key) {
            return Err(E::UnknownBlueprint);
        }
        let module_type = self.module_type_of(item).ok_or(E::IncompatibleItem)?;
        let canonical = self
            .canonical_blueprint(module_type, key)
            .ok_or(E::IncompatibleItem)?;
        let bp = self.blueprint(canonical).ok_or(E::UnknownBlueprint)?;
        self.resolve_in(bp, module_type, item, current)
            .map(|rec| (bp, rec))
    }

    /// Every blueprint with a context for the item, with the reason when unavailable.
    pub fn blueprints_for_item(
        &self,
        item: &str,
        current: Option<&CurrentEng>,
    ) -> Vec<BlueprintOption<'_>> {
        let Some(module_type) = self.module_type_of(item) else {
            return Vec::new();
        };
        self.blueprints
            .iter()
            .filter(|b| b.recipes.iter().any(|r| r.applies_to(module_type, item)))
            .map(|blueprint| BlueprintOption {
                blueprint,
                availability: self.resolve_in(blueprint, module_type, item, current),
            })
            .collect()
    }

    /// Strict consistency report (empty when sound). Runtime loading stays tolerant; this is
    /// for importer-level checks of a whole catalogue.
    pub fn audit(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let ids: Vec<&str> = self.module_types.iter().map(|t| t.id.as_str()).collect();
        for (i, t) in self.module_types.iter().enumerate() {
            for other in &self.module_types[i + 1..] {
                let clash = |a: &[String], b: &[String]| a.iter().any(|s| b.contains(s));
                if t.id == other.id
                    || clash(&t.items, &other.items)
                    || clash(&t.item_prefixes, &other.item_prefixes)
                {
                    errors.push(format!("module types {} and {} collide", t.id, other.id));
                }
            }
        }
        for (i, b) in self.blueprints.iter().enumerate() {
            if self.blueprints[i + 1..].iter().any(|o| o.key == b.key) {
                errors.push(format!("duplicate blueprint {}", b.key));
            }
            for r in &b.recipes {
                if !ids.contains(&r.module_type.as_str()) {
                    errors.push(format!("{}: unknown module type {}", b.key, r.module_type));
                }
            }
            for (j, r) in b.recipes.iter().enumerate() {
                for o in &b.recipes[j + 1..] {
                    if r.module_type == o.module_type
                        && (r.selector.is_none()
                            || o.selector.is_none()
                            || r.selector == o.selector)
                    {
                        errors.push(format!(
                            "{}: overlapping contexts for {}",
                            b.key, r.module_type
                        ));
                    }
                }
            }
        }
        for a in &self.aliases {
            if !self
                .blueprints
                .iter()
                .any(|b| b.key == a.canonical && b.has_module_type(&a.module_type))
                && !self.experimentals.iter().any(|e| e.key == a.canonical)
                && !self.variants.iter().any(|v| v.key == a.canonical)
            {
                errors.push(format!("alias {} targets unknown {}", a.alias, a.canonical));
            }
        }
        for b in &self.blueprints {
            for base in b.recipes.iter().flat_map(|r| match &r.entry {
                EntryPolicy::ExistingOnly { bases } => bases.as_slice(),
                EntryPolicy::Stock => &[],
            }) {
                let exists = match &base.identity {
                    BaseIdentity::Blueprint { key } => {
                        self.blueprints.iter().any(|o| &o.key == key)
                    }
                    BaseIdentity::Variant { key } => self.variants.iter().any(|v| &v.key == key),
                };
                if !exists {
                    errors.push(format!(
                        "{}: base identity {:?} does not exist",
                        b.key, base.identity
                    ));
                }
            }
        }
        for (i, a) in self.aliases.iter().enumerate() {
            if self.aliases[i + 1..].iter().any(|o| {
                o.module_type == a.module_type && o.alias == a.alias && o.canonical != a.canonical
            }) {
                errors.push(format!("ambiguous alias {}/{}", a.module_type, a.alias));
            }
        }
        errors
    }
}

/// Blueprint and experimental catalogue (provisional minimal set, unverified).
pub fn blueprints() -> &'static BlueprintCatalog {
    static CATALOG: OnceLock<BlueprintCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| parse_blueprints(include_str!("../data/blueprints.json")))
}

/// What an engineer offers: one blueprint for one module type, at an explicit set of grades
/// (a gap such as `[1, 3]` is a gap: it never covers grade 2).
#[derive(Debug, Clone, Deserialize)]
pub struct EngineerBlueprint {
    pub module_type: String,
    pub blueprint: String,
    pub grades: Vec<u8>,
}

impl EngineerBlueprint {
    /// Highest offered grade (0 when none).
    pub fn max_grade(&self) -> u8 {
        self.grades.iter().copied().max().unwrap_or(0)
    }

    /// `true` only if every required grade is offered. An empty requirement is never
    /// covered: nothing is "any".
    pub fn covers(&self, module_type: &str, blueprint: &str, required: &[u8]) -> bool {
        !required.is_empty()
            && self.module_type.eq_ignore_ascii_case(module_type)
            && self.blueprint.eq_ignore_ascii_case(blueprint)
            && required.iter().all(|g| self.grades.contains(g))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Contribution {
    pub item: String,
    /// "commodity" | "material" | "credits".
    pub kind: String,
    pub amount: u32,
}

impl Contribution {
    /// `kind` is one of commodity/material/credits (case-insensitive) and `amount > 0`.
    pub fn is_valid(&self) -> bool {
        self.amount > 0
            && ["commodity", "material", "credits"]
                .iter()
                .any(|k| self.kind.eq_ignore_ascii_case(k))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UnlockSteps {
    pub requirement_key: String,
    #[serde(default)]
    pub invite_key: Option<String>,
    #[serde(default)]
    pub contribution: Option<Contribution>,
}

/// Engineer facts (journal `EngineerID`), with provenance.
#[derive(Debug, Clone, Deserialize)]
pub struct EngineerDef {
    pub id: u64,
    pub name: String,
    pub system: String,
    pub star_pos: [f64; 3],
    pub workshop: String,
    #[serde(default)]
    pub market_id: Option<u64>,
    pub on_foot: bool,
    #[serde(default)]
    pub blueprints: Vec<EngineerBlueprint>,
    pub unlock: UnlockSteps,
    pub source: String,
    pub verified: bool,
}

/// Active engineer table: root `entries` (the importer writes the same root).
pub(crate) fn parse_engineers(json: &str) -> Vec<EngineerDef> {
    let Some(entries) = serde_json::from_str::<serde_json::Value>(json)
        .map_err(|error| {
            tracing::warn!(file = "engineers.json", %error, "invalid embedded static data; using empty table")
        })
        .ok()
        .and_then(|v| {
            v.get("entries")
                .and_then(serde_json::Value::as_array)
                .cloned()
        })
    else {
        tracing::warn!(file = "engineers.json", "missing entries array; using empty table");
        return Vec::new();
    };
    let mut out: Vec<EngineerDef> = Vec::new();
    for (index, value) in entries.into_iter().enumerate() {
        match serde_json::from_value::<EngineerDef>(value) {
            Ok(mut e) if engineer_is_valid(&e) && !out.iter().any(|o| o.id == e.id) => {
                // An invalid contribution only drops that step; the engineer row stays.
                if e.unlock
                    .contribution
                    .as_ref()
                    .is_some_and(|c| !c.is_valid())
                {
                    tracing::warn!(
                        file = "engineers.json",
                        index,
                        "invalid contribution; ignoring that unlock step"
                    );
                    e.unlock.contribution = None;
                }
                out.push(e);
            }
            Ok(_) => tracing::warn!(
                file = "engineers.json",
                index,
                "invalid engineer row; skipping"
            ),
            Err(error) => {
                tracing::warn!(file = "engineers.json", index, %error, "invalid engineer row; skipping")
            }
        }
    }
    out
}

fn engineer_is_valid(e: &EngineerDef) -> bool {
    !e.name.trim().is_empty()
        && !e.system.trim().is_empty()
        && !e.source.trim().is_empty()
        && !e.unlock.requirement_key.trim().is_empty()
        && e.star_pos.iter().all(|c| c.is_finite())
        && e.blueprints.iter().all(|b| {
            !b.module_type.trim().is_empty()
                && !b.blueprint.trim().is_empty()
                && !b.grades.is_empty()
                && b.grades.iter().all(|g| (1..=5).contains(g))
        })
}

/// Engineers (provisional minimal set, unverified), embedded once.
pub fn engineers() -> &'static Vec<EngineerDef> {
    static TABLE: OnceLock<Vec<EngineerDef>> = OnceLock::new();
    TABLE.get_or_init(|| parse_engineers(include_str!("../data/engineers.json")))
}

pub fn engineer(id: u64) -> Option<&'static EngineerDef> {
    engineers().iter().find(|e| e.id == id)
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
        assert_eq!(table.len(), 141);
        for (key, m) in table {
            assert_eq!(key, &m.key);
            assert!(!m.source.trim().is_empty());
            assert!((1..=5).contains(&m.grade), "{key}");
            // Guardian grades disagree between EDEngineer and FDevIDs: unverified.
            let guardian = m.family.starts_with("guardian");
            let own = key == "tg_abrasion03" || key == "tg_causticcrystal";
            assert_eq!(m.verified, !guardian && !own, "{key}");
            // Two Thargoid materials used by recipes are absent from EDEngineer: EDDiscovery data.
            if key == "tg_abrasion03" || key == "tg_causticcrystal" {
                assert!(
                    !m.verified && m.source.starts_with("EDDiscovery 7d30920"),
                    "{key}"
                );
            } else {
                assert_eq!(m.source, "EDEngineer 2cadd66 (MIT)", "{key}");
            }
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
    fn materials_full_catalogue_size() {
        use std::collections::BTreeMap;
        let mut by_category: BTreeMap<String, usize> = BTreeMap::new();
        for m in materials().values() {
            *by_category.entry(format!("{:?}", m.category)).or_default() += 1;
        }
        assert_eq!(materials().len(), 141);
        assert_eq!(by_category["Raw"], 28);
        assert_eq!(by_category["Manufactured"], 66);
        assert_eq!(by_category["Encoded"], 47);
        // Raw materials have four grades (seven categories); the rest five.
        let mut raw_one: Vec<u8> = materials()
            .values()
            .filter(|m| m.family == "raw-1")
            .map(|m| m.grade)
            .collect();
        raw_one.sort_unstable();
        assert_eq!(raw_one, vec![1, 2, 3, 4]);
        // Journal keys differ from the display name for some encoded data.
        assert!(materials().contains_key("shieldpatternanalysis"));
        assert!(materials().contains_key("fsdtelemetry"));
        assert!(!materials().contains_key("anomalousfsdtelemetry"));
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
        for family in ["heat", "conductive", "wake-scans"] {
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
            .flat_map(|b| b.recipes.iter().flat_map(|r| r.grades.values()))
            .chain(cat.experimentals.iter().map(|e| &e.cost))
            .flat_map(|c| c.materials.iter());
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
            for r in &b.recipes {
                let grades: Vec<u8> = r.grades.keys().copied().collect();
                let (min, max) = (grades[0], *grades.last().unwrap());
                assert!(max <= 5, "{}", b.key);
                // Stock chains start at G1; ExistingOnly (Operations) upgrades start at G2+.
                match r.entry {
                    EntryPolicy::Stock => assert_eq!(min, 1, "{}", b.key),
                    EntryPolicy::ExistingOnly { .. } => assert!(min >= 2, "{}", b.key),
                }
                assert_eq!(grades, (min..=max).collect::<Vec<_>>(), "{}", b.key);
            }
            assert!(!b.verified && !b.source.trim().is_empty());
            for t in b.module_types() {
                assert!(cat.module_types.iter().any(|m| m.id == t), "{t}");
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
            dirty.recipes[0].grades.keys().copied().collect::<Vec<_>>(),
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
        assert!(!cat.experimentals_for("fsd").is_empty());
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

    #[test]
    fn engineers_load_with_coords_and_provenance() {
        let all = engineers();
        assert!(all.iter().any(|e| e.on_foot));
        for e in all {
            assert!(e.star_pos.iter().all(|c| c.is_finite()), "{}", e.name);
            assert!(!e.source.trim().is_empty() && !e.verified, "{}", e.name);
        }
        let f = engineer(300100).unwrap();
        assert_eq!(
            (f.name.as_str(), f.system.as_str()),
            ("Felicity Farseer", "Deciat")
        );
        assert_eq!(engineer(300160).unwrap().name, "Elvira Martuuk");
        assert_eq!(engineer(300260).unwrap().name, "Tod 'The Blaster' McQuinn");
        assert!(engineer(1).is_none());
        let mut ids: Vec<u64> = all.iter().map(|e| e.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), all.len());
        let dirty = |id| {
            engineer(id)
                .unwrap()
                .blueprints
                .iter()
                .find(|b| b.blueprint == "engine_dirty")
                .map(|b| b.max_grade())
        };
        assert_eq!(
            (dirty(300220), dirty(300100), dirty(300160)),
            (Some(5), Some(3), Some(2))
        );
        assert_eq!(engineer(300220).unwrap().system, "Arque");
        assert!(
            engineer(300100)
                .unwrap()
                .blueprints
                .iter()
                .any(|b| b.blueprint == "fsd_longrange")
        );
    }

    #[test]
    fn engineer_blueprints_exist_in_catalogue() {
        for e in engineers() {
            if e.on_foot {
                continue;
            }
            for b in &e.blueprints {
                assert!(
                    blueprints().blueprint(&b.blueprint).is_some(),
                    "{} {}",
                    e.name,
                    b.blueprint
                );
                assert!((1..=5).contains(&b.max_grade()));
            }
        }
    }

    #[test]
    fn corrupt_engineer_row_skipped_with_warning() {
        let good = serde_json::json!({"id":1,"name":"A","system":"S","star_pos":[0.0,1.0,2.0],
            "workshop":"W","on_foot":false,"blueprints":[],
            "unlock":{"requirement_key":"unlock-a"},"source":"t","verified":false});
        let mut bad = good.clone();
        bad["star_pos"] = serde_json::json!(["x", 1.0, 2.0]);
        let mut bad_grade = good.clone();
        bad_grade["id"] = 3.into();
        bad_grade["blueprints"] =
            serde_json::json!([{"module_type":"engine","blueprint":"engine_dirty","grades":[1,9]}]);
        let mut good2 = good.clone();
        good2["id"] = 2.into();
        let warnings = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        tracing::subscriber::with_default(WarningCounter(warnings.clone()), || {
            let json = serde_json::json!({"entries":[good, bad, bad_grade, good2]}).to_string();
            assert_eq!(parse_engineers(&json).len(), 2);
        });
        assert_eq!(warnings.load(std::sync::atomic::Ordering::Relaxed), 2);
        assert!(parse_engineers("{oops").is_empty());
    }

    #[test]
    fn duplicate_engineer_id_warns_and_first_wins() {
        let row = |id: u64, name: &str| {
            serde_json::json!({"id":id,"name":name,"system":"S","star_pos":[0.0,1.0,2.0],
                "workshop":"W","on_foot":false,"blueprints":[],
                "unlock":{"requirement_key":"unlock-a"},"source":"t","verified":false})
        };
        let warnings = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        tracing::subscriber::with_default(WarningCounter(warnings.clone()), || {
            let json =
                serde_json::json!({"entries":[row(1, "First"), row(1, "Second"), row(2, "Other")]})
                    .to_string();
            let parsed = parse_engineers(&json);
            assert_eq!(parsed.len(), 2);
            assert_eq!(parsed[0].name, "First");
        });
        assert_eq!(warnings.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[test]
    fn invalid_contribution_is_dropped_keeping_the_engineer() {
        let row = |contribution: serde_json::Value| {
            serde_json::json!({"id":1,"name":"A","system":"S","star_pos":[0.0,1.0,2.0],
                "workshop":"W","on_foot":false,"blueprints":[],
                "unlock":{"requirement_key":"unlock-a","contribution":contribution},
                "source":"t","verified":false})
        };
        for bad in [
            serde_json::json!({"item":"x","kind":"gold","amount":3}),
            serde_json::json!({"item":"x","kind":"material","amount":0}),
        ] {
            let json = serde_json::json!({"entries":[row(bad)]}).to_string();
            let parsed = parse_engineers(&json);
            assert_eq!(parsed.len(), 1);
            assert!(parsed[0].unlock.contribution.is_none());
        }
        let ok = serde_json::json!({"item":"x","kind":"Commodity","amount":3});
        let json = serde_json::json!({"entries":[row(ok)]}).to_string();
        assert!(parse_engineers(&json)[0].unlock.contribution.is_some());
    }

    // ----- C2: recipe contexts, eligibility and aliases -----

    fn cost(materials: &[(&str, u32)], merc_coins: u32) -> serde_json::Value {
        serde_json::json!({
            "materials": materials.iter().map(|(m, c)| serde_json::json!({"material": m, "count": c})).collect::<Vec<_>>(),
            "merc_coins": merc_coins,
        })
    }

    fn grades(range: std::ops::RangeInclusive<u8>) -> serde_json::Value {
        let map: serde_json::Map<String, serde_json::Value> = range
            .map(|g| (g.to_string(), cost(&[("iron", u32::from(g))], 0)))
            .collect();
        map.into()
    }

    fn context(
        module_type: &str,
        selector: serde_json::Value,
        entry: serde_json::Value,
        range: std::ops::RangeInclusive<u8>,
    ) -> serde_json::Value {
        serde_json::json!({"module_type": module_type, "selector": selector, "entry": entry,
            "grades": grades(range)})
    }

    fn blueprint_row(key: &str, recipes: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({"key": key, "recipes": recipes, "source": "synthetic fixture", "verified": false})
    }

    fn catalogue(
        blueprints: Vec<serde_json::Value>,
        variants: Vec<serde_json::Value>,
        aliases: Vec<serde_json::Value>,
    ) -> BlueprintCatalog {
        parse_blueprints(
            &serde_json::json!({
                "module_types": [
                    {"id": "engine", "item_prefixes": ["int_engine_"]},
                    {"id": "special_engine", "items": ["int_engine_size3_class5_special"]},
                    {"id": "rack", "item_prefixes": ["int_rack_"]},
                    {"id": "pending", "item_prefixes": []},
                ],
                "blueprints": blueprints,
                "experimentals": [{"key": "special_test", "module_types": ["engine"],
                    "cost": cost(&[("iron", 1)], 0), "source": "synthetic fixture", "verified": false}],
                "variants": variants,
                "aliases": aliases,
            })
            .to_string(),
        )
    }

    fn stock() -> serde_json::Value {
        serde_json::json!("stock")
    }

    fn existing(bases: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({"existing_only": {"bases": bases}})
    }

    fn base(identity: serde_json::Value, grade: u8, items: &[&str]) -> serde_json::Value {
        serde_json::json!({"identity": identity, "selector": {"items": items},
            "completed_grade": grade, "source_rows": ["synthetic:1:0"]})
    }

    fn eng(blueprint: &str, level: u8, quality: f64) -> CurrentEng {
        CurrentEng {
            blueprint: blueprint.into(),
            level,
            quality,
            experimental: None,
        }
    }

    use RecipeResolutionError as E;

    #[test]
    fn grades_match_entry_policy() {
        let warnings = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cat = tracing::subscriber::with_default(WarningCounter(warnings.clone()), || {
            catalogue(
                vec![
                    blueprint_row(
                        "ok_stock",
                        vec![context("engine", serde_json::Value::Null, stock(), 1..=5)],
                    ),
                    blueprint_row(
                        "ok_existing",
                        vec![context(
                            "engine",
                            serde_json::Value::Null,
                            existing(vec![]),
                            2..=5,
                        )],
                    ),
                    // Stock without G1, ExistingOnly with G1, and a gap are rejected rows.
                    blueprint_row(
                        "stock_no_g1",
                        vec![context("engine", serde_json::Value::Null, stock(), 2..=5)],
                    ),
                    blueprint_row(
                        "existing_with_g1",
                        vec![context(
                            "engine",
                            serde_json::Value::Null,
                            existing(vec![]),
                            1..=5,
                        )],
                    ),
                    blueprint_row(
                        "gap",
                        vec![{
                            let mut c = context("engine", serde_json::Value::Null, stock(), 1..=5);
                            c["grades"].as_object_mut().unwrap().remove("3");
                            c
                        }],
                    ),
                    blueprint_row(
                        "grade_six",
                        vec![context("engine", serde_json::Value::Null, stock(), 1..=6)],
                    ),
                ],
                vec![],
                vec![],
            )
        });
        let keys: Vec<&str> = cat.blueprints.iter().map(|b| b.key.as_str()).collect();
        assert_eq!(keys, ["ok_stock", "ok_existing"]);
        assert_eq!(warnings.load(std::sync::atomic::Ordering::Relaxed), 4);
        let existing = &cat.blueprint("ok_existing").unwrap().recipes[0];
        assert_eq!(
            existing.grades.keys().copied().collect::<Vec<_>>(),
            [2, 3, 4, 5]
        );
        assert_eq!(
            cat.blueprint("ok_stock").unwrap().recipes[0]
                .grades
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            [1, 2, 3, 4, 5]
        );
        // ExistingOnly G2 is never filled in with a free G1.
        assert!(!existing.grades.contains_key(&1));
        // Merc Coins alone make a valid cost; an empty cost is not.
        assert!(valid_cost(&RecipeCost {
            materials: vec![],
            merc_coins: 1
        }));
        assert!(!valid_cost(&RecipeCost::default()));
    }

    #[test]
    fn module_exact_symbol_precedes_prefix() {
        let cat = catalogue(vec![], vec![], vec![]);
        assert_eq!(
            cat.module_type_of("int_engine_size3_class5"),
            Some("engine")
        );
        assert_eq!(
            cat.module_type_of("INT_ENGINE_SIZE3_CLASS5_SPECIAL"),
            Some("special_engine")
        );
        assert_eq!(cat.module_type_of("int_unknown_size1"), None);
        // Longest prefix wins; equal specificity with different results gives no answer.
        let cat = parse_blueprints(
            &serde_json::json!({"module_types": [
                {"id": "short", "item_prefixes": ["int_a_"]},
                {"id": "long", "item_prefixes": ["int_a_big"]},
                {"id": "tie_one", "item_prefixes": ["int_t_"]},
                {"id": "tie_two", "item_prefixes": ["int_t_"]},
                {"id": "same", "item_prefixes": ["int_s_", "int_s_"]},
            ], "blueprints": [], "experimentals": []})
            .to_string(),
        );
        assert_eq!(cat.module_type_of("int_a_big_one"), Some("long"));
        assert_eq!(cat.module_type_of("int_a_small"), Some("short"));
        assert_eq!(cat.module_type_of("int_t_x"), None);
        assert_eq!(cat.module_type_of("int_s_x"), Some("same"));
        assert!(cat.audit().iter().any(|e| e.contains("tie_one")));
    }

    #[test]
    fn blueprint_alias_resolves_in_module_context() {
        let cat = catalogue(
            vec![
                blueprint_row(
                    "weapon_longrange",
                    vec![
                        context("engine", serde_json::Value::Null, stock(), 1..=5),
                        context("rack", serde_json::Value::Null, stock(), 1..=5),
                    ],
                ),
                blueprint_row(
                    "rack_only",
                    vec![context("rack", serde_json::Value::Null, stock(), 1..=5)],
                ),
            ],
            vec![],
            vec![
                serde_json::json!({"module_type": "engine", "alias": "Engine_LongRange", "canonical": "weapon_longrange"}),
                serde_json::json!({"module_type": "rack", "alias": "dup", "canonical": "weapon_longrange"}),
                serde_json::json!({"module_type": "rack", "alias": "dup", "canonical": "rack_only"}),
            ],
        );
        assert_eq!(
            cat.canonical_blueprint("engine", "ENGINE_longrange"),
            Some("weapon_longrange")
        );
        // Same spelling means nothing in another module.
        assert_eq!(cat.canonical_blueprint("rack", "engine_longrange"), None);
        // A key with a recipe in the module resolves to itself; raw unknown is not invented.
        assert_eq!(
            cat.canonical_blueprint("rack", "Weapon_LongRange"),
            Some("weapon_longrange")
        );
        assert_eq!(cat.canonical_blueprint("engine", "rack_only"), None);
        assert_eq!(cat.canonical_blueprint("engine", "mystery"), None);
        // An alias with two targets in one module is ambiguous, never first-match.
        assert_eq!(cat.canonical_blueprint("rack", "dup"), None);
        assert!(
            cat.audit()
                .iter()
                .any(|e| e.contains("ambiguous alias rack/dup"))
        );
        // The context-aware resolution goes through the alias.
        let (bp, rec) = cat
            .blueprint_for_item("int_engine_size2_class1", "engine_longrange", None)
            .unwrap();
        assert_eq!(
            (bp.key.as_str(), rec.module_type.as_str()),
            ("weapon_longrange", "engine")
        );
        assert_eq!(
            cat.blueprint_for_item("int_engine_size2_class1", "mystery", None)
                .unwrap_err(),
            E::UnknownBlueprint
        );
        assert_eq!(
            cat.blueprint_for_item("int_engine_size2_class1", "rack_only", None)
                .unwrap_err(),
            E::IncompatibleItem
        );
        assert_eq!(
            cat.blueprint_for_item("int_unknown_x", "weapon_longrange", None)
                .unwrap_err(),
            E::IncompatibleItem
        );
        assert_eq!(
            cat.canonical_experimental("engine", "SPECIAL_TEST"),
            Some("special_test")
        );
        assert_eq!(cat.canonical_experimental("rack", "special_test"), None);
    }

    #[test]
    fn ambiguous_recipe_is_rejected() {
        let both = |order: bool| {
            let mut ctx = vec![
                context("engine", serde_json::Value::Null, stock(), 1..=5),
                context(
                    "engine",
                    serde_json::json!({"items": ["int_engine_size3_class5"]}),
                    stock(),
                    1..=3,
                ),
            ];
            if order {
                ctx.reverse();
            }
            catalogue(vec![blueprint_row("dup", ctx)], vec![], vec![])
        };
        for order in [false, true] {
            let cat = both(order);
            // The exact-symbol context overlaps the unrestricted one: no first match.
            assert_eq!(
                cat.blueprint_for_item("int_engine_size3_class5", "dup", None)
                    .unwrap_err(),
                E::AmbiguousRecipe
            );
            // Another item only fits the unrestricted context.
            let (_, rec) = cat
                .blueprint_for_item("int_engine_size2_class1", "dup", None)
                .unwrap();
            assert!(rec.selector.is_none());
            assert!(cat.audit().iter().any(|e| e.contains("overlapping")));
        }
        // A compatible context still lacking its base mapping blocks the valid one.
        let cat = catalogue(
            vec![blueprint_row(
                "mixed",
                vec![
                    context(
                        "engine",
                        serde_json::json!({"items": ["int_engine_size3_class5"]}),
                        stock(),
                        1..=5,
                    ),
                    context("engine", serde_json::Value::Null, existing(vec![]), 2..=5),
                ],
            )],
            vec![],
            vec![],
        );
        assert_eq!(
            cat.blueprint_for_item("int_engine_size3_class5", "mixed", None)
                .unwrap_err(),
            E::AmbiguousRecipe
        );
        assert_eq!(
            cat.blueprint_for_item("int_engine_size2_class1", "mixed", None)
                .unwrap_err(),
            E::MissingBaseMapping
        );
        // Two satisfied bases of different contexts are ambiguous as well.
        let identity = serde_json::json!({"blueprint": {"key": "start"}});
        let cat = catalogue(
            vec![
                blueprint_row(
                    "start",
                    vec![context("engine", serde_json::Value::Null, stock(), 1..=5)],
                ),
                blueprint_row(
                    "up",
                    vec![
                        context(
                            "engine",
                            serde_json::Value::Null,
                            existing(vec![base(
                                identity.clone(),
                                1,
                                &["int_engine_size3_class5"],
                            )]),
                            2..=5,
                        ),
                        context(
                            "engine",
                            serde_json::json!({"items": ["int_engine_size3_class5"]}),
                            existing(vec![base(identity, 1, &["int_engine_size3_class5"])]),
                            2..=5,
                        ),
                    ],
                ),
            ],
            vec![],
            vec![],
        );
        let current = eng("Start", 1, 1.0);
        assert_eq!(
            cat.blueprint_for_item("int_engine_size3_class5", "up", Some(&current))
                .unwrap_err(),
            E::AmbiguousRecipe
        );
    }

    #[test]
    fn variant_without_recipe_is_not_free() {
        let variant = |journal: Vec<&str>| {
            serde_json::json!({"key": "rack_bought", "module_type": "rack", "grade": 1,
                "selector": {"items": ["int_rack_size5_class1"]}, "journal_blueprints": journal,
                "source": "synthetic fixture", "verified": false})
        };
        let cat = catalogue(
            vec![blueprint_row(
                "rack_upgrade",
                vec![context(
                    "rack",
                    serde_json::Value::Null,
                    existing(vec![]),
                    2..=5,
                )],
            )],
            vec![variant(vec![])],
            vec![],
        );
        assert_eq!(cat.variants_for("rack").len(), 1);
        // The variant key is not a recipe: no free blueprint, no installed identity.
        assert_eq!(
            cat.blueprint_for_item("int_rack_size5_class1", "rack_bought", None)
                .unwrap_err(),
            E::UnknownBlueprint
        );
        assert!(
            cat.blueprints_for_item("int_rack_size5_class1", None)
                .iter()
                .all(|o| o.blueprint.key != "rack_bought")
        );
        let installed = eng("rack_bought", 1, 1.0);
        assert!(
            cat.variant_of("int_rack_size5_class1", &installed)
                .is_none()
        );
        // The pending upgrade stays listed with its reason, never available.
        let options = cat.blueprints_for_item("int_rack_size5_class1", Some(&installed));
        assert_eq!(options.len(), 1);
        assert_eq!(
            options[0].availability.as_ref().unwrap_err(),
            &E::MissingBaseMapping
        );
        // Once a verified journal name exists the variant is recognised, still without a recipe.
        let cat = catalogue(vec![], vec![variant(vec!["Rack_Bought_Journal"])], vec![]);
        let seen = eng("rack_bought_journal", 1, 0.0);
        assert_eq!(
            cat.variant_of("int_rack_size5_class1", &seen).unwrap().key,
            "rack_bought"
        );
        assert!(cat.variant_of("int_rack_size6_class1", &seen).is_none());
        assert!(
            cat.variant_of("int_rack_size5_class1", &eng("rack_bought_journal", 2, 0.0))
                .is_none()
        );
    }

    #[test]
    fn synthetic_base_to_recipe_transition() {
        let build = |bases: Vec<serde_json::Value>, reverse: bool| {
            let mut blueprints = vec![blueprint_row(
                "upgrade_test",
                vec![context(
                    "rack",
                    serde_json::Value::Null,
                    existing(bases),
                    2..=5,
                )],
            )];
            if reverse {
                blueprints.insert(
                    0,
                    blueprint_row(
                        "filler",
                        vec![context("rack", serde_json::Value::Null, stock(), 1..=5)],
                    ),
                );
            }
            catalogue(
                blueprints,
                vec![
                    serde_json::json!({"key": "variant_test", "module_type": "rack", "grade": 1,
                    "selector": {"items": ["int_rack_size5_class1"]},
                    "journal_blueprints": ["variant_test_journal"],
                    "source": "synthetic fixture", "verified": false}),
                ],
                vec![],
            )
        };
        let link = base(
            serde_json::json!({"variant": {"key": "variant_test"}}),
            1,
            &["int_rack_size5_class1"],
        );
        let item = "int_rack_size5_class1";
        for reverse in [false, true] {
            let cat = build(vec![link.clone()], reverse);
            let ok = eng("Variant_Test_Journal", 1, 1.0);
            let (bp, rec) = cat
                .blueprint_for_item(item, "upgrade_test", Some(&ok))
                .unwrap();
            assert_eq!(bp.key, "upgrade_test");
            assert_eq!(rec.grades.keys().next().copied(), Some(2));
            let required = |cur: Option<&CurrentEng>| {
                cat.blueprint_for_item(item, "upgrade_test", cur)
                    .unwrap_err()
            };
            assert_eq!(required(None), E::BaseRequired);
            assert_eq!(
                required(Some(&eng("Variant_Test_Journal", 1, 0.5))),
                E::BaseRequired
            );
            assert_eq!(required(Some(&eng("Other", 1, 1.0))), E::BaseRequired);
            // Same variant on a different symbol does not satisfy the selector.
            assert_eq!(
                cat.blueprint_for_item("int_rack_size6_class1", "upgrade_test", Some(&ok))
                    .unwrap_err(),
                E::BaseRequired
            );
            // Without the link nothing is inferred from names or contiguous grades.
            let cat = build(vec![], reverse);
            assert_eq!(
                cat.blueprint_for_item(item, "upgrade_test", Some(&ok))
                    .unwrap_err(),
                E::MissingBaseMapping
            );
        }
        // A base without row evidence or symbols is an invalid context, not a loose link.
        let mut bad = link.clone();
        bad["source_rows"] = serde_json::json!([]);
        assert!(build(vec![bad], false).blueprint("upgrade_test").is_none());
        let mut bad = link;
        bad["selector"] = serde_json::json!({});
        assert!(build(vec![bad], false).blueprint("upgrade_test").is_none());
    }

    #[test]
    fn blueprint_base_continues_an_applied_upgrade() {
        let identity = serde_json::json!({"blueprint": {"key": "up"}});
        let cat = catalogue(
            vec![blueprint_row(
                "up",
                vec![context(
                    "rack",
                    serde_json::Value::Null,
                    existing(vec![base(identity, 2, &["int_rack_size5_class1"])]),
                    2..=5,
                )],
            )],
            vec![],
            vec![],
        );
        let item = "int_rack_size5_class1";
        assert!(
            cat.blueprint_for_item(item, "UP", Some(&eng("up", 2, 1.0)))
                .is_ok()
        );
        assert!(
            cat.blueprint_for_item(item, "up", Some(&eng("up", 3, 0.1)))
                .is_ok()
        );
        assert_eq!(
            cat.blueprint_for_item(item, "up", Some(&eng("up", 2, 0.998)))
                .unwrap_err(),
            E::BaseRequired
        );
        assert_eq!(
            cat.blueprint_for_item(item, "up", Some(&eng("up", 2, f64::NAN)))
                .unwrap_err(),
            E::BaseRequired
        );
    }

    /// Provisional row shape (`module_types` + `grades`, `ingredients`): still accepted on load.
    const LEGACY_CATALOGUE: &str = r#"{
        "module_types": [{"id": "engine", "item_prefixes": ["int_engine_"]}, {"id": "fsd", "item_prefixes": ["int_hyperdrive_"]}],
        "blueprints": [{"key": "engine_dirty", "module_types": ["engine"], "source": "t", "verified": false,
            "grades": {"1": [{"material": "legacyfirmware", "count": 1}], "2": [{"material": "iron", "count": 1}],
                       "3": [{"material": "iron", "count": 1}], "4": [{"material": "iron", "count": 1}], "5": [{"material": "iron", "count": 1}]}}],
        "experimentals": [{"key": "special_engine_overloaded", "module_types": ["engine"], "source": "t",
            "ingredients": [{"material": "iron", "count": 5}]}]
    }"#;

    #[test]
    fn legacy_rows_still_load_as_stock_contexts() {
        let cat = parse_blueprints(LEGACY_CATALOGUE);
        let dirty = cat.blueprint("engine_dirty").unwrap();
        assert_eq!(dirty.module_types(), ["engine"]);
        assert_eq!(dirty.recipes[0].entry, EntryPolicy::Stock);
        assert_eq!(dirty.recipes[0].grades[&1].merc_coins, 0);
        let (bp, rec) = cat
            .blueprint_for_item("Int_Engine_Size3_Class5", "ENGINE_DIRTY", None)
            .unwrap();
        assert_eq!((bp.key.as_str(), rec.grades.len()), ("engine_dirty", 5));
        assert_eq!(
            cat.blueprint_for_item("int_hyperdrive_size5_class5", "engine_dirty", None)
                .unwrap_err(),
            E::IncompatibleItem
        );
        let options = cat.blueprints_for_item("int_engine_size3_class5", None);
        assert!(options.iter().all(|o| o.availability.is_ok()));
        assert!(options.iter().any(|o| o.blueprint.key == "engine_dirty"));
        assert!(cat.audit().is_empty(), "{:?}", cat.audit());
        assert!(
            !cat.experimental("special_engine_overloaded")
                .unwrap()
                .cost
                .materials
                .is_empty()
        );
    }

    #[test]
    fn active_catalogue_parses_without_losing_rows() {
        let blueprints_json = include_str!("../data/blueprints.json");
        let symbols_json = include_str!("../data/import/engineering/staging/symbols.json");
        let raw: serde_json::Value = serde_json::from_str(blueprints_json).unwrap();
        let rows = |k: &str| raw[k].as_array().unwrap().len();
        let warnings = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cat = tracing::subscriber::with_default(WarningCounter(warnings.clone()), || {
            parse_blueprints(blueprints_json)
        });
        assert_eq!(warnings.load(std::sync::atomic::Ordering::Relaxed), 0);
        // Final numbers fixed after the full audit (manifest `summary`).
        assert_eq!(
            (
                cat.module_types.len(),
                cat.blueprints.len(),
                cat.experimentals.len(),
                cat.variants.len(),
                cat.aliases.len()
            ),
            (47, 89, 87, 15, 382)
        );
        assert_eq!(cat.module_types.len(), rows("module_types"));
        assert_eq!(cat.blueprints.len(), rows("blueprints"));
        assert_eq!(cat.experimentals.len(), rows("experimentals"));
        assert_eq!(cat.variants.len(), rows("variants"));
        assert_eq!(cat.aliases.len(), rows("aliases"));
        let contexts: usize = raw["blueprints"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["recipes"].as_array().unwrap().len())
            .sum();
        let grade_rows: usize = cat
            .blueprints
            .iter()
            .flat_map(|b| &b.recipes)
            .map(|r| r.grades.len())
            .sum();
        assert_eq!(
            cat.blueprints
                .iter()
                .map(|b| b.recipes.len())
                .sum::<usize>(),
            contexts
        );
        assert_eq!((contexts, grade_rows), (207, 987));
        // The alias table of the staging symbols is the one embedded in the active catalogue.
        let symbols: serde_json::Value = serde_json::from_str(symbols_json).unwrap();
        assert_eq!(symbols["aliases"], raw["aliases"]);
        // Merc Coins survive, ExistingOnly contexts stay pending without invented bases.
        let abrasion = cat.blueprint("abrasionblaster_farreaching").unwrap();
        let coins: Vec<u32> = abrasion.recipes[0]
            .grades
            .values()
            .map(|c| c.merc_coins)
            .collect();
        assert_eq!(coins, [5, 5, 10, 20]);
        assert_eq!(
            abrasion.recipes[0].entry,
            EntryPolicy::ExistingOnly { bases: vec![] }
        );
        // Operations rows resolved by the documented override keep their coins.
        let dss = cat.blueprint("detailedsurfacescanner_longrange").unwrap();
        assert_eq!(dss.module_types(), ["surfacescanner"]);
        let coins: Vec<u32> = dss.recipes[0]
            .grades
            .values()
            .map(|c| c.merc_coins)
            .collect();
        assert_eq!(coins, [5, 5, 10, 20]);
        assert!(!dss.verified);
        for key in [
            "special_weapon_toughened",
            "special_weapon_efficient",
            "special_weapon_damage",
            "special_weapon_lightweight",
        ] {
            let e = cat.experimental(key).unwrap();
            assert_eq!(e.module_types.len(), 11, "{key}");
            assert!(!e.verified && e.source.contains("2026-10-22-filas-sin-resolver-c8"));
        }
        // Symbols proven for the active modules keep resolving; pending types match nothing.
        assert_eq!(
            cat.module_type_of("int_engine_size3_class5"),
            Some("engine")
        );
        assert_eq!(
            cat.module_type_of("int_hyperdrive_size5_class5"),
            Some("fsd")
        );
        assert!(
            cat.blueprint_for_item("int_engine_size3_class5", "engine_dirty", None)
                .is_ok()
        );
        assert_eq!(
            cat.blueprint_for_item(
                "int_engine_size3_class5",
                "abrasionblaster_farreaching",
                None
            )
            .unwrap_err(),
            E::IncompatibleItem
        );
        // Variants are kept visible and none resolves to a recipe or an installed identity.
        assert!(cat.variants.iter().all(|v| v.journal_blueprints.is_empty()));
        // MissileRack / SeekerMissileRack array expansion keeps both modules.
        let highcap = cat.blueprint("weapon_highcapacity").unwrap();
        assert!(
            highcap.has_module_type("missilerack") && highcap.has_module_type("seekermissilerack")
        );
        let report = cat.audit();
        assert!(report.is_empty(), "{report:?}");
    }

    #[test]
    fn dangling_aliases_neither_block_a_valid_alias_nor_hide_from_the_audit() {
        let cat = parse_blueprints(
            r#"{
            "module_types": [{"id": "engine", "item_prefixes": ["int_engine_"]}],
            "blueprints": [{"key": "engine_dirty", "source": "t",
                "recipes": [{"module_type": "engine", "entry": "stock",
                    "grades": {"1": {"materials": [{"material": "iron", "count": 1}]}}}]}],
            "aliases": [
                {"module_type": "engine", "alias": "dt", "canonical": "ghost_one"},
                {"module_type": "engine", "alias": "dt", "canonical": "engine_dirty"},
                {"module_type": "engine", "alias": "xx", "canonical": "ghost_two"}
            ]}"#,
        );
        assert_eq!(
            cat.canonical_blueprint("engine", "dt"),
            Some("engine_dirty")
        );
        assert_eq!(cat.canonical_blueprint("engine", "xx"), None);
        let report = cat.audit();
        // Both dangling targets and the (now visible) ambiguity are reported, none aborts the report.
        assert_eq!(report.len(), 3, "{report:?}");
        assert!(
            report
                .iter()
                .any(|r| r.contains("ambiguous alias engine/dt"))
        );
        assert!(report.iter().any(|r| r.contains("ghost_one")));
        assert!(report.iter().any(|r| r.contains("ghost_two")));
    }
}
