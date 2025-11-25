//! SOA-optimized terrain generator
//!
//! This module provides a Structure of Arrays version of the terrain generator
//! for maximum GPU performance and memory bandwidth efficiency.

use crate::gpu::types::terrain::TerrainParams;
use crate::constants::core::CHUNK_SIZE;
use crate::gpu::{
    buffer_layouts::{bindings, layouts, usage},
    soa::{
        BlockDistributionSOA, BufferLayoutPreference, CpuGpuBridge, SoaBufferBuilder,
        TerrainParamsSOA, UnifiedGpuBuffer,
    },
    types::TypedGpuBuffer,
    GpuBufferManager, GpuError,
};
use crate::world::core::ChunkPos;
use crate::world::storage::WorldBuffer;
use std::sync::Arc;
use std::time::Instant;
use wgpu::util::DeviceExt;

/// SOA-optimized GPU terrain generator
pub struct TerrainGeneratorSOA {
    device: Arc<wgpu::Device>,

    /// GPU buffer manager
    buffer_manager: Arc<GpuBufferManager>,

    /// Shader module (must be kept alive for pipeline to remain valid)
    _shader_module: wgpu::ShaderModule,

    /// Compute pipeline for SOA terrain generation
    generate_pipeline: wgpu::ComputePipeline,

    /// SOA parameters buffer
    params_buffer: TypedGpuBuffer<TerrainParamsSOA>,

    /// Bind group layout for SOA terrain generation
    bind_group_layout: wgpu::BindGroupLayout,

    /// Whether to use vectorized shader variant
    use_vectorized: bool,
}

impl TerrainGeneratorSOA {
    /// Validate that a shader entry point exists in the shader source
    fn validate_shader_entry_point(shader_source: &str, entry_point: &str) -> Result<(), String> {
        // Check for the entry point function definition
        let fn_pattern = format!("fn {}(", entry_point);
        if !shader_source.contains(&fn_pattern) {
            return Err(format!(
                "Entry point '{}' not found in shader. Available functions: {}",
                entry_point,
                Self::extract_function_names(shader_source).join(", ")
            ));
        }

        // Check for @compute annotation
        let lines: Vec<&str> = shader_source.lines().collect();
        let mut found_entry_point = false;
        let mut has_compute_annotation = false;

        for (i, line) in lines.iter().enumerate() {
            if line.contains(&fn_pattern) {
                found_entry_point = true;
                // Check previous lines for @compute annotation
                for j in (0..i).rev() {
                    let prev_line = lines[j].trim();
                    if prev_line.is_empty() || prev_line.starts_with("//") {
                        continue;
                    }
                    if prev_line.contains("@compute") {
                        has_compute_annotation = true;
                        break;
                    }
                    // If we hit another function or non-annotation, stop looking
                    if prev_line.contains("fn ")
                        || (!prev_line.starts_with("@") && !prev_line.starts_with("//"))
                    {
                        break;
                    }
                }
                break;
            }
        }

        if !found_entry_point {
            return Err(format!("Entry point function '{}' not found", entry_point));
        }

        if !has_compute_annotation {
            return Err(format!(
                "Entry point '{}' found but missing @compute annotation. Compute shaders require @compute.",
                entry_point
            ));
        }

        log::debug!(
            "[TerrainGeneratorSOA] Shader validation passed for entry point: {}",
            entry_point
        );
        Ok(())
    }

