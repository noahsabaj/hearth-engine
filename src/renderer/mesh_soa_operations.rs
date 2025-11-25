//! Mesh SoA Operations - Pure DOP Functions
//!
//! All functions are pure: take data, return results, no side effects.
//! No methods, no self, just transformations.

use super::mesh_soa_data::{MeshSoAData, MeshStats};
use super::vertex_soa_data::VertexBufferSoAData;
use super::vertex_soa_operations;
use wgpu::util::DeviceExt;

/// Create new empty mesh SoA data
pub fn create_mesh_soa() -> MeshSoAData {
    MeshSoAData {
        vertices: vertex_soa_operations::create_vertex_buffer_soa(),
        indices: Vec::new(),
        index_buffer: None,
    }
}

/// Clear the mesh data
pub fn clear(data: &mut MeshSoAData) {
    vertex_soa_operations::clear(&mut data.vertices);
    data.indices.clear();
    data.index_buffer = None;
}

/// Add a quad (two triangles) to the mesh
pub fn add_quad(
    data: &mut MeshSoAData,
    positions: [[f32; 3]; 4],
    color: [f32; 3],
    normal: [f32; 3],
    light: f32,
    ao: [f32; 4], // AO for each vertex
) {
    let base_index = vertex_soa_operations::len(&data.vertices) as u32;

    // Add vertices
    for i in 0..4 {
        let ao_value = match ao.get(i) {
            Some(&value) => value,
            None => {
                log::warn!("AO value index {} out of bounds, using default", i);
                1.0
            }
        };
        let position = match positions.get(i) {
            Some(&pos) => pos,
            None => {
                log::warn!("Position index {} out of bounds, using origin", i);
                [0.0, 0.0, 0.0]
            }
        };
        vertex_soa_operations::push_vertex(&mut data.vertices, position, color, normal, light, ao_value);
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

/// Upload mesh data to GPU
pub fn upload(data: &mut MeshSoAData, device: &wgpu::Device) {
    if vertex_soa_operations::is_empty(&data.vertices) {
        return;
    }

    // Upload vertex data (SoA handles this internally)
    vertex_soa_operations::upload(&mut data.vertices, device);

    // Upload index data
    data.index_buffer = Some(
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Index Buffer"),
            contents: bytemuck::cast_slice(&data.indices),
            usage: wgpu::BufferUsages::INDEX,
        }),
    );
}

/// Bind mesh for rendering
pub fn bind<'a>(data: &'a MeshSoAData, render_pass: &mut wgpu::RenderPass<'a>) {
    vertex_soa_operations::bind(&data.vertices, render_pass);

    if let Some(buffer) = &data.index_buffer {
        render_pass.set_index_buffer(buffer.slice(..), wgpu::IndexFormat::Uint32);
    }
}

/// Get the number of indices (for draw calls)
pub fn index_count(data: &MeshSoAData) -> u32 {
    data.indices.len() as u32
}

/// Get memory statistics
pub fn memory_stats(data: &MeshSoAData) -> MeshStats {
    let vertex_stats = vertex_soa_operations::memory_stats(&data.vertices);
    let index_size = data.indices.len() * std::mem::size_of::<u32>();

    MeshStats {
        vertex_stats: vertex_stats.clone(),
        index_count: data.indices.len(),
        index_size,
        total_size: vertex_stats.total_size + index_size,
    }
}

/// Convert from traditional mesh for migration
pub fn from_traditional_mesh(vertices: &[super::vertex::Vertex], indices: &[u32]) -> MeshSoAData {
    let mut data = create_mesh_soa();
    data.vertices = vertex_soa_operations::from_aos(vertices);
    data.indices = indices.to_vec();
    data
}
