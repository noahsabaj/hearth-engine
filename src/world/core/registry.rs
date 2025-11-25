use super::BlockId;
use crate::world::blocks::block_data::{BlockProperties, BLOCK_PROPERTIES};
use std::collections::HashMap;

/// Block registration data
#[derive(Debug, Clone)]
pub struct BlockRegistration {
    pub id: BlockId,
    pub name: String,
    pub properties: BlockProperties,
}

/// Registry that stores all block types as data
pub struct BlockRegistry {
    /// Map from BlockId to properties
    blocks: HashMap<BlockId, BlockProperties>,
    /// Map from name to BlockId
    name_to_id: HashMap<String, BlockId>,
    /// All registered blocks
    registrations: Vec<BlockRegistration>,
    next_engine_id: u16,
    next_game_id: u16,
}

impl BlockRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            blocks: HashMap::new(),
            name_to_id: HashMap::new(),
            registrations: Vec::new(),
            next_engine_id: 1, // 0 is reserved for AIR, engine blocks use 1-99
            next_game_id: 100, // Game blocks start at 100
        };
        
        // Register built-in blocks from the static table
        for (id, properties) in BLOCK_PROPERTIES {
            registry.blocks.insert(id, properties);
        }
        
        registry
    }

    /// Register a new block type with properties
    pub fn register_block(&mut self, name: &str, properties: BlockProperties) -> BlockId {
        // Debug logging to track ID assignment
        log::info!("BlockRegistry::register called for '{}'", name);
        log::info!("  - Current next_engine_id: {}, next_game_id: {}", self.next_engine_id, self.next_game_id);
        
        // Determine if this is an engine block or game block based on name prefix
        let is_engine_block = name.starts_with("engine:") || !name.contains(':');
        log::info!("  - Checking '{}': starts_with('engine:')={}, contains(':')={}, is_engine_block={}", 
                  name, 
                  name.starts_with("engine:"), 
                  name.contains(':'),
                  is_engine_block);
        
        let id = if is_engine_block {
            // Engine blocks use IDs 1-99
            if self.next_engine_id >= 100 {
                panic!("Too many engine blocks registered (max 99)");
            }
            let id = BlockId(self.next_engine_id);
            self.next_engine_id += 1;
            log::info!("  - Assigned ENGINE block ID {} to '{}'", id.0, name);
            id
        } else {
            // Game blocks (with mod prefix like "danger_money:") use IDs 100+
            let id = BlockId(self.next_game_id);
            self.next_game_id += 1;
            log::info!("  - Assigned GAME block ID {} to '{}'", id.0, name);
            id
        };

        self.blocks.insert(id, properties.clone());
        self.name_to_id.insert(name.to_string(), id);

        self.registrations.push(BlockRegistration {
            id,
            name: name.to_string(),
            properties,
        });

        log::info!("Registered block '{}' with ID {} (engine: {}, game: {})", 
                  name, id.0, self.next_engine_id, self.next_game_id);
        id
    }

    /// Get block properties by ID
    pub fn get_properties(&self, id: BlockId) -> Option<&BlockProperties> {
        self.blocks.get(&id)
    }

    /// Get a block ID by name
    pub fn get_id(&self, name: &str) -> Option<BlockId> {
        self.name_to_id.get(name).copied()
    }
    
    /// Get all registered blocks
    pub fn get_registrations(&self) -> &[BlockRegistration] {
        &self.registrations
    }
    
    /// Check if a block ID is registered
    pub fn is_registered(&self, id: BlockId) -> bool {
        self.blocks.contains_key(&id)
    }
}
