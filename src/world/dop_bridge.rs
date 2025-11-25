//! DOP Bridge - Compatibility Layer
//!
//! This module provides compatibility between old OOP-style code and new DOP architecture.
//! It will be removed once full DOP conversion is complete.

use super::core::{BlockId, ChunkPos, Ray, RaycastHit, VoxelPos};
use super::data_types::WorldData;
use super::world_operations;
use super::error::WorldError;

/// Bridge function: get_block with OOP-style error handling
pub fn bridge_get_block(
    world: &WorldData,
    pos: VoxelPos,
    chunk_size: u32,
) -> Result<BlockId, WorldError> {
    Ok(world_operations::get_block(world, pos, chunk_size))
}

/// Bridge function: set_block
pub fn bridge_set_block(
    world: &mut WorldData,
    pos: VoxelPos,
    block_id: BlockId,
    chunk_size: u32,
) -> Result<(), WorldError> {
    world_operations::set_block(world, pos, block_id, chunk_size)?;
    Ok(())
}

/// Bridge function: raycast
pub fn bridge_raycast(
    world: &WorldData,
    ray: Ray,
    max_distance: f32,
    chunk_size: u32,
) -> Result<Option<RaycastHit>, WorldError> {
    Ok(world_operations::raycast(world, ray, max_distance, chunk_size))
}

/// Bridge function: is_chunk_loaded
pub fn bridge_is_chunk_loaded(
    world: &WorldData,
    chunk_pos: ChunkPos,
) -> Result<bool, WorldError> {
    Ok(world_operations::is_chunk_loaded(world, chunk_pos))
}

/// Convert WorldData to reference (for compatibility)
pub fn as_world_ref(world: &WorldData) -> &WorldData {
    world
}

/// Convert WorldData to mutable reference (for compatibility)
pub fn as_world_mut(world: &mut WorldData) -> &mut WorldData {
    world
}
