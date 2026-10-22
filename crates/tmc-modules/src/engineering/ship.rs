//! Mi nave modificada: ingeniería actual de cada módulo a partir de `Loadout`.

use serde::{Deserialize, Serialize};
use tmc_model::events::{EngineerCraft, Loadout};

use crate::data::{BlueprintCatalog, blueprints};

/// Ingeniería aplicada a un módulo. `blueprint` conserva la clave del diario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentEng {
    pub blueprint: String,
    pub level: u8,
    pub quality: f64,
    pub experimental: Option<String>,
}

impl CurrentEng {
    /// Clave canónica del blueprint aplicado dentro de un tipo de módulo; `None` si el
    /// texto del diario no se reconoce (la clave original se conserva en `blueprint`).
    pub fn canonical_blueprint<'a>(
        &self,
        catalogue: &'a BlueprintCatalog,
        module_type: &str,
    ) -> Option<&'a str> {
        catalogue.canonical_blueprint(module_type, &self.blueprint)
    }
}

/// Módulo instalado. `current == None` significa «de serie».
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ShipModuleRow {
    pub slot: String,
    pub item: String,
    pub module_type: Option<String>,
    pub current: Option<CurrentEng>,
    pub engineerable: bool,
}

/// Vista de la nave. No incluye `ship_name` ni `ship_ident` (privacidad).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ShipView {
    pub ship_id: Option<u64>,
    pub ship: Option<String>,
    pub modules: Vec<ShipModuleRow>,
}

const COSMETIC_PREFIXES: &[&str] = &[
    "paintjob_",
    "decal_",
    "nameplate_",
    "voicepack_",
    "enginecustomisation_",
    "weaponcustomisation_",
    "bobble",
    "string_lights",
    "shipkit",
];

fn is_cosmetic(item: &str) -> bool {
    let item = item.to_ascii_lowercase();
    COSMETIC_PREFIXES.iter().any(|p| item.starts_with(p))
}

impl ShipView {
    pub fn from_loadout(l: &Loadout) -> ShipView {
        let cat = blueprints();
        let modules = l
            .modules
            .iter()
            .filter_map(|m| {
                let slot = m.slot.clone()?;
                let item = m.item.clone().unwrap_or_default();
                let module_type = cat.module_type_of(&item).map(str::to_owned);
                let engineerable = !is_cosmetic(&item) && module_type.is_some();
                let current = m.engineering.as_ref().and_then(|e| {
                    Some(CurrentEng {
                        blueprint: e.blueprint_name.clone()?,
                        level: e.level.unwrap_or(0).min(5) as u8,
                        quality: e.quality.unwrap_or(0.0),
                        experimental: e
                            .experimental_effect
                            .clone()
                            .or_else(|| e.experimental_effect_localised.clone()),
                    })
                });
                Some(ShipModuleRow {
                    slot,
                    item,
                    module_type,
                    current,
                    engineerable,
                })
            })
            .collect();
        ShipView {
            ship_id: l.ship_id,
            ship: l.ship.clone(),
            modules,
        }
    }

