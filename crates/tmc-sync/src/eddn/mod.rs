//! Mapeador EDDN y saneado conforme a los 16 esquemas oficiales.

pub mod journal;
pub mod sanitize;

use crate::router::{MappedInput, MappedRow, ServiceMapper};
use serde_json::{Value, json};
use tmc_model::sync::{ErrorClass, OutboxState, SyncService};
use tmc_net::eddn_upload::{SchemaMode, build_envelope, schema_ref, uploader_id};

/// Motivo por el que un evento EDDN se descarta localmente (`Omitted`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmitReason {
    Inconsistent,
    LocalValidation(String),
}

pub struct EddnMapper {
    mode: SchemaMode,
    salt: String,
}

impl EddnMapper {
    pub fn new(mode: SchemaMode, salt: impl Into<String>) -> Self {
        Self {
            mode,
            salt: salt.into(),
        }
    }

    pub(crate) fn build_row(
        &self,
        schema: &str,
        sub: u32,
        snap: &crate::tracker::TransientSnapshot,
        result: Result<Value, OmitReason>,
    ) -> MappedRow {
        let ref_url = schema_ref(schema, self.mode);
        match result {
            Ok(msg) => {
                let fid = snap.cmdr_fid.as_deref().unwrap_or("");
                let uid = uploader_id(&self.salt, fid);
                let gv = snap.gameversion.as_deref().unwrap_or("");
                let gb = snap.gamebuild.as_deref().unwrap_or("");
                let env = build_envelope(&ref_url, &uid, gv, gb, &msg);
                MappedRow {
                    kind: ref_url,
                    sub,
                    payload: env,
                    coalesce_key: None,
                    state: OutboxState::Pending,
                    class: None,
                }
            }
            Err(reason) => {
                if let OmitReason::LocalValidation(ref field) = reason {
                    tracing::warn!(target: "tmc_sync::eddn", schema, field, "validación local EDDN fallida; fila omitida");
                }
                MappedRow {
                    kind: ref_url,
                    sub,
                    payload: json!({}),
                    coalesce_key: None,
                    state: OutboxState::Omitted,
                    class: Some(ErrorClass::InvalidEvent),
                }
            }
        }
    }
}

impl ServiceMapper for EddnMapper {
    fn service(&self) -> SyncService {
        SyncService::Eddn
    }

    fn map(&mut self, input: &MappedInput<'_>, out: &mut Vec<MappedRow>) {
        if let Some(res) = journal::map_journal(input.ev, input.snap) {
            out.push(self.build_row("journal/1", 0, input.snap, res));
        }
    }
}
