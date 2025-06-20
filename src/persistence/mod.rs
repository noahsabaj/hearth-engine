//! Persistence system for saving and loading game data

pub mod chunk_serializer;
pub mod world_save;
pub mod player_data_dop;
pub mod compression;
pub mod metadata;
pub mod migration;
pub mod backup;
pub mod error;
pub mod atomic_save;
pub mod state_validator;
pub mod network_validator;

pub use chunk_serializer::{ChunkSerializer, ChunkFormat};
pub use world_save::{WorldSave, WorldSaveError};
pub use player_data_dop::{PlayerDataBuffer, PlayerHotData, PlayerColdData, PlayerBufferMemoryStats, CACHE_LINE_SIZE, MAX_PLAYERS};
pub use compression::{CompressionType, CompressionLevel, Compressor};
pub use metadata::{WorldMetadata, SaveVersion};
pub use migration::{MigrationManager, Migration};
pub use backup::{BackupManager, BackupPolicy};
pub use error::{atomic_write, PersistenceErrorContext, LockResultExt};
pub use atomic_save::{AtomicSaveManager, AtomicSaveConfig, SaveOperation, SavePriority, SaveOperationResult, AtomicSaveStats};
pub use state_validator::{StateValidator, ValidationConfig, ValidationResult, ValidationError, ValidationWarning, StateSnapshot, ValidationStats};
pub use network_validator::{
    NetworkValidator, ValidationConfig as NetworkValidationConfig, ValidationResult as NetworkValidationResult, 
    ValidationError as NetworkValidationError, ValidationWarning as NetworkValidationWarning, 
    ValidationStats as NetworkValidationStats, ValidationType, ChunkValidationData,
    PlayerValidationData, WorldValidationState,
};

/// Result type for persistence operations
pub type PersistenceResult<T> = Result<T, PersistenceError>;

/// Errors that can occur during persistence operations
#[derive(Debug)]
pub enum PersistenceError {
    IoError(std::io::Error),
    SerializationError(String),
    DeserializationError(String),
    CompressionError(String),
    VersionMismatch { expected: u32, found: u32 },
    CorruptedData(String),
    MigrationError(String),
    BackupError(String),
    LockPoisoned(String),
    PlayerNotFound(String),
    CapacityExceeded(String),
}

impl std::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistenceError::IoError(e) => write!(f, "IO error: {}", e),
            PersistenceError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            PersistenceError::DeserializationError(e) => write!(f, "Deserialization error: {}", e),
            PersistenceError::CompressionError(e) => write!(f, "Compression error: {}", e),
            PersistenceError::VersionMismatch { expected, found } => {
                write!(f, "Version mismatch: expected {}, found {}", expected, found)
            }
            PersistenceError::CorruptedData(e) => write!(f, "Corrupted data: {}", e),
            PersistenceError::MigrationError(e) => write!(f, "Migration error: {}", e),
            PersistenceError::BackupError(e) => write!(f, "Backup error: {}", e),
            PersistenceError::LockPoisoned(e) => write!(f, "Lock poisoned: {}", e),
            PersistenceError::PlayerNotFound(e) => write!(f, "Player not found: {}", e),
            PersistenceError::CapacityExceeded(e) => write!(f, "Capacity exceeded: {}", e),
        }
    }
}

impl std::error::Error for PersistenceError {}

impl From<std::io::Error> for PersistenceError {
    fn from(err: std::io::Error) -> Self {
        PersistenceError::IoError(err)
    }
}

impl From<bincode::Error> for PersistenceError {
    fn from(err: bincode::Error) -> Self {
        PersistenceError::SerializationError(err.to_string())
    }
}

impl<T> From<std::sync::PoisonError<T>> for PersistenceError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        PersistenceError::LockPoisoned("A thread panicked while holding a lock".to_string())
    }
}