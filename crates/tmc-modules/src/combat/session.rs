//! Sesión de caza (bajas, créditos, rango) y reconstrucción del libro de vales.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use tmc_core::{Store, StoreError, StoreTask};
use tmc_model::events::{EventKind, parse_line};

use super::vouchers::VoucherLedger;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HuntingSession {
    pub started_at: DateTime<Utc>,
    pub kills: u32,
    pub assists: u32,
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
            assists: 0,
            credits: 0,
            kills_by_type: BTreeMap::new(),
            combat_rank: None,
            combat_progress: None,
        }
    }

    fn kill(&mut self, kind: &str, shared: bool, credits: u64) {
        if shared {
            self.assists += 1;
        } else {
            self.kills += 1;
            *self.kills_by_type.entry(kind.to_owned()).or_default() += 1;
        }
        self.credits = self.credits.saturating_add(credits);
    }

    /// Aplica un evento; devuelve `true` si cambió la sesión. `LoadGame` la reinicia.
    pub fn apply_event(&mut self, ev: &EventKind, now: DateTime<Utc>) -> bool {
        match ev {
            EventKind::LoadGame(_) => {
                self.reset(now);
                true
            }
            EventKind::Bounty(b) => {
                let credits = b.normalized_rewards().iter().map(|r| r.reward).sum();
                let kind = b.target.as_deref().unwrap_or("unknown");
                self.kill(kind, b.shared_with_others.unwrap_or(0) > 0, credits);
                true
            }
            EventKind::FactionKillBond(k) => {
                self.kill("faction_kill_bond", false, k.reward);
                true
            }
            EventKind::CapShipBond(k) => {
                self.kill("cap_ship_bond", false, k.reward);
                true
            }
            EventKind::PvpKill(_) => {
                self.kill("pvp", false, 0);
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
/// (resultado `Stored` con tag `combat.vouchers`: `{cmdr, ledger, cut}`).
pub fn rebuild_task(cmdr: String) -> StoreTask {
    Box::new(move |store| {
        let rows = store.query_json("SELECT COALESCE(MAX(id), 0) FROM event", &[])?;
        let cut = rows[0][0].as_i64().unwrap_or_default();
        let reset = last_event(store, &cmdr, cut, &["Died"], None)?;
        let mut system = match &reset {
            Some(p) => last_event(store, &cmdr, cut, LOCATIONS, Some(p))?
                .and_then(|p| system_of(&p.raw))
                .unwrap_or_default(),
            None => String::new(),
        };
        let mut ledger = VoucherLedger::default();
        let mut cursor = reset;
        loop {
            let rows = store.query_json(
                "SELECT e.id, e.ts, j.name, e.line, e.json FROM event e
                 JOIN journal_file j ON j.id = e.file_id
                 WHERE e.cmdr = ?1 AND e.id <= ?2
                   AND (?3 IS NULL OR (e.ts, j.name, e.line, e.id) > (?3, ?4, ?5, ?6))
                   AND e.name IN ('Location', 'FSDJump', 'CarrierJump', 'Bounty',
                                  'FactionKillBond', 'CapShipBond', 'RedeemVoucher', 'Died')
                 ORDER BY e.ts ASC, j.name ASC, e.line ASC, e.id ASC LIMIT 256",
                &params(&cmdr, cut, cursor.as_ref()),
            )?;
            if rows.is_empty() {
                break;
            }
            for row in &rows {
                let pos = Position::from_row(row);
                if let Ok(ev) = parse_line(&pos.raw) {
                    if let Some(s) = system_of_kind(&ev.kind) {
                        system = s;
                    }
                    ledger.apply_event(&ev.kind, &system, ev.timestamp);
                }
                cursor = Some(pos);
            }
        }
        let ledger =
            serde_json::to_value(&ledger).map_err(|e| StoreError::Unsupported(e.to_string()))?;
        Ok(json!({"cmdr": cmdr, "ledger": ledger, "cut": cut}))
    })
}

const LOCATIONS: &[&str] = &["Location", "FSDJump", "CarrierJump"];

// Los `ts` se normalizan a UTC con nanosegundos fijos al guardarse; el orden
// `(ts, archivo, línea, id)` es el mismo que usa la reconstrucción de exploración.
struct Position {
    id: i64,
    ts: String,
    file: String,
    line: i64,
    raw: String,
}

impl Position {
    fn from_row(row: &Value) -> Self {
        Self {
            id: row[0].as_i64().unwrap_or_default(),
            ts: row[1].as_str().unwrap_or_default().to_owned(),
            file: row[2].as_str().unwrap_or_default().to_owned(),
            line: row[3].as_i64().unwrap_or_default(),
            raw: row[4].as_str().unwrap_or_default().to_owned(),
        }
    }
}

fn params(cmdr: &str, cut: i64, bound: Option<&Position>) -> [Value; 6] {
    match bound {
        Some(b) => [
            json!(cmdr),
            json!(cut),
            json!(b.ts),
            json!(b.file),
            json!(b.line),
            json!(b.id),
        ],
        None => [
            json!(cmdr),
            json!(cut),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    }
}

/// Último evento de los `names` (estrictamente anterior a `before`, si se da).
fn last_event(
    store: &Store,
    cmdr: &str,
    cut: i64,
    names: &[&str],
    before: Option<&Position>,
) -> Result<Option<Position>, StoreError> {
    let list = names
        .iter()
        .map(|n| format!("'{n}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT e.id, e.ts, j.name, e.line, e.json FROM event e
         JOIN journal_file j ON j.id = e.file_id
         WHERE e.cmdr = ?1 AND e.id <= ?2
           AND (?3 IS NULL OR (e.ts, j.name, e.line, e.id) < (?3, ?4, ?5, ?6))
           AND e.name IN ({list})
         ORDER BY e.ts DESC, j.name DESC, e.line DESC, e.id DESC LIMIT 1"
    );
    let rows = store.query_json(&sql, &params(cmdr, cut, before))?;
    Ok(rows.first().map(Position::from_row))
}

fn system_of_kind(kind: &EventKind) -> Option<String> {
    match kind {
        EventKind::Location(l) | EventKind::FsdJump(l) | EventKind::CarrierJump(l) => {
            Some(l.star_system.clone())
        }
        _ => None,
    }
}

fn system_of(raw: &str) -> Option<String> {
    system_of_kind(&parse_line(raw).ok()?.kind)
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
    fn session_counts_kills_assists_and_credits() {
        let s = three();
        assert_eq!((s.kills, s.assists, s.credits), (2, 1, 600_000));
        assert_eq!(s.kills_by_type.get("empire_eagle"), Some(&2));
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
        assert_eq!((s.kills, s.assists, s.credits), (0, 0, 0));
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
}
