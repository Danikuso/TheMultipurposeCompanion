//! Cliente trabajador de EDDN (`E1-5`).
//!
//! Cumple `C6`, `C7` y `C11`:
//! - 1 mensaje/POST (`max_rows: 1`) y máximo 2 POST/s (`RateLimiter::for_service(Service::Eddn)` +
//!   `min_interval: 500 ms`).
//! - Backoff exponencial >= 60 s hasta 30 min (`backoff(SyncService::Eddn, attempts)`).
//! - HTTP 400 rechaza sin bloquear el resto de la cola (`RowVerdict::Rejected`).
//! - HTTP 426 pausa solo ese `$schemaRef` en `sync.eddn.paused_schemas` (limpiándose al cambiar
//!   `CARGO_PKG_VERSION`), deja las filas afectadas `pending` sin consumir intentos y permite que
//!   otros esquemas sigan enviándose sin inanición.

use crate::status::EDDN_PAUSED_SCHEMAS_KEY;
use crate::worker::{
    BatchPolicy, BoxFut, RowVerdict, SendCtx, SendOutcome, ServiceClient, ServiceEffect, backoff,
};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tmc_model::sync::{ErrorClass, SyncService};
use tmc_net::Transport;
use tmc_net::eddn_upload::{
    EddnLocalError, EddnVerdict, SchemaMode, classify, encode_body, schema_ref, upload,
};
use tmc_net::limiter::{RateLimiter, Service};
use tmc_store::{OutboxRow, Store};

const EDDN_PAUSED_VERSION_KEY: &str = "sync.eddn.paused_version";

pub struct EddnClient {
    transport: Arc<dyn Transport>,
    store: Store,
    paused: Mutex<BTreeSet<String>>,
}

impl EddnClient {
    pub fn new(transport: Arc<dyn Transport>, store: Store) -> Self {
        let current_ver = env!("CARGO_PKG_VERSION");
        let saved_ver = store.kv_get(EDDN_PAUSED_VERSION_KEY);
        let paused = if saved_ver.as_deref() == Some(current_ver) {
            load_paused_schemas(&store)
        } else {
            store.kv_set(EDDN_PAUSED_SCHEMAS_KEY, "[]");
            store.kv_set(EDDN_PAUSED_VERSION_KEY, current_ver);
            BTreeSet::new()
        };

        Self {
            transport,
            store,
            paused: Mutex::new(paused),
        }
    }

    fn pause_schema(&self, kind: &str) -> String {
        let short = short_schema(kind).to_string();
        let mut guard = self.paused.lock().expect("paused mutex poisoned");
        if guard.insert(short.clone()) {
            let list: Vec<String> = guard.iter().cloned().collect();
            if let Ok(json) = serde_json::to_string(&list) {
                self.store.kv_set(EDDN_PAUSED_SCHEMAS_KEY, &json);
            }
            self.store
                .kv_set(EDDN_PAUSED_VERSION_KEY, env!("CARGO_PKG_VERSION"));
        }
        short
    }
}

impl ServiceClient for EddnClient {
    fn service(&self) -> SyncService {
        SyncService::Eddn
    }

    fn policy(&self) -> BatchPolicy {
        BatchPolicy {
            max_rows: 1,
            per_cmdr: false,
            min_interval: Duration::from_millis(500),
            limiter: RateLimiter::for_service(Service::Eddn),
            flush_after: None,
        }
    }

    fn needs_credential(&self) -> bool {
        false
    }

    fn excluded_kinds(&self) -> Vec<String> {
        let guard = self.paused.lock().expect("paused mutex poisoned");
        let mut out = Vec::with_capacity(guard.len() * 2);
        for s in guard.iter() {
            out.push(schema_ref(s, SchemaMode::Live));
            out.push(schema_ref(s, SchemaMode::Test));
        }
        out
    }

    fn filter_claimed(&self, rows: Vec<OutboxRow>) -> (Vec<OutboxRow>, Vec<(i64, RowVerdict)>) {
        let guard = self.paused.lock().expect("paused mutex poisoned");
        if guard.is_empty() {
            return (rows, Vec::new());
        }
        let kept = rows
            .into_iter()
            .filter(|row| !guard.contains(short_schema(&row.kind)))
            .collect();
        (kept, Vec::new())
    }

