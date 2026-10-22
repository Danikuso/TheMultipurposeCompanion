//! Filas planas de catalogo, ingenieros y flujo para el REST paginado.
//!
//! Cada fila describe un hecho (receta por grado, capacidad por grado, coste por material)
//! para que ninguna lista crezca sin limite y la paginacion por bytes pueda servirla.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde_json::{Value, json};

use super::plan::PlanCost;
use super::ship::CurrentEng;
use crate::data::{
    BaseIdentity, BlueprintCatalog, EngineerDef, EntryPolicy, Ingredient, ModuleSelector,
    RecipeCost, blueprints, engineers,
};

pub(super) type Pages = BTreeMap<String, Vec<Value>>;

pub(super) const CATALOGUE_SECTIONS: [&str; 6] = [
    "module_types",
    "blueprints",
    "recipes",
    "experimentals",
    "variants",
    "aliases",
];
pub(super) const ENGINEER_SECTIONS: [&str; 3] = ["engineers", "capabilities", "unlock"];
pub(super) const FLOW_SECTIONS: [&str; 5] =
    ["blueprints", "experimentals", "grades", "costs", "lines"];

fn selector(s: &ModuleSelector) -> Value {
    json!({"items": s.items, "item_prefixes": s.item_prefixes})
}

fn materials(list: &[Ingredient]) -> Vec<Value> {
    list.iter()
        .map(|i| json!({"material": i.material, "count": i.count}))
        .collect()
}

fn cost_fields(cost: &RecipeCost) -> (Vec<Value>, u32) {
    (materials(&cost.materials), cost.merc_coins)
}

pub(super) fn catalogue_rows(cat: &BlueprintCatalog) -> Pages {
    let mut out = Pages::new();
    out.insert(
        "module_types".into(),
        cat.module_types
            .iter()
            .map(|t| json!({"id": t.id, "items": t.items, "item_prefixes": t.item_prefixes}))
            .collect(),
    );
    out.insert(
        "blueprints".into(),
        cat.blueprints
            .iter()
            .map(|b| {
                json!({
                    "key": b.key, "module_types": b.module_types(), "max_grade": b.max_grade(),
                    "source": b.source, "verified": b.verified,
                })
            })
            .collect(),
    );
    let mut recipes = Vec::new();
    for b in &cat.blueprints {
        for r in &b.recipes {
            let (entry, bases) = match &r.entry {
                EntryPolicy::Stock => ("stock", Vec::new()),
                EntryPolicy::ExistingOnly { bases } => (
                    "existing_only",
                    bases
                        .iter()
                        .map(|base| {
                            let (kind, key) = match &base.identity {
                                BaseIdentity::Blueprint { key } => ("blueprint", key),
                                BaseIdentity::Variant { key } => ("variant", key),
                            };
                            json!({
                                "kind": kind, "key": key,
                                "completed_grade": base.completed_grade,
                                "selector": selector(&base.selector),
                            })
                        })
                        .collect(),
                ),
            };
            for (grade, cost) in &r.grades {
                let (mats, coins) = cost_fields(cost);
                recipes.push(json!({
                    "blueprint": b.key, "module_type": r.module_type,
                    "selector": r.selector.as_ref().map(selector),
                    "entry": entry, "bases": bases, "grade": grade,
                    "materials": mats, "merc_coins": coins,
                }));
            }
        }
    }
    out.insert("recipes".into(), recipes);
    out.insert(
        "experimentals".into(),
        cat.experimentals
            .iter()
            .map(|e| {
                let (mats, coins) = cost_fields(&e.cost);
                json!({
                    "key": e.key, "module_types": e.module_types, "materials": mats,
                    "merc_coins": coins, "source": e.source, "verified": e.verified,
                })
            })
            .collect(),
    );
    out.insert(
        "variants".into(),
        cat.variants
            .iter()
            .map(|v| {
                json!({
                    "key": v.key, "module_type": v.module_type, "grade": v.grade,
                    "selector": v.selector.as_ref().map(selector),
                    "journal_blueprints": v.journal_blueprints,
                    "source": v.source, "verified": v.verified,
                })
            })
            .collect(),
    );
    out.insert(
        "aliases".into(),
        cat.aliases
            .iter()
            .map(|a| {
                json!({"module_type": a.module_type, "alias": a.alias, "canonical": a.canonical})
            })
            .collect(),
    );
    out
}

