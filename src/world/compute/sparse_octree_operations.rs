//! Sparse Voxel Octree Operations - Pure DOP Functions
//!
//! All functions are pure: take data, return results, no side effects.
//! No methods, no self, just transformations.

use super::sparse_octree_data::{OctreeNode, OctreeStats, OctreeUpdaterData, SparseVoxelOctreeData};
use crate::memory::MemoryManager;
use crate::world::core::ChunkPos;
use crate::world::storage::WorldBuffer;
use std::sync::Arc;
use wgpu::{Device, Queue};

/// Check if octree node is a leaf
pub fn is_leaf(node: &OctreeNode) -> bool {
    (node.metadata & 0xFF) == 0
}

/// Get octree node level
pub fn level(node: &OctreeNode) -> u8 {
    (node.metadata & 0xFF) as u8
}

/// Get octree node occupancy mask
pub fn occupancy_mask(node: &OctreeNode) -> u8 {
    ((node.metadata >> 8) & 0xFF) as u8
}

/// Set child in octree node
pub fn set_child(node: &mut OctreeNode, index: usize, pointer: u32) {
    node.children[index] = pointer;
    if pointer != 0 {
        node.metadata |= 1 << (8 + index);
    } else {
        node.metadata &= !(1 << (8 + index));
    }
}

/// Create new sparse voxel octree
pub fn create_sparse_voxel_octree(
    device: Arc<Device>,
    _memory_manager: &mut MemoryManager,
    world_size: u32,
) -> SparseVoxelOctreeData {
    // Calculate octree parameters
    let max_depth = (world_size as f32).log2().ceil() as u32;
    let node_capacity = 1_000_000; // Start with 1M nodes

    // Allocate GPU buffer for nodes
    let node_buffer_size = node_capacity as u64 * std::mem::size_of::<OctreeNode>() as u64;
    let node_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Octree Node Buffer"),
        size: node_buffer_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    SparseVoxelOctreeData {
        device,
        node_buffer,
        node_capacity,
        next_free_node: 1, // 0 is reserved for null
        world_size,
        max_depth,
    }
}

/// Build octree from world buffer
pub fn build_from_world(
    data: &mut SparseVoxelOctreeData,
    queue: &Queue,
    _world_buffer: &WorldBuffer,
    active_chunks: &[ChunkPos],
) {
    let mut nodes = vec![OctreeNode::EMPTY; data.node_capacity as usize];

    // Root node
    nodes[0] = OctreeNode {
        children: [0; 8],
        metadata: data.max_depth, // Set level
        bbox_min: [0.0, 0.0, 0.0],
        bbox_max: [data.world_size as f32; 3],
    };

    // Build octree for active chunks
    for chunk_pos in active_chunks {
        insert_chunk(data, &mut nodes, chunk_pos);
    }

    // Upload to GPU
    queue.write_buffer(
        &data.node_buffer,
        0,
        bytemuck::cast_slice(&nodes[..data.next_free_node as usize]),
    );
}

