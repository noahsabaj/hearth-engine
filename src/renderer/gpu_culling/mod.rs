use crate::renderer::error::{buffer_mapping_error, RendererErrorContext, RendererResult};
use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix4, Vector3, Vector4};
/// GPU-Driven Culling System
///
/// Manages frustum and occlusion culling entirely on GPU.
use wgpu::{Buffer, Device, Queue};

pub mod frustum_culler;
pub mod hzb_builder;
pub mod indirect_renderer;
pub mod instance_streamer;

pub use frustum_culler::FrustumCuller;
pub use hzb_builder::HierarchicalZBuffer;
pub use indirect_renderer::IndirectRenderer;
pub use instance_streamer::{InstanceStreamer, StreamingMetrics};

/// Camera data for GPU culling
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuCamera {
    pub view_proj: [[f32; 4]; 4],
    pub position: [f32; 3],
    _padding: f32,
    pub frustum_planes: [[f32; 4]; 6], // 6 planes: left, right, top, bottom, near, far
}

impl GpuCamera {
    pub fn from_matrices(view: &Matrix4<f32>, proj: &Matrix4<f32>, position: Vector3<f32>) -> Self {
        let view_proj = proj * view;
        let frustum_planes = extract_frustum_planes(&view_proj);

        Self {
            view_proj: view_proj.into(),
            position: position.into(),
            _padding: 0.0,
            frustum_planes: [
                frustum_planes[0].into(),
                frustum_planes[1].into(),
                frustum_planes[2].into(),
                frustum_planes[3].into(),
                frustum_planes[4].into(),
                frustum_planes[5].into(),
            ],
        }
    }
}

/// Extract frustum planes from view-projection matrix
fn extract_frustum_planes(vp: &Matrix4<f32>) -> [Vector4<f32>; 6] {
    // Extract planes using Gribb-Hartmann method
    let m = vp;

    [
        // Left plane
        Vector4::new(m.x.w + m.x.x, m.y.w + m.y.x, m.z.w + m.z.x, m.w.w + m.w.x).normalize(),
        // Right plane
        Vector4::new(m.x.w - m.x.x, m.y.w - m.y.x, m.z.w - m.z.x, m.w.w - m.w.x).normalize(),
        // Top plane
        Vector4::new(m.x.w - m.x.y, m.y.w - m.y.y, m.z.w - m.z.y, m.w.w - m.w.y).normalize(),
        // Bottom plane
        Vector4::new(m.x.w + m.x.y, m.y.w + m.y.y, m.z.w + m.z.y, m.w.w + m.w.y).normalize(),
        // Near plane
        Vector4::new(m.x.w + m.x.z, m.y.w + m.y.z, m.z.w + m.z.z, m.w.w + m.w.z).normalize(),
        // Far plane
        Vector4::new(m.x.w - m.x.z, m.y.w - m.y.z, m.z.w - m.z.z, m.w.w - m.w.z).normalize(),
    ]
}

/// Normalize plane equation
trait NormalizePlane {
    fn normalize(self) -> Self;
}

impl NormalizePlane for Vector4<f32> {
    fn normalize(self) -> Self {
        let length = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if length > 0.0 {
            self / length
        } else {
            self
        }
    }
}

/// Chunk instance data for culling
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct ChunkInstance {
    pub world_position: [f32; 3],
    pub chunk_size: f32,
    pub lod_level: u32,
    pub flags: u32,
    _padding: [f32; 2],
}

/// Indirect draw command
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct DrawCommand {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

/// Culling statistics
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct CullingStats {
    pub total_chunks: u32,
    pub visible_chunks: u32,
    pub frustum_culled: u32,
    pub distance_culled: u32,
}

/// Complete GPU culling system
pub struct GpuCullingSystem {
    frustum_culler: FrustumCuller,
    hzb: HierarchicalZBuffer,
    indirect_renderer: IndirectRenderer,

    // Statistics
    stats_buffer: Buffer,
    stats_readback: Buffer,
}

impl GpuCullingSystem {
    pub fn new(device: &Device, max_chunks: usize) -> Self {
        let frustum_culler = FrustumCuller::new(device, max_chunks);
        let hzb = HierarchicalZBuffer::new(device, 2048, 2048); // Start with 2K
        let indirect_renderer = IndirectRenderer::new(device, max_chunks);

        // Create stats buffers
        let stats_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Culling Stats Buffer"),
            size: std::mem::size_of::<CullingStats>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let stats_readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Culling Stats Readback"),
            size: std::mem::size_of::<CullingStats>() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            frustum_culler,
            hzb,
            indirect_renderer,
            stats_buffer,
            stats_readback,
        }
    }

    /// Perform complete culling pass
    pub fn cull(
        &mut self,
        device: &Device,
        encoder: &mut wgpu::CommandEncoder,
        camera: &GpuCamera,
        chunk_instances: &Buffer,
        chunk_count: u32,
        depth_texture: &wgpu::TextureView,
    ) -> Option<&Buffer> {
        // Step 1: Build HZB from depth buffer
        self.hzb.build(encoder, depth_texture);

        // Step 2: Frustum culling
        let frustum_visible = self.frustum_culler.cull(
            device,
            encoder,
            camera,
            chunk_instances,
            chunk_count,
            &self.stats_buffer,
        );

        // Step 3: Occlusion culling using HZB
        let final_visible =
            self.hzb
                .cull_occlusion(encoder, camera, chunk_instances, frustum_visible);

        // Step 4: Generate indirect draw commands
        self.indirect_renderer
            .generate_commands(encoder, final_visible)
    }

    /// Read back culling statistics
    pub async fn read_stats(&self, device: &Device, queue: &Queue) -> RendererResult<CullingStats> {
        // Copy stats to readback buffer
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Stats Readback"),
        });

        encoder.copy_buffer_to_buffer(
            &self.stats_buffer,
            0,
            &self.stats_readback,
            0,
            std::mem::size_of::<CullingStats>() as u64,
        );

        queue.submit(Some(encoder.finish()));

        // Map and read
        let buffer_slice = self.stats_readback.slice(..);
        let (sender, receiver) = flume::bounded(1);
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        device.poll(wgpu::Maintain::Wait);
        receiver
            .recv_async()
            .await
            .map_err(|_| buffer_mapping_error("culling stats recv_async"))?
            .map_err(|_| buffer_mapping_error("culling stats map_async"))?;

        let data = buffer_slice.get_mapped_range();
        let stats = bytemuck::from_bytes::<CullingStats>(&data).clone();
        drop(data);
        self.stats_readback.unmap();

        Ok(stats)
    }
}

/// Performance metrics for GPU culling
#[derive(Debug, Default)]
pub struct GpuCullingMetrics {
    pub total_chunks: u32,
    pub visible_after_frustum: u32,
    pub visible_after_occlusion: u32,
    pub culling_time_ms: f32,
    pub draw_calls_saved: u32,
}
