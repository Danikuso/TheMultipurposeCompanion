//! Mapeador de eventos del diario hacia EDSM (`api-journal-v1`).

use super::discard::{DiscardList, DiscardStatus};
use crate::router::{MappedInput, MappedRow, ServiceMapper};
use crate::tracker::TransientSnapshot;
use serde_json::{Value, json};
use std::sync::Arc;
use tmc_model::events::JournalEvent;
use tmc_model::sync::{ErrorClass, OutboxState, SyncService};

/// Lista de privacidad local de TMC: nunca se envían ni encolan en EDSM (DR-16).
pub const TMC_PRIVACY_LIST: &[&str] = &[
    "SendText",
    "ReceiveText",
    "Friends",
    "WingAdd",
    "WingInvite",
    "WingJoin",
    "WingLeave",
    "CrewMemberJoins",
    "CrewMemberQuits",
    "CrewMemberRoleChange",
    "CrewHire",
    "CrewFire",
    "CrewAssign",
    "KickCrewMember",
    "JoinACrew",
    "QuitACrew",
    "EndCrewSession",
];

pub fn build_message(ev: &JournalEvent, snap: &TransientSnapshot) -> Option<Value> {
    let mut val: Value = serde_json::from_str(ev.raw.get()).ok()?;
    let obj = val.as_object_mut()?;
    obj.insert("_systemAddress".into(), json!(snap.system_address));
    obj.insert("_systemName".into(), json!(snap.system_name));
    obj.insert("_systemCoordinates".into(), json!(snap.star_pos));
    obj.insert("_marketId".into(), json!(snap.market_id));
    obj.insert("_stationName".into(), json!(snap.station_name));
    obj.insert("_shipId".into(), json!(snap.ship_id));
    Some(val)
}

pub struct EdsmMapper {
    discard: Arc<DiscardList>,
}

impl EdsmMapper {
    pub fn new(discard: Arc<DiscardList>) -> Self {
        Self { discard }
    }
}

impl ServiceMapper for EdsmMapper {
    fn service(&self) -> SyncService {
        SyncService::Edsm
    }

    fn map(&mut self, input: &MappedInput<'_>, out: &mut Vec<MappedRow>) {
        if TMC_PRIVACY_LIST.contains(&input.ev.name.as_str()) {
            return;
        }

        let status = self.discard.status(&input.ev.name);
        match status {
            DiscardStatus::Discarded => out.push(MappedRow {
                kind: input.ev.name.clone(),
                sub: 0,
                payload: json!({}),
                coalesce_key: None,
                state: OutboxState::Discarded,
                class: None,
            }),
            DiscardStatus::Missing | DiscardStatus::Keep => {
                let Some(msg) = build_message(input.ev, input.snap) else {
                    return;
                };
                let payload = json!({
                    "fromGameVersion": input.snap.gameversion.as_deref().unwrap_or(""),
                    "fromGameBuild": input.snap.gamebuild.as_deref().unwrap_or(""),
                    "message": msg,
                });
                out.push(MappedRow {
                    kind: input.ev.name.clone(),
                    sub: 0,
                    payload,
                    coalesce_key: None,
                    state: OutboxState::Pending,
                    class: matches!(status, DiscardStatus::Missing)
                        .then_some(ErrorClass::DiscardListMissing),
                });
            }
        }
    }
}
