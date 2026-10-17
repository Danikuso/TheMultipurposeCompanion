//! Archivos compañeros del diario (Cargo.json, NavRoute.json, Market.json…).

use crate::ParseError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompanionKind {
    Cargo,
    Market,
    Outfitting,
    Shipyard,
    ModulesInfo,
    NavRoute,
    Backpack,
    ShipLocker,
    FcMaterials,
}

impl CompanionKind {
    pub fn file_name(self) -> &'static str {
        match self {
            Self::Cargo => "Cargo.json",
            Self::Market => "Market.json",
            Self::Outfitting => "Outfitting.json",
            Self::Shipyard => "Shipyard.json",
            Self::ModulesInfo => "ModulesInfo.json",
            Self::NavRoute => "NavRoute.json",
            Self::Backpack => "Backpack.json",
            Self::ShipLocker => "ShipLocker.json",
            Self::FcMaterials => "FCMaterials.json",
        }
    }

    /// Archivo compañero que el juego reescribe tras el evento de diario `name`.
    pub fn for_event(name: &str) -> Option<Self> {
        Some(match name {
            "Cargo" => Self::Cargo,
            "Market" => Self::Market,
            "Outfitting" => Self::Outfitting,
            "Shipyard" => Self::Shipyard,
            "ModuleInfo" => Self::ModulesInfo,
            "NavRoute" => Self::NavRoute,
            "Backpack" => Self::Backpack,
            "ShipLocker" => Self::ShipLocker,
            "FCMaterials" => Self::FcMaterials,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CargoItem {
    pub name: String,
    #[serde(rename = "Name_Localised")]
    pub name_localised: Option<String>,
    pub count: u32,
    pub stolen: u32,
    #[serde(rename = "MissionID")]
    pub mission_id: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CargoFile {
    pub vessel: Option<String>,
    pub count: u32,
    pub inventory: Vec<CargoItem>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct RouteHop {
    pub star_system: String,
    pub system_address: u64,
    pub star_pos: [f64; 3],
    pub star_class: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct NavRouteFile {
    pub route: Vec<RouteHop>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompanionFile {
    Cargo(CargoFile),
    NavRoute(NavRouteFile),
    Raw(CompanionKind, serde_json::Value),
}

/// Analiza el contenido de un archivo compañero (solo Cargo y NavRoute son tipados).
pub fn parse(kind: CompanionKind, text: &str) -> Result<CompanionFile, ParseError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(ParseError::Empty);
    }
    let json_err = |e: serde_json::Error| ParseError::Json(e.to_string());
    Ok(match kind {
        CompanionKind::Cargo => CompanionFile::Cargo(serde_json::from_str(text).map_err(json_err)?),
        CompanionKind::NavRoute => {
            CompanionFile::NavRoute(serde_json::from_str(text).map_err(json_err)?)
        }
        other => CompanionFile::Raw(other, serde_json::from_str(text).map_err(json_err)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn companion_kind_for_event_maps_moduleinfo() {
        assert_eq!(
            CompanionKind::for_event("ModuleInfo"),
            Some(CompanionKind::ModulesInfo)
        );
        assert_eq!(CompanionKind::for_event("FSDJump"), None);
        assert_eq!(CompanionKind::FcMaterials.file_name(), "FCMaterials.json");
    }

    #[test]
    fn parses_cargo_and_navroute_files() {
        let c = parse(
            CompanionKind::Cargo,
            r#"{"timestamp":"2026-03-05T18:00:14Z","event":"Cargo","Vessel":"Ship","Count":3,"Inventory":[{"Name":"gold","Name_Localised":"Gold","Count":3,"Stolen":0}]}"#,
        )
        .unwrap();
        match c {
            CompanionFile::Cargo(f) => {
                assert_eq!(f.count, 3);
                assert_eq!(f.inventory[0].name, "gold");
                assert_eq!(f.inventory[0].name_localised.as_deref(), Some("Gold"));
            }
            other => panic!("{other:?}"),
        }
        let n = parse(
            CompanionKind::NavRoute,
            r#"{"timestamp":"2026-03-05T18:00:14Z","event":"NavRoute","Route":[{"StarSystem":"A","SystemAddress":1,"StarPos":[1.0,2.0,3.0],"StarClass":"K"}]}"#,
        )
        .unwrap();
        match n {
            CompanionFile::NavRoute(f) => assert_eq!(f.route[0].star_pos, [1.0, 2.0, 3.0]),
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            parse(CompanionKind::Market, "{}").unwrap(),
            CompanionFile::Raw(CompanionKind::Market, _)
        ));
        assert_eq!(
            parse(CompanionKind::Cargo, " ").unwrap_err(),
            crate::ParseError::Empty
        );
    }
}
