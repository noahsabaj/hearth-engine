//! Temporary Chunk - For GPU data transfer
//!
//! Temporary chunk structure used for transferring data to/from GPU.
//! Pure data, no methods.

use crate::world::core::{BlockId, ChunkPos};

/// Temporary chunk for GPU transfer
#[derive(Clone, Debug)]
pub struct TempChunk {
    pub position: ChunkPos,
    pub blocks: Vec<BlockId>,
    pub size: u32,
}

impl TempChunk {
    pub fn new(position: ChunkPos, size: u32) -> Self {
        let total_blocks = (size * size * size) as usize;
        Self {
            position,
            blocks: vec![BlockId::AIR; total_blocks],
            size,
        }
    }

    pub fn new_empty(position: ChunkPos, size: u32) -> Self {
        Self::new(position, size)
    }

    pub fn with_blocks(position: ChunkPos, blocks: Vec<BlockId>, size: u32) -> Self {
        Self {
            position,
            blocks,
            size,
        }
    }

    pub fn position(&self) -> &ChunkPos {
        &self.position
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn blocks(&self) -> &[BlockId] {
        &self.blocks
    }

    pub fn set_block(&mut self, x: u32, y: u32, z: u32, block: BlockId) {
        let index = (y * self.size * self.size + z * self.size + x) as usize;
        if index < self.blocks.len() {
            self.blocks[index] = block;
        }
    }
}
