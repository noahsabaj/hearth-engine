/// GPU LOD Selection Compute Shader
/// 
/// Selects appropriate LOD level based on screen size and distance.
/// Updates instance data with selected LOD for rendering.

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    fov_y: f32,
    viewport_width: f32,
    viewport_height: f32,
    _padding: vec2<f32>,
}

struct ChunkInstance {
    world_position: vec3<f32>,
    chunk_size: f32,
    lod_level: u32,
    flags: u32,
    _padding: vec2<f32>,
}

struct LodConfig {
    // Transition distances for each LOD level
    lod_distances: vec4<f32>, // LOD 0->1, 1->2, 2->3, 3->4
    // Screen space error thresholds
    screen_space_errors: vec4<f32>,
    // Minimum pixels for each LOD
    min_pixels: vec4<f32>,
    // LOD bias for quality adjustment
    lod_bias: f32,
    _padding: vec3<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> lod_config: LodConfig;
@group(0) @binding(2) var<storage, read> visible_chunks: array<u32>;
@group(0) @binding(3) var<storage, read_write> chunk_instances: array<ChunkInstance>;
@group(0) @binding(4) var<storage, read_write> lod_histogram: array<atomic<u32>, 5>; // Count per LOD level

const WORKGROUP_SIZE: u32 = 64u;

/// Calculate screen space size of a sphere
fn calculate_screen_size(center: vec3<f32>, radius: f32) -> f32 {
    let distance = length(camera.position - center);
    
    // Avoid division by zero
    if (distance < 0.001) {
        return camera.viewport_height; // Fill screen when very close
    }
    
    // Project sphere radius to screen space
    let angular_size = atan(radius / distance) * 2.0;
    let screen_size = angular_size / camera.fov_y * camera.viewport_height;
    
    return screen_size;
}

/// Select LOD based on multiple criteria
fn select_lod(chunk: ChunkInstance) -> u32 {
    let center = chunk.world_position + vec3<f32>(chunk.chunk_size * 0.5);
    let radius = chunk.chunk_size * 0.866; // sqrt(3)/2 for bounding sphere
    
    // Distance-based LOD
    let distance = length(camera.position - center);
    var distance_lod = 0u;
    
    if (distance > lod_config.lod_distances.x) {
        distance_lod = 1u;
    }
    if (distance > lod_config.lod_distances.y) {
        distance_lod = 2u;
    }
    if (distance > lod_config.lod_distances.z) {
        distance_lod = 3u;
    }
    if (distance > lod_config.lod_distances.w) {
        distance_lod = 4u;
    }
    
    // Screen space error LOD
    let screen_size = calculate_screen_size(center, radius);
    var screen_lod = 0u;
    
    if (screen_size < lod_config.min_pixels.w) {
        screen_lod = 4u;
    } else if (screen_size < lod_config.min_pixels.z) {
        screen_lod = 3u;
    } else if (screen_size < lod_config.min_pixels.y) {
        screen_lod = 2u;
    } else if (screen_size < lod_config.min_pixels.x) {
        screen_lod = 1u;
    }
    
    // Take maximum (lowest quality) of both methods
    var final_lod = max(distance_lod, screen_lod);
    
    // Apply LOD bias
    if (lod_config.lod_bias < 0.0) {
        // Negative bias increases quality
        final_lod = u32(max(0.0, f32(final_lod) + lod_config.lod_bias));
    } else {
        // Positive bias decreases quality
        final_lod = u32(min(4.0, f32(final_lod) + lod_config.lod_bias));
    }
    
    return final_lod;
}

/// Check if LOD transition should be smooth
fn should_smooth_transition(old_lod: u32, new_lod: u32, distance: f32) -> bool {
    if (old_lod == new_lod) {
        return false;
    }
    
    // Check if we're near a transition boundary
    let transition_distance = lod_config.lod_distances[min(old_lod, new_lod)];
    let transition_zone = transition_distance * 0.1; // 10% transition zone
    
    return abs(distance - transition_distance) < transition_zone;
}

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let thread_id = global_id.x;
    let visible_count = arrayLength(&visible_chunks);
    
    if (thread_id >= visible_count) {
        return;
    }
    
    // Get chunk instance
    let chunk_idx = visible_chunks[thread_id];
    var chunk = chunk_instances[chunk_idx];
    let old_lod = chunk.lod_level;
    
    // Select new LOD
    let new_lod = select_lod(chunk);
    
    // Check for smooth transition
    let center = chunk.world_position + vec3<f32>(chunk.chunk_size * 0.5);
    let distance = length(camera.position - center);
    
    if (should_smooth_transition(old_lod, new_lod, distance)) {
        // Mark for smooth transition (could blend between LODs)
        chunk.flags |= 0x1u; // SMOOTH_TRANSITION flag
    } else {
        chunk.flags &= ~0x1u; // Clear flag
    }
    
    // Update LOD level
    chunk.lod_level = new_lod;
    chunk_instances[chunk_idx] = chunk;
    
    // Update histogram for statistics
    atomicAdd(&lod_histogram[min(new_lod, 4u)], 1u);
}

/// Clear LOD histogram
@compute @workgroup_size(1)
fn clear_histogram() {
    for (var i = 0u; i < 5u; i++) {
        atomicStore(&lod_histogram[i], 0u);
    }
}