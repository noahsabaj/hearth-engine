/// Process Executor
///
/// Executes process updates and manages state transitions.
/// Handles resource consumption and output generation.
use crate::instance::InstanceId;
use crate::process::{
    ActualOutput, ProcessData, ProcessId, ProcessStatus, ProcessType, StageValidator, StateMachine,
    TransformStage, TransitionAction, ValidationContext,
};
use std::collections::HashMap;

/// Result of process execution
#[derive(Debug)]
pub struct ExecutionResult {
    /// Processes that completed
    pub completed: Vec<ProcessId>,

    /// Processes that failed
    pub failed: Vec<(ProcessId, String)>,

    /// Resources consumed
    pub consumed: Vec<(InstanceId, u32)>,

    /// Resources produced
    pub produced: Vec<(ProcessType, ActualOutput)>,

    /// Events triggered
    pub events: Vec<String>,
}

impl ExecutionResult {
    fn new() -> Self {
        Self {
            completed: Vec::new(),
            failed: Vec::new(),
            consumed: Vec::new(),
            produced: Vec::new(),
            events: Vec::new(),
        }
    }
}

/// Process executor
pub struct ProcessExecutor {
    /// Validation contexts per player
    contexts: HashMap<InstanceId, ValidationContext>,

    /// Random number generator
    rng: rand::rngs::StdRng,

    /// Resource manager reference (would be injected)
    resource_available: HashMap<u32, u32>,
}

impl ProcessExecutor {
    pub fn new() -> Self {
        use rand::SeedableRng;

        Self {
            contexts: HashMap::new(),
            rng: rand::rngs::StdRng::from_entropy(),
            resource_available: HashMap::new(),
        }
    }

    /// Execute a single process update
    pub fn execute_process(
        &mut self,
        index: usize,
        data: &mut ProcessData,
        state_machine: &mut StateMachine,
        stages: &[TransformStage],
        delta_ticks: u64,
    ) -> Option<ExecutionResult> {
        if !data.active[index] {
            return None;
        }

        let mut result = ExecutionResult::new();

        match data.status[index] {
            ProcessStatus::Pending => {
                // Start the process
                data.status[index] = ProcessStatus::Active;
                state_machine.force_transition(crate::process::ProcessState::PREPARING);
            }

            ProcessStatus::Active => {
                // Update progress
                data.update(index, delta_ticks);
                let progress = data.get_progress(index);

                // Update state machine
                let actions = state_machine.update(delta_ticks, progress);

                // Process transition actions
                for action in actions {
                    self.process_action(action, index, data, &mut result);
                }

                // Check stage completion
                if let Some(current_stage) = self.get_current_stage(state_machine, stages) {
                    if self.is_stage_complete(state_machine, current_stage) {
                        // Generate stage outputs
                        let outputs = StageValidator::calculate_outputs(
                            current_stage,
                            data.quality[index] as u8 as f32 / 4.0, // Convert QualityLevel to f32 (0.0-1.0)
                            &mut self.rng,
                        );

                        for output in outputs {
                            result.produced.push((data.types[index], output));
                        }
                    }
                }

                // Check overall completion
                if data.status[index] == ProcessStatus::Completed {
                    result.completed.push(data.ids[index]);
                    data.active[index] = false;
                }
            }

            ProcessStatus::Failed => {
                result
                    .failed
                    .push((data.ids[index], "Process failed".to_string()));
                data.active[index] = false;
            }

            _ => {}
        }

        Some(result)
    }

    /// Execute batch of processes
    pub fn execute_batch(
        &mut self,
        indices: &[usize],
        data: &mut ProcessData,
        state_machines: &mut [StateMachine],
        all_stages: &[Vec<TransformStage>],
        delta_ticks: u64,
    ) -> ExecutionResult {
        let mut combined_result = ExecutionResult::new();

        for &index in indices {
            if let Some(result) = self.execute_process(
                index,
                data,
                &mut state_machines[index],
                &all_stages[index],
                delta_ticks,
            ) {
                // Combine results
                combined_result.completed.extend(result.completed);
                combined_result.failed.extend(result.failed);
                combined_result.consumed.extend(result.consumed);
                combined_result.produced.extend(result.produced);
                combined_result.events.extend(result.events);
            }
        }

        combined_result
    }

