//! Decodificación y filtro de mensajes EDDN (lógica pura, sin red).
#![forbid(unsafe_code)]

pub mod coords;
pub mod decode;
pub mod filter;
pub mod subscriber;
pub mod transport;

pub use coords::{COORD_INDEX_CAP, CoordIndex};
pub use decode::{
    CARRIER_SET_CAP, COMMODITY_SCHEMA, CarrierSet, Decoded, DropReason, JOURNAL_SCHEMA,
    MAX_INFLATED_BYTES, decode, decode_with,
};
pub use filter::{EddnFilter, EddnStats, accept};
pub use subscriber::{
    BATCH_MAX_AGE, BATCH_MAX_MARKETS, Clock, CoordEntry, EddnOutput, EddnStatus, FakeClock,
    PENDING_CAP, SystemClock, spawn_subscriber,
};
pub use transport::{
    EddnError, EddnMode, FrameSource, MAX_COMPRESSED_BYTES, MAX_PARTS, RELAY_URL, ZmqSource,
    frame_from_parts,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod subscriber_tests;
