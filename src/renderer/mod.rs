//! Renderer Module - Simplified for DOP conversion

pub mod compute_pipeline;
pub mod error;
pub mod gpu_culling;
pub mod gpu_driven;
pub mod gpu_meshing;
pub mod gpu_progress;
pub mod gpu_state_data;
pub mod gpu_state_operations;
pub mod mesh_optimizer;
pub mod mesh_utils;
pub mod renderer_data;
pub mod renderer_operations;
pub mod selection_renderer;
pub mod vertex;

// Simple re-exports
pub use compute_pipeline::ComputePipeline;
pub use mesh_optimizer::MeshOptimizer;
pub use mesh_utils::MeshUtils;
pub use renderer_data::{RendererData, Renderer};
pub use renderer_operations::run_with_buffers;
pub use selection_renderer::SelectionRenderer;
