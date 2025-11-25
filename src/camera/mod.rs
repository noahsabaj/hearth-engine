/// Camera Module - Data-Oriented Programming (DOP) style
///
/// This module follows pure DOP principles:
/// - camera_data.rs: Pure data structures with NO methods
/// - camera_operations.rs: Pure functions that operate on data
/// 

pub mod camera_data;
pub mod camera_operations;

// Re-export data structures
pub use camera_data::{CameraData, CameraTransformBatch, CameraUniform};

// Re-export all operations
pub use camera_operations::{
    // Initialization
    init_camera,
    init_camera_with_spawn,
    
    // View/projection
    build_view_matrix,
    build_projection_matrix,
    build_camera_uniform,
    
    // Updates
    update_aspect_ratio,
    
    // Movement
    move_forward,
    move_right,
    move_up,
    rotate,
    
    // Batch operations
    default_camera_transform_batch,
    apply_transform_batch,
    
    // Utilities
    calculate_forward_vector,
    calculate_right_vector,
    
    // Diagnostics
    camera_chunk_position,
    camera_local_position,
    distance_to_chunk,
    log_camera_context,
    chunks_in_view_distance,
    log_performance_context,
};

// Compatibility aliases for easier migration
pub use camera_operations::{
    move_forward as camera_move_forward,
    move_right as camera_move_right,
    move_up as camera_move_up,
    rotate as camera_rotate,
};

// Resize is just an alias for update_aspect_ratio
pub fn camera_resize(camera: &CameraData, width: u32, height: u32) -> CameraData {
    update_aspect_ratio(camera, width, height)
}

// Calculate forward vector from camera data (compatibility function)
pub fn calculate_forward_vector_from_camera(camera_data: &CameraData) -> cgmath::Vector3<f32> {
    calculate_forward_vector(camera_data.yaw_radians, camera_data.pitch_radians)
}