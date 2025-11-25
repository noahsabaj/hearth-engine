//! SOA Mesh Builder Data - Pure DOP
//!
//! NO METHODS. Just data.
//! All transformations happen in soa_mesh_builder_operations.rs

use crate::BlockId;
use std::collections::HashMap;

/// Mesh generation data in SOA layout
pub struct MeshBuilderSoAData {
    /// Vertex data arrays
    pub positions: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub light_levels: Vec<f32>,
    pub ao_values: Vec<f32>,

    /// Index data
    pub indices: Vec<u32>,

    /// Temporary working arrays (reused across chunks)
    pub temp_positions: Vec<[f32; 3]>,
    pub temp_normals: Vec<[f32; 3]>,
    pub temp_colors: Vec<[f32; 3]>,

    /// Face visibility cache (for greedy meshing)
    pub face_visibility: Vec<bool>,

    /// Block color lookup (pre-computed for cache efficiency)
    pub block_colors: HashMap<BlockId, [f32; 3]>,
}

/// Memory usage statistics for mesh builder
#[derive(Debug, Clone)]
pub struct MeshBuilderStats {
    pub vertex_count: usize,
    pub index_count: usize,
    pub positions_bytes: usize,
    pub colors_bytes: usize,
    pub normals_bytes: usize,
    pub light_bytes: usize,
    pub ao_bytes: usize,
    pub indices_bytes: usize,
}

impl MeshBuilderStats {
    pub fn total_bytes(&self) -> usize {
        self.positions_bytes
            + self.colors_bytes
            + self.normals_bytes
            + self.light_bytes
            + self.ao_bytes
            + self.indices_bytes
    }
}

/// Greedy meshing data using SOA for cache efficiency
pub struct GreedyMeshBuilderSoAData {
    pub builder: MeshBuilderSoAData,
    /// Chunk size for greedy meshing
    pub chunk_size: usize,
    /// Visited mask for greedy algorithm
    pub visited: Vec<bool>,
}
