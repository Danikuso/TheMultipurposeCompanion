//! Tipos de la sincronización con servicios externos (Inara, EDSM, EDDN) de F17.

use crate::companion::CompanionFile;
use crate::events::JournalEvent;
use crate::state::Galaxy;
use serde::{Deserialize, Serialize};

/// Marcador que `tmc-replay play` deja en la carpeta de destino: con él presente no se sincroniza nada.
pub const REPLAY_MARKER: &str = ".tmc-replay";

/// Servicio externo al que se sincroniza.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncService {
    Inara,
    Edsm,
    Eddn,
}

impl SyncService {
    pub const ALL: [SyncService; 3] = [SyncService::Inara, SyncService::Edsm, SyncService::Eddn];

    pub fn as_str(self) -> &'static str {
        match self {
            SyncService::Inara => "inara",
            SyncService::Edsm => "edsm",
            SyncService::Eddn => "eddn",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|service| service.as_str() == s)
    }

    /// Antigüedad máxima de un evento pendiente antes de caducar.
    pub fn max_age(self) -> chrono::Duration {
        match self {
            SyncService::Eddn => chrono::Duration::hours(24),
            SyncService::Inara | SyncService::Edsm => chrono::Duration::days(7),
        }
    }
}

/// Origen de un evento del feed: en vivo o relectura del diario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedOrigin {
    Live,
    CatchUp,
}

/// Origen de una fila de la bandeja de salida.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowOrigin {
    Live,
    Recovered,
    Snapshot,
}

impl RowOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            RowOrigin::Live => "live",
            RowOrigin::Recovered => "recovered",
            RowOrigin::Snapshot => "snapshot",
        }
    }
}

/// Estado de una fila de la bandeja de salida.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxState {
    Pending,
    Inflight,
    Sent,
    Rejected,
    Expired,
    Superseded,
    Discarded,
    Omitted,
}

impl OutboxState {
    pub fn as_str(self) -> &'static str {
        match self {
            OutboxState::Pending => "pending",
            OutboxState::Inflight => "inflight",
            OutboxState::Sent => "sent",
            OutboxState::Rejected => "rejected",
            OutboxState::Expired => "expired",
            OutboxState::Superseded => "superseded",
            OutboxState::Discarded => "discarded",
            OutboxState::Omitted => "omitted",
        }
    }
}

/// Evento del feed de sincronización. Líneas 1-based; cursor ausente = 0.
#[derive(Debug, Clone)]
// El evento del diario es grande, pero el feed se mueve por valor y no se acumula.
#[allow(clippy::large_enum_variant)]
pub enum SyncFeedEvent {
    Journal {
        ev: JournalEvent,
        file: String,
        line: u64,
    },
    Companion(CompanionFile),
    GameProcess {
        running: bool,
    },
    CatchUpDone,
}

/// Evento del feed con su contexto.
#[derive(Debug, Clone)]
pub struct SyncFeed {
    pub event: SyncFeedEvent,
    pub origin: FeedOrigin,
    pub galaxy: Galaxy,
    pub game_running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    Disabled,
    Paused,
    Waiting,
    Connected,
    Error,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncReason {
    NoKey,
    SwitchOff,
    OnlineOff,
    Legacy,
    Beta,
    Replay,
    GameNotRunning,
    OtherInstance,
    PlatformUnsupported,
    KeyUnassigned,
    Network,
    Timeout,
    RateLimited,
    Blocked,
    Auth,
    SoftwareBlocked,
    SchemaOutdated,
    ServerError,
    DiscardListMissing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorClass {
    Network,
    Timeout,
    RateLimited,
    Blocked,
    Auth,
    SoftwareBlocked,
    SchemaOutdated,
    InvalidEvent,
    TooLarge,
    ServerError,
    DiscardListMissing,
}

/// Rechazo reciente mostrado al piloto.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentRejection {
    pub kind: String,
    pub class: ErrorClass,
    pub code: String,
    pub at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Máximo de rechazos recientes conservados por servicio.
pub const RECENT_REJECTIONS_MAX: usize = 20;
/// Máximo de caracteres de una pista devuelta por el servidor.
pub const SERVER_HINT_MAX_CHARS: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncServiceStatus {
    pub state: SyncState,
    pub reason: Option<SyncReason>,
    pub pending: u64,
    pub recovered_session: u64,
    pub sent_total: u64,
    pub sent_session: u64,
    pub rejected_total: u64,
    pub rejected_session: u64,
    pub expired_total: u64,
    pub omitted_total: u64,
    pub last_confirmed_at: Option<String>,
    pub next_attempt_at: Option<String>,
    pub recent_rejections: Vec<RecentRejection>,
}

impl Default for SyncServiceStatus {
    fn default() -> Self {
        Self {
            state: SyncState::Disabled,
            reason: None,
            pending: 0,
            recovered_session: 0,
            sent_total: 0,
            sent_session: 0,
            rejected_total: 0,
            rejected_session: 0,
            expired_total: 0,
            omitted_total: 0,
            last_confirmed_at: None,
            next_attempt_at: None,
            recent_rejections: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CmdrRef {
    pub cmdr_ref: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyAssignment {
    Confirmed,
    AutoPendingConfirmation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommanderRow {
    pub cmdr_ref: String,
    pub name: String,
    pub inara_key: bool,
    pub edsm_key: bool,
    pub key_assignment: Option<KeyAssignment>,
    pub inara_pending: u64,
    pub edsm_pending: u64,
    pub eddn_pending: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingByService {
    pub inara: u64,
    pub edsm: u64,
    pub eddn: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyedServiceStatus {
    #[serde(flatten)]
    pub status: SyncServiceStatus,
    pub has_key: bool,
    pub key_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EddnServiceStatus {
    #[serde(flatten)]
    pub status: SyncServiceStatus,
    pub enabled: bool,
    pub paused_schemas: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RosterReport {
    pub current_cmdr: Option<CmdrRef>,
    pub commanders: Vec<CommanderRow>,
    pub other_cmdrs_pending: PendingByService,
    pub unassigned_keys: Vec<SyncService>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncReport {
    Inara(KeyedServiceStatus),
    Edsm(KeyedServiceStatus),
    Eddn(EddnServiceStatus),
    Roster(RosterReport),
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn max_age_per_service() {
        assert_eq!(SyncService::Eddn.max_age(), Duration::hours(24));
        assert_eq!(SyncService::Edsm.max_age(), Duration::days(7));
        assert_eq!(SyncService::Inara.max_age(), Duration::days(7));
    }

    #[test]
    fn service_and_state_round_trip() {
        assert_eq!(SyncService::parse("edsm"), Some(SyncService::Edsm));
        assert_eq!(SyncService::parse("x"), None);
        for service in SyncService::ALL {
            assert_eq!(SyncService::parse(service.as_str()), Some(service));
        }
        assert_eq!(
            serde_json::to_value(SyncReason::DiscardListMissing).unwrap(),
            "discard_list_missing"
        );
        assert_eq!(OutboxState::Superseded.as_str(), "superseded");
        assert_eq!(RowOrigin::Recovered.as_str(), "recovered");
    }

    #[test]
    fn keyed_status_flattens() {
        let value = serde_json::to_value(KeyedServiceStatus {
            status: SyncServiceStatus::default(),
            has_key: true,
            key_hint: Some("ab12".into()),
        })
        .unwrap();
        assert_eq!(value["state"], "disabled");
        assert_eq!(value["pending"], 0);
        assert_eq!(value["has_key"], true);
        assert_eq!(value["key_hint"], "ab12");
        assert!(value.get("status").is_none());
    }
}
