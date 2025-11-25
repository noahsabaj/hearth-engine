//! Mesh generation utilities for CPU-side mesh creation
//! Following DOP principles - pure functions that generate mesh data

use crate::renderer::vertex::Vertex;
use crate::world::core::{BlockId, ChunkPos, VoxelPos};
use crate::world::{world_operations, data_types::WorldData};

/// Generate vertices for a simple unit cube
/// Returns 24 vertices (6 faces * 4 vertices per face)
pub fn create_simple_cube_vertices() -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(24);

    // Define the 8 corner positions of a unit cube
    let positions = [
        [0.0, 0.0, 0.0], // 0: left, bottom, back
        [1.0, 0.0, 0.0], // 1: right, bottom, back
        [1.0, 1.0, 0.0], // 2: right, top, back
        [0.0, 1.0, 0.0], // 3: left, top, back
        [0.0, 0.0, 1.0], // 4: left, bottom, front
        [1.0, 0.0, 1.0], // 5: right, bottom, front
        [1.0, 1.0, 1.0], // 6: right, top, front
        [0.0, 1.0, 1.0], // 7: left, top, front
    ];

    // Define faces with their vertex indices and properties
    // Each face: (4 vertex indices, normal, color)
    let faces = [
        // Right face (+X)
        ([1, 5, 6, 2], [1.0, 0.0, 0.0], [0.8, 0.3, 0.3]),
        // Left face (-X)
        ([4, 0, 3, 7], [-1.0, 0.0, 0.0], [0.6, 0.2, 0.2]),
        // Top face (+Y)
        ([3, 2, 6, 7], [0.0, 1.0, 0.0], [0.3, 0.8, 0.3]),
        // Bottom face (-Y)
        ([4, 5, 1, 0], [0.0, -1.0, 0.0], [0.2, 0.6, 0.2]),
        // Front face (+Z)
        ([5, 4, 7, 6], [0.0, 0.0, 1.0], [0.3, 0.3, 0.8]),
        // Back face (-Z)
        ([0, 1, 2, 3], [0.0, 0.0, -1.0], [0.2, 0.2, 0.6]),
    ];

    // Generate vertices for each face
    for (indices, normal, color) in faces.iter() {
        for &idx in indices {
            vertices.push(Vertex {
                position: positions[idx],
                color: [color[0], color[1], color[2], 1.0], // Add alpha channel
                normal: *normal,
                light: 15u8, // Max light level
                ao: 3u8,     // Max ambient occlusion
            });
        }
    }

    vertices
}

/// Generate indices for a simple cube
/// Returns 36 indices (6 faces * 2 triangles * 3 vertices)
pub fn create_simple_cube_indices() -> Vec<u32> {
    let mut indices = Vec::with_capacity(36);

    // Each face has 4 vertices, generate 2 triangles per face
    for face in 0..6 {
        let base = face * 4;

        // First triangle (counter-clockwise)
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);

        // Second triangle (counter-clockwise)
        indices.push(base);
        indices.push(base + 2);
        indices.push(base + 3);
    }

    indices
}

/// Generate a cube mesh at a specific position with a specific color
pub fn create_colored_cube_at(
    position: [f32; 3],
    size: f32,
    color: [f32; 3],
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = create_simple_cube_vertices();

    // Transform vertices to the desired position and size
    for vertex in vertices.iter_mut() {
        vertex.position[0] = vertex.position[0] * size + position[0];
        vertex.position[1] = vertex.position[1] * size + position[1];
        vertex.position[2] = vertex.position[2] * size + position[2];
        vertex.color = [color[0], color[1], color[2], 1.0]; // Add alpha channel
    }

    let indices = create_simple_cube_indices();
    (vertices, indices)
}

