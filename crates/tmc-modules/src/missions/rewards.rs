//! Recompensas de la sesión y objetivos comunitarios.

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::BTreeMap;
use tmc_model::events::{CommunityGoal, EventKind};

/// Tendencia acumulada de una facción: último token y puntuación con signo
/// (`+` suma 1, `-` resta 1; `Up*` = +1, `Down*` = -1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FactionDelta {
    pub reputation: String,
    pub influence: String,
    pub reputation_score: i32,
    pub influence_score: i32,
}

fn token_score(t: &str) -> i32 {
    let t = t.trim();
    if t.starts_with("Up") {
        1
    } else if t.starts_with("Down") {
        -1
    } else {
        t.chars()
            .map(|c| match c {
                '+' => 1,
                '-' => -1,
                _ => 0,
            })
            .sum()
    }
}

/// `credits` suma solo `Reward`; la `Donation` queda fuera (es un gasto, no un cobro).
#[derive(Debug, Clone, Default, Serialize)]
pub struct SessionRewards {
    pub credits: u64,
    pub completed: u32,
    pub failed: u32,
    /// Abandonadas: se cuentan aparte, no como fallidas.
    pub abandoned: u32,
    pub materials: BTreeMap<String, u32>,
    pub factions: BTreeMap<String, FactionDelta>,
    pub since: Option<DateTime<Utc>>,
}

impl SessionRewards {
    pub fn reset(&mut self, at: DateTime<Utc>) {
        *self = Self {
            since: Some(at),
            ..Default::default()
        };
    }

