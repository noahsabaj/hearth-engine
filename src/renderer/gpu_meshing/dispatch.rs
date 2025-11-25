//! GPU mesh generation dispatch - pure functions for executing mesh generation

use crate::renderer::gpu_meshing::{
    GpuMeshBuffer, GpuMeshingState, MeshRequest, MeshingParams, MAX_CONCURRENT_MESHES,
    WORKGROUP_SIZE,
};
use crate::world::core::ChunkPos;

// Import constants properly
use crate::constants::*;

/// Mesh generation result
pub struct MeshGenerationResult {
    pub chunk_pos: ChunkPos,
    pub buffer_index: u32,
    // Indirect commands are stored in the global indirect buffer at offset buffer_index * 20 bytes
}

/// Generate meshes for a batch of chunks
pub fn generate_chunk_meshes(
    state: &GpuMeshingState,
    world_buffer: &wgpu::Buffer,
    chunk_positions: &[ChunkPos],
    lod_level: u32,
) -> Vec<MeshGenerationResult> {
    log::info!(
        "[GPU Meshing] generate_chunk_meshes called with {} chunks",
        chunk_positions.len()
    );

    if chunk_positions.is_empty() {
        return Vec::new();
    }

    let batch_size = chunk_positions.len().min(MAX_CONCURRENT_MESHES);
    let chunks = &chunk_positions[..batch_size];

    // Allocate buffer indices and create mesh requests
    let mut allocated_indices = Vec::new();
    let mut requests = Vec::new();

    let allocator = match state.allocator.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::warn!("[GPU Meshing] Allocator mutex was poisoned in generate_chunk_meshes, recovering");
            poisoned.into_inner()
        }
    };
    // Note: allocator is not mutated, only read
    let _ = &allocator; // Suppress unused warning if needed

    for chunk_pos in chunks {
        // For GPU-driven rendering, all chunks use buffer 0
        let buffer_index = 0u32;
        
        log::debug!(
            "[generate_chunk_meshes] Using buffer 0 for chunk {:?}",
            chunk_pos
        );

        allocated_indices.push((chunk_pos, buffer_index));
        requests.push(MeshRequest {
            chunk_pos: [chunk_pos.x, chunk_pos.y, chunk_pos.z],
            lod_level,
            buffer_index,
            flags: 0,
            _padding: [0; 2],
        });
    }

    // Create request buffer
    let request_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Mesh Request Buffer"),
        size: (std::mem::size_of::<MeshRequest>() * requests.len()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Upload requests
    state
        .queue
        .write_buffer(&request_buffer, 0, bytemuck::cast_slice(&requests));

    // Create parameters
    let params = MeshingParams {
        chunk_size: core::CHUNK_SIZE,
        request_count: requests.len() as u32,
        enable_greedy: 1,
        enable_ao: 1,
        max_vertices: super::MAX_VERTICES_PER_CHUNK as u32,
        max_indices: super::MAX_INDICES_PER_CHUNK as u32,
        _padding: [0; 2],
    };

    let params_buffer = state.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Meshing Parameters"),
        size: std::mem::size_of::<MeshingParams>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    state
        .queue
        .write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

    // For GPU-driven rendering, we want all chunks to write to buffer 0
    // This allows us to render all chunks in a single draw call
    let bind_group = super::pipeline::create_mesh_bind_group_for_buffer(
        &state.device,
        &state.bind_group_layout,
        world_buffer,
        &request_buffer,
        &state.mesh_buffers,
        &state.indirect_buffer,
        &params_buffer,
        0, // Always use buffer 0 for merged rendering
    );

    // Create command encoder
    let mut encoder = state
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Mesh Generation Encoder"),
        });

    // One workgroup per chunk
    let workgroups = requests.len() as u32;

    // Dispatch compute
    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Mesh Generation Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&state.mesh_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        compute_pass.dispatch_workgroups(workgroups, 1, 1);
    }

    // Submit
    state.queue.submit(std::iter::once(encoder.finish()));

    log::info!(
        "[GPU Meshing] Dispatched mesh generation for {} chunks with {} workgroups",
        requests.len(),
        workgroups
    );

    // Force GPU synchronization to ensure mesh generation completes
    // This is a temporary debug measure to ensure GPU work is done
    state.device.poll(wgpu::Maintain::Wait);

    log::info!("[GPU Meshing] GPU synchronization complete - meshes should be ready");

    // Return mesh generation results using the allocated buffer indices
    // Note: indirect commands are written to the global indirect buffer by the GPU
    allocated_indices
        .iter()
        .map(|(chunk_pos, buffer_index)| {
            MeshGenerationResult {
                chunk_pos: **chunk_pos,
                buffer_index: *buffer_index,
                // Indirect commands are stored in the global indirect buffer at offset buffer_index * 20
            }
        })
        .collect()
}

/// Get mesh statistics from GPU
pub fn update_mesh_statistics(state: &mut GpuMeshingState, generated_count: u32) {
    state.stats.total_meshes += generated_count as u64;
    // TODO: Read back actual vertex/index counts from GPU
}

/// Check if mesh buffer is ready
pub fn is_mesh_ready(state: &GpuMeshingState, buffer_index: u32) -> bool {
    // In a real implementation, would check GPU fence/query
    buffer_index < state.mesh_buffers.len() as u32
}

/// Get mesh buffer for rendering
pub fn get_mesh_buffer<'a>(
    state: &'a GpuMeshingState,
    buffer_index: u32,
) -> Option<&'a GpuMeshBuffer> {
    state.mesh_buffers.get(buffer_index as usize)
}

/// Free a mesh buffer when a chunk is unloaded
pub fn free_mesh_buffer(state: &GpuMeshingState, chunk_pos: &ChunkPos) {
    let mut allocator = match state.allocator.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::warn!("[GPU Meshing] Allocator mutex was poisoned in free_mesh_buffer, recovering");
            poisoned.into_inner()
        }
    };
    if let Some(buffer_index) = allocator.allocated_buffers.remove(chunk_pos) {
        allocator.free_buffers.push(buffer_index);
        allocator.free_buffers.sort(); // Keep in order
    }
}

/// Clear mesh buffer pool
pub fn clear_mesh_buffers(state: &GpuMeshingState) {
    let mut allocator = match state.allocator.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            log::warn!("[GPU Meshing] Allocator mutex was poisoned in clear_mesh_buffers, recovering");
            poisoned.into_inner()
        }
    };
    // Return all allocated buffers to the free pool
    let buffer_indices: Vec<u32> = allocator
        .allocated_buffers
        .drain()
        .map(|(_, idx)| idx)
        .collect();
    for buffer_index in buffer_indices {
        allocator.free_buffers.push(buffer_index);
    }
    allocator.free_buffers.sort(); // Keep in order for easier debugging
}
