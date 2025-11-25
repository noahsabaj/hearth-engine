//! GPU-first storage systems - WorldBuffer as the primary storage
//!
//! This module provides GPU-resident world storage,
//! following the GPU-first architecture principle.

mod gpu_chunks;
mod temp_chunk;
mod world_buffer;

// Type alias for compatibility
pub use crate::world::data_types::ChunkData as Chunk;

// GPU-first storage (primary)
pub use world_buffer::{VoxelData, WorldBuffer, WorldBufferDescriptor};

// GPU chunk management
pub use gpu_chunks::{GpuChunk, GpuChunkManager, GpuChunkStats};

// Temporary chunk for GPU data transfer only
pub use temp_chunk::TempChunk;

/// GPU-first storage backend
pub struct UnifiedStorage {
    /// GPU-resident storage
    pub world_buffer: std::sync::Arc<std::sync::Mutex<WorldBuffer>>,
    pub device: std::sync::Arc<wgpu::Device>,
}

impl UnifiedStorage {
    /// Create GPU-based storage
    pub async fn new(
        device: std::sync::Arc<wgpu::Device>,
        descriptor: &WorldBufferDescriptor,
    ) -> Result<Self, StorageError> {
        let world_buffer = WorldBuffer::new(device.clone(), descriptor);
        Ok(UnifiedStorage {
            world_buffer: std::sync::Arc::new(std::sync::Mutex::new(world_buffer)),
            device,
        })
    }

    /// Get GPU world buffer
    pub fn gpu_world_buffer(&self) -> std::sync::Arc<std::sync::Mutex<WorldBuffer>> {
        self.world_buffer.clone()
    }
}

/// Storage system errors
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("GPU initialization failed: {message}")]
    GpuInitFailed { message: String },

    #[error("Memory allocation failed: {size} bytes")]
    MemoryAllocationFailed { size: u64 },

    #[error("Invalid chunk position: {x}, {y}, {z}")]
    InvalidChunkPosition { x: i32, y: i32, z: i32 },

    #[error("Backend mismatch: operation requires {required} but storage is {actual}")]
    BackendMismatch { required: String, actual: String },
}
