//! Sesión de caza (bajas, créditos, rango) y reconstrucción del libro de vales.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;
use tmc_core::{Store, StoreError, StoreTask};
use tmc_model::events::{EventKind, parse_line};

use super::vouchers::VoucherLedger;
use crate::history;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HuntingSession {
    pub started_at: DateTime<Utc>,
    pub kills: u32,
    pub credits: u64,
    pub kills_by_type: BTreeMap<String, u32>,
    pub combat_rank: Option<u8>,
    pub combat_progress: Option<u8>,
}

impl HuntingSession {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            started_at: now,
            kills: 0,
            credits: 0,
            kills_by_type: BTreeMap::new(),
            combat_rank: None,
            combat_progress: None,
        }
    }

    fn kill(&mut self, kind: &str, credits: u64) {
        self.kills += 1;
        *self.kills_by_type.entry(kind.to_owned()).or_default() += 1;
        self.credits = self.credits.saturating_add(credits);
    }

    /// Aplica un evento; devuelve `true` si cambió la sesión. `LoadGame` la reinicia.
    pub fn apply_event(&mut self, ev: &EventKind, now: DateTime<Utc>) -> bool {
        match ev {
            EventKind::LoadGame(_) => {
                self.reset(now);
                true
            }
            // `SharedWithOthers` cuenta a otros jugadores, no una baja ajena: toda
            // `Bounty` es una baja completa (spike AX F2, sin asistencias).
            EventKind::Bounty(b) => {
                let credits = b.normalized_rewards().iter().map(|r| r.reward).sum();
                let kind = b.target.as_deref().unwrap_or("unknown");
                self.kill(kind, credits);
                true
            }
            EventKind::FactionKillBond(k) => {
                self.kill("faction_kill_bond", k.reward);
                true
            }
            EventKind::CapShipBond(k) => {
                self.kill("cap_ship_bond", k.reward);
                true
            }
            EventKind::PvpKill(_) => {
                self.kill("pvp", 0);
                true
            }
            EventKind::Rank(r) | EventKind::Progress(r) => {
                let Some(v) = r.combat else { return false };
                let v = u8::try_from(v).unwrap_or(u8::MAX);
                let slot = if matches!(ev, EventKind::Rank(_)) {
                    &mut self.combat_rank
                } else {
                    &mut self.combat_progress
                };
                slot.replace(v) != Some(v)
            }
            EventKind::Promotion(p) => {
                let Some(v) = p.combat else { return false };
                self.combat_rank = Some(v);
                self.combat_progress = Some(0);
                true
            }
            _ => false,
        }
    }

    /// `None` con menos de 60 s de sesión.
    pub fn credits_per_hour(&self, now: DateTime<Utc>) -> Option<u64> {
        let secs = (now - self.started_at).num_seconds();
        if secs < 60 {
            return None;
        }
        Some((u128::from(self.credits) * 3600 / secs as u128) as u64)
    }

    /// Reinicia contadores; conserva el rango de combate.
    pub fn reset(&mut self, now: DateTime<Utc>) {
        *self = Self {
            combat_rank: self.combat_rank,
            combat_progress: self.combat_progress,
            ..Self::new(now)
        };
    }
}

/// Reconstruye el libro de vales del comandante desde el último `Died`
/// (resultado `Stored` con tag `combat.vouchers`: `{cmdr, ledger, cut, cut_ts}`).
pub fn rebuild_task(cmdr: String) -> StoreTask {
    Box::new(move |store| {
        let rows = store.query_json("SELECT COALESCE(MAX(id), 0) FROM event", &[])?;
        let cut = rows[0][0].as_i64().unwrap_or_default();
        // Instante del último evento cubierto: los posteriores aún no están en el almacén.
        let cut_ts = store
            .query_json("SELECT ts FROM event WHERE id = ?1", &[json!(cut)])?
            .first()
            .map(|r| r[0].clone())
            .unwrap_or(serde_json::Value::Null);
        let reset = history::last_before(store, &cmdr, cut, &["Died"], None)?;
        let mut system = match &reset {
            Some(p) => system_before(store, &cmdr, cut, p)?,
            None => String::new(),
        };
        let mut ledger = VoucherLedger::default();
        history::for_each_after(store, &cmdr, cut, REBUILD_NAMES, reset, |pos| {
            if let Ok(ev) = parse_line(&pos.raw) {
                if let Some(s) = system_of_kind(&ev.kind) {
                    system = s;
                }
                ledger.apply_event(&ev.kind, &system, ev.timestamp);
            }
        })?;
        let ledger =
            serde_json::to_value(&ledger).map_err(|e| StoreError::Unsupported(e.to_string()))?;
        Ok(json!({"cmdr": cmdr, "ledger": ledger, "cut": cut, "cut_ts": cut_ts}))
    })
}

