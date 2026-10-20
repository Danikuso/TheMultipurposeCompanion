//! Pila de masacre: reparto de bajas entre misiones según la regla del juego (spec §10.10).
//!
//! Solo cuentan los `Bounty` (y los bonos de zona de conflicto si
//! [`MASSACRE_COUNTS_KILL_BONDS`]) cuya facción víctima coincide con la facción
//! objetivo de la misión y que ocurren después de aceptarla. Cada baja se asigna,
//! por facción emisora, a su misión activa más antigua aún incompleta.

use super::book::{Mission, MissionBook, MissionKind};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use tmc_model::events::EventKind;

/// El spike no confirmó que `FactionKillBond` cuente para la masacre.
pub const MASSACRE_COUNTS_KILL_BONDS: bool = false;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IssuerBar {
    pub faction: String,
    pub kills: u32,
    pub needed: u32,
    pub missions: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MassacrePile {
    pub target_faction: String,
    pub issuers: Vec<IssuerBar>,
    /// Máximo por emisor de `needed - kills`.
    pub kills_remaining: u32,
    pub total_reward: u64,
    /// Hay misiones de esta pila sin detalles (no reciben bajas).
    pub details_missing: bool,
}

/// Única fuente de bajas de las misiones de masacre.
#[derive(Debug, Clone, Default)]
pub struct MassacreStack {
    kills: BTreeMap<u64, u32>,
    complete: BTreeSet<u64>,
}

fn norm(s: &str) -> String {
    s.trim().to_lowercase()
}

fn needed(m: &Mission) -> u32 {
    m.progress.kills_needed.unwrap_or(0)
}

impl MassacreStack {
    /// Recuento local real (una redirigida con 7/10 sigue en 7); la UI usa `view`,
    /// que muestra las redirigidas como completas.
    pub fn kills_for(&self, mission_id: u64) -> u32 {
        self.kills.get(&mission_id).copied().unwrap_or(0)
    }

    /// Debe llamarse con el libro tal como estaba justo antes de aplicar el evento.
    pub fn apply_event(&mut self, ev: &EventKind, at: DateTime<Utc>, book: &MissionBook) {
        match ev {
            EventKind::Bounty(b) => self.assign(b.victim_faction.as_deref(), at, book),
            EventKind::FactionKillBond(b) if MASSACRE_COUNTS_KILL_BONDS => {
                self.assign(b.victim_faction.as_deref(), at, book)
            }
            EventKind::MissionRedirected(r) => {
                if let Some(m) = book
                    .get(r.mission_id)
                    .filter(|m| m.kind == MissionKind::Massacre)
                {
                    let short = self.kills_for(r.mission_id) < needed(m);
                    self.complete.insert(r.mission_id);
                    decide!("missions.massacre.redirected_complete", {
                        "mission": r.mission_id,
                        "short": short,
                    });
                }
            }
            EventKind::MissionCompleted(m) => self.forget(m.mission_id),
            EventKind::MissionFailed(m) | EventKind::MissionAbandoned(m) => {
                self.forget(m.mission_id)
            }
            _ => {}
        }
    }

    fn forget(&mut self, id: u64) {
        self.kills.remove(&id);
        self.complete.remove(&id);
    }

    /// Vacía el estado (cambio de comandante).
    pub fn clear(&mut self) {
        self.kills.clear();
        self.complete.clear();
    }

    fn assign(&mut self, victim: Option<&str>, at: DateTime<Utc>, book: &MissionBook) {
        let Some(victim) = victim.map(norm).filter(|v| !v.is_empty()) else {
            return;
        };
        // Candidata por emisor: (accepted_at, id) mínimo entre las no completas.
        let mut best: BTreeMap<String, (DateTime<Utc>, u64)> = BTreeMap::new();
        let mut matched = false;
        for m in book.active() {
            if m.kind != MissionKind::Massacre {
                continue;
            }
            let (Some(acc), Some(target)) = (m.accepted_at, m.target_faction.as_deref()) else {
                continue;
            };
            if norm(target) != victim || at <= acc {
                continue;
            }
            matched = true;
            if self.complete.contains(&m.id) || self.kills_for(m.id) >= needed(m) {
                continue;
            }
            let issuer = norm(m.faction.as_deref().unwrap_or(""));
            let key = (acc, m.id);
            best.entry(issuer)
                .and_modify(|c| {
                    if key < *c {
                        *c = key;
                    }
                })
                .or_insert(key);
        }
        if !matched {
            decide!("missions.massacre.no_matching_mission", {"matched": false});
            return;
        }
        for (_, id) in best.into_values() {
            *self.kills.entry(id).or_insert(0) += 1;
            decide!("missions.massacre.kill_assigned", {"mission": id});
        }
    }

    pub fn view(&self, book: &MissionBook) -> Vec<MassacrePile> {
        struct Acc {
            name: String,
            issuers: BTreeMap<String, IssuerBar>,
            reward: u64,
            missing: bool,
        }
        let mut piles: BTreeMap<String, Acc> = BTreeMap::new();
        for m in book.active().filter(|m| m.kind == MissionKind::Massacre) {
            let Some(target) = m.target_faction.as_deref().filter(|t| !t.trim().is_empty()) else {
                continue;
            };
            let pile = piles.entry(norm(target)).or_insert_with(|| Acc {
                name: target.trim().to_string(),
                issuers: BTreeMap::new(),
                reward: 0,
                missing: false,
            });
            pile.reward += m.reward.unwrap_or(0);
            pile.missing |= m.details_missing || m.accepted_at.is_none();
            if m.accepted_at.is_none() {
                // Sin detalles: no suma a barras ni al restante, solo la marca.
                continue;
            }
            let need = needed(m);
            let kills = if self.complete.contains(&m.id) {
                need
            } else {
                self.kills_for(m.id).min(need)
            };
            // Emisor desconocido: clave propia "?", nunca fusionado con una facción real.
            let faction = m.faction.clone().unwrap_or_else(|| "?".into());
            let bar = pile
                .issuers
                .entry(norm(&faction))
                .or_insert_with(|| IssuerBar {
                    faction: faction.trim().to_string(),
                    kills: 0,
                    needed: 0,
                    missions: 0,
                });
            bar.kills += kills;
            bar.needed += need;
            bar.missions += 1;
        }
        piles
            .into_values()
            .map(|p| {
                let issuers: Vec<IssuerBar> = p.issuers.into_values().collect();
                let kills_remaining = issuers
                    .iter()
                    .map(|i| i.needed.saturating_sub(i.kills))
                    .max()
                    .unwrap_or(0);
                MassacrePile {
                    target_faction: p.name,
                    issuers,
                    kills_remaining,
                    total_reward: p.reward,
                    details_missing: p.missing,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tmc_model::events::{
        Bounty, MissionAccepted, MissionRedirected, MissionRef, MissionsList, parse_line,
    };

    fn at(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 20, h, m, 0).unwrap()
    }
    fn mass(id: u64, faction: &str, target: &str, n: u32) -> MissionAccepted {
        MissionAccepted {
            mission_id: id,
            name: "Mission_Massacre".into(),
            faction: Some(faction.into()),
            target_faction: Some(target.into()),
            kill_count: Some(n),
            reward: Some(1000),
            ..Default::default()
        }
    }
    fn accept(b: &mut MissionBook, m: MissionAccepted, when: DateTime<Utc>) {
        b.apply_event(&EventKind::MissionAccepted(Box::new(m)), when);
    }
    fn bounty(victim: &str) -> EventKind {
        EventKind::Bounty(Box::new(Bounty {
            victim_faction: Some(victim.into()),
            ..Default::default()
        }))
    }
    fn kill(s: &mut MassacreStack, b: &MissionBook, victim: &str, when: DateTime<Utc>) {
        s.apply_event(&bounty(victim), when, b);
    }

    #[test]
    fn one_kill_counts_once_per_issuer_oldest_first() {
        let mut b = MissionBook::default();
        accept(&mut b, mass(1, "A", "X", 10), at(12, 0));
        accept(&mut b, mass(2, "A", "X", 5), at(12, 1));
        accept(&mut b, mass(3, "B", "X", 8), at(12, 2));
        let mut s = MassacreStack::default();
        for i in 0..12 {
            kill(&mut s, &b, "X", at(13, i));
        }
        assert_eq!(s.kills_for(1), 10);
        assert_eq!(s.kills_for(2), 2);
        assert_eq!(s.kills_for(3), 8);
        let v = s.view(&b);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].kills_remaining, 3);
        assert_eq!(v[0].total_reward, 3000);
    }

    #[test]
    fn kill_before_accept_not_counted() {
        let mut b = MissionBook::default();
        accept(&mut b, mass(1, "A", "X", 10), at(12, 0));
        let mut s = MassacreStack::default();
        kill(&mut s, &b, "X", at(11, 59));
        assert_eq!(s.kills_for(1), 0);
        kill(&mut s, &b, " x ", at(12, 1));
        assert_eq!(s.kills_for(1), 1);
    }

    #[test]
    fn other_target_faction_is_other_pile() {
        let mut b = MissionBook::default();
        accept(&mut b, mass(1, "A", "X", 10), at(12, 0));
        accept(&mut b, mass(2, "A", "Y", 10), at(12, 0));
        let mut s = MassacreStack::default();
        kill(&mut s, &b, "X", at(13, 0));
        assert_eq!(s.kills_for(1), 1);
        assert_eq!(s.kills_for(2), 0);
        assert_eq!(s.view(&b).len(), 2);
    }

    #[test]
    fn redirected_marks_complete_even_if_count_short() {
        let mut b = MissionBook::default();
        accept(&mut b, mass(1, "A", "X", 10), at(12, 0));
        accept(&mut b, mass(2, "A", "X", 5), at(12, 1));
        let mut s = MassacreStack::default();
        for i in 0..7 {
            kill(&mut s, &b, "X", at(13, i));
        }
        let red = EventKind::MissionRedirected(MissionRedirected {
            mission_id: 1,
            new_destination_system: Some("Home".into()),
            ..Default::default()
        });
        s.apply_event(&red, at(13, 10), &b);
        b.apply_event(&red, at(13, 10));
        kill(&mut s, &b, "X", at(13, 11));
        assert_eq!(s.kills_for(1), 7);
        assert_eq!(s.kills_for(2), 1);
        let v = s.view(&b);
        assert_eq!(v[0].issuers[0].kills, 11);
        assert_eq!(v[0].kills_remaining, 4);
    }

    #[test]
    fn tie_on_accept_time_goes_to_lower_mission_id() {
        let mut b = MissionBook::default();
        accept(&mut b, mass(65000005, "A", "X", 3), at(12, 0));
        accept(&mut b, mass(65000004, "A", "X", 3), at(12, 0));
        let mut s = MassacreStack::default();
        kill(&mut s, &b, "X", at(13, 0));
        assert_eq!(s.kills_for(65000004), 1);
        assert_eq!(s.kills_for(65000005), 0);
    }

    #[test]
    fn mission_without_accept_time_gets_no_kills() {
        let mut b = MissionBook::default();
        b.apply_event(
            &EventKind::Missions(Box::new(MissionsList {
                active: vec![MissionRef {
                    mission_id: 1,
                    name: "Mission_Massacre_name".into(),
                    ..Default::default()
                }],
                ..Default::default()
            })),
            at(12, 0),
        );
        accept(&mut b, mass(2, "A", "X", 5), at(12, 1));
        let mut s = MassacreStack::default();
        for i in 0..3 {
            kill(&mut s, &b, "X", at(13, i));
        }
        assert_eq!(s.kills_for(1), 0);
        assert_eq!(s.kills_for(2), 3);
    }

    #[test]
    fn fixture_five_missions_three_factions_matches_manual() {
        let fixture = include_str!("../../../../tests/fixtures/journal/massacre_session.log");
        let mut b = MissionBook::default();
        let mut s = MassacreStack::default();
        for line in fixture.lines() {
            let ev = parse_line(line).unwrap();
            s.apply_event(&ev.kind, ev.timestamp, &b);
            b.apply_event(&ev.kind, ev.timestamp);
        }
        // Cálculo manual: 12 bajas de X tras aceptar (la de 11:59 no cuenta).
        // A: 10/10 + 2/5; B: 8/8; C(X): 10/10 (las bajas 11 y 12 no tienen misión libre).
        assert_eq!(s.kills_for(65000001), 10);
        assert_eq!(s.kills_for(65000002), 2);
        assert_eq!(s.kills_for(65000003), 8);
        assert_eq!(s.kills_for(65000004), 10);
        assert_eq!(s.kills_for(65000005), 3);
        let v = s.view(&b);
        let x = v
            .iter()
            .find(|p| p.target_faction == "Pirate Clan X")
            .unwrap();
        let bar = |n: &str| x.issuers.iter().find(|i| i.faction == n).unwrap().clone();
        assert_eq!(
            (bar("Alpha Union").kills, bar("Alpha Union").needed),
            (12, 15)
        );
        assert_eq!((bar("Beta Corp").kills, bar("Beta Corp").needed), (8, 8));
        assert_eq!(
            (bar("Gamma Party").kills, bar("Gamma Party").needed),
            (10, 10)
        );
        assert_eq!(x.kills_remaining, 3);
        let y = v
            .iter()
            .find(|p| p.target_faction == "Pirate Clan Y")
            .unwrap();
        assert_eq!(y.kills_remaining, 1);
    }

    #[test]
    fn completed_mid_stream_moves_to_next_and_overflow_ignored() {
        let mut b = MissionBook::default();
        accept(&mut b, mass(1, "A", "X", 2), at(12, 0));
        accept(&mut b, mass(2, "A", "X", 2), at(12, 1));
        let mut s = MassacreStack::default();
        kill(&mut s, &b, "X", at(13, 0));
        let mut c = Box::<tmc_model::events::MissionCompleted>::default();
        c.mission_id = 1;
        let ev = EventKind::MissionCompleted(c);
        s.apply_event(&ev, at(13, 1), &b);
        b.apply_event(&ev, at(13, 1));
        for i in 2..8 {
            kill(&mut s, &b, "X", at(13, i));
        }
        assert_eq!(s.kills_for(2), 2);
        assert_eq!(s.view(&b)[0].kills_remaining, 0);
    }

    #[test]
    fn issuer_names_case_and_spaces_group_and_equal_time_not_counted() {
        let mut b = MissionBook::default();
        accept(&mut b, mass(1, "Alpha", "X", 5), at(12, 0));
        accept(&mut b, mass(2, " alpha ", "X", 5), at(12, 1));
        let mut s = MassacreStack::default();
        kill(&mut s, &b, "X", at(12, 0));
        assert_eq!(s.kills_for(1), 0);
        kill(&mut s, &b, "X", at(12, 5));
        assert_eq!(s.kills_for(1), 1);
        let v = s.view(&b);
        assert_eq!(v[0].issuers.len(), 1);
        assert_eq!(v[0].issuers[0].missions, 2);
    }

    #[test]
    fn missing_details_excluded_from_remaining_and_insertion_order_irrelevant() {
        let mut b = MissionBook::default();
        b.apply_event(
            &EventKind::Missions(Box::new(MissionsList {
                active: vec![MissionRef {
                    mission_id: 9,
                    name: "Mission_Massacre_name".into(),
                    ..Default::default()
                }],
                ..Default::default()
            })),
            at(12, 0),
        );
        accept(&mut b, mass(2, "A", "X", 5), at(12, 1));
        let v = MassacreStack::default().view(&b);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].kills_remaining, 5);

        let run = |order: [(u64, u32); 2]| {
            let mut b = MissionBook::default();
            for (id, m) in order {
                accept(&mut b, mass(id, "A", "X", 3), at(12, m));
            }
            let mut s = MassacreStack::default();
            kill(&mut s, &b, "X", at(13, 0));
            (s.kills_for(1), s.kills_for(2))
        };
        assert_eq!(run([(1, 0), (2, 0)]), run([(2, 0), (1, 0)]));
        assert_eq!(run([(1, 0), (2, 0)]), (1, 0));
    }
}
