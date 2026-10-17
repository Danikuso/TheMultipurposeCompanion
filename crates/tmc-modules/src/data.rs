//! Datos estáticos del juego: tabla de FSD y clases de estrella.

use serde::Deserialize;
use std::sync::OnceLock;

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
        serde_json::from_str(include_str!("../data/fsd.json")).expect("fsd.json embebido válido")
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarSpecial {
    Neutron,
    WhiteDwarf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarFlags {
    pub scoopable: bool,
    pub special: Option<StarSpecial>,
}

/// Propiedades de una clase de estrella del diario (`StarType`).
pub fn star_flags(class: &str) -> StarFlags {
    let first = class.chars().next().map(|c| c.to_ascii_uppercase());
    let special = match first {
        Some('N') => Some(StarSpecial::Neutron),
        Some('D') => Some(StarSpecial::WhiteDwarf),
        _ => None,
    };
    StarFlags {
        scoopable: first.is_some_and(|c| "KGBFOAM".contains(c)),
        special,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_flags_classes() {
        assert!(star_flags("K").scoopable);
        assert_eq!(star_flags("N").special, Some(StarSpecial::Neutron));
        assert_eq!(star_flags("DA").special, Some(StarSpecial::WhiteDwarf));
        assert!(!star_flags("DA").scoopable);
        assert!(!star_flags("N").scoopable);
        assert!(!star_flags("H").scoopable);
        assert_eq!(star_flags("M").special, None);
    }

    #[test]
    fn fsd_table_loads_and_has_sco_and_mk2_entries() {
        let t = fsd_table();
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
}
