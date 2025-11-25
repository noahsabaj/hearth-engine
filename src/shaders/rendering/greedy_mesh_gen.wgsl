/// GPU Greedy Mesh Generation Compute Shader
/// 
/// Generates optimized meshes directly on GPU using greedy algorithm.
/// Outputs vertex and index buffers ready for rendering.

struct ChunkData {
    voxels: array<u32, 32768>, // 32^3 packed voxels
}

struct MeshOutput {
    vertex_count: atomic<u32>,
    index_count: atomic<u32>,
    quad_count: atomic<u32>,
}

struct Vertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    tex_coord: vec2<f32>,
    material_id: u32,
    ao_factor: f32,
}

// Uniforms
@group(0) @binding(0) var<storage, read> chunk_data: ChunkData;
@group(0) @binding(1) var<storage, read_write> vertices: array<Vertex>;
@group(0) @binding(2) var<storage, read_write> indices: array<u32>;
@group(0) @binding(3) var<storage, read_write> mesh_output: MeshOutput;

// Shared memory for face masks
var<workgroup> face_mask: array<u32, 1024>; // 32x32 face mask
var<workgroup> material_mask: array<u32, 1024>; // Material IDs
var<workgroup> quad_count: atomic<u32>;

// CHUNK_SIZE is auto-generated from constants.rs
const WORKGROUP_SIZE: u32 = 8u;

/// Get voxel at position
fn get_voxel(x: u32, y: u32, z: u32) -> u32 {
    if (x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE) {
        return 0u; // Air
    }
    let index = x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE;
    return chunk_data.voxels[index];
}

/// Check if face is visible
fn is_face_visible(x: i32, y: i32, z: i32, face: u32) -> bool {
    let voxel = get_voxel(u32(x), u32(y), u32(z));
    if (voxel == 0u) {
        return false; // Air block has no visible faces
    }
    
    // Check neighbor based on face direction
    var nx = x;
    var ny = y; 
    var nz = z;
    
    switch (face) {
        case 0u: { nx = x + 1; } // +X
        case 1u: { nx = x - 1; } // -X
        case 2u: { ny = y + 1; } // +Y
        case 3u: { ny = y - 1; } // -Y
        case 4u: { nz = z + 1; } // +Z
        case 5u: { nz = z - 1; } // -Z
        default: {}
    }
    
    // Face is visible if neighbor is air or out of bounds
    if (nx < 0 || nx >= i32(CHUNK_SIZE) ||
        ny < 0 || ny >= i32(CHUNK_SIZE) ||
        nz < 0 || nz >= i32(CHUNK_SIZE)) {
        return true; // At chunk boundary
    }
    
    return get_voxel(u32(nx), u32(ny), u32(nz)) == 0u;
}

/// Extract greedy quads for a face direction
fn extract_quads_for_face(face: u32, slice: u32) {
    // Clear masks
    for (var i = 0u; i < 1024u; i++) {
        face_mask[i] = 0u;
        material_mask[i] = 0u;
    }
    workgroupBarrier();
    
    // Fill mask based on face direction
    for (var u = 0u; u < CHUNK_SIZE; u++) {
        for (var v = 0u; v < CHUNK_SIZE; v++) {
            var x: i32;
            var y: i32;
            var z: i32;
            
            // Map u,v to x,y,z based on face
            switch (face) {
                case 0u, 1u: { // X faces
                    x = i32(slice);
                    y = i32(u);
                    z = i32(v);
                }
                case 2u, 3u: { // Y faces
                    x = i32(u);
                    y = i32(slice);
                    z = i32(v);
                }
                case 4u, 5u: { // Z faces
                    x = i32(u);
                    y = i32(v);
                    z = i32(slice);
                }
                default: {}
            }
            
            if (is_face_visible(x, y, z, face)) {
                let mask_idx = u + v * CHUNK_SIZE;
                face_mask[mask_idx] = 1u;
                material_mask[mask_idx] = get_voxel(u32(x), u32(y), u32(z));
            }
        }
    }
    workgroupBarrier();
    
    // Extract rectangles using greedy algorithm
    var used_mask = array<u32, 1024>();
    
    for (var start_u = 0u; start_u < CHUNK_SIZE; start_u++) {
        for (var start_v = 0u; start_v < CHUNK_SIZE; start_v++) {
            let start_idx = start_u + start_v * CHUNK_SIZE;
            
            if (used_mask[start_idx] == 1u || face_mask[start_idx] == 0u) {
                continue;
            }
            
            let material = material_mask[start_idx];
            
            // Find width
            var width = 1u;
            while (start_u + width < CHUNK_SIZE) {
                let idx = (start_u + width) + start_v * CHUNK_SIZE;
                if (used_mask[idx] == 1u || 
                    face_mask[idx] == 0u || 
                    material_mask[idx] != material) {
                    break;
                }
                width++;
            }
            
            // Find height
            var height = 1u;
            var can_extend = true;
            while (start_v + height < CHUNK_SIZE && can_extend) {
                for (var u = start_u; u < start_u + width; u++) {
                    let idx = u + (start_v + height) * CHUNK_SIZE;
                    if (used_mask[idx] == 1u || 
                        face_mask[idx] == 0u || 
                        material_mask[idx] != material) {
                        can_extend = false;
                        break;
                    }
                }
                if (can_extend) {
                    height++;
                }
            }
            
            // Mark area as used
            for (var u = start_u; u < start_u + width; u++) {
                for (var v = start_v; v < start_v + height; v++) {
                    used_mask[u + v * CHUNK_SIZE] = 1u;
                }
            }
            
            // Emit quad
            let quad_idx = atomicAdd(&quad_count, 1u);
            emit_quad(face, slice, start_u, start_v, width, height, material);
        }
    }
}

