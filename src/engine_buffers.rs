//! Central EngineBuffers - DOP Architecture Core
//!
//! This module defines the central data buffers that store ALL engine state.
//! Following strict DOP principles:
//! - Data lives here
//! - Functions operate on this data
//! - No methods, just pure data structures

use crate::world::core::{BlockId, ChunkPos, VoxelPos, PhysicsProperties, RenderData};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use parking_lot::RwLock;

/// Central engine buffers - the single source of truth for ALL engine state
///
/// DOP Architecture:
/// - All data lives in these buffers
/// - Operations modules (world_operations, physics_operations, etc.) transform this data
/// - No methods on this struct - pure data only
#[derive(Clone)]
pub struct EngineBuffers {
    /// World data buffer
    pub world: WorldBuffers,

    /// Rendering data buffer
    pub render: RenderBuffers,

    /// Physics data buffer
    pub physics: PhysicsBuffers,

    /// Input state buffer
    pub input: InputBuffers,

    /// Network data buffer
    pub network: NetworkBuffers,

    /// Particle system buffer
    pub particles: ParticleBuffers,

    /// Performance metrics buffer
    pub metrics: MetricsBuffers,
}

/// World state buffers
#[derive(Clone)]
pub struct WorldBuffers {
    /// Chunk data storage (SOA)
    pub chunks: Vec<ChunkBuffer>,

    /// Active chunk positions
    pub active_chunks: HashSet<ChunkPos>,

    /// Chunks pending generation
    pub pending_generation: VecDeque<ChunkPos>,

    /// Chunks dirty and needing remesh
    pub dirty_chunks: HashSet<ChunkPos>,

    /// World modifications queue
    pub modifications: VecDeque<WorldModification>,

    /// World size (in chunks)
    pub world_size: [u32; 3],

    /// World seed
    pub world_seed: u32,

    /// World tick counter
    pub world_tick: u64,

    /// Block registry
    pub block_registry: HashMap<BlockId, BlockProperties>,
}

/// Single chunk buffer (Structure of Arrays for cache efficiency)
#[derive(Clone)]
pub struct ChunkBuffer {
    /// Chunk position
    pub position: ChunkPos,

    /// Block IDs (flat array: 50x50x50 = 125,000 blocks)
    pub blocks: Vec<BlockId>,

    /// Light levels (combined sky + block light)
    pub light_levels: Vec<u8>,

    /// Metadata flags
    pub flags: ChunkFlags,

    /// Last modified tick
    pub last_modified: u64,
}

/// Chunk metadata flags
#[derive(Clone, Copy, Debug)]
pub struct ChunkFlags {
    pub is_generated: bool,
    pub is_meshed: bool,
    pub is_dirty: bool,
    pub is_empty: bool,
}

/// World modification record
#[derive(Clone, Debug)]
pub struct WorldModification {
    pub position: VoxelPos,
    pub old_block: BlockId,
    pub new_block: BlockId,
    pub timestamp: u64,
}

/// Block properties (from registry)
#[derive(Clone, Debug)]
pub struct BlockProperties {
    pub id: BlockId,
    pub name: String,
    pub is_solid: bool,
    pub is_transparent: bool,
    pub transparent: bool,
    pub light_emission: u8,
    pub physics_enabled: bool,
    pub physics: PhysicsProperties,
    pub render_data: RenderData,
    pub hardness: f32,
    pub flammable: bool,
    pub blast_resistance: f32,
}

/// Rendering state buffers
#[derive(Clone)]
pub struct RenderBuffers {
    /// Camera position
    pub camera_position: [f32; 3],

    /// Camera rotation (yaw, pitch, roll)
    pub camera_rotation: [f32; 3],

    /// View matrix (4x4, column-major)
    pub view_matrix: [f32; 16],

    /// Projection matrix (4x4, column-major)
    pub projection_matrix: [f32; 16],

    /// Visible chunks (from frustum culling)
    pub visible_chunks: HashSet<ChunkPos>,

    /// Mesh data per chunk
    pub chunk_meshes: HashMap<ChunkPos, MeshData>,

    /// Frame counter
    pub frame_count: u64,

    /// Delta time (seconds)
    pub delta_time: f32,

    /// Render statistics
    pub stats: RenderStats,
}

/// Mesh data for a chunk
#[derive(Clone)]
pub struct MeshData {
    /// Vertex positions (SOA)
    pub positions: Vec<[f32; 3]>,

    /// Vertex normals (SOA)
    pub normals: Vec<[f32; 3]>,

    /// Vertex UVs (SOA)
    pub uvs: Vec<[f32; 2]>,

    /// Indices
    pub indices: Vec<u32>,

    /// Vertex count
    pub vertex_count: u32,

    /// Triangle count
    pub triangle_count: u32,
}

/// Render statistics
#[derive(Clone, Copy, Default)]
pub struct RenderStats {
    pub triangles_rendered: u32,
    pub chunks_rendered: u32,
    pub draw_calls: u32,
    pub gpu_memory_used: u64,
}

/// Physics state buffers
#[derive(Clone)]
pub struct PhysicsBuffers {
    /// Entity count
    pub entity_count: u32,

    /// Entity positions (SOA)
    pub positions: Vec<[f32; 3]>,

    /// Entity velocities (SOA)
    pub velocities: Vec<[f32; 3]>,

    /// Entity accelerations (SOA)
    pub accelerations: Vec<[f32; 3]>,

    /// Entity AABBs (SOA)
    pub aabbs: Vec<AABB>,

