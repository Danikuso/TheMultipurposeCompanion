//! Mapeador y cliente de sincronización para EDSM (`api-journal-v1`).

pub mod client;
pub mod discard;
pub mod mapper;

pub use client::EdsmClient;
pub use discard::{
    DiscardList, DiscardStatus, EDSM_DISCARD_CACHE_KEY, refresh_discard_if_needed,
    run_discard_refresh,
};
pub use mapper::{EdsmMapper, TMC_PRIVACY_LIST, build_message};
