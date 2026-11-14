//! Mantenimiento periódico de la bandeja de sincronización: caducidad, tope y purga (C9).

use crate::clock::Clock;
use std::sync::Arc;
use std::time::Duration;
use tmc_model::sync::SyncService;
use tmc_store::{SYNC_PENDING_CAP, Store};
use tokio::sync::watch;

const HOUSEKEEPING_EVERY: Duration = Duration::from_secs(86_400);

/// Ejecuta una pasada completa de caducidad, tope de pendientes y purga de cerradas.
pub fn run_once(store: &Store, clock: &dyn Clock) {
    let now = clock.now_utc();
    for service in SyncService::ALL {
        if let Err(error) = store.expire_old(service, now) {
            tracing::warn!(
                failure_class = error.diagnostic_class(),
                "sync housekeeping: expire_old failed"
            );
        }
        if let Err(error) = store.enforce_pending_cap(service, SYNC_PENDING_CAP, now) {
            tracing::warn!(
                failure_class = error.diagnostic_class(),
                "sync housekeeping: pending cap failed"
            );
        }
    }
    if let Err(error) = store.purge_done(now) {
        tracing::warn!(
            failure_class = error.diagnostic_class(),
            "sync housekeeping: purge_done failed"
        );
    }
}

/// Mantenimiento al arrancar y cada 24 h hasta el cierre ordenado.
pub async fn run_housekeeping(
    store: Store,
    clock: Arc<dyn Clock>,
    mut shutdown: watch::Receiver<bool>,
) {
    loop {
        if *shutdown.borrow() {
            break;
        }
        run_once(&store, clock.as_ref());
        tokio::select! {
            _ = tokio::time::sleep(HOUSEKEEPING_EVERY) => {}
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
        }
    }
}