    /// Extract function names from shader source for debugging
    fn extract_function_names(shader_source: &str) -> Vec<String> {
        let mut functions = Vec::new();
        for line in shader_source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("fn ") {
                if let Some(name_start) = trimmed.find("fn ").map(|i| i + 3) {
                    if let Some(name_end) = trimmed[name_start..].find('(') {
                        let name = trimmed[name_start..name_start + name_end].trim();
                        if !name.is_empty() {
                            functions.push(name.to_string());
                        }
                    }
                }
            }
        }
        functions
    }

    /// Validate shader contains required compute logic
    fn validate_compute_shader(shader_source: &str) -> Result<(), String> {
        let mut issues = Vec::new();

        // Check for compute logic patterns (not bindings, which are auto-generated)
        if !shader_source.contains("world_data[") {
            issues.push("Shader missing world_data array access pattern".to_string());
        }

        if !shader_source.contains("pack_voxel") {
            issues.push("Shader missing pack_voxel function".to_string());
        }

        if !issues.is_empty() {
            return Err(format!(
                "Compute shader validation failed: {}",
                issues.join(", ")
            ));
        }

        log::debug!("[TerrainGeneratorSOA] Compute shader validation passed");
        Ok(())
    }

    /// Create compute pipeline with comprehensive validation and error handling
    fn create_compute_pipeline_with_validation(
        device: &wgpu::Device,
        pipeline_layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
        entry_point: &str,
    ) -> Result<wgpu::ComputePipeline, String> {
        log::debug!(
            "[TerrainGeneratorSOA] Attempting to create compute pipeline with entry point: {}",
            entry_point
        );

        // Create pipeline descriptor
        let descriptor = wgpu::ComputePipelineDescriptor {
            label: Some("SOA Terrain Generation Pipeline"),
            layout: Some(pipeline_layout),
            module: shader,
            entry_point,
        };

        // Attempt pipeline creation
        // Note: wgpu doesn't return Result from create_compute_pipeline, but it can panic
        // We'll use std::panic::catch_unwind to catch any panics during creation
        let pipeline_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            device.create_compute_pipeline(&descriptor)
        }));

        match pipeline_result {
            Ok(pipeline) => {
                log::debug!("[TerrainGeneratorSOA] Compute pipeline created successfully");

                // Additional validation - check pipeline is not null/invalid
                // wgpu pipelines don't have a direct "is_valid" method, but we can check basic properties
                log::debug!("[TerrainGeneratorSOA] Pipeline validation complete");
                Ok(pipeline)
            }
            Err(panic_payload) => {
                let error_msg = if let Some(s) = panic_payload.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Unknown panic during pipeline creation".to_string()
                };

                Err(format!("Pipeline creation panicked: {}", error_msg))
            }
        }
    }

    /// Create a new SOA terrain generator with its own buffer manager
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Result<Self, GpuError> {
        let buffer_manager = Arc::new(GpuBufferManager::new(device.as_ref(), queue.as_ref()));
        Self::new_with_manager(device, buffer_manager, false)
    }

    /// Create a new SOA terrain generator
    pub fn new_with_manager(
        device: Arc<wgpu::Device>,
        buffer_manager: Arc<GpuBufferManager>,
        use_vectorized: bool,
    ) -> Result<Self, GpuError> {
        log::info!("[TerrainGeneratorSOA] Initializing SOA-optimized terrain generator");
        log::info!("[TerrainGeneratorSOA] Vectorized mode: {}", use_vectorized);

        // Log SOA sizes for debugging
        log::info!(
            "[TerrainGeneratorSOA] BlockDistributionSOA size: {} bytes",
            std::mem::size_of::<BlockDistributionSOA>()
        );
        log::info!(
            "[TerrainGeneratorSOA] TerrainParamsSOA size: {} bytes",
            std::mem::size_of::<TerrainParamsSOA>()
        );

        // Load shader code - contains ONLY compute logic, no types/bindings/constants
        let shader_code = include_str!("../../shaders/compute/terrain_generation.wgsl");

        log::info!("[TerrainGeneratorSOA] Creating shader through unified GPU system with error recovery");

        // Create error recovery system for shader creation
        let error_recovery = crate::gpu::error_recovery::GpuErrorRecovery::new(
            device.clone(),
            buffer_manager.queue().clone(),
        );
        
        // Create shader through unified system with error recovery
        let validated_shader = match error_recovery.execute_with_recovery(|| {
            crate::gpu::automation::create_gpu_shader(
                &device,
                "terrain_generation_soa",
                &shader_code,
            )
            .map_err(|e| crate::gpu::error_recovery::GpuRecoveryError::OperationFailed {
                message: format!("Shader creation failed: {:?}", e),
            })
        }) {
            Ok(shader) => shader,
            Err(e) => {
                log::error!("[TerrainGeneratorSOA] Failed to create shader with error recovery: {:?}", e);
                return Err(GpuError::ShaderCompilation {
                    message: format!("Failed to create terrain generation shader: {:?}", e),
                });
            }
        };

        // Extract the shader module to keep it alive
        let shader = validated_shader.module;

        // Create bind group layout for SOA shader using centralized definitions
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SOA Terrain Generator Bind Group Layout"),
            entries: &[
                // World buffer binding
                layouts::storage_buffer_entry(
                    bindings::world::VOXEL_BUFFER,
                    false,
                    wgpu::ShaderStages::COMPUTE,
                ),
                // Metadata buffer binding
                layouts::storage_buffer_entry(
                    bindings::world::METADATA_BUFFER,
                    false,
                    wgpu::ShaderStages::COMPUTE,
                ),
                // SOA Parameters buffer
                layouts::storage_buffer_entry(
                    bindings::world::PARAMS_BUFFER,
                    true,
                    wgpu::ShaderStages::COMPUTE,
                ),
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SOA Terrain Generation Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create compute pipeline with comprehensive error handling
        let entry_point = if use_vectorized {
            "generate_terrain_vectorized"
        } else {
            "generate_terrain"
        };

        log::info!("[TerrainGeneratorSOA] Using entry point: {}", entry_point);

        // Validate shader entry point exists before pipeline creation
        if let Err(e) = Self::validate_shader_entry_point(shader_code, entry_point) {
            log::error!("[TerrainGeneratorSOA] Shader validation failed: {}", e);
            return Err(GpuError::ShaderCompilation {
                message: format!("Shader validation failed: {}", e),
            });
        }

        // Validate shader contains required compute logic
        if let Err(e) = Self::validate_compute_shader(shader_code) {
            log::error!(
                "[TerrainGeneratorSOA] Compute shader validation failed: {}",
                e
            );
            return Err(GpuError::ShaderCompilation {
                message: format!("Compute shader validation failed: {}", e),
            });
        }

        // Log detailed pipeline creation parameters for debugging
        log::info!(
            "[TerrainGeneratorSOA] Creating compute pipeline - Entry: {}, Shader size: {} chars, Layout bindings: {}",
            entry_point,
            shader_code.len(),
            3  // We have 3 bindings: voxel_buffer, metadata_buffer, params_buffer
        );

        // Attempt pipeline creation with error recovery
        let generate_pipeline = match error_recovery.execute_with_recovery(|| {
            Self::create_compute_pipeline_with_validation(
                &device,
                &pipeline_layout,
                &shader,
                entry_point,
            )
            .map_err(|e| crate::gpu::error_recovery::GpuRecoveryError::OperationFailed {
                message: format!("Pipeline creation failed: {}", e),
            })
        }) {
            Ok(pipeline) => pipeline,
            Err(e) => {
                log::error!("[TerrainGeneratorSOA] Pipeline creation failed with error recovery: {:?}", e);
                log::error!(
                    "[TerrainGeneratorSOA] Shader source (first 500 chars): {}",
                    &shader_code[..shader_code.len().min(500)]
                );
                log::error!(
                    "[TerrainGeneratorSOA] Entry point requested: {}",
                    entry_point
                );
                return Err(GpuError::ShaderCompilation {
                    message: format!("Pipeline creation failed: {:?}", e),
                });
            }
        };

        log::info!("[TerrainGeneratorSOA] Compute pipeline created successfully");

        // Create SOA parameters buffer
        let default_params = TerrainParams::default();
        let soa_params = TerrainParamsSOA::from_aos(&default_params);

        log::info!("[TerrainGeneratorSOA] Creating SOA parameters buffer");

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SOA Terrain Parameters"),
            contents: bytemuck::bytes_of(&soa_params),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let params_buffer = TypedGpuBuffer::new(
            params_buffer,
            std::mem::size_of::<TerrainParamsSOA>() as u64,
        );

        log::info!("[TerrainGeneratorSOA] SOA terrain generator ready!");

        Ok(Self {
            device,
            buffer_manager,
            _shader_module: shader,
            generate_pipeline,
            params_buffer,
            bind_group_layout,
            use_vectorized,
        })
    }

    /// Update terrain parameters (converts from AOS to SOA)
    pub fn update_params(&self, params: &TerrainParams) -> Result<(), GpuError> {
        let queue = &self.buffer_manager.queue();

        // Convert AOS to SOA
        let soa_params = CpuGpuBridge::pack_terrain_params(params);

        // Update GPU buffer
        queue.write_buffer(
            &self.params_buffer.buffer,
            0,
            bytemuck::bytes_of(&soa_params),
        );

        log::debug!("[TerrainGeneratorSOA] Updated SOA parameters from AOS");
        Ok(())
    }

    /// Update terrain parameters directly with SOA data
    pub fn update_params_soa(&self, params: &TerrainParamsSOA) -> Result<(), GpuError> {
        let queue = &self.buffer_manager.queue();

        // Update GPU buffer directly with SOA data
        queue.write_buffer(&self.params_buffer.buffer, 0, bytemuck::bytes_of(params));

        log::debug!("[TerrainGeneratorSOA] Updated SOA parameters directly");
        Ok(())
    }

    /// Generate chunks using SOA layout
    /// Returns the metadata buffer to keep it alive until GPU work completes
    pub fn generate_chunks(
        &self,
        world_buffer: &mut WorldBuffer,
        chunk_positions: &[ChunkPos],
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<wgpu::Buffer, GpuError> {
        log::debug!("[TerrainGeneratorSOA::generate_chunks] Entry point reached");
        if chunk_positions.is_empty() {
            // Return a dummy buffer if no chunks to generate
            let dummy_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Empty metadata buffer"),
                size: 4,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            return Ok(dummy_buffer);
        }

        let start = Instant::now();
        let batch_size = chunk_positions.len();

        log::info!(
            "[TerrainGeneratorSOA] Generating {} chunks with SOA layout",
            batch_size
        );

        log::debug!("[TerrainGeneratorSOA] About to create metadata buffer");

        // Create metadata buffer for chunk generation
        // Each chunk needs a full ChunkMetadata struct (8 u32 values: 5 fields + 3 reserved)
        log::debug!("[TerrainGeneratorSOA] Starting metadata data creation...");
        let metadata_data: Vec<u32> = chunk_positions
            .iter()
            .enumerate()
            .flat_map(|(idx, pos)| {
                log::debug!(
                    "[TerrainGeneratorSOA] Processing chunk {:?} (index {})",
                    pos, idx
                );
                // Get the slot assignment from WorldBuffer
                let slot = world_buffer.get_chunk_slot(*pos);
                log::debug!(
                    "[TerrainGeneratorSOA] Chunk {:?} (index {}) assigned to slot {}",
                    pos, idx, slot
                );

                // Create ChunkMetadata for each chunk
                // Properly encode signed positions as 16-bit values
                let x_16bit = (pos.x as i16) as u16 as u32;
                let z_16bit = (pos.z as i16) as u16 as u32;
                let flags = (x_16bit << 16) | z_16bit;
                let timestamp = 0u32;
                let checksum = 0u32; // Proper checksum would be calculated from chunk data
                let y_position = pos.y as i32 as u32; // Preserve sign through i32
                let slot_index = slot;
                let _reserved = [0u32; 3];
                vec![
                    flags,
                    timestamp,
                    checksum,
                    y_position,
                    slot_index,
                    _reserved[0],
                    _reserved[1],
                    _reserved[2],
                ]
            })
            .collect();

        log::debug!(
            "[TerrainGeneratorSOA] Creating metadata buffer with {} u32 values ({} bytes) for {} chunks",
            metadata_data.len(),
            metadata_data.len() * std::mem::size_of::<u32>(),
            chunk_positions.len()
        );
        
        let metadata_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("SOA Chunk Metadata"),
                contents: bytemuck::cast_slice(&metadata_data),
                usage: usage::STORAGE,
            });

        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SOA Terrain Generation Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: bindings::world::VOXEL_BUFFER,
                    resource: world_buffer.voxel_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: bindings::world::METADATA_BUFFER,
                    resource: metadata_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: bindings::world::PARAMS_BUFFER,
                    resource: self.params_buffer.buffer.as_entire_binding(),
                },
            ],
        });

        // Record compute pass with comprehensive error handling
        {
            log::debug!(
                "[TerrainGeneratorSOA] Starting compute pass for {} chunks",
                chunk_positions.len()
            );

            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("SOA Terrain Generation Pass"),
                timestamp_writes: None,
            });

            // Validate pipeline before use
            log::debug!("[TerrainGeneratorSOA] Setting compute pipeline");
            log::debug!("[TerrainGeneratorSOA] Setting pipeline on compute pass");
            compute_pass.set_pipeline(&self.generate_pipeline);
            log::debug!("[TerrainGeneratorSOA] Pipeline set successfully");

            // Validate bind group before use
            log::debug!(
                "[TerrainGeneratorSOA] Setting bind group with {} entries",
                3
            );
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Calculate workgroups needed based on chunk size and workgroup size
            let chunk_size = CHUNK_SIZE;
            let workgroup_size_x = if self.use_vectorized { 16 } else { 8 };
            let workgroup_size_y = if self.use_vectorized { 2 } else { 4 };
            let workgroup_size_z = if self.use_vectorized { 2 } else { 4 };

            let workgroups_per_chunk_x = (chunk_size + workgroup_size_x - 1) / workgroup_size_x;
            let workgroups_per_chunk_y = (chunk_size + workgroup_size_y - 1) / workgroup_size_y;
            let workgroups_per_chunk_z = (chunk_size + workgroup_size_z - 1) / workgroup_size_z;

            // Total workgroups in X = workgroups per chunk * number of chunks
            let total_workgroups_x = workgroups_per_chunk_x * chunk_positions.len() as u32;

            log::debug!(
                "[TerrainGeneratorSOA] Dispatching workgroups for {} chunks: {} x {} x {} (total: {} workgroups)",
                chunk_positions.len(),
                total_workgroups_x, workgroups_per_chunk_y, workgroups_per_chunk_z,
                total_workgroups_x * workgroups_per_chunk_y * workgroups_per_chunk_z
            );

            // Dispatch all chunks at once - shader will calculate chunk index from workgroup_id.x
            compute_pass.dispatch_workgroups(
                total_workgroups_x,
                workgroups_per_chunk_y,
                workgroups_per_chunk_z,
            );

            log::debug!("[TerrainGeneratorSOA] Compute pass dispatch completed successfully");
        }

        let elapsed = start.elapsed();
        log::info!(
            "[TerrainGeneratorSOA] Generated {} chunks in {:?} ({} mode)",
            batch_size,
            elapsed,
            if self.use_vectorized {
                "vectorized"
            } else {
                "scalar"
            }
        );

        // Return the metadata buffer to keep it alive until GPU work completes
        Ok(metadata_buffer)
    }

    /// Generate a single chunk (convenience method)
    /// Returns the metadata buffer to keep it alive until GPU work completes
    pub fn generate_chunk(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        world_buffer: &mut WorldBuffer,
        chunk_pos: ChunkPos,
    ) -> wgpu::Buffer {
        log::debug!("[TerrainGeneratorSOA::generate_chunk] Generating single chunk {:?}", chunk_pos);
        self.generate_chunks(world_buffer, &[chunk_pos], encoder)
            .expect("Failed to generate chunk with SOA")
    }
}

/// Builder for creating SOA terrain generator with options
pub struct TerrainGeneratorSOABuilder {
    use_vectorized: bool,
}

impl TerrainGeneratorSOABuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            use_vectorized: false,
        }
    }

    /// Enable vectorized shader variant
    pub fn with_vectorization(mut self, enabled: bool) -> Self {
        self.use_vectorized = enabled;
        self
    }

    /// Build the SOA terrain generator
    pub fn build(
        self,
        device: Arc<wgpu::Device>,
        buffer_manager: Arc<GpuBufferManager>,
    ) -> Result<TerrainGeneratorSOA, GpuError> {
        TerrainGeneratorSOA::new_with_manager(device, buffer_manager, self.use_vectorized)
    }
}

impl Default for TerrainGeneratorSOABuilder {
    fn default() -> Self {
        Self::new()
    }
}