/// Emit a quad as two triangles
fn emit_quad(
    face: u32,
    slice: u32,
    start_u: u32,
    start_v: u32,
    width: u32,
    height: u32,
    material: u32
) {
    // Get vertex offset
    let vertex_offset = atomicAdd(&mesh_output.vertex_count, 4u);
    let index_offset = atomicAdd(&mesh_output.index_count, 6u);
    
    // Generate vertices based on face
    var positions: array<vec3<f32>, 4>;
    var normal: vec3<f32>;
    
    switch (face) {
        case 0u: { // +X
            normal = vec3<f32>(1.0, 0.0, 0.0);
            positions[0] = vec3<f32>(f32(slice) + 1.0, f32(start_u), f32(start_v));
            positions[1] = vec3<f32>(f32(slice) + 1.0, f32(start_u + width), f32(start_v));
            positions[2] = vec3<f32>(f32(slice) + 1.0, f32(start_u + width), f32(start_v + height));
            positions[3] = vec3<f32>(f32(slice) + 1.0, f32(start_u), f32(start_v + height));
        }
        case 1u: { // -X
            normal = vec3<f32>(-1.0, 0.0, 0.0);
            positions[0] = vec3<f32>(f32(slice), f32(start_u), f32(start_v));
            positions[1] = vec3<f32>(f32(slice), f32(start_u), f32(start_v + height));
            positions[2] = vec3<f32>(f32(slice), f32(start_u + width), f32(start_v + height));
            positions[3] = vec3<f32>(f32(slice), f32(start_u + width), f32(start_v));
        }
        case 2u: { // +Y
            normal = vec3<f32>(0.0, 1.0, 0.0);
            positions[0] = vec3<f32>(f32(start_u), f32(slice) + 1.0, f32(start_v));
            positions[1] = vec3<f32>(f32(start_u + width), f32(slice) + 1.0, f32(start_v));
            positions[2] = vec3<f32>(f32(start_u + width), f32(slice) + 1.0, f32(start_v + height));
            positions[3] = vec3<f32>(f32(start_u), f32(slice) + 1.0, f32(start_v + height));
        }
        // ... other faces similar
        default: {}
    }
    
    // Write vertices
    for (var i = 0u; i < 4u; i++) {
        vertices[vertex_offset + i] = Vertex(
            positions[i],
            normal,
            vec2<f32>(
                f32((i == 1u || i == 2u) ? width : 0u),
                f32((i == 2u || i == 3u) ? height : 0u)
            ),
            material,
            1.0 // TODO: Calculate AO
        );
    }
    
    // Write indices (two triangles)
    indices[index_offset + 0u] = vertex_offset + 0u;
    indices[index_offset + 1u] = vertex_offset + 1u;
    indices[index_offset + 2u] = vertex_offset + 2u;
    
    indices[index_offset + 3u] = vertex_offset + 0u;
    indices[index_offset + 4u] = vertex_offset + 2u;
    indices[index_offset + 5u] = vertex_offset + 3u;
    
    // Update quad count
    atomicAdd(&mesh_output.quad_count, 1u);
}

@compute @workgroup_size(WORKGROUP_SIZE, WORKGROUP_SIZE)
fn main(@builtin(workgroup_id) workgroup_id: vec3<u32>) {
    // Each workgroup processes one face direction
    let face = workgroup_id.x;
    let slice = workgroup_id.y;
    
    if (face >= 6u || slice >= CHUNK_SIZE) {
        return;
    }
    
    // Extract quads for this face and slice
    extract_quads_for_face(face, slice);
}