//! Game Module - Pure DOP Interface
//!
//! This module provides the game/engine interface using Data-Oriented Programming.
//! All functions are pure: take data, return results, no side effects.

use crate::camera::{calculate_forward_vector, CameraData};
use crate::input::InputState;
use crate::{BlockId, BlockRegistry, Ray, RaycastHit, VoxelPos};
use crate::world::{world_operations, data_types::WorldData};
use cgmath::{Point3, InnerSpace};

// Gateway modules (DOP system)
pub mod gateway_data;
pub mod gateway_operations;

// Re-export gateway types
pub use gateway_data::{
    GameEvent, GameCommand, GameOperations, GameDataAccess, GameDataHandle,
    InteractionType, MessageType, BlockRegistration, BlockProperties,
    EngineStateView, InputStateView, WorldInfoView, PlayerInfo,
    GameGatewayData, GatewayConfig, GatewayMetrics,
};

pub use gateway_operations::{
    init_gateway, shutdown_gateway, queue_event, queue_events,
    process_update, register_blocks, get_active_block,
    save_game_state, load_game_state, get_metrics, reset_metrics,
    is_gateway_initialized, get_gateway_config, update_gateway_config,
};

/// Game data structure (DOP - no methods)
/// Pure data structure for game state
pub trait GameData: Send + Sync + 'static {}

/// DOP game context that uses engine buffers directly
pub struct GameContextDOP<'a> {
    pub buffers: &'a mut crate::EngineBuffers,
    pub registry: &'a BlockRegistry,
    pub selected_block: Option<RaycastHit>,
    pub chunk_size: u32,
}

/// Register blocks in the registry
/// Function - transforms registry data by registering game blocks
pub fn register_game_blocks<T: GameData + 'static>(_game: &mut T, registry: &mut BlockRegistry) {
    if is_gateway_initialized() {
        register_blocks(registry);
    }
}

/// Update game state using DOP buffers
/// Function - transforms game data using centralized buffers
pub fn update_game_dop<T: GameData + 'static>(
    _game: &mut T,
    buffers: &mut crate::EngineBuffers,
    registry: &BlockRegistry,
    _delta_time: f32,
) {
    let _ctx = GameContextDOP {
        buffers,
        registry,
        selected_block: None,
        chunk_size: 50,
    };
    // Game-specific updates handled by gateway
}

/// Handle block break event
/// Function - processes block break for game data
pub fn handle_block_break<T: GameData + 'static>(_game: &mut T, pos: VoxelPos, block: BlockId) {
    if is_gateway_initialized() {
        queue_event(GameEvent::BlockBreak {
            position: pos,
            block_id: block,
            player_id: None,
        });
    }
}

/// Handle block place event
/// Function - processes block place for game data
pub fn handle_block_place<T: GameData + 'static>(_game: &mut T, pos: VoxelPos, block: BlockId) {
    if is_gateway_initialized() {
        queue_event(GameEvent::BlockPlace {
            position: pos,
            block_id: block,
            player_id: None,
        });
    }
}

/// Get the active block for placement
/// Pure function - reads active block from game data
pub fn get_active_block_from_game<T: GameData + 'static>(_game: &T) -> BlockId {
    if is_gateway_initialized() {
        get_active_block()
    } else {
        BlockId::GRASS
    }
}

// ============================================================================
// DOP World Operations - Pure functions operating on WorldData
// ============================================================================

/// Cast a ray from the camera using DOP
/// Pure function - calculates raycast using world data
pub fn cast_camera_ray_dop(
    world: &WorldData,
    camera: &CameraData,
    max_distance: f32,
    chunk_size: u32,
) -> Option<RaycastHit> {
    let position = Point3::new(
        camera.position[0],
        camera.position[1],
        camera.position[2],
    );
    let forward = calculate_forward_vector(camera.yaw_radians, camera.pitch_radians);
    let ray = Ray::new(position, forward);

    world_operations::raycast(world, ray, max_distance, chunk_size)
}

/// Break a block at the given position
/// Function - transforms world data by breaking block
pub fn break_block_dop(
    world: &mut WorldData,
    pos: VoxelPos,
    chunk_size: u32,
) -> Result<bool, crate::world::error::WorldError> {
    let block = world_operations::get_block(world, pos, chunk_size);
    if block != BlockId::AIR {
        world_operations::set_block(world, pos, BlockId::AIR, chunk_size)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Place a block at the given position
/// Function - transforms world data by placing block
pub fn place_block_dop(
    world: &mut WorldData,
    pos: VoxelPos,
    block_id: BlockId,
    chunk_size: u32,
) -> Result<bool, crate::world::error::WorldError> {
    let current = world_operations::get_block(world, pos, chunk_size);
    if current == BlockId::AIR {
        world_operations::set_block(world, pos, block_id, chunk_size)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Get block at position
/// Pure function - reads block from world data
pub fn get_block_dop(world: &WorldData, pos: VoxelPos, chunk_size: u32) -> BlockId {
    world_operations::get_block(world, pos, chunk_size)
}

/// Check if chunk is loaded
/// Pure function - reads chunk state from world data
pub fn is_chunk_loaded_dop(world: &WorldData, chunk_pos: crate::ChunkPos) -> bool {
    world_operations::is_chunk_loaded(world, chunk_pos)
}