    /// Refleja una mejora hasta el siguiente `Loadout`. Ranura desconocida: se ignora.
    pub fn apply_craft(&mut self, c: &EngineerCraft) {
        let Some(slot) = c.slot.as_deref() else {
            return;
        };
        if let Some(row) = self
            .modules
            .iter_mut()
            .find(|r| r.slot.eq_ignore_ascii_case(slot))
        {
            row.current = Some(CurrentEng {
                blueprint: c.blueprint_name.clone(),
                level: c.level.min(5),
                quality: c.quality.unwrap_or(0.0),
                experimental: c
                    .apply_experimental_effect
                    .clone()
                    .or_else(|| c.experimental_effect.clone())
                    .or_else(|| row.current.as_ref().and_then(|e| e.experimental.clone())),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmc_model::events::{EventKind, parse_line};

    const FIXTURE: &str =
        include_str!("../../../../tests/fixtures/journal/engineering_session.log");

    fn loadout() -> Loadout {
        FIXTURE
            .lines()
            .find_map(|l| match parse_line(l).map(|e| e.kind) {
                Ok(EventKind::Loadout(l)) => Some(l),
                _ => None,
            })
            .expect("fixture has a Loadout")
    }

    #[test]
    fn stock_and_engineered_modules() {
        let v = ShipView::from_loadout(&loadout());
        assert_eq!(v.ship_id, Some(7));
        let engine = &v.modules[0];
        assert_eq!(engine.slot, "MainEngines");
        assert_eq!(engine.module_type.as_deref(), Some("engine"));
        assert_eq!(engine.current, None);
        assert!(engine.engineerable);
        let fsd = &v.modules[1];
        let cur = fsd.current.as_ref().unwrap();
        assert_eq!(cur.level, 3);
        assert_eq!(cur.quality, 1.0);
        assert_eq!(cur.blueprint, "FSD_LongRange");
    }

    #[test]
    fn craft_updates_slot_until_next_loadout() {
        let mut v = ShipView::from_loadout(&loadout());
        let c: EngineerCraft = serde_json::from_str(
            r#"{"Engineer":"Felicity Farseer","EngineerID":300100,"BlueprintName":"Engine_Dirty","Level":1,"Slot":"MainEngines","Quality":0.5}"#,
        )
        .unwrap();
        v.apply_craft(&c);
        let cur = v.modules[0].current.as_ref().unwrap();
        assert_eq!((cur.level, cur.blueprint.as_str()), (1, "Engine_Dirty"));
        let fresh = ShipView::from_loadout(&loadout());
        assert_eq!(fresh.modules[0].current, None);
    }

    fn craft(json: &str) -> EngineerCraft {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn craft_unknown_slot_is_ignored() {
        let mut v = ShipView::from_loadout(&loadout());
        let before = v.clone();
        v.apply_craft(&craft(
            r#"{"Engineer":"F","EngineerID":1,"BlueprintName":"Engine_Dirty","Level":2,"Slot":"Nope"}"#,
        ));
        assert_eq!(v, before);
    }

    #[test]
    fn craft_level_is_clamped_to_five() {
        let mut v = ShipView::from_loadout(&loadout());
        v.apply_craft(&craft(
            r#"{"Engineer":"F","EngineerID":1,"BlueprintName":"Engine_Dirty","Level":9,"Slot":"MainEngines"}"#,
        ));
        assert_eq!(v.modules[0].current.as_ref().unwrap().level, 5);
    }

    #[test]
    fn craft_preserves_experimental_when_absent() {
        let mut v = ShipView::from_loadout(&loadout());
        v.modules[0].current = Some(CurrentEng {
            blueprint: "Engine_Dirty".into(),
            level: 4,
            quality: 1.0,
            experimental: Some("special_engine_overloaded".into()),
        });
        v.apply_craft(&craft(
            r#"{"Engineer":"F","EngineerID":1,"BlueprintName":"Engine_Dirty","Level":5,"Slot":"mainengines"}"#,
        ));
        let cur = v.modules[0].current.as_ref().unwrap();
        assert_eq!(cur.level, 5);
        assert_eq!(
            cur.experimental.as_deref(),
            Some("special_engine_overloaded")
        );
    }

    #[test]
    fn unknown_blueprint_key_is_kept_verbatim() {
        let l: Loadout = serde_json::from_str(
            r#"{"Ship":"asp","ShipID":1,"Modules":[{"Slot":"MainEngines","Item":"int_engine_size3_class5","Engineering":{"BlueprintName":"Engine_Mystery","Level":2,"Quality":0.3}}]}"#,
        )
        .unwrap();
        let v = ShipView::from_loadout(&l);
        let cur = v.modules[0].current.as_ref().unwrap();
        assert_eq!((cur.blueprint.as_str(), cur.level), ("Engine_Mystery", 2));
    }

    #[test]
    fn current_blueprint_canonicalises_per_module_without_rewriting_raw() {
        let cat = blueprints();
        let cur = CurrentEng {
            blueprint: "ENGINE_DIRTY".into(),
            level: 2,
            quality: 0.5,
            experimental: None,
        };
        assert_eq!(cur.canonical_blueprint(cat, "engine"), Some("engine_dirty"));
        assert_eq!(cur.canonical_blueprint(cat, "fsd"), None);
        assert_eq!(cur.blueprint, "ENGINE_DIRTY");
        let unknown = CurrentEng {
            blueprint: "Engine_Mystery".into(),
            ..cur
        };
        assert_eq!(unknown.canonical_blueprint(cat, "engine"), None);
    }

    #[test]
    fn mixed_case_item_resolves_module_type() {
        let l: Loadout = serde_json::from_str(
            r#"{"Ship":"asp","ShipID":1,"Modules":[{"Slot":"MainEngines","Item":"Int_Engine_Size3_Class5"}]}"#,
        )
        .unwrap();
        let v = ShipView::from_loadout(&l);
        assert_eq!(v.modules[0].module_type.as_deref(), Some("engine"));
        assert!(v.modules[0].engineerable);
    }

    #[test]
    fn cosmetic_and_unknown_items_not_engineerable() {
        let l: Loadout = serde_json::from_str(
            r#"{"Ship":"asp","ShipID":1,"ShipName":"X","Modules":[
            {"Slot":"PaintJob","Item":"paintjob_asp_default_01"},
            {"Slot":"ShipCockpit","Item":"decal_explorer_elite"},
            {"Slot":"Slot01_Size3","Item":"int_mystery_size3_class1"}]}"#,
        )
        .unwrap();
        let v = ShipView::from_loadout(&l);
        assert!(v.modules.iter().all(|m| !m.engineerable));
        assert_eq!(v.modules[2].module_type, None);
    }
}