/// Generate terrain mesh for a chunk based on actual voxel data
/// Pure DOP function - takes WorldData directly
pub fn generate_chunk_terrain_mesh(
    world: &WorldData,
    chunk_pos: ChunkPos,
    chunk_size: u32,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Log chunk generation start
    log::info!(
        "[generate_chunk_terrain_mesh] 🏗️ Starting mesh generation for chunk {:?} (size: {})",
        chunk_pos,
        chunk_size
    );

    let mut non_air_blocks = 0;
    let mut total_blocks = 0;
    let mut block_types = std::collections::HashMap::new();

    // Iterate through all voxels in the chunk
    for local_x in 0..chunk_size {
        for local_y in 0..chunk_size {
            for local_z in 0..chunk_size {
                // Calculate world position
                let world_pos = VoxelPos::new(
                    chunk_pos.x * chunk_size as i32 + local_x as i32,
                    chunk_pos.y * chunk_size as i32 + local_y as i32,
                    chunk_pos.z * chunk_size as i32 + local_z as i32,
                );

                // Get the block at this position
                let block = world_operations::get_block(world, world_pos, chunk_size);
                total_blocks += 1;

                // Track block types for debugging
                *block_types.entry(block).or_insert(0) += 1;

                // Log first few blocks for debugging
                if total_blocks <= 5 {
                    log::debug!(
                        "[generate_chunk_terrain_mesh] Block at world pos {:?}: {:?}",
                        world_pos,
                        block
                    );
                }

                if block == BlockId::AIR {
                    continue; // Skip air blocks
                }

                non_air_blocks += 1;

                // Get block color based on type
                let color = match block {
                    BlockId::STONE => [0.5, 0.5, 0.5],
                    BlockId::DIRT => [0.55, 0.4, 0.3],
                    BlockId::GRASS => [0.3, 0.7, 0.3],
                    BlockId::WOOD => [0.6, 0.5, 0.4],
                    BlockId::LEAVES => [0.2, 0.6, 0.2],
                    BlockId::WATER => [0.2, 0.4, 0.8],
                    BlockId::SAND => [0.9, 0.85, 0.6],
                    BlockId::GLASS => [0.9, 0.9, 0.95],
                    _ => [0.8, 0.8, 0.8], // Default color
                };

                // Check each face to see if it's exposed
                let face_offsets = [
                    ([1, 0, 0], [1.0, 0.0, 0.0]),   // +X
                    ([-1, 0, 0], [-1.0, 0.0, 0.0]), // -X
                    ([0, 1, 0], [0.0, 1.0, 0.0]),   // +Y
                    ([0, -1, 0], [0.0, -1.0, 0.0]), // -Y
                    ([0, 0, 1], [0.0, 0.0, 1.0]),   // +Z
                    ([0, 0, -1], [0.0, 0.0, -1.0]), // -Z
                ];

                for (offset, normal) in face_offsets.iter() {
                    let neighbor_pos = VoxelPos::new(
                        world_pos.x + offset[0],
                        world_pos.y + offset[1],
                        world_pos.z + offset[2],
                    );

                    // Enhanced face culling with chunk boundary handling
                    let should_render_face = {
                        let neighbor_block = world_operations::get_block(world, neighbor_pos, chunk_size);

                        // Check if neighbor is at chunk boundary
                        let neighbor_chunk_x = neighbor_pos.x.div_euclid(chunk_size as i32);
                        let neighbor_chunk_y = neighbor_pos.y.div_euclid(chunk_size as i32);
                        let neighbor_chunk_z = neighbor_pos.z.div_euclid(chunk_size as i32);

                        let neighbor_chunk_pos = ChunkPos::new(neighbor_chunk_x, neighbor_chunk_y, neighbor_chunk_z);

                        // If neighbor is in a different chunk, check if that chunk is loaded
                        if neighbor_chunk_pos != chunk_pos {
                            // If neighbor chunk isn't loaded, assume it's AIR (DO render face)
                            // This ensures surface faces are visible until neighbor chunks load
                            if !world_operations::is_chunk_loaded(world, neighbor_chunk_pos) {
                                true // Render face - assume neighbor is AIR until loaded
                            } else {
                                // Neighbor chunk is loaded, check the actual block
                                neighbor_block == BlockId::AIR ||
                                (neighbor_block == BlockId::WATER && block != BlockId::WATER)
                            }
                        } else {
                            // Same chunk - use normal transparency check
                            neighbor_block == BlockId::AIR ||
                            (neighbor_block == BlockId::WATER && block != BlockId::WATER)
                        }
                    };

                    if should_render_face {
                        // Add face vertices
                        let base_vertex = vertices.len() as u32;

                        // Calculate face vertices based on normal
                        let (face_vertices, _) = create_face_vertices(
                            [local_x as f32, local_y as f32, local_z as f32],
                            *normal,
                            color,
                        );

                        vertices.extend(face_vertices);

                        // Add face indices
                        indices.push(base_vertex);
                        indices.push(base_vertex + 1);
                        indices.push(base_vertex + 2);
                        indices.push(base_vertex);
                        indices.push(base_vertex + 2);
                        indices.push(base_vertex + 3);
                    }
                }
            }
        }
    }

    log::info!("[generate_chunk_terrain_mesh] 📊 Generated mesh for chunk {:?}: {} vertices, {} indices (blocks: {}/{} non-air)", 
              chunk_pos, vertices.len(), indices.len(), non_air_blocks, total_blocks);

    // Log block type breakdown
    log::info!(
        "[generate_chunk_terrain_mesh] 📋 Block types in chunk {:?}:",
        chunk_pos
    );
    for (block_id, count) in block_types.iter() {
        log::info!("  - {:?}: {} blocks", block_id, count);
    }

    // Log sample blocks if the chunk is empty
    if non_air_blocks == 0 && total_blocks > 0 {
        // Sample center of chunk
        let center_pos = VoxelPos::new(
            chunk_pos.x * chunk_size as i32 + chunk_size as i32 / 2,
            chunk_pos.y * chunk_size as i32 + chunk_size as i32 / 2,
            chunk_pos.z * chunk_size as i32 + chunk_size as i32 / 2,
        );
        let center_block = world_operations::get_block(world, center_pos, chunk_size);
        log::debug!(
            "[generate_chunk_terrain_mesh] Chunk {:?} is empty. Center block at {:?} is {:?}",
            chunk_pos,
            center_pos,
            center_block
        );
    }

    (vertices, indices)
}