/// Insert a chunk into the octree
fn insert_chunk(data: &mut SparseVoxelOctreeData, nodes: &mut [OctreeNode], chunk_pos: &ChunkPos) {
    let mut current_node = 0;
    let mut current_level = data.max_depth;
    let mut current_size = data.world_size;
    let mut current_pos = [0u32; 3];

    // Traverse down the octree
    while current_level > 0 {
        let half_size = current_size / 2;

        // Determine which octant the chunk belongs to
        let octant = calculate_octant(chunk_pos, current_pos, half_size);

        // Get or create child node
        if nodes[current_node].children[octant] == 0 {
            // Allocate new node
            let new_node = data.next_free_node;
            data.next_free_node += 1;

            set_child(&mut nodes[current_node], octant, new_node);

            // Initialize child node
            nodes[new_node as usize] = OctreeNode {
                children: [0; 8],
                metadata: current_level - 1,
                bbox_min: [
                    current_pos[0] as f32
                        + if octant & 1 != 0 {
                            half_size as f32
                        } else {
                            0.0
                        },
                    current_pos[1] as f32
                        + if octant & 2 != 0 {
                            half_size as f32
                        } else {
                            0.0
                        },
                    current_pos[2] as f32
                        + if octant & 4 != 0 {
                            half_size as f32
                        } else {
                            0.0
                        },
                ],
                bbox_max: [
                    current_pos[0] as f32
                        + if octant & 1 != 0 {
                            current_size as f32
                        } else {
                            half_size as f32
                        },
                    current_pos[1] as f32
                        + if octant & 2 != 0 {
                            current_size as f32
                        } else {
                            half_size as f32
                        },
                    current_pos[2] as f32
                        + if octant & 4 != 0 {
                            current_size as f32
                        } else {
                            half_size as f32
                        },
                ],
            };
        }

        // Move to child
        current_node = nodes[current_node].children[octant] as usize;
        current_level -= 1;
        current_size = half_size;

        // Update position
        if octant & 1 != 0 {
            current_pos[0] += half_size;
        }
        if octant & 2 != 0 {
            current_pos[1] += half_size;
        }
        if octant & 4 != 0 {
            current_pos[2] += half_size;
        }
    }

    // Mark leaf as occupied
    nodes[current_node].metadata |= 0xFF00; // Full occupancy
}

/// Calculate which octant a position belongs to
fn calculate_octant(chunk_pos: &ChunkPos, base_pos: [u32; 3], half_size: u32) -> usize {
    let mut octant = 0;

    if chunk_pos.x as u32 >= base_pos[0] + half_size {
        octant |= 1;
    }
    if chunk_pos.y as u32 >= base_pos[1] + half_size {
        octant |= 2;
    }
    if chunk_pos.z as u32 >= base_pos[2] + half_size {
        octant |= 4;
    }

    octant
}

/// Get the GPU buffer containing octree nodes
pub fn node_buffer(data: &SparseVoxelOctreeData) -> &wgpu::Buffer {
    &data.node_buffer
}

/// Get octree statistics
pub fn get_stats(data: &SparseVoxelOctreeData) -> OctreeStats {
    OctreeStats {
        total_nodes: data.next_free_node,
        node_capacity: data.node_capacity,
        max_depth: data.max_depth,
        memory_usage_mb: (data.next_free_node as f32 * std::mem::size_of::<OctreeNode>() as f32)
            / (1024.0 * 1024.0),
    }
}

/// Create octree updater
pub fn create_octree_updater(device: Arc<Device>) -> OctreeUpdaterData {
    let shader_source = include_str!("../../shaders/compute/octree_update.wgsl");
    let validated_shader = match crate::gpu::automation::create_gpu_shader(
        &device,
        "octree_update",
        shader_source,
    ) {
        Ok(shader) => shader,
        Err(e) => {
            log::error!("Failed to create octree update shader: {}", e);
            panic!("Failed to create octree update shader: {}", e);
        }
    };

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Octree Update Layout"),
        entries: &[
            // Octree nodes
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // World voxels
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Octree Update Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let update_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Octree Update Pipeline"),
        layout: Some(&pipeline_layout),
        module: &validated_shader.module,
        entry_point: "update_octree",
    });

    OctreeUpdaterData {
        device,
        update_pipeline,
        bind_group_layout,
    }
}

/// Update octree based on world changes
pub fn update_octree(
    updater: &OctreeUpdaterData,
    encoder: &mut wgpu::CommandEncoder,
    octree: &SparseVoxelOctreeData,
    world_buffer: &WorldBuffer,
) {
    let bind_group = updater
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Octree Update Bind Group"),
            layout: &updater.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: node_buffer(octree).as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: world_buffer.voxel_buffer().as_entire_binding(),
                },
            ],
        });

    let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("Octree Update Pass"),
        timestamp_writes: None,
    });

    compute_pass.set_pipeline(&updater.update_pipeline);
    compute_pass.set_bind_group(0, &bind_group, &[]);
    compute_pass.dispatch_workgroups(octree.next_free_node / 64 + 1, 1, 1);
}
