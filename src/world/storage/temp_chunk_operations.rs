//! Temporary Chunk Operations - Pure DOP Functions
//!
//! All functions are pure: take data, return results, no side effects.
//! No methods, no self, just transformations.

use super::temp_chunk_data::TempChunkData;
use crate::world::core::{BlockId, ChunkPos};

/// Create new temp chunk with all air blocks
pub fn create_temp_chunk(position: ChunkPos, size: u32) -> TempChunkData {
    let total_blocks = (size * size * size) as usize;
    TempChunkData {
        position,
        blocks: vec![BlockId::AIR; total_blocks],
        size,
    }
}

/// Create new empty temp chunk (alias for create_temp_chunk)
pub fn create_empty(position: ChunkPos, size: u32) -> TempChunkData {
    create_temp_chunk(position, size)
}

/// Create temp chunk with existing blocks
pub fn create_with_blocks(position: ChunkPos, blocks: Vec<BlockId>, size: u32) -> TempChunkData {
    TempChunkData {
        position,
        blocks,
        size,
    }
}

/// Get chunk position
pub fn position(data: &TempChunkData) -> &ChunkPos {
    &data.position
}

/// Get chunk size
pub fn size(data: &TempChunkData) -> u32 {
    data.size
}

/// Get blocks slice
pub fn blocks(data: &TempChunkData) -> &[BlockId] {
    &data.blocks
}

/// Set block at position
pub fn set_block(data: &mut TempChunkData, x: u32, y: u32, z: u32, block: BlockId) {
    let index = (y * data.size * data.size + z * data.size + x) as usize;
    if index < data.blocks.len() {
        data.blocks[index] = block;
    }
}

/// Get block at position
pub fn get_block(data: &TempChunkData, x: u32, y: u32, z: u32) -> BlockId {
    let index = (y * data.size * data.size + z * data.size + x) as usize;
    if index < data.blocks.len() {
        data.blocks[index]
    } else {
        BlockId::AIR
    }
}

/// Calculate voxel index from 3D coordinates
pub fn voxel_index(data: &TempChunkData, x: u32, y: u32, z: u32) -> usize {
    (y * data.size * data.size + z * data.size + x) as usize
}

/// Check if coordinates are within chunk bounds
pub fn is_in_bounds(data: &TempChunkData, x: u32, y: u32, z: u32) -> bool {
    x < data.size && y < data.size && z < data.size
}
