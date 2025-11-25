//! Game Gateway Data - Event Queue System
//!
//! This is the OPTIONAL event queue for games that want async/decoupled interaction.
//! Games can bypass this entirely and call engine operations directly.
//!
//! Pure DOP: No methods, just data structures.

use crate::world::core::{BlockId, VoxelPos};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Game event - things that happen in the game world
#[derive(Clone, Debug)]
pub enum GameEvent {
    /// Player breaks a block
    BlockBreak {
        position: VoxelPos,
        block_id: BlockId,
        player_id: Option<u32>,
    },

    /// Player places a block
    BlockPlace {
        position: VoxelPos,
        block_id: BlockId,
        player_id: Option<u32>,
    },

    /// Player interacts with a block
    BlockInteract {
        position: VoxelPos,
        block_id: BlockId,
        interaction_type: InteractionType,
        player_id: Option<u32>,
    },

    /// Player moves
    PlayerMove {
        player_id: u32,
        from_position: [f32; 3],
        to_position: [f32; 3],
    },

    /// Player joins game
    PlayerJoin {
        player_id: u32,
        player_name: String,
    },

    /// Player leaves game
    PlayerLeave {
        player_id: u32,
    },

    /// Apply force to physics entity
    ApplyForce {
        entity_id: u32,
        force: [f32; 3],
    },

    /// Spawn particle effect
    SpawnParticle {
        position: [f32; 3],
        particle_type: u16,
        count: u32,
    },

    /// Custom game event
    Custom {
        event_type: String,
        data: Vec<u8>,
    },
}

/// Game command - instructions from game to engine
#[derive(Clone, Debug)]
pub enum GameCommand {
    /// Load a chunk
    LoadChunk {
        position: VoxelPos,
    },

    /// Unload a chunk
    UnloadChunk {
        position: VoxelPos,
    },

    /// Save world state
    SaveWorld {
        path: String,
    },

    /// Load world state
    LoadWorld {
        path: String,
    },

    /// Change game settings
    UpdateSettings {
        setting_name: String,
        value: String,
    },

    /// Shutdown game
    Shutdown,
}

/// Block interaction types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionType {
    LeftClick,
    RightClick,
    MiddleClick,
    Use,
    Activate,
}

/// Message types for game communication
#[derive(Clone, Debug)]
pub enum MessageType {
    Info(String),
    Warning(String),
    Error(String),
    Success(String),
    Debug(String),
}

/// Block registration data
#[derive(Clone, Debug)]
pub struct BlockRegistration {
    pub id: BlockId,
    pub name: String,
    pub properties: BlockProperties,
}

/// Block properties for game logic
#[derive(Clone, Debug)]
pub struct BlockProperties {
    pub is_solid: bool,
    pub is_transparent: bool,
    pub is_liquid: bool,
    pub light_emission: u8,
    pub hardness: f32,
    pub friction: f32,
    pub can_interact: bool,
}

/// Engine state view for game read access
#[derive(Clone, Debug)]
pub struct EngineStateView {
    pub frame_count: u64,
    pub delta_time: f32,
    pub is_paused: bool,
    pub world_tick: u64,
}

/// Input state view for game read access
#[derive(Clone, Debug, Default)]
pub struct InputStateView {
    pub keys_down: Vec<u32>,
    pub mouse_position: [f32; 2],
    pub mouse_delta: [f32; 2],
}

/// World info view for game read access
#[derive(Clone, Debug)]
pub struct WorldInfoView {
    pub loaded_chunks: usize,
    pub active_entities: usize,
    pub world_seed: u32,
}

/// Player information
#[derive(Clone, Debug)]
pub struct PlayerInfo {
    pub player_id: u32,
    pub player_name: String,
    pub position: [f32; 3],
    pub health: f32,
    pub active_block: BlockId,
}

/// Game gateway data - the event queue state
pub struct GameGatewayData {
    /// Pending events to process
    pub pending_events: VecDeque<GameEvent>,

    /// Pending commands to execute
    pub pending_commands: VecDeque<GameCommand>,

    /// Messages to display
    pub messages: VecDeque<MessageType>,

    /// Gateway configuration
    pub config: GatewayConfig,

    /// Gateway metrics
    pub metrics: GatewayMetrics,

    /// Is gateway initialized
    pub initialized: bool,

    /// Active block for placement
    pub active_block: BlockId,

    /// Registered blocks
    pub registered_blocks: Vec<BlockRegistration>,
}

/// Gateway configuration
#[derive(Clone, Debug)]
pub struct GatewayConfig {
    /// Maximum events in queue before dropping
    pub max_queue_size: usize,

    /// Process events every N frames
    pub process_interval: u32,

    /// Enable debug logging
    pub debug_logging: bool,

    /// Enable event recording for replay
    pub record_events: bool,
}

/// Gateway metrics for monitoring
#[derive(Clone, Copy, Debug, Default)]
pub struct GatewayMetrics {
    /// Total events processed
    pub events_processed: u64,

    /// Total commands executed
    pub commands_executed: u64,

    /// Events dropped (queue full)
    pub events_dropped: u64,

    /// Average processing time (microseconds)
    pub avg_process_time_us: f32,

    /// Peak queue size
    pub peak_queue_size: usize,
}

/// Game operations trait - games can implement custom handlers
pub trait GameOperations: Send + Sync {
    /// Handle a game event
    fn handle_event(&mut self, event: &GameEvent);

    /// Execute a game command
    fn execute_command(&mut self, command: &GameCommand);

    /// Update game logic
    fn update(&mut self, delta_time: f32);
}

/// Game data access trait - engine provides this to games
pub trait GameDataAccess {
    /// Get engine state view
    fn get_engine_state(&self) -> EngineStateView;

    /// Get input state view
    fn get_input_state(&self) -> InputStateView;

    /// Get world info view
    fn get_world_info(&self) -> WorldInfoView;
}

/// Handle to game data (type-erased for flexibility)
pub type GameDataHandle = Arc<Mutex<dyn std::any::Any + Send + Sync>>;

impl Default for GameGatewayData {
    fn default() -> Self {
        Self {
            pending_events: VecDeque::new(),
            pending_commands: VecDeque::new(),
            messages: VecDeque::new(),
            config: GatewayConfig::default(),
            metrics: GatewayMetrics::default(),
            initialized: false,
            active_block: BlockId(1), // Default to first block
            registered_blocks: Vec::new(),
        }
    }
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10000,
            process_interval: 1,
            debug_logging: false,
            record_events: false,
        }
    }
}

impl Default for BlockProperties {
    fn default() -> Self {
        Self {
            is_solid: true,
            is_transparent: false,
            is_liquid: false,
            light_emission: 0,
            hardness: 1.0,
            friction: 0.5,
            can_interact: false,
        }
    }
}
