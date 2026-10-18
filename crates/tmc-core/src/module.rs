use crate::{Alert, TopicSet};
use serde::Serialize;
use tmc_model::{
    net::{NetQuery, NetRequest},
    source::SourceEvent,
    state::GameState,
};
use tmc_store::{Migration, Store, StoreError};

/// Tarea de SQLite que el actor ejecuta fuera de su bucle (`spawn_blocking`).
pub type StoreTask = Box<dyn FnOnce(&Store) -> Result<serde_json::Value, StoreError> + Send>;

pub trait Module: Send {
    fn id(&self) -> &'static str;
    fn on_event(&mut self, ev: &SourceEvent, state: &GameState, ctx: &mut ModuleCtx) -> TopicSet;
    fn topic_value(&self, topic: &str) -> Option<serde_json::Value>;
    fn topics(&self) -> Vec<String>;
    fn relevance(&self, _state: &GameState) -> Relevance {
        Relevance::Normal
    }
    /// Migraciones de SQLite propias del modulo (se aplican al arrancar).
    fn migrations(&self) -> &'static [Migration] {
        &[]
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
    module: &'static str,
    next_request_id: u64,
    alerts: Vec<Alert>,
    requests: Vec<NetRequest>,
    store_tasks: Vec<(String, StoreTask)>,
}

impl ModuleCtx {
    /// Lo crea el actor; es público para probar módulos de forma aislada.
    #[doc(hidden)]
    pub fn new(module: &'static str, replaying: bool, next_request_id: u64) -> Self {
        Self {
            replaying,
            module,
            next_request_id,
            alerts: Vec::new(),
            requests: Vec::new(),
            store_tasks: Vec::new(),
        }
    }

    /// Pide una consulta de red. Devuelve el id de la peticion, o `None` (y la
    /// descarta) durante el replay.
    pub fn request(&mut self, key: impl Into<String>, query: NetQuery) -> Option<u64> {
        if self.replaying {
            return None;
        }
        let id = self.next_request_id;
        self.next_request_id += 1;
        self.requests.push(NetRequest {
            id,
            module: self.module.to_owned(),
            key: key.into(),
            query,
        });
        Some(id)
    }

    /// Encarga trabajo de SQLite; el resultado vuelve como `SourceEvent::Stored`.
    /// Se ejecuta tambien durante el replay.
    pub fn store_task(&mut self, tag: impl Into<String>, task: StoreTask) {
        self.store_tasks.push((tag.into(), task));
    }

    #[allow(clippy::type_complexity)]
    /// Vacía peticiones y tareas acumuladas; público para probar módulos.
    #[doc(hidden)]
    pub fn take_outputs(&mut self) -> (Vec<NetRequest>, Vec<(String, StoreTask)>, u64) {
        (
            std::mem::take(&mut self.requests),
            std::mem::take(&mut self.store_tasks),
            self.next_request_id,
        )
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
        let mut ctx = ModuleCtx::new("test", true, 1);
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
    fn module_migrations_default_empty() {
        assert!(MinimalModule.migrations().is_empty());
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
