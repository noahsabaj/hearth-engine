//! Performance Metrics - Stub Implementation

pub struct PerformanceMetrics;

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self
    }

    pub fn start_measurement(&self, _name: &str) {
        // Stub implementation
    }

    pub fn get_comparisons(&self) -> Vec<String> {
        // Stub implementation
        Vec::new()
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}
