//! Capa de red de TMC. Bloques puros y deterministas (reloj inyectado) más el
//! cliente asíncrono que los compone.

pub mod ardent;
pub mod bgs_tick;
pub mod cache_policy;
pub mod canonn;
pub mod capi;
pub mod client;
pub mod eddn_sender;
pub mod edsm;
pub mod edsm_sync;
pub mod health;
pub mod inara;
mod jsonx;
pub mod limiter;
pub mod spansh;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod transport;

pub use cache_policy::{CacheKind, TtlPolicy, cache_keys, ttl_for};
pub use client::NetClient;
pub use health::ServiceHealth;
pub use limiter::{RateLimiter, Service};
pub use spansh::RoutePoll;
pub use transport::{
    BoxFut, HttpPost, HttpReply, HttpReplyEx, ReqwestTransport, Transport, TransportError,
};
