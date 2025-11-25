/// System Coordinator
///
/// Coordinates execution of different engine systems with proper ordering,
/// resource management, and error handling.
///
/// This addresses the system integration bottlenecks by:
/// 1. Orchestrating system update order based on dependencies
/// 2. Managing resource allocation across systems  
/// 3. Monitoring system health and performance
/// 4. Providing loose coupling through events
/// 5. Handling cross-system synchronization
use crate::error::{EngineError, EngineResult};
use crate::thread_pool::{GpuWorkloadCategory, GpuThreadPoolData, ThreadPoolManager, submit_gpu_command_task};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

/// System identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SystemId {
    WorldGeneration,
    Physics,
    Renderer,
    Lighting,
    Network,
    Persistence,
    Audio,
    Input,
    UI,
    Particles,
    Weather,
}

/// System execution priority
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemPriority {
    Critical = 0, // Must run every frame
    High = 1,     // Should run most frames
    Normal = 2,   // Can skip frames if needed
    Low = 3,      // Can run at reduced frequency
}

/// Thread pool category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolCategory {
    WorldGeneration,
    Physics,
    MeshBuilding,
    Lighting,
    Network,
    FileIO,
    Compute,
}

/// System state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemState {
    Stopped,
    Starting,
    Running,
    Paused,
    Stopping,
    Error,
}

/// System health status
#[derive(Debug, Clone)]
pub struct SystemHealth {
    pub system_id: SystemId,
    pub state: SystemState,
    pub last_update: Instant,
    pub average_frame_time_ms: f64,
    pub error_count: u32,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub is_healthy: bool,
}

/// System dependencies specification
#[derive(Debug, Clone)]
pub struct SystemDependencies {
    /// Systems that must complete before this system runs
    pub depends_on: Vec<SystemId>,
    /// Systems that cannot run concurrently with this system
    pub conflicts_with: Vec<SystemId>,
    /// Maximum time to wait for dependencies (ms)
    pub max_wait_time_ms: u64,
}

/// System execution context
pub struct SystemExecutionContext {
    pub frame_budget_ms: f64,
    pub elapsed_time_ms: f64,
    pub current_frame: u64,
    pub target_fps: f32,
    pub systems_completed: HashSet<SystemId>,
    pub systems_in_progress: HashSet<SystemId>,
}

/// System coordinator manages system execution order and health
pub struct SystemCoordinator {
    /// System health monitoring
    health_monitor: Arc<RwLock<HashMap<SystemId, SystemHealth>>>,

    /// System dependencies
    dependencies: HashMap<SystemId, SystemDependencies>,

    /// System execution order (topologically sorted)
    execution_order: Vec<SystemId>,

    /// System execution times for scheduling
    execution_times: RwLock<HashMap<SystemId, VecDeque<Duration>>>,

    /// Frame timing budget manager
    frame_budget: FrameBudgetManager,

    /// System synchronization barriers
    sync_barriers: Mutex<HashMap<SystemId, Arc<std::sync::Barrier>>>,

    /// Error recovery strategies
    recovery_strategies: HashMap<SystemId, RecoveryStrategy>,

    /// Performance metrics
    metrics: Arc<RwLock<SystemMetrics>>,

    /// Event system for loose coupling
    event_bus: Arc<SystemEventBus>,

    /// Current frame information
    current_frame: Arc<RwLock<SystemExecutionContext>>,
}

/// Frame budget manager
#[derive(Debug)]
pub struct FrameBudgetManager {
    target_frame_time_ms: f64,
    system_budgets: HashMap<SystemId, f64>,
    budget_usage: RwLock<HashMap<SystemId, f64>>,
    adaptive_scaling: bool,
}

/// Recovery strategy for system errors
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    Restart,
    Skip,
    FallbackMode,
    Shutdown,
}

/// System performance metrics
#[derive(Debug, Default, Clone)]
pub struct SystemMetrics {
    pub frame_count: u64,
    pub total_frame_time_ms: f64,
    pub system_times: HashMap<SystemId, f64>,
    pub sync_wait_times: HashMap<SystemId, f64>,
    pub error_counts: HashMap<SystemId, u32>,
    pub memory_usage: HashMap<SystemId, f64>,
}

/// Event system for loose coupling between systems
pub struct SystemEventBus {
    subscribers: RwLock<HashMap<SystemEventType, Vec<Weak<dyn SystemEventHandler>>>>,
    event_queue: Mutex<VecDeque<SystemEvent>>,
}

