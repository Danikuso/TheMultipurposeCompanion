//! Un módulo por rol del juego; F0 incorpora el panel general.
#![forbid(unsafe_code)]

pub mod combat;
pub mod dashboard;
pub mod data;
pub mod eval;
pub mod exobiology;
pub mod exploration;
pub mod history;
pub mod inflight;
pub mod mining;
pub mod navigation;
pub mod registry;
pub use dashboard::Dashboard;
pub use registry::all_modules;
