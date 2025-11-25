//! SOA Mesh Builder Operations - Pure DOP Functions
//!
//! All functions are pure: take data, return results, no side effects.
//! No methods, no self, just transformations.

use super::soa_mesh_builder_data::{GreedyMeshBuilderSoAData, MeshBuilderSoAData, MeshBuilderStats};
use super::vertex_soa_data::VertexBufferSoAData;
use super::vertex_soa_operations;
use crate::BlockId;
use std::collections::HashMap;

/// Initialize block colors lookup
pub fn init_block_colors() -> HashMap<BlockId, [f32; 3]> {
    let mut colors = HashMap::new();
    colors.insert(BlockId::AIR, [0.0, 0.0, 0.0]);
    colors.insert(BlockId::GRASS, [0.4, 0.8, 0.2]);
    colors.insert(BlockId::DIRT, [0.6, 0.4, 0.2]);
    colors.insert(BlockId::STONE, [0.5, 0.5, 0.5]);
    colors.insert(BlockId::WOOD, [0.6, 0.3, 0.1]);
    colors.insert(BlockId::SAND, [0.9, 0.8, 0.6]);
    colors.insert(BlockId::WATER, [0.2, 0.4, 0.8]);
    colors.insert(BlockId::LAVA, [1.0, 0.3, 0.0]);
    colors
}

/// Create new mesh builder data
pub fn create_mesh_builder() -> MeshBuilderSoAData {
    MeshBuilderSoAData {
        positions: Vec::new(),
        colors: Vec::new(),
        normals: Vec::new(),
        light_levels: Vec::new(),
        ao_values: Vec::new(),
        indices: Vec::new(),
        temp_positions: Vec::new(),
        temp_normals: Vec::new(),
        temp_colors: Vec::new(),
        face_visibility: Vec::new(),
        block_colors: init_block_colors(),
    }
}

/// Clear all mesh data
pub fn clear(data: &mut MeshBuilderSoAData) {
    data.positions.clear();
    data.colors.clear();
    data.normals.clear();
    data.light_levels.clear();
    data.ao_values.clear();
    data.indices.clear();

    // Keep temp arrays allocated but clear them
    data.temp_positions.clear();
    data.temp_normals.clear();
    data.temp_colors.clear();
    data.face_visibility.clear();
}

/// Reserve capacity for expected vertex count
pub fn reserve(data: &mut MeshBuilderSoAData, vertex_count: usize) {
    data.positions.reserve(vertex_count);
    data.colors.reserve(vertex_count);
    data.normals.reserve(vertex_count);
    data.light_levels.reserve(vertex_count);
    data.ao_values.reserve(vertex_count);
    data.indices.reserve(vertex_count / 4 * 6); // Rough estimate for quads
}

/// Add a quad to the mesh (cache-friendly batch operation)
pub fn add_quad_soa(
    data: &mut MeshBuilderSoAData,
    quad_positions: [[f32; 3]; 4],
    normal: [f32; 3],
    block_id: BlockId,
    light: f32,
    ao_values: [f32; 4],
) {
    let base_index = data.positions.len() as u32;
    let color = data
        .block_colors
        .get(&block_id)
        .copied()
        .unwrap_or([1.0, 0.0, 1.0]);

    // Add vertices in batch (cache-friendly)
    for i in 0..4 {
        data.positions.push(quad_positions[i]);
        data.colors.push(color);
        data.normals.push(normal);
        data.light_levels.push(light);
        data.ao_values.push(ao_values[i]);
    }

    // Add indices for two triangles
    data.indices.extend_from_slice(&[
        base_index,
        base_index + 1,
        base_index + 2,
        base_index,
        base_index + 2,
        base_index + 3,
    ]);
}

/// Batch add multiple quads (more cache-efficient)
pub fn add_quads_batch<I>(data: &mut MeshBuilderSoAData, quads: I)
where
    I: Iterator<Item = ([[f32; 3]; 4], [f32; 3], BlockId, f32, [f32; 4])>,
{
    // Collect into temporary arrays first for better memory access patterns
    data.temp_positions.clear();
    data.temp_normals.clear();
    data.temp_colors.clear();

    let mut temp_light_levels = Vec::new();
    let mut temp_ao_values = Vec::new();
    let mut temp_indices = Vec::new();

    for (i, (quad_positions, normal, block_id, light, ao_values)) in quads.enumerate() {
        let base_index = (data.positions.len() + i * 4) as u32;
        let color = data
            .block_colors
            .get(&block_id)
            .copied()
            .unwrap_or([1.0, 0.0, 1.0]);

        // Collect vertices
        for j in 0..4 {
            data.temp_positions.push(quad_positions[j]);
            data.temp_normals.push(normal);
            data.temp_colors.push(color);
            temp_light_levels.push(light);
            temp_ao_values.push(ao_values[j]);
        }

        // Collect indices
        temp_indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);
    }

    // Batch append to main arrays (better cache behavior)
    data.positions.extend_from_slice(&data.temp_positions);
    data.colors.extend_from_slice(&data.temp_colors);
    data.normals.extend_from_slice(&data.temp_normals);
    data.light_levels.extend_from_slice(&temp_light_levels);
    data.ao_values.extend_from_slice(&temp_ao_values);
    data.indices.extend_from_slice(&temp_indices);
}

