use crate::{BlockId, ChunkPos, VoxelPos};
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
/// Pre-allocated pools to replace runtime Vec::new(), HashMap::new() allocations
use std::sync::Arc;

/// Pre-allocated vector pool for different sizes
pub struct VectorPool<T> {
    small_pool: Arc<RwLock<Vec<Vec<T>>>>,  // Cap 16
    medium_pool: Arc<RwLock<Vec<Vec<T>>>>, // Cap 64
    large_pool: Arc<RwLock<Vec<Vec<T>>>>,  // Cap 256
    huge_pool: Arc<RwLock<Vec<Vec<T>>>>,   // Cap 1024
}

impl<T> VectorPool<T> {
    pub fn new() -> Self {
        let mut small_pool = Vec::with_capacity(32);
        let mut medium_pool = Vec::with_capacity(16);
        let mut large_pool = Vec::with_capacity(8);
        let mut huge_pool = Vec::with_capacity(4);

        // Pre-allocate vectors
        for _ in 0..32 {
            small_pool.push(Vec::with_capacity(16));
        }
        for _ in 0..16 {
            medium_pool.push(Vec::with_capacity(64));
        }
        for _ in 0..8 {
            large_pool.push(Vec::with_capacity(256));
        }
        for _ in 0..4 {
            huge_pool.push(Vec::with_capacity(1024));
        }

        Self {
            small_pool: Arc::new(RwLock::new(small_pool)),
            medium_pool: Arc::new(RwLock::new(medium_pool)),
            large_pool: Arc::new(RwLock::new(large_pool)),
            huge_pool: Arc::new(RwLock::new(huge_pool)),
        }
    }

    pub fn acquire(&self, expected_size: usize) -> PooledVector<T> {
        let (pool, capacity) = match expected_size {
            0..=16 => (Arc::clone(&self.small_pool), 16),
            17..=64 => (Arc::clone(&self.medium_pool), 64),
            65..=256 => (Arc::clone(&self.large_pool), 256),
            _ => (Arc::clone(&self.huge_pool), 1024),
        };

        let vec = match pool.write().pop() {
            Some(mut vec) => {
                vec.clear();
                vec
            }
            None => Vec::with_capacity(capacity),
        };

        PooledVector {
            vec: Some(vec),
            pool,
        }
    }
}

pub struct PooledVector<T> {
    vec: Option<Vec<T>>,
    pool: Arc<RwLock<Vec<Vec<T>>>>,
}

impl<T> std::ops::Deref for PooledVector<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        self.vec
            .as_ref()
            .expect("PooledVector accessed after consumption")
    }
}

impl<T> std::ops::DerefMut for PooledVector<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.vec
            .as_mut()
            .expect("PooledVector accessed after consumption")
    }
}

impl<T> Drop for PooledVector<T> {
    fn drop(&mut self) {
        if let Some(vec) = self.vec.take() {
            // Only return to pool if it hasn't grown too large
            if vec.capacity() <= 1024 {
                self.pool.write().push(vec);
            }
        }
    }
}

/// Pre-allocated HashMap pool
pub struct HashMapPool<K, V> {
    small_pool: Arc<RwLock<Vec<HashMap<K, V>>>>,  // Cap 16
    medium_pool: Arc<RwLock<Vec<HashMap<K, V>>>>, // Cap 64
    large_pool: Arc<RwLock<Vec<HashMap<K, V>>>>,  // Cap 256
}

