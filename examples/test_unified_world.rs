//! Test the unified world module
//!
//! This example demonstrates basic usage of the unified world management system.

use hearth_engine::{
    constants::core::CHUNK_SIZE,
    world::{
        core::{ChunkPos, VoxelPos},
        management::{Backend, UnifiedWorldManager, WorldManagerConfig},
    },
};
use std::sync::Arc;

fn main() {
    env_logger::init();

    println!("Testing Unified World Module...");

    // Run async test
    pollster::block_on(test_unified_world());
}

async fn test_unified_world() {
    // Create GPU device
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find adapter");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Test Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let device = Arc::new(device);
    let queue = Arc::new(queue);

    println!("[OK] GPU device created");

    // Create world manager with unified architecture
    let config = WorldManagerConfig {
        backend: Backend::Auto,
        chunk_size: CHUNK_SIZE,
        render_distance: 8,
        seed: 42,
    };

    let mut world_manager = UnifiedWorldManager::new_gpu(device.clone(), queue.clone(), config)
        .await
        .expect("Failed to create world manager");

    println!("[OK] Unified world manager created");

    // Test GPU mode detection
    let is_gpu = world_manager.is_gpu();
    println!("  Running in GPU mode: {}", is_gpu);

    // Test block operations
    let test_pos = VoxelPos {
        x: 10,
        y: 64,
        z: 10,
    };
    let block = world_manager.get_block(test_pos);
    println!("  Block at {:?}: {:?}", test_pos, block);

    // Test chunk position conversion
    let chunk_pos = test_pos.to_chunk_pos(CHUNK_SIZE);
    println!("  Chunk position for {:?}: {:?}", test_pos, chunk_pos);

    // Test chunk loading
    let load_result = world_manager.load_chunk(ChunkPos { x: 0, y: 0, z: 0 });
    match load_result {
        Ok(()) => println!("  Chunk loaded successfully"),
        Err(e) => println!("  Chunk load result: {:?}", e),
    }

    println!("\n[OK] All unified world tests completed!");
}