    /// Process a transition action
    fn process_action(
        &mut self,
        action: TransitionAction,
        index: usize,
        data: &mut ProcessData,
        result: &mut ExecutionResult,
    ) {
        match action {
            TransitionAction::ConsumeResources(resources) => {
                for (resource_id, amount) in resources {
                    // In real implementation, would consume from inventory
                    self.resource_available
                        .entry(resource_id)
                        .and_modify(|v| *v = v.saturating_sub(amount))
                        .or_insert(0);

                    result.consumed.push((InstanceId::nil(), amount));
                }
            }

            TransitionAction::ProduceResources(resources) => {
                for (resource_id, amount) in resources {
                    // In real implementation, would add to inventory
                    *self.resource_available.entry(resource_id).or_insert(0) += amount;

                    result.produced.push((
                        data.types[index],
                        ActualOutput {
                            output_type: crate::process::OutputType::Item(resource_id),
                            quantity: amount,
                            quality: data.quality[index] as u8 as f32 / 4.0, // Convert QualityLevel to f32
                        },
                    ));
                }
            }

            TransitionAction::ApplyQuality(modifier) => {
                let current = data.quality[index] as i8;
                let new_quality = (current + modifier).clamp(0, 4);
                // SAFETY: Transmuting u8 to QualityLevel enum is safe because:
                // - QualityLevel is repr(u8) with values 0-4
                // - new_quality is clamped to 0-4 range, matching valid enum variants
                // - The enum has explicit discriminants for all values in this range
                // - Misuse would only occur if QualityLevel enum definition changes
                data.quality[index] = unsafe { std::mem::transmute(new_quality as u8) };
            }

            TransitionAction::TriggerEvent(event) => {
                result.events.push(event);
            }

            TransitionAction::LogMessage(msg) => {
                // In real implementation, would log
                println!("Process {}: {}", data.ids[index].0, msg);
            }
        }
    }

    /// Get current stage based on state
    fn get_current_stage<'a>(
        &self,
        state_machine: &StateMachine,
        stages: &'a [TransformStage],
    ) -> Option<&'a TransformStage> {
        let state_value = state_machine.current_state().0;

        // Map state to stage index (simplified)
        if state_value >= 10 && state_value < 10 + stages.len() as u16 {
            let stage_index = (state_value - 10) as usize;
            stages.get(stage_index)
        } else {
            None
        }
    }

    /// Check if current stage is complete
    fn is_stage_complete(&self, state_machine: &StateMachine, stage: &TransformStage) -> bool {
        // Duration is in seconds (f32), convert to ticks (assuming 20 ticks/second)
        let stage_duration_ticks = (stage.duration * 20.0) as u64;
        state_machine.state_time() >= stage_duration_ticks
    }

    /// Update validation context for a player
    pub fn update_context(&mut self, player: InstanceId, context: ValidationContext) {
        self.contexts.insert(player, context);
    }

    /// Get validation context for a player
    pub fn get_context(&self, player: &InstanceId) -> Option<&ValidationContext> {
        self.contexts.get(player)
    }
}

/// Process scheduler for prioritized execution
pub struct ProcessScheduler {
    /// Priority queues
    queues: [Vec<usize>; 4], // One per priority level
}

impl ProcessScheduler {
    pub fn new() -> Self {
        Self {
            queues: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
        }
    }

    /// Schedule processes for execution
    pub fn schedule(&mut self, data: &ProcessData) -> Vec<usize> {
        // Clear queues
        for queue in &mut self.queues {
            queue.clear();
        }

        // Sort processes by priority
        for i in 0..data.len() {
            let (is_active, status, priority) =
                match (data.active.get(i), data.status.get(i), data.priority.get(i)) {
                    (Some(&active), Some(&status), Some(&priority)) => (active, status, priority),
                    _ => continue,
                };

            if is_active && status == ProcessStatus::Active {
                let priority_idx = priority as usize;
                if let Some(queue) = self.queues.get_mut(priority_idx) {
                    queue.push(i);
                }
            }
        }

        // Build execution order (highest priority first)
        let mut order = Vec::new();
        for queue in self.queues.iter().rev() {
            order.extend(queue);
        }

        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::ProcessState;

    #[test]
    fn test_process_execution() {
        let mut executor = ProcessExecutor::new();
        let mut data = ProcessData::new();
        let mut state_machine = StateMachine::new();

        let id = ProcessId::new();
        let owner = InstanceId::new();
        let index = data.add(id, ProcessType::default(), owner, 100);

        // Execute pending -> active
        let result = executor.execute_process(index, &mut data, &mut state_machine, &[], 10);

        assert!(result.is_some());
        assert_eq!(data.status[index], ProcessStatus::Active);
        assert_eq!(state_machine.current_state(), ProcessState::PREPARING);
    }

    #[test]
    fn test_scheduler() {
        let mut scheduler = ProcessScheduler::new();
        let mut data = ProcessData::new();

        // Add processes with different priorities
        for i in 0..4 {
            let id = ProcessId::new();
            let owner = InstanceId::new();
            let index = data.add(id, ProcessType::default(), owner, 100);
            data.status[index] = ProcessStatus::Active;
            // SAFETY: Transmuting u8 to priority enum is safe because:
            // - Priority enum is repr(u8) with valid values 0-3
            // - Loop index i is in range 0..4, so i as u8 produces 0-3
            // - These values directly map to valid Priority enum variants
            // - The test controls the input values, ensuring they're always valid
            data.priority[index] = unsafe { std::mem::transmute(i as u8) };
        }

        let order = scheduler.schedule(&data);

        // Should be ordered by priority (highest first)
        assert_eq!(order, vec![3, 2, 1, 0]);
    }
}
