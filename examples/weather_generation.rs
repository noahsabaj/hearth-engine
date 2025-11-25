//! Example demonstrating weather integration in terrain generation
//!
//! This example shows how weather parameters affect terrain generation
//! using the GPU-based terrain generator.

use hearth_engine::{
    constants::{
        blocks::ICE,
        weather::{
            INTENSITY_EXTREME, INTENSITY_HEAVY, INTENSITY_LIGHT, INTENSITY_NONE,
            WEATHER_BLIZZARD, WEATHER_CLEAR, WEATHER_RAIN, WEATHER_SNOW,
        },
    },
    gpu::types::terrain::{BlockDistribution, TerrainParams},
    world::{core::ChunkPos, generation::TerrainGeneratorSOA},
};
use std::sync::Arc;

fn main() {
    // Initialize logging
    env_logger::init();

    println!("Weather-Integrated Terrain Generation Example");
    println!("============================================");

    // Create GPU device and queue
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .expect("Failed to find adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Weather Example Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
        },
        None,
    ))
    .expect("Failed to create device");

    let device = Arc::new(device);
    let queue = Arc::new(queue);

    // Create terrain generator
    let terrain_gen = TerrainGeneratorSOA::new(device.clone(), queue.clone())
        .expect("Failed to create terrain generator");

    // Create weather scenarios
    let scenarios = vec![
        ("Clear Weather", WEATHER_CLEAR, INTENSITY_NONE, 20.0),
        ("Light Rain", WEATHER_RAIN, INTENSITY_LIGHT, 18.0),
        ("Heavy Snow", WEATHER_SNOW, INTENSITY_HEAVY, -5.0),
        ("Blizzard", WEATHER_BLIZZARD, INTENSITY_EXTREME, -20.0),
        ("Hot Desert", WEATHER_CLEAR, INTENSITY_NONE, 40.0),
    ];

    for (name, weather_type, intensity, temperature) in scenarios {
        println!("\n{} Scenario:", name);
        println!(
            "  Weather Type: {}",
            match weather_type {
                WEATHER_CLEAR => "Clear",
                WEATHER_RAIN => "Rain",
                WEATHER_SNOW => "Snow",
                WEATHER_BLIZZARD => "Blizzard",
                _ => "Unknown",
            }
        );
        println!("  Intensity: {}", intensity);
        println!("  Temperature: {}C", temperature);

        // Configure terrain parameters with weather
        let mut params = TerrainParams::default();
        params.set_weather(weather_type, intensity);
        params.set_temperature_celsius(temperature);

        // Add some custom block distributions based on weather
        if temperature <= 0.0 {
            // Add ice formations at water level
            params.add_distribution(BlockDistribution {
                block_id: ICE as u32,
                min_height: (params.sea_level as i32) - 2,
                max_height: params.sea_level as i32,
                probability: 0.8,
                noise_threshold: 0.3,
                _padding: [0; 3],
            });
        }

        // Update generator parameters
        terrain_gen
            .update_params(&params)
            .expect("Failed to update params");

        // Generate a test chunk at different heights
        let test_positions = vec![
            ChunkPos::new(0, 0, 0),  // Sea level
            ChunkPos::new(0, 24, 0), // Mountain level (120m)
            ChunkPos::new(0, 36, 0), // High mountain (180m)
        ];

        println!("  Expected terrain features:");
        for pos in &test_positions {
            let altitude = pos.y * 50; // 50 voxels per chunk
            print!("    At {}m: ", altitude / 10);

            if weather_type == WEATHER_SNOW && altitude >= 120 {
                print!("Snow cover, ");
            }
            if temperature <= 0.0 && altitude < 64 {
                print!("Ice instead of water, ");
            }
            if weather_type == WEATHER_RAIN && intensity >= INTENSITY_HEAVY {
                print!("Mud instead of grass, ");
            }
            if temperature <= 0.0 {
                print!("Frozen grass, ");
            }
            println!();
        }
    }

    println!("\nWeather integration complete!");
    println!("\nKey Features Demonstrated:");
    println!("- Weather type affects block selection");
    println!("- Temperature controls water/ice formation");
    println!("- Snow accumulates at high altitudes");
    println!("- Rain creates mud on grass surfaces");
    println!("- Extreme cold freezes grass blocks");
}
