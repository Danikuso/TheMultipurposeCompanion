//! Zona de conflicto: bando, bajas y bonos por facción. Los bonos thargoides son de AX.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use tmc_model::events::EventKind;

use super::ax::is_thargoid_faction;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConflictZone {
    pub system: String,
    pub side: Option<String>,
    pub kills: u32,
    pub bonds_by_faction: BTreeMap<String, u64>,
}

#[derive(Debug, Default)]
pub struct CzTracker {
    zone: Option<ConflictZone>,
}

impl CzTracker {
    /// Aplica el evento; devuelve `true` si cambió la zona visible.
    /// El bando es el `AwardingFaction` del primer bono (los `Comms` no se tipan en F2).
    pub fn apply_event(&mut self, ev: &EventKind, system: &str, _now: DateTime<Utc>) -> bool {
        match ev {
            EventKind::FsdJump(_) | EventKind::Docked(_) | EventKind::Died => {
                self.zone.take().is_some()
            }
            EventKind::FactionKillBond(k) | EventKind::CapShipBond(k) => {
                if k.victim_faction.as_deref().is_some_and(is_thargoid_faction) {
                    return false;
                }
                let faction = k.awarding_faction.clone();
                let zone = self.zone.get_or_insert_with(|| ConflictZone {
                    system: system.to_owned(),
                    side: faction.clone(),
                    kills: 0,
                    bonds_by_faction: BTreeMap::new(),
                });
                zone.kills += 1;
                let key = faction.unwrap_or_else(|| "unknown".to_owned());
                let slot = zone.bonds_by_faction.entry(key).or_default();
                *slot = slot.saturating_add(k.reward);
                true
            }
            _ => false,
        }
    }

    pub fn view(&self) -> Option<ConflictZone> {
        self.zone.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::ax::AxTracker;
    use chrono::TimeZone;
    use tmc_model::events::FactionKillBond;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 18, 12, 0, 0).unwrap()
    }

    fn bond(reward: u64, award: &str, victim: &str) -> EventKind {
        EventKind::FactionKillBond(FactionKillBond {
            reward,
            awarding_faction: Some(award.into()),
            victim_faction: Some(victim.into()),
            ..Default::default()
        })
    }

    #[test]
    fn cz_side_and_bonds_by_faction() {
        let mut cz = CzTracker::default();
        assert!(cz.apply_event(&bond(20000, "A", "B"), "Kappa", now()));
        assert!(cz.apply_event(&bond(30000, "A", "B"), "Kappa", now()));
        let v = cz.view().unwrap();
        assert_eq!(v.system, "Kappa");
        assert_eq!(v.side.as_deref(), Some("A"));
        assert_eq!(v.kills, 2);
        assert_eq!(v.bonds_by_faction.get("A"), Some(&50000));
    }

    #[test]
    fn cz_closes_on_jump_dock_and_death() {
        for closer in [
            EventKind::FsdJump(Box::default()),
            EventKind::Docked(Default::default()),
            EventKind::Died,
        ] {
            let mut cz = CzTracker::default();
            cz.apply_event(&bond(1, "A", "B"), "Kappa", now());
            assert!(cz.apply_event(&closer, "Kappa", now()));
            assert_eq!(cz.view(), None);
            assert!(!cz.apply_event(&closer, "Kappa", now()));
        }
    }

    #[test]
    fn thargoid_bond_goes_to_ax_not_cz() {
        let ev = bond(6_000_000, "Aegis", "$faction_Thargoid;");
        let mut cz = CzTracker::default();
        let mut ax = AxTracker::default();
        assert!(!cz.apply_event(&ev, "Kappa", now()));
        assert!(ax.apply_event(&ev, now()));
        assert_eq!(cz.view(), None);
        assert_eq!(ax.bonds, 6_000_000);
    }
}
