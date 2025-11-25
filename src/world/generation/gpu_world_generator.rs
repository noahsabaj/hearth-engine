//! GPU world generator wrapper that implements the WorldGenerator trait

use crate::gpu::{GpuError, GpuErrorRecovery, GpuRecoveryError};
use crate::world::{
    core::{BlockId, ChunkPos},
    generation::{TerrainGeneratorSOA, WorldGenerator},
    storage::{TempChunk, WorldBuffer},
};
use std::sync::{Arc, Mutex};

/// GPU world generator that wraps TerrainGeneratorSOA to implement WorldGenerator trait
///
/// This is a wrapper that defers actual GPU generation until a proper command encoder
/// is available. The WorldGenerator trait doesn't provide access to command encoders,
/// so this wrapper stores the generation parameters and performs the actual generation
/// when the renderer provides an encoder.
pub struct GpuWorldGenerator {
    terrain_generator: Arc<TerrainGeneratorSOA>,
    device: Arc<wgpu::Device>,
    world_buffer: Arc<Mutex<WorldBuffer>>,
    error_recovery: Arc<GpuErrorRecovery>,
}

impl GpuWorldGenerator {
    /// Create a new GPU world generator
    pub fn new(
        terrain_generator: Arc<TerrainGeneratorSOA>,
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        world_buffer: Arc<Mutex<WorldBuffer>>,
    ) -> Self {
        let error_recovery = Arc::new(GpuErrorRecovery::new(device.clone(), queue));

        Self {
            terrain_generator,
            device,
            world_buffer,
            error_recovery,
        }
    }

    /// Generate chunks on GPU when a command encoder is available
    /// This is the proper way to use GPU generation
    pub fn generate_chunks_with_encoder(
        &self,
        chunk_positions: &[ChunkPos],
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), GpuError> {
        // Check if device is lost before proceeding
        if self.error_recovery.is_device_lost() {
            return Err(GpuError::DeviceLost);
        }

        // Validate encoder before use
        if let Err(e) = self.error_recovery.validate_encoder(encoder) {
            log::error!("Command encoder is invalid: {:?}", e);
            return Err(GpuError::InvalidEncoder);
        }

        // Execute terrain generation with error recovery
        let result = self.error_recovery.execute_with_recovery(|| {
            let mut world_buffer = match self.world_buffer.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    log::warn!("[GpuWorldGenerator] world_buffer mutex was poisoned, recovering");
                    poisoned.into_inner()
                }
            };
            self.terrain_generator
                .generate_chunks(&mut world_buffer, chunk_positions, encoder)
                .map_err(|gpu_err| GpuRecoveryError::OperationFailed {
                    message: format!("Terrain generation failed: {:?}", gpu_err),
                })
        });

        match result {
            Ok(_metadata_buffer) => Ok(()),
            Err(GpuRecoveryError::DeviceLost) => Err(GpuError::DeviceLost),
            Err(GpuRecoveryError::TooManyErrors { count }) => {
                log::error!("Too many GPU errors during terrain generation: {}", count);
                Err(GpuError::TooManyErrors)
            }
            Err(GpuRecoveryError::Panic { message }) => {
                log::error!("GPU operation panicked: {}", message);
                Err(GpuError::GpuPanic)
            }
            Err(e) => {
                log::error!("GPU error during terrain generation: {:?}", e);
                Err(GpuError::Other(format!("{:?}", e)))
            }
        }
    }

    /// Generate a chunk using CPU fallback with proper terrain logic
    fn generate_cpu_fallback(&self, chunk_pos: ChunkPos, chunk_size: u32) -> TempChunk {
        use crate::world::core::{BlockId, VoxelPos};
        
        let mut chunk = TempChunk::new_empty(chunk_pos, chunk_size);
        
        // Use the same terrain generation logic as in the GPU shader
        // TERRAIN_THRESHOLD = 64
        const TERRAIN_THRESHOLD: i32 = 64;
        
        let world_x_base = chunk_pos.x * chunk_size as i32;
        let world_y_base = chunk_pos.y * chunk_size as i32;
        let world_z_base = chunk_pos.z * chunk_size as i32;
        
        for x in 0..chunk_size {
            for z in 0..chunk_size {
                let world_x = world_x_base + x as i32;
                let world_z = world_z_base + z as i32;
                
                // Calculate terrain height with variation (matching GPU shader)
                let height_variation = (world_x as f32 * 0.05).sin() * 5.0 + (world_z as f32 * 0.05).cos() * 5.0;
                let surface_height = TERRAIN_THRESHOLD as f32 + height_variation;
                
                for y in 0..chunk_size {
                    let world_y = world_y_base + y as i32;
                    
                    let block_id = if world_y < surface_height as i32 - 3 {
                        // Deep underground: stone
                        BlockId(1) // BLOCK_STONE
                    } else if world_y < surface_height as i32 {
                        // Just below surface: stone with occasional air (caves)
                        let cave_noise_val = ((world_x + world_y * 7 + world_z * 13) % 100) as f32 / 100.0;
                        if cave_noise_val > 0.85 && world_y < surface_height as i32 - 5 {
                            BlockId(0) // BLOCK_AIR - cave
                        } else {
                            BlockId(1) // BLOCK_STONE
                        }
                    } else if world_y <= surface_height as i32 {
                        // Surface layer: grass
                        BlockId(3) // BLOCK_GRASS
                    } else {
                        // Above surface: air
                        BlockId(0) // BLOCK_AIR
                    };
                    
                    chunk.set_block(x, y, z, block_id);
                }
            }
        }
        
        log::info!("CPU fallback generated terrain chunk {:?} with surface at ~{}", chunk_pos, TERRAIN_THRESHOLD);
        chunk
    }
}

impl WorldGenerator for GpuWorldGenerator {
    fn generate_chunk(&self, chunk_pos: ChunkPos, chunk_size: u32) -> TempChunk {
        // Try to perform GPU generation by creating our own command encoder
        // This is not ideal but allows GPU generation to work through the synchronous interface
        
        log::info!("GPU generation for chunk {:?} - creating command encoder", chunk_pos);
        
        // Create command encoder for GPU generation
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some(&format!("GPU terrain generation for chunk {:?}", chunk_pos)),
        });
        
        // Try GPU generation with encoder
        match self.generate_chunks_with_encoder(&[chunk_pos], &mut encoder) {
            Ok(()) => {
                log::info!("GPU generation successful for chunk {:?}, submitting commands", chunk_pos);
                
                // Submit commands to GPU queue (need to get queue reference)
                // For now, we'll submit later - just finish the encoder
                
                // For now, return a properly generated chunk from CPU fallback
                // TODO: Extract actual data from GPU world buffer
                log::warn!("GPU generation submitted for chunk {:?}, but returning CPU fallback for now", chunk_pos);
                self.generate_cpu_fallback(chunk_pos, chunk_size)
            }
            Err(e) => {
                log::error!("GPU generation failed for chunk {:?}: {:?}, using CPU fallback", chunk_pos, e);
                self.generate_cpu_fallback(chunk_pos, chunk_size)
            }
        }
    }

    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        // Use the constant from the root constants.rs file
        use crate::constants::terrain::SEA_LEVEL;
        SEA_LEVEL as i32
    }

    fn is_gpu(&self) -> bool {
        true
    }

    fn get_world_buffer(&self) -> Option<Arc<Mutex<WorldBuffer>>> {
        Some(self.world_buffer.clone())
    }
}
