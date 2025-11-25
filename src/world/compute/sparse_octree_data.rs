//! Sparse Voxel Octree Data - Pure DOP
//!
//! NO METHODS. Just data.
//! All transformations happen in sparse_octree_operations.rs

use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::{Buffer, Device};

/// Octree node stored on GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct OctreeNode {
    /// Child pointers (0 = empty, high bit set = leaf)
    pub children: [u32; 8],
    /// Node metadata
    /// Bits 0-7: Node level (0 = leaf)
    /// Bits 8-15: Occupancy mask
    /// Bits 16-23: Material predominance
    /// Bits 24-31: Flags
    pub metadata: u32,
    /// Bounding box min (optional, for BVH integration)
    pub bbox_min: [f32; 3],
    /// Bounding box max
    pub bbox_max: [f32; 3],
}

impl OctreeNode {
    pub const EMPTY: Self = Self {
        children: [0; 8],
        metadata: 0,
        bbox_min: [0.0; 3],
        bbox_max: [0.0; 3],
    };
}

/// Sparse voxel octree data
pub struct SparseVoxelOctreeData {
    pub device: Arc<Device>,

    /// GPU buffer containing all octree nodes
    pub node_buffer: Buffer,

    /// Node allocation info
    pub node_capacity: u32,
    pub next_free_node: u32,

    /// Octree configuration
    pub world_size: u32,
    pub max_depth: u32,
}

/// Octree statistics
#[derive(Debug, Clone)]
pub struct OctreeStats {
    pub total_nodes: u32,
    pub node_capacity: u32,
    pub max_depth: u32,
    pub memory_usage_mb: f32,
}

/// GPU compute shader data for octree updates
pub struct OctreeUpdaterData {
    pub device: Arc<Device>,
    pub update_pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}
