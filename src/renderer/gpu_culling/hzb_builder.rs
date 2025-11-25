//! HZB Builder - Stub
use super::GpuCamera;

pub struct HzbBuilder;

pub struct HierarchicalZBuffer;

impl HierarchicalZBuffer {
    pub fn new(_device: &wgpu::Device, _width: u32, _height: u32) -> Self {
        Self
    }

    pub fn build(&self, _encoder: &mut wgpu::CommandEncoder, _depth_texture: &wgpu::TextureView) {
        // Stub implementation
    }

    pub fn cull_occlusion(
        &self,
        _encoder: &mut wgpu::CommandEncoder,
        _camera: &GpuCamera,
        _chunk_instances: &wgpu::Buffer,
        _frustum_visible: (),
    ) -> () {
        // Stub implementation - returns unit type for now
        ()
    }
}
