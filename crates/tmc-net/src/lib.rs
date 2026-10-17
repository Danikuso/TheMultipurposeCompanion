//! Capa de red de TMC. Bloques puros y deterministas (reloj inyectado).

pub mod cache_policy;
pub mod health;
pub mod limiter;

pub use cache_policy::{CacheKind, TtlPolicy, cache_keys, ttl_for};
pub use health::ServiceHealth;
pub use limiter::{RateLimiter, Service};
