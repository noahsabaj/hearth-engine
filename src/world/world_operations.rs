//! World Operations - Pure DOP Functions
//!
//! This is the PUBLIC API for world manipulation.
//! All functions are pure: take data, return results, no side effects.
//!
//! This is what GAMES call directly to interact with the world.

use super::core::{BlockId, ChunkPos, Ray, RaycastHit, VoxelPos, BlockFace};
use super::data_types::WorldData;
use super::error::WorldError;
use cgmath::{InnerSpace, Point3};

// ============================================================================
// BLOCK OPERATIONS
// ============================================================================

/// Get block at position (pure function)
///
/// # Arguments
/// * `world` - World data to read from
/// * `pos` - Voxel position
/// * `chunk_size` - Chunk size (usually 50)
///
/// # Returns
/// BlockId at that position, or AIR if out of bounds
pub fn get_block(world: &WorldData, pos: VoxelPos, chunk_size: u32) -> BlockId {
    // Calculate chunk position
    let chunk_size_i32 = chunk_size as i32;
    let chunk_pos = ChunkPos {
        x: pos.x.div_euclid(chunk_size_i32),
        y: pos.y.div_euclid(chunk_size_i32),
        z: pos.z.div_euclid(chunk_size_i32),
    };

    // Find chunk in world data
    if let Some(chunk) = world.chunks.iter().find(|c| c.position == chunk_pos) {
        // Calculate local position within chunk
        let local_x = pos.x.rem_euclid(chunk_size_i32) as u32;
        let local_y = pos.y.rem_euclid(chunk_size_i32) as u32;
        let local_z = pos.z.rem_euclid(chunk_size_i32) as u32;

        // Calculate index in flat array
        let index = (local_x + local_y * chunk_size + local_z * chunk_size * chunk_size) as usize;

        if index < chunk.blocks.len() {
            chunk.blocks[index]
        } else {
            BlockId::AIR
        }
    } else {
        BlockId::AIR
    }
}

/// Set block at position (returns new world state)
///
/// # Arguments
/// * `world` - World data to modify
/// * `pos` - Voxel position
/// * `block_id` - Block to set
/// * `chunk_size` - Chunk size (usually 50)
///
/// # Returns
/// Ok(WorldModification) if successful
pub fn set_block(
    world: &mut WorldData,
    pos: VoxelPos,
    block_id: BlockId,
    chunk_size: u32,
) -> Result<WorldModification, WorldError> {
    // Calculate chunk position
    let chunk_size_i32 = chunk_size as i32;
    let chunk_pos = ChunkPos {
        x: pos.x.div_euclid(chunk_size_i32),
        y: pos.y.div_euclid(chunk_size_i32),
        z: pos.z.div_euclid(chunk_size_i32),
    };

    // Find chunk in world data
    if let Some(chunk) = world.chunks.iter_mut().find(|c| c.position == chunk_pos) {
        // Calculate local position within chunk
        let local_x = pos.x.rem_euclid(chunk_size_i32) as u32;
        let local_y = pos.y.rem_euclid(chunk_size_i32) as u32;
        let local_z = pos.z.rem_euclid(chunk_size_i32) as u32;

        // Calculate index in flat array
        let index = (local_x + local_y * chunk_size + local_z * chunk_size * chunk_size) as usize;

        if index < chunk.blocks.len() {
            let old_block = chunk.blocks[index];
            chunk.blocks[index] = block_id;

            Ok(WorldModification {
                position: pos,
                old_block,
                new_block: block_id,
                timestamp: world.tick,
            })
        } else {
            Err(WorldError::InvalidPosition)
        }
    } else {
        Err(WorldError::ChunkNotLoaded)
    }
}

/// World modification record
#[derive(Clone, Copy, Debug)]
pub struct WorldModification {
    pub position: VoxelPos,
    pub old_block: BlockId,
    pub new_block: BlockId,
    pub timestamp: u64,
}

// ============================================================================
// RAYCASTING
// ============================================================================

