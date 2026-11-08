//! Utilidades compartidas por los tests del crate.

use std::path::PathBuf;
use tmc_model::events::{JournalEvent, parse_line};

/// Ruta a un fixture sintético de `tests/fixtures/sync/core/`.
pub fn core_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/sync/core")
        .join(name)
}

/// Analiza una línea de diario de test.
pub fn ev(line: &str) -> JournalEvent {
    parse_line(line).expect("línea de test válida")
}
