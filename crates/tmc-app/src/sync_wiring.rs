//! Adaptador de plataforma y arranque de `tmc-sync` dentro de `tmc-app`.

use std::ffi::OsString;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tmc_model::source::SourceEvent;
use tmc_net::Transport;
use tmc_store::Store;
use tmc_sync::{EnvGates, LiveSignalPolicy, SyncDeps, SyncHandle, SystemClock, spawn_sync};
use tokio::sync::mpsc;

const E2E_GAME_RUNNING_ENV: &str = "TMC_E2E_GAME_RUNNING";

pub(crate) fn env_gates_from(lookup: impl FnOnce(&str) -> Option<OsString>) -> EnvGates {
    EnvGates {
        e2e_simulation: lookup(E2E_GAME_RUNNING_ENV).is_some(),
    }
}

/// Guardas de entorno comprobadas en cualquier compilación (PD-9).
pub fn env_gates() -> EnvGates {
    env_gates_from(|k| std::env::var_os(k))
}

/// Señal de juego en vivo por plataforma (ruling T0-1).
pub fn live_signal_for_platform() -> LiveSignalPolicy {
    if cfg!(target_os = "windows") {
        LiveSignalPolicy::ProcessCheck
    } else if cfg!(target_os = "linux") {
        LiveSignalPolicy::JournalActivity
    } else {
        LiveSignalPolicy::Unavailable
    }
}

/// Compone e inicia el subsistema de sincronización para el proceso actual.
pub fn start_sync(
    store: &Store,
    transport: Arc<dyn Transport>,
    online: Arc<AtomicBool>,
    core_events: mpsc::Sender<SourceEvent>,
    journal_dir: &Path,
    process_kind: &'static str,
) -> SyncHandle {
    spawn_sync(SyncDeps {
        store: store.clone(),
        transport,
        online,
        core_events,
        journal_dir: journal_dir.to_path_buf(),
        env: env_gates(),
        live_signal: live_signal_for_platform(),
        clock: Arc::new(SystemClock),
        process_kind,
        clients: Vec::new(),
        mappers: Vec::new(),
    })
}
