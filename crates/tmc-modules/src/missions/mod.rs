//! Módulo `missions`: libro de misiones activas y su reconstrucción.
//!
//! El evento `Missions` manda sobre la lista activa; los detalles de cada misión
//! salen de `MissionAccepted` (en vivo o desde el historial). Las bajas de las
//! masacres no se cuentan aquí (las lleva `MassacreStack`). Las escrituras al
//! almacén llevan la etiqueta `*.write` y no ocurren en replay.

mod alerts;
mod book;
mod massacre;
mod passengers;
mod rewards;
mod tourism;

pub use alerts::{AlertKind, Here, MissionAlert, alert_from_change, evaluate_alerts};
pub use book::{
    BookChange, Mission, MissionBook, MissionKind, MissionProgress, accepted_from_result,
    accepted_rebuild_task, classify_kind,
};
pub use massacre::{IssuerBar, MASSACRE_COUNTS_KILL_BONDS, MassacrePile, MassacreStack};
pub use passengers::{CabinOccupancy, Cabins, PassengerGroup, groups, wanted_aboard};
pub use rewards::{CgView, CommunityGoals, FactionDelta, SessionRewards};
pub use tourism::{
    EXACT_MAX_STOPS, PASSENGER_STATS_MIGRATION, PlanInput, TAG_PASSENGER_STATS,
    TAG_STATS_COMPLETE_WRITE, TAG_STATS_WRITE, TourismPlan, Verdict, own_reference,
    plan as tourism_plan, rows_from_result, stats_complete_task, stats_read_task, stats_write_task,
};
