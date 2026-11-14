//! Lista oficial de eventos descartados por EDSM (`/api-journal-v1/discard`) y caché 24 h.

use crate::clock::Clock;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tmc_net::Transport;
use tmc_net::edsm_journal::{EDSM_DISCARD_URL, parse_discard};
use tmc_store::Store;
use tokio::sync::{Notify, watch};

pub const EDSM_DISCARD_CACHE_KEY: &str = "edsm.discard";
pub const DISCARD_TTL_HOURS: i64 = 24;
const RETRY_MIN: Duration = Duration::from_secs(60);
const RETRY_MAX: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscardStatus {
    Missing,
    Discarded,
    Keep,
}

#[derive(Default)]
pub struct DiscardList {
    entries: RwLock<Option<HashSet<String>>>,
}

impl DiscardList {
    pub fn empty() -> Arc<Self> {
        Arc::new(Self {
            entries: RwLock::new(None),
        })
    }

    pub fn with_set(entries: HashSet<String>) -> Arc<Self> {
        Arc::new(Self {
            entries: RwLock::new(Some(entries)),
        })
    }

    pub fn from_store(store: &Store) -> Arc<Self> {
        let initial = match store.api_cache_get(EDSM_DISCARD_CACHE_KEY) {
            Ok(Some(entry)) => parse_discard(&entry.value).ok(),
            _ => None,
        };
        Arc::new(Self {
            entries: RwLock::new(initial),
        })
    }

    pub fn set(&self, entries: HashSet<String>) {
        let mut guard = self.entries.write().expect("discard mutex poisoned");
        *guard = Some(entries);
    }

    pub fn is_ready(&self) -> bool {
        self.entries
            .read()
            .expect("discard mutex poisoned")
            .is_some()
    }

    pub fn status(&self, event_name: &str) -> DiscardStatus {
        let guard = self.entries.read().expect("discard mutex poisoned");
        match guard.as_ref() {
            None => DiscardStatus::Missing,
            Some(set) if set.contains(event_name) => DiscardStatus::Discarded,
            Some(_) => DiscardStatus::Keep,
        }
    }
}

fn cache_fresh(store: &Store, clock: &dyn Clock) -> Option<HashSet<String>> {
    let entry = store.api_cache_get(EDSM_DISCARD_CACHE_KEY).ok().flatten()?;
    if clock.now_utc() - entry.fetched_at >= chrono::Duration::hours(DISCARD_TTL_HOURS) {
        return None;
    }
    parse_discard(&entry.value).ok()
}

fn next_ttl_delay(store: &Store, clock: &dyn Clock) -> Option<Duration> {
    let entry = store.api_cache_get(EDSM_DISCARD_CACHE_KEY).ok().flatten()?;
    let ttl = chrono::Duration::hours(DISCARD_TTL_HOURS);
    let age = clock.now_utc() - entry.fetched_at;
    if age < ttl {
        (ttl - age).to_std().ok()
    } else {
        None
    }
}

/// Descarga y actualiza la lista `/discard` salvo que siga vigente (< 24 h) y no se fuerce.
pub async fn refresh_discard_if_needed(
    store: &Store,
    transport: &dyn Transport,
    clock: &dyn Clock,
    list: &DiscardList,
    force: bool,
) -> bool {
    if !force && let Some(cached) = cache_fresh(store, clock) {
        if !list.is_ready() {
            list.set(cached);
        }
        return false;
    }

    let Ok(reply) = transport.get(EDSM_DISCARD_URL).await else {
        return false;
    };
    if !(200..300).contains(&reply.status) {
        return false;
    }
    let Ok(set) = parse_discard(&reply.body) else {
        return false;
    };
    let _ = store.api_cache_put(EDSM_DISCARD_CACHE_KEY, &reply.body, clock.now_utc());
    list.set(set);
    true
}

#[allow(clippy::too_many_arguments)]
pub async fn run_discard_refresh(
    store: Store,
    transport: Arc<dyn Transport>,
    clock: Arc<dyn Clock>,
    list: Arc<DiscardList>,
    online: Arc<AtomicBool>,
    refresh_wake: Arc<Notify>,
    worker_wake: Arc<Notify>,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut force_next = false;
    let mut retry_delay = RETRY_MIN;

    loop {
        if *shutdown.borrow() {
            break;
        }

        let updated = if online.load(Ordering::Acquire) {
            refresh_discard_if_needed(
                &store,
                transport.as_ref(),
                clock.as_ref(),
                &list,
                force_next,
            )
            .await
        } else {
            false
        };
        force_next = false;

        if updated {
            retry_delay = RETRY_MIN;
            worker_wake.notify_one();
        }

        let wait = next_ttl_delay(&store, clock.as_ref()).unwrap_or_else(|| {
            let d = retry_delay;
            retry_delay = (retry_delay * 2).min(RETRY_MAX);
            d
        });

        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            _ = refresh_wake.notified() => {
                force_next = true;
            }
            _ = tokio::time::sleep(wait) => {}
        }
    }
}
