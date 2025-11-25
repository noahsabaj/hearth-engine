/// GPU-Driven Frustum Culling Compute Shader
/// 
/// Performs frustum culling entirely on GPU with zero CPU involvement.
/// Writes visibility results and indirect draw commands directly.

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
    frustum_planes: array<vec4<f32>, 6>, // Left, Right, Top, Bottom, Near, Far
}

struct ChunkInstance {
    world_position: vec3<f32>,
    chunk_size: f32,
    lod_level: u32,
    flags: u32,
    _padding: vec2<f32>,
}

struct DrawCommand {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

struct CullingStats {
    total_chunks: atomic<u32>,
    visible_chunks: atomic<u32>,
    frustum_culled: atomic<u32>,
    distance_culled: atomic<u32>,
}

// Uniforms
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<storage, read> chunk_instances: array<ChunkInstance>;
@group(0) @binding(2) var<storage, read_write> draw_commands: array<DrawCommand>;
@group(0) @binding(3) var<storage, read_write> visible_instances: array<u32>;
@group(0) @binding(4) var<storage, read_write> culling_stats: CullingStats;
@group(0) @binding(5) var<storage, read_write> draw_count: atomic<u32>;

// Constants
const CHUNK_VERTICES: u32 = 36u; // 6 faces * 2 triangles * 3 vertices
const MAX_RENDER_DISTANCE: f32 = 1000.0;
const WORKGROUP_SIZE: u32 = 128u;

// Shared memory for parallel reduction
var<workgroup> shared_visible_count: atomic<u32>;
var<workgroup> shared_visible_indices: array<u32, WORKGROUP_SIZE>;

/// Check if AABB is inside frustum planes
fn is_aabb_in_frustum(center: vec3<f32>, half_extents: f32) -> bool {
    // Test against all 6 frustum planes
    for (var i = 0u; i < 6u; i++) {
        let plane = camera.frustum_planes[i];
        let distance = dot(plane.xyz, center) + plane.w;
        
        // Conservative test - use sphere radius
        let radius = half_extents * 1.732; // sqrt(3) for diagonal
        
        if (distance < -radius) {
            return false; // Outside this plane
        }
    }
    
    return true; // Inside all planes
}

/// Calculate LOD based on distance
fn calculate_lod(distance: f32) -> u32 {
    if (distance < 32.0) {
        return 0u; // Full detail
    } else if (distance < 64.0) {
        return 1u; // Half detail
    } else if (distance < 128.0) {
        return 2u; // Quarter detail
    } else {
        return 3u; // Minimum detail
    }
}

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let chunk_index = global_id.x;
    let local_index = local_id.x;
    
    // Initialize shared memory
    if (local_index == 0u) {
        atomicStore(&shared_visible_count, 0u);
    }
    workgroupBarrier();
    
    // Bounds check
    let total_chunks = arrayLength(&chunk_instances);
    if (chunk_index >= total_chunks) {
        return;
    }
    
    // Update total chunks stat
    if (chunk_index == 0u) {
        atomicStore(&culling_stats.total_chunks, total_chunks);
    }
    
    // Load chunk data
    let chunk = chunk_instances[chunk_index];
    let chunk_center = chunk.world_position + vec3<f32>(chunk.chunk_size * 0.5);
    let half_extents = chunk.chunk_size * 0.5;
    
    // Distance culling first (cheaper)
    let distance = length(camera.position - chunk_center);
    var is_visible = distance < MAX_RENDER_DISTANCE;
    
    if (!is_visible) {
        atomicAdd(&culling_stats.distance_culled, 1u);
    } else {
        // Frustum culling
        is_visible = is_aabb_in_frustum(chunk_center, half_extents);
        
        if (!is_visible) {
            atomicAdd(&culling_stats.frustum_culled, 1u);
        }
    }
    
    // Store visibility result
    var local_visible_count = 0u;
    if (is_visible) {
        // Update LOD based on distance
        let lod = calculate_lod(distance);
        
        // Store visible instance index in shared memory
        local_visible_count = atomicAdd(&shared_visible_count, 1u);
        shared_visible_indices[local_visible_count] = chunk_index;
        
        // Update global stats
        atomicAdd(&culling_stats.visible_chunks, 1u);
    }
    
    workgroupBarrier();
    
    // Parallel reduction to write draw commands
    let total_visible = atomicLoad(&shared_visible_count);
    
    // Each thread writes its visible instances to global memory
    if (local_index < total_visible) {
        let instance_idx = shared_visible_indices[local_index];
        let draw_idx = atomicAdd(&draw_count, 1u);
        
        // Write visible instance index
        visible_instances[draw_idx] = instance_idx;
        
        // Create indirect draw command
        draw_commands[draw_idx] = DrawCommand(
            CHUNK_VERTICES,      // vertex_count
            1u,                  // instance_count
            0u,                  // first_vertex
            draw_idx             // first_instance (index into visible_instances)
        );
    }
}

/// Entry point for clearing counters before culling
@compute @workgroup_size(1)
fn clear_counters() {
    atomicStore(&draw_count, 0u);
    atomicStore(&culling_stats.total_chunks, 0u);
    atomicStore(&culling_stats.visible_chunks, 0u);
    atomicStore(&culling_stats.frustum_culled, 0u);
    atomicStore(&culling_stats.distance_culled, 0u);
}