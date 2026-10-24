//! Módulo `powerplay`: seguimiento de méritos, rangos oficiales Powerplay 2.0 y métricas de sesión.

use chrono::{DateTime, Datelike, Duration, Timelike, Utc, Weekday};
use serde_json::json;
use tmc_core::{Module, ModuleCtx, Relevance, TopicSet};
use tmc_model::events::EventKind;
use tmc_model::powerplay::merits_threshold_for_rank;
use tmc_model::source::SourceEvent;
use tmc_model::state::GameState;

pub const TOPIC_STATUS: &str = "powerplay.status";
pub const TOPIC_SESSION: &str = "powerplay.session";
const TOPICS: [&str; 2] = [TOPIC_STATUS, TOPIC_SESSION];

/// Calcula la fecha/hora del próximo tick semanal de Powerplay (jueves 07:00 UTC).
pub fn next_powerplay_cycle_time(now: DateTime<Utc>) -> DateTime<Utc> {
    let weekday_num = now.weekday().number_from_monday(); // 1..=7
    let target_weekday = Weekday::Thu.number_from_monday(); // 4

    let mut days_ahead = (target_weekday as i64 - weekday_num as i64).rem_euclid(7);
    if days_ahead == 0 {
        let time = now.time();
        if time.hour() >= 7 {
            days_ahead = 7;
        }
    }
    let target_date = now.date_naive() + Duration::days(days_ahead);
    target_date.and_hms_opt(7, 0, 0).unwrap().and_utc()
}

#[derive(Debug, Clone, Default)]
pub struct PowerplayModule {
    pub power: Option<String>,
    pub rank: u32,
    pub merits: u32,
    pub votes: Option<u32>,
    pub time_pledged: Option<u64>,

    // Métricas de sesión
    pub session_start: Option<DateTime<Utc>>,
    pub session_merits_earned: u32,
    pub last_merits_time: Option<DateTime<Utc>>,
    pub last_now: Option<DateTime<Utc>>,
}

impl PowerplayModule {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Module for PowerplayModule {
    fn id(&self) -> &'static str {
        "powerplay"
    }

    fn topics(&self) -> Vec<String> {
        TOPICS.iter().map(|s| s.to_string()).collect()
    }

    fn relevance(&self, _state: &GameState) -> Relevance {
        if self.power.is_none() {
            Relevance::Hidden
        } else if let (Some(now), Some(last)) = (self.last_now, self.last_merits_time) {
            if now.signed_duration_since(last).num_minutes() < 5 {
                Relevance::Active
            } else {
                Relevance::Normal
            }
        } else {
            Relevance::Normal
        }
    }

    fn on_event(&mut self, ev: &SourceEvent, _state: &GameState, _ctx: &mut ModuleCtx) -> TopicSet {
        let mut changed = TopicSet::new();

        match ev {
            SourceEvent::Status(status) => {
                self.last_now = Some(status.timestamp);
            }
            SourceEvent::Journal { ev, .. } => {
                let timestamp = ev.timestamp;
                self.last_now = Some(timestamp);
                if self.session_start.is_none() {
                    self.session_start = Some(timestamp);
                }

                match &ev.kind {
                    EventKind::Powerplay(pp) => {
                        self.power = Some(pp.power.clone());
                        self.rank = pp.rank;
                        self.merits = pp.merits;
                        self.votes = pp.votes;
                        self.time_pledged = pp.time_pledged;

                        changed.insert(TOPIC_STATUS.to_string());
                    }
                    EventKind::PowerplayMerits(m) => {
                        self.power = Some(m.power.clone());
                        self.merits = m.total_merits;
                        self.session_merits_earned =
                            self.session_merits_earned.saturating_add(m.merits_earned);
                        self.last_merits_time = Some(timestamp);

                        changed.insert(TOPIC_STATUS.to_string());
                        changed.insert(TOPIC_SESSION.to_string());
                    }
                    EventKind::PowerplayLeave(_) => {
                        self.power = None;
                        self.rank = 0;
                        self.merits = 0;
                        self.votes = None;
                        self.time_pledged = None;

                        changed.insert(TOPIC_STATUS.to_string());
                    }
                    EventKind::PowerplayDefect(def) => {
                        self.power = Some(def.to_power.clone());
                        self.merits /= 2;

                        changed.insert(TOPIC_STATUS.to_string());
                    }
                    EventKind::PowerplayJoin(join) => {
                        self.power = Some(join.power.clone());
                        self.rank = 1;
                        self.merits = 0;
                        self.votes = None;
                        self.time_pledged = Some(0);

                        changed.insert(TOPIC_STATUS.to_string());
                    }
                    EventKind::LoadGame(_) => {
                        self.session_merits_earned = 0;
                        self.session_start = Some(timestamp);
                        self.last_merits_time = None;

                        changed.insert(TOPIC_SESSION.to_string());
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        changed
    }

    fn topic_value(&self, topic: &str) -> Option<serde_json::Value> {
        let now = self.last_now.unwrap_or_else(Utc::now);

        match topic {
            TOPIC_STATUS => {
                let is_pledged = self.power.is_some();
                let weeks_pledged = self
                    .time_pledged
                    .map(|sec| (sec / 604_800) as u32)
                    .unwrap_or(0);

                let base_merits = merits_threshold_for_rank(self.rank);
                let next_rank_target = merits_threshold_for_rank(self.rank.saturating_add(1));
                let merits_needed = next_rank_target.saturating_sub(self.merits);

                let progress_percent = if next_rank_target <= base_merits {
                    100.0
                } else {
                    let current_in_tier = self.merits.saturating_sub(base_merits);
                    let tier_span = next_rank_target - base_merits;
                    (current_in_tier as f64 / tier_span as f64 * 100.0).clamp(0.0, 100.0)
                };

                let next_cycle = next_powerplay_cycle_time(now);
                let seconds_to_cycle = next_cycle.signed_duration_since(now).num_seconds().max(0);

                Some(json!({
                    "is_pledged": is_pledged,
                    "power": self.power,
                    "rank": self.rank,
                    "merits": self.merits,
                    "votes": self.votes,
                    "weeks_pledged": weeks_pledged,
                    "time_pledged_seconds": self.time_pledged,
                    "current_rank_base": base_merits,
                    "next_rank_target": next_rank_target,
                    "merits_needed": merits_needed,
                    "progress_percent": progress_percent,
                    "next_cycle_time": next_cycle,
                    "seconds_to_cycle": seconds_to_cycle,
                }))
            }
            TOPIC_SESSION => {
                let elapsed_seconds = self
                    .session_start
                    .map(|start| now.signed_duration_since(start).num_seconds().max(0))
                    .unwrap_or(0);

                let merits_per_hour = if elapsed_seconds >= 60 {
                    (self.session_merits_earned as f64 / elapsed_seconds as f64) * 3600.0
                } else {
                    0.0
                };

                Some(json!({
                    "session_merits_earned": self.session_merits_earned,
                    "merits_per_hour": merits_per_hour,
                    "session_duration_seconds": elapsed_seconds,
                }))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
