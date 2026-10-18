//! Constantes AX y seguimiento de actividad contra thargoides (todo NO VERIFICADO, ver spike).

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use tmc_model::events::{EventKind, LocationInfo};

/// Símbolos de `ShipTargeted.Ship` thargoides. NO VERIFICADO: viene de fuentes secundarias
/// (ver `docs/research/notes/spike_ax_f2.md` §4); los valores desconocidos se toleran.
pub const THARGOID_SHIPS: &[&str] = &[
    "scout", "scout_hq", "scout_q", "scout_c", "scout_nq", "cyclops", "basilisk", "medusa",
    "hydra", "scythe", "glaive", "orthrus",
];

/// `true` si el símbolo de nave (en minúsculas o no) es thargoide.
pub fn is_thargoid_ship(key: &str) -> bool {
    let k = key.trim().to_ascii_lowercase();
    THARGOID_SHIPS.contains(&k.as_str())
}

/// `true` si la facción es la thargoide (`$faction_Thargoid;`). NO VERIFICADO.
pub fn is_thargoid_faction(f: &str) -> bool {
    f.trim().eq_ignore_ascii_case("$faction_Thargoid;")
}

/// Minutos de actividad AX que mantienen el panel activo sin presencia confirmada.
const ACTIVE_MINUTES: i64 = 5;

/// Tipo bajo el que se cuentan los bonos AX sin nave thargoide asociada (nunca se descartan).
pub const UNKNOWN_TYPE: &str = "unknown";

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct AxTracker {
    pub kills_by_type: BTreeMap<String, u32>,
    pub bonds: u64,
    pub presence: bool,
    pub last_activity: Option<DateTime<Utc>>,
    /// `Ship` del último `ShipTargeted` thargoide, para atribuir el siguiente bono.
    pub last_target_ship: Option<String>,
}

impl AxTracker {
    /// Aplica el evento; devuelve `true` si cambió el estado.
    pub fn apply_event(&mut self, ev: &EventKind, now: DateTime<Utc>) -> bool {
        match ev {
            EventKind::ShipTargeted(t) if t.target_locked => {
                let Some(ship) = t.ship.as_deref().map(str::to_ascii_lowercase) else {
                    return false;
                };
                let tg = is_thargoid_ship(&ship)
                    || t.faction.as_deref().is_some_and(is_thargoid_faction);
                if !tg {
                    return false;
                }
                self.last_activity = Some(now);
                self.last_target_ship.replace(ship.clone()) != Some(ship)
            }
            EventKind::FactionKillBond(k) | EventKind::CapShipBond(k)
                if k.victim_faction.as_deref().is_some_and(is_thargoid_faction) =>
            {
                let kind = self
                    .last_target_ship
                    .take()
                    .unwrap_or_else(|| UNKNOWN_TYPE.to_owned());
                *self.kills_by_type.entry(kind).or_default() += 1;
                self.bonds = self.bonds.saturating_add(k.reward);
                self.last_activity = Some(now);
                true
            }
            _ => false,
        }
    }

    /// Actualiza la presencia thargoide desde `Location`/`FSDJump`.
    pub fn on_location(&mut self, loc: &LocationInfo) -> bool {
        let p = loc.thargoid_presence();
        let changed = self.presence != p;
        self.presence = p;
        changed
    }

    /// Presencia o actividad en los últimos 5 minutos.
    pub fn active(&self, now: DateTime<Utc>) -> bool {
        self.presence
            || self
                .last_activity
                .is_some_and(|t| now - t <= Duration::minutes(ACTIVE_MINUTES))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tmc_model::events::{FactionKillBond, ShipTargeted, parse_line};

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 18, 12, 0, 0).unwrap()
    }

    fn target(ship: &str) -> EventKind {
        EventKind::ShipTargeted(Box::new(ShipTargeted {
            target_locked: true,
            ship: Some(ship.into()),
            hull_health: Some(55.0),
            ..Default::default()
        }))
    }

    fn tg_bond(reward: u64) -> EventKind {
        EventKind::FactionKillBond(FactionKillBond {
            reward,
            awarding_faction: Some("Aegis".into()),
            victim_faction: Some("$faction_Thargoid;".into()),
            ..Default::default()
        })
    }

    fn loc(json: &str) -> LocationInfo {
        match parse_line(json).unwrap().kind {
            EventKind::Location(l) | EventKind::FsdJump(l) => *l,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn ax_kills_by_type() {
        let mut ax = AxTracker::default();
        assert!(ax.apply_event(&target("Scout"), now()));
        assert!(ax.apply_event(&tg_bond(6_000_000), now()));
        assert_eq!(ax.kills_by_type.get("scout"), Some(&1));
        assert_eq!(ax.bonds, 6_000_000);
        assert_eq!(ax.last_target_ship, None);
    }

    #[test]
    fn unrecognized_bond_is_counted_under_unknown() {
        let mut ax = AxTracker::default();
        ax.apply_event(&tg_bond(100), now());
        assert_eq!(ax.kills_by_type.get(UNKNOWN_TYPE), Some(&1));
        ax.apply_event(&target("cyclops"), now());
        ax.apply_event(&tg_bond(200), now());
        ax.apply_event(&tg_bond(300), now());
        assert_eq!(ax.kills_by_type.get("cyclops"), Some(&1));
        assert_eq!(ax.kills_by_type.get(UNKNOWN_TYPE), Some(&2));
        assert_eq!(ax.bonds, 600);
    }

    #[test]
    fn human_target_and_bond_are_ignored() {
        let mut ax = AxTracker::default();
        assert!(!ax.apply_event(&target("anaconda"), now()));
        let human = EventKind::FactionKillBond(FactionKillBond {
            reward: 5,
            victim_faction: Some("B".into()),
            ..Default::default()
        });
        assert!(!ax.apply_event(&human, now()));
        assert_eq!(ax, AxTracker::default());
    }

    #[test]
    fn thargoid_presence_from_location() {
        let mut ax = AxTracker::default();
        let war = loc(
            r#"{"timestamp":"2026-10-18T12:00:00Z","event":"Location","StarSystem":"Kappa","ThargoidWar":{"CurrentState":"Thargoid_Probing"}}"#,
        );
        assert!(ax.on_location(&war));
        assert!(ax.presence);
        assert!(!ax.on_location(&war));
        let clean =
            loc(r#"{"timestamp":"2026-10-18T12:05:00Z","event":"FSDJump","StarSystem":"Sol"}"#);
        assert!(ax.on_location(&clean));
        assert!(!ax.presence);
    }

    #[test]
    fn active_by_presence_or_recent_activity() {
        let mut ax = AxTracker::default();
        assert!(!ax.active(now()));
        ax.apply_event(&tg_bond(1), now());
        assert!(ax.active(now() + Duration::minutes(5)));
        assert!(!ax.active(now() + Duration::minutes(5) + Duration::seconds(1)));
        ax.presence = true;
        assert!(ax.active(now() + Duration::hours(1)));
    }
}
