//! Decodificación y filtro de mensajes EDDN (lógica pura, sin red).
#![forbid(unsafe_code)]

pub mod coords;
pub mod decode;
pub mod filter;

pub use coords::{COORD_INDEX_CAP, CoordIndex};
pub use decode::{
    COMMODITY_SCHEMA, CarrierSet, Decoded, DropReason, JOURNAL_SCHEMA, MAX_INFLATED_BYTES, decode,
    decode_with,
};
pub use filter::{EddnFilter, EddnStats, accept};

#[cfg(test)]
mod tests;
