//! Clean interfaces for the unified world system
//!
//! This module provides abstract interfaces that work across both GPU and CPU
//! implementations, allowing for seamless switching between backends.
//!
//! NOTE: WorldInterface trait is deprecated - use world_operations DOP functions instead.

#[allow(deprecated)]
mod generator_interface;
#[allow(deprecated)]
mod world_interface;

pub use generator_interface::{GenerationRequest, GenerationResult, GeneratorInterface};
#[allow(deprecated)]
pub use world_interface::{
    ChunkData, ChunkManager, DefaultChunkManager, OperationResult, QueryResult,
    ReadOnlyWorldInterface, UnifiedWorldInterface, WorldConfig, WorldError, WorldInterface,
    WorldOperation, WorldQuery,
};

// Re-export chunk manager interface from management
pub use crate::world::management::{ChunkManagerInterface, ChunkStats};

/// Unified interface for any world system component
pub trait UnifiedInterface: Send + Sync {
    /// Get the backend type this interface is using
    fn backend_type(&self) -> &str;

    /// Check if this interface supports a specific capability
    fn supports_capability(&self, capability: &str) -> bool;

    /// Get performance metrics if available
    fn performance_metrics(&self) -> Option<std::collections::HashMap<String, f64>> {
        None
    }
}

/// Common capabilities that interfaces may support
pub mod capabilities {
    pub const REAL_TIME_GENERATION: &str = "real_time_generation";
    pub const BATCH_OPERATIONS: &str = "batch_operations";
    pub const GPU_ACCELERATION: &str = "gpu_acceleration";
    pub const INFINITE_WORLDS: &str = "infinite_worlds";
    pub const PHYSICS_SIMULATION: &str = "physics_simulation";
    pub const LIGHTING_CALCULATION: &str = "lighting_calculation";
    pub const WEATHER_EFFECTS: &str = "weather_effects";
    pub const MULTI_THREADING: &str = "multi_threading";
    pub const MEMORY_STREAMING: &str = "memory_streaming";
    pub const LOD_SUPPORT: &str = "lod_support";
}

/// Common query types for world interfaces
#[derive(Debug, Clone)]
pub enum QueryType {
    /// Get block at position
    GetBlock { pos: crate::world::core::VoxelPos },
    /// Get surface height at coordinates
    GetSurfaceHeight { x: f64, z: f64 },
    /// Check if chunk is loaded
    IsChunkLoaded { pos: crate::world::core::ChunkPos },
    /// Get chunks in radius
    GetChunksInRadius {
        center: crate::world::core::ChunkPos,
        radius: u32,
    },
    /// Raycast from origin in direction
    Raycast {
        origin: [f32; 3],
        direction: [f32; 3],
        max_distance: f32,
    },
}

/// Interface factory for creating unified interfaces
pub struct InterfaceFactory;

impl InterfaceFactory {
    /// Create a world interface from a unified world manager
    /// DEPRECATED: Use world_operations DOP functions instead
    #[allow(deprecated)]
    pub fn create_world_interface(
        manager: std::sync::Arc<std::sync::Mutex<crate::world::management::UnifiedWorldManager>>,
    ) -> Box<dyn WorldInterface> {
        Box::new(world_interface::UnifiedWorldInterface::new(manager))
    }

    /// Create a generator interface from a unified generator
    pub fn create_generator_interface(
        generator: std::sync::Arc<crate::world::generation::UnifiedGenerator>,
    ) -> Box<dyn GeneratorInterface> {
        Box::new(generator_interface::UnifiedGeneratorInterface::new(
            generator,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities_constants() {
        assert_eq!(capabilities::REAL_TIME_GENERATION, "real_time_generation");
        assert_eq!(capabilities::GPU_ACCELERATION, "gpu_acceleration");
    }

    #[test]
    fn test_query_type_creation() {
        let query = QueryType::GetBlock {
            pos: crate::world::core::VoxelPos { x: 0, y: 0, z: 0 },
        };

        match query {
            QueryType::GetBlock { pos } => {
                assert_eq!(pos.x, 0);
                assert_eq!(pos.y, 0);
                assert_eq!(pos.z, 0);
            }
            _ => panic!("Wrong query type"),
        }
    }
}
