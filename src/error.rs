//! Comprehensive error handling for Hearth Engine
//!
//! This module provides a unified error type that replaces all unwrap() calls
//! with proper error handling, ensuring the engine never panics in production.

use std::error::Error as StdError;
use std::fmt;
use std::sync::{MutexGuard, PoisonError, RwLockReadGuard, RwLockWriteGuard};

/// Main error type for Hearth Engine
#[derive(Debug)]
pub enum EngineError {
    // Resource Errors
    BufferAccess {
        index: usize,
        size: usize,
    },
    TextureNotFound {
        id: String,
    },
    ShaderCompilation {
        source: String,
        error: String,
    },
    MeshGeneration {
        chunk_pos: (i32, i32, i32),
        error: String,
    },

    // World Errors
    ChunkNotLoaded {
        pos: (i32, i32, i32),
    },
    BlockOutOfBounds {
        pos: (i32, i32, i32),
        chunk_size: u32,
    },
    InvalidBlockType {
        id: u32,
    },
    BiomeNotFound {
        id: u32,
    },

    // Persistence Errors
    SaveFailed {
        path: String,
        error: String,
    },
    LoadFailed {
        path: String,
        error: String,
    },
    CorruptedData {
        reason: String,
    },
    VersionMismatch {
        expected: u32,
        found: u32,
    },

    // Network Errors
    ConnectionFailed {
        addr: String,
        error: String,
    },
    ProtocolError {
        message: String,
    },
    PacketTooLarge {
        size: usize,
        max_size: usize,
    },
    PlayerNotFound {
        id: u64,
    },

    // Threading Errors
    LockPoisoned {
        resource: String,
    },
    ChannelClosed {
        name: String,
    },
    TaskJoinError {
        task: String,
    },

    // GPU Errors
    DeviceNotFound,
    BufferCreationFailed {
        size: u64,
        usage: String,
    },
    BindGroupLayoutMismatch {
        expected: String,
        found: String,
    },
    RenderPipelineError {
        error: String,
    },

    // Memory Errors
    AllocationFailed {
        size: usize,
        reason: String,
    },
    OutOfMemory {
        requested: usize,
        available: usize,
    },

    // Configuration Errors
    InvalidConfig {
        field: String,
        value: String,
        reason: String,
    },
    MissingConfig {
        field: String,
    },

    // System Errors
    InitializationError(String),
    IoError {
        path: String,
        error: String,
    },
    Utf8Error {
        context: String,
    },
    ParseError {
        value: String,
        expected_type: String,
    },

    // Hot Reload Errors
    AssetWatchError {
        path: String,
        error: String,
    },
    ShaderReloadFailed {
        name: String,
        error: String,
    },

    // System Errors (continued)
    SystemError {
        component: String,
        error: String,
    },
    BufferError {
        operation: String,
        error: String,
    },
    StateError {
        expected: String,
        actual: String,
    },
    ResourceNotFound {
        resource_type: String,
        id: String,
    },
    GpuOperationFailed {
        operation: String,
        error: String,
    },
    SerializationError {
        context: String,
        error: String,
    },
    DeserializationError {
        context: String,
        error: String,
    },

    // Additional integration errors
    ValidationFailed(String),
    TimeoutError(String),
    ProcessingFailed(String),
    ResourceExhausted(String),
    FeatureDisabled(String),

    // Generic fallback for unexpected errors
    Internal {
        message: String,
    },
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::BufferAccess { index, size } => write!(
                f,
                "Buffer access out of bounds: index {} >= size {}",
                index, size
            ),
            EngineError::TextureNotFound { id } => write!(f, "Texture not found: {}", id),
            EngineError::ShaderCompilation { source, error } => {
                write!(f, "Shader compilation failed for {}: {}", source, error)
            }
            EngineError::MeshGeneration { chunk_pos, error } => write!(
                f,
                "Mesh generation failed for chunk {:?}: {}",
                chunk_pos, error
            ),

            EngineError::ChunkNotLoaded { pos } => {
                write!(f, "Chunk not loaded at position {:?}", pos)
            }
            EngineError::BlockOutOfBounds { pos, chunk_size } => write!(
                f,
                "Block position {:?} out of bounds for chunk size {}",
                pos, chunk_size
            ),
            EngineError::InvalidBlockType { id } => write!(f, "Invalid block type ID: {}", id),
            EngineError::BiomeNotFound { id } => write!(f, "Biome not found: {}", id),

