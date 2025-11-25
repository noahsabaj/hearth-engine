//! Block Registry Operations - Pure DOP Functions
//!
//! All functions are pure: take data, return results, no side effects.
//! No methods, no self, just transformations.

use super::registry_data::{BlockRegistration, BlockRegistryData};
use super::BlockId;
use crate::world::blocks::block_data::{BlockProperties, BLOCK_PROPERTIES};
use std::collections::HashMap;

/// Create new block registry data
pub fn create_block_registry() -> BlockRegistryData {
    let mut data = BlockRegistryData {
        blocks: HashMap::new(),
        name_to_id: HashMap::new(),
        registrations: Vec::new(),
        next_engine_id: 1, // 0 is reserved for AIR, engine blocks use 1-99
        next_game_id: 100, // Game blocks start at 100
    };

    // Register built-in blocks from the static table
    for (id, properties) in BLOCK_PROPERTIES {
        data.blocks.insert(id, properties);
    }

    data
}

/// Register a new block type with properties
pub fn register_block(
    data: &mut BlockRegistryData,
    name: &str,
    properties: BlockProperties,
) -> BlockId {
    // Debug logging to track ID assignment
    log::info!("BlockRegistry::register called for '{}'", name);
    log::info!(
        "  - Current next_engine_id: {}, next_game_id: {}",
        data.next_engine_id,
        data.next_game_id
    );

    // Determine if this is an engine block or game block based on name prefix
    let is_engine_block = name.starts_with("engine:") || !name.contains(':');
    log::info!(
        "  - Checking '{}': starts_with('engine:')={}, contains(':')={}, is_engine_block={}",
        name,
        name.starts_with("engine:"),
        name.contains(':'),
        is_engine_block
    );

    let id = if is_engine_block {
        // Engine blocks use IDs 1-99
        if data.next_engine_id >= 100 {
            panic!("Too many engine blocks registered (max 99)");
        }
        let id = BlockId(data.next_engine_id);
        data.next_engine_id += 1;
        log::info!("  - Assigned ENGINE block ID {} to '{}'", id.0, name);
        id
    } else {
        // Game blocks (with mod prefix like "danger_money:") use IDs 100+
        let id = BlockId(data.next_game_id);
        data.next_game_id += 1;
        log::info!("  - Assigned GAME block ID {} to '{}'", id.0, name);
        id
    };

    data.blocks.insert(id, properties.clone());
    data.name_to_id.insert(name.to_string(), id);

    data.registrations.push(BlockRegistration {
        id,
        name: name.to_string(),
        properties,
    });

    log::info!(
        "Registered block '{}' with ID {} (engine: {}, game: {})",
        name,
        id.0,
        data.next_engine_id,
        data.next_game_id
    );
    id
}

/// Get block properties by ID
pub fn get_properties(data: &BlockRegistryData, id: BlockId) -> Option<&BlockProperties> {
    data.blocks.get(&id)
}

/// Get a block ID by name
pub fn get_id(data: &BlockRegistryData, name: &str) -> Option<BlockId> {
    data.name_to_id.get(name).copied()
}

/// Get all registered blocks
pub fn get_registrations(data: &BlockRegistryData) -> &[BlockRegistration] {
    &data.registrations
}

/// Check if a block ID is registered
pub fn is_registered(data: &BlockRegistryData, id: BlockId) -> bool {
    data.blocks.contains_key(&id)
}
