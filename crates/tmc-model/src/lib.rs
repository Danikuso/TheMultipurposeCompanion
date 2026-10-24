//! Modelo de dominio de TMC.
#![forbid(unsafe_code)]

pub mod accounts;
pub mod carrier;
pub mod colonisation;
pub mod companion;
pub mod events;
pub mod market;
pub mod net;
pub mod powerplay;
pub mod settings;
pub mod source;
pub mod state;
pub mod status;
pub mod theme;
pub mod tts;

/// Error de análisis compartido por Status.json y los eventos del diario.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("el texto está vacío")]
    Empty,
    #[error("JSON inválido: {0}")]
    Json(String),
    #[error("falta el campo obligatorio {0}")]
    MissingField(&'static str),
}
#[cfg(debug_assertions)]
pub mod diagnostics;

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! diagnostic {
    ($context:expr, $code:literal, $payload:expr $(,)?) => {
        $crate::diagnostics::emit($context, $code, &$payload)
    };
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! diagnostic {
    ($context:expr, $code:literal, $payload:expr $(,)?) => {{}};
}
