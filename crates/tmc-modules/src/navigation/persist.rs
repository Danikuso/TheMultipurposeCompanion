//! Persistencia por comandante de la ruta planificada activa (`nav_route_plan`).

use super::route::ActivePlan;
use chrono::{DateTime, SecondsFormat};
use serde_json::{Value, json};
use tmc_core::{Migration, Store, StoreError, StoreTask};

pub const MIGRATIONS: &[Migration] = &[Migration {
    id: "navigation-001",
    sql: "CREATE TABLE nav_route_plan (cmdr TEXT PRIMARY KEY, created_at TEXT NOT NULL, origin TEXT NOT NULL, destination TEXT NOT NULL, mode TEXT NOT NULL, params TEXT NOT NULL, hops TEXT NOT NULL, current_index INTEGER NOT NULL);",
}];

fn bad(e: impl std::fmt::Display) -> StoreError {
    StoreError::Unsupported(e.to_string())
}

pub fn save_plan(cmdr: String, plan: ActivePlan) -> StoreTask {
    Box::new(move |store: &Store| {
        let mode = serde_json::to_value(plan.mode)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| bad("mode"))?;
        let index = i64::try_from(plan.current_index).map_err(bad)?;
        store.exec_json(
            "INSERT OR REPLACE INTO nav_route_plan (cmdr, created_at, origin, destination, mode, params, hops, current_index) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            &[
                Value::from(cmdr.as_str()),
                Value::from(
                    plan.created_at
                        .to_rfc3339_opts(SecondsFormat::Nanos, true),
                ),
                Value::from(plan.origin.as_str()),
                Value::from(plan.destination.as_str()),
                Value::from(mode),
                Value::from(plan.params.to_string()),
                Value::from(serde_json::to_string(&plan.hops).map_err(bad)?),
                Value::from(index),
            ],
        )?;
        Ok(json!({"cmdr": cmdr, "saved": true}))
    })
}

/// Resultado: `{"cmdr", "plan": ActivePlan | null}`.
pub fn load_plan(cmdr: String) -> StoreTask {
    Box::new(move |store: &Store| {
        let rows = store.query_json(
            "SELECT created_at, origin, destination, mode, params, hops, current_index FROM nav_route_plan WHERE cmdr=?1",
            &[Value::from(cmdr.as_str())],
        )?;
        let plan = rows.first().and_then(parse_row);
        Ok(json!({"cmdr": cmdr, "plan": plan}))
    })
}

fn parse_row(row: &Value) -> Option<Value> {
    let text = |i: usize| row.get(i).and_then(Value::as_str);
    DateTime::parse_from_rfc3339(text(0)?).ok()?;
    Some(json!({
        "created_at": text(0)?,
        "origin": text(1)?,
        "destination": text(2)?,
        "mode": text(3)?,
        "params": serde_json::from_str::<Value>(text(4)?).ok()?,
        "hops": serde_json::from_str::<Value>(text(5)?).ok()?,
        "current_index": row.get(6).and_then(Value::as_u64)?,
    }))
}

pub fn delete_plan(cmdr: String) -> StoreTask {
    Box::new(move |store: &Store| {
        store.exec_json(
            "DELETE FROM nav_route_plan WHERE cmdr=?1",
            &[Value::from(cmdr.as_str())],
        )?;
        Ok(json!({"cmdr": cmdr, "deleted": true}))
    })
}
