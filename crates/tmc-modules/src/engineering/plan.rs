//! Cálculo de materiales para ingenierizar un módulo: grado de partida, tiradas por grado y total.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::ship::CurrentEng;
use crate::data::{
    BaseIdentity, BaseRequirement, Blueprint, BlueprintCatalog, EntryPolicy, Ingredient,
    ModuleBlueprint, RecipeResolutionError,
};

/// Petición de cálculo. `rolls[i]` son las tiradas del grado `i + 1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanRequest {
    pub module_type: String,
    pub blueprint: String,
    pub experimental: Option<String>,
    pub target_grade: u8,
    pub rolls: [u8; 5],
}

/// Punto de partida: primer grado a tirar, tiradas restantes si está a medias y si
/// se sustituye un blueprint distinto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StartPoint {
    pub grade: u8,
    pub partial_rolls: Option<u8>,
    pub replaces: bool,
}

/// Coste por grado (`(tiradas, materiales)`), experimental y total.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct PlanCost {
    pub per_grade: BTreeMap<u8, (u8, BTreeMap<String, u32>)>,
    pub experimental: Option<BTreeMap<String, u32>>,
    pub total: BTreeMap<String, u32>,
    /// Merc Coins por grado (ya multiplicadas por las tiradas), solo grados con coste.
    /// Moneda aparte: nunca entra en `total` ni en el inventario.
    pub merc_coins_by_grade: BTreeMap<u8, u64>,
    pub merc_coins_experimental: u64,
    pub merc_coins_total: u64,
    /// Motivos por los que el cálculo no está completo; vacío = cálculo completo.
    pub blockers: Vec<PlanBlocker>,
    pub replaces_blueprint: bool,
    /// Dato sin verificar; no equivale a cálculo incompleto (ver `blockers`).
    pub verified: bool,
    /// Grados pedidos sin receta (derivado de `blockers`; se cuentan sin materiales).
    pub missing_grades: Vec<u8>,
}

/// Grado máximo definido para un blueprint (objetivo por defecto).
pub fn max_grade(bp: &Blueprint) -> u8 {
    bp.max_grade().unwrap_or(5)
}

pub fn start_grade(current: Option<&CurrentEng>, blueprint: &str, rolls: &[u8; 5]) -> StartPoint {
    let Some(cur) = current else {
        return StartPoint {
            grade: 1,
            partial_rolls: None,
            replaces: false,
        };
    };
    if !cur.blueprint.eq_ignore_ascii_case(blueprint) {
        return StartPoint {
            grade: 1,
            partial_rolls: None,
            replaces: true,
        };
    }
    let level = cur.level.min(5);
    if level == 0 {
        return StartPoint {
            grade: 1,
            partial_rolls: None,
            replaces: false,
        };
    }
    let q = if cur.quality.is_finite() {
        cur.quality.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if q >= 0.999 {
        return StartPoint {
            grade: level + 1,
            partial_rolls: None,
            replaces: false,
        };
    }
    let total = f64::from(rolls[usize::from(level) - 1]);
    let remaining = ((total * (1.0 - q)) - 1e-9).ceil().max(1.0) as u8;
    StartPoint {
        grade: level,
        partial_rolls: Some(remaining),
        replaces: false,
    }
}

fn add(into: &mut BTreeMap<String, u32>, list: &[Ingredient], times: u32) {
    for i in list {
        *into.entry(i.material.clone()).or_insert(0) += i.count * times;
    }
}

fn add_into_total(total: &mut BTreeMap<String, u32>, mats: &BTreeMap<String, u32>) {
    for (k, v) in mats {
        *total.entry(k.clone()).or_insert(0) += v;
    }
}

/// Why a calculation cannot be completed. Distinct from `verified: false`, which only says the
/// underlying data is not verified yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanBlocker {
    MissingRecipe { grade: u8 },
    MissingBaseMapping,
    BaseRequired,
    AmbiguousRecipe,
    IncompatibleItem,
    UnknownBlueprint,
    UnknownExperimental,
}

impl From<RecipeResolutionError> for PlanBlocker {
    fn from(e: RecipeResolutionError) -> Self {
        match e {
            RecipeResolutionError::UnknownBlueprint => Self::UnknownBlueprint,
            RecipeResolutionError::IncompatibleItem => Self::IncompatibleItem,
            RecipeResolutionError::MissingBaseMapping => Self::MissingBaseMapping,
            RecipeResolutionError::BaseRequired => Self::BaseRequired,
            RecipeResolutionError::AmbiguousRecipe => Self::AmbiguousRecipe,
        }
    }
}

impl PlanCost {
    /// Blockers that make the request itself unusable (anything but a gap in the recipe chain).
    pub fn has_resolution_blocker(&self) -> bool {
        self.blockers
            .iter()
            .any(|b| !matches!(b, PlanBlocker::MissingRecipe { .. }))
    }

    fn blocked(blocker: PlanBlocker) -> Self {
        Self {
            blockers: vec![blocker],
            ..Self::default()
        }
    }
}

