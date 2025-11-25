/// Instance & Metadata System
///
/// Provides unique identification and metadata storage for all game entities.
/// Every item, block, entity can have a unique UUID with associated metadata.
/// Purely data-oriented - no instance "objects", just tables of data.

// Data structures
pub mod instance_data;
// Pure functions
pub mod instance_operations;

// Original modules (to be converted)
pub mod copy_on_write;
pub mod error;
pub mod history;
pub mod instance_id;
pub mod metadata_store;
pub mod network_sync;
pub mod query;

// Re-export data structures
pub use instance_data::*;
// Re-export operations
pub use instance_operations::*;

// Re-export from original modules (temporarily until full conversion)
pub use error::{timestamp_error, InstanceErrorContext, InstanceResult};
pub use instance_id::InstanceId;
pub use metadata_store::{MetadataKey, MetadataValue};

// Re-export types that are still using old pattern
pub use copy_on_write::{CowHandle, CowMetadata};
pub use history::{HistoryEntry, HistoryEvent, HistoryLog};
pub use instance_id::{InstanceIdGenerator};
pub use metadata_store::{MetadataStore};
pub use network_sync::{InstanceSync, SyncPacket, SyncState};
pub use query::{InstanceQuery, QueryFilter, QueryResult};

// Tests module
#[cfg(test)]
mod tests;