    pub fn apply_event(&mut self, ev: &EventKind, at: DateTime<Utc>) {
        match ev {
            EventKind::LoadGame(_) => self.reset(at),
            EventKind::MissionCompleted(m) => {
                self.completed += 1;
                self.credits += m.reward.unwrap_or(0);
                for mat in &m.materials_reward {
                    let key = mat.name.trim().to_lowercase();
                    if !key.is_empty() {
                        *self.materials.entry(key).or_default() += mat.count;
                    }
                }
                for f in &m.faction_effects {
                    if f.faction.is_empty() {
                        continue;
                    }
                    let d = self.factions.entry(f.faction.clone()).or_default();
                    if let Some(r) = f.reputation.as_ref().filter(|r| !r.is_empty()) {
                        d.reputation = r.clone();
                        d.reputation_score += token_score(r);
                    }
                    if let Some(i) = f.influence.iter().rev().find(|i| !i.influence.is_empty()) {
                        d.influence = i.influence.clone();
                    }
                    for i in &f.influence {
                        d.influence_score += token_score(&i.influence);
                    }
                }
            }
            EventKind::MissionFailed(_) => self.failed += 1,
            EventKind::MissionAbandoned(_) => self.abandoned += 1,
            _ => {}
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CgView {
    pub cgid: u64,
    pub title: String,
    pub system: Option<String>,
    pub station: Option<String>,
    pub contribution: u64,
    pub percentile: Option<u32>,
    pub tier: Option<String>,
    pub expiry: Option<DateTime<Utc>>,
    pub bonus: Option<u64>,
    pub complete: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CommunityGoals {
    pub goals: Vec<CgView>,
}

impl CommunityGoals {
    /// Cada evento sustituye la lista entera.
    pub fn apply(&mut self, ev: &CommunityGoal) {
        self.goals = ev
            .current_goals
            .iter()
            .map(|g| CgView {
                cgid: g.cgid,
                title: g.title.clone(),
                system: g.system_name.clone(),
                station: g.market_name.clone(),
                contribution: g.player_contribution.unwrap_or(0),
                percentile: g.player_percentile_band,
                tier: g.tier_reached.clone(),
                expiry: g.expiry,
                bonus: g.bonus,
                complete: g.is_complete,
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tmc_model::events::{
        CgEntry, FactionEffect, InfluenceEffect, LoadGame, MaterialReward, MissionCompleted,
        MissionEnded,
    };

    fn at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 20, 12, 0, 0).unwrap()
    }
    fn done(reward: u64) -> EventKind {
        EventKind::MissionCompleted(Box::new(MissionCompleted {
            reward: Some(reward),
            ..Default::default()
        }))
    }

    #[test]
    fn session_rewards_sum_and_reset() {
        let mut r = SessionRewards::default();
        r.apply_event(&done(1_000_000), at());
        r.apply_event(&done(500_000), at());
        r.apply_event(&EventKind::MissionFailed(MissionEnded::default()), at());
        assert_eq!((r.credits, r.completed, r.failed), (1_500_000, 2, 1));
        r.apply_event(&EventKind::LoadGame(LoadGame::default()), at());
        assert_eq!((r.credits, r.completed, r.failed), (0, 0, 0));
    }

    #[test]
    fn rewards_track_materials_and_faction_trends() {
        let mut r = SessionRewards::default();
        for n in [2, 1] {
            r.apply_event(
                &EventKind::MissionCompleted(Box::new(MissionCompleted {
                    materials_reward: vec![MaterialReward {
                        name: "Tellurium".into(),
                        count: n,
                        ..Default::default()
                    }],
                    faction_effects: vec![FactionEffect {
                        faction: "A".into(),
                        reputation: Some("++".into()),
                        influence: vec![InfluenceEffect {
                            influence: "+++".into(),
                            ..Default::default()
                        }],
                    }],
                    ..Default::default()
                })),
                at(),
            );
        }
        assert_eq!(r.materials["tellurium"], 3);
        assert_eq!(r.factions["A"].influence, "+++");
        assert_eq!(r.factions["A"].reputation, "++");
        assert_eq!(r.factions["A"].influence_score, 6);
        assert_eq!(r.factions["A"].reputation_score, 4);
    }

    #[test]
    fn distinct_trends_accumulate_and_abandoned_is_separate() {
        let mut r = SessionRewards::default();
        for tok in ["+", "++", "DownBad"] {
            r.apply_event(
                &EventKind::MissionCompleted(Box::new(MissionCompleted {
                    donation: Some(5),
                    faction_effects: vec![FactionEffect {
                        faction: "B".into(),
                        reputation: None,
                        influence: vec![InfluenceEffect {
                            influence: tok.into(),
                            ..Default::default()
                        }],
                    }],
                    ..Default::default()
                })),
                at(),
            );
        }
        assert_eq!(r.factions["B"].influence_score, 2);
        assert_eq!(r.factions["B"].influence, "DownBad");
        assert_eq!(r.credits, 0);
        r.apply_event(&EventKind::MissionAbandoned(MissionEnded::default()), at());
        assert_eq!((r.abandoned, r.failed), (1, 0));
    }

    #[test]
    fn community_goal_view() {
        let mut c = CommunityGoals::default();
        c.apply(&CommunityGoal {
            current_goals: vec![CgEntry {
                cgid: 750,
                player_percentile_band: Some(25),
                bonus: Some(15_000_000),
                ..Default::default()
            }],
        });
        assert_eq!(c.goals[0].cgid, 750);
        assert_eq!(c.goals[0].percentile, Some(25));
        assert_eq!(c.goals[0].bonus, Some(15_000_000));
        c.apply(&CommunityGoal {
            current_goals: vec![CgEntry {
                cgid: 1,
                tier_reached: Some("Tier 2".into()),
                expiry: Some(at()),
                player_contribution: Some(500),
                ..Default::default()
            }],
        });
        assert_eq!(c.goals.len(), 1);
        assert_eq!(c.goals[0].tier.as_deref(), Some("Tier 2"));
        assert_eq!(c.goals[0].expiry, Some(at()));
        assert_eq!(c.goals[0].contribution, 500);
        c.apply(&CommunityGoal {
            current_goals: vec![CgEntry {
                cgid: 750,
                player_percentile_band: Some(25),
                bonus: Some(15_000_000),
                ..Default::default()
            }],
        });
        c.apply(&CommunityGoal::default());
        assert!(c.goals.is_empty());
    }
}
