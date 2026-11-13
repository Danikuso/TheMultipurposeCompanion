//! Sincronización con servicios externos (Inara, EDSM, EDDN) de F17.

pub mod clock;
pub mod ids;
pub mod recovery;
pub mod router;
pub mod tracker;

#[cfg(test)]
mod tests;

pub use clock::{Clock, SystemClock, TestClock};
pub use ids::{cmdr_ref, idem_key, key_hint, snapshot_idem_key};
pub use router::{
    EnvGates, LiveSignalPolicy, MappedInput, MappedRow, REPLAY_MARKER, Router, RouterConfig,
    SendGates, ServiceMapper,
};
pub use tracker::{TransientSnapshot, TransientTracker};
