//! Camera operations - Pure DOP functions
//!
//! All functions are pure: they take data, return new data, no side effects.
//! No methods, no self, just transformations.

use super::camera_data::{CameraData, CameraTransformBatch, CameraUniform, CameraConfig};
use crate::world::core::{ChunkPos, VoxelPos};
use cgmath::{InnerSpace, Matrix4, Point3, Rad, Vector3};

// ============================================================================
// INITIALIZATION
// ============================================================================

/// Initialize camera with default settings
pub fn init_camera(position: Point3<f32>, yaw: f32, pitch: f32) -> CameraData {
    CameraData {
        position,
        yaw_radians: yaw,
        pitch_radians: pitch,
        ..Default::default()
    }
}

/// Initialize camera with spawn position
pub fn init_camera_with_spawn(spawn: Point3<f32>) -> CameraData {
    CameraData {
        position: spawn,
        ..Default::default()
    }
}

/// Initialize camera from config
pub fn init_camera_from_config(config: &CameraConfig) -> CameraData {
    CameraData {
        position: config.initial_position,
        yaw_radians: config.initial_yaw,
        pitch_radians: config.initial_pitch,
        fov_radians: config.fov_degrees.to_radians(),
        aspect_ratio: config.aspect_ratio,
        near_plane: config.near_plane,
        far_plane: config.far_plane,
        movement_speed: config.movement_speed,
        rotation_sensitivity: config.rotation_sensitivity,
    }
}

// ============================================================================
// VIEW/PROJECTION MATRICES
// ============================================================================

/// Build view matrix from camera data
pub fn build_view_matrix(camera: &CameraData) -> Matrix4<f32> {
    let forward = calculate_forward_vector(camera.yaw_radians, camera.pitch_radians);
    let target = camera.position + forward;
    let up = Vector3::new(0.0, 1.0, 0.0);

    Matrix4::look_at_rh(camera.position, target, up)
}

/// Build projection matrix from camera data
pub fn build_projection_matrix(camera: &CameraData) -> Matrix4<f32> {
    cgmath::perspective(
        Rad(camera.fov_radians),
        camera.aspect_ratio,
        camera.near_plane,
        camera.far_plane,
    )
}

/// Build camera uniform for GPU
pub fn build_camera_uniform(camera: &CameraData) -> CameraUniform {
    let view_matrix = build_view_matrix(camera);
    let projection_matrix = build_projection_matrix(camera);
    let view_projection = projection_matrix * view_matrix;

    let forward = calculate_forward_vector(camera.yaw_radians, camera.pitch_radians);
    let right = calculate_right_vector(camera.yaw_radians);
    let up = right.cross(forward).normalize();

    CameraUniform {
        view_matrix: view_matrix.into(),
        projection_matrix: projection_matrix.into(),
        view_projection_matrix: view_projection.into(),
        camera_position: [camera.position.x, camera.position.y, camera.position.z, 1.0],
        camera_forward: [forward.x, forward.y, forward.z, 0.0],
        camera_right: [right.x, right.y, right.z, 0.0],
        camera_up: [up.x, up.y, up.z, 0.0],
        planes: [camera.near_plane, camera.far_plane, 0.0, 0.0],
        fov: camera.fov_radians,
        aspect: camera.aspect_ratio,
        _padding: [0.0, 0.0],
    }
}

// ============================================================================
// UPDATES
// ============================================================================

/// Update aspect ratio (e.g., on window resize)
pub fn update_aspect_ratio(camera: &CameraData, width: u32, height: u32) -> CameraData {
    let mut new_camera = *camera;
    new_camera.aspect_ratio = width as f32 / height as f32;
    new_camera
}

/// Update FOV
pub fn update_fov(camera: &CameraData, fov_degrees: f32) -> CameraData {
    let mut new_camera = *camera;
    new_camera.fov_radians = fov_degrees.to_radians();
    new_camera
}

// ============================================================================
// MOVEMENT
// ============================================================================

/// Move camera forward by distance (in camera's forward direction)
pub fn move_forward(camera: &CameraData, distance: f32) -> CameraData {
    let forward = calculate_forward_vector(camera.yaw_radians, camera.pitch_radians);
    let mut new_camera = *camera;
    new_camera.position += forward * distance;
    new_camera
}

/// Move camera right by distance (in camera's right direction)
pub fn move_right(camera: &CameraData, distance: f32) -> CameraData {
    let right = calculate_right_vector(camera.yaw_radians);
    let mut new_camera = *camera;
    new_camera.position += right * distance;
    new_camera
}

/// Move camera up by distance (in world up direction)
pub fn move_up(camera: &CameraData, distance: f32) -> CameraData {
    let mut new_camera = *camera;
    new_camera.position.y += distance;
    new_camera
}

/// Rotate camera by yaw/pitch deltas (radians)
pub fn rotate(camera: &CameraData, yaw_delta: f32, pitch_delta: f32) -> CameraData {
    let mut new_camera = *camera;
    new_camera.yaw_radians += yaw_delta;
    new_camera.pitch_radians += pitch_delta;

    // Clamp pitch to avoid gimbal lock
    const PITCH_LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 0.01;
    new_camera.pitch_radians = new_camera.pitch_radians.clamp(-PITCH_LIMIT, PITCH_LIMIT);

    new_camera
}

// ============================================================================
// BATCH OPERATIONS
// ============================================================================

/// Create default transform batch (no movement)
pub fn default_camera_transform_batch() -> CameraTransformBatch {
    CameraTransformBatch::default()
}

