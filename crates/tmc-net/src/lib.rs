//! Capa de red de TMC. Bloques puros y deterministas (reloj inyectado) más el
//! cliente asíncrono que los compone.

pub mod ardent;
pub mod bgs_tick;
pub mod cache_policy;
pub mod canonn;
pub mod client;
pub mod eddn_sender;
pub mod edsm;
pub mod edsm_sync;
pub mod health;
mod jsonx;
pub mod limiter;
pub mod spansh;
pub mod transport;

pub use cache_policy::{CacheKind, TtlPolicy, cache_keys, ttl_for};
pub use client::NetClient;
pub use health::ServiceHealth;
pub use limiter::{RateLimiter, Service};
pub use spansh::RoutePoll;
pub use transport::{BoxFut, HttpReply, ReqwestTransport, Transport, TransportError};