/// Raycast through world to find first solid block
///
/// # Arguments
/// * `world` - World data to raycast through
/// * `ray` - Ray to cast (origin + direction)
/// * `max_distance` - Maximum distance to check
/// * `chunk_size` - Chunk size (usually 50)
///
/// # Returns
/// Some(RaycastHit) if hit, None if no hit
pub fn raycast(
    world: &WorldData,
    ray: Ray,
    max_distance: f32,
    chunk_size: u32,
) -> Option<RaycastHit> {
    let step_size = 0.1; // 10cm steps (1 voxel = 10cm)
    let mut distance = 0.0;

    while distance <= max_distance {
        // Calculate current point along ray
        let point = Point3::new(
            ray.origin.x + ray.direction.x * distance,
            ray.origin.y + ray.direction.y * distance,
            ray.origin.z + ray.direction.z * distance,
        );

        // Convert to voxel position
        let voxel_pos = VoxelPos {
            x: point.x.floor() as i32,
            y: point.y.floor() as i32,
            z: point.z.floor() as i32,
        };

        // Check block at this position
        let block = get_block(world, voxel_pos, chunk_size);

        if block != BlockId::AIR {
            // Hit! Calculate which face we hit
            let face = calculate_hit_face(&point, &voxel_pos);

            return Some(RaycastHit {
                position: voxel_pos,
                face,
                distance,
                block,
            });
        }

        distance += step_size;
    }

    None
}

/// Calculate which face of a block was hit
fn calculate_hit_face(hit_point: &Point3<f32>, voxel_pos: &VoxelPos) -> BlockFace {
    // Calculate relative position within voxel (0.0 to 1.0)
    let rel_x = hit_point.x - voxel_pos.x as f32;
    let rel_y = hit_point.y - voxel_pos.y as f32;
    let rel_z = hit_point.z - voxel_pos.z as f32;

    // Find which face is closest
    let distances = [
        (rel_x, BlockFace::West),           // -X
        (1.0 - rel_x, BlockFace::East),     // +X
        (rel_y, BlockFace::Bottom),         // -Y
        (1.0 - rel_y, BlockFace::Top),      // +Y
        (rel_z, BlockFace::North),          // -Z
        (1.0 - rel_z, BlockFace::South),    // +Z
    ];

    distances
        .iter()
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, face)| *face)
        .unwrap_or(BlockFace::Top)
}

// ============================================================================
// CHUNK OPERATIONS
// ============================================================================

/// Check if chunk is loaded
pub fn is_chunk_loaded(world: &WorldData, chunk_pos: ChunkPos) -> bool {
    world.active_chunks.contains(&chunk_pos)
}

/// Load a chunk (mark as active and generate if needed)
pub fn load_chunk(
    world: &mut WorldData,
    chunk_pos: ChunkPos,
    chunk_size: u32,
) -> Result<(), WorldError> {
    // Check if already loaded
    if world.active_chunks.contains(&chunk_pos) {
        return Ok(());
    }

    // Check if chunk exists in storage
    let chunk_exists = world.chunks.iter().any(|c| c.position == chunk_pos);

    if !chunk_exists {
        // Create new empty chunk
        use super::data_types::{ChunkData, ChunkMetadata};
        let blocks_per_chunk = (chunk_size * chunk_size * chunk_size) as usize;
        let new_chunk = ChunkData {
            position: chunk_pos,
            blocks: vec![BlockId::AIR; blocks_per_chunk],
            flags: ChunkMetadata::default(),
            last_modified: world.tick,
        };
        world.chunks.push(new_chunk);
    }

    // Mark as active
    world.active_chunks.insert(chunk_pos);

    Ok(())
}

/// Unload a chunk (mark as inactive)
pub fn unload_chunk(world: &mut WorldData, chunk_pos: ChunkPos) -> Result<(), WorldError> {
    world.active_chunks.remove(&chunk_pos);
    Ok(())
}

/// Get all loaded chunks
pub fn get_loaded_chunks(world: &WorldData) -> Vec<ChunkPos> {
    world.active_chunks.iter().copied().collect()
}

