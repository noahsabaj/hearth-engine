use crate::renderer::{MeshLod, Vertex};
/// LOD Transition System
///
/// Handles smooth transitions between LOD levels using geomorphing
/// and temporal blending to avoid popping artifacts.

use cgmath::{InnerSpace, Vector3, Zero};
use std::collections::HashMap;

/// LOD transition state for smooth morphing
#[derive(Debug, Clone)]
pub struct LodTransition {
    pub current_lod: MeshLod,
    pub target_lod: MeshLod,
    pub blend_factor: f32,
    pub transition_time: f32,
    pub elapsed_time: f32,
}

impl LodTransition {
    pub fn new(current: MeshLod, target: MeshLod, transition_time: f32) -> Self {
        Self {
            current_lod: current,
            target_lod: target,
            blend_factor: 0.0,
            transition_time,
            elapsed_time: 0.0,
        }
    }

    /// Update transition progress
    pub fn update(&mut self, delta_time: f32) -> bool {
        if self.current_lod == self.target_lod {
            return false; // No transition
        }

        self.elapsed_time += delta_time;
        self.blend_factor = (self.elapsed_time / self.transition_time).min(1.0);

        if self.blend_factor >= 1.0 {
            self.current_lod = self.target_lod;
            self.elapsed_time = 0.0;
            self.blend_factor = 0.0;
            false // Transition complete
        } else {
            true // Still transitioning
        }
    }

    /// Get smooth blend factor with easing
    pub fn get_smooth_blend(&self) -> f32 {
        // Smooth step function for better visual transition
        let t = self.blend_factor;
        t * t * (3.0 - 2.0 * t)
    }
}

/// Geomorphing LOD system for smooth transitions
pub struct GeomorphLod {
    /// Morph targets for each LOD transition
    morph_targets: HashMap<(MeshLod, MeshLod), MorphData>,

    /// Transition states per chunk
    transitions: HashMap<u64, LodTransition>, // ChunkId -> Transition

    /// Configuration
    transition_distance: f32,
    transition_time: f32,
}

/// Morph data between two LOD levels
struct MorphData {
    /// Vertex mapping from high to low LOD
    vertex_mapping: Vec<MorphVertex>,

    /// Edge collapse order for progressive mesh
    collapse_order: Vec<EdgeCollapse>,
}

/// Vertex morphing information
#[derive(Debug, Clone)]
struct MorphVertex {
    high_lod_index: u32,
    low_lod_index: u32,
    morph_offset: Vector3<f32>,
}

/// Edge collapse operation
#[derive(Debug, Clone)]
struct EdgeCollapse {
    vertex_to_remove: u32,
    vertex_to_keep: u32,
    affected_faces: Vec<u32>,
    collapse_point: Vector3<f32>,
}

impl GeomorphLod {
    pub fn new(transition_distance: f32, transition_time: f32) -> Self {
        Self {
            morph_targets: HashMap::new(),
            transitions: HashMap::new(),
            transition_distance,
            transition_time,
        }
    }

    /// Pre-compute morph targets between LOD levels
    pub fn compute_morph_targets(
        &mut self,
        lod_high: MeshLod,
        lod_low: MeshLod,
        vertices_high: &[Vertex],
        vertices_low: &[Vertex],
        indices_high: &[u32],
        indices_low: &[u32],
    ) {
        let morph_data =
            self.compute_vertex_mapping(vertices_high, vertices_low, indices_high, indices_low);

        self.morph_targets.insert((lod_high, lod_low), morph_data);
    }

    /// Compute vertex mapping between LODs
    fn compute_vertex_mapping(
        &self,
        vertices_high: &[Vertex],
        vertices_low: &[Vertex],
        indices_high: &[u32],
        indices_low: &[u32],
    ) -> MorphData {
        let mut vertex_mapping = Vec::new();

        // For each vertex in high LOD, find nearest in low LOD
        for (high_idx, high_vert) in vertices_high.iter().enumerate() {
            let high_pos = Vector3::from(high_vert.position);

            let mut best_low_idx = 0;
            let mut best_distance = f32::MAX;

            for (low_idx, low_vert) in vertices_low.iter().enumerate() {
                let low_pos = Vector3::from(low_vert.position);
                let distance = (high_pos - low_pos).magnitude();

                if distance < best_distance {
                    best_distance = distance;
                    best_low_idx = low_idx;
                }
            }

            let low_pos = Vector3::from(vertices_low[best_low_idx].position);
            let morph_offset = low_pos - high_pos;

            vertex_mapping.push(MorphVertex {
                high_lod_index: high_idx as u32,
                low_lod_index: best_low_idx as u32,
                morph_offset,
            });
        }

        // Compute edge collapses for progressive mesh
        let collapse_order = self.compute_edge_collapses(vertices_high, indices_high);

        MorphData {
            vertex_mapping,
            collapse_order,
        }
    }

