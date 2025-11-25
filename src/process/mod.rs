pub mod error;
// pub mod parallel_processor; // Removed - using DOP modules instead
pub mod parallel_processor_data;
pub mod parallel_processor_operations;
pub mod process_control;
/// Process & Transform System
///
/// Generic time-based transformation framework for any gameplay system.
/// Can handle crafting, building, growth, training, or any multi-stage process.
/// Purely data-oriented - no process "objects", just tables of data.

pub mod process_data;
pub mod process_executor;
pub mod stage_validator;
pub mod state_machine;
pub mod system_coordinator;
pub mod transform_stage_data;
pub mod transform_stage_operations;
pub mod visual_indicators_data;
pub mod visual_indicators_operations;

pub use parallel_processor_data::ParallelProcessorData;
pub use parallel_processor_data::ProcessBatch;
pub use parallel_processor_operations::{create_parallel_processor_data, submit_process_batch_to_gpu};
pub use process_control::{InterruptReason, ProcessControl};
pub use process_data::{ProcessData, ProcessId, ProcessStatus, ProcessType};
pub use process_executor::{ExecutionResult, ProcessExecutor};
pub use stage_validator::StageValidator;
pub use state_machine::{ProcessState, StateMachine, StateTransition, TransitionAction};
pub use transform_stage_data::{
    ActualOutput, OutputType, StageOutput, StageRequirement, TransformStage,
    ValidationContext, ValidationResult, ItemRequirement, ToolRequirement,
    EnvironmentRequirement, WeatherType,
};
pub use transform_stage_operations::{
    validate_requirements, calculate_outputs, create_crafting_stage, create_smelting_stage,
};
pub use visual_indicators_data::{
    ProcessVisual, ProgressBar, StatusIcon, ProgressColor, BarAnimation,
    TextOverlay, TextPosition, TextStyle, ParticleEffect, ParticleType, AnimationState,
};
pub use visual_indicators_operations::{
    update_progress, update_status, calculate_segments, add_text, add_particle,
    update_visual, create_crafting_visual, create_smelting_visual, create_growth_visual,
    quality_to_visual, generate_progress_bar_vertices,
};

use crate::instance::InstanceId;
use serde::{Deserialize, Serialize};

/// Maximum concurrent processes
pub const MAX_PROCESSES: usize = 1 << 16; // 65k

/// Process types for categorization
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProcessCategory {
    Crafting = 0,
    Building = 1,
    Growth = 2,
    Training = 3,
    Research = 4,
    Repair = 5,
    Upgrade = 6,
    Custom = 255,
}

/// Time units for processes
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TimeUnit {
    Ticks(u64),    // Game ticks
    Seconds(f32),  // Real-time seconds
    Minutes(f32),  // Real-time minutes
    Hours(f32),    // Real-time hours
    GameDays(f32), // In-game days
}

impl TimeUnit {
    /// Convert to game ticks (assuming 20 ticks/second)
    pub fn to_ticks(&self) -> u64 {
        match self {
            TimeUnit::Ticks(t) => *t,
            TimeUnit::Seconds(s) => (*s * 20.0) as u64,
            TimeUnit::Minutes(m) => (*m * 20.0 * 60.0) as u64,
            TimeUnit::Hours(h) => (*h * 20.0 * 60.0 * 60.0) as u64,
            TimeUnit::GameDays(d) => (*d * 24000.0) as u64, // MC day = 24000 ticks
        }
    }
}

/// Process priority for scheduling
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProcessPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Quality levels for process outputs
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum QualityLevel {
    Poor = 0,
    Normal = 1,
    Good = 2,
    Excellent = 3,
    Perfect = 4,
}

/// Core process manager (Structure of Arrays)
pub struct ProcessManager {
    /// Process data tables
    pub processes: ProcessData,

    /// State machines for each process
    pub state_machines: Vec<StateMachine>,

    /// Transform stages for complex processes
    pub transform_stages: Vec<Vec<TransformStage>>,

    /// Visual indicators
    pub visuals: Vec<ProcessVisual>,

