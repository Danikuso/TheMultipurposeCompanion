//! Un módulo por rol del juego; F0 incorpora el panel general.
#![forbid(unsafe_code)]

pub mod dashboard;
pub mod data;
pub mod exploration;
pub mod navigation;
pub mod registry;
pub use dashboard::Dashboard;
pub use registry::all_modules;