fn add_coins(coins: u32, times: u32) -> u64 {
    u64::from(coins).saturating_mul(u64::from(times))
}

/// Start of an `ExistingOnly` route from a proven base. A base that is the target blueprint
/// itself keeps the ordinary grade/partial-quality rules; any other base starts right after
/// its completed grade, with no replacement warning (it is the approved transition).
fn base_start(
    base: &BaseRequirement,
    bp_key: &str,
    current: Option<&CurrentEng>,
    rolls: &[u8; 5],
) -> StartPoint {
    let same_identity = matches!(&base.identity, BaseIdentity::Blueprint { key } if key == bp_key);
    if same_identity {
        return start_grade(current, bp_key, rolls);
    }
    StartPoint {
        grade: base.completed_grade.saturating_add(1),
        partial_rolls: None,
        replaces: false,
    }
}

fn normalise(
    cat: &BlueprintCatalog,
    current: Option<&CurrentEng>,
    req: &PlanRequest,
) -> Option<CurrentEng> {
    current.map(|c| {
        let mut c = c.clone();
        if let Some(k) = cat.canonical_blueprint(&req.module_type, &c.blueprint) {
            c.blueprint = k.to_owned();
        }
        c
    })
}

/// Resolves the recipe context and the starting point of a request. Shared by the cost and by
/// the engineer recommendation so both see the same route (aliases canonicalised, base policy).
pub(crate) fn resolve_start<'a>(
    cat: &'a BlueprintCatalog,
    item: &str,
    current: Option<&CurrentEng>,
    req: &PlanRequest,
) -> Result<(&'a Blueprint, &'a ModuleBlueprint, StartPoint), PlanBlocker> {
    if cat.module_type_of(item) != Some(req.module_type.as_str()) {
        return Err(PlanBlocker::IncompatibleItem);
    }
    let (bp, rec) = cat
        .blueprint_for_item(item, &req.blueprint, current)
        .map_err(PlanBlocker::from)?;
    // Compare the installed blueprint through its canonical key, so journal aliases match.
    let normalised = normalise(cat, current, req);
    let start = match &rec.entry {
        EntryPolicy::Stock => start_grade(normalised.as_ref(), &bp.key, &req.rolls),
        EntryPolicy::ExistingOnly { .. } => {
            let mut starts: Vec<StartPoint> = Vec::new();
            for b in cat.satisfied_bases(rec, item, current) {
                let s = base_start(b, &bp.key, normalised.as_ref(), &req.rolls);
                if !starts.contains(&s) {
                    starts.push(s);
                }
            }
            match starts.as_slice() {
                [one] => *one,
                [] => return Err(PlanBlocker::BaseRequired),
                _ => return Err(PlanBlocker::AmbiguousRecipe),
            }
        }
    };
    Ok((bp, rec, start))
}

/// Grades an engineer must offer for the request: from the resolved start to the target.
/// Empty when already achieved. If the route cannot be resolved the whole `1..=target` route
/// is assumed. Experimentals never change it.
pub fn required_grades(
    cat: &BlueprintCatalog,
    item: &str,
    current: Option<&CurrentEng>,
    req: &PlanRequest,
) -> Vec<u8> {
    let start = resolve_start(cat, item, current, req).map_or(1, |(_, _, s)| s.grade);
    (start..=req.target_grade.min(5)).collect()
}

