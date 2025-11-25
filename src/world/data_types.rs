//! World Data Types - Pure DOP Structures
//!
//! These are the data structures that world_operations functions operate on.
//! NO METHODS - just pure data.

use super::core::{BlockId, ChunkPos};
use std::collections::HashSet;

/// World data - the main data structure for world state
///
/// This is what world_operations functions take as parameters.
/// In a real implementation, this would likely be GPU-resident buffers.
#[derive(Clone)]
pub struct WorldData {
    /// Chunk data (SOA - Structure of Arrays)
    pub chunks: Vec<ChunkData>,

    /// Active chunk positions
    pub active_chunks: HashSet<ChunkPos>,

    /// World size in chunks
    pub size_x: u32,
    pub size_y: u32,
    pub size_z: u32,

    /// Chunk capacity (pre-allocated size)
    pub chunk_capacity: usize,

    /// World generation seed
    pub seed: u32,

    /// World tick counter
    pub tick: u64,
}

/// Single chunk's data (Structure of Arrays)
#[derive(Clone)]
pub struct ChunkData {
    /// Chunk position in chunk coordinates
    pub position: ChunkPos,

    /// Block IDs (flat array: size^3 blocks)
    /// For chunk_size=50: 50*50*50 = 125,000 blocks
    pub blocks: Vec<BlockId>,

    /// Chunk metadata flags
    pub flags: ChunkMetadata,

    /// Last modified tick
    pub last_modified: u64,
}

// Temporary methods for compatibility (TODO: Remove and use fields directly)
impl ChunkData {
    pub fn blocks(&self) -> &[BlockId] {
        &self.blocks
    }

    pub fn position(&self) -> ChunkPos {
        self.position
    }
}

/// Chunk metadata
#[derive(Clone, Copy, Debug)]
pub struct ChunkMetadata {
    pub is_generated: bool,
    pub is_dirty: bool,
    pub is_empty: bool,
    pub needs_lighting_update: bool,
}

impl Default for ChunkMetadata {
    fn default() -> Self {
        Self {
            is_generated: false,
            is_dirty: false,
            is_empty: true,
            needs_lighting_update: false,
        }
    }
}

impl WorldData {
    /// Create new empty world data
    pub fn new(seed: u32, size_x: u32, size_y: u32, size_z: u32) -> Self {
        Self {
            chunks: Vec::new(),
            active_chunks: HashSet::new(),
            size_x,
            size_y,
            size_z,
            chunk_capacity: 0,
            seed,
            tick: 0,
        }
    }

    /// Create with pre-allocated chunk capacity
    pub fn with_capacity(seed: u32, size_x: u32, size_y: u32, size_z: u32, capacity: usize) -> Self {
        Self {
            chunks: Vec::with_capacity(capacity),
            active_chunks: HashSet::with_capacity(capacity),
            size_x,
            size_y,
            size_z,
            chunk_capacity: capacity,
            seed,
            tick: 0,
        }
    }
}

impl ChunkData {
    /// Create new chunk with given position and chunk size
    pub fn new(position: ChunkPos, chunk_size: u32) -> Self {
        let total_blocks = (chunk_size * chunk_size * chunk_size) as usize;

        Self {
            position,
            blocks: vec![BlockId::AIR; total_blocks],
            flags: ChunkMetadata::default(),
            last_modified: 0,
        }
    }

    /// Create chunk filled with a specific block
    pub fn filled(position: ChunkPos, chunk_size: u32, block: BlockId) -> Self {
        let total_blocks = (chunk_size * chunk_size * chunk_size) as usize;

        Self {
            position,
            blocks: vec![block; total_blocks],
            flags: ChunkMetadata {
                is_generated: true,
                is_dirty: false,
                is_empty: block == BlockId::AIR,
                needs_lighting_update: true,
            },
            last_modified: 0,
        }
    }
}

/// Chunk generation parameters
#[derive(Clone, Debug)]
pub struct ChunkGenParams {
    pub chunk_pos: ChunkPos,
    pub chunk_size: u32,
    pub world_seed: u32,
    pub generation_type: GenerationType,
}

/// Generation type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenerationType {
    Empty,
    Flat,
    Terrain,
    Custom,
}

/// Block change record for undo/redo
#[derive(Clone, Copy, Debug)]
pub struct BlockChange {
    pub position: super::core::VoxelPos,
    pub old_block: BlockId,
    pub new_block: BlockId,
    pub timestamp: u64,
}

/// World statistics
#[derive(Clone, Copy, Debug, Default)]
pub struct WorldStats {
    pub total_chunks: usize,
    pub loaded_chunks: usize,
    pub dirty_chunks: usize,
    pub total_blocks: u64,
    pub non_air_blocks: u64,
}
