//! Reglas de avisos del panel general.

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use std::collections::BTreeMap;
use tmc_core::{Alert, Severity};
use tmc_model::{
    events::{EventKind, JournalEvent},
    status::{Flags, Flags2, Status},
};

pub const DEBOUNCE_WINDOW: Duration = Duration::seconds(30);
pub const EVENT_ALERT_TTL: Duration = Duration::seconds(60);
pub const MAX_ACTIVE_ALERTS: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSource {
    StatusFlag,
    JournalEvent,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActiveAlert {
    pub id: String,
    pub severity: Severity,
    pub key: String,
    pub args: BTreeMap<String, serde_json::Value>,
    pub since: DateTime<Utc>,
    #[serde(skip)]
    pub source: AlertSource,
}

impl ActiveAlert {
    pub fn from_alert(alert: &Alert, since: DateTime<Utc>, source: AlertSource) -> Self {
        Self {
            id: alert.id.clone(),
            severity: alert.severity,
            key: alert.key.clone(),
            args: alert.args.clone(),
            since,
            source,
        }
    }
}

pub fn status_flag_rules(status: &Status) -> [(&'static str, Severity, bool); 5] {
    [
        (
            "alert-low-fuel",
            Severity::Warn,
            status.flags.contains(Flags::LOW_FUEL),
        ),
        (
            "alert-overheating",
            Severity::Warn,
            status.flags.contains(Flags::OVERHEATING),
        ),
        (
            "alert-interdiction",
            Severity::Critical,
            status.flags.contains(Flags::BEING_INTERDICTED),
        ),
        (
            "alert-low-oxygen",
            Severity::Critical,
            status.flags2.contains(Flags2::LOW_OXYGEN),
        ),
        (
            "alert-low-health",
            Severity::Critical,
            status.flags2.contains(Flags2::LOW_HEALTH),
        ),
    ]
}

pub fn journal_event_alert(ev: &JournalEvent) -> Option<Alert> {
    match &ev.kind {
        EventKind::UnderAttack(_) => Some(Alert::new(Severity::Warn, "alert-under-attack")),
        EventKind::HullDamage(damage) => {
            if damage.player_pilot == Some(false) {
                return None;
            }
            let health = damage.health?;
            let severity = if health <= 0.25 {
                Severity::Critical
            } else {
                Severity::Warn
            };
            let pct = (health * 100.0).round() as i64;
            Some(Alert::new(severity, "alert-hull-damage").arg("pct", pct))
        }
        _ => None,
    }
}

pub fn is_session_reset(ev: &JournalEvent) -> bool {
    matches!(
        ev.kind,
        EventKind::Fileheader(_) | EventKind::LoadGame(_) | EventKind::Shutdown
    )
}
