//! Módulo `accounts`: publicación del topic `accounts.status` a partir de `SyncReport`
//! reales de `tmc-sync` y del estado de Frontier CAPI.

use tmc_core::{Module, ModuleCtx, Relevance, TopicSet};
use tmc_model::accounts::AccountsStatusTopic;
use tmc_model::source::SourceEvent;
use tmc_model::state::GameState;
use tmc_model::sync::SyncReport;

pub const TOPIC_STATUS: &str = "accounts.status";
const TOPICS: [&str; 1] = [TOPIC_STATUS];

#[derive(Debug, Clone, Default)]
pub struct AccountsModule {
    status: AccountsStatusTopic,
}

impl AccountsModule {
    pub fn new() -> Self {
        Self {
            status: AccountsStatusTopic::default(),
        }
    }

    pub fn with_status(status: AccountsStatusTopic) -> Self {
        Self { status }
    }

    pub fn status(&self) -> &AccountsStatusTopic {
        &self.status
    }
}

impl Module for AccountsModule {
    fn id(&self) -> &'static str {
        "accounts"
    }

    fn topics(&self) -> Vec<String> {
        TOPICS.iter().map(|s| (*s).to_string()).collect()
    }

    fn relevance(&self, _state: &GameState) -> Relevance {
        Relevance::Normal
    }

    fn topic_value(&self, topic: &str) -> Option<serde_json::Value> {
        if topic == TOPIC_STATUS {
            serde_json::to_value(&self.status).ok()
        } else {
            None
        }
    }

    fn on_event(&mut self, ev: &SourceEvent, _state: &GameState, _ctx: &mut ModuleCtx) -> TopicSet {
        let mut changed = TopicSet::new();

        match ev {
            SourceEvent::SyncReport(report) => {
                match &**report {
                    SyncReport::Inara(inara) => self.status.inara = inara.clone(),
                    SyncReport::Edsm(edsm) => self.status.edsm = edsm.clone(),
                    SyncReport::Eddn(eddn) => self.status.eddn = eddn.clone(),
                    SyncReport::Roster(roster) => {
                        self.status.current_cmdr = roster.current_cmdr.clone();
                        self.status.commanders = roster.commanders.clone();
                        self.status.other_cmdrs_pending = roster.other_cmdrs_pending.clone();
                        self.status.unassigned_keys = roster.unassigned_keys.clone();
                    }
                }
                changed.insert(TOPIC_STATUS.to_string());
            }
            SourceEvent::Action { name, args } if name == "accounts.set_settings" => {
                let mut touched = false;
                if let Some(capi_en) = args.get("capi_enabled").and_then(|v| v.as_bool()) {
                    self.status.capi.configured = capi_en;
                    touched = true;
                }
                if let Some(capi_conn) = args.get("capi_connected").and_then(|v| v.as_bool()) {
                    self.status.capi.connected = capi_conn;
                    touched = true;
                }
                if let Some(ships) = args
                    .get("capi_remote_ships_count")
                    .or_else(|| args.get("remote_ships_count"))
                    .and_then(|v| v.as_u64())
                {
                    self.status.capi.remote_ships_count = ships as u32;
                    touched = true;
                }
                if let Some(capi_cmdr) = args.get("capi_commander_name").and_then(|v| v.as_str()) {
                    self.status.capi.commander_name = Some(capi_cmdr.to_string());
                    touched = true;
                }
                if touched {
                    changed.insert(TOPIC_STATUS.to_string());
                }
            }
            _ => {}
        }

        changed
    }
}

#[cfg(test)]
mod tests;