    /// Process executor
    pub executor: ProcessExecutor,

    /// Parallel processor data for batch updates
    pub parallel_data: ParallelProcessorData,
    
    /// GPU thread pool for command submission
    pub gpu_thread_pool: crate::thread_pool::GpuThreadPoolData,

    /// Control system for interrupts
    pub control: ProcessControl,
}

impl ProcessManager {
    pub fn new() -> Result<Self, crate::error::EngineError> {
        Ok(Self {
            processes: ProcessData::new(),
            state_machines: Vec::with_capacity(MAX_PROCESSES),
            transform_stages: Vec::with_capacity(MAX_PROCESSES),
            visuals: Vec::with_capacity(MAX_PROCESSES),
            executor: ProcessExecutor::new(),
            parallel_data: create_parallel_processor_data()
                .map_err(|e| crate::error::EngineError::InitializationError(e))?,
            gpu_thread_pool: crate::thread_pool::create_gpu_thread_pool_data(
                crate::thread_pool::GpuThreadPoolConfig::default()
            ).map_err(|e| crate::error::EngineError::InitializationError(e))?,
            control: ProcessControl::new(),
        })
    }

    /// Start a new process
    pub fn start_process(
        &mut self,
        process_type: ProcessType,
        owner: InstanceId,
        inputs: Vec<InstanceId>,
        duration: TimeUnit,
    ) -> ProcessId {
        let id = ProcessId::new();
        let index = self
            .processes
            .add(id, process_type, owner, duration.to_ticks());

        // Initialize state machine
        self.state_machines.push(StateMachine::new());

        // Initialize transform stages (empty for now)
        self.transform_stages.push(Vec::new());

        // Initialize visual
        self.visuals.push(ProcessVisual::default());

        id
    }

    /// Update all processes (called each tick)
    pub fn update(&mut self, delta_ticks: u64) {
        // Use parallel processor for batch updates
        let batch = ProcessBatch {
            indices: (0..self.processes.len()).collect(),
            delta_ticks: vec![delta_ticks as f32], // Convert u64 to Vec<f32>
        };

        submit_process_batch_to_gpu(
            &mut self.parallel_data,
            &self.gpu_thread_pool,
            &mut self.processes,
            &mut self.state_machines,
            batch,
        );

        // Update visuals based on progress
        for i in 0..self.processes.len() {
            if self.processes.active[i] {
                let progress = self.processes.get_progress(i);
                update_progress(&mut self.visuals[i], progress);
            }
        }
    }

    /// Get process info
    pub fn get_process(&self, id: ProcessId) -> Option<ProcessInfo> {
        let index = self.processes.find_index(id)?;

        Some(ProcessInfo {
            id,
            process_type: self.processes.types[index],
            owner: self.processes.owners[index],
            status: self.processes.status[index],
            progress: self.processes.get_progress(index),
            time_remaining: self.processes.get_time_remaining(index),
            current_state: self.state_machines[index].current_state(),
        })
    }
}

/// Process information struct
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub id: ProcessId,
    pub process_type: ProcessType,
    pub owner: InstanceId,
    pub status: ProcessStatus,
    pub progress: f32,
    pub time_remaining: u64,
    pub current_state: ProcessState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_conversion() {
        assert_eq!(TimeUnit::Seconds(1.0).to_ticks(), 20);
        assert_eq!(TimeUnit::Minutes(1.0).to_ticks(), 1200);
        assert_eq!(TimeUnit::GameDays(1.0).to_ticks(), 24000);
    }

    #[test]
    fn test_process_creation() {
        let mut manager = ProcessManager::new().expect("Failed to create manager");
        let owner = InstanceId::new();

        let process_id = manager.start_process(
            ProcessType::default(),
            owner,
            vec![],
            TimeUnit::Seconds(5.0),
        );

        let info = manager
            .get_process(process_id)
            .expect("Process should exist in test");
        assert_eq!(info.owner, owner);
        assert_eq!(info.time_remaining, 100); // 5 seconds * 20 ticks
    }
}