            EngineError::SaveFailed { path, error } => {
                write!(f, "Save failed for {}: {}", path, error)
            }
            EngineError::LoadFailed { path, error } => {
                write!(f, "Load failed for {}: {}", path, error)
            }
            EngineError::CorruptedData { reason } => write!(f, "Data corrupted: {}", reason),
            EngineError::VersionMismatch { expected, found } => write!(
                f,
                "Version mismatch: expected {}, found {}",
                expected, found
            ),

            EngineError::ConnectionFailed { addr, error } => {
                write!(f, "Connection failed to {}: {}", addr, error)
            }
            EngineError::ProtocolError { message } => write!(f, "Protocol error: {}", message),
            EngineError::PacketTooLarge { size, max_size } => {
                write!(f, "Packet too large: {} bytes (max: {})", size, max_size)
            }
            EngineError::PlayerNotFound { id } => write!(f, "Player not found: {}", id),

            EngineError::LockPoisoned { resource } => {
                write!(f, "Lock poisoned for resource: {}", resource)
            }
            EngineError::ChannelClosed { name } => write!(f, "Channel closed: {}", name),
            EngineError::TaskJoinError { task } => write!(f, "Task join error: {}", task),

            EngineError::DeviceNotFound => write!(f, "GPU device not found"),
            EngineError::BufferCreationFailed { size, usage } => {
                write!(f, "Buffer creation failed: size={}, usage={}", size, usage)
            }
            EngineError::BindGroupLayoutMismatch { expected, found } => write!(
                f,
                "Bind group layout mismatch: expected {}, found {}",
                expected, found
            ),
            EngineError::RenderPipelineError { error } => {
                write!(f, "Render pipeline error: {}", error)
            }

            EngineError::AllocationFailed { size, reason } => {
                write!(f, "Allocation failed for {} bytes: {}", size, reason)
            }
            EngineError::OutOfMemory {
                requested,
                available,
            } => write!(
                f,
                "Out of memory: requested {} bytes, available {}",
                requested, available
            ),

            EngineError::InvalidConfig {
                field,
                value,
                reason,
            } => write!(f, "Invalid config: {} = {} ({})", field, value, reason),
            EngineError::MissingConfig { field } => write!(f, "Missing required config: {}", field),

            EngineError::InitializationError(msg) => write!(f, "Initialization error: {}", msg),
            EngineError::IoError { path, error } => write!(f, "IO error for {}: {}", path, error),
            EngineError::Utf8Error { context } => write!(f, "UTF-8 error in {}", context),
            EngineError::ParseError {
                value,
                expected_type,
            } => write!(
                f,
                "Parse error: '{}' is not a valid {}",
                value, expected_type
            ),

            EngineError::AssetWatchError { path, error } => {
                write!(f, "Asset watch error for {}: {}", path, error)
            }
            EngineError::ShaderReloadFailed { name, error } => {
                write!(f, "Shader reload failed for {}: {}", name, error)
            }

            EngineError::SystemError { component, error } => {
                write!(f, "System error in {}: {}", component, error)
            }
            EngineError::BufferError { operation, error } => {
                write!(f, "Buffer error during {}: {}", operation, error)
            }
            EngineError::StateError { expected, actual } => {
                write!(f, "State error: expected {}, actual {}", expected, actual)
            }
            EngineError::ResourceNotFound { resource_type, id } => {
                write!(f, "Resource not found: {} '{}'", resource_type, id)
            }
            EngineError::GpuOperationFailed { operation, error } => {
                write!(f, "GPU operation '{}' failed: {}", operation, error)
            }
            EngineError::SerializationError { context, error } => {
                write!(f, "Serialization error in {}: {}", context, error)
            }
            EngineError::DeserializationError { context, error } => {
                write!(f, "Deserialization error in {}: {}", context, error)
            }

            EngineError::ValidationFailed(msg) => write!(f, "Validation failed: {}", msg),
            EngineError::TimeoutError(msg) => write!(f, "Timeout error: {}", msg),
            EngineError::ProcessingFailed(msg) => write!(f, "Processing failed: {}", msg),
            EngineError::ResourceExhausted(msg) => write!(f, "Resource exhausted: {}", msg),
            EngineError::FeatureDisabled(msg) => write!(f, "Feature disabled: {}", msg),

            EngineError::Internal { message } => write!(f, "Internal error: {}", message),
        }
    }
}

impl StdError for EngineError {}

/// Type alias for Results in Hearth Engine
pub type EngineResult<T> = Result<T, EngineError>;

// Conversion traits for common error types

impl From<std::io::Error> for EngineError {
    fn from(error: std::io::Error) -> Self {
        EngineError::IoError {
            path: String::new(),
            error: error.to_string(),
        }
    }
}

