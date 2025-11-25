//! GPU-First World Module
//!
//! This module provides all world functionality following a GPU-first
//! architecture principle. CPU is only used for orchestration and I/O.
//!
//! # Architecture Overview
//!
//! - **Core**: Fundamental data types (Block, Position, Ray)
//! - **Storage**: GPU WorldBuffer as primary storage
//! - **Generation**: GPU TerrainGeneratorSOA for world generation
//! - **Compute**: GPU kernels, shaders, and optimization structures
//! - **Management**: GPU-based world management
//! - **Interfaces**: Clean abstractions for GPU operations
//!
//! # Design Principles
//!
//! 1. **GPU-first**: All computation happens on GPU
//! 2. **CPU orchestration**: CPU only handles coordination and I/O
//! 3. **DOP architecture**: Data-oriented design throughout
//! 4. **Zero-copy**: Minimize CPU/GPU transfers

pub mod blocks;
pub mod compute;
pub mod core;
pub mod data_types;
pub mod dop_bridge;
pub mod error;
pub mod generation;
pub mod interfaces;
pub mod lighting;
pub mod management;
pub mod storage;
pub mod weather_manager;
pub mod world_operations;

// Re-export core types for convenience
pub use core::{
    BlockFace, BlockId, BlockRegistry, ChunkPos, PhysicsProperties, Ray, RaycastHit,
    RenderData, VoxelPos,
};

// Re-export storage systems
pub use storage::{
    GpuChunk,
    GpuChunkManager,
    GpuChunkStats,
    TempChunk,
    VoxelData,
    // GPU-first storage
    WorldBuffer,
    WorldBufferDescriptor,
};

// Re-export generation systems
pub use generation::{
    CaveGenerator,
    OreGenerator,
    // GPU generators
    TerrainGeneratorSOA,
    TerrainGeneratorSOABuilder,
    // Unified generation interface
    WorldGenerator,
};

// Re-export compute systems
pub use compute::{
    // GPU optimization structures will be added later
    // GPU lighting and effects
    GpuLighting,
    PrecipitationParticle,
    SystemFlags,
    UnifiedKernelConfig,
    // GPU kernels and optimization
    UnifiedWorldKernel,
    WeatherData,
    WeatherGpu,
};

// Re-export management systems
pub use management::{
    GenerationStats,
    // Parallel world support
    ParallelWorld,
    ParallelWorldConfig,
    SpawnFinder,
    // Unified managers
    UnifiedWorldManager,
    WorldManagerConfig,
    // Performance and statistics
    WorldPerformanceMetrics,
};

// Re-export interfaces (WorldInterface removed - use world_operations instead)
pub use interfaces::{
    ChunkData, ChunkManager, ChunkManagerInterface, DefaultChunkManager, GeneratorInterface,
    OperationResult, QueryResult, ReadOnlyWorldInterface, UnifiedWorldInterface, WorldConfig,
    WorldError, WorldOperation, WorldQuery,
};

// Re-export DOP world operations as the primary API
pub use world_operations::{
    get_block, set_block, raycast, is_chunk_loaded, load_chunk, unload_chunk,
    get_chunks_in_radius, get_loaded_chunks, WorldModification,
    voxel_to_chunk, chunk_to_world, get_local_position,
    get_world_size, get_world_seed, get_world_tick, get_active_chunk_count,
    set_blocks_batch, get_blocks_batch, log_world_stats, validate_world_data,
};

// Re-export block system
pub use blocks::register_basic_blocks;

// Re-export lighting system
pub use lighting::{
    DayNightCycleData, LightLevel, LightType, LightUpdate, LightingStats, SkylightCalculator,
    TimeOfDayData,
};

// Re-export weather system
pub use weather_manager::{WeatherManager, WeatherZone};

/// Helper function to convert voxel position to chunk position
/// Following DOP principles - pure function that transforms data
pub fn voxel_to_chunk_pos(voxel_pos: VoxelPos, chunk_size: u32) -> ChunkPos {
    voxel_pos.to_chunk_pos(chunk_size)
}

/// Create GPU-based world manager
pub async fn create_unified_world(
    device: std::sync::Arc<wgpu::Device>,
    queue: std::sync::Arc<wgpu::Queue>,
    config: WorldManagerConfig,
) -> Result<UnifiedWorldManager, crate::world::management::WorldError> {
    UnifiedWorldManager::new_gpu(device, queue, config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::core::CHUNK_SIZE;

    #[test]
    fn test_voxel_to_chunk_conversion() {
        let voxel_pos = VoxelPos {
            x: 65,
            y: 32,
            z: -15,
        };
        let chunk_pos = voxel_to_chunk_pos(voxel_pos, 32);

        // 65 / 32 = 2, 32 / 32 = 1, -15 / 32 = -1
        assert_eq!(chunk_pos.x, 2);
        assert_eq!(chunk_pos.y, 1);
        assert_eq!(chunk_pos.z, -1);
    }

    #[test]
    fn test_voxel_to_chunk_conversion_with_constant() {
        // Test with actual chunk size constant
        let voxel_pos = VoxelPos {
            x: 125,
            y: 75,
            z: -25,
        };
        let chunk_pos = voxel_to_chunk_pos(voxel_pos, CHUNK_SIZE);

        // With CHUNK_SIZE=50: 125/50=2, 75/50=1, -25/50=-1
        assert_eq!(chunk_pos.x, 2);
        assert_eq!(chunk_pos.y, 1);
        assert_eq!(chunk_pos.z, -1);
    }

    #[test]
    fn test_block_id_constants() {
        assert_eq!(BlockId::AIR, BlockId(0));
        assert_ne!(BlockId::STONE, BlockId::AIR);
    }
}