/// Convert to VertexBufferSoA for GPU upload
pub fn build_vertex_buffer(data: &MeshBuilderSoAData) -> VertexBufferSoAData {
    let mut vertex_buffer = vertex_soa_operations::create_vertex_buffer_soa();

    // Batch copy all vertex data
    for i in 0..data.positions.len() {
        vertex_soa_operations::push_vertex(
            &mut vertex_buffer,
            data.positions[i],
            data.colors[i],
            data.normals[i],
            data.light_levels[i],
            data.ao_values[i],
        );
    }

    vertex_buffer
}

/// Get current vertex count
pub fn vertex_count(data: &MeshBuilderSoAData) -> usize {
    data.positions.len()
}

/// Get current index count
pub fn index_count(data: &MeshBuilderSoAData) -> usize {
    data.indices.len()
}

/// Get memory statistics
pub fn memory_stats(data: &MeshBuilderSoAData) -> MeshBuilderStats {
    MeshBuilderStats {
        vertex_count: vertex_count(data),
        index_count: index_count(data),
        positions_bytes: data.positions.len() * std::mem::size_of::<[f32; 3]>(),
        colors_bytes: data.colors.len() * std::mem::size_of::<[f32; 3]>(),
        normals_bytes: data.normals.len() * std::mem::size_of::<[f32; 3]>(),
        light_bytes: data.light_levels.len() * std::mem::size_of::<f32>(),
        ao_bytes: data.ao_values.len() * std::mem::size_of::<f32>(),
        indices_bytes: data.indices.len() * std::mem::size_of::<u32>(),
    }
}

// ============================================================================
// GREEDY MESH BUILDER OPERATIONS
// ============================================================================

/// Create new greedy mesh builder data
pub fn create_greedy_mesh_builder(chunk_size: usize) -> GreedyMeshBuilderSoAData {
    GreedyMeshBuilderSoAData {
        builder: create_mesh_builder(),
        chunk_size,
        visited: vec![false; chunk_size * chunk_size * chunk_size],
    }
}

/// Build mesh using greedy algorithm with SOA data
pub fn build_greedy_mesh(
    data: &mut GreedyMeshBuilderSoAData,
    blocks: &[BlockId],
    light_data: &[u8],
    chunk_size: usize,
) -> VertexBufferSoAData {
    clear(&mut data.builder);
    data.visited.fill(false);

    // Process each face direction for greedy meshing
    for axis in 0..3 {
        for direction in 0..2 {
            build_greedy_quads_for_axis(data, blocks, light_data, chunk_size, axis, direction);
        }
    }

    build_vertex_buffer(&data.builder)
}

/// Build greedy quads for a specific axis and direction
fn build_greedy_quads_for_axis(
    data: &mut GreedyMeshBuilderSoAData,
    blocks: &[BlockId],
    light_data: &[u8],
    chunk_size: usize,
    axis: usize,
    direction: usize,
) {
    let (u_axis, v_axis) = match axis {
        0 => (1, 2), // X axis: U=Y, V=Z
        1 => (0, 2), // Y axis: U=X, V=Z
        _ => (0, 1), // Z axis: U=X, V=Y
    };

    for layer in 0..chunk_size {
        build_layer_quads(
            data,
            blocks,
            light_data,
            chunk_size,
            axis,
            direction,
            layer,
            u_axis,
            v_axis,
        );
    }
}

/// Build quads for a single layer (optimized with SOA access patterns)
fn build_layer_quads(
    data: &mut GreedyMeshBuilderSoAData,
    blocks: &[BlockId],
    light_data: &[u8],
    chunk_size: usize,
    axis: usize,
    direction: usize,
    layer: usize,
    u_axis: usize,
    v_axis: usize,
) {
    // Reset visited for this layer
    for u in 0..chunk_size {
        for v in 0..chunk_size {
            let index = get_block_index(data.chunk_size, axis, layer, u, v, u_axis, v_axis);
            if index < data.visited.len() {
                data.visited[index] = false;
            }
        }
    }

    // Find and build quads using greedy algorithm
    for u in 0..chunk_size {
        for v in 0..chunk_size {
            let index = get_block_index(data.chunk_size, axis, layer, u, v, u_axis, v_axis);

            if index >= blocks.len() || data.visited[index] {
                continue;
            }

            let block = blocks[index];
            if block == BlockId::AIR {
                continue;
            }

            // Check if face should be rendered
            if !should_render_face(
                data, blocks, chunk_size, axis, direction, layer, u, v, u_axis, v_axis,
            ) {
                continue;
            }

            // Find the largest possible quad starting from this position
            let (width, height) = find_quad_size(
                data, blocks, chunk_size, axis, layer, u, v, u_axis, v_axis, block,
            );

            // Mark visited area
            for du in 0..width {
                for dv in 0..height {
                    let visit_index = get_block_index(
                        data.chunk_size,
                        axis,
                        layer,
                        u + du,
                        v + dv,
                        u_axis,
                        v_axis,
                    );
                    if visit_index < data.visited.len() {
                        data.visited[visit_index] = true;
                    }
                }
            }

            // Generate quad
            generate_quad(
                data,
                axis,
                direction,
                layer,
                u,
                v,
                width,
                height,
                block,
                light_data,
                chunk_size,
                u_axis,
                v_axis,
            );
        }
    }
}

