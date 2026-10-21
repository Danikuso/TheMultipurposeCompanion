//! Lista de la compra: objetivos fijados, reservas por orden y persistencia (spec §3.5 y §10.2.6).
//!
//! Cada objetivo reserva, en el orden de la lista, lo que tiene de lo que necesita. Lo
//! reservado por cualquier objetivo nunca es excedente ni se ofrece en un intercambio.
//! La escritura a la tabla propia lleva etiqueta `*.write` y no ocurre en replay.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tmc_core::{Migration, StoreError, StoreTask};

use super::plan::PlanRequest;
use super::ship::CurrentEng;
use super::trades::{TradeProposal, plan_trades};
use crate::data::{TraderRules, materials};

pub const MAX_GOALS: usize = 20;
pub const TAG_GOALS_READ: &str = "engineering.goals";
pub const TAG_GOALS_WRITE: &str = "engineering.goals.write";
pub const STATUS_OK: &str = "ok";
pub const STATUS_TOO_MANY: &str = "too_many";

pub const ENGINEERING_MIGRATION: Migration = Migration {
    id: "engineering-001",
    sql: "CREATE TABLE engineering_goals (cmdr TEXT NOT NULL, position INTEGER NOT NULL, id TEXT NOT NULL, json TEXT NOT NULL, PRIMARY KEY(cmdr, id));",
};

/// Objetivo fijado. `id` = `"{ship_id}:{slot}:{blueprint}:{experimental|-}:{grade}"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub ship_id: Option<u64>,
    pub slot: String,
    pub item: String,
    pub request: PlanRequest,
    pub base: Option<CurrentEng>,
}

/// Identificador estable de un objetivo (`-` si no hay nave).
pub fn goal_id(ship_id: Option<u64>, slot: &str, req: &PlanRequest) -> String {
    let ship = ship_id.map_or_else(|| "-".to_string(), |s| s.to_string());
    format!(
        "{ship}:{slot}:{}:{}:{}",
        req.blueprint,
        req.experimental.as_deref().unwrap_or("-"),
        req.target_grade
    )
}

impl Goal {
    pub fn new(
        ship_id: Option<u64>,
        slot: String,
        item: String,
        request: PlanRequest,
        base: Option<CurrentEng>,
    ) -> Self {
        Self {
            id: goal_id(ship_id, &slot, &request),
            ship_id,
            slot,
            item,
            request,
            base,
        }
    }
}