/// System event types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SystemEventType {
    SystemStarted(SystemId),
    SystemStopped(SystemId),
    SystemError(SystemId),
    ResourceAvailable(String),
    ResourceExhausted(String),
    PerformanceAlert(SystemId),
    Custom(String),
}

/// System event
#[derive(Debug, Clone)]
pub struct SystemEvent {
    pub event_type: SystemEventType,
    pub timestamp: Instant,
    pub data: Option<Vec<u8>>,
}

/// System event handler trait
pub trait SystemEventHandler: Send + Sync {
    fn handle_event(&self, event: &SystemEvent);
}

impl SystemCoordinator {
    /// Create a new system coordinator
    pub fn new(target_fps: f32) -> Self {
        let target_frame_time_ms = 1000.0 / target_fps as f64;

        Self {
            health_monitor: Arc::new(RwLock::new(HashMap::new())),
            dependencies: HashMap::new(),
            execution_order: Vec::new(),
            execution_times: RwLock::new(HashMap::new()),
            frame_budget: FrameBudgetManager::new(target_frame_time_ms),
            sync_barriers: Mutex::new(HashMap::new()),
            recovery_strategies: Self::default_recovery_strategies(),
            metrics: Arc::new(RwLock::new(SystemMetrics::default())),
            event_bus: Arc::new(SystemEventBus::new()),
            current_frame: Arc::new(RwLock::new(SystemExecutionContext {
                frame_budget_ms: target_frame_time_ms,
                elapsed_time_ms: 0.0,
                current_frame: 0,
                target_fps,
                systems_completed: HashSet::new(),
                systems_in_progress: HashSet::new(),
            })),
        }
    }

    /// Register a system with dependencies
    pub fn register_system(
        &mut self,
        system_id: SystemId,
        dependencies: SystemDependencies,
        budget_percentage: f64,
    ) -> EngineResult<()> {
        // Add to dependencies
        self.dependencies.insert(system_id, dependencies);

        // Set frame budget
        self.frame_budget
            .set_system_budget(system_id, budget_percentage);

        // Initialize health monitoring
        {
            let mut health = self.health_monitor.write();
            health.insert(
                system_id,
                SystemHealth {
                    system_id,
                    state: SystemState::Stopped,
                    last_update: Instant::now(),
                    average_frame_time_ms: 0.0,
                    error_count: 0,
                    memory_usage_mb: 0.0,
                    cpu_usage_percent: 0.0,
                    is_healthy: true,
                },
            );
        }

        // Recalculate execution order
        self.update_execution_order()?;

        Ok(())
    }

    /// Update execution order based on dependencies (topological sort)
    fn update_execution_order(&mut self) -> EngineResult<()> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        for &system_id in self.dependencies.keys() {
            if !visited.contains(&system_id) {
                self.visit_system(system_id, &mut visited, &mut visiting, &mut order)?;
            }
        }

