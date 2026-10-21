//! Planificador de intercambios con el comerciante de materiales (spec §3.6 y §10.2.5).
//!
//! Solo se intercambia dentro del mismo tipo de comerciante. Misma familia: subir *n* grados
//! cuesta `6^n:1`, bajar *n* grados `1:3^n`. Otra familia del mismo tipo: la tasa de grado con
//! `cross_family_factor` aplicado al coste y fracción reducida.

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;
use tmc_model::net::TraderKind;

use crate::data::{MaterialDef, TraderRules, grade_cap, materials};

/// Tasa de intercambio: se entregan `give` y se reciben `get`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Rate {
    pub give: u32,
    pub get: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TradeProposal {
    pub give_key: String,
    pub give: u32,
    pub get_key: String,
    pub get: u32,
    pub trader: TraderKind,
    pub same_family: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TradePlan {
    pub proposals: Vec<TradeProposal>,
    pub uncovered: BTreeMap<String, u32>,
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Tasa para convertir `from` en `to`; `None` si son de distinto tipo, no intercambiables
/// o el mismo material.
pub fn rate(from: &MaterialDef, to: &MaterialDef, rules: &TraderRules) -> Option<Rate> {
    if from.category != to.category || !from.tradeable || !to.tradeable || from.key == to.key {
        return None;
    }
    let (mut give, mut get) = if to.grade >= from.grade {
        (
            rules
                .same_family_up
                .checked_pow(u32::from(to.grade - from.grade))?,
            1,
        )
    } else {
        (
            1,
            rules
                .same_family_down
                .checked_pow(u32::from(from.grade - to.grade))?,
        )
    };
    if from.family != to.family {
        give = give.checked_mul(rules.cross_family_factor)?;
        let g = gcd(give, get).max(1);
        give /= g;
        get /= g;
    }
    (give > 0 && get > 0).then_some(Rate { give, get })
}

/// Planifica con el catálogo embebido.
pub fn plan_trades(
    missing: &BTreeMap<String, u32>,
    surplus: &BTreeMap<String, u32>,
    have: &dyn Fn(&str) -> u32,
    rules: &TraderRules,
) -> TradePlan {
    plan_trades_with(materials(), missing, surplus, have, rules)
}

type Candidate<'a> = (bool, u32, u32, &'a str, Rate, u32);

pub(crate) fn plan_trades_with(
    defs: &HashMap<String, MaterialDef>,
    missing: &BTreeMap<String, u32>,
    surplus: &BTreeMap<String, u32>,
    have: &dyn Fn(&str) -> u32,
    rules: &TraderRules,
) -> TradePlan {
    let mut plan = TradePlan::default();
    let mut left: BTreeMap<String, u32> = surplus.clone();

    let mut order: Vec<(&String, u32)> = missing.iter().map(|(k, n)| (k, *n)).collect();
    order.sort_by(|a, b| {
        let ga = defs.get(a.0).map_or(0, |d| d.grade);
        let gb = defs.get(b.0).map_or(0, |d| d.grade);
        gb.cmp(&ga).then_with(|| a.0.cmp(b.0))
    });

    for (key, need) in order {
        if need == 0 {
            continue;
        }
        let Some(target) = defs.get(key).filter(|d| d.tradeable) else {
            plan.uncovered.insert(key.clone(), need);
            continue;
        };
        let room = grade_cap(target.grade).saturating_sub(have(key));

        // (same_family, give_total, remaining, key, rate, trades)
        let mut best: Option<Candidate> = None;
        for (skey, avail) in &left {
            let Some(src) = defs.get(skey) else { continue };
            let Some(r) = rate(src, target, rules) else {
                continue;
            };
            let trades = need.div_ceil(r.get).min(room / r.get);
            let give_total = trades.saturating_mul(r.give);
            if trades == 0 || give_total > *avail {
                continue;
            }
            let cand: Candidate = (
                src.family == target.family,
                give_total,
                *avail - give_total,
                skey.as_str(),
                r,
                trades,
            );
            // misma familia primero; menor gasto; mayor excedente restante; clave
            let better = best.as_ref().is_none_or(|b| {
                (!cand.0, cand.1, std::cmp::Reverse(cand.2), cand.3)
                    < (!b.0, b.1, std::cmp::Reverse(b.2), b.3)
            });
            if better {
                best = Some(cand);
            }
        }

        let Some((same_family, give_total, _, skey, r, trades)) = best else {
            plan.uncovered.insert(key.clone(), need);
            continue;
        };
        let skey = skey.to_owned();
        let received = trades * r.get;
        if let Some(s) = left.get_mut(&skey) {
            *s -= give_total;
        }
        plan.proposals.push(TradeProposal {
            give_key: skey,
            give: give_total,
            get_key: key.clone(),
            get: received,
            trader: TraderKind::from(target.category),
            same_family,
        });
        if received < need {
            plan.uncovered.insert(key.clone(), need - received);
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmc_model::events::MaterialCategory;

    fn rules() -> TraderRules {
        TraderRules::default()
    }

    fn map(items: &[(&str, u32)]) -> BTreeMap<String, u32> {
        items.iter().map(|(k, n)| ((*k).to_owned(), *n)).collect()
    }

    fn def(
        key: &str,
        cat: MaterialCategory,
        grade: u8,
        family: &str,
        tradeable: bool,
    ) -> MaterialDef {
        MaterialDef {
            key: key.to_owned(),
            name_en: key.to_owned(),
            category: cat,
            grade,
            family: family.to_owned(),
            tradeable,
            hints: vec![],
            source: String::new(),
            verified: false,
        }
    }

    fn defs(list: Vec<MaterialDef>) -> HashMap<String, MaterialDef> {
        list.into_iter().map(|d| (d.key.clone(), d)).collect()
    }

    fn run(m: &[(&str, u32)], s: &[(&str, u32)], h: &[(&str, u32)]) -> TradePlan {
        let have: BTreeMap<String, u32> = map(h);
        plan_trades(
            &map(m),
            &map(s),
            &|k| have.get(k).copied().unwrap_or(0),
            &rules(),
        )
    }

    #[test]
    fn same_family_up_six_to_one() {
        let p = run(&[("niobium", 1)], &[("vanadium", 12)], &[]);
        assert_eq!(p.proposals.len(), 1);
        let t = &p.proposals[0];
        assert_eq!(
            (t.give_key.as_str(), t.give, t.get_key.as_str(), t.get),
            ("vanadium", 6, "niobium", 1)
        );
        assert!(t.same_family);
        assert_eq!(t.trader, TraderKind::Raw);
        assert!(p.uncovered.is_empty());
    }

    #[test]
    fn same_family_down_one_to_three() {
        let p = run(&[("carbon", 3)], &[("vanadium", 5)], &[]);
        let t = &p.proposals[0];
        assert_eq!((t.give, t.get), (1, 3));
        // chained: G3 -> G1 is 1:9
        let p = run(&[("carbon", 9)], &[("niobium", 5)], &[]);
        assert_eq!((p.proposals[0].give, p.proposals[0].get), (1, 9));
    }

    #[test]
    fn cross_family_uses_penalty_from_rules() {
        let p = run(&[("heatexchangers", 1)], &[("conductiveceramics", 12)], &[]);
        let t = &p.proposals[0];
        assert_eq!((t.give, t.get), (6, 1));
        assert!(!t.same_family);
        let a = &materials()["conductiveceramics"];
        let b = &materials()["heatvanes"];
        assert_eq!(rate(a, b, &rules()), Some(Rate { give: 36, get: 1 }));
        assert_eq!(rate(b, a, &rules()), Some(Rate { give: 2, get: 1 }));
    }

    #[test]
    fn prefers_same_family_then_least_spent() {
        let d = defs(vec![
            def("t", MaterialCategory::Raw, 2, "a", true),
            def("same_low", MaterialCategory::Raw, 1, "a", true),
            def("same_hi", MaterialCategory::Raw, 3, "a", true),
            def("other_hi", MaterialCategory::Raw, 3, "b", true),
        ]);
        let go =
            |s: &[(&str, u32)]| plan_trades_with(&d, &map(&[("t", 1)]), &map(s), &|_| 0, &rules());
        // same_low costs 6 (up), other_hi costs 2 (cross down): same family wins
        let p = go(&[("same_low", 20), ("other_hi", 20)]);
        assert_eq!(p.proposals[0].give_key, "same_low");
        // two same family: same_hi costs 1 (down), same_low costs 6
        let p = go(&[("same_low", 20), ("same_hi", 20)]);
        assert_eq!(
            (p.proposals[0].give_key.as_str(), p.proposals[0].give),
            ("same_hi", 1)
        );
        // same family without enough surplus falls to other family
        let p = go(&[("same_low", 5), ("other_hi", 20)]);
        assert_eq!(p.proposals[0].give_key, "other_hi");
    }

    #[test]
    fn never_crosses_trader_type() {
        let p = run(
            &[("disruptedwakeechoes", 1)],
            &[("carbon", 99), ("heatexchangers", 99)],
            &[],
        );
        assert!(p.proposals.is_empty());
        assert_eq!(p.uncovered, map(&[("disruptedwakeechoes", 1)]));
    }

    #[test]
    fn non_tradeable_materials_skipped() {
        let d = defs(vec![
            def("t", MaterialCategory::Raw, 2, "a", true),
            def("locked_src", MaterialCategory::Raw, 2, "a", false),
            def("ok_src", MaterialCategory::Raw, 2, "b", true),
            def("locked_t", MaterialCategory::Raw, 2, "a", false),
        ]);
        let p = plan_trades_with(
            &d,
            &map(&[("t", 1), ("locked_t", 1)]),
            &map(&[("locked_src", 50), ("ok_src", 50)]),
            &|_| 0,
            &rules(),
        );
        assert_eq!(p.proposals.len(), 1);
        assert_eq!(p.proposals[0].give_key, "ok_src");
        assert_eq!(p.uncovered, map(&[("locked_t", 1)]));
    }

    #[test]
    fn trade_never_exceeds_grade_cap() {
        let p = run(&[("polonium", 3)], &[("yttrium", 60)], &[("polonium", 99)]);
        assert_eq!(p.proposals.len(), 1);
        assert_eq!((p.proposals[0].give, p.proposals[0].get), (6, 1));
        assert_eq!(p.uncovered, map(&[("polonium", 2)]));
        let p = run(&[("polonium", 1)], &[("yttrium", 60)], &[("polonium", 100)]);
        assert!(p.proposals.is_empty());
        assert_eq!(p.uncovered, map(&[("polonium", 1)]));
    }

    #[test]
    fn surplus_is_consumed_across_missing() {
        let p = run(&[("niobium", 1), ("yttrium", 1)], &[("vanadium", 6)], &[]);
        // G4 first: vanadium->yttrium costs 36 > 6, not possible; niobium takes it
        assert_eq!(p.proposals.len(), 1);
        assert_eq!(p.proposals[0].get_key, "niobium");
        assert_eq!(p.uncovered, map(&[("yttrium", 1)]));
    }
}
