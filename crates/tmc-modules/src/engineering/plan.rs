//! Cálculo de materiales para ingenierizar un módulo: grado de partida, tiradas por grado y total.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::ship::CurrentEng;
use crate::data::{Blueprint, Experimental, Ingredient};

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
    pub replaces_blueprint: bool,
    pub verified: bool,
    /// Grados pedidos sin receta en el dato (se cuentan sin materiales y `verified` queda en falso).
    pub missing_grades: Vec<u8>,
}

/// Grado máximo definido para un blueprint (objetivo por defecto).
pub fn max_grade(bp: &Blueprint) -> u8 {
    bp.grades.keys().next_back().copied().unwrap_or(5)
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

pub fn plan_cost(
    bp: &Blueprint,
    exp: Option<&Experimental>,
    current: Option<&CurrentEng>,
    req: &PlanRequest,
) -> PlanCost {
    let start = start_grade(current, &bp.key, &req.rolls);
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
        match bp.grades.get(&g) {
            Some(recipe) => add(&mut mats, recipe, u32::from(rolls)),
            None => {
                cost.missing_grades.push(g);
                cost.verified = false;
            }
        }
        add_into_total(&mut cost.total, &mats);
        cost.per_grade.insert(g, (rolls, mats));
    }
    if let Some(wanted) = req.experimental.as_deref() {
        let same = !start.replaces
            && current
                .and_then(|c| c.experimental.as_deref())
                .is_some_and(|e| e.eq_ignore_ascii_case(wanted));
        match exp {
            Some(e) => {
                cost.verified &= e.verified;
                if !same {
                    let mut mats = BTreeMap::new();
                    add(&mut mats, &e.ingredients, 1);
                    add_into_total(&mut cost.total, &mats);
                    cost.experimental = Some(mats);
                }
            }
            None => cost.verified = false,
        }
    }
    cost
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::blueprints;

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
        let cat = blueprints();
        let bp = cat.blueprint(&r.blueprint).unwrap();
        let exp = r.experimental.as_deref().and_then(|e| cat.experimental(e));
        plan_cost(bp, exp, current, r)
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
        let mut bp = blueprints().blueprint("engine_dirty").unwrap().clone();
        bp.grades.remove(&2);
        let cost = plan_cost(&bp, None, None, &req(3, None, [1; 5]));
        assert_eq!(cost.missing_grades, [2]);
        assert!(!cost.verified);
        assert_eq!(cost.per_grade[&2].0, 1);
    }
}
