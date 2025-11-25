use crate::gpu::buffer_layouts::ChunkMetadata;
use bytemuck::{Pod, Zeroable};
/// Pure Buffer-Based World State
///
/// No objects, no methods, just data transformations and GPU buffers.
///
/// This is the culmination of the data-oriented journey:
/// - All world data lives in contiguous buffers
/// - All operations are pure functions
/// - GPU owns the data, CPU just coordinates
/// - Zero allocations in steady state
use std::sync::Arc;
use wgpu::{Buffer, Device, Queue};

/// Complete world state - all game data in one place
pub struct WorldState {
    /// GPU device reference
    pub device: Arc<Device>,

    /// === Voxel Data ===
    pub world_buffer: Arc<Buffer>,
    pub chunk_metadata: Arc<Buffer>,

    /// === Entity Data ===
    /// All entities stored in SoA layout
    pub entity_positions: Arc<Buffer>,
    pub entity_velocities: Arc<Buffer>,
    pub entity_attributes: Arc<Buffer>,
    pub entity_metadata: Arc<Buffer>,

    /// === Physics Data ===
    /// Physics bodies and collision data
    pub physics_bodies: Arc<Buffer>,
    pub collision_pairs: Arc<Buffer>,
    pub spatial_hash: Arc<Buffer>,

    /// === Rendering Data ===
    /// Mesh and instance data
    pub mesh_vertices: Arc<Buffer>,
    pub mesh_indices: Arc<Buffer>,
    pub instance_transforms: Arc<Buffer>,
    pub draw_commands: Arc<Buffer>,

    /// === Fluid Data ===
    /// Fluid simulation state
    pub fluid_cells: Arc<Buffer>,
    pub fluid_pressure: Arc<Buffer>,
    pub fluid_velocity: Arc<Buffer>,

    /// === Lighting Data ===
    /// Light propagation data
    pub light_sources: Arc<Buffer>,
    pub light_values: Arc<Buffer>,
    pub ao_values: Arc<Buffer>,

    /// === Network Data ===
    /// Packet buffers for GPU networking
    pub outgoing_packets: Arc<Buffer>,
    pub incoming_packets: Arc<Buffer>,

    /// === Metadata ===
    pub frame_number: u64,
    pub delta_time_ms: u32,
    pub active_chunks: u32,
    pub entity_count: u32,
}

/// World state configuration
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct WorldConfig {
    pub world_size: u32,
    pub chunk_size: u32,
    pub max_entities: u32,
    pub max_chunks: u32,
    pub view_distance: u32,
    pub physics_substeps: u32,
    pub network_tick_rate: u32,
    _padding: u32,
}

/// Frame update parameters
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct FrameParams {
    pub frame_number: u64,
    pub delta_time_ms: u32,
    pub player_position: [f32; 3],
    pub player_rotation: [f32; 2],
    pub input_flags: u32,
    pub random_seed: u32,
    _padding: [u32; 2],
}

/// Pure functional world operations
pub mod operations {
    use super::*;
    use crate::memory::{MemoryManager, MemoryResult};

