//! Memory Module - Simplified for DOP conversion
//!
//! This module will be properly implemented after DOP conversion is complete.

pub mod bandwidth_profiler;
pub mod memory_pool;
pub mod performance_metrics;
pub mod persistent_buffer;
pub mod sync_barrier;

// Simple re-exports matching our stub implementations
pub use bandwidth_profiler::BandwidthProfiler;
pub use memory_pool::MemoryPool;
pub use performance_metrics::PerformanceMetrics;
pub use persistent_buffer::PersistentBuffer;
pub use sync_barrier::SyncBarrier;

// Memory module error (stub)
pub mod error {
    pub type MemoryResult<T> = Result<T, String>;
}

pub use error::MemoryResult;

use std::sync::Arc;

// Buffer allocation wrapper
pub struct ManagedBuffer {
    buffer: Arc<wgpu::Buffer>,
}

impl ManagedBuffer {
    pub fn buffer_arc(self) -> Arc<wgpu::Buffer> {
        self.buffer
    }
}

// MemoryManager stub
pub struct MemoryManager;

impl MemoryManager {
    pub fn alloc_buffer(&mut self, _size: u64, _usage: wgpu::BufferUsages) -> MemoryResult<ManagedBuffer> {
        Err("MemoryManager::alloc_buffer not implemented".to_string())
    }
}

// Implementation and MetricType for world/compute
pub enum Implementation { Cpu, Gpu }
pub enum MetricType { Time, Memory, FrameTime }
