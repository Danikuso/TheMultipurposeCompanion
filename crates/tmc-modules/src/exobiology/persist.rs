//! Persistencia por comandante de las posiciones de muestreo (`exo_sample`).
//!
//! `ts` es la marca del diario normalizada igual que la tabla `event`
//! (RFC 3339 UTC con nanosegundos), para casar filas por `ts` durante el replay.

use super::sampling::ExoSampleRow;
use chrono::SecondsFormat;
use serde_json::{Value, json};
use tmc_core::{Migration, Store, StoreError, StoreTask};

pub const MIGRATIONS: &[Migration] = &[Migration {
    id: "exobiology-001",
    sql: "CREATE TABLE exo_sample (id INTEGER PRIMARY KEY, cmdr TEXT NOT NULL, system_address INTEGER NOT NULL, body_id INTEGER NOT NULL, species TEXT NOT NULL, scan_type TEXT NOT NULL, lat REAL, lon REAL, ts TEXT NOT NULL, UNIQUE(cmdr, system_address, body_id, species, scan_type, ts));",
}];

fn bad(e: impl std::fmt::Display) -> StoreError {
    StoreError::Unsupported(e.to_string())
}

/// Inserta la fila (idempotente por la restricción UNIQUE).
pub fn save_sample(cmdr: String, row: ExoSampleRow) -> StoreTask {
    Box::new(move |store: &Store| {
        let system = i64::try_from(row.system_address).map_err(bad)?;
        let changed = store.exec_json(
            "INSERT OR IGNORE INTO exo_sample (cmdr, system_address, body_id, species, scan_type, lat, lon, ts) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            &[
                Value::from(cmdr.as_str()),
                Value::from(system),
                Value::from(row.body_id),
                Value::from(row.species.as_str()),
                Value::from(row.scan_type.as_str()),
                row.lat.map_or(Value::Null, Value::from),
                row.lon.map_or(Value::Null, Value::from),
                Value::from(row.ts.to_rfc3339_opts(SecondsFormat::Nanos, true)),
            ],
        )?;
        Ok(json!({"cmdr": cmdr, "saved": changed}))
    })
}

/// Resultado: `{"cmdr", "rows": [ExoSampleRow]}` con el último muestreo no
/// completado de cada cuerpo, en orden cronológico.
pub fn load_open_samples(cmdr: String) -> StoreTask {
    Box::new(move |store: &Store| {
        let raw = store.query_json(
            "SELECT system_address, body_id, species, scan_type, lat, lon, ts FROM exo_sample WHERE cmdr=?1 ORDER BY ts, id",
            &[Value::from(cmdr.as_str())],
        )?;
        let rows: Vec<ExoSampleRow> = raw.iter().filter_map(|r| parse_row(&cmdr, r)).collect();
        Ok(json!({"cmdr": cmdr, "rows": open_rows(rows)}))
    })
}

fn parse_row(cmdr: &str, row: &Value) -> Option<ExoSampleRow> {
    Some(ExoSampleRow {
        cmdr: cmdr.to_owned(),
        system_address: row.get(0)?.as_u64()?,
        body_id: u32::try_from(row.get(1)?.as_u64()?).ok()?,
        species: row.get(2)?.as_str()?.to_owned(),
        scan_type: row.get(3)?.as_str()?.to_owned(),
        lat: row.get(4).and_then(Value::as_f64),
        lon: row.get(5).and_then(Value::as_f64),
        ts: chrono::DateTime::parse_from_rfc3339(row.get(6)?.as_str()?)
            .ok()?
            .to_utc(),
    })
}

/// Recorre las filas por cuerpo: un `Log` abre un muestreo nuevo y un `Analyse`
/// lo cierra; solo se conservan los muestreos aún abiertos.
fn open_rows(rows: Vec<ExoSampleRow>) -> Vec<ExoSampleRow> {
    let mut open: Vec<((u64, u32), Vec<ExoSampleRow>)> = Vec::new();
    for row in rows {
        let key = (row.system_address, row.body_id);
        let slot = open.iter().position(|(k, _)| *k == key);
        match (row.scan_type.as_str(), slot) {
            ("Log", Some(i)) => open[i].1 = vec![row],
            ("Log", None) => open.push((key, vec![row])),
            ("Analyse", Some(i)) => {
                open.remove(i);
            }
            (_, Some(i)) if open[i].1[0].species == row.species => open[i].1.push(row),
            _ => {}
        }
    }
    let mut all: Vec<ExoSampleRow> = open.into_iter().flat_map(|(_, rows)| rows).collect();
    all.sort_by_key(|row| row.ts);
    all
}

/// Borra todo el rastro del comandante (`Died`).
pub fn clear_samples(cmdr: String) -> StoreTask {
    Box::new(move |store: &Store| {
        let removed = store.exec_json(
            "DELETE FROM exo_sample WHERE cmdr=?1",
            &[Value::from(cmdr.as_str())],
        )?;
        Ok(json!({"cmdr": cmdr, "cleared": removed}))
    })
}

/// Borra los muestreos completados (con `Analyse` posterior o igual) y conserva
/// los incompletos (`SellOrganicData`).
pub fn clear_completed(cmdr: String) -> StoreTask {
    Box::new(move |store: &Store| {
        let removed = store.exec_json(
            "DELETE FROM exo_sample WHERE cmdr=?1 AND EXISTS (SELECT 1 FROM exo_sample a WHERE a.cmdr=exo_sample.cmdr AND a.system_address=exo_sample.system_address AND a.body_id=exo_sample.body_id AND a.species=exo_sample.species AND a.scan_type='Analyse' AND a.ts>=exo_sample.ts)",
            &[Value::from(cmdr.as_str())],
        )?;
        Ok(json!({"cmdr": cmdr, "cleared": removed}))
    })
}
