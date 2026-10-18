//! Datos estáticos del juego: tabla de FSD y clases de estrella.

use crate::exobiology::predict::BioRules;
use crate::exobiology::species::SpeciesTable;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tmc_model::events::AsteroidContent;
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
        self.entries.iter().find(|e| e.ship == ship)
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
        assert_eq!(p.entries.len(), 46);
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
}
