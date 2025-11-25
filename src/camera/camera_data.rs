//! Camera data structures - Pure DOP
//!
//! NO METHODS. Just data.
//! All transformations happen in camera_operations.rs

use cgmath::{Matrix4, Point3};

/// Camera data structure - pure data, no methods
#[derive(Debug, Clone, Copy)]
pub struct CameraData {
    /// Camera position in world space (voxel coordinates)
    pub position: Point3<f32>,

    /// Yaw rotation (radians, around Y axis)
    pub yaw_radians: f32,

    /// Pitch rotation (radians, around X axis)
    pub pitch_radians: f32,

    /// Field of view (vertical, radians)
    pub fov_radians: f32,

    /// Aspect ratio (width / height)
    pub aspect_ratio: f32,

    /// Near clipping plane distance
    pub near_plane: f32,

    /// Far clipping plane distance
    pub far_plane: f32,

    /// Movement speed (voxels per second)
    pub movement_speed: f32,

    /// Rotation sensitivity (radians per pixel)
    pub rotation_sensitivity: f32,
}

/// Batch transform data for camera updates
/// Allows efficient batching of camera transformations
#[derive(Debug, Clone, Copy, Default)]
pub struct CameraTransformBatch {
    /// Forward/backward movement delta
    pub forward_delta: f32,

    /// Left/right movement delta
    pub right_delta: f32,

    /// Up/down movement delta
    pub up_delta: f32,

    /// Yaw rotation delta (radians)
    pub yaw_delta: f32,

    /// Pitch rotation delta (radians)
    pub pitch_delta: f32,

    /// FOV delta (radians)
    pub fov_delta: f32,
}

/// Camera uniform buffer data for GPU
/// Must match shader layout exactly
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    /// View matrix (4x4, column-major)
    pub view_matrix: [[f32; 4]; 4],

    /// Projection matrix (4x4, column-major)
    pub projection_matrix: [[f32; 4]; 4],

    /// View-projection matrix (4x4, column-major)
    pub view_projection_matrix: [[f32; 4]; 4],

    /// Camera position (vec3 + padding)
    pub camera_position: [f32; 4],

    /// Camera forward vector (vec3 + padding)
    pub camera_forward: [f32; 4],

    /// Camera right vector (vec3 + padding)
    pub camera_right: [f32; 4],

    /// Camera up vector (vec3 + padding)
    pub camera_up: [f32; 4],

    /// Near/far planes (vec2 + padding)
    pub planes: [f32; 4], // [near, far, padding, padding]

    /// FOV (radians)
    pub fov: f32,

    /// Aspect ratio
    pub aspect: f32,

    /// Padding to align to 16 bytes
    pub _padding: [f32; 2],
}

impl Default for CameraData {
    fn default() -> Self {
        Self {
            position: Point3::new(0.0, 0.0, 0.0),
            yaw_radians: 0.0,
            pitch_radians: 0.0,
            fov_radians: 70.0_f32.to_radians(),
            aspect_ratio: 16.0 / 9.0,
            near_plane: 0.1,
            far_plane: 10000.0,
            movement_speed: 100.0, // 10 m/s (100 voxels/s with 10cm voxels)
            rotation_sensitivity: 0.002,
        }
    }
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self {
            view_matrix: Matrix4::from_scale(1.0).into(),
            projection_matrix: Matrix4::from_scale(1.0).into(),
            view_projection_matrix: Matrix4::from_scale(1.0).into(),
            camera_position: [0.0, 0.0, 0.0, 1.0],
            camera_forward: [0.0, 0.0, -1.0, 0.0],
            camera_right: [1.0, 0.0, 0.0, 0.0],
            camera_up: [0.0, 1.0, 0.0, 0.0],
            planes: [0.1, 10000.0, 0.0, 0.0],
            fov: 70.0_f32.to_radians(),
            aspect: 16.0 / 9.0,
            _padding: [0.0, 0.0],
        }
    }
}

/// Camera configuration for initialization
#[derive(Debug, Clone, Copy)]
pub struct CameraConfig {
    pub initial_position: Point3<f32>,
    pub initial_yaw: f32,
    pub initial_pitch: f32,
    pub fov_degrees: f32,
    pub aspect_ratio: f32,
    pub near_plane: f32,
    pub far_plane: f32,
    pub movement_speed: f32,
    pub rotation_sensitivity: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            initial_position: Point3::new(0.0, 70.0, 0.0), // Spawn at 70 voxels height
            initial_yaw: 0.0,
            initial_pitch: 0.0,
            fov_degrees: 70.0,
            aspect_ratio: 16.0 / 9.0,
            near_plane: 0.1,
            far_plane: 10000.0,
            movement_speed: 100.0,
            rotation_sensitivity: 0.002,
        }
    }
}
