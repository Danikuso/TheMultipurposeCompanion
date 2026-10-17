//! Eventos que las fuentes (diario, Status.json, archivos compañeros…) envían al actor.

use crate::companion::CompanionFile;
use crate::events::JournalEvent;
use crate::status::Status;
use crate::theme::HudTheme;

#[derive(Debug, Clone)]
pub enum SourceEvent {
    Journal {
        ev: JournalEvent,
        file: String,
        line: u64,
        replaying: bool,
    },
    Status(Status),
    Companion(CompanionFile),
    GameProcess {
        running: bool,
    },
    Theme(HudTheme),
    ImportProgress {
        done: u64,
        total: u64,
    },
}
