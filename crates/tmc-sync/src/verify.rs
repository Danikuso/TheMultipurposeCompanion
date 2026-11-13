//! Verificación de claves API por servicio y reintento con retroceso exponencial (DR-22).

use crate::clock::Clock;
use crate::worker::backoff;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tmc_model::sync::{KeyAssignment, SyncService};
use tmc_net::edsm_journal::{classify_get_ranks, get_ranks_url};
use tmc_net::inara::{INARA_API_URL, build_profile_request, classify_profile};
use tmc_net::limiter::SharedRateLimiter;
use tmc_net::transport::HttpPost;
use tmc_net::{HttpReplyEx, KeyCheck, Transport};
use tmc_store::Store;
use tokio::sync::{Notify, mpsc, watch};

/// Actualización de estado emitida por la verificación de claves hacia el agregador.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyUpdate {
    pub service: SyncService,
    pub fid: String,
    pub check: Option<KeyCheck>,
    pub next_retry: Option<DateTime<Utc>>,
}

/// Verifica una clave personal contra el endpoint oficial del servicio respetando limitadores compartidos.
pub async fn check_service_key_limited(
    transport: &dyn Transport,
    service: SyncService,
    cmdr_name: &str,
    cmdr_fid: Option<&str>,
    api_key: &str,
    inara_limiter: Option<&SharedRateLimiter>,
) -> KeyCheck {
    match service {
        SyncService::Edsm => {
            let url = get_ranks_url(cmdr_name, api_key);
            let reply = transport.get(&url).await.map(|r| HttpReplyEx {
                status: r.status,
                body: r.body,
                retry_after_s: None,
                rate_limit_remaining: None,
                rate_limit_reset_s: None,
            });
            classify_get_ranks(&reply)
        }
        SyncService::Inara => {
            if let Some(limiter) = inara_limiter {
                loop {
                    let now_lim = tokio::time::Instant::now().into_std();
                    match limiter.try_acquire(now_lim) {
                        Ok(()) => break,
                        Err(wait) => tokio::time::sleep(wait).await,
                    }
                }
            }
            let req = build_profile_request(api_key, cmdr_name, cmdr_fid);
            let Ok(body) = serde_json::to_vec(&req) else {
                return KeyCheck::Unreachable;
            };
            let reply = transport
                .post(HttpPost {
                    url: INARA_API_URL,
                    body: &body,
                    content_type: "application/json",
                    headers: &[],
                    http1_only: false,
                })
                .await;
            classify_profile(&reply)
        }
        SyncService::Eddn => KeyCheck::Valid,
    }
}

/// Verifica una clave personal contra el endpoint oficial del servicio sin enviar datos de diario.
pub async fn check_service_key(
    transport: &dyn Transport,
    service: SyncService,
    cmdr_name: &str,
    api_key: &str,
) -> KeyCheck {
    check_service_key_limited(transport, service, cmdr_name, None, api_key, None).await
}

struct RetryEntry {
    api_key: String,
    attempts: u32,
    next_at: DateTime<Utc>,
    exhausted_rejected: bool,
}

