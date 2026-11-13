//! Sincronización con servicios externos (Inara, EDSM, EDDN) de F17.

pub mod clock;
pub mod housekeeping;
pub mod ids;
pub mod lease;
pub mod recovery;
pub mod router;
pub mod status;
pub mod tracker;
pub mod worker;

#[cfg(test)]
mod tests;

pub use clock::{Clock, SystemClock, TestClock};
pub use housekeeping::run_housekeeping;
pub use ids::{cmdr_ref, idem_key, key_hint, snapshot_idem_key};
pub use lease::{LEASE_RENEW_EVERY, LEASE_TTL_S, Lease};
pub use router::{
    EnvGates, LiveSignalPolicy, MappedInput, MappedRow, REPLAY_MARKER, Router, RouterConfig,
    SendGates, ServiceMapper,
};
pub use status::{EDDN_PAUSED_SCHEMAS_KEY, StatusAggregator};
pub use tracker::{TransientSnapshot, TransientTracker};
pub use worker::{
    BatchPolicy, MAX_WORKER_SLEEP, RowVerdict, SendCtx, SendOutcome, ServiceClient, ServiceEffect,
    Worker, WorkerEvent, backoff,
};
