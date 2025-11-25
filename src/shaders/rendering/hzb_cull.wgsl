/// Hierarchical Z-Buffer Occlusion Culling
/// 
/// Performs GPU occlusion culling using a hierarchical depth buffer.
/// Works in tandem with frustum culling for maximum efficiency.

struct ChunkInstance {
    world_position: vec3<f32>,
    chunk_size: f32,
    lod_level: u32,
    flags: u32,
    _padding: vec2<f32>,
}

struct Camera {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

// HZB mip levels - supports up to 4K resolution
@group(0) @binding(0) var hzb_texture: texture_2d<f32>;
@group(0) @binding(1) var hzb_sampler: sampler;
@group(0) @binding(2) var<uniform> camera: Camera;
@group(0) @binding(3) var<storage, read> chunk_instances: array<ChunkInstance>;
@group(0) @binding(4) var<storage, read> visible_from_frustum: array<u32>;
@group(0) @binding(5) var<storage, read_write> visible_after_occlusion: array<u32>;
@group(0) @binding(6) var<storage, read_write> occlusion_count: atomic<u32>;

// Constants
const WORKGROUP_SIZE: u32 = 64u;
const HZB_LEVELS: u32 = 12u; // log2(4096)

/// Project AABB to screen space and return conservative screen rect
fn project_aabb_to_screen(center: vec3<f32>, half_extents: f32) -> vec4<f32> {
    var min_screen = vec2<f32>(1.0, 1.0);
    var max_screen = vec2<f32>(-1.0, -1.0);
    
    // Test all 8 corners of the AABB
    for (var i = 0u; i < 8u; i++) {
        let corner = center + vec3<f32>(
            select(-half_extents, half_extents, (i & 1u) != 0u),
            select(-half_extents, half_extents, (i & 2u) != 0u),
            select(-half_extents, half_extents, (i & 4u) != 0u)
        );
        
        // Project to clip space
        let clip_pos = camera.view_proj * vec4<f32>(corner, 1.0);
        
        // Skip if behind camera
        if (clip_pos.w <= 0.0) {
            continue;
        }
        
        // Perspective divide to NDC
        let ndc = clip_pos.xy / clip_pos.w;
        
        // Update screen bounds
        min_screen = min(min_screen, ndc);
        max_screen = max(max_screen, ndc);
    }
    
    // Convert from NDC [-1,1] to texture coordinates [0,1]
    min_screen = (min_screen + 1.0) * 0.5;
    max_screen = (max_screen + 1.0) * 0.5;
    
    // Return as (minX, minY, maxX, maxY)
    return vec4<f32>(min_screen, max_screen);
}

/// Get the minimum depth from the HZB for a screen rect
fn sample_hzb_conservative(screen_rect: vec4<f32>, closest_z: f32) -> f32 {
    let hzb_dims = textureDimensions(hzb_texture, 0);
    
    // Convert to pixel coordinates
    let min_pixel = vec2<i32>(screen_rect.xy * vec2<f32>(hzb_dims));
    let max_pixel = vec2<i32>(screen_rect.zw * vec2<f32>(hzb_dims));
    
    // Calculate rect size and choose appropriate mip level
    let rect_size = max_pixel - min_pixel;
    let max_size = max(rect_size.x, rect_size.y);
    let mip_level = min(u32(ceil(log2(f32(max_size)))), HZB_LEVELS - 1u);
    
    // Sample the four corners at the chosen mip level
    let mip_scale = f32(1u << mip_level);
    let sample_min = vec2<f32>(min_pixel) / mip_scale;
    let sample_max = vec2<f32>(max_pixel) / mip_scale;
    
    // Conservative sampling - take minimum of 4 samples
    var min_depth = 1.0;
    min_depth = min(min_depth, textureSampleLevel(hzb_texture, hzb_sampler, sample_min, f32(mip_level)).r);
    min_depth = min(min_depth, textureSampleLevel(hzb_texture, hzb_sampler, vec2<f32>(sample_max.x, sample_min.y), f32(mip_level)).r);
    min_depth = min(min_depth, textureSampleLevel(hzb_texture, hzb_sampler, vec2<f32>(sample_min.x, sample_max.y), f32(mip_level)).r);
    min_depth = min(min_depth, textureSampleLevel(hzb_texture, hzb_sampler, sample_max, f32(mip_level)).r);
    
    return min_depth;
}

/// Test if chunk is occluded by the HZB
fn is_occluded(chunk: ChunkInstance) -> bool {
    let center = chunk.world_position + vec3<f32>(chunk.chunk_size * 0.5);
    let half_extents = chunk.chunk_size * 0.5;
    
    // Project AABB to screen
    let screen_rect = project_aabb_to_screen(center, half_extents);
    
    // Check if completely off-screen
    if (screen_rect.z < 0.0 || screen_rect.w < 0.0 || 
        screen_rect.x > 1.0 || screen_rect.y > 1.0) {
        return true; // Off-screen, consider occluded
    }
    
    // Get closest Z of the AABB in view space
    let view_center = (camera.view_proj * vec4<f32>(center, 1.0)).z;
    let closest_z = view_center - half_extents; // Conservative
    
    // Normalize to depth buffer range [0,1]
    let near = 0.1; // Should match camera near plane
    let far = 1000.0; // Should match camera far plane
    let normalized_z = (closest_z - near) / (far - near);
    
    // Sample HZB
    let hzb_depth = sample_hzb_conservative(screen_rect, normalized_z);
    
    // Object is occluded if its closest point is behind the HZB
    return normalized_z > hzb_depth;
}

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let thread_id = global_id.x;
    
    // Get the index from frustum culling results
    let visible_count = arrayLength(&visible_from_frustum);
    if (thread_id >= visible_count) {
        return;
    }
    
    let chunk_index = visible_from_frustum[thread_id];
    let chunk = chunk_instances[chunk_index];
    
    // Skip occlusion culling for very close objects
    let distance = length(camera.position - chunk.world_position);
    if (distance < 50.0) {
        // Always visible when close
        let out_index = atomicAdd(&occlusion_count, 1u);
        visible_after_occlusion[out_index] = chunk_index;
        return;
    }
    
    // Perform occlusion test
    if (!is_occluded(chunk)) {
        let out_index = atomicAdd(&occlusion_count, 1u);
        visible_after_occlusion[out_index] = chunk_index;
    }
}

/// Build HZB mip chain from depth buffer
@compute @workgroup_size(8, 8)
fn build_hzb_mip(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    // This would be a separate compute shader that builds the HZB mip chain
    // For now, this is a placeholder showing the structure
    
    // Each dispatch builds one mip level from the previous
    // Takes maximum depth (furthest) in 2x2 regions for conservative culling
}