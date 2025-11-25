/// Adaptive Tessellation for SDF Terrain
/// 
/// Dynamically adjusts mesh density based on surface curvature and
/// screen space error for optimal quality/performance tradeoff.

use cgmath::{Vector3, Vector2};
use crate::sdf::SdfBuffer;
use crate::renderer::Vertex;

/// Tessellation parameters
#[derive(Debug, Clone)]
pub struct TessellationParams {
    /// Maximum tessellation level (subdivisions)
    pub max_level: u32,
    
    /// Screen space error threshold in pixels
    pub screen_error_threshold: f32,
    
    /// Curvature threshold for subdivision
    pub curvature_threshold: f32,
    
    /// Distance-based LOD factor
    pub distance_factor: f32,
    
    /// Minimum edge length to prevent over-tessellation
    pub min_edge_length: f32,
}

impl Default for TessellationParams {
    fn default() -> Self {
        Self {
            max_level: 4,
            screen_error_threshold: 2.0,
            curvature_threshold: 0.1,
            distance_factor: 0.01,
            min_edge_length: 0.5,
        }
    }
}

/// Adaptive tessellation system for SDF meshes
pub struct AdaptiveTessellator {
    params: TessellationParams,
}

impl AdaptiveTessellator {
    pub fn new(params: TessellationParams) -> Self {
        Self { params }
    }
    
    /// Tessellate SDF surface adaptively
    pub fn tessellate_sdf(
        &self,
        sdf: &SdfBuffer,
        region: (Vector3<f32>, Vector3<f32>),
        view_pos: Vector3<f32>,
        viewport_size: (f32, f32),
        fov: f32,
    ) -> TessellatedMesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        // Start with base grid
        let base_resolution = 16;
        let mut patches = self.create_base_patches(region, base_resolution);
        
        // Adaptively subdivide patches
        let mut final_patches = Vec::new();
        
        while let Some(patch) = patches.pop() {
            if self.should_subdivide(&patch, sdf, view_pos, viewport_size, fov) {
                // Subdivide into 4 sub-patches
                let sub_patches = self.subdivide_patch(&patch);
                patches.extend(sub_patches);
            } else {
                final_patches.push(patch);
            }
        }
        
        // Generate vertices and indices for final patches
        let mut vertex_map = std::collections::HashMap::new();
        
        for patch in &final_patches {
            let patch_indices = self.generate_patch_mesh(
                &patch,
                sdf,
                &mut vertices,
                &mut vertex_map,
            );
            indices.extend(patch_indices);
        }
        
        // Optimize vertex order for GPU cache
        self.optimize_vertex_order(&mut vertices, &mut indices);
        
