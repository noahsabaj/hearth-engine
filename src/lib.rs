#![allow(unused_variables, dead_code, unused_imports)]

// Hearth Engine - Data-Oriented Programming (DOP) Architecture
// 
// This engine is transitioning from OOP to DOP for better performance and cache efficiency.
// - NEW: Use EngineBuffers and *_operations modules for data transformations
// - DEPRECATED: WorldInterface trait and method-based APIs are being phased out
// 
// For new code, prefer:
// - engine_buffers::EngineBuffers for centralized data storage
// - world_operations for world manipulation
// - renderer_operations for rendering operations
// - Pure functions over methods

// Constants module
pub mod constants;

// Core engine modules
// pub mod dop_integration_example; // TODO: Create example when needed
pub mod engine_buffers;
pub mod error;
pub mod panic_handler;

// Export DOP types and functions
pub use engine_buffers::{
    EngineBuffers, SharedEngineBuffers, create_engine_buffers, create_shared_buffers,
    WorldBuffers, RenderBuffers, PhysicsBuffers, NetworkBuffers, InputBuffers,
    ParticleBuffers, MetricsBuffers,
};

// Essential systems
pub mod camera;
pub mod game;
pub mod input;
// pub mod lighting; // MIGRATED: Lighting moved to world::lighting for GPU-first architecture
pub mod memory;
pub mod morton;
pub mod network;
pub mod particles;
pub mod persistence;
pub mod physics;
pub mod renderer;
// World module - GPU-first unified architecture
pub mod world;

// GPU and data systems
pub mod gpu;

// Utilities
pub mod event_system;
pub mod event_system_data;
pub mod event_system_operations;
pub mod event_streams;
pub mod instance;
pub mod process;
pub mod system_monitor;
pub mod system_monitor_data;
pub mod system_monitor_operations;
pub mod thread_pool;
pub mod utils;
pub mod world_state;

use anyhow::Result;
use std::sync::Arc;
use winit::event_loop::{EventLoop, EventLoopBuilder};

pub use camera::{CameraData, CameraUniform};
pub use error::{EngineError, EngineResult, ErrorContext, OptionExt};
pub use game::{GameContextDOP, GameData};
pub use input::KeyCode;
pub use physics::AABB;
pub use renderer::Renderer;
// === Core World Types ===
// Export from world - GPU-first architecture with CPU fallback
pub use world::core::{
    BlockFace, BlockId, BlockRegistry, ChunkPos, PhysicsProperties, Ray,
    RaycastHit, RenderData, VoxelPos,
};
// cast_ray removed - use world::raycast (DOP) instead
pub use world::generation::WorldGenerator;
pub use world::interfaces::ChunkData;
// WorldInterface is deprecated - use world_operations module functions instead
pub use world::management::UnifiedWorldManager as World;
// ChunkSoA removed - GPU-first architecture doesn't need CPU chunk storage

// Re-export ParallelWorld implementation
pub use world::management::{ParallelWorld, ParallelWorldConfig, SpawnFinder};
pub use world::voxel_to_chunk_pos;

// Re-export world module for GPU-first architecture
pub use world::{
    ChunkManagerInterface,
    DayNightCycleData,
    GeneratorInterface,
    LightLevel,
    LightType,
    LightUpdate,
    LightingStats,
    // Re-export GPU lighting system
    TimeOfDayData,
    UnifiedWorldManager,
    WorldManagerConfig as UnifiedWorldConfig,
};

// Re-export wgpu for games that need GPU access (e.g., custom world generators)
pub use wgpu;

/// World generator type for EngineConfig
#[derive(Debug, Clone, PartialEq)]
pub enum WorldGeneratorType {
    Default,
    DangerMoney,
    Custom(String),
}

/// Factory function type for creating world generators when GPU resources are available
/// Accepts the full EngineConfig to ensure proper configuration propagation
pub type WorldGeneratorFactory = Box<
    dyn Fn(
            Arc<wgpu::Device>,
            Arc<wgpu::Queue>,
            &EngineConfig,
        ) -> Box<dyn WorldGenerator + Send + Sync>
        + Send
        + Sync,
>;

/// Main engine configuration
pub struct EngineConfig {
    pub window_title: String,
    pub window_width: u32,
    pub window_height: u32,
    pub chunk_size: u32,
    pub render_distance: u32,
    pub world_generator: Option<Box<dyn WorldGenerator + Send + Sync>>,
    pub world_generator_type: WorldGeneratorType,
    pub world_generator_factory: Option<WorldGeneratorFactory>,
}

