//! Módulo `exploration`: cuerpos del sistema, progreso del FSS, valores y consejo.

pub mod system;
pub mod unsold;
pub mod value;

use serde_json::Value;
use system::SystemBodies;
use tmc_core::{Module, ModuleCtx, Relevance, TopicSet};
use tmc_model::events::EventKind;
use tmc_model::settings::ModuleSettings;
use tmc_model::source::SourceEvent;
use tmc_model::state::GameState;
use tmc_model::status::GuiFocus;

const TOPIC_SYSTEM: &str = "exploration.system";
const TOPIC_UNSOLD: &str = "exploration.unsold";

pub struct Exploration {
    threshold: u64,
    /// Del `LoadGame` del diario; por defecto `true` (juego actual).
    odyssey: bool,
    system: SystemBodies,
    system_value: Value,
    unsold: unsold::UnsoldState,
    unsold_value: Value,
}

impl Exploration {
    pub fn new() -> Self {
        Self {
            threshold: ModuleSettings::default().map_value_threshold,
            odyssey: true,
            system: SystemBodies::default(),
            system_value: Value::Null,
            unsold: unsold::UnsoldState::default(),
            unsold_value: serde_json::json!({"total":0,"systems":0,"at_risk":false}),
        }
    }

    /// Aplica un evento del diario; devuelve si pudo cambiar el tópico.
    fn on_journal(&mut self, kind: &EventKind) -> bool {
        match kind {
            EventKind::LoadGame(l) => {
                let odyssey = l.odyssey.unwrap_or(self.odyssey);
                std::mem::replace(&mut self.odyssey, odyssey) != odyssey
            }
            EventKind::FsdJump(l) | EventKind::CarrierJump(l) | EventKind::Location(l) => {
                self.system.enter(&l.star_system, l.system_address)
            }
            EventKind::Scan(s) => self.system.on_scan(s),
            EventKind::FssDiscoveryScan(d) => {
                self.system.on_fss_discovery(d.body_count, d.system_address)
            }
            EventKind::FssAllBodiesFound(a) => self.system.on_all_found(a.count, a.system_address),
            EventKind::FssBodySignals(f) => self.system.on_fss_signals(f.body_id, &f.signals),
            EventKind::SaaSignalsFound(f) => self.system.on_saa_signals(f.body_id, &f.signals),
            EventKind::SaaScanComplete(c) => {
                self.system
                    .on_saa_complete(c.body_id, c.probes_used, c.efficiency_target)
            }
            _ => false,
        }
    }
}

impl Default for Exploration {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Exploration {
    fn id(&self) -> &'static str {
        "exploration"
    }

    fn on_event(&mut self, ev: &SourceEvent, state: &GameState, ctx: &mut ModuleCtx) -> TopicSet {
        let unsold_changed = self.unsold.on_event(ev, state, ctx);
        let changed = match ev {
            SourceEvent::Settings(s) => {
                std::mem::replace(&mut self.threshold, s.map_value_threshold)
                    != s.map_value_threshold
            }
            SourceEvent::Journal { ev, .. } => self.on_journal(&ev.kind),
            _ => false,
        };
        let mut dirty = TopicSet::new();
        if unsold_changed {
            let unsold = self.unsold.topic();
            if self.unsold_value != unsold {
                self.unsold_value = unsold;
                decide!(
                    "exploration.unsold.topic.dirty",
                    tmc_model::diagnostics::json!({})
                );
                dirty.insert(TOPIC_UNSOLD.to_owned());
            }
        }
        if changed && self.system.has_data() {
            let value = serde_json::to_value(self.system.render(self.threshold, self.odyssey))
                .unwrap_or_default();
            if self.system_value != value {
                self.system_value = value;
                decide!(
                    "exploration.system.topic.dirty",
                    tmc_model::diagnostics::json!({"threshold": self.threshold})
                );
                dirty.insert(TOPIC_SYSTEM.to_owned());
            }
        }
        dirty
    }

    fn topic_value(&self, topic: &str) -> Option<Value> {
        if topic == TOPIC_UNSOLD {
            return Some(self.unsold_value.clone());
        }
        (topic == TOPIC_SYSTEM && !self.system_value.is_null()).then(|| self.system_value.clone())
    }

    fn topics(&self) -> Vec<String> {
        vec![TOPIC_SYSTEM.to_owned(), TOPIC_UNSOLD.to_owned()]
    }

    /// `Active` con el FSS abierto (`GuiFocus::Fss`) o con cuerpos del FSS sin escanear.
    fn relevance(&self, state: &GameState) -> Relevance {
        let fss_open = state
            .status
            .as_ref()
            .is_some_and(|s| s.gui_focus == GuiFocus::Fss);
        let pending = self
            .system
            .body_count()
            .is_some_and(|n| n > self.system.scanned());
        if fss_open || pending {
            Relevance::Active
        } else {
            Relevance::Normal
        }
    }
}

#[cfg(test)]
mod tests;
