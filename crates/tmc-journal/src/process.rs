use std::ffi::OsStr;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

pub fn is_game_running() -> bool {
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
