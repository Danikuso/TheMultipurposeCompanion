//! Coalescencia de filas `set*` para Inara antes de cada envío.

use std::collections::HashMap;
use tmc_store::OutboxRow;

/// Mantiene únicamente la fila más reciente por `coalesce_key` para eventos `set*`.
/// Los eventos `add*` (y filas sin `coalesce_key`) nunca se fusionan.
pub fn coalesce(rows: Vec<OutboxRow>) -> (Vec<OutboxRow>, Vec<i64>) {
    let mut latest_by_key: HashMap<String, usize> = HashMap::new();
    for (idx, row) in rows.iter().enumerate() {
        if !row.kind.starts_with("set") {
            continue;
        }
        let Some(ck) = row.coalesce_key.as_deref().filter(|k| !k.is_empty()) else {
            continue;
        };
        match latest_by_key.get(ck).copied() {
            Some(prev_idx) => {
                let prev = &rows[prev_idx];
                if (row.event_ts, row.id) >= (prev.event_ts, prev.id) {
                    latest_by_key.insert(ck.to_string(), idx);
                }
            }
            None => {
                latest_by_key.insert(ck.to_string(), idx);
            }
        }
    }

    let mut kept = Vec::with_capacity(rows.len());
    let mut superseded = Vec::new();
    for (idx, row) in rows.into_iter().enumerate() {
        if row.kind.starts_with("set")
            && let Some(ck) = row.coalesce_key.as_deref().filter(|k| !k.is_empty())
            && latest_by_key.get(ck).copied() != Some(idx)
        {
            superseded.push(row.id);
            continue;
        }
        kept.push(row);
    }
    (kept, superseded)
}
