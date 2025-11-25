//! Vertex Buffer SoA Operations - Pure DOP Functions
//!
//! All functions are pure: take data, return results, no side effects.
//! No methods, no self, just transformations.

use super::vertex_soa_data::{VertexBufferSoAData, VertexBufferStats};
use wgpu::util::DeviceExt;

/// Create new empty vertex buffer SoA data
pub fn create_vertex_buffer_soa() -> VertexBufferSoAData {
    VertexBufferSoAData {
        positions: Vec::new(),
        colors: Vec::new(),
        normals: Vec::new(),
        lights: Vec::new(),
        aos: Vec::new(),
        position_buffer: None,
        color_buffer: None,
        normal_buffer: None,
        light_buffer: None,
        ao_buffer: None,
    }
}

/// Add a vertex to the buffer
pub fn push_vertex(
    data: &mut VertexBufferSoAData,
    position: [f32; 3],
    color: [f32; 3],
    normal: [f32; 3],
    light: f32,
    ao: f32,
) {
    data.positions.push(position);
    data.colors.push(color);
    data.normals.push(normal);
    data.lights.push(light);
    data.aos.push(ao);
}

/// Clear all vertex data
pub fn clear(data: &mut VertexBufferSoAData) {
    data.positions.clear();
    data.colors.clear();
    data.normals.clear();
    data.lights.clear();
    data.aos.clear();
}

/// Get the number of vertices
pub fn len(data: &VertexBufferSoAData) -> usize {
    data.positions.len()
}

/// Check if empty
pub fn is_empty(data: &VertexBufferSoAData) -> bool {
    data.positions.is_empty()
}

/// Upload to GPU - creates separate buffers for each attribute
pub fn upload(data: &mut VertexBufferSoAData, device: &wgpu::Device) {
    if is_empty(data) {
        return;
    }

    // Create position buffer
    data.position_buffer = Some(
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Position Buffer"),
            contents: bytemuck::cast_slice(&data.positions),
            usage: wgpu::BufferUsages::VERTEX,
        }),
    );

    // Create color buffer
    data.color_buffer = Some(
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Color Buffer"),
            contents: bytemuck::cast_slice(&data.colors),
            usage: wgpu::BufferUsages::VERTEX,
        }),
    );

    // Create normal buffer
    data.normal_buffer = Some(
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Normal Buffer"),
            contents: bytemuck::cast_slice(&data.normals),
            usage: wgpu::BufferUsages::VERTEX,
        }),
    );

    // Create light buffer
    data.light_buffer = Some(
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Light Buffer"),
            contents: bytemuck::cast_slice(&data.lights),
            usage: wgpu::BufferUsages::VERTEX,
        }),
    );

    // Create AO buffer
    data.ao_buffer = Some(
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex AO Buffer"),
            contents: bytemuck::cast_slice(&data.aos),
            usage: wgpu::BufferUsages::VERTEX,
        }),
    );
}

/// Bind buffers for rendering
pub fn bind<'a>(data: &'a VertexBufferSoAData, render_pass: &mut wgpu::RenderPass<'a>) {
    if let Some(buffer) = &data.position_buffer {
        render_pass.set_vertex_buffer(0, buffer.slice(..));
    }
    if let Some(buffer) = &data.color_buffer {
        render_pass.set_vertex_buffer(1, buffer.slice(..));
    }
    if let Some(buffer) = &data.normal_buffer {
        render_pass.set_vertex_buffer(2, buffer.slice(..));
    }
    if let Some(buffer) = &data.light_buffer {
        render_pass.set_vertex_buffer(3, buffer.slice(..));
    }
    if let Some(buffer) = &data.ao_buffer {
        render_pass.set_vertex_buffer(4, buffer.slice(..));
    }
}

/// Get vertex buffer layouts for SoA
pub fn get_vertex_buffer_layouts<'a>() -> Vec<wgpu::VertexBufferLayout<'a>> {
    vec![
        // Position buffer
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        },
        // Color buffer
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
            }],
        },
        // Normal buffer
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x3,
            }],
        },
        // Light buffer
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<f32>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 3,
                format: wgpu::VertexFormat::Float32,
            }],
        },
        // AO buffer
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<f32>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 4,
                format: wgpu::VertexFormat::Float32,
            }],
        },
    ]
}

/// Convert from Array-of-Structs for migration
pub fn from_aos(vertices: &[super::vertex::Vertex]) -> VertexBufferSoAData {
    let mut data = create_vertex_buffer_soa();
    for vertex in vertices {
        push_vertex(
            &mut data,
            vertex.position,
            vertex.color,
            vertex.normal,
            vertex.light,
            vertex.ao,
        );
    }
    data
}

/// Get memory statistics
pub fn memory_stats(data: &VertexBufferSoAData) -> VertexBufferStats {
    let positions_size = data.positions.len() * std::mem::size_of::<[f32; 3]>();
    let colors_size = data.colors.len() * std::mem::size_of::<[f32; 3]>();
    let normals_size = data.normals.len() * std::mem::size_of::<[f32; 3]>();
    let lights_size = data.lights.len() * std::mem::size_of::<f32>();
    let aos_size = data.aos.len() * std::mem::size_of::<f32>();

    VertexBufferStats {
        vertex_count: len(data),
        total_size: positions_size + colors_size + normals_size + lights_size + aos_size,
        positions_size,
        colors_size,
        normals_size,
        lights_size,
        aos_size,
    }
}
