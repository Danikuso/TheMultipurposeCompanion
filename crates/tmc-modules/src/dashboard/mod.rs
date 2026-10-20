use chrono::{DateTime, Utc};
use rules::{
    ActiveAlert, AlertSource, DEBOUNCE_WINDOW, EVENT_ALERT_TTL, MAX_ACTIVE_ALERTS,
    is_session_reset, journal_event_alert, status_flag_rules,
};
use std::collections::{BTreeMap, BTreeSet};
use tmc_core::{Alert, Module, ModuleCtx, Relevance, Severity, TopicSet};
use tmc_model::{source::SourceEvent, state::GameState, status::Status};

mod rules;

pub struct Dashboard {
    active_alerts: Vec<ActiveAlert>,
    active_flags: BTreeSet<&'static str>,
    last_notified: BTreeMap<String, DateTime<Utc>>,
    alerts_enabled: bool,
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            active_alerts: Vec::new(),
            active_flags: BTreeSet::new(),
            last_notified: BTreeMap::new(),
            alerts_enabled: true,
        }
    }

    fn reset(&mut self) {
        self.active_alerts.clear();
        self.active_flags.clear();
        self.last_notified.clear();
    }

    fn expire_event_alerts(&mut self, now: DateTime<Utc>) {
        let before = self.active_alerts.len();
        self.active_alerts.retain(|alert| {
            alert.source != AlertSource::JournalEvent || now < alert.since + EVENT_ALERT_TTL
        });
        decide_if!(self.active_alerts.len() != before, "dashboard.alerts.expired",
            {"removed": before - self.active_alerts.len()});
    }

    fn can_notify(&self, key: &str, now: DateTime<Utc>) -> bool {
        self.last_notified
            .get(key)
            .is_none_or(|&last| now >= last + DEBOUNCE_WINDOW)
    }

    fn push_active(&mut self, entry: ActiveAlert) {
        self.active_alerts.retain(|a| a.key != entry.key);
        self.active_alerts.push(entry);
        if self.active_alerts.len() > MAX_ACTIVE_ALERTS {
            let overflow = self.active_alerts.len() - MAX_ACTIVE_ALERTS;
            self.active_alerts.drain(0..overflow);
        }
    }

    fn process_status(&mut self, status: &Status, replaying: bool) -> Vec<Alert> {
        let now = status.timestamp;
        self.expire_event_alerts(now);
        if replaying {
            return Vec::new();
        }
        let mut emitted = Vec::new();
        #[allow(unused_variables)]
        for (flag, (key, severity, is_active)) in status_flag_rules(status).into_iter().enumerate()
        {
            if is_active {
                let was_active = !self.active_flags.insert(key);
                if !was_active {
                    let alert = Alert::new(severity, key);
                    self.push_active(ActiveAlert::from_alert(
                        &alert,
                        now,
                        AlertSource::StatusFlag,
                    ));
                    decide!("dashboard.status_flag.activated", {"flag": flag});
                    if self.can_notify(key, now) {
                        self.last_notified.insert(key.to_owned(), now);
                        emitted.push(alert);
                        decide!(
                            "dashboard.status_flag.notified",
                            tmc_model::diagnostics::json!({})
                        );
                    } else {
                        decide!(
                            "dashboard.status_flag.suppressed_cooldown",
                            tmc_model::diagnostics::json!({})
                        );
                    }
                }
            } else if self.active_flags.remove(key) {
                decide!("dashboard.status_flag.cleared", {"flag": flag});
                self.active_alerts.retain(|a| a.key != key);
            }
        }
        emitted
    }

    fn process(&mut self, ev: &SourceEvent, _state: &GameState, replaying: bool) -> Vec<Alert> {
        match ev {
            SourceEvent::Settings(settings) => {
                self.alerts_enabled = settings.alerts_enabled;
                if !self.alerts_enabled {
                    self.reset();
                }
                Vec::new()
            }
            // Con las alertas desactivadas no se evalúa nada ni se acumula estado.
            _ if !self.alerts_enabled => Vec::new(),
            SourceEvent::Status(status) => self.process_status(status, replaying),
            SourceEvent::Journal {
                ev,
                replaying: ev_replaying,
                ..
            } => {
                if is_session_reset(ev) {
                    decide!("dashboard.session.reset", tmc_model::diagnostics::json!({}));
                    self.reset();
                    return Vec::new();
                }
                let now = ev.timestamp;
                self.expire_event_alerts(now);
                if replaying || *ev_replaying {
                    return Vec::new();
                }
                let Some(alert) = journal_event_alert(ev) else {
                    decide_if!(
                        matches!(ev.kind, tmc_model::events::EventKind::HullDamage(_)),
                        "dashboard.journal_alert.discarded_by_type",
                        {}
                    );
                    return Vec::new();
                };
                self.push_active(ActiveAlert::from_alert(
                    &alert,
                    now,
                    AlertSource::JournalEvent,
                ));
                if !self.can_notify(&alert.key, now) {
                    decide!(
                        "dashboard.journal_alert.suppressed_cooldown",
                        tmc_model::diagnostics::json!({})
                    );
                    return Vec::new();
                }
                decide!(
                    "dashboard.journal_alert.notified",
                    tmc_model::diagnostics::json!({})
                );
                self.last_notified.insert(alert.key.clone(), now);
                vec![alert]
            }
            _ => Vec::new(),
        }
    }
}