    fn send<'a>(&'a self, ctx: SendCtx<'a>, rows: &'a [OutboxRow]) -> BoxFut<'a, SendOutcome> {
        Box::pin(async move {
            let Some(row) = rows.first() else {
                return SendOutcome {
                    verdicts: Vec::new(),
                    effect: ServiceEffect::None,
                };
            };

            let envelope: serde_json::Value = match serde_json::from_str(&row.payload) {
                Ok(v) => v,
                Err(e) => {
                    return SendOutcome {
                        verdicts: vec![(
                            row.id,
                            RowVerdict::Rejected {
                                code: "eddn_invalid_payload".into(),
                                class: ErrorClass::InvalidEvent,
                                hint: Some(e.to_string()),
                            },
                        )],
                        effect: ServiceEffect::None,
                    };
                }
            };

            let body = match encode_body(&envelope) {
                Ok(b) => b,
                Err(EddnLocalError::Serialize(msg)) => {
                    return SendOutcome {
                        verdicts: vec![(
                            row.id,
                            RowVerdict::Rejected {
                                code: "eddn_serialize".into(),
                                class: ErrorClass::InvalidEvent,
                                hint: Some(msg),
                            },
                        )],
                        effect: ServiceEffect::None,
                    };
                }
                Err(EddnLocalError::TooLarge(bytes)) => {
                    return SendOutcome {
                        verdicts: vec![(
                            row.id,
                            RowVerdict::Rejected {
                                code: "eddn_too_large".into(),
                                class: ErrorClass::TooLarge,
                                hint: Some(format!("{bytes} bytes")),
                            },
                        )],
                        effect: ServiceEffect::None,
                    };
                }
            };

            let reply = upload(&*self.transport, &body).await;
            match classify(&reply) {
                EddnVerdict::Sent => SendOutcome {
                    verdicts: vec![(row.id, RowVerdict::Sent { code: "200".into() })],
                    effect: ServiceEffect::Connected,
                },
                EddnVerdict::Rejected { class, code, hint } => SendOutcome {
                    verdicts: vec![(row.id, RowVerdict::Rejected { code, class, hint })],
                    effect: ServiceEffect::Connected,
                },
                EddnVerdict::SchemaOutdated { code } => {
                    let schema = self.pause_schema(&row.kind);
                    tracing::warn!(
                        target: "tmc_sync::eddn",
                        schema = %schema,
                        code = %code,
                        "esquema EDDN obsoleto (HTTP 426); pausado hasta actualización de la aplicación"
                    );
                    // Sin veredicto sobre la fila: Worker::process_batch la devuelve a `pending`
                    // sin consumir `attempts` ni bloquear el resto de esquemas.
                    SendOutcome {
                        verdicts: Vec::new(),
                        effect: ServiceEffect::PauseSchema(schema),
                    }
                }
                EddnVerdict::Retry { class, code } => {
                    let wait = backoff(SyncService::Eddn, row.attempts + 1);
                    let wait_chrono = chrono::Duration::from_std(wait)
                        .unwrap_or_else(|_| chrono::Duration::seconds(1800));
                    SendOutcome {
                        verdicts: vec![(
                            row.id,
                            RowVerdict::Retry {
                                code,
                                class,
                                after: None,
                            },
                        )],
                        effect: ServiceEffect::ServiceError {
                            class,
                            until: Some(ctx.now + wait_chrono),
                        },
                    }
                }
            }
        })
    }
}

fn load_paused_schemas(store: &Store) -> BTreeSet<String> {
    let Some(raw) = store.kv_get(EDDN_PAUSED_SCHEMAS_KEY) else {
        return BTreeSet::new();
    };
    serde_json::from_str::<Vec<String>>(&raw)
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn short_schema(kind: &str) -> &str {
    kind.trim_start_matches("https://eddn.edcd.io/schemas/")
        .trim_end_matches('#')
        .trim_end_matches("/test")
}
