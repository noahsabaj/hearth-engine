use crate::constants::core::{CHUNK_SIZE, MAX_WORLD_SIZE, VOXELS_PER_CHUNK};
use crate::constants::buffer_layouts::*;
use crate::gpu::buffer_layouts::{bindings, calculations, layouts, usage};
use crate::constants::gpu_limits;
use crate::morton::morton_encode;
use crate::world::core::ChunkPos;
use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Packed voxel data format for GPU storage
/// Uses 32 bits per voxel:
/// - Bits 0-15: Block ID (64K block types)
/// - Bits 16-19: Light level (0-15)
/// - Bits 20-23: Sky light level (0-15)
/// - Bits 24-27: Metadata (flags, rotation, etc)
/// - Bits 28-31: Reserved
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct VoxelData(pub u32);

impl VoxelData {
    pub const AIR: Self = Self(0);

    #[inline]
    pub fn new(block_id: u16, light: u8, sky_light: u8, metadata: u8) -> Self {
        let packed = (block_id as u32)
            | ((light as u32 & 0xF) << 16)
            | ((sky_light as u32 & 0xF) << 20)
            | ((metadata as u32 & 0xF) << 24);
        Self(packed)
    }

    #[inline]
    pub fn block_id(&self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }

    #[inline]
    pub fn light_level(&self) -> u8 {
        ((self.0 >> 16) & 0xF) as u8
    }

    #[inline]
    pub fn sky_light_level(&self) -> u8 {
        ((self.0 >> 20) & 0xF) as u8
    }

    #[inline]
    pub fn metadata(&self) -> u8 {
        ((self.0 >> 24) & 0xF) as u8
    }
}

/// Descriptor for creating a WorldBuffer
pub struct WorldBufferDescriptor {
    /// View distance in chunks (determines buffer size)
    pub view_distance: u32,
    /// Enable atomic operations for modifications
    pub enable_atomics: bool,
    /// Enable readback for debugging
    pub enable_readback: bool,
}

impl Default for WorldBufferDescriptor {
    fn default() -> Self {
        Self {
            // Use view distance to determine buffer size (safe for GPU limits)
            view_distance: recommended_view_distance(256), // Conservative default for 256MB GPUs
            enable_atomics: true,
            enable_readback: cfg!(debug_assertions),
        }
    }
}

/// GPU-resident world buffer containing all voxel data
pub struct WorldBuffer {
    device: Arc<wgpu::Device>,

    /// Main voxel storage buffer
    voxel_buffer: wgpu::Buffer,

    /// Chunk metadata buffer (loaded/generated flags, timestamps, etc)
    metadata_buffer: wgpu::Buffer,

    /// Staging buffer for CPU->GPU uploads (if needed)
    staging_buffer: Option<wgpu::Buffer>,

    /// Bind group for compute shaders
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,

    /// Maximum chunks that can be loaded (based on view distance)
    max_chunks: u32,
    /// View distance in chunks
    view_distance: u32,
    total_voxels: u64,

    /// Chunk slot management: maps chunk position to buffer slot index
    /// Protected by mutex to prevent race conditions during parallel generation
    chunk_slots: Arc<Mutex<HashMap<ChunkPos, u32>>>,
    /// Next available slot (simple round-robin allocation)
    /// Protected by same mutex as chunk_slots
    next_slot: Arc<Mutex<u32>>,
}

