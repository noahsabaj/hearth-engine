// Octree Update Shader
// Updates sparse voxel octree based on world changes

// VoxelData is auto-generated from gpu/types/world.rs

struct OctreeNode {
    children: array<u32, 8>,
    metadata: u32,
    bbox_min: vec3<f32>,
    bbox_max: vec3<f32>,
}

@group(0) @binding(0) var<storage, read_write> octree_nodes: array<OctreeNode>;
@group(0) @binding(1) var<storage, read> world_voxels: array<VoxelData>;

// CHUNK_SIZE is auto-generated from constants.rs

// Check if a region contains any solid voxels
fn region_is_empty(base_pos: vec3<u32>, size: u32) -> bool {
    // Sample at regular intervals
    let step = max(1u, size / 8u);
    
    for (var y = base_pos.y; y < base_pos.y + size; y += step) {
        for (var z = base_pos.z; z < base_pos.z + size; z += step) {
            for (var x = base_pos.x; x < base_pos.x + size; x += step) {
                let chunk_pos = vec3<u32>(x, y, z) / CHUNK_SIZE;
                let local_pos = vec3<u32>(x, y, z) % CHUNK_SIZE;
                
                // Morton encode for cache efficiency
                let chunk_morton = morton_encode_3d(chunk_pos.x, chunk_pos.y, chunk_pos.z);
                let voxel_morton = morton_encode_3d(local_pos.x, local_pos.y, local_pos.z);
                let global_index = chunk_morton * (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) + voxel_morton;
                
                if global_index < arrayLength(&world_voxels) {
                    let voxel = world_voxels[global_index];
                    if (voxel.data & 0xFFFFu) != 0u {
                        return false; // Found a solid voxel
                    }
                }
            }
        }
    }
    
    return true;
}

// Morton encoding functions from unified GPU system
#include "morton.wgsl"

@compute @workgroup_size(64, 1, 1)
fn update_octree(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let node_index = global_id.x;
    
    if node_index >= arrayLength(&octree_nodes) {
        return;
    }
    
    var node = octree_nodes[node_index];
    
    // Skip if this is a leaf node
    let level = node.metadata & 0xFFu;
    if level == 0u {
        return;
    }
    
    // Calculate node size from bounding box
    let node_size = u32(node.bbox_max.x - node.bbox_min.x);
    let child_size = node_size / 2u;
    
    // Update occupancy mask based on children
    var new_occupancy: u32 = 0u;
    
    for (var i = 0u; i < 8u; i++) {
        let child_index = node.children[i];
        
        if child_index != 0u {
            // Check if child region is empty
            let child_offset = vec3<u32>(
                u32(i & 1u) * child_size,
                u32((i >> 1u) & 1u) * child_size,
                u32((i >> 2u) & 1u) * child_size
            );
            
            let child_base = vec3<u32>(node.bbox_min) + child_offset;
            
            if !region_is_empty(child_base, child_size) {
                new_occupancy |= (1u << i);
            } else {
                // Mark child as empty
                node.children[i] = 0u;
            }
        }
    }
    
    // Update occupancy in metadata
    node.metadata = (node.metadata & 0xFFFF00FFu) | (new_occupancy << 8u);
    
    // Write back updated node
    octree_nodes[node_index] = node;
}

// Helper function to traverse octree and find intersections
fn octree_ray_intersect(
    ray_origin: vec3<f32>,
    ray_dir: vec3<f32>,
    node_index: u32,
) -> bool {
    if node_index == 0u {
        return false;
    }
    
    let node = octree_nodes[node_index];
    
    // Ray-AABB intersection test
    let inv_dir = 1.0 / ray_dir;
    let t1 = (node.bbox_min - ray_origin) * inv_dir;
    let t2 = (node.bbox_max - ray_origin) * inv_dir;
    
    let tmin = max(max(min(t1.x, t2.x), min(t1.y, t2.y)), min(t1.z, t2.z));
    let tmax = min(min(max(t1.x, t2.x), max(t1.y, t2.y)), max(t1.z, t2.z));
    
    if tmax < 0.0 || tmin > tmax {
        return false;
    }
    
    // If leaf node, we have a hit
    if (node.metadata & 0xFFu) == 0u {
        return (node.metadata & 0xFF00u) != 0u; // Check occupancy
    }
    
    // Otherwise check children
    for (var i = 0u; i < 8u; i++) {
        if node.children[i] != 0u && octree_ray_intersect(ray_origin, ray_dir, node.children[i]) {
            return true;
        }
    }
    
    return false;
}