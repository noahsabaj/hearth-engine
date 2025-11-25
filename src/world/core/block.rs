#![allow(unused_variables, dead_code, unused_imports)]

use serde::{Deserialize, Serialize};

// Re-export basic blocks from basic_blocks.rs
// Basic blocks are now in a separate module
use std::fmt;

/// Unique identifier for a block type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct BlockId(pub u16);

// Safe because BlockId is just a u16
unsafe impl bytemuck::Pod for BlockId {}
unsafe impl bytemuck::Zeroable for BlockId {}

impl Default for BlockId {
    fn default() -> Self {
        BlockId::AIR
    }
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display block name if it's a known block
        match *self {
            BlockId::AIR => write!(f, "Air"),
            BlockId::GRASS => write!(f, "Grass"),
            BlockId::DIRT => write!(f, "Dirt"),
            BlockId::STONE => write!(f, "Stone"),
            BlockId::WOOD => write!(f, "Wood"),
            BlockId::SAND => write!(f, "Sand"),
            BlockId::WATER => write!(f, "Water"),
            BlockId::LEAVES => write!(f, "Leaves"),
            BlockId::GLASS => write!(f, "Glass"),
            BlockId::COAL_ORE => write!(f, "Coal Ore"),
            BlockId::IRON_ORE => write!(f, "Iron Ore"),
            BlockId::GOLD_ORE => write!(f, "Gold Ore"),
            BlockId::DIAMOND_ORE => write!(f, "Diamond Ore"),
            BlockId::BEDROCK => write!(f, "Bedrock"),
            BlockId::PLANKS => write!(f, "Planks"),
            BlockId::COBBLESTONE => write!(f, "Cobblestone"),
            BlockId::CRAFTING_TABLE => write!(f, "Crafting Table"),
            BlockId::FURNACE => write!(f, "Furnace"),
            BlockId::CHEST => write!(f, "Chest"),
            BlockId::TORCH => write!(f, "Torch"),
            BlockId::LADDER => write!(f, "Ladder"),
            BlockId::LAVA => write!(f, "Lava"),
            BlockId::LOG => write!(f, "Log"),
            BlockId::SANDSTONE => write!(f, "Sandstone"),
            BlockId::RED_SAND => write!(f, "Red Sand"),
            BlockId::RED_SANDSTONE => write!(f, "Red Sandstone"),
            BlockId::TALL_GRASS => write!(f, "Tall Grass"),
            BlockId::FLOWER_RED => write!(f, "Red Flower"),
            BlockId::FLOWER_YELLOW => write!(f, "Yellow Flower"),
            BlockId::CACTUS => write!(f, "Cactus"),
            BlockId::DEAD_BUSH => write!(f, "Dead Bush"),
            BlockId::MUSHROOM_RED => write!(f, "Red Mushroom"),
            BlockId::MUSHROOM_BROWN => write!(f, "Brown Mushroom"),
            BlockId::SUGAR_CANE => write!(f, "Sugar Cane"),
            BlockId::VINES => write!(f, "Vines"),
            BlockId::BRICK => write!(f, "Brick"),
            BlockId::GLOWSTONE => write!(f, "Glowstone"),
            _ => write!(f, "Block({})", self.0),
        }
    }
}

impl BlockId {
    pub const AIR: BlockId = BlockId(0);
    pub const GRASS: BlockId = BlockId(1);
    pub const DIRT: BlockId = BlockId(2);
    pub const STONE: BlockId = BlockId(3);
    pub const WOOD: BlockId = BlockId(4);
    pub const SAND: BlockId = BlockId(5);
    pub const WATER: BlockId = BlockId(6);
    pub const LEAVES: BlockId = BlockId(7);
    pub const GLASS: BlockId = BlockId(8);
    pub const COAL_ORE: BlockId = BlockId(9);
    pub const IRON_ORE: BlockId = BlockId(10);
    pub const GOLD_ORE: BlockId = BlockId(11);
    pub const DIAMOND_ORE: BlockId = BlockId(12);
    pub const BEDROCK: BlockId = BlockId(13);
    pub const PLANKS: BlockId = BlockId(14);
    pub const COBBLESTONE: BlockId = BlockId(15);
    pub const CRAFTING_TABLE: BlockId = BlockId(16);
    pub const FURNACE: BlockId = BlockId(17);
    pub const CHEST: BlockId = BlockId(18);
    pub const TORCH: BlockId = BlockId(19);
    pub const LADDER: BlockId = BlockId(20);
    pub const LAVA: BlockId = BlockId(21);
    pub const LOG: BlockId = BlockId(22);
    pub const SANDSTONE: BlockId = BlockId(23);
    pub const RED_SAND: BlockId = BlockId(24);
    pub const RED_SANDSTONE: BlockId = BlockId(25);
    pub const TALL_GRASS: BlockId = BlockId(26);
    pub const FLOWER_RED: BlockId = BlockId(27);
    pub const FLOWER_YELLOW: BlockId = BlockId(28);
    pub const CACTUS: BlockId = BlockId(29);
    pub const DEAD_BUSH: BlockId = BlockId(30);
    pub const MUSHROOM_RED: BlockId = BlockId(31);
    pub const MUSHROOM_BROWN: BlockId = BlockId(32);
    pub const SUGAR_CANE: BlockId = BlockId(33);
    pub const VINES: BlockId = BlockId(34);
    pub const BRICK: BlockId = BlockId(35);
    pub const GLOWSTONE: BlockId = BlockId(36);

    /// Create a new BlockId from a raw u16 value
    pub const fn new(id: u16) -> Self {
        BlockId(id)
    }
}

/// Data needed to render a block
#[derive(Debug, Clone, Copy)]
pub struct RenderData {
    pub color: [f32; 3],
    pub texture_id: u32,
    pub light_emission: u8,
}

/// Physical properties of a block
#[derive(Debug, Clone, Copy)]
pub struct PhysicsProperties {
    pub solid: bool,
    pub density: f32,
}

// Block trait has been removed in favor of data-oriented design
// See block_data.rs for the new BlockProperties system
// 
// OLD USAGE:
// impl Block for MyBlock { ... }
// 
// NEW USAGE:
// let properties = BlockProperties {
//     name: "myblock",
//     render_data: RenderData { ... },
//     physics: PhysicsProperties { ... },
//     transparent: false,
//     hardness: 1.0,
//     flammable: false,
//     blast_resistance: 3.0,
// };
// registry.register_block("mod:myblock", properties);
