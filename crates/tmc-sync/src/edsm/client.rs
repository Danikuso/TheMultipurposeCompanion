//! Cliente trabajador de EDSM (`E2-3`).
//!
//! Cumple la especificación §7.2:
//! - Lotes de hasta 50 eventos o 15 s de espera (`flush_after: 15 s`), por comandante (`per_cmdr: true`).
//! - Límite duro de 10 peticiones / 10 s (`RateLimiter::for_service(Service::Edsm)`) con intervalo
//!   objetivo mínimo de 5 s entre envíos (`min_interval: 5 s`).
//! - Falla cerrado: sin lista `/discard` cargada no envía ningún evento (`ErrorClass::DiscardListMissing`).
//! - Credencial inválida (`201..=203`) retiene en `pending` hasta que el piloto corrija la clave.
//! - Software bloqueado (`204 | 205`) detiene el servicio sin reintentos automáticos hasta reiniciar.
//! - Código `304` marca `discarded` y refresca la lista oficial `/discard`.

use super::discard::{DiscardList, DiscardStatus, refresh_discard_if_needed};
use crate::clock::Clock;
use crate::worker::{
    BatchPolicy, BoxFut, RowVerdict, SendCtx, SendOutcome, ServiceClient, ServiceEffect,
};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tmc_model::sync::{ErrorClass, SyncService};
use tmc_net::Transport;
use tmc_net::edsm_journal::{EDSM_JOURNAL_URL, EdsmBatch, EdsmVerdict, build_request, classify};
use tmc_net::limiter::{RateLimiter, Service};
use tmc_net::transport::HttpPost;
use tmc_store::{OutboxRow, Store};
use tokio::sync::Notify;

pub struct EdsmClient {
    transport: Arc<dyn Transport>,
    store: Store,
    clock: Arc<dyn Clock>,
    discard: Arc<DiscardList>,
    refresh_wake: Arc<Notify>,
    software_blocked: AtomicBool,
}

impl EdsmClient {
    pub fn new(
        transport: Arc<dyn Transport>,
        store: Store,
        clock: Arc<dyn Clock>,
        discard: Arc<DiscardList>,
        refresh_wake: Arc<Notify>,
    ) -> Self {
        Self {
            transport,
            store,
            clock,
            discard,
            refresh_wake,
            software_blocked: AtomicBool::new(false),
        }
    }
}

fn unwrap_payload(raw: &str) -> (Option<String>, Option<String>, Value) {
    let Ok(val) = serde_json::from_str::<Value>(raw) else {
        return (None, None, Value::Null);
    };
    if let Some(msg) = val.get("message") {
        let gv = val
            .get("fromGameVersion")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
        let gb = val
            .get("fromGameBuild")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
        return (gv, gb, msg.clone());
    }
    (None, None, val)
}

fn map_verdict(verdict: EdsmVerdict, need_discard_refresh: &mut bool) -> RowVerdict {
    match verdict {
        EdsmVerdict::Sent { code } => RowVerdict::Sent {
            code: code.to_string(),
        },
        EdsmVerdict::Rejected { code, class } => RowVerdict::Rejected {
            code: code.to_string(),
            class,
            hint: None,
        },
        EdsmVerdict::BenignRejected { code } => RowVerdict::Rejected {
            code: code.to_string(),
            class: ErrorClass::InvalidEvent,
            hint: None,
        },
        EdsmVerdict::Discarded { code } => {
            *need_discard_refresh = true;
            RowVerdict::Discarded {
                code: code.to_string(),
            }
        }
        EdsmVerdict::CredentialError { code } => RowVerdict::Hold {
            code: code.to_string(),
            class: ErrorClass::Auth,
        },
        EdsmVerdict::SoftwareBlocked { code } => RowVerdict::Hold {
            code: code.to_string(),
            class: ErrorClass::SoftwareBlocked,
        },
        EdsmVerdict::Retry { class, code, after } => RowVerdict::Retry { code, class, after },
    }
}

impl ServiceClient for EdsmClient {
    fn service(&self) -> SyncService {
        SyncService::Edsm
    }

    fn policy(&self) -> BatchPolicy {
        BatchPolicy {
            max_rows: 50,
            per_cmdr: true,
            min_interval: Duration::from_secs(5),
            limiter: RateLimiter::for_service(Service::Edsm),
            flush_after: Some(Duration::from_secs(15)),
        }
    }

    fn needs_credential(&self) -> bool {
        true
    }

