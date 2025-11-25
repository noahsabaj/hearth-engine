use super::EntityId;
use std::sync::atomic::{AtomicU32, Ordering};

/// Maximum number of collision pairs per frame
pub const MAX_COLLISION_PAIRS: usize = 16384;

/// Contact point information for a collision
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct ContactPoint {
    pub position: [f32; 3],
    pub penetration_depth: f32,
    pub normal: [f32; 3],
    pub _padding: f32,
}

impl ContactPoint {
    pub fn new(position: [f32; 3], normal: [f32; 3], penetration_depth: f32) -> Self {
        Self {
            position,
            penetration_depth,
            normal,
            _padding: 0.0,
        }
    }
}

/// Collision pair representing two entities in contact
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContactPair {
    pub entity_a: EntityId,
    pub entity_b: EntityId,
}

impl ContactPair {
    pub fn new(a: EntityId, b: EntityId) -> Self {
        // Always store smaller ID first for consistency
        if a < b {
            Self {
                entity_a: a,
                entity_b: b,
            }
        } else {
            Self {
                entity_a: b,
                entity_b: a,
            }
        }
    }

    pub fn contains(&self, entity: EntityId) -> bool {
        self.entity_a == entity || self.entity_b == entity
    }
}

/// Collision data storage using struct-of-arrays
pub struct CollisionData {
    // Number of active collision pairs
    pair_count: AtomicU32,

    // Collision pair data
    pub contact_pairs: Vec<ContactPair>,
    pub contact_points: Vec<ContactPoint>,
    pub contact_counts: Vec<u32>, // Number of contacts per pair

    // Impulse cache for warm starting
    pub normal_impulses: Vec<f32>,
    pub tangent_impulses: Vec<[f32; 2]>,

    // Collision response data
    pub relative_velocities: Vec<[f32; 3]>,
    pub combined_restitutions: Vec<f32>,
    pub combined_frictions: Vec<f32>,

    // Previous frame data for temporal coherence
    pub previous_pairs: Vec<ContactPair>,
    pub previous_impulses: Vec<f32>,
}

impl CollisionData {
    pub fn new(max_pairs: usize) -> Self {
        Self {
            pair_count: AtomicU32::new(0),

            contact_pairs: Vec::with_capacity(max_pairs),
            contact_points: Vec::with_capacity(max_pairs * 4), // Up to 4 contacts per pair
            contact_counts: Vec::with_capacity(max_pairs),

            normal_impulses: Vec::with_capacity(max_pairs * 4),
            tangent_impulses: Vec::with_capacity(max_pairs * 4),

            relative_velocities: Vec::with_capacity(max_pairs),
            combined_restitutions: Vec::with_capacity(max_pairs),
            combined_frictions: Vec::with_capacity(max_pairs),

            previous_pairs: Vec::with_capacity(max_pairs),
            previous_impulses: Vec::with_capacity(max_pairs * 4),
        }
    }

    /// Clear collision data for new frame
    pub fn clear(&mut self) {
        // Save current data as previous for warm starting
        self.previous_pairs.clear();
        self.previous_pairs.extend_from_slice(&self.contact_pairs);

        self.previous_impulses.clear();
        self.previous_impulses
            .extend_from_slice(&self.normal_impulses);

        // Clear current frame data
        self.contact_pairs.clear();
        self.contact_points.clear();
        self.contact_counts.clear();

        self.normal_impulses.clear();
        self.tangent_impulses.clear();

        self.relative_velocities.clear();
        self.combined_restitutions.clear();
        self.combined_frictions.clear();

        self.pair_count.store(0, Ordering::SeqCst);
    }

    /// Add a new collision pair
    pub fn add_collision(
        &mut self,
        entity_a: EntityId,
        entity_b: EntityId,
        contact: ContactPoint,
        restitution: f32,
        friction: f32,
    ) -> usize {
        let pair = ContactPair::new(entity_a, entity_b);

        // Check if pair already exists
        if let Some(idx) = self.contact_pairs.iter().position(|&p| p == pair) {
            // Add contact to existing pair
            self.contact_points.push(contact);
            self.contact_counts[idx] += 1;
            self.normal_impulses.push(0.0);
            self.tangent_impulses.push([0.0, 0.0]);

            // Warm start if we have previous frame data
            if let Some(prev_idx) = self.previous_pairs.iter().position(|&p| p == pair) {
                if prev_idx < self.previous_impulses.len() {
                    if let Some(last) = self.normal_impulses.last_mut() {
                        *last = self.previous_impulses[prev_idx];
                    }
                }
            }

            idx
        } else {
            // New collision pair
            let idx = self.pair_count.fetch_add(1, Ordering::SeqCst) as usize;

            self.contact_pairs.push(pair);
            self.contact_points.push(contact);
            self.contact_counts.push(1);

            self.normal_impulses.push(0.0);
            self.tangent_impulses.push([0.0, 0.0]);

            self.relative_velocities.push([0.0, 0.0, 0.0]);
            self.combined_restitutions.push(restitution);
            self.combined_frictions.push(friction);

            // Warm start if we have previous frame data
            if let Some(prev_idx) = self.previous_pairs.iter().position(|&p| p == pair) {
                if prev_idx < self.previous_impulses.len() {
                    if let Some(last) = self.normal_impulses.last_mut() {
                        *last = self.previous_impulses[prev_idx];
                    }
                }
            }

            idx
        }
    }

    /// Get collision pair count
    pub fn pair_count(&self) -> usize {
        self.pair_count.load(Ordering::SeqCst) as usize
    }

    /// Get contact points for a specific pair
    pub fn get_contacts_for_pair(&self, pair_idx: usize) -> &[ContactPoint] {
        if pair_idx >= self.contact_counts.len() {
            return &[];
        }

        // Calculate start index for this pair's contacts
        let mut start = 0;
        for i in 0..pair_idx {
            start += self.contact_counts[i] as usize;
        }

        let count = self.contact_counts[pair_idx] as usize;
        &self.contact_points[start..start + count]
    }

    /// Batch process collision pairs in parallel
    pub fn prepare_parallel_batches(&self, batch_size: usize) -> Vec<(usize, usize)> {
        let count = self.pair_count();
        let mut batches = Vec::new();

        for start in (0..count).step_by(batch_size) {
            let end = (start + batch_size).min(count);
            batches.push((start, end));
        }

        batches
    }
}

/// Collision statistics for performance monitoring
#[derive(Debug, Default)]
pub struct CollisionStats {
    pub broad_phase_pairs: usize,
    pub narrow_phase_pairs: usize,
    pub contact_points: usize,
    pub cache_hits: usize,
    pub broad_phase_time_us: u64,
    pub narrow_phase_time_us: u64,
    pub solver_time_us: u64,
}

impl CollisionStats {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn total_time_us(&self) -> u64 {
        self.broad_phase_time_us + self.narrow_phase_time_us + self.solver_time_us
    }
}
