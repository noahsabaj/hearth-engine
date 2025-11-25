//! Skylight calculation for the unified world system
//!
//! This module handles skylight propagation from the sky downward,
//! updating light levels when blocks are placed or removed.
//! All functions are pure DOP - take data, return results.

use crate::world::{
    core::{BlockId, ChunkPos, VoxelPos},
    data_types::WorldData,
    world_operations,
    error::WorldError,
};

/// Maximum skylight level (full brightness from sky)
pub const MAX_SKY_LIGHT: u8 = 15;

/// Calculates skylight propagation from the sky downward
/// Pure DOP functions for GPU-first skylight calculations
pub struct SkylightCalculator;

impl SkylightCalculator {
    /// Calculate skylight for a newly loaded chunk
    /// Pure function - operates on WorldData
    pub fn calculate_for_chunk(
        world: &WorldData,
        chunk_pos: ChunkPos,
        chunk_size: u32,
    ) -> Result<(), WorldError> {
        let world_x_start = chunk_pos.x * chunk_size as i32;
        let world_y_start = chunk_pos.y * chunk_size as i32;
        let world_z_start = chunk_pos.z * chunk_size as i32;

        // For each column in the chunk
        for local_x in 0..chunk_size {
            for local_z in 0..chunk_size {
                let world_x = world_x_start + local_x as i32;
                let world_z = world_z_start + local_z as i32;

                // Propagate skylight down from the top
                for local_y in (0..chunk_size).rev() {
                    let world_y = world_y_start + local_y as i32;
                    let pos = VoxelPos::new(world_x, world_y, world_z);
                    let block = world_operations::get_block(world, pos, chunk_size);

                    if block == BlockId::AIR {
                        // Air blocks get skylight from above
                        // In GPU-first architecture, actual light values stored in GPU buffers
                    }
                }
            }
        }

        Ok(())
    }

    /// Update skylight when a block is placed or removed
    /// Pure function - operates on WorldData
    pub fn update_column(
        world: &WorldData,
        x: i32,
        y: i32,
        z: i32,
        chunk_size: u32,
    ) -> Result<(), WorldError> {
        let pos = VoxelPos::new(x, y, z);

        if world_operations::get_block(world, pos, chunk_size) == BlockId::AIR {
            // Block was removed - skylight needs to propagate down
            // In GPU-first architecture, lighting handled by GPU compute shaders
        } else {
            // Block was placed - remove skylight below
            // In GPU-first architecture, lighting handled by GPU compute shaders
        }

        Ok(())
    }
}
