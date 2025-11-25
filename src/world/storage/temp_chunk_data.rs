//! Temporary Chunk Data - Pure DOP
//!
//! NO METHODS. Just data.
//! All transformations happen in temp_chunk_operations.rs

use crate::world::core::{BlockId, ChunkPos};

/// Temporary chunk data for GPU transfer
#[derive(Clone, Debug)]
pub struct TempChunkData {
    pub position: ChunkPos,
    pub blocks: Vec<BlockId>,
    pub size: u32,
}
