//! Modelo de dominio de TMC.
#![forbid(unsafe_code)]

pub mod companion;
pub mod events;
pub mod source;
pub mod state;
pub mod status;
pub mod theme;

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
