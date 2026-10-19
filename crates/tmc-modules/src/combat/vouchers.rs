//! Libro de vales sin cobrar (recompensas por bounty y bonos de combate).
//!
//! Lógica pura: sin E/S. Reglas en la spec F2 §10.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tmc_model::events::{EventKind, RedeemVoucher};

use super::ax::is_thargoid_faction;

/// Spike AX §5 (ruling `BROKER_AMOUNT_IS_NET`): `Amount` se usa tal cual; el
/// `BrokerPercentage` es solo informativo.
pub const BROKER_AMOUNT_IS_NET: bool = false;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoucherKind {
    Bounty,
    CombatBond,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoucherEntry {
    pub kind: VoucherKind,
    pub faction: String,
    pub amount: u64,
    pub system: String,
    pub earned_at: DateTime<Utc>,
    pub ax: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FactionTotal {
    pub kind: VoucherKind,
    pub faction: String,
    pub amount: u64,
    pub systems: Vec<String>,
    pub ax: bool,
}

/// Valor nominal de un canje: si `Amount` es neto se reconstruye
/// `Amount / (1 - pct/100)`; si no, se usa tal cual.
pub fn nominal(amount: u64, pct: f64, is_net: bool) -> u64 {
    if !is_net || !(0.0..100.0).contains(&pct) {
        return amount;
    }
    (amount as f64 / (1.0 - pct / 100.0)).round() as u64
}

/// Clave de comparación tolerante a símbolos de localización.
fn faction_key(f: &str) -> String {
    let lower = f.trim().to_lowercase();
    match lower
        .strip_prefix("$faction_")
        .and_then(|t| t.strip_suffix(';'))
    {
        Some(inner) => inner.trim().to_string(),
        None => lower,
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VoucherLedger {
    entries: Vec<VoucherEntry>,
}

/// Tipo de vale que canjea el evento; `None` si no es bounty ni bono.
pub fn redeem_kind(r: &RedeemVoucher) -> Option<VoucherKind> {
    match r.type_.as_str() {
        "bounty" => Some(VoucherKind::Bounty),
        "CombatBond" => Some(VoucherKind::CombatBond),
        _ => None,
    }
}

/// Importe nominal que resta el libro al canjear: suma de `Factions[]` si
/// existe; si no, `Amount` (con la regla del corredor de `nominal`).
pub fn declared_amount(r: &RedeemVoucher) -> u64 {
    if r.factions.is_empty() {
        nominal(
            r.amount,
            r.broker_percentage.unwrap_or(0.0),
            BROKER_AMOUNT_IS_NET,
        )
    } else {
        r.factions
            .iter()
            .fold(0u64, |acc, f| acc.saturating_add(f.amount))
    }
}

impl VoucherLedger {
    fn push(&mut self, e: VoucherEntry) {
        let idx = self.entries.partition_point(|x| x.earned_at <= e.earned_at);
        self.entries.insert(idx, e);
    }

    /// Resta `amount` del tipo (y facción si se da), de más antiguo a más
    /// nuevo. Devuelve el importe que no se pudo restar.
    fn subtract(&mut self, kind: VoucherKind, faction: Option<&str>, mut amount: u64) -> u64 {
        let key = faction.map(faction_key);
        for e in self.entries.iter_mut() {
            if amount == 0 {
                break;
            }
            if e.kind != kind || key.as_ref().is_some_and(|k| *k != faction_key(&e.faction)) {
                continue;
            }
            let take = e.amount.min(amount);
            e.amount -= take;
            amount -= take;
        }
        self.entries.retain(|e| e.amount > 0);
        amount
    }

    /// Resta por facción; el sobrante se aplica a vales del tipo sin facción.
    fn subtract_faction(&mut self, kind: VoucherKind, faction: &str, amount: u64) {
        let left = self.subtract(kind, Some(faction), amount);
        if left > 0 {
            self.subtract(kind, Some(""), left);
        }
    }

    fn redeem(&mut self, r: &RedeemVoucher) -> bool {
        let Some(kind) = redeem_kind(r) else {
            return false;
        };
        if !r.factions.is_empty() {
            for f in &r.factions {
                self.subtract_faction(kind, &f.faction, f.amount);
            }
        } else {
            let amount = declared_amount(r);
            let faction = r.faction.as_deref().filter(|f| !f.is_empty());
            match faction {
                Some(f) => self.subtract_faction(kind, f, amount),
                None => {
                    self.subtract(kind, None, amount);
                }
            }
        }
        true
    }

    /// Aplica un evento. Devuelve `true` si el evento afecta al libro.
    pub fn apply_event(&mut self, ev: &EventKind, system: &str, now: DateTime<Utc>) -> bool {
        match ev {
            EventKind::Bounty(b) => {
                let ax = b.victim_faction.as_deref().is_some_and(is_thargoid_faction);
                let mut changed = false;
                for r in b.normalized_rewards() {
                    if r.reward > 0 {
                        self.push(VoucherEntry {
                            kind: VoucherKind::Bounty,
                            faction: r.faction,
                            amount: r.reward,
                            system: system.to_string(),
                            earned_at: now,
                            ax,
                        });
                        changed = true;
                    }
                }
                changed
            }
            EventKind::FactionKillBond(k) | EventKind::CapShipBond(k) => {
                if k.reward == 0 {
                    return false;
                }
                self.push(VoucherEntry {
                    kind: VoucherKind::CombatBond,
                    faction: k.awarding_faction.clone().unwrap_or_default(),
                    amount: k.reward,
                    system: system.to_string(),
                    earned_at: now,
                    ax: k.victim_faction.as_deref().is_some_and(is_thargoid_faction),
                });
                true
            }
            EventKind::RedeemVoucher(r) => self.redeem(r),
            EventKind::Died => {
                let had = !self.entries.is_empty();
                self.reset();
                had
            }
            _ => false,
        }
    }

    /// Totales por (tipo, facción, AX), en orden de primera aparición.
    pub fn unredeemed(&self) -> Vec<FactionTotal> {
        let mut out: Vec<(String, FactionTotal)> = Vec::new();
        for e in &self.entries {
            let key = faction_key(&e.faction);
            let found = out
                .iter_mut()
                .find(|(k, t)| *k == key && t.kind == e.kind && t.ax == e.ax);
            match found {
                Some((_, t)) => {
                    t.amount = t.amount.saturating_add(e.amount);
                    if !t.systems.contains(&e.system) {
                        t.systems.push(e.system.clone());
                    }
                }
                None => out.push((
                    key,
                    FactionTotal {
                        kind: e.kind,
                        faction: e.faction.clone(),
                        amount: e.amount,
                        systems: vec![e.system.clone()],
                        ax: e.ax,
                    },
                )),
            }
        }
        out.into_iter().map(|(_, t)| t).collect()
    }

    pub fn total(&self, kind: VoucherKind) -> u64 {
        self.entries
            .iter()
            .filter(|e| e.kind == kind)
            .fold(0u64, |acc, e| acc.saturating_add(e.amount))
    }

    pub fn reset(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tmc_model::events::{Bounty, BountyReward, FactionKillBond, VoucherFaction};

    fn t(s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 10, 20, 0, s).unwrap()
    }

    fn bounty(rewards: &[(&str, u64)]) -> EventKind {
        EventKind::Bounty(Box::new(Bounty {
            rewards: rewards
                .iter()
                .map(|(f, r)| BountyReward {
                    faction: f.to_string(),
                    reward: *r,
                })
                .collect(),
            ..Default::default()
        }))
    }

    fn bond(f: &str, r: u64) -> EventKind {
        EventKind::FactionKillBond(FactionKillBond {
            reward: r,
            awarding_faction: Some(f.to_string()),
            ..Default::default()
        })
    }

    fn redeem(ty: &str, amount: u64, factions: &[(&str, u64)]) -> EventKind {
        EventKind::RedeemVoucher(RedeemVoucher {
            type_: ty.to_string(),
            amount,
            factions: factions
                .iter()
                .map(|(f, a)| VoucherFaction {
                    faction: f.to_string(),
                    amount: *a,
                })
                .collect(),
            ..Default::default()
        })
    }

    fn amount_of(l: &VoucherLedger, kind: VoucherKind, f: &str) -> u64 {
        l.unredeemed()
            .iter()
            .filter(|x| x.kind == kind && x.faction == f)
            .map(|x| x.amount)
            .sum()
    }

    fn sample() -> VoucherLedger {
        let mut l = VoucherLedger::default();
        assert!(l.apply_event(&bounty(&[("A", 50000), ("B", 10000)]), "Sol", t(0)));
        assert!(l.apply_event(&bond("A", 20000), "Alpha", t(1)));
        l
    }

    #[test]
    fn bounties_and_bonds_accumulate_by_faction_and_kind() {
        let l = sample();
        assert_eq!(amount_of(&l, VoucherKind::Bounty, "A"), 50000);
        assert_eq!(amount_of(&l, VoucherKind::Bounty, "B"), 10000);
        assert_eq!(amount_of(&l, VoucherKind::CombatBond, "A"), 20000);
        assert_eq!(l.total(VoucherKind::Bounty), 60000);
        assert_eq!(l.total(VoucherKind::CombatBond), 20000);
        let u = l.unredeemed();
        assert_eq!(u.len(), 3);
        assert_eq!(u[2].systems, vec!["Alpha".to_string()]);
    }

    #[test]
    fn redeem_with_factions_subtracts_per_faction() {
        let mut l = sample();
        assert!(l.apply_event(&redeem("bounty", 50000, &[("A", 50000)]), "Sol", t(5)));
        assert_eq!(amount_of(&l, VoucherKind::Bounty, "A"), 0);
        assert_eq!(amount_of(&l, VoucherKind::Bounty, "B"), 10000);
        assert_eq!(amount_of(&l, VoucherKind::CombatBond, "A"), 20000);
    }

    #[test]
    fn redeem_combat_bond_with_single_faction() {
        let mut l = sample();
        let ev = EventKind::RedeemVoucher(RedeemVoucher {
            type_: "CombatBond".into(),
            amount: 20000,
            faction: Some("$faction_A;".into()),
            ..Default::default()
        });
        // "$faction_A;" debe casar con "A"
        assert!(l.apply_event(&ev, "Sol", t(5)));
        assert_eq!(l.total(VoucherKind::CombatBond), 0);
        assert_eq!(l.total(VoucherKind::Bounty), 60000);
    }

    #[test]
    fn redeem_without_faction_subtracts_oldest_first() {
        let mut l = VoucherLedger::default();
        l.apply_event(&bounty(&[("A", 30000)]), "S1", t(0));
        l.apply_event(&bounty(&[("B", 20000)]), "S2", t(1));
        l.apply_event(&redeem("bounty", 40000, &[]), "S1", t(2));
        assert_eq!(amount_of(&l, VoucherKind::Bounty, "A"), 0);
        assert_eq!(amount_of(&l, VoucherKind::Bounty, "B"), 10000);
        assert_eq!(l.total(VoucherKind::Bounty), 10000);
    }

    #[test]
    fn broker_redeem_clears_face_value() {
        // Ruling Spike AX §5: Amount se usa tal cual (BROKER_AMOUNT_IS_NET = false).
        let mut l = VoucherLedger::default();
        l.apply_event(&bounty(&[("A", 50000)]), "Sol", t(0));
        let ev = EventKind::RedeemVoucher(RedeemVoucher {
            type_: "bounty".into(),
            amount: 50000,
            broker_percentage: Some(25.0),
            ..Default::default()
        });
        assert!(l.apply_event(&ev, "Sol", t(1)));
        assert_eq!(l.total(VoucherKind::Bounty), 0);
        assert!(l.unredeemed().is_empty());
    }

    #[test]
    fn nominal_value_rule_both_variants() {
        assert_eq!(nominal(37500, 25.0, true), 50000);
        assert_eq!(nominal(50000, 25.0, false), 50000);
        assert_eq!(nominal(1000, 100.0, true), 1000);
    }

    #[test]
    fn other_voucher_types_are_ignored() {
        let mut l = sample();
        assert!(!l.apply_event(&redeem("scannable", 1000, &[]), "Sol", t(5)));
        assert_eq!(l.total(VoucherKind::Bounty), 60000);
        assert_eq!(l.total(VoucherKind::CombatBond), 20000);
    }

    #[test]
    fn died_zeroes_all_unredeemed_vouchers() {
        let mut l = sample();
        l.apply_event(&bounty(&[("C", 1)]), "Sol", t(2));
        assert_eq!(l.unredeemed().len(), 4);
        assert!(l.apply_event(&EventKind::Died, "Sol", t(3)));
        assert!(l.unredeemed().is_empty());
        assert_eq!(l.total(VoucherKind::Bounty), 0);
    }

    #[test]
    fn never_goes_negative() {
        let mut l = VoucherLedger::default();
        l.apply_event(&redeem("bounty", 999999, &[]), "Sol", t(0));
        assert_eq!(l.total(VoucherKind::Bounty), 0);
        l.apply_event(&bounty(&[("A", 100)]), "Sol", t(1));
        l.apply_event(&redeem("bounty", 0, &[("A", 999999)]), "Sol", t(2));
        assert_eq!(l.total(VoucherKind::Bounty), 0);
    }

    #[test]
    fn fixture_like_sequence_sums_equal_redeemed_amount() {
        let mut l = VoucherLedger::default();
        l.apply_event(&bounty(&[("A", 50000), ("B", 10000)]), "S1", t(0));
        l.apply_event(&bond("A", 20000), "S2", t(1));
        let before = l.total(VoucherKind::Bounty);
        let amount = 60000;
        l.apply_event(
            &redeem("bounty", amount, &[("A", 50000), ("B", 10000)]),
            "S1",
            t(2),
        );
        assert_eq!(before - l.total(VoucherKind::Bounty), amount);
        assert_eq!(l.total(VoucherKind::Bounty), 0);
        assert_eq!(l.total(VoucherKind::CombatBond), 20000);
    }

    #[test]
    fn thargoid_victim_marks_ax_and_fifo_is_stable() {
        let mut l = VoucherLedger::default();
        let ev = EventKind::FactionKillBond(FactionKillBond {
            reward: 5000,
            awarding_faction: Some("X".into()),
            victim_faction: Some("$faction_Thargoid;".into()),
            ..Default::default()
        });
        l.apply_event(&ev, "S", t(5));
        l.apply_event(&bond("X", 100), "S", t(1));
        let u = l.unredeemed();
        assert_eq!(u.len(), 2);
        assert_eq!((u[0].amount, u[0].ax), (100, false));
        assert_eq!((u[1].amount, u[1].ax), (5000, true));
    }

    fn bond_ev(ty: &str, faction: Option<&str>, reward: u64) -> EventKind {
        let k = FactionKillBond {
            reward,
            awarding_faction: faction.map(String::from),
            ..Default::default()
        };
        if ty == "cap" {
            EventKind::CapShipBond(k)
        } else {
            EventKind::FactionKillBond(k)
        }
    }

    #[test]
    fn faction_key_keeps_inner_characters() {
        assert_eq!(faction_key(" $Faction_Alpha-1; "), "alpha-1");
        assert_ne!(faction_key("Alpha-1"), faction_key("Alpha 1"));
        assert_eq!(faction_key("$faction_A;"), faction_key("a"));
        let mut l = VoucherLedger::default();
        l.apply_event(&bounty(&[("Alpha-1", 100), ("Alpha 1", 200)]), "S", t(0));
        l.apply_event(&redeem("bounty", 0, &[("alpha 1", 200)]), "S", t(1));
        assert_eq!(amount_of(&l, VoucherKind::Bounty, "Alpha-1"), 100);
        assert_eq!(l.total(VoucherKind::Bounty), 100);
    }

    #[test]
    fn unmatched_faction_leftover_hits_factionless_bonds_only() {
        let mut l = VoucherLedger::default();
        l.apply_event(&bond_ev("f", None, 300), "S", t(0));
        l.apply_event(&bond_ev("f", Some("Z"), 500), "S", t(1));
        l.apply_event(&bounty(&[("", 50)]), "S", t(2));
        let ev = EventKind::RedeemVoucher(RedeemVoucher {
            type_: "CombatBond".into(),
            amount: 200,
            faction: Some("Q".into()),
            ..Default::default()
        });
        l.apply_event(&ev, "S", t(3));
        assert_eq!(amount_of(&l, VoucherKind::CombatBond, ""), 100);
        assert_eq!(amount_of(&l, VoucherKind::CombatBond, "Z"), 500);
        assert_eq!(l.total(VoucherKind::Bounty), 50);
        // Factions[] con sobrante mayor que lo disponible: nunca negativo
        l.apply_event(&redeem("CombatBond", 0, &[("Q", 999)]), "S", t(4));
        assert_eq!(amount_of(&l, VoucherKind::CombatBond, ""), 0);
        assert_eq!(l.total(VoucherKind::CombatBond), 500);
    }

    #[test]
    fn partial_and_unknown_faction_redemptions() {
        let mut l = sample();
        l.apply_event(&redeem("bounty", 20000, &[("A", 20000)]), "S", t(5));
        assert_eq!(amount_of(&l, VoucherKind::Bounty, "A"), 30000);
        l.apply_event(&redeem("bounty", 5000, &[("Nope", 5000)]), "S", t(6));
        assert_eq!(amount_of(&l, VoucherKind::Bounty, "A"), 30000);
        assert_eq!(amount_of(&l, VoucherKind::Bounty, "B"), 10000);
        assert_eq!(amount_of(&l, VoucherKind::CombatBond, "A"), 20000);
    }

    #[test]
    fn per_faction_amounts_win_over_total_amount() {
        let mut l = sample();
        l.apply_event(&redeem("bounty", 999999, &[("B", 10000)]), "S", t(5));
        assert_eq!(amount_of(&l, VoucherKind::Bounty, "A"), 50000);
        assert_eq!(amount_of(&l, VoucherKind::Bounty, "B"), 0);
    }

    #[test]
    fn capship_bond_and_kind_separation() {
        let mut l = VoucherLedger::default();
        assert!(l.apply_event(&bond_ev("cap", Some("A"), 7000), "S", t(0)));
        l.apply_event(&bounty(&[("A", 3000)]), "S", t(1));
        assert_eq!(l.total(VoucherKind::CombatBond), 7000);
        l.apply_event(&redeem("CombatBond", 7000, &[("A", 7000)]), "S", t(2));
        assert_eq!(l.total(VoucherKind::CombatBond), 0);
        assert_eq!(l.total(VoucherKind::Bounty), 3000);
        l.apply_event(&bond_ev("f", Some("A"), 100), "S", t(3));
        l.apply_event(&redeem("bounty", 3000, &[]), "S", t(4));
        assert_eq!(l.total(VoucherKind::Bounty), 0);
        assert_eq!(l.total(VoucherKind::CombatBond), 100);
    }
}
