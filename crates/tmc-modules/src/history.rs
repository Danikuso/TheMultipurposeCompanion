//! Paginador compartido del historial de eventos (`event` + `journal_file`).
//!
//! Orden estable `(ts, archivo, línea, id)`; páginas de 256; todo con
//! parámetros enlazados (los nombres de evento viajan como array JSON).

use serde_json::{Value, json};
use tmc_core::{Store, StoreError};

const PAGE: usize = 256;

/// Los `ts` se normalizan a UTC con nanosegundos fijos al guardarse; archivo y
/// línea conservan el orden dentro del mismo segundo.
#[derive(Debug, Clone)]
pub struct Position {
    pub id: i64,
    pub ts: String,
    pub file: String,
    pub line: i64,
    pub raw: String,
}

impl Position {
    fn from_row(row: &Value) -> Self {
        Self {
            id: row[0].as_i64().unwrap_or_default(),
            ts: row[1].as_str().unwrap_or_default().to_owned(),
            file: row[2].as_str().unwrap_or_default().to_owned(),
            line: row[3].as_i64().unwrap_or_default(),
            raw: row[4].as_str().unwrap_or_default().to_owned(),
        }
    }
}

pub fn bound_params(cmdr: &str, cut: i64, names: &[&str], bound: Option<&Position>) -> [Value; 7] {
    let names = json!(names).to_string();
    match bound {
        Some(b) => [
            json!(cmdr),
            json!(cut),
            json!(b.ts),
            json!(b.file),
            json!(b.line),
            json!(b.id),
            json!(names),
        ],
        None => [
            json!(cmdr),
            json!(cut),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            json!(names),
        ],
    }
}

fn page(
    store: &Store,
    cmdr: &str,
    cut: i64,
    names: &[&str],
    bound: Option<&Position>,
    forward: bool,
) -> Result<Vec<Position>, StoreError> {
    let (cmp, dir) = if forward { (">", "ASC") } else { ("<", "DESC") };
    let sql = format!(
        "SELECT e.id, e.ts, j.name, e.line, e.json FROM event e
         JOIN journal_file j ON j.id = e.file_id
         WHERE e.cmdr = ?1 AND e.id <= ?2
           AND (?3 IS NULL OR (e.ts, j.name, e.line, e.id) {cmp} (?3, ?4, ?5, ?6))
           AND e.name IN (SELECT value FROM json_each(?7))
         ORDER BY e.ts {dir}, j.name {dir}, e.line {dir}, e.id {dir} LIMIT {PAGE}"
    );
    let rows = store.query_json(&sql, &bound_params(cmdr, cut, names, bound))?;
    Ok(rows.iter().map(Position::from_row).collect())
}

/// Recorre hacia atrás (más reciente primero) los eventos de `names`
/// estrictamente anteriores a `before`. `f` devuelve `true` para parar.
pub fn for_each_before(
    store: &Store,
    cmdr: &str,
    cut: i64,
    names: &[&str],
    before: Option<&Position>,
    mut f: impl FnMut(&Position) -> bool,
) -> Result<(), StoreError> {
    let mut upper = before.cloned();
    loop {
        let rows = page(store, cmdr, cut, names, upper.as_ref(), false)?;
        if rows.is_empty() {
            return Ok(());
        }
        for pos in rows {
            if f(&pos) {
                return Ok(());
            }
            upper = Some(pos);
        }
    }
}

/// Primer evento (más reciente) de `names` anterior a `before`.
pub fn last_before(
    store: &Store,
    cmdr: &str,
    cut: i64,
    names: &[&str],
    before: Option<&Position>,
) -> Result<Option<Position>, StoreError> {
    let mut found = None;
    for_each_before(store, cmdr, cut, names, before, |p| {
        found = Some(p.clone());
        true
    })?;
    Ok(found)
}

/// Recorre hacia delante los eventos de `names` posteriores a `cursor`;
/// devuelve la última posición visitada (o `cursor` si no hubo ninguna).
pub fn for_each_after(
    store: &Store,
    cmdr: &str,
    cut: i64,
    names: &[&str],
    cursor: Option<Position>,
    mut f: impl FnMut(&Position),
) -> Result<Option<Position>, StoreError> {
    let mut cursor = cursor;
    loop {
        let rows = page(store, cmdr, cut, names, cursor.as_ref(), true)?;
        if rows.is_empty() {
            return Ok(cursor);
        }
        for pos in rows {
            f(&pos);
            cursor = Some(pos);
        }
    }
}
