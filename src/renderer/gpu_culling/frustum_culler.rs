//! Frustum Culler - Stub
use super::GpuCamera;

pub struct FrustumCuller;

impl FrustumCuller {
    pub fn new(_device: &wgpu::Device, _max_chunks: usize) -> Self {
        Self
    }

    pub fn cull(
        &self,
        _device: &wgpu::Device,
        _encoder: &mut wgpu::CommandEncoder,
        _camera: &GpuCamera,
        _chunk_instances: &wgpu::Buffer,
        _chunk_count: u32,
        _stats_buffer: &wgpu::Buffer,
    ) -> () {
        // Stub implementation - returns unit type for now
        ()
    }
}
