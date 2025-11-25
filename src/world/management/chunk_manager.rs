//! Chunk Manager - Stub Implementation
//!
//! This will be properly implemented after DOP conversion is complete.

use crate::world::core::ChunkPos;

/// Chunk manager configuration
#[derive(Clone, Debug)]
pub struct ChunkManagerConfig {
    pub max_loaded_chunks: usize,
    pub load_radius: u32,
    pub unload_radius: u32,
}

impl Default for ChunkManagerConfig {
    fn default() -> Self {
        Self {
            max_loaded_chunks: 1000,
            load_radius: 8,
            unload_radius: 12,
        }
    }
}

/// Chunk statistics
#[derive(Clone, Copy, Debug, Default)]
pub struct ChunkStats {
    pub loaded_chunks: usize,
    pub active_chunks: usize,
    pub dirty_chunks: usize,
}

/// Chunk manager interface
pub trait ChunkManagerInterface: Send + Sync {
    fn load_chunk(&mut self, pos: ChunkPos) -> Result<(), String>;
    fn unload_chunk(&mut self, pos: ChunkPos) -> Result<(), String>;
    fn is_loaded(&self, pos: ChunkPos) -> bool;
    fn get_stats(&self) -> ChunkStats;
}

/// Unified chunk manager (stub)
pub struct UnifiedChunkManager {
    config: ChunkManagerConfig,
    stats: ChunkStats,
}

impl UnifiedChunkManager {
    pub fn new(config: ChunkManagerConfig) -> Self {
        Self {
            config,
            stats: ChunkStats::default(),
        }
    }
}

impl ChunkManagerInterface for UnifiedChunkManager {
    fn load_chunk(&mut self, _pos: ChunkPos) -> Result<(), String> {
        Ok(())
    }

    fn unload_chunk(&mut self, _pos: ChunkPos) -> Result<(), String> {
        Ok(())
    }

    fn is_loaded(&self, _pos: ChunkPos) -> bool {
        false
    }

    fn get_stats(&self) -> ChunkStats {
        self.stats
    }
}