/// Get block index for 3D coordinates
fn get_block_index(
    _chunk_size_data: usize,
    axis: usize,
    layer: usize,
    u: usize,
    v: usize,
    u_axis: usize,
    v_axis: usize,
) -> usize {
    let chunk_size = 50; // Use constant chunk size
    let mut coords = [0; 3];
    coords[axis] = layer;
    coords[u_axis] = u;
    coords[v_axis] = v;

    coords[0] + coords[1] * chunk_size + coords[2] * chunk_size * chunk_size
}

/// Check if a face should be rendered
fn should_render_face(
    data: &GreedyMeshBuilderSoAData,
    blocks: &[BlockId],
    chunk_size: usize,
    axis: usize,
    direction: usize,
    layer: usize,
    u: usize,
    v: usize,
    u_axis: usize,
    v_axis: usize,
) -> bool {
    // Check adjacent block
    let neighbor_layer = if direction == 0 {
        if layer == 0 {
            return true;
        }
        layer - 1
    } else {
        if layer == chunk_size - 1 {
            return true;
        }
        layer + 1
    };

    let neighbor_index =
        get_block_index(data.chunk_size, axis, neighbor_layer, u, v, u_axis, v_axis);
    if neighbor_index >= blocks.len() {
        return true;
    }

    blocks[neighbor_index] == BlockId::AIR
}

/// Find the largest possible quad size
fn find_quad_size(
    data: &GreedyMeshBuilderSoAData,
    blocks: &[BlockId],
    chunk_size: usize,
    axis: usize,
    layer: usize,
    start_u: usize,
    start_v: usize,
    u_axis: usize,
    v_axis: usize,
    block_type: BlockId,
) -> (usize, usize) {
    // Find width (expand in U direction)
    let mut width = 1;
    while start_u + width < chunk_size {
        let index = get_block_index(
            data.chunk_size,
            axis,
            layer,
            start_u + width,
            start_v,
            u_axis,
            v_axis,
        );
        if index >= blocks.len() || data.visited[index] || blocks[index] != block_type {
            break;
        }
        width += 1;
    }

    // Find height (expand in V direction)
    let mut height = 1;
    'height_loop: while start_v + height < chunk_size {
        // Check entire row at this height
        for u_offset in 0..width {
            let index = get_block_index(
                data.chunk_size,
                axis,
                layer,
                start_u + u_offset,
                start_v + height,
                u_axis,
                v_axis,
            );
            if index >= blocks.len() || data.visited[index] || blocks[index] != block_type {
                break 'height_loop;
            }
        }
        height += 1;
    }

    (width, height)
}

/// Generate a quad with the given parameters
fn generate_quad(
    data: &mut GreedyMeshBuilderSoAData,
    axis: usize,
    direction: usize,
    layer: usize,
    u: usize,
    v: usize,
    width: usize,
    height: usize,
    block: BlockId,
    light_data: &[u8],
    chunk_size: usize,
    u_axis: usize,
    v_axis: usize,
) {
    // Calculate quad positions
    let mut positions = [[0.0f32; 3]; 4];

    // Base position
    positions[0][axis] = layer as f32;
    positions[0][u_axis] = u as f32;
    positions[0][v_axis] = v as f32;

    // Adjust for direction
    if direction == 1 {
        positions[0][axis] += 1.0;
    }

    // Create quad vertices
    positions[1] = positions[0];
    positions[1][u_axis] += width as f32;

    positions[2] = positions[1];
    positions[2][v_axis] += height as f32;

    positions[3] = positions[0];
    positions[3][v_axis] += height as f32;

    // Calculate normal
    let mut normal = [0.0f32; 3];
    normal[axis] = if direction == 0 { -1.0 } else { 1.0 };

    // Get light level (sample from center of quad)
    let light_index = get_block_index(
        data.chunk_size,
        axis,
        layer,
        u + width / 2,
        v + height / 2,
        u_axis,
        v_axis,
    );
    let light = if light_index < light_data.len() {
        light_data[light_index] as f32 / 15.0
    } else {
        1.0
    };

    // Generate AO values (simplified for greedy meshing)
    let ao_values = [1.0, 1.0, 1.0, 1.0];

    // Add quad to builder
    add_quad_soa(&mut data.builder, positions, normal, block, light, ao_values);
}

/// Get mesh builder statistics for greedy builder
pub fn greedy_stats(data: &GreedyMeshBuilderSoAData) -> MeshBuilderStats {
    memory_stats(&data.builder)
}
