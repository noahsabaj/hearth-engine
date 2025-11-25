/// World Error Handling
///
/// Provides error types and utilities for the unified world system.
use crate::error::{EngineError, EngineResult};

/// World-specific result type
pub type WorldResult<T> = EngineResult<T>;

/// Error context for world operations
pub trait WorldErrorContext<T> {
    fn world_context(self, context: &str) -> WorldResult<T>;
}

impl<T> WorldErrorContext<T> for Option<T> {
    fn world_context(self, context: &str) -> WorldResult<T> {
        self.ok_or_else(|| EngineError::ResourceNotFound {
            resource_type: "world".to_string(),
            id: context.to_string(),
        })
    }
}

impl<T, E> WorldErrorContext<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn world_context(self, context: &str) -> WorldResult<T> {
        self.map_err(|e| EngineError::SystemError {
            component: "world".to_string(),
            error: format!("{}: {}", context, e),
        })
    }
}

// Re-export for compatibility during migration
pub type WorldGpuResult<T> = WorldResult<T>;
pub trait WorldGpuErrorContext<T>: WorldErrorContext<T> {}
impl<T> WorldGpuErrorContext<T> for Option<T> {}
impl<T, E> WorldGpuErrorContext<T> for Result<T, E> where E: std::fmt::Display {}

// WorldError for DOP operations
#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("Chunk not loaded")]
    ChunkNotLoaded,

    #[error("Invalid position")]
    InvalidPosition,

    #[error("Operation failed: {0}")]
    OperationFailed(String),
}
