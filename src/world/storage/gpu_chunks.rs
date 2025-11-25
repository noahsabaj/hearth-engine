use crate::constants::core::VOXELS_PER_CHUNK;
use crate::world::core::ChunkPos;
use crate::world::storage::{Chunk, TempChunk};
use wgpu::util::DeviceExt;

/// GPU-resident chunk data for efficient GPU processing
pub struct GpuChunk {
    position: ChunkPos,
    size: u32,

    // GPU buffers
    block_buffer: wgpu::Buffer,
    light_buffer: wgpu::Buffer,
    metadata_buffer: wgpu::Buffer,

    // Bind group for compute shaders
    bind_group: Option<wgpu::BindGroup>,

    // Track if CPU data has changed
    dirty: bool,
}

impl GpuChunk {
    /// Create a new GPU chunk from temporary chunk data
    pub fn new(device: &wgpu::Device, chunk: &TempChunk) -> Self {
        let size = chunk.size();
        let block_count = (size * size * size) as usize;

        // Create block buffer
        let blocks = chunk.blocks();
        let block_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GPU Chunk Block Buffer"),
            contents: bytemuck::cast_slice(&blocks),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create light buffer (placeholder for now)
        let light_data = vec![0u8; block_count * 2]; // 2 bytes per block for light
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GPU Chunk Light Buffer"),
            contents: &light_data,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create metadata buffer
        let metadata = ChunkMetadata {
            position: [
                chunk.position().x,
                chunk.position().y,
                chunk.position().z,
                0,
            ],
            size: size,
            block_count: block_count as u32,
            flags: 0,
            _padding: 0,
        };
        let metadata_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GPU Chunk Metadata Buffer"),
            contents: bytemuck::bytes_of(&metadata),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            position: chunk.position().clone(),
            size,
            block_buffer,
            light_buffer,
            metadata_buffer,
            bind_group: None,
            dirty: false,
        }
    }

    /// Create a new GPU chunk from ChunkData
    pub fn new_from_chunk(device: &wgpu::Device, chunk: &Chunk) -> Self {
        let blocks = chunk.blocks();
        let block_count = blocks.len();
        let size = (block_count as f64).cbrt() as u32; // Derive size from block count

        // Create block buffer
        let block_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GPU Chunk Block Buffer"),
            contents: bytemuck::cast_slice(blocks),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create light buffer (placeholder for now)
        let light_data = vec![0u8; block_count * 2]; // 2 bytes per block for light
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GPU Chunk Light Buffer"),
            contents: &light_data,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create metadata buffer
        let pos = chunk.position();
        let metadata = ChunkMetadata {
            position: [pos.x, pos.y, pos.z, 0], // xyz + padding for alignment
            size,
            block_count: block_count as u32,
            flags: 0,
            _padding: 0,
        };
        let metadata_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GPU Chunk Metadata Buffer"),
            contents: bytemuck::bytes_of(&metadata),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            position: pos,
            size,
            block_buffer,
            light_buffer,
            metadata_buffer,
            bind_group: None,
            dirty: false,
        }
    }

    /// Update GPU buffers from CPU chunk data
    pub fn update(&mut self, queue: &wgpu::Queue, chunk: &Chunk) {
        // Update block data
        let blocks = chunk.blocks();
        queue.write_buffer(&self.block_buffer, 0, bytemuck::cast_slice(&blocks));

        // TODO: Update light data when available

        self.dirty = false;
    }

    /// Create bind group for compute shaders
    pub fn create_bind_group(&mut self, device: &wgpu::Device, layout: &wgpu::BindGroupLayout) {
        self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GPU Chunk Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.metadata_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.block_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.light_buffer.as_entire_binding(),
                },
            ],
        }));
    }

    /// Get the bind group for rendering/compute
    pub fn bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.bind_group.as_ref()
    }

    /// Mark chunk as dirty (needs update from CPU)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Check if chunk needs update
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn position(&self) -> ChunkPos {
        self.position
    }
}

/// Metadata structure for GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ChunkMetadata {
    position: [i32; 4], // xyz + padding for alignment
    size: u32,
    block_count: u32,
    flags: u32,
    _padding: u32,
}

/// Manager for GPU chunks
pub struct GpuChunkManager {
    chunks: std::collections::HashMap<ChunkPos, GpuChunk>,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuChunkManager {
    pub fn new(device: &wgpu::Device) -> Self {
        // Create bind group layout for chunks
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GPU Chunk Bind Group Layout"),
            entries: &[
                // Metadata
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Block data
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Light data
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        Self {
            chunks: std::collections::HashMap::new(),
            bind_group_layout,
        }
    }

    /// Add or update a chunk on GPU
    pub fn update_chunk(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, chunk: &Chunk) {
        let pos = chunk.position();

        if let Some(gpu_chunk) = self.chunks.get_mut(&pos) {
            // Update existing chunk
            gpu_chunk.update(queue, chunk);
        } else {
            // Create new GPU chunk
            let mut gpu_chunk = GpuChunk::new_from_chunk(device, chunk);
            gpu_chunk.create_bind_group(device, &self.bind_group_layout);
            self.chunks.insert(pos, gpu_chunk);
        }
    }

    /// Remove a chunk from GPU
    pub fn remove_chunk(&mut self, pos: ChunkPos) {
        self.chunks.remove(&pos);
    }

    /// Get a GPU chunk
    pub fn get_chunk(&self, pos: &ChunkPos) -> Option<&GpuChunk> {
        self.chunks.get(pos)
    }

    /// Get mutable GPU chunk
    pub fn get_chunk_mut(&mut self, pos: &ChunkPos) -> Option<&mut GpuChunk> {
        self.chunks.get_mut(pos)
    }

    /// Get bind group layout for creating pipelines
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Get statistics
    pub fn stats(&self) -> GpuChunkStats {
        let chunk_count = self.chunks.len();
        let total_blocks = chunk_count * VOXELS_PER_CHUNK as usize;
        let memory_usage = chunk_count
            * (
                VOXELS_PER_CHUNK as usize * 2 + // blocks
            VOXELS_PER_CHUNK as usize * 2 + // light
            16
                // metadata
            );

        GpuChunkStats {
            chunk_count,
            total_blocks,
            memory_usage_bytes: memory_usage,
        }
    }
}

#[derive(Debug)]
pub struct GpuChunkStats {
    pub chunk_count: usize,
    pub total_blocks: usize,
    pub memory_usage_bytes: usize,
}