/// Apply batch of transformations to camera
pub fn apply_transform_batch(
    camera: &CameraData,
    batch: &CameraTransformBatch,
    delta_time: f32,
) -> CameraData {
    let mut result = *camera;

    // Apply rotations
    result = rotate(&result, batch.yaw_delta, batch.pitch_delta);

    // Apply movements scaled by delta time
    let scaled_forward = batch.forward_delta * delta_time;
    let scaled_right = batch.right_delta * delta_time;
    let scaled_up = batch.up_delta * delta_time;

    result = move_forward(&result, scaled_forward);
    result = move_right(&result, scaled_right);
    result = move_up(&result, scaled_up);

    // Apply FOV change
    if batch.fov_delta.abs() > 0.001 {
        result.fov_radians += batch.fov_delta;
        result.fov_radians = result.fov_radians.clamp(0.1, 3.0);
    }

    result
}

// ============================================================================
// UTILITIES
// ============================================================================

/// Calculate forward vector from yaw and pitch
pub fn calculate_forward_vector(yaw: f32, pitch: f32) -> Vector3<f32> {
    Vector3::new(
        yaw.cos() * pitch.cos(),
        pitch.sin(),
        yaw.sin() * pitch.cos(),
    )
    .normalize()
}

/// Calculate right vector from yaw
pub fn calculate_right_vector(yaw: f32) -> Vector3<f32> {
    Vector3::new(
        (yaw - std::f32::consts::FRAC_PI_2).cos(),
        0.0,
        (yaw - std::f32::consts::FRAC_PI_2).sin(),
    )
    .normalize()
}

/// Calculate up vector from yaw and pitch
pub fn calculate_up_vector(yaw: f32, pitch: f32) -> Vector3<f32> {
    let forward = calculate_forward_vector(yaw, pitch);
    let right = calculate_right_vector(yaw);
    right.cross(forward).normalize()
}

// ============================================================================
// DIAGNOSTICS
// ============================================================================

/// Get camera's current chunk position
pub fn camera_chunk_position(camera: &CameraData, chunk_size: u32) -> ChunkPos {
    let chunk_size = chunk_size as i32;
    ChunkPos {
        x: (camera.position.x as i32) / chunk_size,
        y: (camera.position.y as i32) / chunk_size,
        z: (camera.position.z as i32) / chunk_size,
    }
}

/// Get camera's local position within its chunk
pub fn camera_local_position(camera: &CameraData, chunk_size: u32) -> VoxelPos {
    let chunk_size = chunk_size as i32;
    VoxelPos {
        x: (camera.position.x as i32) % chunk_size,
        y: (camera.position.y as i32) % chunk_size,
        z: (camera.position.z as i32) % chunk_size,
    }
}

/// Calculate distance from camera to chunk center
pub fn distance_to_chunk(camera: &CameraData, chunk_pos: ChunkPos, chunk_size: u32) -> f32 {
    let half_chunk = chunk_size as f32 / 2.0;
    let chunk_center = Point3::new(
        chunk_pos.x as f32 * chunk_size as f32 + half_chunk,
        chunk_pos.y as f32 * chunk_size as f32 + half_chunk,
        chunk_pos.z as f32 * chunk_size as f32 + half_chunk,
    );

    let dx = camera.position.x - chunk_center.x;
    let dy = camera.position.y - chunk_center.y;
    let dz = camera.position.z - chunk_center.z;

    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Get all chunks within view distance
pub fn chunks_in_view_distance(
    camera: &CameraData,
    chunk_size: u32,
    view_distance_chunks: u32,
) -> Vec<ChunkPos> {
    let camera_chunk = camera_chunk_position(camera, chunk_size);
    let radius = view_distance_chunks as i32;

    let mut chunks = Vec::new();

    for x in (camera_chunk.x - radius)..=(camera_chunk.x + radius) {
        for y in (camera_chunk.y - radius)..=(camera_chunk.y + radius) {
            for z in (camera_chunk.z - radius)..=(camera_chunk.z + radius) {
                let chunk_pos = ChunkPos { x, y, z };
                let distance = distance_to_chunk(camera, chunk_pos, chunk_size);

                // Use spherical view distance
                if distance <= (view_distance_chunks * chunk_size) as f32 {
                    chunks.push(chunk_pos);
                }
            }
        }
    }

    chunks
}

/// Log camera context for debugging
pub fn log_camera_context(camera: &CameraData, chunk_size: u32) {
    let chunk_pos = camera_chunk_position(camera, chunk_size);
    let local_pos = camera_local_position(camera, chunk_size);

    log::debug!(
        "[Camera] Position: ({:.1}, {:.1}, {:.1}) | Chunk: ({}, {}, {}) | Local: ({}, {}, {})",
        camera.position.x,
        camera.position.y,
        camera.position.z,
        chunk_pos.x,
        chunk_pos.y,
        chunk_pos.z,
        local_pos.x,
        local_pos.y,
        local_pos.z
    );

    log::debug!(
        "[Camera] Yaw: {:.3}rad ({:.1}°) | Pitch: {:.3}rad ({:.1}°) | FOV: {:.3}rad ({:.1}°)",
        camera.yaw_radians,
        camera.yaw_radians.to_degrees(),
        camera.pitch_radians,
        camera.pitch_radians.to_degrees(),
        camera.fov_radians,
        camera.fov_radians.to_degrees()
    );
}

/// Log performance context
pub fn log_performance_context(
    camera: &CameraData,
    chunk_size: u32,
    view_distance: u32,
    loaded_chunks: usize,
) {
    let visible_chunks = chunks_in_view_distance(camera, chunk_size, view_distance);

    log::debug!(
        "[Camera Performance] Chunks in view: {} | Loaded: {} | View distance: {} chunks",
        visible_chunks.len(),
        loaded_chunks,
        view_distance
    );
}
