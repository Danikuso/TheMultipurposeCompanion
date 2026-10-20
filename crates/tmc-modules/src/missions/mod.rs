//! Módulo `missions`: libro de misiones activas y su reconstrucción.
//!
//! El evento `Missions` manda sobre la lista activa; los detalles de cada misión
//! salen de `MissionAccepted` (en vivo o desde el historial). Las bajas de las
//! masacres no se cuentan aquí (las lleva `MassacreStack`). Las escrituras al
//! almacén llevan la etiqueta `*.write` y no ocurren en replay.

mod book;
mod massacre;

pub use book::{
    BookChange, Mission, MissionBook, MissionKind, MissionProgress, accepted_from_result,
    accepted_rebuild_task, classify_kind,
};
pub use massacre::{IssuerBar, MASSACRE_COUNTS_KILL_BONDS, MassacrePile, MassacreStack};