/// Reintenta en segundo plano claves guardadas sin verificar (`verified_at = None`, DR-22).
#[allow(clippy::too_many_arguments)]
pub async fn run_key_verifier(
    store: Store,
    transport: Arc<dyn Transport>,
    clock: Arc<dyn Clock>,
    online: Arc<AtomicBool>,
    verify_wake: Arc<Notify>,
    worker_wakers: Vec<Arc<Notify>>,
    status_wake: Arc<Notify>,
    verify_tx: mpsc::Sender<VerifyUpdate>,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut retries: HashMap<(SyncService, String), RetryEntry> = HashMap::new();

    loop {
        if *shutdown.borrow() {
            break;
        }

        let now = clock.now_utc();
        let mut active_keys: Vec<(SyncService, tmc_store::SyncCredential)> = Vec::new();
        if let Ok(cmdrs) = store.known_cmdrs() {
            for (fid, _) in cmdrs {
                let service = SyncService::Edsm;
                if let Ok(Some(cred)) = store.sync_credential(service, &fid)
                    && cred.verified_at.is_none()
                    && cred.assignment == KeyAssignment::Confirmed
                {
                    active_keys.push((service, cred));
                }
            }
        }

        // Purga entradas cuya clave haya desaparecido o haya cambiado.
        retries.retain(|(svc, fid), entry| {
            active_keys
                .iter()
                .any(|(s, c)| s == svc && &c.cmdr_fid == fid && c.api_key == entry.api_key)
        });

        let mut next_deadline: Option<DateTime<Utc>> = None;

        for (service, mut cred) in active_keys {
            if *shutdown.borrow() {
                break;
            }
            let key = (service, cred.cmdr_fid.clone());
            let entry = retries.entry(key).or_insert_with(|| RetryEntry {
                api_key: cred.api_key.clone(),
                attempts: 1,
                next_at: now + backoff_cd(service, 1),
                exhausted_rejected: false,
            });
            if entry.exhausted_rejected {
                continue;
            }

            if !online.load(Ordering::Acquire) {
                continue;
            }

            if now < entry.next_at {
                next_deadline = Some(match next_deadline {
                    Some(prev) => prev.min(entry.next_at),
                    None => entry.next_at,
                });
                continue;
            }

            match check_service_key(transport.as_ref(), service, &cred.cmdr_name, &cred.api_key)
                .await
            {
                KeyCheck::Valid => {
                    let verified_now = clock.now_utc();
                    cred.verified_at =
                        Some(verified_now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
                    let _ = store.put_sync_credential(&cred);
                    if store
                        .activation(service, &cred.cmdr_fid)
                        .ok()
                        .flatten()
                        .is_none()
                    {
                        let _ = store.set_activation(service, &cred.cmdr_fid, verified_now);
                    }
                    let _ = verify_tx.try_send(VerifyUpdate {
                        service,
                        fid: cred.cmdr_fid.clone(),
                        check: Some(KeyCheck::Valid),
                        next_retry: None,
                    });
                    for w in &worker_wakers {
                        w.notify_one();
                    }
                    status_wake.notify_one();
                    retries.remove(&(service, cred.cmdr_fid));
                }
                KeyCheck::Rejected => {
                    entry.exhausted_rejected = true;
                    let _ = verify_tx.try_send(VerifyUpdate {
                        service,
                        fid: cred.cmdr_fid.clone(),
                        check: Some(KeyCheck::Rejected),
                        next_retry: None,
                    });
                    status_wake.notify_one();
                }
                KeyCheck::Unreachable => {
                    entry.attempts = entry.attempts.saturating_add(1);
                    let retry_at = clock.now_utc() + backoff_cd(service, entry.attempts);
                    entry.next_at = retry_at;
                    next_deadline = Some(match next_deadline {
                        Some(prev) => prev.min(retry_at),
                        None => retry_at,
                    });
                    let _ = verify_tx.try_send(VerifyUpdate {
                        service,
                        fid: cred.cmdr_fid.clone(),
                        check: Some(KeyCheck::Unreachable),
                        next_retry: Some(retry_at),
                    });
                    status_wake.notify_one();
                }
            }
        }

        let wait_dur = next_deadline
            .and_then(|d| (d - clock.now_utc()).to_std().ok())
            .map(|d| d.max(Duration::from_millis(50)));

        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            _ = verify_wake.notified() => {}
            _ = sleep_opt(wait_dur) => {}
        }
    }
}

fn backoff_cd(service: SyncService, attempts: u32) -> chrono::Duration {
    chrono::Duration::from_std(backoff(service, attempts))
        .unwrap_or_else(|_| chrono::Duration::minutes(30))
}

async fn sleep_opt(dur: Option<Duration>) {
    match dur {
        Some(d) => tokio::time::sleep(d).await,
        None => std::future::pending().await,
    }
}
