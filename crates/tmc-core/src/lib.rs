//! Actor de estado y contrato de módulos de TMC.

pub mod actor;
pub mod alert;
#[cfg(debug_assertions)]
pub mod debug_hooks;
pub mod module;
pub mod topics;

pub use actor::{CoreHandle, CoreMessage, CoreOptions, spawn_core, spawn_core_with};
pub use alert::{Alert, Severity};
pub use module::{Module, ModuleCtx, ModulePageQuery, ModuleQueryError, Relevance, StoreTask};
pub use tmc_store::{
    CachedEntry, EventQuery, EventRow, EventWriter, Migration, Store, StoreError, StoredEvent,
};
pub use topics::TopicSet;
