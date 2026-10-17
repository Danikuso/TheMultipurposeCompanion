use crate::{Alert, TopicSet};
use serde::Serialize;
use tmc_model::{source::SourceEvent, state::GameState};

pub trait Module: Send {
    fn id(&self) -> &'static str;
    fn on_event(&mut self, ev: &SourceEvent, state: &GameState, ctx: &mut ModuleCtx) -> TopicSet;
    fn topic_value(&self, topic: &str) -> Option<serde_json::Value>;
    fn topics(&self) -> Vec<String>;
    fn relevance(&self, _state: &GameState) -> Relevance {
        Relevance::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Relevance {
    Hidden,
    Normal,
    Active,
    Urgent,
}

pub struct ModuleCtx {
    pub replaying: bool,
    alerts: Vec<Alert>,
}

impl ModuleCtx {
    pub(crate) fn new(replaying: bool) -> Self {
        Self {
            replaying,
            alerts: Vec::new(),
        }
    }

    pub fn alert(&mut self, alert: Alert) {
        if !self.replaying {
            self.alerts.push(alert);
        }
    }

    pub(crate) fn drain_alerts(&mut self) -> Vec<Alert> {
        std::mem::take(&mut self.alerts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Severity;

    #[test]
    fn module_context_suppresses_replay_and_drains_live_alerts_once() {
        let mut ctx = ModuleCtx::new(true);
        ctx.alert(Alert::new(Severity::Warn, "alert-low-fuel"));
        assert!(ctx.drain_alerts().is_empty());
        ctx.replaying = false;
        ctx.alert(Alert::new(Severity::Info, "alert-first"));
        ctx.alert(Alert::new(Severity::Critical, "alert-second"));
        let alerts = ctx.drain_alerts();
        assert_eq!(alerts.len(), 2);
        assert_eq!(alerts[0].key, "alert-first");
        assert_eq!(alerts[1].key, "alert-second");
        assert!(ctx.drain_alerts().is_empty());
    }

    struct MinimalModule;
    impl Module for MinimalModule {
        fn id(&self) -> &'static str {
            "minimal"
        }
        fn on_event(&mut self, _: &SourceEvent, _: &GameState, _: &mut ModuleCtx) -> TopicSet {
            TopicSet::new()
        }
        fn topic_value(&self, _: &str) -> Option<serde_json::Value> {
            None
        }
        fn topics(&self) -> Vec<String> {
            Vec::new()
        }
    }

    #[test]
    fn module_relevance_defaults_to_normal_and_serializes_lowercase() {
        assert_eq!(
            MinimalModule.relevance(&GameState::default()),
            Relevance::Normal
        );
        for (relevance, expected) in [
            (Relevance::Hidden, "hidden"),
            (Relevance::Normal, "normal"),
            (Relevance::Active, "active"),
            (Relevance::Urgent, "urgent"),
        ] {
            assert_eq!(serde_json::to_value(relevance).unwrap(), expected);
        }
    }
}
