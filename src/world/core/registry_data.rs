//! Block Registry Data - Pure DOP
//!
//! NO METHODS. Just data.
//! All transformations happen in registry_operations.rs

use super::BlockId;
use crate::world::blocks::block_data::BlockProperties;
use std::collections::HashMap;

/// Block registration data
#[derive(Debug, Clone)]
pub struct BlockRegistration {
    pub id: BlockId,
    pub name: String,
    pub properties: BlockProperties,
}

/// Registry data that stores all block types
pub struct BlockRegistryData {
    /// Map from BlockId to properties
    pub blocks: HashMap<BlockId, BlockProperties>,
    /// Map from name to BlockId
    pub name_to_id: HashMap<String, BlockId>,
    /// All registered blocks
    pub registrations: Vec<BlockRegistration>,
    pub next_engine_id: u16,
    pub next_game_id: u16,
}