impl<K, V> HashMapPool<K, V>
where
    K: Eq + std::hash::Hash,
{
    pub fn new() -> Self {
        let mut small_pool = Vec::with_capacity(16);
        let mut medium_pool = Vec::with_capacity(8);
        let mut large_pool = Vec::with_capacity(4);

        // Pre-allocate hashmaps
        for _ in 0..16 {
            small_pool.push(HashMap::with_capacity(16));
        }
        for _ in 0..8 {
            medium_pool.push(HashMap::with_capacity(64));
        }
        for _ in 0..4 {
            large_pool.push(HashMap::with_capacity(256));
        }

        Self {
            small_pool: Arc::new(RwLock::new(small_pool)),
            medium_pool: Arc::new(RwLock::new(medium_pool)),
            large_pool: Arc::new(RwLock::new(large_pool)),
        }
    }

    pub fn acquire(&self, expected_size: usize) -> PooledHashMap<K, V> {
        let (pool, capacity) = match expected_size {
            0..=16 => (Arc::clone(&self.small_pool), 16),
            17..=64 => (Arc::clone(&self.medium_pool), 64),
            _ => (Arc::clone(&self.large_pool), 256),
        };

        let map = match pool.write().pop() {
            Some(mut map) => {
                map.clear();
                map
            }
            None => HashMap::with_capacity(capacity),
        };

        PooledHashMap {
            map: Some(map),
            pool,
        }
    }
}

pub struct PooledHashMap<K, V> {
    map: Option<HashMap<K, V>>,
    pool: Arc<RwLock<Vec<HashMap<K, V>>>>,
}

impl<K, V> std::ops::Deref for PooledHashMap<K, V> {
    type Target = HashMap<K, V>;

    fn deref(&self) -> &Self::Target {
        self.map
            .as_ref()
            .expect("PooledHashMap accessed after consumption")
    }
}

impl<K, V> std::ops::DerefMut for PooledHashMap<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.map
            .as_mut()
            .expect("PooledHashMap accessed after consumption")
    }
}

impl<K, V> Drop for PooledHashMap<K, V> {
    fn drop(&mut self) {
        if let Some(map) = self.map.take() {
            // Only return to pool if it hasn't grown too large
            if map.capacity() <= 512 {
                self.pool.write().push(map);
            }
        }
    }
}

/// Specialized pools for common game data types
pub struct GameDataPools {
    pub chunk_pos_vectors: VectorPool<ChunkPos>,
    pub voxel_pos_vectors: VectorPool<VoxelPos>,
    pub block_id_vectors: VectorPool<BlockId>,
    pub chunk_pos_maps: HashMapPool<ChunkPos, u32>,
    pub temp_string_pool: VectorPool<String>,
}

impl GameDataPools {
    pub fn new() -> Self {
        Self {
            chunk_pos_vectors: VectorPool::new(),
            voxel_pos_vectors: VectorPool::new(),
            block_id_vectors: VectorPool::new(),
            chunk_pos_maps: HashMapPool::new(),
            temp_string_pool: VectorPool::new(),
        }
    }
}

// Global pool instances
lazy_static::lazy_static! {
    pub static ref GAME_POOLS: GameDataPools = GameDataPools::new();
}

/// Convenience macro for getting a pooled vector
#[macro_export]
macro_rules! pooled_vec {
    ($type:ty, $size:expr) => {
        $crate::renderer::zero_alloc_pools::GAME_POOLS
            .chunk_pos_vectors
            .acquire($size)
    };
}

/// Convenience macro for getting a pooled hashmap
#[macro_export]
macro_rules! pooled_map {
    ($key:ty, $value:ty, $size:expr) => {
        $crate::renderer::zero_alloc_pools::GAME_POOLS
            .chunk_pos_maps
            .acquire($size)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_pool() {
        let pool = VectorPool::<i32>::new();

        {
            let mut vec1 = pool.acquire(10);
            vec1.push(1);
            vec1.push(2);
            assert_eq!(vec1.len(), 2);
        } // vec1 returned to pool here

        {
            let vec2 = pool.acquire(10);
            // Should get a cleared vector from pool
            assert_eq!(vec2.len(), 0);
            assert!(vec2.capacity() >= 10);
        }
    }

    #[test]
    fn test_hashmap_pool() {
        let pool = HashMapPool::<i32, String>::new();

        {
            let mut map1 = pool.acquire(10);
            map1.insert(1, "test".to_string());
            assert_eq!(map1.len(), 1);
        } // map1 returned to pool here

        {
            let map2 = pool.acquire(10);
            // Should get a cleared map from pool
            assert_eq!(map2.len(), 0);
        }
    }
}