/// Create vertices for a single face of a voxel
fn create_face_vertices(
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::with_capacity(4);

    // Generate vertices based on face normal
    let face_vertices = if normal[0] > 0.0 {
        // +X face
        [
            [position[0] + 1.0, position[1], position[2]],
            [position[0] + 1.0, position[1], position[2] + 1.0],
            [position[0] + 1.0, position[1] + 1.0, position[2] + 1.0],
            [position[0] + 1.0, position[1] + 1.0, position[2]],
        ]
    } else if normal[0] < 0.0 {
        // -X face
        [
            [position[0], position[1], position[2] + 1.0],
            [position[0], position[1], position[2]],
            [position[0], position[1] + 1.0, position[2]],
            [position[0], position[1] + 1.0, position[2] + 1.0],
        ]
    } else if normal[1] > 0.0 {
        // +Y face
        [
            [position[0], position[1] + 1.0, position[2]],
            [position[0] + 1.0, position[1] + 1.0, position[2]],
            [position[0] + 1.0, position[1] + 1.0, position[2] + 1.0],
            [position[0], position[1] + 1.0, position[2] + 1.0],
        ]
    } else if normal[1] < 0.0 {
        // -Y face
        [
            [position[0], position[1], position[2] + 1.0],
            [position[0] + 1.0, position[1], position[2] + 1.0],
            [position[0] + 1.0, position[1], position[2]],
            [position[0], position[1], position[2]],
        ]
    } else if normal[2] > 0.0 {
        // +Z face
        [
            [position[0] + 1.0, position[1], position[2] + 1.0],
            [position[0], position[1], position[2] + 1.0],
            [position[0], position[1] + 1.0, position[2] + 1.0],
            [position[0] + 1.0, position[1] + 1.0, position[2] + 1.0],
        ]
    } else {
        // -Z face
        [
            [position[0], position[1], position[2]],
            [position[0] + 1.0, position[1], position[2]],
            [position[0] + 1.0, position[1] + 1.0, position[2]],
            [position[0], position[1] + 1.0, position[2]],
        ]
    };

    for pos in face_vertices.iter() {
        vertices.push(Vertex {
            position: *pos,
            color: [color[0], color[1], color[2], 1.0], // Add alpha channel
            normal,
            light: 15u8, // Max light level
            ao: 3u8,     // Max ambient occlusion
        });
    }

    (vertices, vec![0, 1, 2, 0, 2, 3])
}

pub struct MeshUtils;