    /// Compute edge collapse sequence using quadric error metric
    fn compute_edge_collapses(&self, vertices: &[Vertex], indices: &[u32]) -> Vec<EdgeCollapse> {
        // Simplified implementation - in practice would use quadric error metrics
        let mut collapses = Vec::new();

        // Build edge list
        let mut edges: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
        for chunk in indices.chunks(3) {
            if chunk.len() == 3 {
                let v0 = chunk[0];
                let v1 = chunk[1];
                let v2 = chunk[2];

                edges.insert((v0.min(v1), v0.max(v1)), vec![]);
                edges.insert((v1.min(v2), v1.max(v2)), vec![]);
                edges.insert((v2.min(v0), v2.max(v0)), vec![]);
            }
        }

        // Simple collapse: merge vertices that are close
        for ((v0, v1), _) in edges.iter().take(vertices.len() / 10) {
            let pos0 = match vertices.get(*v0 as usize) {
                Some(v) => Vector3::from(v.position),
                None => {
                    log::warn!("Vertex {} out of bounds during edge collapse", v0);
                    Vector3::zero()
                }
            };
            let pos1 = match vertices.get(*v1 as usize) {
                Some(v) => Vector3::from(v.position),
                None => {
                    log::warn!("Vertex {} out of bounds during edge collapse", v1);
                    Vector3::zero()
                }
            };
            let collapse_point = (pos0 + pos1) * 0.5;

            collapses.push(EdgeCollapse {
                vertex_to_remove: *v1,
                vertex_to_keep: *v0,
                affected_faces: vec![], // Would compute affected faces
                collapse_point,
            });
        }

        collapses
    }

    /// Start LOD transition for a chunk
    pub fn start_transition(&mut self, chunk_id: u64, current: MeshLod, target: MeshLod) {
        if current != target {
            let transition = LodTransition::new(current, target, self.transition_time);
            self.transitions.insert(chunk_id, transition);
        }
    }

    /// Update all active transitions
    pub fn update_transitions(&mut self, delta_time: f32) {
        self.transitions
            .retain(|_, transition| transition.update(delta_time));
    }

    /// Apply geomorphing to vertices
    pub fn apply_morph(
        &self,
        chunk_id: u64,
        vertices: &mut [Vertex],
        current_lod: MeshLod,
        target_lod: MeshLod,
    ) {
        if let Some(transition) = self.transitions.get(&chunk_id) {
            if let Some(morph_data) = self.morph_targets.get(&(current_lod, target_lod)) {
                let blend = transition.get_smooth_blend();

                for morph in &morph_data.vertex_mapping {
                    if (morph.high_lod_index as usize) < vertices.len() {
                        let vertex = match vertices.get_mut(morph.high_lod_index as usize) {
                            Some(v) => v,
                            None => {
                                log::warn!(
                                    "Morph high_lod_index {} out of bounds",
                                    morph.high_lod_index
                                );
                                continue;
                            }
                        };
                        let morphed_pos =
                            Vector3::from(vertex.position) + morph.morph_offset * blend;
                        vertex.position = morphed_pos.into();
                    }
                }
            }
        }
    }

    /// Check if chunk needs LOD transition
    pub fn check_lod_transition(&self, distance: f32, current_lod: MeshLod) -> Option<MeshLod> {
        let target_lod = MeshLod::from_distance(distance);

        // Add hysteresis to prevent rapid switching
        let hysteresis_factor = 1.2;
        let current_threshold = self.get_lod_distance(current_lod);
        let target_threshold = self.get_lod_distance(target_lod);

        if current_lod != target_lod {
            if target_lod as u32 > current_lod as u32 {
                // Transitioning to lower detail - use hysteresis
                if distance > current_threshold * hysteresis_factor {
                    return Some(target_lod);
                }
            } else {
                // Transitioning to higher detail
                if distance < target_threshold {
                    return Some(target_lod);
                }
            }
        }

        None
    }

    /// Get distance threshold for LOD level
    fn get_lod_distance(&self, lod: MeshLod) -> f32 {
        match lod {
            MeshLod::Lod0 => 50.0,
            MeshLod::Lod1 => 100.0,
            MeshLod::Lod2 => 200.0,
            MeshLod::Lod3 => 400.0,
            MeshLod::Lod4 => f32::MAX,
        }
    }

    /// Get active transition for chunk
    pub fn get_transition(&self, chunk_id: u64) -> Option<&LodTransition> {
        self.transitions.get(&chunk_id)
    }
}