/// Static catalogue rows, built once and shared by every client and module instance.
pub(super) fn catalogue_pages() -> &'static Pages {
    static PAGES: OnceLock<Pages> = OnceLock::new();
    PAGES.get_or_init(|| catalogue_rows(blueprints()))
}

pub(super) fn engineer_static_rows(defs: &[EngineerDef]) -> Pages {
    let mut capabilities = Vec::new();
    let mut unlock = Vec::new();
    for e in defs {
        for b in &e.blueprints {
            for grade in &b.grades {
                capabilities.push(json!({
                    "engineer_id": e.id, "module_type": b.module_type,
                    "blueprint": b.blueprint, "grade": grade,
                }));
            }
        }
        unlock.push(
            json!({"engineer_id": e.id, "field": "requirement", "key": e.unlock.requirement_key}),
        );
        if let Some(key) = &e.unlock.invite_key {
            unlock.push(json!({"engineer_id": e.id, "field": "invite", "key": key}));
        }
        if let Some(c) = &e.unlock.contribution {
            unlock.push(json!({
                "engineer_id": e.id, "field": "contribution",
                "item": c.item, "kind": c.kind, "amount": c.amount,
            }));
        }
    }
    Pages::from([
        ("capabilities".to_owned(), capabilities),
        ("unlock".to_owned(), unlock),
    ])
}

pub(super) fn engineer_static_pages() -> &'static Pages {
    static PAGES: OnceLock<Pages> = OnceLock::new();
    PAGES.get_or_init(|| engineer_static_rows(engineers()))
}

/// Flow rows for the open flow: options for the slot's module, applicable grades and
/// per-grade costs. `lines` are the rows already rendered for the topic.
pub(super) fn flow_pages(
    cat: &BlueprintCatalog,
    slot: Option<(&str, Option<&str>, Option<&CurrentEng>)>,
    cost: Option<&PlanCost>,
    lines: &Value,
) -> Pages {
    let mut out: Pages = FLOW_SECTIONS
        .iter()
        .map(|s| ((*s).to_owned(), Vec::new()))
        .collect();
    if let Some((item, Some(module_type), current)) = slot {
        out.insert(
            "blueprints".into(),
            cat.blueprints_for_item(item, current)
                .iter()
                .map(|o| {
                    json!({
                        "key": o.blueprint.key, "available": o.availability.is_ok(),
                        "reason": o.availability.as_ref().err(),
                    })
                })
                .collect(),
        );
        out.insert(
            "experimentals".into(),
            cat.experimentals_for(module_type)
                .iter()
                .map(|e| json!({"key": e.key}))
                .collect(),
        );
    }
    if let Some(cost) = cost.filter(|c| !c.has_resolution_blocker()) {
        let mut grades = Vec::new();
        let mut costs = Vec::new();
        for (grade, (rolls, mats)) in &cost.per_grade {
            let coins = cost.merc_coins_by_grade.get(grade).copied().unwrap_or(0);
            grades.push(json!({"grade": grade, "rolls": rolls, "merc_coins": coins}));
            for (material, count) in mats {
                costs.push(
                    json!({"grade": grade, "kind": "material", "material": material, "count": count}),
                );
            }
            if coins > 0 {
                costs.push(json!({"grade": grade, "kind": "merc_coins", "count": coins}));
            }
        }
        for (material, count) in cost.experimental.iter().flatten() {
            costs.push(
                json!({"grade": null, "kind": "material", "material": material, "count": count}),
            );
        }
        if cost.merc_coins_experimental > 0 {
            costs.push(
                json!({"grade": null, "kind": "merc_coins", "count": cost.merc_coins_experimental}),
            );
        }
        out.insert("grades".into(), grades);
        out.insert("costs".into(), costs);
    }
    out.insert(
        "lines".into(),
        lines.as_array().cloned().unwrap_or_default(),
    );
    out
}
