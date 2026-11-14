//! Lease periódica de instancia emisora única (C3c, §5.2).

use crate::clock::Clock;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tmc_store::Store;
use tokio::sync::{Notify, watch};
use uuid::Uuid;

pub const LEASE_RENEW_EVERY: Duration = Duration::from_secs(15);
pub const LEASE_TTL_S: i64 = 45;

/// Titular local del lease SQLite y estado compartido con router/trabajadores.
pub struct Lease {
    pub owner: String,
    pub held: Arc<AtomicBool>,
}

impl Lease {
    /// Crea un titular con identificador único `<uuid>:<kind>` (`desktop` o `headless`).
    pub fn new(kind: &str) -> Self {
        Self {
            owner: format!("{}:{kind}", Uuid::new_v4()),
            held: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Ejecuta un intento síncrono de adquisición/renovación (falla cerrado en error).
    pub fn try_step(&self, store: &Store, clock: &dyn Clock) -> bool {
        let acquired = store
            .try_acquire_lease(
                &self.owner,
                clock.now_utc(),
                chrono::Duration::seconds(LEASE_TTL_S),
            )
            .unwrap_or_else(|error| {
                tracing::warn!(
                    failure_class = error.diagnostic_class(),
                    "sync lease: acquire failed"
                );
                false
            });
        let prev = self.held.swap(acquired, Ordering::SeqCst);
        if prev != acquired {
            tracing::info!(held = acquired, "sync lease: state changed");
        }
        acquired
    }

    /// Bucle de renovación cada `LEASE_RENEW_EVERY` hasta el cierre ordenado.
    pub async fn run(
        self,
        store: Store,
        clock: Arc<dyn Clock>,
        mut shutdown: watch::Receiver<bool>,
        on_change: Arc<Notify>,
    ) {
        let mut first = true;
        loop {
            let prev = self.held.load(Ordering::SeqCst);
            let now_held = self.try_step(&store, clock.as_ref());
            if first || prev != now_held {
                first = false;
                on_change.notify_waiters();
            }
            if *shutdown.borrow() {
                break;
            }
            tokio::select! {
                _ = tokio::time::sleep(LEASE_RENEW_EVERY) => {}
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
            }
        }
        self.held.store(false, Ordering::SeqCst);
        let _ = store.release_lease(&self.owner);
        on_change.notify_waiters();
    }
}