impl WorldBuffer {
    pub fn new(device: Arc<wgpu::Device>, desc: &WorldBufferDescriptor) -> Self {
        let view_distance = desc.view_distance;

        // Calculate maximum chunks based on view distance
        // Use sphere approximation: chunks within view_distance radius
        // Conservative estimate: (2 * view_distance + 1)³ to ensure we have enough space
        let diameter = 2 * view_distance + 1;
        let max_chunks = diameter * diameter * diameter;

        // Safety check: prevent massive allocations
        let memory_mb = (max_chunks as u64 * CHUNK_BUFFER_SLOT_SIZE) / (1024 * 1024);
        let memory_bytes = max_chunks as u64 * CHUNK_BUFFER_SLOT_SIZE;

        // GPU binding limit check
        if memory_bytes > gpu_limits::MAX_BUFFER_BINDING_SIZE {
            log::error!("WorldBuffer: view_distance {} would require {} MB which exceeds GPU binding limit of {} MB", 
                       view_distance, memory_mb, gpu_limits::MAX_BUFFER_BINDING_SIZE / (1024 * 1024));

            // Calculate maximum safe view distance
            let max_safe_chunks = gpu_limits::MAX_BUFFER_BINDING_SIZE / CHUNK_BUFFER_SLOT_SIZE;
            let max_safe_diameter = (max_safe_chunks as f64).powf(1.0 / 3.0).floor() as u32;
            let max_safe_view_distance = (max_safe_diameter - 1) / 2;

            panic!("WorldBuffer: Reduce view_distance to {} or less (current: {}, required: {} MB, limit: {} MB)", 
                   max_safe_view_distance, view_distance, memory_mb, gpu_limits::MAX_BUFFER_BINDING_SIZE / (1024 * 1024));
        }

        if memory_mb > 4096 {
            panic!("WorldBuffer: view_distance {} would require {} MB GPU memory (max 4096MB recommended)", 
                   view_distance, memory_mb);
        }

        log::info!(
            "Creating WorldBuffer with view_distance {} ({} max chunks, {} MB)",
            view_distance,
            max_chunks,
            memory_mb
        );

        let total_voxels = max_chunks as u64 * VOXELS_PER_CHUNK as u64;
        let buffer_size = max_chunks as u64 * CHUNK_BUFFER_SLOT_SIZE;

        // Main voxel buffer
        let usage = if desc.enable_readback {
            usage::STORAGE_READ
        } else {
            usage::STORAGE
        };

        let voxel_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("World Voxel Buffer"),
            size: buffer_size,
            usage,
            mapped_at_creation: false,
        });

        // Chunk metadata buffer
        let metadata_size = max_chunks as u64 * CHUNK_METADATA_SIZE;
        let metadata_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chunk Metadata Buffer"),
            size: metadata_size,
            usage: usage::STORAGE,
            mapped_at_creation: false,
        });

        // Optional staging buffer for uploads
        let staging_buffer = if desc.enable_readback {
            Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("World Staging Buffer"),
                size: CHUNK_BUFFER_SLOT_SIZE,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }))
        } else {
            None
        };

        // Create bind group layout using centralized definitions
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("World Buffer Bind Group Layout"),
            entries: &[
                layouts::storage_buffer_entry(
                    bindings::world::VOXEL_BUFFER,
                    false,
                    wgpu::ShaderStages::COMPUTE
                        | wgpu::ShaderStages::VERTEX
                        | wgpu::ShaderStages::FRAGMENT,
                ),
                layouts::storage_buffer_entry(
                    bindings::world::METADATA_BUFFER,
                    false,
                    wgpu::ShaderStages::COMPUTE,
                ),
            ],
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("World Buffer Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: bindings::world::VOXEL_BUFFER,
                    resource: voxel_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: bindings::world::METADATA_BUFFER,
                    resource: metadata_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            device,
            voxel_buffer,
            metadata_buffer,
            staging_buffer,
            bind_group,
            bind_group_layout,
            max_chunks,
            view_distance,
            total_voxels,
            chunk_slots: Arc::new(Mutex::new(HashMap::new())),
            next_slot: Arc::new(Mutex::new(0)),
        }
    }

    /// Get the bind group for use in compute/render passes
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    /// Get the bind group layout for pipeline creation
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Get the voxel buffer (for custom bind groups)
    pub fn voxel_buffer(&self) -> &wgpu::Buffer {
        &self.voxel_buffer
    }

    /// Get the metadata buffer (for custom bind groups)
    pub fn metadata_buffer(&self) -> &wgpu::Buffer {
        &self.metadata_buffer
    }

    /// Get the view distance
    pub fn view_distance(&self) -> u32 {
        self.view_distance
    }

    /// Get the maximum chunks this buffer can hold
    pub fn max_chunks(&self) -> u32 {
        self.max_chunks
    }

    /// Get or allocate a buffer slot for a chunk position
    /// CRITICAL: Prevents slot collisions that cause GPU readback failures
    pub fn get_chunk_slot(&mut self, chunk_pos: ChunkPos) -> u32 {
        log::debug!("[WORLD_BUFFER::get_chunk_slot] Called for chunk {:?}", chunk_pos);
        // Lock both mutexes to ensure thread safety
        let mut chunk_slots = match self.chunk_slots.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::warn!("[WORLD_BUFFER] chunk_slots mutex was poisoned, recovering");
                poisoned.into_inner()
            }
        };
        let mut next_slot = match self.next_slot.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::warn!("[WORLD_BUFFER] next_slot mutex was poisoned, recovering");
                poisoned.into_inner()
            }
        };
        
        if let Some(&slot) = chunk_slots.get(&chunk_pos) {
            log::debug!(
                "[WORLD_BUFFER] Reusing existing slot {} for chunk {:?}",
                slot,
                chunk_pos
            );
            slot
        } else {
            // CRITICAL FIX: Find an unused slot instead of immediately overwriting
            let mut slot = *next_slot % self.max_chunks;
            let mut attempts = 0;

            // Try to find an unused slot first to prevent race conditions
            while attempts < self.max_chunks {
                let slot_in_use = chunk_slots.iter().any(|(_, &s)| s == slot);
                if !slot_in_use {
                    log::debug!(
                        "[WORLD_BUFFER] Found unused slot {} for chunk {:?}",
                        slot,
                        chunk_pos
                    );
                    break;
                }
                slot = (slot + 1) % self.max_chunks;
                attempts += 1;
            }

            // If all slots are used, only then overwrite the oldest one
            if attempts >= self.max_chunks {
                slot = *next_slot % self.max_chunks;
                // Remove old chunk mapping for this slot
                let old_chunk =
                    chunk_slots
                        .iter()
                        .find_map(|(pos, &s)| if s == slot { Some(*pos) } else { None });
                if let Some(old_pos) = old_chunk {
                    log::warn!("[WORLD_BUFFER] SLOT COLLISION: Evicting chunk {:?} from slot {} for chunk {:?} (buffer full)", 
                              old_pos, slot, chunk_pos);
                    chunk_slots.remove(&old_pos);
                }
            }

            // Map new chunk to slot
            chunk_slots.insert(chunk_pos, slot);
            *next_slot = (slot + 1) % self.max_chunks;

            log::debug!(
                "[WORLD_BUFFER] Allocated slot {} for chunk {:?} (usage: {}/{})",
                slot,
                chunk_pos,
                chunk_slots.len(),
                self.max_chunks
            );

            slot
        }
    }

    /// Calculate buffer offset for a chunk slot
    pub fn slot_offset(&self, slot: u32) -> u64 {
        calculations::chunk_slot_offset(slot)
    }

    /// Upload a single chunk from CPU (migration path)
    pub fn upload_chunk(&mut self, queue: &wgpu::Queue, chunk_pos: ChunkPos, voxels: &[VoxelData]) {
        let start = Instant::now();

        assert_eq!(
            voxels.len(),
            VOXELS_PER_CHUNK as usize,
            "[WORLD_BUFFER] Invalid voxel count for chunk {:?}: expected {}, got {}",
            chunk_pos,
            VOXELS_PER_CHUNK,
            voxels.len()
        );

        log::info!(
            "[WORLD_BUFFER] Uploading chunk {:?} to GPU ({} voxels)",
            chunk_pos,
            voxels.len()
        );

        // Count non-air voxels for diagnostics
        let non_air_count = voxels.iter().filter(|v| v.block_id() != 0).count();
        let fill_percentage = (non_air_count as f64 / voxels.len() as f64) * 100.0;

        let slot = self.get_chunk_slot(chunk_pos);
        let offset = self.slot_offset(slot);
        let upload_size = voxels.len() * std::mem::size_of::<VoxelData>();

        log::debug!(
            "[WORLD_BUFFER] Upload details: slot {}, offset {} bytes, size {} bytes",
            slot,
            offset,
            upload_size
        );
        log::debug!(
            "[WORLD_BUFFER] Chunk content: {} non-air voxels ({:.1}% filled)",
            non_air_count,
            fill_percentage
        );

        let upload_start = Instant::now();
        queue.write_buffer(&self.voxel_buffer, offset, bytemuck::cast_slice(voxels));
        let upload_duration = upload_start.elapsed();

        let total_duration = start.elapsed();

        // Calculate upload bandwidth
        let bandwidth_mbps = if upload_duration.as_secs_f64() > 0.0 {
            (upload_size as f64 / upload_duration.as_secs_f64()) / (1024.0 * 1024.0)
        } else {
            0.0
        };

        log::info!(
            "[WORLD_BUFFER] Chunk {:?} upload completed: {:.2}ms total, {:.1} MB/s bandwidth",
            chunk_pos,
            total_duration.as_secs_f64() * 1000.0,
            bandwidth_mbps
        );
    }

    /// Clear a chunk to air
    pub fn clear_chunk(&mut self, encoder: &mut wgpu::CommandEncoder, chunk_pos: ChunkPos) {
        let start = Instant::now();

        log::debug!("[WORLD_BUFFER] Clearing chunk {:?} to air", chunk_pos);

        let slot = self.get_chunk_slot(chunk_pos);
        let offset = self.slot_offset(slot);
        let size = CHUNK_BUFFER_SLOT_SIZE;

        log::debug!(
            "[WORLD_BUFFER] Clear operation: slot {}, offset {} bytes, size {} bytes",
            slot,
            offset,
            size
        );

        encoder.clear_buffer(&self.voxel_buffer, offset, Some(size));

        let duration = start.elapsed();
        log::debug!(
            "[WORLD_BUFFER] Chunk {:?} clear operation queued in {:.1}μs",
            chunk_pos,
            duration.as_micros()
        );
    }

    /// Get total voxel count
    pub fn total_voxels(&self) -> u64 {
        self.total_voxels
    }

    /// Get buffer size in bytes
    pub fn buffer_size(&self) -> u64 {
        self.total_voxels * std::mem::size_of::<VoxelData>() as u64
    }

    /// Read chunk data from GPU buffer
    /// This is the critical missing piece for GPU→CPU data extraction
    pub fn read_chunk(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        chunk_pos: ChunkPos,
    ) -> Result<Vec<VoxelData>, Box<dyn std::error::Error>> {
        let overall_start = Instant::now();

        log::info!(
            "[WORLD_BUFFER] Starting GPU→CPU readback for chunk {:?}",
            chunk_pos
        );

        // Check staging buffer exists
        if self.staging_buffer.is_none() {
            let error_msg = "WorldBuffer readback not enabled - missing staging buffer";
            log::error!("[WORLD_BUFFER] {}", error_msg);
            return Err(error_msg.into());
        }

        // Get chunk slot and calculate source offset
        let slot = self.get_chunk_slot(chunk_pos);
        let source_offset = self.slot_offset(slot);
        let chunk_size_bytes = VOXELS_PER_CHUNK as u64 * std::mem::size_of::<VoxelData>() as u64;

        // Get staging buffer reference after mutable operations
        let staging_buffer = match self.staging_buffer.as_ref() {
            Some(buffer) => buffer,
            None => {
                let error_msg = "WorldBuffer staging buffer unexpectedly None after check";
                log::error!("[WORLD_BUFFER] {}", error_msg);
                return Err(error_msg.into());
            }
        };

        log::info!("[WORLD_BUFFER] GPU readback details: chunk {:?} from slot {} at offset {} bytes (size: {} bytes)", 
                  chunk_pos, slot, source_offset, chunk_size_bytes);

        // Create command encoder for copy operation
        let encoder_start = Instant::now();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("WorldBuffer Readback"),
        });

        log::debug!("[WORLD_BUFFER] Copying GPU buffer to staging buffer...");

        // Copy GPU buffer data to staging buffer
        encoder.copy_buffer_to_buffer(
            &self.voxel_buffer, // source: main GPU buffer
            source_offset,      // source offset for this chunk
            staging_buffer,     // destination: CPU-readable staging buffer
            0,                  // staging buffer offset (always 0 for single chunk)
            chunk_size_bytes,   // size: one chunk worth of voxel data
        );

        let encoder_duration = encoder_start.elapsed();

        // Submit copy command and wait for completion
        let submit_start = Instant::now();
        queue.submit(std::iter::once(encoder.finish()));
        let submit_duration = submit_start.elapsed();

        log::debug!(
            "[WORLD_BUFFER] GPU copy command submitted (encode: {:.1}μs, submit: {:.1}μs)",
            encoder_duration.as_micros(),
            submit_duration.as_micros()
        );

        // Map staging buffer for reading
        let mapping_start = Instant::now();
        log::debug!("[WORLD_BUFFER] Mapping staging buffer for CPU read access...");

        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        // Poll device until mapping completes
        let poll_start = Instant::now();
        device.poll(wgpu::Maintain::Wait);
        let poll_duration = poll_start.elapsed();

        let map_result = receiver
            .recv()
            .map_err(|_| "Failed to receive mapping result")?
            .map_err(|e| format!("Buffer mapping failed: {:?}", e))?;

        let mapping_duration = mapping_start.elapsed();
        log::debug!(
            "[WORLD_BUFFER] Buffer mapping completed (poll: {:.1}ms, total: {:.1}ms)",
            poll_duration.as_secs_f64() * 1000.0,
            mapping_duration.as_secs_f64() * 1000.0
        );

        // Read the mapped data
        let read_start = Instant::now();
        let mapped_data = buffer_slice.get_mapped_range();
        let voxel_data: &[VoxelData] = bytemuck::cast_slice(&mapped_data);

        log::debug!(
            "[WORLD_BUFFER] Reading {} voxels from mapped buffer...",
            voxel_data.len()
        );

        // Copy data out before unmapping
        let copy_start = Instant::now();
        let result: Vec<VoxelData> = voxel_data.to_vec();
        let copy_duration = copy_start.elapsed();

        // Analyze the data for diagnostics
        let non_air_count = result.iter().filter(|v| v.block_id() != 0).count();
        let fill_percentage = (non_air_count as f64 / result.len() as f64) * 100.0;

        // Unmap the buffer
        drop(mapped_data);
        staging_buffer.unmap();

        let read_duration = read_start.elapsed();
        let total_duration = overall_start.elapsed();

        // Calculate readback performance metrics
        let readback_bandwidth = if total_duration.as_secs_f64() > 0.0 {
            (chunk_size_bytes as f64 / total_duration.as_secs_f64()) / (1024.0 * 1024.0)
        } else {
            0.0
        };

        log::info!("[WORLD_BUFFER] GPU→CPU readback completed for chunk {:?}: {} voxels ({} non-air, {:.1}% filled)", 
                  chunk_pos, result.len(), non_air_count, fill_percentage);

        log::debug!("[WORLD_BUFFER] Readback performance: {:.2}ms total, {:.1} MB/s bandwidth (copy: {:.1}μs, read: {:.1}ms)", 
                   total_duration.as_secs_f64() * 1000.0, readback_bandwidth,
                   copy_duration.as_micros(), read_duration.as_secs_f64() * 1000.0);

        // Warn if readback is slow
        if total_duration.as_millis() > 10 {
            log::warn!(
                "[WORLD_BUFFER] Slow GPU→CPU readback for chunk {:?}: {:.2}ms (expected <10ms)",
                chunk_pos,
                total_duration.as_secs_f64() * 1000.0
            );
        }

        Ok(result)
    }

    /// Read chunk data from GPU buffer (blocking)
    /// This is an alias for read_chunk for backward compatibility
    pub fn read_chunk_blocking(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        chunk_pos: ChunkPos,
    ) -> Result<Vec<VoxelData>, Box<dyn std::error::Error>> {
        self.read_chunk(device, queue, chunk_pos)
    }
}