/// Get chunks in radius around a position
pub fn get_chunks_in_radius(
    center: ChunkPos,
    radius: u32,
) -> Vec<ChunkPos> {
    let mut chunks = Vec::new();
    let radius = radius as i32;

    for x in (center.x - radius)..=(center.x + radius) {
        for y in (center.y - radius)..=(center.y + radius) {
            for z in (center.z - radius)..=(center.z + radius) {
                let chunk_pos = ChunkPos { x, y, z };

                // Check if within spherical radius
                let dx = (chunk_pos.x - center.x) as f32;
                let dy = (chunk_pos.y - center.y) as f32;
                let dz = (chunk_pos.z - center.z) as f32;
                let distance_sq = dx * dx + dy * dy + dz * dz;

                if distance_sq <= (radius * radius) as f32 {
                    chunks.push(chunk_pos);
                }
            }
        }
    }

    chunks
}

// ============================================================================
// WORLD QUERIES
// ============================================================================

/// Get world size (in chunks)
pub fn get_world_size(world: &WorldData) -> (u32, u32, u32) {
    (world.size_x, world.size_y, world.size_z)
}

/// Get world seed
pub fn get_world_seed(world: &WorldData) -> u32 {
    world.seed
}

/// Get world tick
pub fn get_world_tick(world: &WorldData) -> u64 {
    world.tick
}

/// Get active chunk count
pub fn get_active_chunk_count(world: &WorldData) -> usize {
    world.active_chunks.len()
}

// ============================================================================
// BATCH OPERATIONS
// ============================================================================

/// Set multiple blocks at once (more efficient than individual sets)
pub fn set_blocks_batch(
    world: &mut WorldData,
    blocks: &[(VoxelPos, BlockId)],
    chunk_size: u32,
) -> Vec<Result<WorldModification, WorldError>> {
    blocks
        .iter()
        .map(|(pos, block_id)| set_block(world, *pos, *block_id, chunk_size))
        .collect()
}

/// Get multiple blocks at once
pub fn get_blocks_batch(
    world: &WorldData,
    positions: &[VoxelPos],
    chunk_size: u32,
) -> Vec<BlockId> {
    positions
        .iter()
        .map(|pos| get_block(world, *pos, chunk_size))
        .collect()
}

// ============================================================================
// UTILITIES
// ============================================================================

/// Convert voxel position to chunk position
pub fn voxel_to_chunk(pos: VoxelPos, chunk_size: u32) -> ChunkPos {
    let chunk_size_i32 = chunk_size as i32;
    ChunkPos {
        x: pos.x.div_euclid(chunk_size_i32),
        y: pos.y.div_euclid(chunk_size_i32),
        z: pos.z.div_euclid(chunk_size_i32),
    }
}

/// Convert chunk position to world position (chunk corner)
pub fn chunk_to_world(chunk_pos: ChunkPos, chunk_size: u32) -> VoxelPos {
    let chunk_size_i32 = chunk_size as i32;
    VoxelPos {
        x: chunk_pos.x * chunk_size_i32,
        y: chunk_pos.y * chunk_size_i32,
        z: chunk_pos.z * chunk_size_i32,
    }
}

/// Get local position within chunk (0 to chunk_size-1)
pub fn get_local_position(pos: VoxelPos, chunk_size: u32) -> (u32, u32, u32) {
    let chunk_size_i32 = chunk_size as i32;
    (
        pos.x.rem_euclid(chunk_size_i32) as u32,
        pos.y.rem_euclid(chunk_size_i32) as u32,
        pos.z.rem_euclid(chunk_size_i32) as u32,
    )
}

// ============================================================================
// DIAGNOSTICS
// ============================================================================

/// Log world statistics
pub fn log_world_stats(world: &WorldData) {
    log::info!("[World] Statistics:");
    log::info!("  World size: {}x{}x{} chunks", world.size_x, world.size_y, world.size_z);
    log::info!("  Seed: {}", world.seed);
    log::info!("  Tick: {}", world.tick);
    log::info!("  Active chunks: {}", world.active_chunks.len());
    log::info!("  Chunk capacity: {}", world.chunk_capacity);
}

/// Validate world data integrity
pub fn validate_world_data(world: &WorldData, chunk_size: u32) -> Result<(), String> {
    let expected_blocks_per_chunk = (chunk_size * chunk_size * chunk_size) as usize;

    for chunk in &world.chunks {
        if chunk.blocks.len() != expected_blocks_per_chunk {
            return Err(format!(
                "Chunk at {:?} has {} blocks, expected {}",
                chunk.position,
                chunk.blocks.len(),
                expected_blocks_per_chunk
            ));
        }
    }

    Ok(())
}
