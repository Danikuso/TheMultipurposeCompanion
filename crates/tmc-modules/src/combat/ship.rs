//! Mi nave: escudos, casco por escalones, calor de tres estados y pips exactos.
//!
//! Nota para el cableado (Task 16): `view(now)` cambia solo con el paso del
//! tiempo (caducidad de `HeatDamage` a los 10 s, `UnderAttack` a los 30 s), así
//! que hay que volver a renderizar con un tick periódico o en el siguiente evento.

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use tmc_model::events::{EventKind, HullDamage};
use tmc_model::status::{Flags, Status};

/// Segundos sin otro `HeatDamage` tras los que el estado vuelve a `Overheating`.
const HEAT_DAMAGE_SECS: i64 = 10;
/// Segundos que `UnderAttack` permanece como alerta.
const UNDER_ATTACK_SECS: i64 = 30;
/// Porcentaje de casco por debajo del cual se avisa.
const HULL_LOW_PCT: u8 = 25;

/// Pips en unidades 0.0–4.0 con paso 0.5.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Pips {
    pub sys: f32,
    pub eng: f32,
    pub wep: f32,
}

impl Pips {
    pub fn from_half_pips(p: [u8; 3]) -> Pips {
        let f = |v: u8| f32::from(v.min(8)) / 2.0;
        Pips {
            sys: f(p[0]),
            eng: f(p[1]),
            wep: f(p[2]),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HeatState {
    #[default]
    Normal,
    Overheating,
    HeatDamage,
}

#[derive(Debug, Default)]
pub struct HeatTracker {
    state: HeatState,
    since: Option<DateTime<Utc>>,
    last_damage: Option<DateTime<Utc>>,
    total_s: u64,
}

impl HeatTracker {
    fn enter(&mut self, s: HeatState, now: DateTime<Utc>) {
        self.state = s;
        self.since.get_or_insert(now);
    }

    fn leave(&mut self, now: DateTime<Utc>) {
        if let Some(since) = self.since.take() {
            self.total_s += (now - since).num_seconds().max(0) as u64;
        }
        self.state = HeatState::Normal;
        self.last_damage = None;
    }

    fn damage_expired(&self, now: DateTime<Utc>) -> bool {
        self.last_damage
            .is_none_or(|d| d + Duration::seconds(HEAT_DAMAGE_SECS) <= now)
    }

    /// Aplica el flag `OVERHEATING` de Status; `true` si el estado cambió.
    pub fn on_status(&mut self, overheating: bool, now: DateTime<Utc>) -> bool {
        let before = self.state;
        if !overheating {
            if before != HeatState::Normal {
                self.leave(now);
            }
        } else if before == HeatState::Normal {
            self.enter(HeatState::Overheating, now);
        } else if before == HeatState::HeatDamage && self.damage_expired(now) {
            self.state = HeatState::Overheating;
        }
        self.state != before
    }

    pub fn on_heat_warning(&mut self, now: DateTime<Utc>) -> bool {
        if self.state == HeatState::Normal {
            self.enter(HeatState::Overheating, now);
            return true;
        }
        false
    }

    pub fn on_heat_damage(&mut self, now: DateTime<Utc>) -> bool {
        let visible = self.state_at(now);
        self.last_damage = Some(now);
        self.enter(HeatState::HeatDamage, now);
        visible != HeatState::HeatDamage
    }

    pub fn state(&self) -> HeatState {
        self.state
    }

    /// Estado efectivo en `now` (el daño por calor caduca a los 10 s).
    pub fn state_at(&self, now: DateTime<Utc>) -> HeatState {
        if self.state == HeatState::HeatDamage && self.damage_expired(now) {
            HeatState::Overheating
        } else {
            self.state
        }
    }

    pub fn overheating_since(&self) -> Option<DateTime<Utc>> {
        self.since
    }

    /// Segundos acumulados de episodios de sobrecalentamiento ya terminados.
    pub fn overheat_total_s(&self) -> u64 {
        self.total_s
    }

    fn reset(&mut self, now: DateTime<Utc>) -> bool {
        let changed = self.state != HeatState::Normal;
        self.leave(now);
        changed
    }
}

#[derive(Debug, Default)]
pub struct HullTracker {
    pct: Option<u8>,
}

fn to_pct(v: f64) -> u8 {
    (v * 100.0).round().clamp(0.0, 100.0) as u8
}

/// `Wear` es integridad de la nave, no casco; `All` repara todo.
fn is_hull_item(s: &str) -> bool {
    matches!(s.to_ascii_lowercase().as_str(), "hull" | "all")
}

impl HullTracker {
    /// Aplica el evento; `true` si el casco visible cambió.
    pub fn apply(&mut self, ev: &EventKind) -> bool {
        let new = match ev {
            EventKind::Loadout(l) => l.hull_health.map(to_pct).or(self.pct),
            EventKind::HullDamage(HullDamage {
                health: Some(h),
                player_pilot,
                fighter,
            }) if *fighter != Some(true) && *player_pilot != Some(false) => Some(to_pct(*h)),
            EventKind::RepairAll(_) | EventKind::Resurrect => Some(100),
            EventKind::Repair(r) => {
                if r.item.iter().chain(r.items.iter()).any(|i| is_hull_item(i)) {
                    Some(100)
                } else {
                    self.pct
                }
            }
            EventKind::Died => None,
            _ => self.pct,
        };
        let changed = new != self.pct;
        self.pct = new;
        changed
    }

    pub fn pct(&self) -> Option<u8> {
        self.pct
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShipAlert {
    HullLow,
    ShieldsDown,
    Overheating,
    UnderAttack,
    Interdiction,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ShipView {
    pub shields_up: Option<bool>,
    pub hull_pct: Option<u8>,
    pub heat: HeatState,
    pub overheating_since: Option<DateTime<Utc>>,
    pub overheat_total_s: u64,
    pub pips: Option<Pips>,
    pub hardpoints: bool,
    pub alerts: Vec<ShipAlert>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StatusKey {
    shields_up: bool,
    overheating: bool,
    hardpoints: bool,
    interdicted: bool,
    pips: Option<[u8; 3]>,
}

#[derive(Debug, Default)]
pub struct ShipTracker {
    heat: HeatTracker,
    hull: HullTracker,
    shields_up: Option<bool>,
    hardpoints: bool,
    interdicted: bool,
    pips: Option<[u8; 3]>,
    last_key: Option<StatusKey>,
    under_attack_at: Option<DateTime<Utc>>,
}

impl ShipTracker {
    /// Aplica Status.json; `true` si algo visible cambió.
    pub fn on_status(&mut self, s: &Status, now: DateTime<Utc>) -> bool {
        let key = StatusKey {
            shields_up: s.flags.contains(Flags::SHIELDS_UP),
            overheating: s.flags.contains(Flags::OVERHEATING),
            hardpoints: s.flags.contains(Flags::HARDPOINTS_DEPLOYED),
            interdicted: s.flags.contains(Flags::BEING_INTERDICTED),
            pips: s.pips,
        };
        let heat_changed = self.heat.on_status(key.overheating, now);
        if self.last_key == Some(key) {
            return heat_changed;
        }
        self.last_key = Some(key);
        self.shields_up = Some(key.shields_up);
        self.hardpoints = key.hardpoints;
        self.interdicted = key.interdicted;
        self.pips = key.pips;
        true
    }

    pub fn on_event(&mut self, ev: &EventKind, now: DateTime<Utc>) -> bool {
        let mut changed = self.hull.apply(ev);
        match ev {
            EventKind::ShieldState(s) => {
                changed |= self.shields_up != Some(s.shields_up);
                self.shields_up = Some(s.shields_up);
            }
            EventKind::HeatWarning => changed |= self.heat.on_heat_warning(now),
            EventKind::HeatDamage => changed |= self.heat.on_heat_damage(now),
            EventKind::UnderAttack(_) => {
                self.under_attack_at = Some(now);
                changed = true;
            }
            EventKind::Died => {
                changed |= self.heat.reset(now);
                changed |= self.shields_up.is_some()
                    || self.interdicted
                    || self.hardpoints
                    || self.under_attack_at.is_some();
                self.shields_up = None;
                self.interdicted = false;
                self.hardpoints = false;
                self.under_attack_at = None;
                self.last_key = None;
            }
            _ => {}
        }
        changed
    }

    pub fn view(&self, now: DateTime<Utc>) -> ShipView {
        let heat = self.heat.state_at(now);
        let hull_pct = self.hull.pct();
        let mut alerts = Vec::new();
        if hull_pct.is_some_and(|p| p < HULL_LOW_PCT) {
            alerts.push(ShipAlert::HullLow);
        }
        if self.shields_up == Some(false) {
            alerts.push(ShipAlert::ShieldsDown);
        }
        if heat != HeatState::Normal {
            alerts.push(ShipAlert::Overheating);
        }
        if self
            .under_attack_at
            .is_some_and(|t| now >= t && now < t + Duration::seconds(UNDER_ATTACK_SECS))
        {
            alerts.push(ShipAlert::UnderAttack);
        }
        if self.interdicted {
            alerts.push(ShipAlert::Interdiction);
        }
        ShipView {
            shields_up: self.shields_up,
            hull_pct,
            heat,
            overheating_since: self.heat.overheating_since(),
            overheat_total_s: self.heat.overheat_total_s(),
            pips: self.pips.map(Pips::from_half_pips),
            hardpoints: self.hardpoints,
            alerts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tmc_model::events::{Loadout, Repair, RepairAll};
    use tmc_model::status::Flags2;

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 18, 12, 0, 0).unwrap()
    }
    fn secs(n: i64) -> DateTime<Utc> {
        t0() + Duration::seconds(n)
    }
    fn status(flags: Flags, pips: Option<[u8; 3]>) -> Status {
        Status {
            timestamp: t0(),
            flags,
            flags2: Flags2::empty(),
            pips,
            fire_group: None,
            gui_focus: Default::default(),
            fuel: None,
            cargo: None,
            legal_state: None,
            latitude: None,
            longitude: None,
            altitude: None,
            heading: None,
            body_name: None,
            planet_radius: None,
            balance: None,
            destination: None,
            oxygen: None,
            health: None,
            temperature: None,
            gravity: None,
            selected_weapon: None,
        }
    }
    fn loadout(h: f64) -> EventKind {
        EventKind::Loadout(Loadout {
            hull_health: Some(h),
            ..Default::default()
        })
    }
    fn hull_dmg(h: f64, fighter: bool) -> EventKind {
        EventKind::HullDamage(HullDamage {
            health: Some(h),
            player_pilot: Some(!fighter),
            fighter: Some(fighter),
        })
    }

    #[test]
    fn pips_half_units_exact() {
        let p = Pips::from_half_pips([4, 8, 0]);
        assert_eq!((p.sys, p.eng, p.wep), (2.0, 4.0, 0.0));
        let p = Pips::from_half_pips([3, 5, 4]);
        assert_eq!((p.sys, p.eng, p.wep), (1.5, 2.5, 2.0));
        let p = Pips::from_half_pips([8, 2, 2]);
        assert_eq!((p.sys, p.eng, p.wep), (4.0, 1.0, 1.0));
    }

    #[test]
    fn hull_steps_follow_last_known_value() {
        let mut h = HullTracker::default();
        assert!(h.apply(&loadout(1.0)));
        assert_eq!(h.pct(), Some(100));
        assert!(h.apply(&hull_dmg(0.8, false)));
        assert_eq!(h.pct(), Some(80));
        assert!(h.apply(&hull_dmg(0.6, false)));
        assert_eq!(h.pct(), Some(60));
        assert!(h.apply(&EventKind::RepairAll(RepairAll::default())));
        assert_eq!(h.pct(), Some(100));
        h.apply(&hull_dmg(0.5, false));
        h.apply(&EventKind::Repair(Repair {
            item: Some("Hull".into()),
            ..Default::default()
        }));
        assert_eq!(h.pct(), Some(100));
        h.apply(&hull_dmg(0.5, false));
        h.apply(&EventKind::Repair(Repair {
            items: vec!["Cockpit".into()],
            ..Default::default()
        }));
        assert_eq!(h.pct(), Some(50));
    }

    #[test]
    fn fighter_hull_damage_is_ignored() {
        let mut h = HullTracker::default();
        h.apply(&loadout(1.0));
        assert!(!h.apply(&hull_dmg(0.2, true)));
        assert_eq!(h.pct(), Some(100));
    }

    #[test]
    fn heat_three_states_and_timer() {
        let mut h = HeatTracker::default();
        assert!(h.on_status(true, t0()));
        assert_eq!(h.state(), HeatState::Overheating);
        assert_eq!(h.overheating_since(), Some(t0()));
        assert!(h.on_heat_damage(secs(3)));
        assert_eq!(h.state(), HeatState::HeatDamage);
        assert_eq!(h.overheating_since(), Some(t0()));
        assert!(h.on_status(false, secs(8)));
        assert_eq!(h.state(), HeatState::Normal);
        assert_eq!(h.overheating_since(), None);
        assert_eq!(h.overheat_total_s(), 8);
    }

    #[test]
    fn heat_damage_expires_after_10s_without_repeat() {
        let mut h = HeatTracker::default();
        h.on_heat_damage(t0());
        assert_eq!(h.state(), HeatState::HeatDamage);
        assert!(!h.on_status(true, secs(5)));
        assert_eq!(h.state(), HeatState::HeatDamage);
        assert!(h.on_status(true, secs(11)));
        assert_eq!(h.state(), HeatState::Overheating);
    }

    #[test]
    fn alerts_hull_low_and_shields_down() {
        let mut s = ShipTracker::default();
        s.on_event(&loadout(1.0), t0());
        s.on_event(&hull_dmg(0.2, false), t0());
        s.on_status(&status(Flags::empty(), None), t0());
        let v = s.view(t0());
        assert_eq!(v.hull_pct, Some(20));
        assert_eq!(v.alerts, vec![ShipAlert::HullLow, ShipAlert::ShieldsDown]);
    }

    #[test]
    fn identical_status_burst_marks_ship_dirty_once() {
        let mut s = ShipTracker::default();
        let st = status(Flags::SHIELDS_UP, Some([4, 8, 0]));
        let dirty = (0..500).filter(|_| s.on_status(&st, t0())).count();
        assert_eq!(dirty, 1);
        let v = s.view(t0());
        assert_eq!(v.shields_up, Some(true));
        assert_eq!(v.pips.unwrap().eng, 4.0);
        assert!(s.on_status(&status(Flags::SHIELDS_UP, Some([4, 6, 2])), t0()));
    }

    #[test]
    fn died_clears_hull_and_heat() {
        let mut s = ShipTracker::default();
        s.on_event(&loadout(0.5), t0());
        s.on_status(&status(Flags::OVERHEATING, None), t0());
        assert_eq!(s.view(t0()).heat, HeatState::Overheating);
        assert!(s.on_event(&EventKind::Died, secs(4)));
        let v = s.view(secs(4));
        assert_eq!(v.hull_pct, None);
        assert_eq!(v.heat, HeatState::Normal);
        assert_eq!(v.overheating_since, None);
        assert_eq!(v.overheat_total_s, 4);
    }

    #[test]
    fn under_attack_lasts_30s_and_interdiction_follows_flag() {
        let mut s = ShipTracker::default();
        s.on_status(&status(Flags::SHIELDS_UP, None), t0());
        s.on_event(&EventKind::UnderAttack(Default::default()), t0());
        assert_eq!(s.view(secs(29)).alerts, vec![ShipAlert::UnderAttack]);
        assert!(s.view(secs(30)).alerts.is_empty());
        let flags = Flags::SHIELDS_UP | Flags::BEING_INTERDICTED;
        s.on_status(&status(flags, None), secs(31));
        assert_eq!(s.view(secs(31)).alerts, vec![ShipAlert::Interdiction]);
    }

    #[test]
    fn expired_heat_damage_then_new_damage_reports_change() {
        let mut h = HeatTracker::default();
        assert!(h.on_heat_damage(t0()));
        assert!(!h.on_heat_damage(secs(5)));
        assert_eq!(h.state_at(secs(20)), HeatState::Overheating);
        assert!(h.on_heat_damage(secs(20)));
        assert_eq!(h.state_at(secs(20)), HeatState::HeatDamage);
    }

    #[test]
    fn repair_hull_items_rules() {
        let rep = |item: &str| {
            let mut h = HullTracker::default();
            h.apply(&hull_dmg(0.5, false));
            h.apply(&EventKind::Repair(Repair {
                item: Some(item.into()),
                ..Default::default()
            }));
            h.pct()
        };
        assert_eq!(rep("Hull"), Some(100));
        assert_eq!(rep("hull"), Some(100));
        assert_eq!(rep("All"), Some(100));
        assert_eq!(rep("Wear"), Some(50));
        let mut h = HullTracker::default();
        h.apply(&hull_dmg(0.3, false));
        assert!(h.apply(&EventKind::Resurrect));
        assert_eq!(h.pct(), Some(100));
    }

    #[test]
    fn died_clears_ship_flags() {
        let mut s = ShipTracker::default();
        let flags = Flags::SHIELDS_UP | Flags::BEING_INTERDICTED;
        s.on_status(&status(flags, None), t0());
        s.on_event(&EventKind::UnderAttack(Default::default()), t0());
        s.on_event(&EventKind::Died, secs(1));
        let v = s.view(secs(2));
        assert_eq!(v.shields_up, None);
        assert!(v.alerts.is_empty());
        assert!(s.on_status(&status(flags, None), secs(3)));
    }

    #[test]
    fn view_changes_with_time_alone() {
        let mut s = ShipTracker::default();
        s.on_status(&status(Flags::SHIELDS_UP | Flags::OVERHEATING, None), t0());
        s.on_event(&EventKind::HeatDamage, t0());
        assert_eq!(s.view(secs(9)).heat, HeatState::HeatDamage);
        assert_eq!(s.view(secs(11)).heat, HeatState::Overheating);
    }

    #[test]
    fn flapping_overheat_accumulates_total() {
        let mut h = HeatTracker::default();
        h.on_status(true, t0());
        h.on_status(false, secs(5));
        h.on_status(true, secs(7));
        h.on_status(false, secs(9));
        assert_eq!(h.overheat_total_s(), 7);
        assert_eq!(h.overheating_since(), None);
    }
}
