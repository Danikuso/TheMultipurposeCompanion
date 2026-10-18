//! Objetivo actual: estado legal, subsistema y salud.

use super::ax::is_thargoid_ship;
use crate::data::ship_pads;
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use tmc_model::events::ShipTargeted;

/// Segundos que el objetivo perdido sigue visible.
const LOST_VISIBLE_SECS: i64 = 10;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LegalBadge {
    Scanning,
    Wanted { bounty: u64 },
    Clean,
    Lawless,
    Enemy,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubsystemKind {
    PowerPlant,
    Drive,
    Fsd,
    LifeSupport,
    Distributor,
    Sensors,
    Weapons,
    CargoHatch,
    Other,
}

impl SubsystemKind {
    pub fn from_journal(s: &str) -> Self {
        let s = s.to_ascii_lowercase();
        if s.contains("powerplant") {
            Self::PowerPlant
        } else if s.contains("hyperdrive") || s.contains("fsd") {
            Self::Fsd
        } else if s.contains("engine") || s.contains("drive") {
            Self::Drive
        } else if s.contains("lifesupport") {
            Self::LifeSupport
        } else if s.contains("powerdistributor") || s.contains("distributor") {
            Self::Distributor
        } else if s.contains("sensors") {
            Self::Sensors
        } else if s.contains("cargobay") || s.contains("cargohatch") {
            Self::CargoHatch
        } else if s.contains("hardpoint") || s.contains("weapon") {
            Self::Weapons
        } else {
            Self::Other
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SubsystemView {
    pub kind: SubsystemKind,
    pub name: Option<String>,
    pub health_pct: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TargetView {
    pub ship_key: String,
    pub ship_name: Option<String>,
    pub pilot: Option<String>,
    pub rank: Option<String>,
    pub faction: Option<String>,
    pub badge: LegalBadge,
    pub shield_pct: Option<f64>,
    pub hull_pct: Option<f64>,
    pub subsystem: Option<SubsystemView>,
    pub thargoid: bool,
    pub lost_at: Option<DateTime<Utc>>,
}

fn badge_for(ev: &ShipTargeted) -> LegalBadge {
    if ev.scan_stage.unwrap_or(0) < 3 {
        return LegalBadge::Scanning;
    }
    let bounty = ev.bounty.unwrap_or(0);
    let wanted = LegalBadge::Wanted { bounty };
    match ev.legal_status.as_deref() {
        Some("Wanted") => wanted,
        _ if bounty > 0 => wanted,
        Some("Clean") => LegalBadge::Clean,
        Some("Lawless") => LegalBadge::Lawless,
        Some("Enemy" | "WantedEnemy" | "Hostile") => LegalBadge::Enemy,
        _ => LegalBadge::Unknown,
    }
}

#[derive(Debug, Default)]
pub struct TargetTracker {
    current: Option<TargetView>,
}

impl TargetTracker {
    /// Aplica el evento; devuelve `true` si el estado visible cambió.
    pub fn apply(&mut self, ev: &ShipTargeted, now: DateTime<Utc>) -> bool {
        if !ev.target_locked {
            return match self.current.as_mut() {
                Some(t) if t.lost_at.is_none() => {
                    t.lost_at = Some(now);
                    true
                }
                _ => false,
            };
        }
        let key = ev.ship.as_deref().unwrap_or("").to_ascii_lowercase();
        let ship_name = ev
            .ship_localised
            .clone()
            .or_else(|| ship_pads().display_name(&key).map(str::to_owned));
        let subsystem = ev.subsystem.as_deref().map(|s| SubsystemView {
            kind: SubsystemKind::from_journal(s),
            name: ev.subsystem_localised.clone(),
            health_pct: ev.subsystem_health,
        });
        let view = TargetView {
            thargoid: is_thargoid_ship(&key),
            ship_key: key,
            ship_name,
            pilot: ev
                .pilot_name_localised
                .clone()
                .or_else(|| ev.pilot_name.clone()),
            rank: ev.pilot_rank.clone(),
            faction: ev.faction.clone(),
            badge: badge_for(ev),
            shield_pct: ev.shield_health,
            hull_pct: ev.hull_health,
            subsystem,
            lost_at: None,
        };
        if self.current.as_ref() == Some(&view) {
            return false;
        }
        self.current = Some(view);
        true
    }

    /// Objetivo visible; `None` si no hay o se perdió hace 10 s o más.
    pub fn view(&self, now: DateTime<Utc>) -> Option<TargetView> {
        let t = self.current.as_ref()?;
        if let Some(lost) = t.lost_at
            && lost + Duration::seconds(LOST_VISIBLE_SECS) <= now
        {
            return None;
        }
        Some(t.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 18, 12, 0, 0).unwrap()
    }

    fn lock(stage: u8, legal: &str, bounty: Option<u64>) -> ShipTargeted {
        ShipTargeted {
            target_locked: true,
            ship: Some("anaconda".into()),
            scan_stage: Some(stage),
            legal_status: Some(legal.into()),
            bounty,
            hull_health: Some(80.0),
            ..Default::default()
        }
    }

    fn badge(ev: &ShipTargeted) -> LegalBadge {
        let mut t = TargetTracker::default();
        t.apply(ev, t0());
        t.view(t0()).unwrap().badge
    }

    #[test]
    fn wanted_target_shows_bounty_badge() {
        assert_eq!(
            badge(&lock(3, "Wanted", Some(125000))),
            LegalBadge::Wanted { bounty: 125000 }
        );
    }

    #[test]
    fn clean_target_shows_clean_badge() {
        assert_eq!(badge(&lock(3, "Clean", None)), LegalBadge::Clean);
    }

    #[test]
    fn scan_stage_below_3_is_scanning() {
        assert_eq!(badge(&lock(1, "Wanted", None)), LegalBadge::Scanning);
    }

    #[test]
    fn unknown_legal_status_is_unknown_badge() {
        assert_eq!(badge(&lock(3, "Foo", None)), LegalBadge::Unknown);
    }

    #[test]
    fn subsystem_kind_and_health() {
        let mut ev = lock(3, "Clean", None);
        ev.subsystem = Some("$int_powerplant_size3_class5_name;".into());
        ev.subsystem_health = Some(62.0);
        let mut t = TargetTracker::default();
        t.apply(&ev, t0());
        let s = t.view(t0()).unwrap().subsystem.unwrap();
        assert_eq!(s.kind, SubsystemKind::PowerPlant);
        assert_eq!(s.health_pct, Some(62.0));
    }

    #[test]
    fn target_lost_sets_lost_at_and_keeps_data() {
        let mut t = TargetTracker::default();
        t.apply(&lock(3, "Clean", None), t0());
        let lost = ShipTargeted::default();
        assert!(t.apply(&lost, t0()));
        let v = t.view(t0() + Duration::seconds(9)).unwrap();
        assert_eq!(v.hull_pct, Some(80.0));
        assert_eq!(v.lost_at, Some(t0()));
        assert_eq!(v.ship_name.as_deref(), Some("Anaconda"));
    }

    #[test]
    fn view_clears_10s_after_loss() {
        let mut t = TargetTracker::default();
        t.apply(&lock(3, "Clean", None), t0());
        t.apply(&ShipTargeted::default(), t0());
        assert_eq!(t.view(t0() + Duration::seconds(10)), None);
    }

    #[test]
    fn new_lock_after_loss_resets_lost_at() {
        let mut t = TargetTracker::default();
        t.apply(&lock(3, "Clean", None), t0());
        t.apply(&ShipTargeted::default(), t0());
        assert!(t.apply(&lock(3, "Clean", None), t0() + Duration::seconds(5)));
        assert_eq!(t.view(t0() + Duration::seconds(5)).unwrap().lost_at, None);
    }

    #[test]
    fn legal_badge_serializes_tagged() {
        let w = serde_json::to_string(&LegalBadge::Wanted { bounty: 125000 }).unwrap();
        assert_eq!(w, r#"{"kind":"wanted","bounty":125000}"#);
        assert_eq!(
            serde_json::to_string(&LegalBadge::Clean).unwrap(),
            r#"{"kind":"clean"}"#
        );
    }

    #[test]
    fn identical_targeted_event_reports_no_change() {
        let mut t = TargetTracker::default();
        let ev = lock(3, "Clean", None);
        assert!(t.apply(&ev, t0()));
        assert!(!t.apply(&ev, t0()));
    }

    #[test]
    fn thargoid_ship_detected_and_unknown_tolerated() {
        assert!(is_thargoid_ship("Cyclops"));
        assert!(!is_thargoid_ship("anaconda"));
        assert_eq!(SubsystemKind::from_journal("$weird;"), SubsystemKind::Other);
    }
}
