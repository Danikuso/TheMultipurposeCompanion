use std::ffi::OsStr;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

/// Solo en depuración: simula que el juego está en marcha (pruebas e2e).
#[cfg(debug_assertions)]
static FORCE_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(debug_assertions)]
pub fn force_game_running_for_e2e(on: bool) {
    FORCE_RUNNING.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub fn is_game_running() -> bool {
    // Solo en depuración (pruebas e2e sin juego): `TMC_E2E_GAME_RUNNING=1` simula el proceso.
    #[cfg(debug_assertions)]
    if FORCE_RUNNING.load(std::sync::atomic::Ordering::Relaxed)
        || std::env::var("TMC_E2E_GAME_RUNNING").as_deref() == Ok("1")
    {
        return true;
    }
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    let running = system.processes().values().any(|process| {
        process.name() == OsStr::new("EliteDangerous64.exe")
            || process.name() == OsStr::new("EliteDangerous64")
    });
    tmc_model::diagnostic!(
        tmc_model::diagnostics::DiagnosticContext::State,
        "source.process.sample",
        tmc_model::diagnostics::json!({"running":running,"process_count":system.processes().len()})
    );
    running
}
