//! Core world data types and fundamental structures
//!
//! This module contains the essential data types that form the foundation
//! of the world system, independent of whether CPU or GPU backend is used.

mod block;
mod position;
mod ray;
mod registry;

pub use block::{BlockId, PhysicsProperties, RenderData};
pub use position::{ChunkPos, VoxelPos};
pub use ray::{BlockFace, Ray, RaycastHit};
pub use registry::{BlockRegistry, BlockRegistration};