const LOCATIONS: &[&str] = &["Location", "FSDJump", "CarrierJump"];
const REBUILD_NAMES: &[&str] = &[
    "Location",
    "FSDJump",
    "CarrierJump",
    "Bounty",
    "FactionKillBond",
    "CapShipBond",
    "RedeemVoucher",
    "Died",
];

/// Sistema del último `Location`/`FSDJump`/`CarrierJump` legible anterior a `before`.
fn system_before(
    store: &Store,
    cmdr: &str,
    cut: i64,
    before: &history::Position,
) -> Result<String, StoreError> {
    let mut system = None;
    history::for_each_before(store, cmdr, cut, LOCATIONS, Some(before), |p| {
        system = parse_line(&p.raw)
            .ok()
            .and_then(|ev| system_of_kind(&ev.kind));
        system.is_some()
    })?;
    Ok(system.unwrap_or_default())
}

fn system_of_kind(kind: &EventKind) -> Option<String> {
    match kind {
        EventKind::Location(l) | EventKind::FsdJump(l) | EventKind::CarrierJump(l) => {
            Some(l.star_system.clone())
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};
    use tmc_core::{EventWriter, StoredEvent};
    use tmc_model::events::{Bounty, BountyReward, Promotion, Rank};

    fn t0() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 18, 10, 0, 0).unwrap()
    }

    fn bounty(reward: u64, shared: Option<u32>) -> EventKind {
        EventKind::Bounty(Box::new(Bounty {
            rewards: vec![BountyReward {
                faction: "F".into(),
                reward,
            }],
            target: Some("empire_eagle".into()),
            shared_with_others: shared,
            ..Default::default()
        }))
    }

    fn three() -> HuntingSession {
        let mut s = HuntingSession::new(t0());
        assert!(s.apply_event(&bounty(100_000, None), t0()));
        assert!(s.apply_event(&bounty(200_000, Some(1)), t0()));
        assert!(s.apply_event(&bounty(300_000, Some(0)), t0()));
        s
    }

    #[test]
    fn session_counts_every_bounty_as_a_kill() {
        let s = three();
        assert_eq!((s.kills, s.credits), (3, 600_000));
        assert_eq!(s.kills_by_type.get("empire_eagle"), Some(&3));
    }

    #[test]
    fn credits_per_hour_after_30_min() {
        let s = three();
        assert_eq!(
            s.credits_per_hour(t0() + Duration::minutes(30)),
            Some(1_200_000)
        );
    }

    #[test]
    fn credits_per_hour_none_under_one_minute() {
        let s = three();
        assert_eq!(s.credits_per_hour(t0() + Duration::seconds(30)), None);
    }

    #[test]
    fn promotion_updates_combat_rank() {
        let mut s = HuntingSession::new(t0());
        let progress = EventKind::Progress(Rank {
            combat: Some(40),
            ..Default::default()
        });
        assert!(s.apply_event(&progress, t0()));
        assert_eq!((s.combat_rank, s.combat_progress), (None, Some(40)));
        let promo = EventKind::Promotion(Promotion {
            combat: Some(5),
            ..Default::default()
        });
        assert!(s.apply_event(&promo, t0()));
        assert_eq!((s.combat_rank, s.combat_progress), (Some(5), Some(0)));
        assert!(!s.apply_event(&EventKind::HeatWarning, t0()));
    }

    #[test]
    fn reset_session_keeps_ledger() {
        let mut s = three();
        let mut ledger = VoucherLedger::default();
        ledger.apply_event(&bounty(100_000, None), "Sol", t0());
        let later = t0() + Duration::minutes(5);
        s.reset(later);
        assert_eq!((s.kills, s.credits), (0, 0));
        assert!(s.kills_by_type.is_empty());
        assert_eq!(s.started_at, later);
        assert_eq!(
            ledger.total(super::super::vouchers::VoucherKind::Bounty),
            100_000
        );
    }

    #[test]
    fn ledger_rebuilt_after_restart_matches_live_ledger() {
        let fixture = include_str!("../../../../tests/fixtures/journal/combat_session.log");
        let store = Store::open_in_memory().unwrap();
        let writer = EventWriter::spawn(&store);
        let mut live = VoucherLedger::default();
        let mut system = String::new();
        for (i, raw) in fixture.lines().enumerate() {
            let ev = parse_line(raw).unwrap();
            match &ev.kind {
                EventKind::Location(l) | EventKind::FsdJump(l) | EventKind::CarrierJump(l) => {
                    system = l.star_system.clone()
                }
                _ => {}
            }
            live.apply_event(&ev.kind, &system, ev.timestamp);
            writer.send(StoredEvent {
                file_name: "combat.log".into(),
                line: i as u64 + 1,
                ts: ev.timestamp,
                name: ev.name,
                cmdr: Some("TESTER".into()),
                json: raw.into(),
            });
        }
        writer.flush_blocking().unwrap();
        let expected = live.unredeemed();
        assert_eq!(
            live.total(super::super::vouchers::VoucherKind::Bounty),
            500_000
        );
        assert_eq!(expected.len(), 3);
        let result = rebuild_task("TESTER".into())(&store).unwrap();
        assert_eq!(result["cmdr"], "TESTER");
        let rebuilt: VoucherLedger = serde_json::from_value(result["ledger"].clone()).unwrap();
        assert_eq!(rebuilt.unredeemed(), expected);
        let other = rebuild_task("OTHER".into())(&store).unwrap();
        let empty: VoucherLedger = serde_json::from_value(other["ledger"].clone()).unwrap();
        assert!(empty.unredeemed().is_empty());
    }

    #[test]
    fn bonds_and_pvp_count_as_kills_and_loadgame_resets() {
        use tmc_model::events::{FactionKillBond, LoadGame, PvpKill};
        let mut s = HuntingSession::new(t0());
        let bond = FactionKillBond {
            reward: 7_000,
            ..Default::default()
        };
        assert!(s.apply_event(&EventKind::FactionKillBond(bond.clone()), t0()));
        assert!(s.apply_event(&EventKind::CapShipBond(bond), t0()));
        assert!(s.apply_event(&EventKind::PvpKill(PvpKill::default()), t0()));
        assert_eq!((s.kills, s.credits), (3, 14_000));
        assert_eq!(s.kills_by_type.get("cap_ship_bond"), Some(&1));
        assert_eq!(s.kills_by_type.get("pvp"), Some(&1));
        let rank = EventKind::Rank(Rank {
            combat: Some(7),
            ..Default::default()
        });
        assert!(s.apply_event(&rank, t0()));
        assert_eq!(s.combat_rank, Some(7));
        let later = t0() + Duration::minutes(2);
        assert!(s.apply_event(&EventKind::LoadGame(LoadGame::default()), later));
        assert_eq!((s.kills, s.credits, s.started_at), (0, 0, later));
        assert_eq!(s.combat_rank, Some(7));
    }

    fn ev_line(ts: &str, name: &str, extra: &str) -> String {
        format!(r#"{{"timestamp":"{ts}","event":"{name}"{extra}}}"#)
    }

    fn put(store: &Store, file: &str, rows: Vec<(u64, String)>) {
        let writer = EventWriter::spawn(store);
        for (line, raw) in rows {
            let ev = parse_line(&raw).unwrap();
            writer.send(StoredEvent {
                file_name: file.into(),
                line,
                ts: ev.timestamp,
                name: ev.name,
                cmdr: Some("TESTER".into()),
                json: raw,
            });
        }
        writer.flush_blocking().unwrap();
    }

    fn rebuilt(store: &Store) -> VoucherLedger {
        let r = rebuild_task("TESTER".into())(store).unwrap();
        serde_json::from_value(r["ledger"].clone()).unwrap()
    }

    const BOUNTY: &str = r#","Rewards":[{"Faction":"F","Reward":1000}]"#;

    #[test]
    fn system_is_inherited_from_before_died_and_skips_unreadable_events() {
        let store = Store::open_in_memory().unwrap();
        let ts = "2026-10-18T10:00:00Z";
        put(
            &store,
            "a.log",
            vec![
                (1, ev_line(ts, "Location", r#","StarSystem":"Old Sys""#)),
                (2, ev_line(ts, "FSDJump", r#","StarSystem":42"#)),
                (3, ev_line(ts, "Died", "")),
                (4, ev_line(ts, "Bounty", BOUNTY)),
            ],
        );
        let l = rebuilt(&store);
        let u = l.unredeemed();
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].systems, vec!["Old Sys".to_string()]);
        assert_eq!(u[0].amount, 1000);
    }

    #[test]
    fn rebuild_paginates_over_256_events_and_orders_same_ts_by_file_and_line() {
        let store = Store::open_in_memory().unwrap();
        let ts = "2026-10-18T10:00:00Z";
        // Insertadas al revés: el orden correcto lo dan archivo y línea.
        put(
            &store,
            "b.log",
            vec![(1, ev_line(ts, "Location", r#","StarSystem":"Second""#))],
        );
        let mut rows: Vec<(u64, String)> = (2..=301)
            .map(|i| (i, ev_line(ts, "Bounty", BOUNTY)))
            .collect();
        rows.reverse();
        rows.push((1, ev_line(ts, "Location", r#","StarSystem":"First""#)));
        put(&store, "a.log", rows);
        let l = rebuilt(&store);
        // a.log (First + 300 bounties) va antes que b.log: los vales quedan en "First".
        let u = l.unredeemed();
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].amount, 300_000);
        assert_eq!(u[0].systems, vec!["First".to_string()]);
    }
}