impl std::fmt::Debug for EngineConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineConfig")
            .field("window_title", &self.window_title)
            .field("window_width", &self.window_width)
            .field("window_height", &self.window_height)
            .field("chunk_size", &self.chunk_size)
            .field("render_distance", &self.render_distance)
            .field(
                "world_generator",
                &self
                    .world_generator
                    .as_ref()
                    .map(|_| "<Custom WorldGenerator>"),
            )
            .field("world_generator_type", &self.world_generator_type)
            .field(
                "world_generator_factory",
                &self
                    .world_generator_factory
                    .as_ref()
                    .map(|_| "<WorldGenerator Factory>"),
            )
            .finish()
    }
}

impl EngineConfig {
    /// Validate configuration parameters
    pub fn validate(&self) -> Result<()> {
        // Validate chunk size
        if self.chunk_size == 0 {
            return Err(anyhow::anyhow!("EngineConfig: chunk_size cannot be 0"));
        }

        if self.chunk_size > 256 {
            return Err(anyhow::anyhow!(
                "EngineConfig: chunk_size {} exceeds maximum of 256",
                self.chunk_size
            ));
        }

        // Validate render distance
        if self.render_distance == 0 {
            return Err(anyhow::anyhow!("EngineConfig: render_distance cannot be 0"));
        }

        // Calculate memory requirements for world buffer
        let voxel_data_size = 4u64; // 4 bytes per voxel
        let voxels_per_chunk = (self.chunk_size as u64).pow(3);
        let chunk_memory_bytes = voxels_per_chunk * voxel_data_size;

        // Maximum view distance based on chunk size and GPU limits
        let max_safe_chunks = crate::constants::gpu_limits::MAX_BUFFER_BINDING_SIZE / chunk_memory_bytes;
        let max_safe_diameter = (max_safe_chunks as f64).powf(1.0 / 3.0).floor() as u32;
        let max_safe_view_distance = (max_safe_diameter.saturating_sub(1)) / 2;

        log::info!(
            "[EngineConfig] Validation: chunk_size={}, voxels_per_chunk={}, chunk_memory={}KB, max_safe_view_distance={}",
            self.chunk_size, voxels_per_chunk, chunk_memory_bytes / 1024, max_safe_view_distance
        );

        // Validate render distance against GPU memory limits
        if self.render_distance > max_safe_view_distance {
            return Err(anyhow::anyhow!(
                "EngineConfig: render_distance {} exceeds GPU memory limit. Maximum safe render_distance for chunk_size {} is {}. {}",
                self.render_distance,
                self.chunk_size,
                max_safe_view_distance,
                self.suggest_safe_config()
            ));
        }

        // Validate window dimensions
        if self.window_width < 320 || self.window_height < 240 {
            return Err(anyhow::anyhow!(
                "EngineConfig: Window dimensions too small (min 320x240)"
            ));
        }

        if self.window_width > 16384 || self.window_height > 16384 {
            return Err(anyhow::anyhow!(
                "EngineConfig: Window dimensions too large (max 16384x16384)"
            ));
        }

        log::info!("[EngineConfig] Configuration validated successfully");
        Ok(())
    }

    /// Calculate safe view distance for a given chunk size
    pub fn calculate_safe_view_distance(chunk_size: u32) -> u32 {
        let voxel_data_size = 4u64; // 4 bytes per voxel
        let voxels_per_chunk = (chunk_size as u64).pow(3);
        let chunk_memory_bytes = voxels_per_chunk * voxel_data_size;

        let max_safe_chunks = crate::constants::gpu_limits::MAX_BUFFER_BINDING_SIZE / chunk_memory_bytes;
        let max_safe_diameter = (max_safe_chunks as f64).powf(1.0 / 3.0).floor() as u32;
        (max_safe_diameter.saturating_sub(1)) / 2
    }