/// Materials and Merc Coins to engineer `item` as `req` says, starting from `current`.
/// The recipe context is resolved from item and installed base; when none or several apply the
/// result carries a blocker and no costs, never a silently chosen or free route.
pub fn plan_cost(
    cat: &BlueprintCatalog,
    item: &str,
    current: Option<&CurrentEng>,
    req: &PlanRequest,
) -> PlanCost {
    let (bp, rec, start) = match resolve_start(cat, item, current, req) {
        Ok(found) => found,
        Err(blocker) => return PlanCost::blocked(blocker),
    };
    let normalised = normalise(cat, current, req);
    let mut cost = PlanCost {
        replaces_blueprint: start.replaces,
        verified: bp.verified,
        ..PlanCost::default()
    };
    let target = req.target_grade.min(5);
    for g in start.grade..=target {
        let rolls = match start.partial_rolls {
            Some(p) if g == start.grade => p,
            _ => req.rolls[usize::from(g) - 1],
        };
        let mut mats = BTreeMap::new();
        match rec.grades.get(&g) {
            Some(recipe) => {
                add(&mut mats, &recipe.materials, u32::from(rolls));
                let coins = add_coins(recipe.merc_coins, u32::from(rolls));
                if coins > 0 {
                    cost.merc_coins_by_grade.insert(g, coins);
                    cost.merc_coins_total = cost.merc_coins_total.saturating_add(coins);
                }
            }
            None => cost.blockers.push(PlanBlocker::MissingRecipe { grade: g }),
        }
        add_into_total(&mut cost.total, &mats);
        cost.per_grade.insert(g, (rolls, mats));
    }
    if let Some(wanted) = req.experimental.as_deref() {
        let key = cat.canonical_experimental(&req.module_type, wanted);
        match key.and_then(|k| cat.experimental(k)) {
            Some(e) => {
                cost.verified &= e.verified;
                let installed = normalised.as_ref().and_then(|c| c.experimental.as_deref());
                let same = !start.replaces
                    && installed.is_some_and(|i| {
                        cat.canonical_experimental(&req.module_type, i)
                            .map_or_else(|| i.eq_ignore_ascii_case(&e.key), |k| k == e.key)
                    });
                if !same {
                    let mut mats = BTreeMap::new();
                    add(&mut mats, &e.cost.materials, 1);
                    add_into_total(&mut cost.total, &mats);
                    cost.experimental = Some(mats);
                    let coins = u64::from(e.cost.merc_coins);
                    cost.merc_coins_experimental = coins;
                    cost.merc_coins_total = cost.merc_coins_total.saturating_add(coins);
                }
            }
            None => {
                cost.verified = false;
                cost.blockers.push(PlanBlocker::UnknownExperimental);
            }
        }
    }
    cost.missing_grades = cost
        .blockers
        .iter()
        .filter_map(|b| match b {
            PlanBlocker::MissingRecipe { grade } => Some(*grade),
            _ => None,
        })
        .collect();
    cost
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{
        BlueprintAlias, EntryPolicy, Experimental, ModuleBlueprint, ModuleSelector, ModuleTypeDef,
        RecipeCost, ShipVariant, blueprints,
    };

    const ENGINE_ITEM: &str = "int_engine_size5_class5";

    const RAW: &str = include_str!("../../data/blueprints.json");

    fn cur(bp: &str, level: u8, quality: f64, exp: Option<&str>) -> CurrentEng {
        CurrentEng {
            blueprint: bp.into(),
            level,
            quality,
            experimental: exp.map(str::to_owned),
        }
    }

    fn req(target: u8, exp: Option<&str>, rolls: [u8; 5]) -> PlanRequest {
        PlanRequest {
            module_type: "engine".into(),
            blueprint: "engine_dirty".into(),
            experimental: exp.map(str::to_owned),
            target_grade: target,
            rolls,
        }
    }

    fn run(current: Option<&CurrentEng>, r: &PlanRequest) -> PlanCost {
        plan_cost(blueprints(), ENGINE_ITEM, current, r)
    }

    fn raw_mats(v: &serde_json::Value) -> Vec<(String, u64)> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|i| {
                (
                    i["material"].as_str().unwrap().to_owned(),
                    i["count"].as_u64().unwrap(),
                )
            })
            .collect()
    }

    #[test]
    fn stock_thrusters_dirty_g5_drag_drives_matches_manual() {
        let json: serde_json::Value = serde_json::from_str(RAW).unwrap();
        let bp = json["blueprints"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["key"] == "engine_dirty")
            .unwrap();
        let exp = json["experimentals"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["key"] == "special_engine_overloaded")
            .unwrap();
        let mut oracle: BTreeMap<String, u64> = BTreeMap::new();
        for g in 1..=5u32 {
            let times = if g == 5 { 4 } else { 1 };
            for (m, c) in raw_mats(&bp["grades"][g.to_string()]) {
                *oracle.entry(m).or_default() += c * times;
            }
        }
        for (m, c) in raw_mats(&exp["ingredients"]) {
            *oracle.entry(m).or_default() += c;
        }
        let r = req(5, Some("special_engine_overloaded"), [1, 1, 1, 1, 4]);
        let cost = run(None, &r);
        let got: BTreeMap<String, u64> = cost
            .total
            .iter()
            .map(|(k, v)| (k.clone(), u64::from(*v)))
            .collect();
        assert_eq!(got, oracle);
        // Valores concretos del dato: G5 x4 y G1 x1.
        assert_eq!(cost.total["polonium"], 4);
        assert_eq!(cost.total["carbon"], 1);
        assert_eq!(
            cost.per_grade.keys().copied().collect::<Vec<_>>(),
            [1, 2, 3, 4, 5]
        );
        assert!(!cost.replaces_blueprint);
        assert!(cost.experimental.is_some());
    }

    #[test]
    fn same_blueprint_g3_complete_counts_from_g4() {
        let c = cur("Engine_Dirty", 3, 1.0, None);
        let cost = run(Some(&c), &req(5, None, [1, 1, 1, 1, 4]));
        assert_eq!(cost.per_grade.keys().copied().collect::<Vec<_>>(), [4, 5]);
        assert!(!cost.replaces_blueprint);
    }

    #[test]
    fn partial_grade_counts_remaining_rolls() {
        let c = cur("Engine_Dirty", 5, 0.5, None);
        let cost = run(Some(&c), &req(5, None, [1, 1, 1, 1, 4]));
        assert_eq!(cost.per_grade.keys().copied().collect::<Vec<_>>(), [5]);
        assert_eq!(cost.per_grade[&5].0, 2);
        assert_eq!(cost.total["polonium"], 2);
    }

    #[test]
    fn other_blueprint_restarts_at_g1_with_warning() {
        let c = cur("Engine_Tuned", 5, 1.0, None);
        let cost = run(Some(&c), &req(5, None, [1, 1, 1, 1, 4]));
        assert_eq!(
            cost.per_grade.keys().copied().collect::<Vec<_>>(),
            [1, 2, 3, 4, 5]
        );
        assert!(cost.replaces_blueprint);
    }

    #[test]
    fn target_below_current_costs_nothing() {
        let c = cur("Engine_Dirty", 5, 1.0, None);
        let cost = run(Some(&c), &req(3, None, [1, 1, 1, 1, 4]));
        assert!(cost.per_grade.is_empty());
        assert!(cost.total.is_empty());
    }

    #[test]
    fn same_experimental_not_counted_twice() {
        let c = cur("Engine_Dirty", 5, 1.0, Some("Special_Engine_Overloaded"));
        let cost = run(
            Some(&c),
            &req(5, Some("special_engine_overloaded"), [1, 1, 1, 1, 4]),
        );
        assert_eq!(cost.experimental, None);
    }

    #[test]
    fn custom_rolls_change_totals() {
        let cost = run(None, &req(5, None, [2, 1, 1, 1, 6]));
        assert_eq!(cost.per_grade[&1].0, 2);
        assert_eq!(cost.per_grade[&1].1["carbon"], 2);
        assert_eq!(cost.per_grade[&5].0, 6);
        assert_eq!(cost.per_grade[&5].1["polonium"], 6);
    }

    #[test]
    fn missing_grade_is_flagged_not_fatal() {
        let mut cat = blueprints().clone();
        let bp = cat
            .blueprints
            .iter_mut()
            .find(|b| b.key == "engine_dirty")
            .unwrap();
        let data_verified = bp.verified;
        bp.recipes[0].grades.remove(&2);
        let cost = plan_cost(&cat, ENGINE_ITEM, None, &req(3, None, [1; 5]));
        assert_eq!(cost.missing_grades, [2]);
        assert_eq!(cost.blockers, [PlanBlocker::MissingRecipe { grade: 2 }]);
        assert_eq!(cost.verified, data_verified);
        assert_eq!(cost.per_grade[&2].0, 1);
    }

    #[test]
    fn quality_boundary_0998_is_partial_and_0999_is_complete() {
        let almost = cur("Engine_Dirty", 3, 0.998, None);
        let sp = start_grade(Some(&almost), "engine_dirty", &[1, 1, 1, 1, 4]);
        assert_eq!((sp.grade, sp.partial_rolls), (3, Some(1)));
        let done = cur("Engine_Dirty", 3, 0.999, None);
        let sp = start_grade(Some(&done), "engine_dirty", &[1, 1, 1, 1, 4]);
        assert_eq!((sp.grade, sp.partial_rolls), (4, None));
        let cost = run(Some(&almost), &req(5, None, [1, 1, 1, 1, 4]));
        assert_eq!(
            cost.per_grade.keys().copied().collect::<Vec<_>>(),
            [3, 4, 5]
        );
        let cost = run(Some(&done), &req(5, None, [1, 1, 1, 1, 4]));
        assert_eq!(cost.per_grade.keys().copied().collect::<Vec<_>>(), [4, 5]);
    }

    #[test]
    fn differing_experimental_is_charged_and_same_one_is_free() {
        let wanted = "special_engine_overloaded";
        let other = cur("Engine_Dirty", 5, 1.0, Some("special_engine_haulage"));
        let cost = run(Some(&other), &req(5, Some(wanted), [1, 1, 1, 1, 4]));
        assert!(cost.experimental.is_some());
        assert!(!cost.total.is_empty());
        let same = cur("Engine_Dirty", 5, 1.0, Some(wanted));
        let cost = run(Some(&same), &req(5, Some(wanted), [1, 1, 1, 1, 4]));
        assert!(cost.experimental.is_none());
        assert!(cost.total.is_empty());
    }

    // ----- Synthetic catalogue: it does not prove any real game link. -----

    const TEST_ITEM: &str = "int_test_module";

    fn ing(material: &str, count: u32) -> Ingredient {
        Ingredient {
            material: material.into(),
            count,
        }
    }

    fn cost_of(materials: Vec<Ingredient>, merc_coins: u32) -> RecipeCost {
        RecipeCost {
            materials,
            merc_coins,
        }
    }

    fn items(list: &[&str]) -> ModuleSelector {
        ModuleSelector {
            items: list.iter().map(|s| (*s).to_owned()).collect(),
            item_prefixes: Vec::new(),
        }
    }

    fn base(identity: BaseIdentity, completed_grade: u8) -> BaseRequirement {
        BaseRequirement {
            identity,
            selector: items(&[TEST_ITEM]),
            completed_grade,
            source_rows: vec!["synthetic fixture".into()],
        }
    }

    fn variant_base(grade: u8) -> BaseRequirement {
        base(
            BaseIdentity::Variant {
                key: "variant_test".into(),
            },
            grade,
        )
    }

    /// `upgrade_test` G2-G5 (coins 5/5/10/20, one material per grade) behind `bases`.
    fn upgrade_context(
        first: u8,
        selector: Option<ModuleSelector>,
        bases: Vec<BaseRequirement>,
    ) -> ModuleBlueprint {
        let coins = [0, 5, 5, 10, 20];
        ModuleBlueprint {
            module_type: "test".into(),
            selector,
            entry: EntryPolicy::ExistingOnly { bases },
            grades: (first..=5)
                .map(|g| {
                    (
                        g,
                        cost_of(
                            vec![ing(&format!("mat{g}"), u32::from(g))],
                            coins[usize::from(g) - 1],
                        ),
                    )
                })
                .collect(),
        }
    }

    fn blueprint(key: &str, recipes: Vec<ModuleBlueprint>) -> Blueprint {
        Blueprint {
            key: key.into(),
            recipes,
            source: "synthetic".into(),
            verified: false,
        }
    }

    fn synthetic(variant_grade: u8, contexts: Vec<ModuleBlueprint>) -> BlueprintCatalog {
        BlueprintCatalog {
            module_types: vec![ModuleTypeDef {
                id: "test".into(),
                items: vec![TEST_ITEM.into(), "int_test_other".into()],
                item_prefixes: Vec::new(),
            }],
            blueprints: vec![
                blueprint("upgrade_test", contexts),
                // Target of the journal spelling of the variant, for the two-starts case.
                blueprint(
                    "alt_test",
                    vec![ModuleBlueprint {
                        module_type: "test".into(),
                        selector: Some(items(&["int_test_other"])),
                        entry: EntryPolicy::Stock,
                        grades: (1..=5)
                            .map(|g| (g, cost_of(vec![ing("alt", 1)], 0)))
                            .collect(),
                    }],
                ),
            ],
            experimentals: vec![Experimental {
                key: "special_test".into(),
                module_types: vec!["test".into()],
                cost: cost_of(vec![ing("exp", 2)], 7),
                source: "synthetic".into(),
                verified: true,
            }],
            variants: vec![ShipVariant {
                key: "variant_test".into(),
                module_type: "test".into(),
                grade: variant_grade,
                selector: Some(items(&[TEST_ITEM])),
                journal_blueprints: vec!["variant_test_journal".into()],
                source: "synthetic".into(),
                verified: false,
            }],
            aliases: vec![
                BlueprintAlias {
                    module_type: "test".into(),
                    alias: "upgrade_alias".into(),
                    canonical: "upgrade_test".into(),
                },
                BlueprintAlias {
                    module_type: "test".into(),
                    alias: "variant_test_journal".into(),
                    canonical: "alt_test".into(),
                },
            ],
        }
    }

    fn standard() -> BlueprintCatalog {
        synthetic(1, vec![upgrade_context(2, None, vec![variant_base(1)])])
    }

    fn treq(exp: Option<&str>, target: u8) -> PlanRequest {
        PlanRequest {
            module_type: "test".into(),
            blueprint: "upgrade_test".into(),
            experimental: exp.map(str::to_owned),
            target_grade: target,
            rolls: [1, 1, 1, 1, 4],
        }
    }

    fn variant_cur(level: u8, quality: f64) -> CurrentEng {
        cur("variant_test_journal", level, quality, None)
    }

    #[test]
    fn proven_variant_base_unlocks_g2_to_g5_with_one_hundred_coins() {
        let cost = plan_cost(
            &standard(),
            TEST_ITEM,
            Some(&variant_cur(1, 1.0)),
            &treq(None, 5),
        );
        assert!(cost.blockers.is_empty());
        assert_eq!(
            cost.per_grade.keys().copied().collect::<Vec<_>>(),
            [2, 3, 4, 5]
        );
        assert_eq!(
            cost.merc_coins_by_grade,
            BTreeMap::from([(2, 5), (3, 5), (4, 10), (5, 80)])
        );
        assert_eq!(cost.merc_coins_total, 100);
        assert_eq!(cost.merc_coins_experimental, 0);
        assert!(!cost.replaces_blueprint);
        // Materials: G5 is multiplied by its four rolls; coins never become a material.
        assert_eq!(
            cost.total,
            BTreeMap::from([
                ("mat2".to_owned(), 2),
                ("mat3".to_owned(), 3),
                ("mat4".to_owned(), 4),
                ("mat5".to_owned(), 20),
            ])
        );
        assert!(cost.total.keys().all(|k| k.starts_with("mat")));
        // Unverified data is not an incomplete calculation.
        assert!(!cost.verified);
    }

    #[test]
    fn row_order_does_not_change_the_result() {
        let a = standard();
        let mut b = standard();
        b.blueprints.reverse();
        b.variants.reverse();
        b.aliases.reverse();
        let cur = variant_cur(1, 1.0);
        assert_eq!(
            plan_cost(&a, TEST_ITEM, Some(&cur), &treq(None, 5)),
            plan_cost(&b, TEST_ITEM, Some(&cur), &treq(None, 5))
        );
    }

    #[test]
    fn stock_or_partial_base_is_base_required_with_no_costs() {
        let cat = standard();
        for current in [None, Some(variant_cur(1, 0.5))] {
            let cost = plan_cost(&cat, TEST_ITEM, current.as_ref(), &treq(None, 5));
            assert_eq!(cost.blockers, [PlanBlocker::BaseRequired]);
            assert!(cost.per_grade.is_empty() && cost.total.is_empty());
            assert_eq!(cost.merc_coins_total, 0);
        }
    }

    #[test]
    fn removed_link_is_missing_base_mapping() {
        let cat = synthetic(1, vec![upgrade_context(2, None, Vec::new())]);
        let cost = plan_cost(&cat, TEST_ITEM, Some(&variant_cur(1, 1.0)), &treq(None, 5));
        assert_eq!(cost.blockers, [PlanBlocker::MissingBaseMapping]);
        assert!(cost.per_grade.is_empty());
    }

    #[test]
    fn two_valid_contexts_are_ambiguous_in_any_order() {
        let ctx = || upgrade_context(2, None, vec![variant_base(1)]);
        let mut other = ctx();
        other.selector = Some(items(&[TEST_ITEM]));
        for contexts in [vec![ctx(), other.clone()], vec![other, ctx()]] {
            let cat = synthetic(1, contexts);
            let cost = plan_cost(&cat, TEST_ITEM, Some(&variant_cur(1, 1.0)), &treq(None, 5));
            assert_eq!(cost.blockers, [PlanBlocker::AmbiguousRecipe]);
            assert!(cost.total.is_empty() && cost.per_grade.is_empty());
        }
    }

    #[test]
    fn two_different_starting_points_are_ambiguous() {
        // Level 2 complete: variant_test (grade 2) completed G1 -> start G2; the alt blueprint,
        // reached through the journal spelling, completed G2 -> start G3.
        let alt = base(
            BaseIdentity::Blueprint {
                key: "alt_test".into(),
            },
            2,
        );
        let cat = synthetic(
            2,
            vec![upgrade_context(2, None, vec![variant_base(1), alt])],
        );
        let cost = plan_cost(&cat, TEST_ITEM, Some(&variant_cur(2, 1.0)), &treq(None, 5));
        assert_eq!(cost.blockers, [PlanBlocker::AmbiguousRecipe]);
        assert!(cost.per_grade.is_empty());
    }

    #[test]
    fn existing_only_context_starting_at_g3_is_allowed() {
        let cat = synthetic(2, vec![upgrade_context(3, None, vec![variant_base(2)])]);
        let cost = plan_cost(&cat, TEST_ITEM, Some(&variant_cur(2, 1.0)), &treq(None, 5));
        assert!(cost.blockers.is_empty());
        assert_eq!(
            cost.per_grade.keys().copied().collect::<Vec<_>>(),
            [3, 4, 5]
        );
        assert_eq!(cost.merc_coins_total, 5 + 10 + 80);
        // A base that only completed G1 leaves G2 without a recipe: never a free step.
        let gap = synthetic(1, vec![upgrade_context(3, None, vec![variant_base(1)])]);
        let cost = plan_cost(&gap, TEST_ITEM, Some(&variant_cur(1, 1.0)), &treq(None, 5));
        assert_eq!(cost.blockers, [PlanBlocker::MissingRecipe { grade: 2 }]);
        assert_eq!(cost.missing_grades, [2]);
    }

    #[test]
    fn continuing_the_same_identity_keeps_the_partial_grade() {
        let continuation = base(
            BaseIdentity::Blueprint {
                key: "upgrade_test".into(),
            },
            2,
        );
        let cat = synthetic(
            1,
            vec![upgrade_context(
                2,
                None,
                vec![variant_base(1), continuation],
            )],
        );
        let mut r = treq(None, 5);
        r.rolls = [1, 1, 4, 1, 4];
        // Alias spelling of the installed blueprint, half of G3 done: two of four rolls left.
        let installed = cur("UPGRADE_ALIAS", 3, 0.5, None);
        let cost = plan_cost(&cat, TEST_ITEM, Some(&installed), &r);
        assert!(cost.blockers.is_empty());
        assert!(!cost.replaces_blueprint);
        assert_eq!(
            cost.per_grade.keys().copied().collect::<Vec<_>>(),
            [3, 4, 5]
        );
        assert_eq!(cost.per_grade[&3].0, 2);
        assert_eq!(cost.merc_coins_by_grade[&3], 10);
    }

    #[test]
    fn experimental_is_charged_once_materials_and_coins() {
        let cat = standard();
        let mut r = treq(Some("SPECIAL_TEST"), 5);
        r.rolls = [1, 1, 1, 1, 4];
        let cost = plan_cost(&cat, TEST_ITEM, Some(&variant_cur(1, 1.0)), &r);
        assert_eq!(
            cost.experimental,
            Some(BTreeMap::from([("exp".to_owned(), 2)]))
        );
        assert_eq!(cost.total["exp"], 2);
        assert_eq!(cost.merc_coins_experimental, 7);
        assert_eq!(cost.merc_coins_total, 107);
        assert!(!cost.verified, "the recipe data is still unverified");
    }

    #[test]
    fn unresolvable_requests_report_a_blocker_instead_of_zero_cost() {
        let cat = standard();
        let mut unknown = treq(None, 5);
        unknown.blueprint = "nope".into();
        assert_eq!(
            plan_cost(&cat, TEST_ITEM, None, &unknown).blockers,
            [PlanBlocker::UnknownBlueprint]
        );
        let cost = plan_cost(&cat, "int_other_thing", None, &treq(None, 5));
        assert_eq!(cost.blockers, [PlanBlocker::IncompatibleItem]);
        let bad_exp = plan_cost(
            &cat,
            TEST_ITEM,
            Some(&variant_cur(1, 1.0)),
            &treq(Some("special_none"), 5),
        );
        assert_eq!(bad_exp.blockers, [PlanBlocker::UnknownExperimental]);
        assert!(bad_exp.has_resolution_blocker());
    }

    // ----- Independent oracle from the approved amendment (section 8). -----

    fn oracle_catalogue() -> BlueprintCatalog {
        let g = |list: &[(&str, u32)]| cost_of(list.iter().map(|(m, c)| ing(m, *c)).collect(), 0);
        let grades = BTreeMap::from([
            (1, g(&[("legacyfirmware", 1)])),
            (2, g(&[("legacyfirmware", 1), ("mechanicalequipment", 1)])),
            (
                3,
                g(&[
                    ("legacyfirmware", 1),
                    ("chromium", 1),
                    ("mechanicalcomponents", 1),
                ]),
            ),
            (
                4,
                g(&[
                    ("consumerfirmware", 1),
                    ("selenium", 1),
                    ("configurablecomponents", 1),
                ]),
            ),
            (
                5,
                g(&[
                    ("industrialfirmware", 1),
                    ("cadmium", 1),
                    ("pharmaceuticalisolators", 1),
                ]),
            ),
        ]);
        BlueprintCatalog {
            module_types: vec![ModuleTypeDef {
                id: "engine".into(),
                items: Vec::new(),
                item_prefixes: vec!["int_engine_".into()],
            }],
            blueprints: vec![blueprint(
                "engine_dirty",
                vec![ModuleBlueprint {
                    module_type: "engine".into(),
                    selector: None,
                    entry: EntryPolicy::Stock,
                    grades,
                }],
            )],
            experimentals: vec![Experimental {
                key: "special_engine_overloaded".into(),
                module_types: vec!["engine".into()],
                cost: g(&[
                    ("iron", 5),
                    ("hybridcapacitors", 3),
                    ("securityfirmware", 1),
                ]),
                source: "synthetic".into(),
                verified: false,
            }],
            ..BlueprintCatalog::default()
        }
    }

    #[test]
    fn amendment_oracle_dirty_drag_defaults_has_thirteen_keys_and_no_coins() {
        let cat = oracle_catalogue();
        let cost = plan_cost(
            &cat,
            ENGINE_ITEM,
            None,
            &req(5, Some("special_engine_overloaded"), [1, 1, 1, 1, 4]),
        );
        let expected: BTreeMap<String, u32> = [
            ("legacyfirmware", 3),
            ("mechanicalequipment", 1),
            ("chromium", 1),
            ("mechanicalcomponents", 1),
            ("consumerfirmware", 1),
            ("selenium", 1),
            ("configurablecomponents", 1),
            ("industrialfirmware", 4),
            ("cadmium", 4),
            ("pharmaceuticalisolators", 4),
            ("iron", 5),
            ("hybridcapacitors", 3),
            ("securityfirmware", 1),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect();
        assert_eq!(cost.total, expected);
        assert_eq!(cost.total.len(), 13);
        assert_eq!(cost.merc_coins_total, 0);
        assert!(cost.blockers.is_empty());
    }

    #[test]
    fn amendment_oracle_g3_complete_costs_only_g4_and_g5() {
        let cat = oracle_catalogue();
        let c = cur("Engine_Dirty", 3, 1.0, None);
        let cost = plan_cost(&cat, ENGINE_ITEM, Some(&c), &req(5, None, [1, 1, 1, 1, 4]));
        let expected: BTreeMap<String, u32> = [
            ("consumerfirmware", 1),
            ("selenium", 1),
            ("configurablecomponents", 1),
            ("industrialfirmware", 4),
            ("cadmium", 4),
            ("pharmaceuticalisolators", 4),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect();
        assert_eq!(cost.total, expected);
        assert_eq!(cost.per_grade.keys().copied().collect::<Vec<_>>(), [4, 5]);
    }

    fn stock_catalogue() -> BlueprintCatalog {
        let mut cat = standard();
        cat.blueprints[0].recipes = vec![ModuleBlueprint {
            module_type: "test".into(),
            selector: None,
            entry: EntryPolicy::Stock,
            grades: (1..=5)
                .map(|g| (g, cost_of(vec![ing("m", 1)], 0)))
                .collect(),
        }];
        cat
    }

    fn grades_of(cost: &PlanCost) -> Vec<u8> {
        cost.per_grade.keys().copied().collect()
    }

    #[test]
    fn required_grades_match_the_cost_route_through_aliases() {
        let cat = stock_catalogue();
        // G3 a medias escrito con un alias del diario: la ruta es [3, 4, 5], no [1..=5].
        let current = cur("upgrade_alias", 3, 0.5, None);
        let req = treq(None, 5);
        let cost = plan_cost(&cat, TEST_ITEM, Some(&current), &req);
        assert_eq!(grades_of(&cost), [3, 4, 5]);
        assert_eq!(
            required_grades(&cat, TEST_ITEM, Some(&current), &req),
            [3, 4, 5]
        );
        // Sin canonicalizar, el grado de partida sería 1 (el fallo que esto evita).
        assert_eq!(
            start_grade(Some(&current), "upgrade_test", &req.rolls).grade,
            1
        );
        // Sin ingeniería actual: toda la ruta.
        assert_eq!(
            required_grades(&cat, TEST_ITEM, None, &req),
            [1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn required_grades_follow_the_base_policy_and_ignore_experimentals() {
        let cat = standard();
        let current = variant_cur(1, 1.0);
        let plain = treq(None, 5);
        let cost = plan_cost(&cat, TEST_ITEM, Some(&current), &plain);
        assert_eq!(grades_of(&cost), [2, 3, 4, 5]);
        assert_eq!(
            required_grades(&cat, TEST_ITEM, Some(&current), &plain),
            [2, 3, 4, 5]
        );
        let with_exp = treq(Some("special_test"), 5);
        assert_eq!(
            required_grades(&cat, TEST_ITEM, Some(&current), &with_exp),
            [2, 3, 4, 5]
        );
        // Ruta no resoluble (base exigida): se asume la ruta completa.
        assert_eq!(
            required_grades(&cat, TEST_ITEM, None, &plain),
            [1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn installed_experimental_alias_is_not_charged_again() {
        let mut cat = stock_catalogue();
        cat.aliases.push(BlueprintAlias {
            module_type: "test".into(),
            alias: "special_alias".into(),
            canonical: "special_test".into(),
        });
        let req = treq(Some("special_test"), 5);
        let same = cur("upgrade_alias", 3, 1.0, Some("SPECIAL_ALIAS"));
        let cost = plan_cost(&cat, TEST_ITEM, Some(&same), &req);
        assert_eq!(cost.experimental, None);
        assert_eq!(cost.merc_coins_experimental, 0);
        assert!(!cost.total.contains_key("exp"));
        // otro experimental instalado si se cobra
        let other = cur("upgrade_alias", 3, 1.0, Some("special_other"));
        let cost = plan_cost(&cat, TEST_ITEM, Some(&other), &req);
        assert!(cost.experimental.is_some());
        assert_eq!(cost.merc_coins_experimental, 7);
        // y pedirlo con el alias da el mismo resultado que con la clave
        let by_alias = treq(Some("special_alias"), 5);
        assert_eq!(
            plan_cost(&cat, TEST_ITEM, Some(&same), &by_alias).experimental,
            None
        );
    }

    #[test]
    fn base_stays_satisfied_when_installed_level_is_above_its_completed_grade() {
        let same_blueprint = base(
            BaseIdentity::Blueprint {
                key: "upgrade_test".into(),
            },
            2,
        );
        let cat = synthetic(1, vec![upgrade_context(3, None, vec![same_blueprint])]);
        let rec = &cat.blueprint("upgrade_test").unwrap().recipes[0];
        // el nivel instalado (4, a medias) supera el grado completado de la base (2)
        let above = cur("upgrade_alias", 4, 0.2, None);
        assert_eq!(cat.satisfied_bases(rec, TEST_ITEM, Some(&above)).len(), 1);
        let cost = plan_cost(&cat, TEST_ITEM, Some(&above), &treq(None, 5));
        assert!(cost.blockers.is_empty(), "{:?}", cost.blockers);
        assert_eq!(grades_of(&cost), [4, 5]);
        // en el propio grado de la base a medias aun no esta completada
        let partial = cur("upgrade_alias", 2, 0.5, None);
        assert!(
            cat.satisfied_bases(rec, TEST_ITEM, Some(&partial))
                .is_empty()
        );
    }

    #[test]
    fn start_above_target_requires_no_grades() {
        let cat = stock_catalogue();
        let done = cur("upgrade_test", 5, 1.0, None);
        assert!(required_grades(&cat, TEST_ITEM, Some(&done), &treq(None, 5)).is_empty());
        let g4 = cur("upgrade_alias", 4, 1.0, None);
        assert!(required_grades(&cat, TEST_ITEM, Some(&g4), &treq(None, 4)).is_empty());
    }
}