impl From<std::str::Utf8Error> for EngineError {
    fn from(_: std::str::Utf8Error) -> Self {
        EngineError::Utf8Error {
            context: "unknown".to_string(),
        }
    }
}

impl<T> From<PoisonError<MutexGuard<'_, T>>> for EngineError {
    fn from(_: PoisonError<MutexGuard<'_, T>>) -> Self {
        EngineError::LockPoisoned {
            resource: "mutex".to_string(),
        }
    }
}

impl<T> From<PoisonError<RwLockReadGuard<'_, T>>> for EngineError {
    fn from(_: PoisonError<RwLockReadGuard<'_, T>>) -> Self {
        EngineError::LockPoisoned {
            resource: "rwlock_read".to_string(),
        }
    }
}

impl<T> From<PoisonError<RwLockWriteGuard<'_, T>>> for EngineError {
    fn from(_: PoisonError<RwLockWriteGuard<'_, T>>) -> Self {
        EngineError::LockPoisoned {
            resource: "rwlock_write".to_string(),
        }
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for EngineError {
    fn from(_: std::sync::mpsc::SendError<T>) -> Self {
        EngineError::ChannelClosed {
            name: "mpsc".to_string(),
        }
    }
}

impl From<std::sync::mpsc::RecvError> for EngineError {
    fn from(_: std::sync::mpsc::RecvError) -> Self {
        EngineError::ChannelClosed {
            name: "mpsc".to_string(),
        }
    }
}

impl From<crate::persistence::PersistenceError> for EngineError {
    fn from(err: crate::persistence::PersistenceError) -> Self {
        use crate::persistence::PersistenceError;
        match err {
            PersistenceError::SaveFailed(e) => EngineError::SaveFailed {
                path: String::new(),
                error: e,
            },
            PersistenceError::IoError(e) => EngineError::IoError {
                path: String::new(),
                error: e.to_string(),
            },
            PersistenceError::SerializationError(e) => EngineError::SerializationError {
                context: "persistence".to_string(),
                error: e,
            },
            PersistenceError::DeserializationError(e) => EngineError::DeserializationError {
                context: "persistence".to_string(),
                error: e,
            },
            PersistenceError::CompressionError(e) => EngineError::Internal {
                message: format!("Compression error: {}", e),
            },
            PersistenceError::VersionMismatch { expected, found } => {
                EngineError::VersionMismatch {
                    expected: expected.parse().unwrap_or(0),
                    found: found.parse().unwrap_or(0),
                }
            }
            PersistenceError::CorruptedData(e) => EngineError::CorruptedData { reason: e },
            PersistenceError::MigrationError(e) => EngineError::Internal {
                message: format!("Migration error: {}", e),
            },
            PersistenceError::BackupError(e) => EngineError::Internal {
                message: format!("Backup error: {}", e),
            },
            PersistenceError::LockPoisoned(e) => EngineError::LockPoisoned { resource: e },
            PersistenceError::PlayerNotFound(e) => EngineError::Internal {
                message: format!("Player not found: {}", e),
            },
            PersistenceError::CapacityExceeded(e) => EngineError::Internal {
                message: format!("Capacity exceeded: {}", e),
            },
        }
    }
}

// Helper functions for common error patterns

/// Convert Option to Result with context
pub trait OptionExt<T> {
    fn ok_or_engine<F>(self, f: F) -> EngineResult<T>
    where
        F: FnOnce() -> EngineError;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_engine<F>(self, f: F) -> EngineResult<T>
    where
        F: FnOnce() -> EngineError,
    {
        self.ok_or_else(f)
    }
}

/// Extension trait for adding context to errors
pub trait ErrorContext<T> {
    fn context(self, msg: &str) -> EngineResult<T>;
    fn with_context<F>(self, f: F) -> EngineResult<T>
    where
        F: FnOnce() -> String;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: Into<EngineError>,
{
    fn context(self, msg: &str) -> EngineResult<T> {
        self.map_err(|_| EngineError::Internal {
            message: msg.to_string(),
        })
    }

    fn with_context<F>(self, f: F) -> EngineResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|_| EngineError::Internal { message: f() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = EngineError::BufferAccess { index: 10, size: 5 };
        assert_eq!(
            err.to_string(),
            "Buffer access out of bounds: index 10 >= size 5"
        );
    }

    #[test]
    fn test_option_ext() {
        let opt: Option<i32> = None;
        let result = opt.ok_or_engine(|| EngineError::Internal {
            message: "test".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_error_context() {
        let result: Result<i32, std::io::Error> = Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        let with_context = result.context("loading config");
        assert!(with_context.is_err());
    }
}