        self.execution_order = order;
        Ok(())
    }

    fn visit_system(
        &self,
        system_id: SystemId,
        visited: &mut HashSet<SystemId>,
        visiting: &mut HashSet<SystemId>,
        order: &mut Vec<SystemId>,
    ) -> EngineResult<()> {
        if visiting.contains(&system_id) {
            return Err(EngineError::ValidationFailed(format!(
                "Circular dependency detected involving {:?}",
                system_id
            )));
        }

        if visited.contains(&system_id) {
            return Ok(());
        }

        visiting.insert(system_id);

        if let Some(deps) = self.dependencies.get(&system_id) {
            for &dep in &deps.depends_on {
                self.visit_system(dep, visited, visiting, order)?;
            }
        }

        visiting.remove(&system_id);
        visited.insert(system_id);
        order.push(system_id);

        Ok(())
    }

    /// Execute all systems for one frame
    pub fn execute_frame(&self) -> EngineResult<FrameExecutionReport> {
        let frame_start = Instant::now();
        let mut report = FrameExecutionReport::new();

        // Update frame context
        {
            let mut ctx = self.current_frame.write();
            ctx.current_frame += 1;
            ctx.elapsed_time_ms = 0.0;
            ctx.systems_completed.clear();
            ctx.systems_in_progress.clear();
        }

        // Execute systems in order
        for &system_id in &self.execution_order {
            let system_start = Instant::now();

            // Check if system is healthy enough to run
            if !self.is_system_healthy(system_id) {
                report.skipped_systems.push(system_id);
                continue;
            }

            // Wait for dependencies
            if let Err(e) = self.wait_for_dependencies(system_id) {
                log::warn!("Failed to wait for dependencies for {:?}: {}", system_id, e);
                report.failed_systems.push((system_id, e.to_string()));
                continue;
            }

            // Check frame budget
            let budget = self.frame_budget.get_system_budget(system_id);
            let elapsed = frame_start.elapsed().as_secs_f64() * 1000.0;
            if elapsed + budget > self.frame_budget.target_frame_time_ms {
                report.budget_exceeded_systems.push(system_id);
                continue;
            }

            // Execute system
            match self.execute_system(system_id) {
                Ok(()) => {
                    let execution_time = system_start.elapsed();
                    report.executed_systems.push((system_id, execution_time));

                    // Update execution times history
                    let mut times = self.execution_times.write();
                    let history = times.entry(system_id).or_insert_with(VecDeque::new);
                    history.push_back(execution_time);
                    if history.len() > 60 {
                        // Keep last 60 frames
                        history.pop_front();
                    }

                    // Mark as completed
                    self.current_frame
                        .write()
                        .systems_completed
                        .insert(system_id);
                }
                Err(e) => {
                    log::error!("System {:?} execution failed: {}", system_id, e);
                    report.failed_systems.push((system_id, e.to_string()));

                    // Handle error recovery
                    self.handle_system_error(system_id, e);
                }
            }
        }

        // Update metrics
        let total_frame_time = frame_start.elapsed();
        self.update_metrics(&report, total_frame_time);

        // Process events
        self.event_bus.process_events();

        report.total_frame_time = total_frame_time;
        Ok(report)
    }

    /// Execute a single system
    fn execute_system(&self, system_id: SystemId) -> EngineResult<()> {
        // Mark as in progress
        self.current_frame
            .write()
            .systems_in_progress
            .insert(system_id);

        // Update health monitoring
        {
            let mut health = self.health_monitor.write();
            if let Some(h) = health.get_mut(&system_id) {
                h.state = SystemState::Running;
                h.last_update = Instant::now();
            }
        }

        // Execute on appropriate thread pool
        let pool_category = self.get_pool_category(system_id);

        // Convert PoolCategory to GpuWorkloadCategory
        let gpu_category = match pool_category {
            PoolCategory::Physics => crate::thread_pool::GpuWorkloadCategory::Physics,
            PoolCategory::MeshBuilding => crate::thread_pool::GpuWorkloadCategory::Rendering,
            _ => crate::thread_pool::GpuWorkloadCategory::Compute,
        };

        // For now, this is a placeholder - actual system execution will be
        // handled by the specific system implementations
        ThreadPoolManager::global().execute(gpu_category, || {
            // System-specific execution logic would go here
            std::thread::sleep(Duration::from_millis(1)); // Placeholder
        });

        // Remove from in progress
        self.current_frame
            .write()
            .systems_in_progress
            .remove(&system_id);

        Ok(())
    }

    /// Wait for system dependencies to complete
    fn wait_for_dependencies(&self, system_id: SystemId) -> EngineResult<()> {
        if let Some(deps) = self.dependencies.get(&system_id) {
            let start = Instant::now();
            let timeout = Duration::from_millis(deps.max_wait_time_ms);

            while start.elapsed() < timeout {
                let ctx = self.current_frame.read();

                // Check if all dependencies are completed
                let all_complete = deps
                    .depends_on
                    .iter()
                    .all(|dep| ctx.systems_completed.contains(dep));

                if all_complete {
                    return Ok(());
                }

                // Check for conflicts
                let has_conflicts = deps
                    .conflicts_with
                    .iter()
                    .any(|conflict| ctx.systems_in_progress.contains(conflict));

                if has_conflicts {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                }

                std::thread::sleep(Duration::from_millis(1));
            }

            return Err(EngineError::TimeoutError(format!(
                "Timeout waiting for dependencies of {:?}",
                system_id
            )));
        }

        Ok(())
    }

    /// Check if a system is healthy
    fn is_system_healthy(&self, system_id: SystemId) -> bool {
        let health = self.health_monitor.read();
        health
            .get(&system_id)
            .map(|h| h.is_healthy && h.state != SystemState::Error)
            .unwrap_or(false)
    }

    /// Handle system execution error
    fn handle_system_error(&self, system_id: SystemId, error: EngineError) {
        // Update health
        {
            let mut health = self.health_monitor.write();
            if let Some(h) = health.get_mut(&system_id) {
                h.state = SystemState::Error;
                h.error_count += 1;
                h.is_healthy = h.error_count < 5; // Allow up to 5 errors
            }
        }

        // Emit error event
        self.event_bus.emit_event(SystemEvent {
            event_type: SystemEventType::SystemError(system_id),
            timestamp: Instant::now(),
            data: Some(error.to_string().into_bytes()),
        });

        // Apply recovery strategy
        if let Some(strategy) = self.recovery_strategies.get(&system_id) {
            match strategy {
                RecoveryStrategy::Restart => {
                    log::info!("Attempting to restart system {:?}", system_id);
                    // Restart logic would go here
                }
                RecoveryStrategy::Skip => {
                    log::info!("Skipping system {:?} due to error", system_id);
                }
                RecoveryStrategy::FallbackMode => {
                    log::info!("Switching system {:?} to fallback mode", system_id);
                    // Fallback logic would go here
                }
                RecoveryStrategy::Shutdown => {
                    log::error!(
                        "Critical system {:?} failed, initiating shutdown",
                        system_id
                    );
                    // Shutdown logic would go here
                }
            }
        }
    }

    /// Update performance metrics
    fn update_metrics(&self, report: &FrameExecutionReport, total_time: Duration) {
        let mut metrics = self.metrics.write();
        metrics.frame_count += 1;
        metrics.total_frame_time_ms += total_time.as_secs_f64() * 1000.0;

        for (system_id, execution_time) in &report.executed_systems {
            let time_ms = execution_time.as_secs_f64() * 1000.0;
            *metrics.system_times.entry(*system_id).or_insert(0.0) += time_ms;
        }
    }

    /// Get thread pool category for system
    fn get_pool_category(&self, system_id: SystemId) -> PoolCategory {
        match system_id {
            SystemId::WorldGeneration => PoolCategory::WorldGeneration,
            SystemId::Physics => PoolCategory::Physics,
            SystemId::Renderer => PoolCategory::MeshBuilding,
            SystemId::Lighting => PoolCategory::Lighting,
            SystemId::Network => PoolCategory::Network,
            SystemId::Persistence => PoolCategory::FileIO,
            _ => PoolCategory::Compute,
        }
    }

    /// Default recovery strategies
    fn default_recovery_strategies() -> HashMap<SystemId, RecoveryStrategy> {
        let mut strategies = HashMap::new();
        strategies.insert(SystemId::WorldGeneration, RecoveryStrategy::Restart);
        strategies.insert(SystemId::Physics, RecoveryStrategy::Restart);
        strategies.insert(SystemId::Renderer, RecoveryStrategy::FallbackMode);
        strategies.insert(SystemId::Lighting, RecoveryStrategy::Skip);
        strategies.insert(SystemId::Network, RecoveryStrategy::Restart);
        strategies.insert(SystemId::Persistence, RecoveryStrategy::Shutdown);
        strategies.insert(SystemId::Audio, RecoveryStrategy::Skip);
        strategies.insert(SystemId::Input, RecoveryStrategy::Restart);
        strategies.insert(SystemId::UI, RecoveryStrategy::FallbackMode);
        strategies.insert(SystemId::Particles, RecoveryStrategy::Skip);
        strategies.insert(SystemId::Weather, RecoveryStrategy::Skip);
        strategies
    }

    /// Get system health report
    pub fn get_health_report(&self) -> Vec<SystemHealth> {
        self.health_monitor.read().values().cloned().collect()
    }

    /// Get performance metrics
    pub fn get_metrics(&self) -> SystemMetrics {
        self.metrics.read().clone()
    }

    /// Subscribe to system events
    pub fn subscribe_to_events<H: SystemEventHandler + 'static>(
        &self,
        event_type: SystemEventType,
        handler: Arc<H>,
    ) {
        self.event_bus.subscribe(event_type, handler);
    }
}