impl Default for Dashboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Dashboard {
    fn id(&self) -> &'static str {
        "dashboard"
    }

    fn on_event(&mut self, ev: &SourceEvent, state: &GameState, ctx: &mut ModuleCtx) -> TopicSet {
        for alert in self.process(ev, state, ctx.replaying) {
            ctx.alert(alert);
        }
        ["dashboard.alerts".into()].into()
    }

    fn topic_value(&self, topic: &str) -> Option<serde_json::Value> {
        (topic == "dashboard.alerts")
            .then(|| serde_json::to_value(&self.active_alerts).unwrap_or_default())
    }

    fn topics(&self) -> Vec<String> {
        vec!["dashboard.alerts".into()]
    }

    fn relevance(&self, _state: &GameState) -> Relevance {
        if self
            .active_alerts
            .iter()
            .any(|a| a.severity == Severity::Critical)
        {
            Relevance::Urgent
        } else {
            Relevance::Normal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Duration, Utc};
    use tmc_core::Severity;
    use tmc_model::{
        events::parse_line,
        status::{Flags, Flags2, Status},
    };

    fn time(seconds: i64) -> DateTime<Utc> {
        "2026-10-17T18:00:00Z".parse::<DateTime<Utc>>().unwrap() + Duration::seconds(seconds)
    }
    fn status(seconds: i64, flags: Flags, flags2: Flags2) -> SourceEvent {
        SourceEvent::Status(
            Status::parse(&format!(
                r#"{{"timestamp":"{}","Flags":{},"Flags2":{}}}"#,
                time(seconds).to_rfc3339(),
                flags.bits(),
                flags2.bits()
            ))
            .unwrap(),
        )
    }
    fn journal(seconds: i64, name: &str, extra: &str, replaying: bool) -> SourceEvent {
        SourceEvent::Journal {
            ev: parse_line(&format!(
                r#"{{"timestamp":"{}","event":"{name}"{extra}}}"#,
                time(seconds).to_rfc3339()
            ))
            .unwrap(),
            file: "synthetic.log".into(),
            line: 1,
            replaying,
        }
    }
    fn apply(dashboard: &mut Dashboard, state: &mut GameState, event: SourceEvent) -> Vec<Alert> {
        match &event {
            SourceEvent::Status(s) => {
                state.apply_status(s);
            }
            SourceEvent::Journal { ev, .. } => {
                state.apply_event(ev);
            }
            _ => {}
        }
        let replaying = matches!(
            &event,
            SourceEvent::Journal {
                replaying: true,
                ..
            }
        );
        dashboard.process(&event, state, replaying)
    }
    fn active(dashboard: &Dashboard) -> Vec<serde_json::Value> {
        dashboard
            .topic_value("dashboard.alerts")
            .unwrap()
            .as_array()
            .unwrap()
            .clone()
    }

    #[test]
    fn low_fuel_alert_on_rising_edge_only() {
        let mut d = Dashboard::new();
        let mut g = GameState::default();
        let alerts = apply(&mut d, &mut g, status(0, Flags::LOW_FUEL, Flags2::empty()));
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].key, "alert-low-fuel");
        assert!(apply(&mut d, &mut g, status(35, Flags::LOW_FUEL, Flags2::empty())).is_empty());
        assert_eq!(active(&d).len(), 1);
    }

    #[test]
    fn same_alert_debounced_for_30s() {
        let mut d = Dashboard::new();
        let mut g = GameState::default();
        assert_eq!(
            apply(&mut d, &mut g, journal(0, "UnderAttack", "", false)).len(),
            1
        );
        assert!(apply(&mut d, &mut g, journal(29, "UnderAttack", "", false)).is_empty());
        assert_eq!(
            apply(&mut d, &mut g, journal(30, "UnderAttack", "", false)).len(),
            1
        );
    }

    #[test]
    fn hull_damage_severity_by_threshold() {
        for (health, severity, pct) in [
            (0.25, Severity::Critical, 25),
            (0.251, Severity::Warn, 25),
            (0.804, Severity::Warn, 80),
        ] {
            let mut d = Dashboard::new();
            let mut g = GameState::default();
            let alerts = apply(
                &mut d,
                &mut g,
                journal(
                    0,
                    "HullDamage",
                    &format!(r#", "Health":{health},"PlayerPilot":true"#),
                    false,
                ),
            );
            assert_eq!(alerts.len(), 1);
            assert_eq!(alerts[0].severity, severity);
            assert_eq!(alerts[0].args["pct"], pct);
        }
        let mut d = Dashboard::new();
        let mut g = GameState::default();
        assert!(
            apply(
                &mut d,
                &mut g,
                journal(
                    0,
                    "HullDamage",
                    r#", "Health":0.1,"PlayerPilot":false"#,
                    false
                )
            )
            .is_empty()
        );
        assert!(apply(&mut d, &mut g, journal(1, "HullDamage", "", false)).is_empty());
    }

    #[test]
    fn replayed_events_do_not_raise_alerts() {
        let mut d = Dashboard::new();
        let mut g = GameState::default();
        assert!(apply(&mut d, &mut g, journal(0, "UnderAttack", "", true)).is_empty());
        assert!(active(&d).is_empty());
        assert_eq!(
            apply(&mut d, &mut g, journal(1, "UnderAttack", "", false)).len(),
            1
        );
    }

    #[test]
    fn alert_clears_when_flag_drops() {
        let mut d = Dashboard::new();
        let mut g = GameState::default();
        apply(
            &mut d,
            &mut g,
            status(0, Flags::BEING_INTERDICTED, Flags2::empty()),
        );
        assert_eq!(d.relevance(&g), Relevance::Urgent);
        apply(&mut d, &mut g, status(1, Flags::empty(), Flags2::empty()));
        assert!(active(&d).is_empty());
        assert_eq!(d.relevance(&g), Relevance::Normal);
    }

    #[tokio::test(start_paused = true)]
    async fn dashboard_emits_no_alerts_when_disabled_by_settings() {
        use tmc_core::CoreMessage;
        use tmc_model::settings::ModuleSettings;
        for (enabled, expected) in [(false, 0usize), (true, 1usize)] {
            let core = tmc_core::spawn_core(
                crate::all_modules(),
                None,
                std::time::Duration::from_millis(100),
            );
            let mut rx = core.subscribe();
            core.events
                .send(SourceEvent::Settings(ModuleSettings {
                    alerts_enabled: enabled,
                    ..ModuleSettings::default()
                }))
                .await
                .unwrap();
            core.events
                .send(status(0, Flags::LOW_FUEL, Flags2::empty()))
                .await
                .unwrap();
            let snapshot = core.snapshot().await;
            let mut alerts = 0;
            while let Ok(msg) = rx.try_recv() {
                if let CoreMessage::Alert(a) = msg {
                    assert_eq!(a.key, "alert-low-fuel");
                    alerts += 1;
                }
            }
            assert_eq!(alerts, expected, "alerts_enabled={enabled}");
            assert_eq!(
                snapshot["dashboard.alerts"].as_array().unwrap().len(),
                expected
            );
        }
    }

    #[test]
    fn registry_contains_dashboard() {
        let modules = crate::all_modules();
        assert_eq!(modules.len(), 6);
        assert_eq!(modules[0].id(), "dashboard");
        assert_eq!(modules[0].topics(), vec!["dashboard.alerts"]);
        assert!(modules[0].topic_value("unknown").is_none());
    }

    #[test]
    fn all_status_rules_raise_expected_severity_and_preserve_since() {
        let mut d = Dashboard::new();
        let mut g = GameState::default();
        let alerts = apply(
            &mut d,
            &mut g,
            status(
                0,
                Flags::LOW_FUEL | Flags::OVERHEATING | Flags::BEING_INTERDICTED,
                Flags2::LOW_OXYGEN | Flags2::LOW_HEALTH,
            ),
        );
        assert_eq!(alerts.len(), 5);
        for key in ["alert-interdiction", "alert-low-oxygen", "alert-low-health"] {
            assert_eq!(
                alerts.iter().find(|a| a.key == key).unwrap().severity,
                Severity::Critical
            );
        }
        let before = active(&d);
        assert!(
            apply(
                &mut d,
                &mut g,
                status(
                    40,
                    Flags::LOW_FUEL | Flags::OVERHEATING | Flags::BEING_INTERDICTED,
                    Flags2::LOW_OXYGEN | Flags2::LOW_HEALTH
                )
            )
            .is_empty()
        );
        assert_eq!(active(&d), before);
    }

    #[test]
    fn condition_reentry_is_visible_but_notification_debounced() {
        let mut d = Dashboard::new();
        let mut g = GameState::default();
        apply(
            &mut d,
            &mut g,
            status(0, Flags::BEING_INTERDICTED, Flags2::empty()),
        );
        apply(&mut d, &mut g, status(1, Flags::empty(), Flags2::empty()));
        assert!(
            apply(
                &mut d,
                &mut g,
                status(2, Flags::BEING_INTERDICTED, Flags2::empty())
            )
            .is_empty()
        );
        assert_eq!(active(&d).len(), 1);
        assert_eq!(d.relevance(&g), Relevance::Urgent);
        apply(&mut d, &mut g, status(29, Flags::empty(), Flags2::empty()));
        assert_eq!(
            apply(
                &mut d,
                &mut g,
                status(30, Flags::BEING_INTERDICTED, Flags2::empty())
            )
            .len(),
            1
        );
    }

    #[test]
    fn event_alert_expires_at_exact_60_second_timestamp_without_reemission() {
        let mut d = Dashboard::new();
        let mut g = GameState::default();
        apply(
            &mut d,
            &mut g,
            journal(0, "HullDamage", r#", "Health":0.25"#, false),
        );
        assert_eq!(d.relevance(&g), Relevance::Urgent);
        assert!(apply(&mut d, &mut g, journal(59, "Unknown", "", false)).is_empty());
        assert_eq!(active(&d).len(), 1);
        assert!(apply(&mut d, &mut g, journal(60, "Unknown", "", false)).is_empty());
        assert!(active(&d).is_empty());
        assert_eq!(d.relevance(&g), Relevance::Normal);
    }

    #[test]
    fn new_session_clears_alerts_and_debounce_from_previous_commander() {
        let mut d = Dashboard::new();
        let mut g = GameState::default();
        apply(&mut d, &mut g, journal(0, "UnderAttack", "", false));
        apply(
            &mut d,
            &mut g,
            journal(1, "LoadGame", r#", "Commander":"TEST_B""#, false),
        );
        assert!(active(&d).is_empty());
        assert_eq!(
            apply(&mut d, &mut g, journal(2, "UnderAttack", "", false)).len(),
            1
        );
        apply(&mut d, &mut g, journal(3, "Shutdown", "", false));
        assert!(active(&d).is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn dashboard_snapshot_exposes_complete_wire_alert_and_urgent_relevance() {
        let core = tmc_core::spawn_core(
            crate::all_modules(),
            None,
            std::time::Duration::from_millis(100),
        );
        core.events
            .send(status(0, Flags::BEING_INTERDICTED, Flags2::empty()))
            .await
            .unwrap();
        let snapshot = core.snapshot().await;
        let alert = &snapshot["dashboard.alerts"][0];
        for key in ["id", "severity", "key", "args", "since"] {
            assert!(alert.get(key).is_some(), "{key}");
        }
        assert_eq!(alert["severity"], "critical");
        assert_eq!(alert["key"], "alert-interdiction");
        assert_eq!(snapshot["core.modules"][0]["relevance"], "urgent");
    }

    #[test]
    fn hull_damage_escalation_within_debounce_updates_active_alert_and_relevance() {
        let mut d = Dashboard::new();
        let mut g = GameState::default();
        let first = apply(
            &mut d,
            &mut g,
            journal(
                0,
                "HullDamage",
                r#", "Health":0.60,"PlayerPilot":true"#,
                false,
            ),
        );
        assert_eq!(first.len(), 1);
        assert_eq!(d.relevance(&g), Relevance::Normal);
        let second = apply(
            &mut d,
            &mut g,
            journal(
                5,
                "HullDamage",
                r#", "Health":0.20,"PlayerPilot":true"#,
                false,
            ),
        );
        assert!(second.is_empty());
        assert_eq!(d.relevance(&g), Relevance::Urgent);
        assert_eq!(active(&d)[0]["args"]["pct"], 20);
    }
}
