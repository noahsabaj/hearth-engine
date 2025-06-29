/// Indirect Chunk Rendering Shader
/// 
/// Renders chunks using indirect draw commands generated by GPU culling.
/// Part of Sprint 28: GPU-Driven Rendering Optimization

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

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<storage, read> chunk_instances: array<ChunkInstance>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) instance_index: u32, // Index into visible instances
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) chunk_coords: vec3<f32>,
    @location(2) lod_level: f32,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    // Get chunk instance data
    let chunk = chunk_instances[input.instance_index];
    
    // Transform vertex position to world space
    let world_pos = chunk.world_position + input.position * chunk.chunk_size;
    
    var output: VertexOutput;
    output.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    output.world_position = world_pos;
    output.chunk_coords = input.position + 0.5; // 0-1 range within chunk
    output.lod_level = f32(chunk.lod_level);
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Simple visualization - color based on LOD level
    let lod_colors = array<vec3<f32>, 4>(
        vec3<f32>(1.0, 1.0, 1.0), // LOD 0 - white
        vec3<f32>(0.8, 0.8, 1.0), // LOD 1 - light blue
        vec3<f32>(0.6, 0.6, 1.0), // LOD 2 - medium blue
        vec3<f32>(0.4, 0.4, 1.0), // LOD 3 - dark blue
    );
    
    let lod_index = u32(input.lod_level) % 4u;
    let base_color = lod_colors[lod_index];
    
    // Add some shading based on chunk coordinates
    let shade = input.chunk_coords.x * 0.3 + input.chunk_coords.y * 0.3 + input.chunk_coords.z * 0.4;
    let final_color = base_color * (0.5 + shade * 0.5);
    
    return vec4<f32>(final_color, 1.0);
}