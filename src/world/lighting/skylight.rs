//! Skylight calculation for the unified world system
//!
//! This module provides skylight propagation and column updates
//! compatible with the GPU-first architecture.
//! All functions are pure DOP - take data, return results.

use crate::world::core::{BlockId, VoxelPos};
use crate::world::{world_operations, data_types::WorldData};

/// Skylight calculator - provides column-based skylight updates
/// Pure DOP functions for skylight calculations
pub struct SkylightCalculator;

impl SkylightCalculator {
    /// Update skylight values for a vertical column
    ///
    /// This recalculates skylight propagation when blocks change.
    /// In the GPU-first architecture, this would typically be done on GPU.
    pub fn update_column(world: &WorldData, x: i32, _y: i32, z: i32, chunk_size: u32) {
        // Start from the top of the world
        let mut current_light = 15u8; // Full skylight at top

        // Scan down the column
        for y in (0..256).rev() {
            let pos = VoxelPos::new(x, y, z);
            let block = world_operations::get_block(world, pos, chunk_size);

            // Air blocks get full skylight from above
            if block == BlockId::AIR {
                // In a full implementation, we'd set skylight value here
                // For now, just track it
                current_light = 15;
            } else if is_transparent(block) {
                // Transparent blocks reduce light slightly
                current_light = current_light.saturating_sub(1);
            } else {
                // Opaque blocks block all skylight
                current_light = 0;
            }

            // In a full implementation, we'd propagate horizontally here
        }
    }

    /// Update skylight for a specific position and its neighbors
    pub fn update_at_position(world: &WorldData, pos: VoxelPos, chunk_size: u32) {
        // Update the column containing this position
        Self::update_column(world, pos.x, pos.y, pos.z, chunk_size);

        // Also update neighboring columns that might be affected
        for dx in -1..=1 {
            for dz in -1..=1 {
                if dx != 0 || dz != 0 {
                    Self::update_column(world, pos.x + dx, pos.y, pos.z + dz, chunk_size);
                }
            }
        }
    }
}

/// Helper function to check if a block is transparent for skylight
/// Pure function - no world reference needed
fn is_transparent(block_id: BlockId) -> bool {
    // Water is transparent but dims light
    if block_id == BlockId::WATER {
        return true;
    }

    // Glass and similar blocks would be transparent
    if block_id == BlockId::GLASS {
        return true;
    }

    // Most blocks are opaque
    false
}
