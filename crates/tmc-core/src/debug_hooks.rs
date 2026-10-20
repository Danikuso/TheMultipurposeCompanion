//! Ganchos solo de depuración para pruebas e2e: registran cuándo se ejecutan las tareas de
//! almacén por etiqueta. No existen en compilaciones de release.

use std::sync::Mutex;
use std::time::Instant;

const CAP: usize = 4096;
static STORE_TASKS: Mutex<Vec<(Instant, String)>> = Mutex::new(Vec::new());

pub(crate) fn record_store_task(tag: &str) {
    if let Ok(mut log) = STORE_TASKS.lock() {
        if log.len() >= CAP {
            log.remove(0);
        }
        log.push((Instant::now(), tag.to_owned()));
    }
}

/// Instantes en que se ejecutó una tarea de almacén con esa etiqueta.
pub fn store_task_times(tag: &str) -> Vec<Instant> {
    STORE_TASKS
        .lock()
        .map(|log| {
            log.iter()
                .filter(|(_, t)| t == tag)
                .map(|(at, _)| *at)
                .collect()
        })
        .unwrap_or_default()
}