/// Frame execution report
#[derive(Debug)]
pub struct FrameExecutionReport {
    pub executed_systems: Vec<(SystemId, Duration)>,
    pub failed_systems: Vec<(SystemId, String)>,
    pub skipped_systems: Vec<SystemId>,
    pub budget_exceeded_systems: Vec<SystemId>,
    pub total_frame_time: Duration,
}

impl FrameExecutionReport {
    fn new() -> Self {
        Self {
            executed_systems: Vec::new(),
            failed_systems: Vec::new(),
            skipped_systems: Vec::new(),
            budget_exceeded_systems: Vec::new(),
            total_frame_time: Duration::ZERO,
        }
    }
}

impl FrameBudgetManager {
    fn new(target_frame_time_ms: f64) -> Self {
        Self {
            target_frame_time_ms,
            system_budgets: HashMap::new(),
            budget_usage: RwLock::new(HashMap::new()),
            adaptive_scaling: true,
        }
    }

    fn set_system_budget(&mut self, system_id: SystemId, percentage: f64) {
        let budget_ms = self.target_frame_time_ms * (percentage / 100.0);
        self.system_budgets.insert(system_id, budget_ms);
    }

    fn get_system_budget(&self, system_id: SystemId) -> f64 {
        self.system_budgets.get(&system_id).copied().unwrap_or(1.0)
    }
}

