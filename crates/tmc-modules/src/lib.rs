//! Un módulo por rol del juego; F0 incorpora el panel general.
#![forbid(unsafe_code)]

/// Registra una decisión interna del módulo (solo compilaciones debug; en release no evalúa nada).
macro_rules! decide {
    ($code:literal, { $($payload:tt)* }) => {
        decide!($code, tmc_model::diagnostics::json!({ $($payload)* }))
    };
    ($code:literal, $payload:expr) => {
        tmc_model::diagnostic!(
            tmc_model::diagnostics::DiagnosticContext::State,
            $code,
            $payload
        )
    };
}

/// Como `decide!`, pero solo si `$cond` es cierta; en release la condición se sigue evaluando
/// (barata y sin efectos secundarios) y no se emite nada.
#[cfg(debug_assertions)]
macro_rules! decide_if {
    ($cond:expr, $code:literal, $($payload:tt)*) => {
        if $cond {
            decide!($code, $($payload)*);
        }
    };
}
#[cfg(not(debug_assertions))]
macro_rules! decide_if {
    ($cond:expr, $code:literal, $($payload:tt)*) => {{
        let _ = $cond;
    }};
}

pub mod accounts;
pub mod bgs;
pub mod carrier;
pub mod colonisation;
pub mod combat;
pub mod crime;
pub mod dashboard;
pub mod data;
pub mod engineering;
pub mod eval;
pub mod exobiology;
pub mod exploration;
pub mod history;
pub mod inflight;
pub mod mining;
pub mod missions;
pub mod navigation;
pub mod powerplay;
pub mod registry;
pub mod ship;
pub(crate) mod topicx;
pub mod trade;
pub use dashboard::Dashboard;
pub use registry::all_modules;

#[cfg(all(test, debug_assertions))]
mod diag_tests;
