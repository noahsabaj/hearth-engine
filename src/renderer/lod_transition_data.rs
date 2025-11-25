//! LOD Transition Data - Pure DOP
//!
//! NO METHODS. Just data.
//! All transformations happen in lod_transition_operations.rs

use crate::renderer::MeshLod;
use cgmath::Vector3;
use std::collections::HashMap;

/// LOD transition state for smooth morphing
#[derive(Debug, Clone)]
pub struct LodTransitionData {
    pub current_lod: MeshLod,
    pub target_lod: MeshLod,
    pub blend_factor: f32,
    pub transition_time: f32,
    pub elapsed_time: f32,
}

/// Vertex morphing information
#[derive(Debug, Clone)]
pub struct MorphVertex {
    pub high_lod_index: u32,
    pub low_lod_index: u32,
    pub morph_offset: Vector3<f32>,
}

/// Edge collapse operation
#[derive(Debug, Clone)]
pub struct EdgeCollapse {
    pub vertex_to_remove: u32,
    pub vertex_to_keep: u32,
    pub affected_faces: Vec<u32>,
    pub collapse_point: Vector3<f32>,
}

/// Morph data between two LOD levels
pub struct MorphData {
    /// Vertex mapping from high to low LOD
    pub vertex_mapping: Vec<MorphVertex>,

    /// Edge collapse order for progressive mesh
    pub collapse_order: Vec<EdgeCollapse>,
}

/// Geomorphing LOD system data
pub struct GeomorphLodData {
    /// Morph targets for each LOD transition
    pub morph_targets: HashMap<(MeshLod, MeshLod), MorphData>,

    /// Transition states per chunk
    pub transitions: HashMap<u64, LodTransitionData>, // ChunkId -> Transition

    /// Configuration
    pub transition_distance: f32,
    pub transition_time: f32,
}
