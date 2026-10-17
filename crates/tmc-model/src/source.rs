//! Eventos que las fuentes (diario, Status.json, archivos compañeros…) envían al actor.

use crate::companion::CompanionFile;
use crate::events::JournalEvent;
use crate::net::NetResponse;
use crate::settings::ModuleSettings;
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
    Net(NetResponse),
    Action {
        name: String,
        args: serde_json::Value,
    },
    Settings(ModuleSettings),
    Stored {
        module: String,
        tag: String,
        value: serde_json::Value,
    },
}
