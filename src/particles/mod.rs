//! Particles Module - Simplified for DOP conversion

pub mod dop_system_operations;
pub mod emitter_data;
pub mod emitter_operations;
pub mod effects_data;
pub mod effects_operations;
pub mod gpu_particle_system;
pub mod particle_data;
pub mod particle_operations;
pub mod particle_system_data;
pub mod particle_system_operations;
pub mod particle_types;
pub mod physics_data;
pub mod physics_operations;
pub mod system_data;

// Simple re-exports
pub use emitter_data::EmitterData;
pub use effects_data::EffectsData;
pub use particle_data::{ParticleData, ParticleGPUData};
pub use particle_system_data::ParticleSystemData;
pub use particle_types::{ParticleType, particle_type_to_id};
pub use physics_data::PhysicsData;
pub use system_data::SystemData;
pub use gpu_particle_system::GpuParticleSystem;
