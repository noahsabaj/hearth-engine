use super::{SparseVoxelOctree, VoxelBvh};
use crate::error::EngineError;
use crate::memory::MemoryManager;
use crate::world::error::WorldGpuResult;
use crate::world::storage::WorldBuffer;
use bytemuck::{Pod, Zeroable};
/// Hierarchical Physics Queries

use std::sync::Arc;
use wgpu::{Buffer, ComputePipeline, Device, Queue};

/// Physics query types
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum QueryType {
    /// Ray cast for line-of-sight checks
    RayCast = 0,
    /// Sphere cast for projectiles
    SphereCast = 1,
    /// Box cast for character movement
    BoxCast = 2,
    /// Frustum query for visibility culling
    FrustumQuery = 3,
    /// Overlap test for triggers
    OverlapTest = 4,
}

/// Physics query on GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PhysicsQuery {
    /// Query type
    pub query_type: u32,
    /// Origin or center
    pub origin: [f32; 3],
    /// Max distance for casts
    pub max_distance: f32,
    /// Direction for casts (normalized)
    pub direction: [f32; 3],
    /// Radius for sphere cast
    pub radius: f32,
    /// Box half-extents
    pub half_extents: [f32; 3],
    /// Query flags
    pub flags: u32,
}

/// Physics query result
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct QueryResult {
    /// Hit distance (-1 if no hit)
    pub hit_distance: f32,
    /// Hit position
    pub hit_position: [f32; 3],
    /// Hit normal
    pub hit_normal: [f32; 3],
    /// Hit block ID
    pub block_id: u32,
    /// Hit chunk index
    pub chunk_index: u32,
    /// Padding
    pub _padding: [u32; 3],
}

/// Hierarchical physics system
pub struct HierarchicalPhysics {
    device: Arc<Device>,

    /// Ray cast pipeline
    raycast_pipeline: ComputePipeline,

    /// Sphere cast pipeline
    spherecast_pipeline: ComputePipeline,

    /// Box cast pipeline
    boxcast_pipeline: ComputePipeline,

    /// Overlap test pipeline
    overlap_pipeline: ComputePipeline,

    /// Query buffer
    query_buffer: Buffer,
    query_capacity: u32,

    /// Result buffer
    result_buffer: Buffer,

    /// Bind group layout
    bind_group_layout: wgpu::BindGroupLayout,
}

impl HierarchicalPhysics {
    pub fn new(device: Arc<Device>, memory_manager: &mut MemoryManager, max_queries: u32) -> Self {
        // Create shader module using unified GPU system
        let shader_source = include_str!("../../shaders/compute/hierarchical_physics.wgsl");
        let validated_shader = match crate::gpu::automation::create_gpu_shader(
            &device,
            "hierarchical_physics",
            shader_source,
        ) {
            Ok(shader) => shader,
            Err(e) => {
                log::error!("Failed to create hierarchical physics shader: {}", e);
                panic!("Failed to create hierarchical physics shader: {}", e);
            }
        };

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Physics Query Layout"),
            entries: &[
                // World voxels
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Octree
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // BVH
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Queries
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Results
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Physics Query Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create pipelines
        let raycast_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Ray Cast Pipeline"),
            layout: Some(&pipeline_layout),
            module: &validated_shader.module,
            entry_point: "raycast_query",
        });

        let spherecast_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Sphere Cast Pipeline"),
                layout: Some(&pipeline_layout),
                module: &validated_shader.module,
                entry_point: "spherecast_query",
            });

        let boxcast_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Box Cast Pipeline"),
            layout: Some(&pipeline_layout),
            module: &validated_shader.module,
            entry_point: "boxcast_query",
        });

        let overlap_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Overlap Test Pipeline"),
            layout: Some(&pipeline_layout),
            module: &validated_shader.module,
            entry_point: "overlap_query",
        });

        // Allocate buffers
        let query_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Physics Query Buffer"),
            size: max_queries as u64 * std::mem::size_of::<PhysicsQuery>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let result_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Physics Result Buffer"),
            size: max_queries as u64 * std::mem::size_of::<QueryResult>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Self {
            device,
            raycast_pipeline,
            spherecast_pipeline,
            boxcast_pipeline,
            overlap_pipeline,
            query_buffer,
            query_capacity: max_queries,
            result_buffer,
            bind_group_layout,
        }
    }

    /// Execute physics queries
    pub fn execute_queries(
        &self,
        queue: &Queue,
        encoder: &mut wgpu::CommandEncoder,
        world_buffer: &WorldBuffer,
        octree: &SparseVoxelOctree,
        bvh: &VoxelBvh,
        queries: &[PhysicsQuery],
    ) {
        if queries.is_empty() || queries.len() > self.query_capacity as usize {
            return;
        }

        // Upload queries
        queue.write_buffer(&self.query_buffer, 0, bytemuck::cast_slice(queries));

        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Physics Query Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: world_buffer.voxel_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: octree.node_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: bvh.node_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.query_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.result_buffer.as_entire_binding(),
                },
            ],
        });

        // Group queries by type for better GPU utilization
        let mut raycast_count = 0;
        let mut spherecast_count = 0;
        let mut boxcast_count = 0;
        let mut overlap_count = 0;

        for query in queries {
            match query.query_type {
                0 => raycast_count += 1,
                1 => spherecast_count += 1,
                2 => boxcast_count += 1,
                4 => overlap_count += 1,
                _ => {}
            }
        }

        // Execute each query type
        if raycast_count > 0 {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Ray Cast Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.raycast_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups((raycast_count + 63) / 64, 1, 1);
        }

        if spherecast_count > 0 {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Sphere Cast Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.spherecast_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups((spherecast_count + 63) / 64, 1, 1);
        }

        if boxcast_count > 0 {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Box Cast Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.boxcast_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups((boxcast_count + 63) / 64, 1, 1);
        }

        if overlap_count > 0 {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Overlap Test Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.overlap_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups((overlap_count + 63) / 64, 1, 1);
        }
    }

    /// Read back query results
    pub async fn read_results(
        &self,
        device: &Device,
        queue: &Queue,
        count: usize,
    ) -> WorldGpuResult<Vec<QueryResult>> {
        // Create staging buffer
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Physics Results Staging"),
            size: (count * std::mem::size_of::<QueryResult>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Copy results to staging
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Physics Results Copy"),
        });

        encoder.copy_buffer_to_buffer(
            &self.result_buffer,
            0,
            &staging_buffer,
            0,
            (count * std::mem::size_of::<QueryResult>()) as u64,
        );

        queue.submit(std::iter::once(encoder.finish()));

        // Map and read
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        device.poll(wgpu::Maintain::Wait);
        let map_result = receiver
            .await
            .map_err(|_| EngineError::SystemError {
                component: "world_gpu".to_string(),
                error: "Failed to receive mapping result".to_string(),
            })?
            .map_err(|_| EngineError::BufferError {
                operation: "physics_results".to_string(),
                error: "Failed to map buffer for reading".to_string(),
            })?;

        let data = buffer_slice.get_mapped_range();
        let results: Vec<QueryResult> = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        staging_buffer.unmap();

        Ok(results)
    }
}