    fn filter_claimed(&self, rows: Vec<OutboxRow>) -> (Vec<OutboxRow>, Vec<(i64, RowVerdict)>) {
        if self.software_blocked.load(Ordering::Acquire) {
            let verdicts = rows
                .into_iter()
                .map(|r| {
                    (
                        r.id,
                        RowVerdict::Hold {
                            code: "204".into(),
                            class: ErrorClass::SoftwareBlocked,
                        },
                    )
                })
                .collect();
            return (Vec::new(), verdicts);
        }

        if !self.discard.is_ready() {
            let verdicts = rows
                .into_iter()
                .map(|r| {
                    (
                        r.id,
                        RowVerdict::Hold {
                            code: "discard_list_missing".into(),
                            class: ErrorClass::DiscardListMissing,
                        },
                    )
                })
                .collect();
            return (Vec::new(), verdicts);
        }

        let mut to_send = Vec::with_capacity(rows.len());
        let mut verdicts = Vec::new();
        for row in rows {
            match self.discard.status(&row.kind) {
                DiscardStatus::Missing => verdicts.push((
                    row.id,
                    RowVerdict::Hold {
                        code: "discard_list_missing".into(),
                        class: ErrorClass::DiscardListMissing,
                    },
                )),
                DiscardStatus::Discarded => verdicts.push((
                    row.id,
                    RowVerdict::Discarded {
                        code: "discard_list".into(),
                    },
                )),
                DiscardStatus::Keep => to_send.push(row),
            }
        }
        (to_send, verdicts)
    }

    fn send<'a>(&'a self, ctx: SendCtx<'a>, rows: &'a [OutboxRow]) -> BoxFut<'a, SendOutcome> {
        Box::pin(async move {
            let Some(cred) = ctx.credential else {
                return SendOutcome {
                    verdicts: rows
                        .iter()
                        .map(|r| {
                            (
                                r.id,
                                RowVerdict::Hold {
                                    code: "202".into(),
                                    class: ErrorClass::Auth,
                                },
                            )
                        })
                        .collect(),
                    effect: ServiceEffect::CmdrError(ErrorClass::Auth),
                };
            };

            let mut game_version = String::new();
            let mut game_build = String::new();
            let mut messages = Vec::with_capacity(rows.len());
            for row in rows {
                let (gv, gb, msg) = unwrap_payload(&row.payload);
                if game_version.is_empty()
                    && let Some(v) = gv
                {
                    game_version = v;
                }
                if game_build.is_empty()
                    && let Some(b) = gb
                {
                    game_build = b;
                }
                messages.push(msg);
            }

            let req_json = build_request(&EdsmBatch {
                commander_name: &cred.cmdr_name,
                api_key: &cred.api_key,
                game_version: &game_version,
                game_build: &game_build,
                messages: &messages,
            });
            let Ok(body) = serde_json::to_vec(&req_json) else {
                return SendOutcome {
                    verdicts: rows
                        .iter()
                        .map(|r| {
                            (
                                r.id,
                                RowVerdict::Rejected {
                                    code: "encode".into(),
                                    class: ErrorClass::InvalidEvent,
                                    hint: None,
                                },
                            )
                        })
                        .collect(),
                    effect: ServiceEffect::None,
                };
            };

            let reply = self
                .transport
                .post(HttpPost {
                    url: EDSM_JOURNAL_URL,
                    body: &body,
                    content_type: "application/json",
                    headers: &[],
                    http1_only: false,
                })
                .await;

            let classified = classify(&reply, rows.len());
            let mut need_discard_refresh = false;
            let mut verdicts = Vec::with_capacity(rows.len());

            let effect = if let Some(global) = classified.global {
                let effect = match &global {
                    EdsmVerdict::Sent { .. }
                    | EdsmVerdict::Discarded { .. }
                    | EdsmVerdict::Rejected { .. }
                    | EdsmVerdict::BenignRejected { .. } => ServiceEffect::Connected,
                    EdsmVerdict::CredentialError { .. } => {
                        ServiceEffect::CmdrError(ErrorClass::Auth)
                    }
                    EdsmVerdict::SoftwareBlocked { .. } => {
                        self.software_blocked.store(true, Ordering::Release);
                        ServiceEffect::ServiceError {
                            class: ErrorClass::SoftwareBlocked,
                            until: None,
                        }
                    }
                    EdsmVerdict::Retry { class, after, .. } => {
                        let until = after
                            .and_then(|d| chrono::Duration::from_std(d).ok())
                            .map(|cd| self.clock.now_utc() + cd);
                        ServiceEffect::ServiceError {
                            class: *class,
                            until,
                        }
                    }
                };
                for row in rows {
                    verdicts.push((
                        row.id,
                        map_verdict(global.clone(), &mut need_discard_refresh),
                    ));
                }
                effect
            } else {
                let mut effect = ServiceEffect::Connected;
                for (row, ev_verdict) in rows.iter().zip(classified.per_event) {
                    match &ev_verdict {
                        EdsmVerdict::SoftwareBlocked { .. } => {
                            self.software_blocked.store(true, Ordering::Release);
                            effect = ServiceEffect::ServiceError {
                                class: ErrorClass::SoftwareBlocked,
                                until: None,
                            };
                        }
                        EdsmVerdict::CredentialError { .. }
                            if !matches!(effect, ServiceEffect::ServiceError { .. }) =>
                        {
                            effect = ServiceEffect::CmdrError(ErrorClass::Auth);
                        }
                        _ => {}
                    }
                    verdicts.push((row.id, map_verdict(ev_verdict, &mut need_discard_refresh)));
                }
                effect
            };

            if need_discard_refresh {
                self.refresh_wake.notify_one();
                let _ = refresh_discard_if_needed(
                    &self.store,
                    self.transport.as_ref(),
                    self.clock.as_ref(),
                    &self.discard,
                    true,
                )
                .await;
            }

            SendOutcome { verdicts, effect }
        })
    }
}
