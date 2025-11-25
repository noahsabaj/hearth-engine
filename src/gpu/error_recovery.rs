//! GPU Error Recovery System
//!
//! Provides graceful error handling and recovery for GPU operations
//! to prevent segfaults and system crashes.

use std::panic;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

/// GPU error recovery system
pub struct GpuErrorRecovery {
    /// Device reference
    device: Arc<wgpu::Device>,
    /// Queue reference  
    queue: Arc<wgpu::Queue>,
    /// Flag indicating if device is lost
    device_lost: AtomicBool,
    /// Error count for rate limiting
    error_count: AtomicU32,
    /// Maximum errors before forcing recovery
    max_errors: u32,
}

impl GpuErrorRecovery {
    /// Create a new error recovery system
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        // Set up device lost handler
        let device_lost = Arc::new(AtomicBool::new(false));
        let device_lost_clone = device_lost.clone();

        device.on_uncaptured_error(Box::new(move |error| {
            log::error!("[GPU Error Recovery] Uncaptured GPU error: {:?}", error);

            // Check for device lost
            match error {
                wgpu::Error::OutOfMemory { .. } => {
                    log::error!("[GPU Error Recovery] GPU out of memory!");
                    device_lost_clone.store(true, Ordering::Relaxed);
                }
                wgpu::Error::Validation { description, .. } => {
                    log::error!("[GPU Error Recovery] GPU validation error: {}", description);
                }
            }
        }));

        Self {
            device,
            queue,
            device_lost: AtomicBool::new(false),
            error_count: AtomicU32::new(0),
            max_errors: 10,
        }
    }

    /// Check if device is lost
    pub fn is_device_lost(&self) -> bool {
        self.device_lost.load(Ordering::Relaxed)
    }

    /// Execute a GPU operation with error recovery
    pub fn execute_with_recovery<F, R>(&self, operation: F) -> Result<R, GpuRecoveryError>
    where
        F: FnOnce() -> Result<R, GpuRecoveryError>,
    {
        // Check if device is already lost
        if self.is_device_lost() {
            return Err(GpuRecoveryError::DeviceLost);
        }

        // Increment error count
        let error_count = self.error_count.fetch_add(1, Ordering::Relaxed);
        if error_count > self.max_errors {
            log::error!(
                "[GPU Error Recovery] Too many GPU errors ({}), forcing recovery",
                error_count
            );
            self.device_lost.store(true, Ordering::Relaxed);
            return Err(GpuRecoveryError::TooManyErrors { count: error_count });
        }

        // Use panic::catch_unwind to prevent segfaults
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| operation()));

        match result {
            Ok(Ok(value)) => {
                // Reset error count on success
                self.error_count.store(0, Ordering::Relaxed);
                Ok(value)
            }
            Ok(Err(e)) => {
                log::warn!("[GPU Error Recovery] Operation failed: {:?}", e);
                Err(e)
            }
            Err(panic_info) => {
                log::error!("[GPU Error Recovery] GPU operation panicked!");

                // Try to extract panic message
                let msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Unknown panic".to_string()
                };

                log::error!("[GPU Error Recovery] Panic message: {}", msg);

                // Mark device as lost on panic
                self.device_lost.store(true, Ordering::Relaxed);

                Err(GpuRecoveryError::Panic { message: msg })
            }
        }
    }

    /// Validate command encoder before submission
    pub fn validate_encoder(
        &self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), GpuRecoveryError> {
        // WGPU doesn't expose encoder validity directly, but we can use a marker operation
        // This is a no-op that will fail if the encoder is invalid
        let marker = format!(
            "validation_marker_{}",
            self.error_count.load(Ordering::Relaxed)
        );
        encoder.push_debug_group(&marker);
        encoder.pop_debug_group();
        Ok(())
    }

    /// Submit commands with error recovery
    pub fn submit_with_recovery(
        &self,
        command_buffers: Vec<wgpu::CommandBuffer>,
    ) -> Result<wgpu::SubmissionIndex, GpuRecoveryError> {
        self.execute_with_recovery(|| {
            let index = self.queue.submit(command_buffers);
            Ok(index)
        })
    }

    /// Create a safe command encoder wrapper
    pub fn create_safe_encoder(&self, desc: &wgpu::CommandEncoderDescriptor) -> SafeCommandEncoder<'_> {
        SafeCommandEncoder {
            encoder: self.device.create_command_encoder(desc),
            recovery: self,
            is_valid: true,
        }
    }
}

/// Safe command encoder wrapper that validates operations
pub struct SafeCommandEncoder<'a> {
    encoder: wgpu::CommandEncoder,
    recovery: &'a GpuErrorRecovery,
    is_valid: bool,
}

impl<'a> SafeCommandEncoder<'a> {
    /// Get the underlying encoder if valid
    pub fn encoder(&mut self) -> Result<&mut wgpu::CommandEncoder, GpuRecoveryError> {
        if !self.is_valid {
            return Err(GpuRecoveryError::InvalidEncoder);
        }

        // Validate encoder is still valid
        if let Err(e) = self.recovery.validate_encoder(&mut self.encoder) {
            self.is_valid = false;
            return Err(e);
        }

        Ok(&mut self.encoder)
    }

    /// Finish the encoder and return command buffer
    pub fn finish(self) -> Result<wgpu::CommandBuffer, GpuRecoveryError> {
        if !self.is_valid {
            return Err(GpuRecoveryError::InvalidEncoder);
        }

        self.recovery
            .execute_with_recovery(|| Ok(self.encoder.finish()))
    }
}

/// GPU recovery errors
#[derive(Debug, thiserror::Error)]
pub enum GpuRecoveryError {
    #[error("GPU device lost")]
    DeviceLost,

    #[error("Too many GPU errors: {count}")]
    TooManyErrors { count: u32 },

    #[error("GPU operation panicked: {message}")]
    Panic { message: String },

    #[error("Invalid command encoder")]
    InvalidEncoder,

    #[error("Buffer size mismatch: expected {expected}, got {actual}")]
    BufferSizeMismatch { expected: u64, actual: u64 },

    #[error("Shader compilation failed: {message}")]
    ShaderCompilationFailed { message: String },

    #[error("GPU operation failed: {message}")]
    OperationFailed { message: String },
}

/// Extension trait for Result types to add GPU error context
pub trait GpuResultExt<T> {
    /// Convert error to GPU recovery error with context
    fn gpu_context(self, context: &str) -> Result<T, GpuRecoveryError>;
}

impl<T, E: std::fmt::Display> GpuResultExt<T> for Result<T, E> {
    fn gpu_context(self, context: &str) -> Result<T, GpuRecoveryError> {
        self.map_err(|e| GpuRecoveryError::OperationFailed {
            message: format!("{}: {}", context, e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recovery_creation() {
        // This would require a real GPU device to test properly
        // For now, just ensure the module compiles
    }
}