impl SystemEventBus {
    fn new() -> Self {
        Self {
            subscribers: RwLock::new(HashMap::new()),
            event_queue: Mutex::new(VecDeque::new()),
        }
    }

    fn subscribe<H: SystemEventHandler + 'static>(
        &self,
        event_type: SystemEventType,
        handler: Arc<H>,
    ) {
        let mut subscribers = self.subscribers.write();
        subscribers
            .entry(event_type)
            .or_insert_with(Vec::new)
            .push(Arc::downgrade(&handler) as Weak<dyn SystemEventHandler>);
    }

    fn emit_event(&self, event: SystemEvent) {
        self.event_queue.lock().push_back(event);
    }

    fn process_events(&self) {
        let mut queue = self.event_queue.lock();
        let events: Vec<_> = queue.drain(..).collect();
        drop(queue);

        let subscribers = self.subscribers.read();

        for event in events {
            if let Some(handlers) = subscribers.get(&event.event_type) {
                for weak_handler in handlers {
                    if let Some(handler) = weak_handler.upgrade() {
                        handler.handle_event(&event);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_coordinator_creation() {
        let coordinator = SystemCoordinator::new(60.0);
        assert_eq!(coordinator.execution_order.len(), 0);
    }

    #[test]
    fn test_dependency_ordering() {
        let mut coordinator = SystemCoordinator::new(60.0);

        // World generation depends on nothing
        coordinator
            .register_system(
                SystemId::WorldGeneration,
                SystemDependencies {
                    depends_on: vec![],
                    conflicts_with: vec![],
                    max_wait_time_ms: 1000,
                },
                20.0,
            )
            .expect("Failed to register WorldGeneration system");

        // Physics depends on world generation
        coordinator
            .register_system(
                SystemId::Physics,
                SystemDependencies {
                    depends_on: vec![SystemId::WorldGeneration],
                    conflicts_with: vec![],
                    max_wait_time_ms: 1000,
                },
                30.0,
            )
            .expect("Failed to register Physics system");

        // Renderer depends on physics
        coordinator
            .register_system(
                SystemId::Renderer,
                SystemDependencies {
                    depends_on: vec![SystemId::Physics],
                    conflicts_with: vec![],
                    max_wait_time_ms: 1000,
                },
                40.0,
            )
            .expect("Failed to register Renderer system");

        // Check execution order
        assert_eq!(coordinator.execution_order[0], SystemId::WorldGeneration);
        assert_eq!(coordinator.execution_order[1], SystemId::Physics);
        assert_eq!(coordinator.execution_order[2], SystemId::Renderer);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut coordinator = SystemCoordinator::new(60.0);

        // System A depends on B
        coordinator
            .register_system(
                SystemId::Physics,
                SystemDependencies {
                    depends_on: vec![SystemId::Renderer],
                    conflicts_with: vec![],
                    max_wait_time_ms: 1000,
                },
                50.0,
            )
            .expect("Failed to register Physics system");

        // System B depends on A (circular)
        let result = coordinator.register_system(
            SystemId::Renderer,
            SystemDependencies {
                depends_on: vec![SystemId::Physics],
                conflicts_with: vec![],
                max_wait_time_ms: 1000,
            },
            50.0,
        );

        assert!(result.is_err());
    }
}