    /// Initialize world state with all buffers
    pub fn init_world_state(
        device: Arc<Device>,
        config: &WorldConfig,
        memory_manager: &mut MemoryManager,
    ) -> MemoryResult<WorldState> {
        // Calculate buffer sizes
        let voxels_per_chunk = config.chunk_size * config.chunk_size * config.chunk_size;
        let world_buffer_size = (config.max_chunks * voxels_per_chunk * 4) as u64;
        let entity_buffer_size = (config.max_entities * 64) as u64; // 64 bytes per entity component

        // Allocate all buffers through memory manager
        Ok(WorldState {
            device: device.clone(),

            // Voxel data
            world_buffer: memory_manager
                .alloc_buffer(
                    world_buffer_size,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                )?
                .buffer_arc(),

            chunk_metadata: memory_manager
                .alloc_buffer(
                    (config.max_chunks * 64) as u64,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                )?
                .buffer_arc(),

            // Entity data
            entity_positions: memory_manager
                .alloc_buffer(
                    entity_buffer_size,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
                )?
                .buffer_arc(),

            entity_velocities: memory_manager
                .alloc_buffer(entity_buffer_size, wgpu::BufferUsages::STORAGE)?
                .buffer_arc(),

            entity_attributes: memory_manager
                .alloc_buffer(
                    entity_buffer_size * 4, // More space for attributes
                    wgpu::BufferUsages::STORAGE,
                )?
                .buffer_arc(),

            entity_metadata: memory_manager
                .alloc_buffer(entity_buffer_size, wgpu::BufferUsages::STORAGE)?
                .buffer_arc(),

            // Physics data
            physics_bodies: memory_manager
                .alloc_buffer(entity_buffer_size * 2, wgpu::BufferUsages::STORAGE)?
                .buffer_arc(),

            collision_pairs: memory_manager
                .alloc_buffer(
                    crate::constants::buffer_sizes::COLLISION_PAIRS_BUFFER_SIZE,
                    wgpu::BufferUsages::STORAGE,
                )?
                .buffer_arc(),

            spatial_hash: memory_manager
                .alloc_buffer(
                    crate::constants::buffer_sizes::SPATIAL_HASH_BUFFER_SIZE,
                    wgpu::BufferUsages::STORAGE,
                )?
                .buffer_arc(),

            // Rendering data
            mesh_vertices: memory_manager
                .alloc_buffer(
                    crate::constants::buffer_sizes::VERTEX_BUFFER_SIZE,
                    wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                )?
                .buffer_arc(),

            mesh_indices: memory_manager
                .alloc_buffer(
                    crate::constants::buffer_sizes::INDEX_BUFFER_SIZE,
                    wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                )?
                .buffer_arc(),

            instance_transforms: memory_manager
                .alloc_buffer(
                    entity_buffer_size,
                    wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
                )?
                .buffer_arc(),

            draw_commands: memory_manager
                .alloc_buffer(
                    crate::constants::buffer_sizes::INDIRECT_COMMANDS_BUFFER_SIZE,
                    wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::STORAGE,
                )?
                .buffer_arc(),

            // Fluid data
            fluid_cells: memory_manager
                .alloc_buffer(
                    crate::constants::buffer_sizes::FLUID_CELLS_BUFFER_SIZE,
                    wgpu::BufferUsages::STORAGE,
                )?
                .buffer_arc(),

            fluid_pressure: memory_manager
                .alloc_buffer(crate::constants::buffer_sizes::FLUID_PRESSURE_BUFFER_SIZE, wgpu::BufferUsages::STORAGE)?
                .buffer_arc(),

            fluid_velocity: memory_manager
                .alloc_buffer(crate::constants::buffer_sizes::FLUID_VELOCITY_BUFFER_SIZE, wgpu::BufferUsages::STORAGE)?
                .buffer_arc(),

            // Lighting data
            light_sources: memory_manager
                .alloc_buffer(
                    crate::constants::buffer_sizes::LIGHT_SOURCES_BUFFER_SIZE,
                    wgpu::BufferUsages::STORAGE,
                )?
                .buffer_arc(),

            light_values: memory_manager
                .alloc_buffer(
                    world_buffer_size / 4, // 1 byte per voxel
                    wgpu::BufferUsages::STORAGE,
                )?
                .buffer_arc(),

            ao_values: memory_manager
                .alloc_buffer(world_buffer_size / 4, wgpu::BufferUsages::STORAGE)?
                .buffer_arc(),

            // Network data
            outgoing_packets: memory_manager
                .alloc_buffer(
                    crate::constants::buffer_sizes::PACKET_BUFFER_SIZE,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                )?
                .buffer_arc(),

            incoming_packets: memory_manager
                .alloc_buffer(
                    crate::constants::buffer_sizes::PACKET_BUFFER_SIZE,
                    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                )?
                .buffer_arc(),

            // Initial metadata
            frame_number: 0,
            delta_time_ms: 16,
            active_chunks: 0,
            entity_count: 0,
        })
    }

    /// Update world state for a frame (GPU dispatch)
    pub fn update_frame(
        state: &mut WorldState,
        queue: &Queue,
        encoder: &mut wgpu::CommandEncoder,
        params: &FrameParams,
        unified_kernel: &crate::world::compute::UnifiedWorldKernel,
    ) {
        // Update frame metadata
        state.frame_number = params.frame_number;
        state.delta_time_ms = params.delta_time_ms;

        // Single unified kernel updates everything
        let config = crate::world::compute::UnifiedKernelConfig {
            frame_number: params.frame_number as u32,
            delta_time_ms: params.delta_time_ms,
            world_size: crate::constants::core::DEFAULT_WORLD_SIZE, // Default world size in chunks
            active_chunks: state.active_chunks,
            physics_substeps: 4,
            lighting_iterations: 2,
            system_flags: crate::world::compute::SystemFlags::ALL,
            random_seed: params.random_seed,
        };

        // One dispatch to rule them all
        unified_kernel.update_world(queue, encoder, config, 256);
    }

    /// Create bind groups for rendering
    pub fn create_render_bind_groups(
        state: &WorldState,
        device: &Device,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("World State Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: state.world_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: state.instance_transforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: state.light_values.as_entire_binding(),
                },
                // Add more bindings as needed
            ],
        })
    }
}

/// Buffer views for CPU access (debugging/tools only)
pub mod views {
    use crate::gpu::buffer_layouts::ChunkMetadata;

    /// Read-only view of entity positions
    pub struct EntityPositionView<'a> {
        pub data: &'a [[f32; 3]],
        pub count: usize,
    }

    /// Read-only view of chunk metadata
    pub struct ChunkMetadataView<'a> {
        pub data: &'a [ChunkMetadata],
        pub count: usize,
    }

    // Note: In production, CPU rarely needs to read GPU data
    // These views are mainly for debugging and tools
}

/// Performance metrics
#[derive(Default, Debug)]
pub struct WorldStateMetrics {
    pub frame_time_us: u64,
    pub gpu_time_us: u64,
    pub entity_count: u32,
    pub active_chunks: u32,
    pub triangles_rendered: u64,
    pub bandwidth_gb_per_sec: f32,
}

// This is it - the entire game state in pure data buffers.
// No classes, no methods, no allocations, just data and transformations.
// The CPU is now just a thin coordination layer over massive GPU compute.
