//! Actor de estado y contrato de módulos de TMC.

pub mod actor;
pub mod alert;
pub mod module;
pub mod topics;

pub use actor::{CoreHandle, CoreMessage, spawn_core};
pub use alert::{Alert, Severity};
pub use module::{Module, ModuleCtx, Relevance};
pub use tmc_store::{EventQuery, EventRow, EventWriter, Store, StoreError, StoredEvent};
pub use topics::TopicSet;
