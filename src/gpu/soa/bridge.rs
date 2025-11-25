//! CPU-GPU bridge for SOA data transformation
//!
//! Provides utilities for converting between CPU-friendly Array of Structures (AOS)
//! and GPU-optimized Structure of Arrays (SOA) representations.

use crate::gpu::soa::types::{BlockDistributionSOA, SoaCompatible, TerrainParamsSOA};
use crate::gpu::types::terrain::BlockDistribution;
use crate::gpu::types::terrain::TerrainParams;

/// Bridge between CPU (AOS) and GPU (SOA) data representations
pub struct CpuGpuBridge;

impl CpuGpuBridge {
    /// Pack CPU-friendly AOS data into GPU-optimized SOA layout
    pub fn pack_for_gpu<T: SoaCompatible>(cpu_data: &[T]) -> T::Arrays {
        T::to_soa(cpu_data)
    }

    /// Unpack GPU SOA data for CPU processing
    pub fn unpack_from_gpu<T: SoaCompatible>(gpu_data: &T::Arrays, index: usize) -> T {
        T::from_soa(gpu_data, index)
    }

    /// Update a single item in GPU SOA data
    pub fn update_in_gpu<T: SoaCompatible>(gpu_data: &mut T::Arrays, index: usize, cpu_item: &T) {
        T::update_soa(gpu_data, index, cpu_item);
    }

    /// Extract all items from GPU SOA data
    pub fn unpack_all<T: SoaCompatible>(gpu_data: &T::Arrays) -> Vec<T> {
        let count = T::soa_count(gpu_data);
        (0..count).map(|i| T::from_soa(gpu_data, i)).collect()
    }

    /// Pack terrain parameters for GPU
    pub fn pack_terrain_params(params: &TerrainParams) -> TerrainParamsSOA {
        TerrainParamsSOA::from_aos(params)
    }

    /// Unpack terrain parameters from GPU
    pub fn unpack_terrain_params(soa_params: &TerrainParamsSOA) -> TerrainParams {
        soa_params.to_aos()
    }
}

/// Helper trait for ergonomic SOA operations
pub trait SoaOps<T: SoaCompatible> {
    /// Convert to SOA representation
    fn to_soa(&self) -> T::Arrays;
}

impl<T: SoaCompatible> SoaOps<T> for Vec<T> {
    fn to_soa(&self) -> T::Arrays {
        CpuGpuBridge::pack_for_gpu(self)
    }
}

impl<T: SoaCompatible> SoaOps<T> for [T] {
    fn to_soa(&self) -> T::Arrays {
        CpuGpuBridge::pack_for_gpu(self)
    }
}

/// Iterator for accessing SOA data
pub struct SoaIterator<'a, T: SoaCompatible> {
    data: &'a T::Arrays,
    index: usize,
    count: usize,
}

impl<'a, T: SoaCompatible> SoaIterator<'a, T> {
    /// Create a new SOA iterator
    pub fn new(data: &'a T::Arrays) -> Self {
        Self {
            data,
            index: 0,
            count: T::soa_count(data),
        }
    }
}

impl<'a, T: SoaCompatible> Iterator for SoaIterator<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.count {
            let item = T::from_soa(self.data, self.index);
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a, T: SoaCompatible> ExactSizeIterator for SoaIterator<'a, T> {}

/// Extension trait for SOA data iteration
pub trait SoaIterExt<T: SoaCompatible> {
    /// Iterate over items in SOA data
    fn iter_soa(&self) -> SoaIterator<'_, T>;
}

impl<T: SoaCompatible> SoaIterExt<T> for T::Arrays {
    fn iter_soa(&self) -> SoaIterator<'_, T> {
        SoaIterator::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_gpu_bridge() {
        // Create test AOS data
        let distributions = vec![
            BlockDistribution {
                block_id: 1,
                min_height: 0,
                max_height: 10,
                probability: 0.5,
                noise_threshold: 0.3,
                _padding: [0; 3],
            },
            BlockDistribution {
                block_id: 2,
                min_height: 10,
                max_height: 20,
                probability: 0.3,
                noise_threshold: 0.5,
                _padding: [0; 3],
            },
        ];

        // Pack to SOA
        let soa_data = CpuGpuBridge::pack_for_gpu(&distributions);
        assert_eq!(soa_data.count, 2);

        // Unpack single item
        let item: BlockDistribution = CpuGpuBridge::unpack_from_gpu(&soa_data, 0);
        assert_eq!(item.block_id, 1);
        assert_eq!(item.min_height, 0);

        // Unpack all items
        let unpacked: Vec<BlockDistribution> = CpuGpuBridge::unpack_all(&soa_data);
        assert_eq!(unpacked.len(), 2);
        assert_eq!(unpacked[1].block_id, 2);
    }

    #[test]
    fn test_soa_iterator() {
        let distributions = vec![
            BlockDistribution {
                block_id: 1,
                min_height: 0,
                max_height: 10,
                probability: 0.5,
                noise_threshold: 0.3,
                _padding: [0; 3],
            },
            BlockDistribution {
                block_id: 2,
                min_height: 10,
                max_height: 20,
                probability: 0.3,
                noise_threshold: 0.5,
                _padding: [0; 3],
            },
        ];

        let soa_data = distributions.to_soa();
        let collected: Vec<BlockDistribution> = soa_data.iter_soa().collect();

        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0].block_id, 1);
        assert_eq!(collected[1].block_id, 2);
    }
}
