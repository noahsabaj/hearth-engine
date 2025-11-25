//! Thread Pool Data - Stub
pub struct ThreadPoolData;
pub enum GpuWorkloadCategory { Rendering, Physics, Compute }
pub struct GpuThreadPoolData;
pub struct GpuThreadPoolConfig;

pub struct ThreadPoolManager;

impl ThreadPoolManager {
    pub fn global() -> &'static Self {
        static INSTANCE: ThreadPoolManager = ThreadPoolManager;
        &INSTANCE
    }

    pub fn execute<F: FnOnce() + Send + 'static>(&self, _category: GpuWorkloadCategory, _f: F) {
        // Stub - would execute on thread pool
    }
}

impl Default for GpuThreadPoolConfig {
    fn default() -> Self {
        Self
    }
}
