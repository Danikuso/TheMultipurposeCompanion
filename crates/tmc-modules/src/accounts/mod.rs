//! Módulo `accounts`: orquestación de sincronización externa (EDDN, EDSM, Inara, Frontier CAPI)
//! y publicación del topic `accounts.status`.

use chrono::Utc;
use tmc_core::{Module, ModuleCtx, Relevance, TopicSet};
use tmc_model::accounts::{AccountsStatusTopic, CapiStatus, EddnStatus, EdsmStatus, InaraStatus};
use tmc_model::events::EventKind;
use tmc_model::source::SourceEvent;
use tmc_model::state::{Galaxy, GameState};

pub const TOPIC_STATUS: &str = "accounts.status";
const TOPICS: [&str; 1] = [TOPIC_STATUS];

#[derive(Debug, Clone, Default)]
pub struct AccountsModule {
    status: AccountsStatusTopic,
}

impl AccountsModule {
    pub fn new() -> Self {
        Self {
            status: AccountsStatusTopic {
                eddn: EddnStatus {
                    enabled: true,
                    connected: true,
                    uploader_id: "TMC-Live-Uploader".to_string(),
                    ..Default::default()
                },
                edsm: EdsmStatus::default(),
                inara: InaraStatus::default(),
                capi: CapiStatus::default(),
            },
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

    fn on_event(&mut self, ev: &SourceEvent, state: &GameState, ctx: &mut ModuleCtx) -> TopicSet {
        let mut changed = TopicSet::new();

        match ev {
            SourceEvent::Journal { ev, replaying, .. } => {
                // Innegociable §0 y §1.5: La galaxia Legacy tiene aborto absoluto de cualquier sincronización
                if state.galaxy == Galaxy::Legacy {
                    decide!("accounts.dispatch.legacy_rejected", { "galaxy": "Legacy" });
                    return changed;
                }

                // Durante replaying histórico no se transmiten eventos a gateways públicos
                if *replaying || ctx.replaying {
                    return changed;
                }

                match &ev.kind {
                    EventKind::Commander(cmdr) => {
                        if let Some(name) = &cmdr.name {
                            self.status.edsm.commander_name = Some(name.clone());
                            self.status.inara.commander_name = Some(name.clone());
                            self.status.capi.commander_name = Some(name.clone());
                            if self.status.eddn.uploader_id.is_empty()
                                || self.status.eddn.uploader_id == "TMC-Live-Uploader"
                            {
                                self.status.eddn.uploader_id = format!("TMC-{name}");
                            }
                            changed.insert(TOPIC_STATUS.to_string());
                        }
                    }
                    EventKind::FsdJump(_) => {
                        let now_iso = ev.timestamp.to_rfc3339();
                        if self.status.eddn.enabled {
                            self.status.eddn.messages_sent_session += 1;
                            self.status.eddn.last_upload_time = Some(now_iso.clone());
                        }
                        if self.status.edsm.enabled && self.status.edsm.has_key {
                            self.status.edsm.last_sync_time = Some(now_iso.clone());
                        }
                        if self.status.inara.enabled && self.status.inara.has_key {
                            self.status.inara.pending_events += 1;
                        }
                        changed.insert(TOPIC_STATUS.to_string());
                    }
                    EventKind::Market | EventKind::Outfitting | EventKind::Shipyard => {
                        if self.status.eddn.enabled {
                            self.status.eddn.messages_sent_session += 1;
                            self.status.eddn.last_upload_time = Some(ev.timestamp.to_rfc3339());
                            changed.insert(TOPIC_STATUS.to_string());
                        }
                    }
                    EventKind::Rank(_) | EventKind::Progress(_) => {
                        let now_iso = ev.timestamp.to_rfc3339();
                        if self.status.edsm.enabled && self.status.edsm.has_key {
                            self.status.edsm.last_sync_time = Some(now_iso);
                        }
                        if self.status.inara.enabled && self.status.inara.has_key {
                            self.status.inara.pending_events += 1;
                        }
                        changed.insert(TOPIC_STATUS.to_string());
                    }
                    EventKind::Loadout(_)
                    | EventKind::Materials(_)
                    | EventKind::ShipLocker(_)
                    | EventKind::CarrierStats(_)
                    | EventKind::CommunityGoal(_)
                        if self.status.inara.enabled && self.status.inara.has_key =>
                    {
                        self.status.inara.pending_events += 1;
                        changed.insert(TOPIC_STATUS.to_string());
                    }
                    _ => {}
                }
            }
            SourceEvent::Action { name, args } => match name.as_str() {
                "accounts.sync_now" => {
                    if state.galaxy == Galaxy::Live {
                        let now_iso = Utc::now().to_rfc3339();
                        if self.status.edsm.enabled {
                            self.status.edsm.last_sync_time = Some(now_iso.clone());
                        }
                        if self.status.inara.enabled {
                            self.status.inara.last_sync_time = Some(now_iso);
                            self.status.inara.pending_events = 0;
                        }
                        changed.insert(TOPIC_STATUS.to_string());
                    }
                }
                "accounts.set_settings" => {
                    if let Some(eddn_en) = args.get("eddn_enabled").and_then(|v| v.as_bool()) {
                        self.status.eddn.enabled = eddn_en;
                    }
                    if let Some(edsm_en) = args.get("edsm_enabled").and_then(|v| v.as_bool()) {
                        self.status.edsm.enabled = edsm_en;
                    }
                    if let Some(edsm_key) = args.get("edsm_has_key").and_then(|v| v.as_bool()) {
                        self.status.edsm.has_key = edsm_key;
                    }
                    if let Some(inara_en) = args.get("inara_enabled").and_then(|v| v.as_bool()) {
                        self.status.inara.enabled = inara_en;
                    }
                    if let Some(inara_key) = args.get("inara_has_key").and_then(|v| v.as_bool()) {
                        self.status.inara.has_key = inara_key;
                    }
                    if let Some(capi_en) = args.get("capi_enabled").and_then(|v| v.as_bool()) {
                        self.status.capi.configured = capi_en;
                    }
                    if let Some(capi_conn) = args.get("capi_connected").and_then(|v| v.as_bool()) {
                        self.status.capi.connected = capi_conn;
                    }
                    if let Some(ships) = args.get("remote_ships_count").and_then(|v| v.as_u64()) {
                        self.status.capi.remote_ships_count = ships as u32;
                    }
                    changed.insert(TOPIC_STATUS.to_string());
                }
                _ => {}
            },
            _ => {}
        }

        changed
    }
}

#[cfg(test)]
mod tests;