    /// Entity flags (SOA)
    pub flags: Vec<PhysicsFlags>,

    /// Collision pairs
    pub collisions: Vec<CollisionPair>,

    /// Physics tick counter
    pub physics_tick: u64,

    /// Fixed timestep accumulator
    pub time_accumulator: f32,
}

/// Axis-Aligned Bounding Box
#[derive(Clone, Copy, Debug)]
pub struct AABB {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// Physics entity flags
#[derive(Clone, Copy, Debug)]
pub struct PhysicsFlags {
    pub is_static: bool,
    pub is_kinematic: bool,
    pub is_dynamic: bool,
    pub has_gravity: bool,
    pub is_grounded: bool,
}

/// Collision pair
#[derive(Clone, Copy, Debug)]
pub struct CollisionPair {
    pub entity_a: u32,
    pub entity_b: u32,
    pub penetration_depth: f32,
    pub normal: [f32; 3],
}

/// Input state buffers
#[derive(Clone, Default)]
pub struct InputBuffers {
    /// Keys currently pressed
    pub keys_down: HashSet<u32>,

    /// Keys pressed this frame
    pub keys_pressed: HashSet<u32>,

    /// Keys released this frame
    pub keys_released: HashSet<u32>,

    /// Mouse buttons down
    pub mouse_buttons_down: HashSet<u8>,

    /// Mouse position (screen space)
    pub mouse_position: [f32; 2],

    /// Mouse delta (this frame)
    pub mouse_delta: [f32; 2],

    /// Mouse scroll delta
    pub scroll_delta: f32,
}

/// Network state buffers
#[derive(Clone, Default)]
pub struct NetworkBuffers {
    /// Connected clients
    pub client_count: u32,

    /// Inbound packet queue
    pub inbound_packets: VecDeque<NetworkPacket>,

    /// Outbound packet queue
    pub outbound_packets: VecDeque<NetworkPacket>,

    /// Network statistics
    pub stats: NetworkStats,
}

/// Network packet
#[derive(Clone, Debug)]
pub struct NetworkPacket {
    pub client_id: u32,
    pub packet_type: u16,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

/// Network statistics
#[derive(Clone, Copy, Default)]
pub struct NetworkStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packets_dropped: u64,
}

/// Particle system buffers
#[derive(Clone, Default)]
pub struct ParticleBuffers {
    /// Active particle count
    pub particle_count: u32,

    /// Particle positions (SOA)
    pub positions: Vec<[f32; 3]>,

    /// Particle velocities (SOA)
    pub velocities: Vec<[f32; 3]>,

    /// Particle lifetimes (SOA)
    pub lifetimes: Vec<f32>,

    /// Particle ages (SOA)
    pub ages: Vec<f32>,

    /// Particle types (SOA)
    pub types: Vec<u16>,
}

/// Performance metrics buffers
#[derive(Clone, Default)]
pub struct MetricsBuffers {
    /// Frame times (circular buffer, last 100 frames)
    pub frame_times: VecDeque<f32>,

    /// CPU usage percentage
    pub cpu_usage: f32,

    /// Memory usage (bytes)
    pub memory_usage: u64,

    /// GPU memory usage (bytes)
    pub gpu_memory_usage: u64,

    /// Active threads
    pub thread_count: u32,
}

/// Thread-safe shared buffers (Arc<RwLock<>>)
pub type SharedEngineBuffers = Arc<RwLock<EngineBuffers>>;

/// Create new engine buffers with default values
pub fn create_engine_buffers() -> EngineBuffers {
    EngineBuffers {
        world: WorldBuffers {
            chunks: Vec::new(),
            active_chunks: HashSet::new(),
            pending_generation: VecDeque::new(),
            dirty_chunks: HashSet::new(),
            modifications: VecDeque::new(),
            world_size: [0, 0, 0],
            world_seed: 0,
            world_tick: 0,
            block_registry: HashMap::new(),
        },
        render: RenderBuffers {
            camera_position: [0.0, 0.0, 0.0],
            camera_rotation: [0.0, 0.0, 0.0],
            view_matrix: [0.0; 16],
            projection_matrix: [0.0; 16],
            visible_chunks: HashSet::new(),
            chunk_meshes: HashMap::new(),
            frame_count: 0,
            delta_time: 0.0,
            stats: RenderStats::default(),
        },
        physics: PhysicsBuffers {
            entity_count: 0,
            positions: Vec::new(),
            velocities: Vec::new(),
            accelerations: Vec::new(),
            aabbs: Vec::new(),
            flags: Vec::new(),
            collisions: Vec::new(),
            physics_tick: 0,
            time_accumulator: 0.0,
        },
        input: InputBuffers::default(),
        network: NetworkBuffers::default(),
        particles: ParticleBuffers::default(),
        metrics: MetricsBuffers::default(),
    }
}

/// Create thread-safe shared buffers
pub fn create_shared_buffers() -> SharedEngineBuffers {
    Arc::new(RwLock::new(create_engine_buffers()))
}

impl Default for EngineBuffers {
    fn default() -> Self {
        create_engine_buffers()
    }
}

impl Default for ChunkFlags {
    fn default() -> Self {
        Self {
            is_generated: false,
            is_meshed: false,
            is_dirty: false,
            is_empty: true,
        }
    }
}

impl Default for PhysicsFlags {
    fn default() -> Self {
        Self {
            is_static: false,
            is_kinematic: false,
            is_dynamic: true,
            has_gravity: true,
            is_grounded: false,
        }
    }
}
