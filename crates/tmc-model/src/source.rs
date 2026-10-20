//! Eventos que las fuentes (diario, Status.json, archivos compañeros…) envían al actor.

use crate::companion::CompanionFile;
use crate::events::JournalEvent;
use crate::net::NetResponse;
use crate::settings::ModuleSettings;
use crate::status::Status;
use crate::theme::HudTheme;
use chrono::{DateTime, Utc};

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
    /// Aviso ligero del servicio EDDN: los datos viven en la tabla de precios.
    Eddn(EddnNotice),
    Stored {
        module: String,
        tag: String,
        value: serde_json::Value,
    },
}

/// Tope de mercancías listadas en un aviso EDDN.
pub const EDDN_NOTICE_MAX_COMMODITIES: usize = 200;

/// Mercados escritos por EDDN y estado de la conexión.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EddnNotice {
    pub markets: Vec<u64>,
    /// Claves de mercancía tocadas (acotado a [`EDDN_NOTICE_MAX_COMMODITIES`]).
    pub commodities: Vec<String>,
    pub connected: bool,
    pub at: DateTime<Utc>,
}
