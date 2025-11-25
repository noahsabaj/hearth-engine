//! Vertex Buffer SoA Data - Pure DOP
//!
//! NO METHODS. Just data.
//! All transformations happen in vertex_soa_operations.rs

/// Struct-of-Arrays vertex buffer data for better cache efficiency
pub struct VertexBufferSoAData {
    // Separate arrays for each attribute
    pub positions: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub lights: Vec<f32>,
    pub aos: Vec<f32>,

    // GPU buffers (created on upload)
    pub position_buffer: Option<wgpu::Buffer>,
    pub color_buffer: Option<wgpu::Buffer>,
    pub normal_buffer: Option<wgpu::Buffer>,
    pub light_buffer: Option<wgpu::Buffer>,
    pub ao_buffer: Option<wgpu::Buffer>,
}

/// Memory statistics for vertex buffer
#[derive(Debug, Clone)]
pub struct VertexBufferStats {
    pub vertex_count: usize,
    pub total_size: usize,
    pub positions_size: usize,
    pub colors_size: usize,
    pub normals_size: usize,
    pub lights_size: usize,
    pub aos_size: usize,
}

impl std::fmt::Display for VertexBufferStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VertexBuffer: {} vertices, {} bytes total (pos: {}, col: {}, norm: {}, light: {}, ao: {})",
            self.vertex_count,
            self.total_size,
            self.positions_size,
            self.colors_size,
            self.normals_size,
            self.lights_size,
            self.aos_size
        )
    }
}
