//! Block Data - Pure DOP
//!
//! Block property data structures. No methods.

use crate::world::core::BlockId;

// Re-export BlockProperties from engine_buffers for compatibility
pub use crate::engine_buffers::BlockProperties;

// Stub constant for compatibility - array of (BlockId, BlockProperties) tuples
pub const BLOCK_PROPERTIES: [(BlockId, BlockProperties); 0] = [];

/// Block data structure
#[derive(Clone, Debug)]
pub struct BlockData {
    pub id: BlockId,
    pub name: String,
    pub is_solid: bool,
    pub is_transparent: bool,
    pub light_emission: u8,
}

impl Default for BlockData {
    fn default() -> Self {
        Self {
            id: BlockId::AIR,
            name: "air".to_string(),
            is_solid: false,
            is_transparent: true,
            light_emission: 0,
        }
    }
}
