//! Basic engine blocks for the unified world system
//!
//! This module defines the fundamental blocks that come with the engine.
//! Games can register additional blocks on top of these.

use crate::world::core::{BlockId, BlockRegistry, PhysicsProperties, RenderData};
use crate::world::blocks::block_data::BlockProperties;

/// Create grass block properties
pub fn create_grass_properties() -> BlockProperties {
    BlockProperties {
        id: BlockId::GRASS,
        name: "grass".to_string(),
        is_solid: true,
        is_transparent: false,
        transparent: false,
        light_emission: 0,
        physics_enabled: true,
        render_data: RenderData {
            color: [0.3, 0.8, 0.2], // Green grass color
            texture_id: 1,
            light_emission: 0,
        },
        physics: PhysicsProperties {
            solid: true,
            density: 1500.0, // kg/m³
        },
        hardness: 0.6, // Quick to break
        flammable: false,
        blast_resistance: 3.0,
    }
}

/// Create dirt block properties
pub fn create_dirt_properties() -> BlockProperties {
    BlockProperties {
        id: BlockId::DIRT,
        name: "dirt".to_string(),
        is_solid: true,
        is_transparent: false,
        transparent: false,
        light_emission: 0,
        physics_enabled: true,
        render_data: RenderData {
            color: [0.5, 0.3, 0.1], // Brown dirt color
            texture_id: 2,
            light_emission: 0,
        },
        physics: PhysicsProperties {
            solid: true,
            density: 1600.0,
        },
        hardness: 0.5,
        flammable: false,
        blast_resistance: 2.5,
    }
}

/// Create stone block properties
pub fn create_stone_properties() -> BlockProperties {
    BlockProperties {
        id: BlockId::STONE,
        name: "stone".to_string(),
        is_solid: true,
        is_transparent: false,
        transparent: false,
        light_emission: 0,
        physics_enabled: true,
        render_data: RenderData {
            color: [0.5, 0.5, 0.5], // Gray stone color
            texture_id: 3,
            light_emission: 0,
        },
        physics: PhysicsProperties {
            solid: true,
            density: 2500.0,
        },
        hardness: 1.5, // Harder to break
        flammable: false,
        blast_resistance: 30.0,
    }
}

/// Create water block properties
pub fn create_water_properties() -> BlockProperties {
    BlockProperties {
        id: BlockId::WATER,
        name: "water".to_string(),
        is_solid: false,
        is_transparent: true,
        transparent: true,
        light_emission: 0,
        physics_enabled: true,
        render_data: RenderData {
            color: [0.2, 0.3, 0.8], // Blue water color
            texture_id: 4,
            light_emission: 0,
        },
        physics: PhysicsProperties {
            solid: false,
            density: 1000.0,
        },
        hardness: 100.0, // Can't break water
        flammable: false,
        blast_resistance: 500.0,
    }
}

/// Create sand block properties
pub fn create_sand_properties() -> BlockProperties {
    BlockProperties {
        id: BlockId::SAND,
        name: "sand".to_string(),
        is_solid: true,
        is_transparent: false,
        transparent: false,
        light_emission: 0,
        physics_enabled: true,
        render_data: RenderData {
            color: [0.9, 0.8, 0.6], // Sandy color
            texture_id: 5,
            light_emission: 0,
        },
        physics: PhysicsProperties {
            solid: true,
            density: 1800.0,
        },
        hardness: 0.5,
        flammable: false,
        blast_resistance: 2.5,
    }
}

/// Create glowstone block properties
pub fn create_glowstone_properties() -> BlockProperties {
    BlockProperties {
        id: BlockId::GLOWSTONE,
        name: "glowstone".to_string(),
        is_solid: true,
        is_transparent: false,
        transparent: false,
        light_emission: 15,
        physics_enabled: true,
        render_data: RenderData {
            color: [1.0, 0.9, 0.6], // Bright yellow color
            texture_id: 6,
            light_emission: 15, // Maximum light level
        },
        physics: PhysicsProperties {
            solid: true,
            density: 2000.0,
        },
        hardness: 0.8,
        flammable: false,
        blast_resistance: 4.0,
    }
}

/// Register all basic engine blocks
///
/// This function registers the fundamental blocks that come with the engine.
/// Games should call this before registering their own blocks.
pub fn register_basic_blocks(registry: &mut BlockRegistry) {
    // Note: Air (BlockId 0) is handled specially by the engine
    
    // Register terrain blocks with their properties
    registry.register_block("engine:grass", create_grass_properties());
    registry.register_block("engine:dirt", create_dirt_properties());
    registry.register_block("engine:stone", create_stone_properties());
    registry.register_block("engine:water", create_water_properties());
    registry.register_block("engine:sand", create_sand_properties());
    registry.register_block("engine:glowstone", create_glowstone_properties());
}

// Usage example for games:
// 
// let mut registry = BlockRegistry::new();
// register_basic_blocks(&mut registry);
// 
// // Get properties for a block
// if let Some(props) = registry.get_properties(BlockId::GRASS) {
//     println!("Grass hardness: {}", props.hardness);
// }