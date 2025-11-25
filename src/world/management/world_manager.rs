//! World Manager - Stub Implementation
//!
//! Unified world manager that can use GPU or CPU backend.

use crate::world::core::{BlockId, ChunkPos, VoxelPos};
use super::Backend;

/// World manager configuration
#[derive(Clone, Debug)]
pub struct WorldManagerConfig {
    pub backend: Backend,
    pub chunk_size: u32,
    pub render_distance: u32,
    pub seed: u32,
}

impl Default for WorldManagerConfig {
    fn default() -> Self {
        Self {
            backend: Backend::Auto,
            chunk_size: 50,
            render_distance: 8,
            seed: 0,
        }
    }
}

/// World manager error
#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("GPU initialization failed: {0}")]
    GpuInitFailed(String),

    #[error("Chunk not loaded: {0:?}")]
    ChunkNotLoaded(ChunkPos),

    #[error("Invalid position: {0:?}")]
    InvalidPosition(VoxelPos),

    #[error("Backend error: {0}")]
    BackendError(String),
}

/// Unified world manager (stub)
pub struct UnifiedWorldManager {
    config: WorldManagerConfig,
}

impl UnifiedWorldManager {
    pub fn new(config: WorldManagerConfig) -> Self {
        Self { config }
    }

    pub async fn new_gpu(
        _device: std::sync::Arc<wgpu::Device>,
        _queue: std::sync::Arc<wgpu::Queue>,
        config: WorldManagerConfig,
    ) -> Result<Self, WorldError> {
        Ok(Self { config })
    }

    pub fn is_gpu(&self) -> bool {
        matches!(self.config.backend, Backend::Gpu)
    }

    pub fn get_block(&self, _pos: VoxelPos) -> BlockId {
        BlockId::AIR
    }

    pub fn set_block(&mut self, _pos: VoxelPos, _block: BlockId) -> Result<(), WorldError> {
        Ok(())
    }

    pub fn load_chunk(&mut self, _pos: ChunkPos) -> Result<(), WorldError> {
        Ok(())
    }
}
