//! Mesh SoA Data - Pure DOP
//!
//! NO METHODS. Just data.
//! All transformations happen in mesh_soa_operations.rs

use super::vertex_soa_data::VertexBufferSoAData;

/// Mesh using Struct-of-Arrays data for better cache efficiency
pub struct MeshSoAData {
    pub vertices: VertexBufferSoAData,
    pub indices: Vec<u32>,
    pub index_buffer: Option<wgpu::Buffer>,
}

/// Memory statistics for mesh
#[derive(Debug)]
pub struct MeshStats {
    pub vertex_stats: super::vertex_soa_data::VertexBufferStats,
    pub index_count: usize,
    pub index_size: usize,
    pub total_size: usize,
}

impl std::fmt::Display for MeshStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Mesh: {} indices ({}B), Vertices: {}, Total: {}B",
            self.index_count, self.index_size, self.vertex_stats, self.total_size
        )
    }
}
