//! Persistence Module - Simplified for DOP conversion

// Data modules
pub mod atomic_save_data;
pub mod backup_data;
pub mod chunk_serializer_data;
pub mod compression_data;
pub mod metadata_data;
pub mod migration_data;
pub mod network_validator_data;
pub mod state_validator_data;
pub mod world_save_data;

// Operations modules
pub mod atomic_save_operations;
pub mod backup_operations;
pub mod chunk_serializer_operations;
pub mod compression_operations;
pub mod metadata_operations;
pub mod migration_operations;
pub mod network_validator_operations;
pub mod state_validator_operations;
pub mod world_save_operations;

// Simple re-exports
pub use atomic_save_data::AtomicSaveData;
pub use backup_data::BackupData;
pub use chunk_serializer_data::ChunkSerializerData;
pub use compression_data::CompressionData;
pub use metadata_data::MetadataData;
pub use migration_data::MigrationData;
pub use network_validator_data::NetworkValidatorData;
pub use state_validator_data::StateValidatorData;
pub use world_save_data::WorldSaveData;

// Error types (stubs)
pub type PersistenceResult<T> = Result<T, PersistenceError>;

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("Save failed: {0}")]
    SaveFailed(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    #[error("Compression error: {0}")]
    CompressionError(String),
    #[error("Version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: String, found: String },
    #[error("Corrupted data: {0}")]
    CorruptedData(String),
    #[error("Migration error: {0}")]
    MigrationError(String),
    #[error("Backup error: {0}")]
    BackupError(String),
    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),
    #[error("Player not found: {0}")]
    PlayerNotFound(String),
    #[error("Capacity exceeded: {0}")]
    CapacityExceeded(String),
}

// Stub types for compatibility
pub enum SaveOperation {
    Save,
    Load,
    Player { player_id: u32, uuid: String, position: [f32; 3], priority: SavePriority },
    ChunkBatch { chunks: Vec<u64>, positions: Vec<(i32, i32, i32)>, priority: SavePriority },
}
#[derive(Clone, Copy, Debug)]
pub enum SavePriority { Critical, High, Normal, Low }

// Stub function for compatibility
pub fn queue_operation<T>(_data: &T, _op: SaveOperation) -> PersistenceResult<()> {
    Ok(())
}
