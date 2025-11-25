//! Parallel World - Stub Implementation
//!
//! Support for multiple parallel worlds (different dimensions/servers).

use crate::world::core::{BlockId, VoxelPos};
use cgmath::Point3;

/// Parallel world configuration
#[derive(Clone, Debug)]
pub struct ParallelWorldConfig {
    pub world_id: u32,
    pub world_name: String,
    pub seed: u32,
}

impl Default for ParallelWorldConfig {
    fn default() -> Self {
        Self {
            world_id: 0,
            world_name: "main".to_string(),
            seed: 0,
        }
    }
}

/// Parallel world instance (stub)
pub struct ParallelWorld {
    config: ParallelWorldConfig,
}

impl ParallelWorld {
    pub fn new(config: ParallelWorldConfig) -> Self {
        Self { config }
    }

    pub fn get_block(&self, _pos: VoxelPos) -> BlockId {
        BlockId::AIR
    }

    pub fn set_block(&mut self, _pos: VoxelPos, _block: BlockId) -> Result<(), String> {
        Ok(())
    }
}

/// Spawn finder - finds safe spawn locations (stub)
pub struct SpawnFinder;

impl SpawnFinder {
    pub fn new() -> Self {
        Self
    }

    pub fn find_spawn_point(&self, _seed: u32) -> Point3<f32> {
        Point3::new(0.0, 70.0, 0.0)
    }
}

impl Default for SpawnFinder {
    fn default() -> Self {
        Self::new()
    }
}
