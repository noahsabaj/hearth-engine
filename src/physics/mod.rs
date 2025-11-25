//! Physics Module - Simplified for DOP conversion

pub mod aabb;
pub mod collision_data;
pub mod gpu_physics_world;
pub mod gpu_physics_world_data;
pub mod gpu_physics_world_operations;
pub mod integration;
pub mod parallel_solver;
pub mod parallel_solver_data;
pub mod parallel_solver_operations;
pub mod preallocated_spatial_hash;
pub mod spatial_hash;

// Simple re-exports
pub use aabb::AABB;
pub use collision_data::{CollisionData, ContactPoint, ContactPair, CollisionStats};
pub use gpu_physics_world::GpuPhysicsWorld;
pub use gpu_physics_world_data::GpuPhysicsWorldData;
pub use integration::Integration;
pub use parallel_solver::ParallelSolver;
pub use parallel_solver_data::ParallelSolverData;
pub use preallocated_spatial_hash::PreallocatedSpatialHash;
pub use spatial_hash::SpatialHash;

// Re-export DOP operations
pub use gpu_physics_world_operations::{initialize_gpu_physics_world, add_physics_entity, update_physics};

/// Entity ID type
pub type EntityId = u32;