/// Fija un objetivo: con un `id` ya presente lo sustituye en su posición; un nuevo
/// por encima de [`MAX_GOALS`] se rechaza (`"too_many"`, lista intacta).
pub fn pin_goal(goals: &[Goal], goal: Goal) -> (Vec<Goal>, &'static str) {
    let mut out = goals.to_vec();
    if let Some(pos) = out.iter().position(|g| g.id == goal.id) {
        out[pos] = goal;
        return (out, STATUS_OK);
    }
    if out.len() >= MAX_GOALS {
        return (out, STATUS_TOO_MANY);
    }
    out.push(goal);
    (out, STATUS_OK)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reservations {
    pub by_goal: BTreeMap<String, BTreeMap<String, u32>>,
}

impl Reservations {
    pub fn reserved_total(&self, key: &str) -> u32 {
        self.by_goal
            .values()
            .map(|m| m.get(key).copied().unwrap_or(0))
            .sum()
    }

    /// Excedente: lo que se tiene menos lo reservado por todos los objetivos.
    pub fn surplus(&self, key: &str, have: u32) -> u32 {
        have.saturating_sub(self.reserved_total(key))
    }
}

/// Reserva en el orden de la lista: cada objetivo toma `min(lo que queda, lo que necesita)`.
pub fn reserve(
    goals_needs: &[(String, BTreeMap<String, u32>)],
    have: &dyn Fn(&str) -> u32,
) -> Reservations {
    let mut left: BTreeMap<String, u32> = BTreeMap::new();
    let mut res = Reservations::default();
    for (id, needs) in goals_needs {
        let mut mine = BTreeMap::new();
        for (key, need) in needs {
            let avail = left.entry(key.clone()).or_insert_with(|| have(key));
            let take = (*avail).min(*need);
            *avail -= take;
            mine.insert(key.clone(), take);
        }
        res.by_goal.insert(id.clone(), mine);
    }
    res
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GoalView {
    pub id: String,
    pub percent: u8,
    pub missing: BTreeMap<String, u32>,
    pub trades: Vec<TradeProposal>,
    pub completable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShoppingView {
    pub status: String,
    pub goals: Vec<GoalView>,
    pub needs: BTreeMap<String, u32>,
    pub missing: BTreeMap<String, u32>,
    pub surplus: BTreeMap<String, u32>,
}

/// Evalúa la lista en orden (el flujo abierto va al final como un objetivo más).
pub fn evaluate(
    goals_needs: &[(String, BTreeMap<String, u32>)],
    have: &dyn Fn(&str) -> u32,
    rules: &TraderRules,
    status: &str,
) -> ShoppingView {
    let res = reserve(goals_needs, have);
    let mut surplus: BTreeMap<String, u32> = materials()
        .keys()
        .filter_map(|k| {
            let s = res.surplus(k, have(k));
            (s > 0).then(|| (k.clone(), s))
        })
        .collect();
    let mut view = ShoppingView {
        status: status.to_string(),
        goals: Vec::new(),
        needs: BTreeMap::new(),
        missing: BTreeMap::new(),
        surplus: surplus.clone(),
    };
    for (id, needs) in goals_needs {
        let empty = BTreeMap::new();
        let reserved = res.by_goal.get(id).unwrap_or(&empty);
        let mut missing = BTreeMap::new();
        let (mut covered, mut total) = (0u64, 0u64);
        for (key, need) in needs {
            let got = reserved.get(key).copied().unwrap_or(0).min(*need);
            covered += u64::from(got);
            total += u64::from(*need);
            *view.needs.entry(key.clone()).or_insert(0) += need;
            if got < *need {
                missing.insert(key.clone(), need - got);
                *view.missing.entry(key.clone()).or_insert(0) += need - got;
            }
        }
        let percent = (covered * 100)
            .checked_div(total)
            .map_or(100, |p| u8::try_from(p).unwrap_or(100));
        let plan = plan_trades(&missing, &surplus, have, rules);
        // lo ofrecido a este objetivo no se vuelve a ofrecer a los siguientes
        for p in &plan.proposals {
            if let Some(s) = surplus.get_mut(&p.give_key) {
                *s = s.saturating_sub(p.give);
            }
        }
        view.goals.push(GoalView {
            id: id.clone(),
            percent,
            completable: plan.uncovered.is_empty(),
            missing,
            trades: plan.proposals,
        });
    }
    view
}

fn bad(e: impl std::fmt::Display) -> StoreError {
    StoreError::Unsupported(e.to_string())
}

/// Lectura (`engineering.goals`): `{cmdr, goals}` en el orden fijado.
pub fn goals_read_task(cmdr: String) -> StoreTask {
    Box::new(move |store| {
        let rows = store.query_json(
            "SELECT json FROM engineering_goals WHERE cmdr = ?1 ORDER BY position",
            &[json!(cmdr)],
        )?;
        let goals: Vec<Value> = rows
            .iter()
            .filter_map(|r| r[0].as_str())
            .filter_map(|s| serde_json::from_str::<Value>(s).ok())
            .collect();
        Ok(json!({"cmdr": cmdr, "goals": goals}))
    })
}

/// Interpreta el resultado de [`goals_read_task`]; descarta filas ilegibles.
pub fn goals_from_result(result: &Value) -> Vec<Goal> {
    result["goals"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|g| serde_json::from_value(g.clone()).ok())
        .collect()
}

/// Escritura (`engineering.goals.write`): reemplaza todas las del comandante en una transacción.
pub fn goals_write_task(cmdr: String, goals: Vec<Goal>) -> StoreTask {
    Box::new(move |store| {
        store.exec_json("BEGIN IMMEDIATE", &[])?;
        let run = || -> Result<(), StoreError> {
            store.exec_json(
                "DELETE FROM engineering_goals WHERE cmdr = ?1",
                &[json!(cmdr)],
            )?;
            for (i, g) in goals.iter().enumerate() {
                let text = serde_json::to_string(g).map_err(bad)?;
                store.exec_json(
                    "INSERT OR REPLACE INTO engineering_goals (cmdr, position, id, json) VALUES (?1, ?2, ?3, ?4)",
                    &[json!(cmdr), json!(i), json!(g.id), json!(text)],
                )?;
            }
            Ok(())
        };
        match run() {
            Ok(()) => {
                store.exec_json("COMMIT", &[])?;
                Ok(json!({"cmdr": cmdr, "count": goals.len()}))
            }
            Err(e) => {
                let _ = store.exec_json("ROLLBACK", &[]);
                Err(e)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::trader_rules;
    use tmc_core::Store;

    fn m(pairs: &[(&str, u32)]) -> BTreeMap<String, u32> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    fn needs(id: &str, pairs: &[(&str, u32)]) -> (String, BTreeMap<String, u32>) {
        (id.to_string(), m(pairs))
    }

    fn req(grade: u8) -> PlanRequest {
        PlanRequest {
            module_type: "engine".into(),
            blueprint: "engine_dirty".into(),
            experimental: Some("special_engine_overloaded".into()),
            target_grade: grade,
            rolls: [1; 5],
        }
    }

    fn goal(grade: u8) -> Goal {
        Goal::new(
            Some(7),
            "MainEngines".into(),
            "Drive".into(),
            req(grade),
            None,
        )
    }

    #[test]
    fn reservations_follow_pinned_order() {
        let list = [needs("A", &[("X", 6)]), needs("B", &[("X", 8)])];
        let r = reserve(&list, &|_| 10);
        assert_eq!(r.by_goal["A"]["X"], 6);
        assert_eq!(r.by_goal["B"]["X"], 4);
        let have = |k: &str| if k == "X" { 10 } else { 0 };
        let v = evaluate(&list, &have, trader_rules(), STATUS_OK);
        assert_eq!(v.goals[1].missing, m(&[("X", 4)]));
        assert!(v.goals[0].missing.is_empty());
        assert_eq!(v.goals[0].percent, 100);
        assert_eq!(v.goals[1].percent, 50);
    }

    #[test]
    fn reserved_by_other_goal_never_offered() {
        let list = [
            needs("A", &[("vanadium", 6)]),
            needs("B", &[("niobium", 1)]),
        ];
        let have = |k: &str| if k == "vanadium" { 6 } else { 0 };
        let v = evaluate(&list, &have, trader_rules(), STATUS_OK);
        assert!(v.goals[1].trades.is_empty());
        assert!(!v.goals[1].completable);
        assert!(!v.surplus.contains_key("vanadium"));
    }

    #[test]
    fn surplus_excludes_all_reservations() {
        let list = [needs("A", &[("X", 5)]), needs("B", &[("X", 7)])];
        let r = reserve(&list, &|_| 30);
        assert_eq!(r.surplus("X", 30), 18);
    }

    #[test]
    fn completable_when_trades_cover_missing() {
        let list = [needs("A", &[("niobium", 1)])];
        let have = |k: &str| if k == "vanadium" { 6 } else { 0 };
        let v = evaluate(&list, &have, trader_rules(), STATUS_OK);
        assert!(!v.goals[0].trades.is_empty());
        assert!(v.goals[0].completable);
        assert!(v.goals[0].percent < 100);
    }

    #[test]
    fn goals_persist_across_restart() {
        let store = Store::open_in_memory().unwrap();
        store.apply_migrations(&[ENGINEERING_MIGRATION]).unwrap();
        let goals = vec![goal(5), goal(4)];
        goals_write_task("C".into(), goals.clone())(&store).unwrap();
        let back = goals_from_result(&goals_read_task("C".into())(&store).unwrap());
        assert_eq!(back, goals);
        // reescribir reemplaza, y otro comandante no se ve afectado
        goals_write_task("D".into(), vec![goal(3)])(&store).unwrap();
        goals_write_task("C".into(), vec![goal(4)])(&store).unwrap();
        let back = goals_from_result(&goals_read_task("C".into())(&store).unwrap());
        assert_eq!(back, vec![goal(4)]);
        let other = goals_from_result(&goals_read_task("D".into())(&store).unwrap());
        assert_eq!(other.len(), 1);
    }

    #[test]
    fn reorder_changes_reservations() {
        let have = |_: &str| 10;
        let ab = [needs("A", &[("X", 6)]), needs("B", &[("X", 8)])];
        let ba = [needs("B", &[("X", 8)]), needs("A", &[("X", 6)])];
        let r = reserve(&ba, &have);
        assert_eq!(r.by_goal["B"]["X"], 8);
        assert_eq!(r.by_goal["A"]["X"], 2);
        assert_eq!(reserve(&ab, &have).by_goal["B"]["X"], 4);
    }

    #[test]
    fn max_goals_enforced() {
        let mut goals = Vec::new();
        for i in 0..MAX_GOALS {
            let mut g = goal(5);
            g.id = format!("g{i}");
            let (next, status) = pin_goal(&goals, g);
            assert_eq!(status, STATUS_OK);
            goals = next;
        }
        let mut extra = goal(5);
        extra.id = "extra".into();
        let (next, status) = pin_goal(&goals, extra);
        assert_eq!(status, STATUS_TOO_MANY);
        assert_eq!(next.len(), MAX_GOALS);
        // sustituir uno existente sigue permitido con la lista llena
        let mut same = goal(5);
        same.id = "g3".into();
        let (next, status) = pin_goal(&goals, same);
        assert_eq!((next.len(), status), (MAX_GOALS, STATUS_OK));
    }

    #[test]
    fn pin_same_id_replaces() {
        let g5 = goal(5);
        assert_eq!(
            g5.id,
            "7:MainEngines:engine_dirty:special_engine_overloaded:5"
        );
        let (list, _) = pin_goal(&[], g5.clone());
        let (list, _) = pin_goal(&list, goal(4));
        let (list, _) = pin_goal(&list, g5.clone());
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], g5);
        let (list, _) = pin_goal(std::slice::from_ref(&g5), g5.clone());
        assert_eq!(list.len(), 1);
    }
}