        TessellatedMesh {
            vertices,
            indices,
            stats: TessellationStats {
                patch_count: final_patches.len(),
                vertex_count: vertices.len(),
                triangle_count: indices.len() / 3,
            },
        }
    }
    
    /// Create initial patches covering the region
    fn create_base_patches(
        &self,
        region: (Vector3<f32>, Vector3<f32>),
        resolution: u32,
    ) -> Vec<TerrainPatch> {
        let mut patches = Vec::new();
        let (min, max) = region;
        let size = max - min;
        let patch_size = size / resolution as f32;
        
        for x in 0..resolution {
            for z in 0..resolution {
                let patch_min = min + Vector3::new(
                    x as f32 * patch_size.x,
                    0.0,
                    z as f32 * patch_size.z,
                );
                
                let patch_max = patch_min + Vector3::new(
                    patch_size.x,
                    size.y,
                    patch_size.z,
                );
                
                patches.push(TerrainPatch {
                    min: patch_min,
                    max: patch_max,
                    level: 0,
                });
            }
        }
        
        patches
    }
    
    /// Check if patch should be subdivided
    fn should_subdivide(
        &self,
        patch: &TerrainPatch,
        sdf: &SdfBuffer,
        view_pos: Vector3<f32>,
        viewport_size: (f32, f32),
        fov: f32,
    ) -> bool {
        // Don't subdivide beyond max level
        if patch.level >= self.params.max_level {
            return false;
        }
        
        // Check edge length
        let edge_length = (patch.max - patch.min).x;
        if edge_length < self.params.min_edge_length {
            return false;
        }
        
        // Distance-based check
        let center = (patch.min + patch.max) * 0.5;
        let distance = (center - view_pos).magnitude();
        let distance_factor = 1.0 / (1.0 + distance * self.params.distance_factor);
        
        // Screen space error check
        let screen_size = self.estimate_screen_size(patch, view_pos, viewport_size, fov);
        if screen_size < self.params.screen_error_threshold * distance_factor {
            return false;
        }
        
        // Curvature check
        let curvature = self.estimate_curvature(patch, sdf);
        if curvature > self.params.curvature_threshold {
            return true;
        }
        
        false
    }
    
    /// Estimate screen space size of patch
    fn estimate_screen_size(
        &self,
        patch: &TerrainPatch,
        view_pos: Vector3<f32>,
        viewport_size: (f32, f32),
        fov: f32,
    ) -> f32 {
        let center = (patch.min + patch.max) * 0.5;
        let size = (patch.max - patch.min).magnitude();
        let distance = (center - view_pos).magnitude();
        
        // Project to screen space
        let angular_size = (size / distance).atan();
        let screen_size = angular_size / fov * viewport_size.1;
        
        screen_size
    }
    
    /// Estimate surface curvature in patch
    fn estimate_curvature(&self, patch: &TerrainPatch, sdf: &SdfBuffer) -> f32 {
        // Sample SDF at multiple points
        let samples = 4;
        let mut gradients = Vec::new();
        
        for i in 0..samples {
            for j in 0..samples {
                let u = i as f32 / (samples - 1) as f32;
                let v = j as f32 / (samples - 1) as f32;
                
                let pos = patch.min + (patch.max - patch.min) * Vector3::new(u, 0.5, v);
                
                // Sample gradient
                let gradient = self.sample_gradient(pos, sdf);
                gradients.push(gradient);
            }
        }
        
        // Compute curvature as gradient variation
        let mut max_variation = 0.0;
        for i in 0..gradients.len() {
            for j in i+1..gradients.len() {
                let variation = (gradients[i] - gradients[j]).magnitude();
                max_variation = max_variation.max(variation);
            }
        }
        
        max_variation
    }
    
    /// Sample SDF gradient at position
    fn sample_gradient(&self, pos: Vector3<f32>, sdf: &SdfBuffer) -> Vector3<f32> {
        let h = 0.1; // Small offset for finite difference
        
        let dx = sdf.sample(pos + Vector3::unit_x() * h) - 
                 sdf.sample(pos - Vector3::unit_x() * h);
        let dy = sdf.sample(pos + Vector3::unit_y() * h) - 
                 sdf.sample(pos - Vector3::unit_y() * h);
        let dz = sdf.sample(pos + Vector3::unit_z() * h) - 
                 sdf.sample(pos - Vector3::unit_z() * h);
        
        Vector3::new(dx, dy, dz) / (2.0 * h)
    }
    
    /// Subdivide patch into 4 sub-patches
    fn subdivide_patch(&self, patch: &TerrainPatch) -> Vec<TerrainPatch> {
        let center = (patch.min + patch.max) * 0.5;
        let level = patch.level + 1;
        
        vec![
            TerrainPatch {
                min: patch.min,
                max: center,
                level,
            },
            TerrainPatch {
                min: Vector3::new(center.x, patch.min.y, patch.min.z),
                max: Vector3::new(patch.max.x, center.y, center.z),
                level,
            },
            TerrainPatch {
                min: Vector3::new(patch.min.x, patch.min.y, center.z),
                max: Vector3::new(center.x, center.y, patch.max.z),
                level,
            },
            TerrainPatch {
                min: Vector3::new(center.x, patch.min.y, center.z),
                max: Vector3::new(patch.max.x, center.y, patch.max.z),
                level,
            },
        ]
    }
    
    /// Generate mesh for a patch
    fn generate_patch_mesh(
        &self,
        patch: &TerrainPatch,
        sdf: &SdfBuffer,
        vertices: &mut Vec<Vertex>,
        vertex_map: &mut std::collections::HashMap<(u32, u32), u32>,
    ) -> Vec<u32> {
        let mut indices = Vec::new();
        
        // Resolution based on tessellation level
        let resolution = 2u32.pow(patch.level) + 1;
        
        // Generate vertices
        for i in 0..resolution {
            for j in 0..resolution {
                let u = i as f32 / (resolution - 1) as f32;
                let v = j as f32 / (resolution - 1) as f32;
                
                let pos = patch.min + (patch.max - patch.min) * Vector3::new(u, 0.0, v);
                
                // Project to SDF surface
                let surface_pos = self.project_to_surface(pos, sdf);
                let normal = self.sample_gradient(surface_pos, sdf).normalize();
                
                let vertex_key = (i, j);
                let vertex_index = vertices.len() as u32;
                
                vertices.push(Vertex {
                    position: surface_pos.into(),
                    normal: normal.into(),
                    tex_coords: [u, v],
                    color: [1.0, 1.0, 1.0, 1.0],
                    ao: 1.0,
                });
                
                vertex_map.insert(vertex_key, vertex_index);
            }
        }
        
        // Generate indices
        for i in 0..resolution-1 {
            for j in 0..resolution-1 {
                let v00 = *vertex_map.get(&(i, j))
                    .expect("Vertex (i, j) should exist in map");
                let v10 = *vertex_map.get(&(i+1, j))
                    .expect("Vertex (i+1, j) should exist in map");
                let v01 = *vertex_map.get(&(i, j+1))
                    .expect("Vertex (i, j+1) should exist in map");
                let v11 = *vertex_map.get(&(i+1, j+1))
                    .expect("Vertex (i+1, j+1) should exist in map");
                
                // Two triangles per quad
                indices.extend_from_slice(&[v00, v10, v11]);
                indices.extend_from_slice(&[v00, v11, v01]);
            }
        }
        
        indices
    }
    
    /// Project point to SDF surface
    fn project_to_surface(&self, mut pos: Vector3<f32>, sdf: &SdfBuffer) -> Vector3<f32> {
        // Simple iterative projection
        for _ in 0..10 {
            let distance = sdf.sample(pos);
            if distance.abs() < 0.01 {
                break;
            }
            
            let gradient = self.sample_gradient(pos, sdf);
            pos -= gradient.normalize() * distance;
        }
        
        pos
    }
    
    /// Optimize vertex order for GPU vertex cache
    fn optimize_vertex_order(&self, vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>) {
        // Simple optimization: sort by spatial locality
        // In practice, would use more sophisticated vertex cache optimization
        
        // Create vertex remapping
        let mut vertex_remap: Vec<u32> = (0..vertices.len() as u32).collect();
        vertex_remap.sort_by_key(|&idx| {
            let pos = match vertices.get(idx as usize) {
                Some(v) => Vector3::from(v.position),
                None => {
                    log::warn!("Vertex index {} out of bounds during optimization", idx);
                    Vector3::zero()
                }
            };
            // Sort by Morton code for spatial locality
            let x = (pos.x * 1000.0) as u32;
            let z = (pos.z * 1000.0) as u32;
            morton_2d(x, z)
        });
        
        // Build inverse mapping
        let mut inverse_remap = vec![0u32; vertices.len()];
        for (new_idx, &old_idx) in vertex_remap.iter().enumerate() {
            if let Some(elem) = inverse_remap.get_mut(old_idx as usize) {
                *elem = new_idx as u32;
            }
        }
        
        // Reorder vertices
        let mut new_vertices = Vec::with_capacity(vertices.len());
        for &old_idx in &vertex_remap {
            if let Some(vertex) = vertices.get(old_idx as usize) {
                new_vertices.push(*vertex);
            }
        }
        *vertices = new_vertices;
        
        // Update indices
        for idx in indices.iter_mut() {
            *idx = match inverse_remap.get(*idx as usize) {
                Some(&new_idx) => new_idx,
                None => {
                    log::warn!("Index {} not found in remap, keeping original", *idx);
                    *idx
                }
            };
        }
    }
}

/// Terrain patch for adaptive subdivision
#[derive(Debug, Clone)]
struct TerrainPatch {
    min: Vector3<f32>,
    max: Vector3<f32>,
    level: u32,
}

/// Tessellated mesh result
pub struct TessellatedMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub stats: TessellationStats,
}

/// Tessellation statistics
#[derive(Debug)]
pub struct TessellationStats {
    pub patch_count: usize,
    pub vertex_count: usize,
    pub triangle_count: usize,
}

/// Simple 2D Morton encoding for spatial sorting
fn morton_2d(x: u32, y: u32) -> u64 {
    let mut result = 0u64;
    for i in 0..16 {
        result |= ((x & (1 << i)) as u64) << (2 * i);
        result |= ((y & (1 << i)) as u64) << (2 * i + 1);
    }
    result
}