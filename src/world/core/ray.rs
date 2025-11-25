use super::{BlockId, VoxelPos};
use cgmath::{InnerSpace, Point3, Vector3};

#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Point3<f32>,
    pub direction: Vector3<f32>,
}

impl Ray {
    pub fn new(origin: Point3<f32>, direction: Vector3<f32>) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BlockFace {
    Right,  // +X
    Left,   // -X
    Top,    // +Y
    Bottom, // -Y
    Front,  // +Z
    Back,   // -Z
    // Cardinal directions (aliases for compatibility)
    North,  // +Z
    South,  // -Z
    East,   // +X
    West,   // -X
}

impl BlockFace {
    pub fn normal(&self) -> Vector3<f32> {
        match self {
            BlockFace::Right | BlockFace::East => Vector3::new(1.0, 0.0, 0.0),
            BlockFace::Left | BlockFace::West => Vector3::new(-1.0, 0.0, 0.0),
            BlockFace::Top => Vector3::new(0.0, 1.0, 0.0),
            BlockFace::Bottom => Vector3::new(0.0, -1.0, 0.0),
            BlockFace::Front | BlockFace::North => Vector3::new(0.0, 0.0, 1.0),
            BlockFace::Back | BlockFace::South => Vector3::new(0.0, 0.0, -1.0),
        }
    }

    pub fn offset(&self) -> Vector3<i32> {
        match self {
            BlockFace::Right | BlockFace::East => Vector3::new(1, 0, 0),
            BlockFace::Left | BlockFace::West => Vector3::new(-1, 0, 0),
            BlockFace::Top => Vector3::new(0, 1, 0),
            BlockFace::Bottom => Vector3::new(0, -1, 0),
            BlockFace::Front | BlockFace::North => Vector3::new(0, 0, 1),
            BlockFace::Back | BlockFace::South => Vector3::new(0, 0, -1),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RaycastHit {
    pub position: VoxelPos,
    pub face: BlockFace,
    pub distance: f32,
    pub block: BlockId,
}

// Legacy cast_ray removed - use world_operations::raycast instead

fn determine_hit_face(hit_point: Point3<f32>, voxel_pos: VoxelPos) -> BlockFace {
    // Calculate the local position within the voxel (0-1 range)
    let local_x = hit_point.x - voxel_pos.x as f32;
    let local_y = hit_point.y - voxel_pos.y as f32;
    let local_z = hit_point.z - voxel_pos.z as f32;

    // Find which face is closest to the hit point
    let epsilon = 0.01;

    if local_x < epsilon {
        BlockFace::Left
    } else if local_x > 1.0 - epsilon {
        BlockFace::Right
    } else if local_y < epsilon {
        BlockFace::Bottom
    } else if local_y > 1.0 - epsilon {
        BlockFace::Top
    } else if local_z < epsilon {
        BlockFace::Back
    } else {
        BlockFace::Front
    }
}