    /// Suggest safe configuration parameters
    pub fn suggest_safe_config(&self) -> String {
        let mut suggestions = Vec::new();

        if self.chunk_size > 0 {
            let safe_view_distance = Self::calculate_safe_view_distance(self.chunk_size);
            suggestions.push(format!(
                "For chunk_size={}, maximum safe view_distance is {}",
                self.chunk_size, safe_view_distance
            ));
        }

        // Common safe configurations
        suggestions.push("Common safe configurations:".to_string());
        suggestions.push(format!("  - chunk_size={}, view_distance=3 ({}MB)", 
            crate::constants::core::CHUNK_SIZE, 
            (crate::constants::core::VOXELS_PER_CHUNK as u64 * 4 * 343) / 1024 / 1024));
        suggestions.push(format!("  - chunk_size={}, view_distance=2 ({}MB)", 
            crate::constants::core::CHUNK_SIZE,
            (crate::constants::core::VOXELS_PER_CHUNK as u64 * 4 * 125) / 1024 / 1024));
        suggestions.push("  - chunk_size=64, view_distance=1 (64MB)".to_string());

        suggestions.join("\n")
    }
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            window_title: "Hearth Engine".to_string(),
            window_width: 1280,
            window_height: 720,
            chunk_size: crate::constants::core::CHUNK_SIZE, // Optimized for 1dcm³ (10cm) voxels: 5m x 5m x 5m chunks
            render_distance: 8,
            world_generator: None, // Use engine's default generator when None
            world_generator_type: WorldGeneratorType::Default,
            world_generator_factory: None, // Use engine's default generator when None
        }
    }
}

/// Main engine struct that runs the game loop
pub struct Engine {
    config: EngineConfig,
    event_loop: Option<EventLoop<()>>,
    /// Centralized engine data buffers (DOP architecture)
    buffers: SharedEngineBuffers,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Self {
        log::debug!("[Engine::new] Starting engine initialization");

        // Validate configuration before proceeding
        if let Err(e) = config.validate() {
            log::error!("[Engine::new] Configuration validation failed: {}", e);
            log::error!(
                "[Engine::new] Suggestions:\n{}",
                config.suggest_safe_config()
            );
            panic!(
                "Invalid engine configuration: {}. See log for suggestions.",
                e
            );
        }

        // Force X11 backend for WSL compatibility
        #[cfg(target_os = "linux")]
        let event_loop = {
            log::debug!("[Engine::new] Creating X11 event loop for Linux...");
            use winit::platform::x11::EventLoopBuilderExtX11;
            let result = EventLoopBuilder::new().with_x11().build();
            match result {
                Ok(loop_) => {
                    log::info!("[Engine::new] X11 event loop created successfully");
                    loop_
                }
                Err(e) => {
                    log::error!("[Engine::new] Failed to create X11 event loop: {}", e);
                    panic!("Failed to create event loop: {}", e);
                }
            }
        };

        #[cfg(not(target_os = "linux"))]
        let event_loop = {
            log::debug!("[Engine::new] Creating default event loop...");
            match EventLoop::new() {
                Ok(loop_) => {
                    log::info!("[Engine::new] Event loop created successfully");
                    loop_
                }
                Err(e) => {
                    log::error!("[Engine::new] Failed to create event loop: {}", e);
                    panic!("Failed to create event loop: {}", e);
                }
            }
        };

        // Initialize GPU thread pool with DOP architecture
        let thread_pool_config = thread_pool::GpuThreadPoolConfig::default();
        let _gpu_thread_pool = match thread_pool::create_gpu_thread_pool_data(thread_pool_config) {
            Ok(pool) => {
                log::info!("[Engine::new] GPU thread pool initialized successfully");
                pool
            }
            Err(e) => {
                log::error!("[Engine::new] Failed to create GPU thread pool: {}", e);
                panic!("Failed to create GPU thread pool: {}", e);
            }
        };

        // Initialize engine buffers (DOP architecture)
        let buffers = create_shared_buffers();
        log::info!("[Engine::new] Engine buffers initialized (DOP architecture)");

        log::info!("[Engine::new] Engine initialization complete");

        Self {
            config,
            event_loop: Some(event_loop),
            buffers,
        }
    }

    pub fn run<G: GameData + 'static>(mut self, game: G) -> Result<()> {
        log::info!("[Engine::run] Starting engine run method");

        let event_loop = match self.event_loop.take() {
            Some(loop_) => {
                log::debug!("[Engine::run] Event loop retrieved successfully");
                loop_
            }
            None => {
                log::error!("[Engine::run] Event loop already taken!");
                panic!("Event loop already taken");
            }
        };

        let config = self.config;
        let buffers = self.buffers;
        log::info!(
            "[Engine::run] Calling renderer::run with config: {:?}",
            config
        );

        // Pass buffers to renderer for DOP architecture
        let result = renderer::run_with_buffers(event_loop, config, game, buffers);

        match &result {
            Ok(_) => log::info!("[Engine::run] Renderer returned successfully"),
            Err(e) => log::error!("[Engine::run] Renderer error: {}", e),
        }

        result
    }
}
