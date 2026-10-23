//! Módulo `carrier`: telemetría, combustible, cuenta atrás de saltos y salud financiera del Fleet Carrier.

use chrono::{DateTime, Datelike, Duration, Timelike, Utc, Weekday};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tmc_core::{Module, ModuleCtx, Relevance, TopicSet};
use tmc_model::carrier::MaintenanceHealth;
use tmc_model::events::EventKind;
use tmc_model::source::SourceEvent;
use tmc_model::state::GameState;

pub const TOPIC_OVERVIEW: &str = "carrier.overview";
pub const TOPIC_JUMP: &str = "carrier.jump";
pub const TOPIC_FINANCES: &str = "carrier.finances";
const TOPICS: [&str; 3] = [TOPIC_OVERVIEW, TOPIC_JUMP, TOPIC_FINANCES];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CarrierJumpStatus {
    #[default]
    Idle,
    CountingDown,
    Locked,
    Cooldown,
}

/// Calcula la fecha/hora del próximo cobro semanal de mantenimiento del Fleet Carrier (jueves 07:00 UTC).
pub fn next_weekly_upkeep_time(now: DateTime<Utc>) -> DateTime<Utc> {
    let weekday_num = now.weekday().number_from_monday(); // 1..=7 (Mon..Sun)
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
pub struct Carrier {
    // Stats & Overview
    pub carrier_id: Option<u64>,
    pub callsign: Option<String>,
    pub name: Option<String>,
    pub current_system: Option<String>,
    pub current_body: Option<String>,
    pub docking_access: Option<String>,
    pub fuel_depot: u32,
    pub fuel_cargo: u32,
    pub space_usage_cargo: u32,
    pub space_usage_free: u32,
    pub space_usage_total: u32,
    pub is_aboard: bool,

    // Jump state
    pub jump_status: CarrierJumpStatus,
    pub jump_destination_system: Option<String>,
    pub jump_destination_body: Option<String>,
    pub jump_departure_time: Option<DateTime<Utc>>,
    pub jump_cooldown_until: Option<DateTime<Utc>>,
    pub jump_cancelled: bool,

    // Finance state
    pub finance_balance: i64,
    pub finance_reserve: i64,
    pub finance_available: i64,
    pub finance_weekly_upkeep: i64,
    pub finance_health: MaintenanceHealth,
    pub finance_weeks_covered: f64,

    // Time tracking
    pub last_now: Option<DateTime<Utc>>,
}

impl Carrier {
    pub fn new() -> Self {
        Self {
            finance_weekly_upkeep: 5_000_000,
            ..Default::default()
        }
    }

    pub fn update_jump_clock(&mut self, now: DateTime<Utc>) -> bool {
        self.last_now = Some(now);
        match self.jump_status {
            CarrierJumpStatus::CountingDown | CarrierJumpStatus::Locked => {
                if let Some(departure) = self.jump_departure_time {
                    let diff = departure.signed_duration_since(now).num_seconds();
                    if diff <= 0 {
                        // Salto completado
                        if let Some(dest) = self.jump_destination_system.take() {
                            self.current_system = Some(dest);
                        }
                        if let Some(body) = self.jump_destination_body.take() {
                            self.current_body = Some(body);
                        }
                        self.jump_status = CarrierJumpStatus::Cooldown;
                        self.jump_cooldown_until = Some(now + Duration::seconds(300));
                        return true;
                    } else if diff <= 120 && self.jump_status != CarrierJumpStatus::Locked {
                        self.jump_status = CarrierJumpStatus::Locked;
                        return true;
                    }
                }
                true
            }
            CarrierJumpStatus::Cooldown => {
                if let Some(until) = self.jump_cooldown_until
                    && now >= until
                {
                    self.jump_status = CarrierJumpStatus::Idle;
                    self.jump_cooldown_until = None;
                    return true;
                }
                true
            }
            CarrierJumpStatus::Idle => false,
        }
    }
}

impl Module for Carrier {
    fn id(&self) -> &'static str {
        "carrier"
    }

    fn topics(&self) -> Vec<String> {
        TOPICS.iter().map(|s| s.to_string()).collect()
    }

    fn relevance(&self, _state: &GameState) -> Relevance {
        if self.carrier_id.is_none() && self.callsign.is_none() {
            Relevance::Hidden
        } else if self.is_aboard
            || self.jump_status == CarrierJumpStatus::CountingDown
            || self.jump_status == CarrierJumpStatus::Locked
        {
            Relevance::Active
        } else {
            Relevance::Normal
        }
    }

    fn on_event(&mut self, ev: &SourceEvent, _state: &GameState, _ctx: &mut ModuleCtx) -> TopicSet {
        let mut changed = TopicSet::new();

        match ev {
            SourceEvent::Status(status) => {
                if self.update_jump_clock(status.timestamp) {
                    changed.insert(TOPIC_JUMP.to_string());
                }
            }
            SourceEvent::Journal { ev, .. } => {
                let timestamp = ev.timestamp;
                if self.update_jump_clock(timestamp) {
                    changed.insert(TOPIC_JUMP.to_string());
                }

                match &ev.kind {
                    EventKind::CarrierStats(stats) => {
                        self.carrier_id = Some(stats.carrier_id);
                        self.callsign = Some(stats.callsign.clone());
                        self.name = Some(stats.name.clone());
                        self.docking_access = Some(stats.docking_access.clone());
                        self.fuel_depot = stats.fuel_level;
                        self.space_usage_cargo = stats.space_usage.cargo;
                        self.space_usage_free = stats.space_usage.free_space;
                        self.space_usage_total = stats.space_usage.total_capacity;

                        self.finance_balance = stats.finance.carrier_balance;
                        self.finance_reserve = stats.finance.reserve_balance;
                        self.finance_available = stats.finance.available_balance;
                        self.finance_weekly_upkeep = stats.estimated_weekly_upkeep();
                        self.finance_health = stats.finance.health(self.finance_weekly_upkeep);
                        self.finance_weeks_covered =
                            stats.finance.weeks_covered(self.finance_weekly_upkeep);

                        changed.insert(TOPIC_OVERVIEW.to_string());
                        changed.insert(TOPIC_FINANCES.to_string());
                    }
                    EventKind::CarrierFinance(fin) => {
                        self.finance_balance = fin.carrier_balance;
                        self.finance_reserve = fin.reserve_balance;
                        self.finance_available = fin.available_balance;
                        self.finance_health = fin.health(self.finance_weekly_upkeep);
                        self.finance_weeks_covered = fin.weeks_covered(self.finance_weekly_upkeep);

                        changed.insert(TOPIC_FINANCES.to_string());
                    }
                    EventKind::CarrierDepositFuel(deposit) => {
                        self.fuel_depot = deposit.total;
                        changed.insert(TOPIC_OVERVIEW.to_string());
                    }
                    EventKind::CarrierBankTransfer(transfer) => {
                        self.finance_balance = transfer.carrier_balance;
                        if self.finance_weekly_upkeep > 0 {
                            self.finance_weeks_covered =
                                self.finance_balance as f64 / self.finance_weekly_upkeep as f64;
                            self.finance_health = if self.finance_weeks_covered < 2.0 {
                                MaintenanceHealth::Critical
                            } else if self.finance_weeks_covered < 4.0 {
                                MaintenanceHealth::Warning
                            } else {
                                MaintenanceHealth::Safe
                            };
                        }
                        changed.insert(TOPIC_FINANCES.to_string());
                    }
                    EventKind::CarrierJumpRequest(req) => {
                        self.jump_destination_system = Some(req.system_name.clone());
                        self.jump_destination_body = req.body.clone();
                        self.jump_departure_time = req.departure_time;
                        self.jump_cancelled = false;

                        if let Some(dep) = req.departure_time {
                            let diff = dep.signed_duration_since(timestamp).num_seconds();
                            self.jump_status = if diff <= 120 {
                                CarrierJumpStatus::Locked
                            } else {
                                CarrierJumpStatus::CountingDown
                            };
                        } else {
                            self.jump_status = CarrierJumpStatus::CountingDown;
                        }

                        changed.insert(TOPIC_JUMP.to_string());
                    }
                    EventKind::CarrierJumpCancelled(_) => {
                        self.jump_cancelled = true;
                        self.jump_status = CarrierJumpStatus::Idle;
                        self.jump_destination_system = None;
                        self.jump_destination_body = None;
                        self.jump_departure_time = None;

                        changed.insert(TOPIC_JUMP.to_string());
                    }
                    EventKind::CarrierJump(fsd) => {
                        self.current_system = Some(fsd.star_system.clone());
                        self.jump_status = CarrierJumpStatus::Cooldown;
                        self.jump_cooldown_until = Some(timestamp + Duration::seconds(300));
                        self.jump_destination_system = None;
                        self.jump_destination_body = None;

                        changed.insert(TOPIC_OVERVIEW.to_string());
                        changed.insert(TOPIC_JUMP.to_string());
                    }
                    EventKind::Docked(docked)
                        if docked.station_type.as_deref() == Some("FleetCarrier") =>
                    {
                        let match_callsign = self.callsign.as_deref().is_some_and(|c| {
                            docked
                                .station_name
                                .as_deref()
                                .is_some_and(|sn| sn.contains(c))
                        });
                        let match_name = self.name.as_deref().is_some_and(|n| {
                            docked
                                .station_name
                                .as_deref()
                                .is_some_and(|sn| sn.contains(n))
                        });
                        if match_callsign || match_name {
                            self.is_aboard = true;
                            changed.insert(TOPIC_OVERVIEW.to_string());
                        }
                    }
                    EventKind::Undocked(_) => {
                        if self.is_aboard {
                            self.is_aboard = false;
                            changed.insert(TOPIC_OVERVIEW.to_string());
                        }
                    }
                    EventKind::Location(loc)
                        if loc.station_type.as_deref() == Some("FleetCarrier") =>
                    {
                        let match_callsign = self.callsign.as_deref().is_some_and(|c| {
                            loc.station_name.as_deref().is_some_and(|sn| sn.contains(c))
                        });
                        let match_name = self.name.as_deref().is_some_and(|n| {
                            loc.station_name.as_deref().is_some_and(|sn| sn.contains(n))
                        });
                        if match_callsign || match_name {
                            self.is_aboard = true;
                            changed.insert(TOPIC_OVERVIEW.to_string());
                        }
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
            TOPIC_OVERVIEW => {
                let fuel_total = self.fuel_depot + self.fuel_cargo;
                let max_jumps = fuel_total / 100;

                Some(json!({
                    "carrier_id": self.carrier_id,
                    "callsign": self.callsign,
                    "name": self.name,
                    "current_system": self.current_system,
                    "current_body": self.current_body,
                    "docking_access": self.docking_access,
                    "fuel_depot": self.fuel_depot,
                    "fuel_cargo": self.fuel_cargo,
                    "fuel_total": fuel_total,
                    "max_jumps_possible": max_jumps,
                    "cargo_free": self.space_usage_free,
                    "cargo_total": self.space_usage_total,
                    "cargo_used": self.space_usage_cargo,
                    "is_aboard": self.is_aboard,
                }))
            }
            TOPIC_JUMP => {
                let seconds_remaining = self.jump_departure_time.map(|dep| {
                    let diff = dep.signed_duration_since(now).num_seconds();
                    diff.max(0)
                });
                let cooldown_remaining = self.jump_cooldown_until.map(|until| {
                    let diff = until.signed_duration_since(now).num_seconds();
                    diff.max(0)
                });

                Some(json!({
                    "status": self.jump_status,
                    "destination_system": self.jump_destination_system,
                    "destination_body": self.jump_destination_body,
                    "departure_time": self.jump_departure_time,
                    "seconds_remaining": seconds_remaining,
                    "cooldown_seconds_remaining": cooldown_remaining.unwrap_or(0),
                    "is_cancelled": self.jump_cancelled,
                }))
            }
            TOPIC_FINANCES => {
                let next_upkeep = next_weekly_upkeep_time(now);

                Some(json!({
                    "balance": self.finance_balance,
                    "reserve_balance": self.finance_reserve,
                    "available_balance": self.finance_available,
                    "weekly_upkeep": self.finance_weekly_upkeep,
                    "maintenance_health": self.finance_health,
                    "weeks_covered": self.finance_weeks_covered,
                    "next_payment_time": next_upkeep,
                }))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
