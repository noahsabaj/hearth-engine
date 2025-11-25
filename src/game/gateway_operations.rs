//! Game Gateway Operations - Pure DOP Functions
//!
//! Functions that operate on GameGatewayData.
//! This is the OPTIONAL event queue - games can bypass and call engine operations directly.

use super::gateway_data::{
    GameEvent, GameCommand, GameGatewayData, GatewayConfig, GatewayMetrics,
    MessageType, BlockRegistration,
};
use crate::world::core::{BlockId, BlockRegistry};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Global gateway storage (optional - only used if game wants event queue)
static GATEWAY: Mutex<Option<GameGatewayData>> = Mutex::new(None);

// ============================================================================
// INITIALIZATION
// ============================================================================

/// Initialize gateway with default config
pub fn init_gateway() {
    let mut guard = GATEWAY.lock().expect("[Gateway] Failed to lock");
    *guard = Some(GameGatewayData::default());
}

/// Initialize gateway with custom config
pub fn init_gateway_with_config(config: GatewayConfig) {
    let mut guard = GATEWAY.lock().expect("[Gateway] Failed to lock");
    let mut gateway = GameGatewayData::default();
    gateway.config = config;
    gateway.initialized = true;
    *guard = Some(gateway);
}

/// Shutdown gateway and clear all state
pub fn shutdown_gateway() {
    let mut guard = GATEWAY.lock().expect("[Gateway] Failed to lock");
    *guard = None;
}

/// Check if gateway is initialized
pub fn is_gateway_initialized() -> bool {
    let guard = GATEWAY.lock().expect("[Gateway] Failed to lock");
    guard.is_some()
}

// ============================================================================
// EVENT QUEUE OPERATIONS
// ============================================================================

/// Queue a single event for processing
pub fn queue_event(event: GameEvent) {
    let mut guard = GATEWAY.lock().expect("[Gateway] Failed to lock");

    if let Some(gateway) = guard.as_mut() {
        // Check queue size
        if gateway.pending_events.len() >= gateway.config.max_queue_size {
            gateway.metrics.events_dropped += 1;
            if gateway.config.debug_logging {
                log::warn!("[Gateway] Event queue full, dropping event: {:?}", event);
            }
            return;
        }

        gateway.pending_events.push_back(event);

        // Update peak queue size
        if gateway.pending_events.len() > gateway.metrics.peak_queue_size {
            gateway.metrics.peak_queue_size = gateway.pending_events.len();
        }
    }
}

/// Queue multiple events at once
pub fn queue_events(events: Vec<GameEvent>) {
    let mut guard = GATEWAY.lock().expect("[Gateway] Failed to lock");

    if let Some(gateway) = guard.as_mut() {
        for event in events {
            if gateway.pending_events.len() >= gateway.config.max_queue_size {
                gateway.metrics.events_dropped += 1;
                continue;
            }

            gateway.pending_events.push_back(event);
        }

        if gateway.pending_events.len() > gateway.metrics.peak_queue_size {
            gateway.metrics.peak_queue_size = gateway.pending_events.len();
        }
    }
}

/// Process all pending events (call this once per frame/tick)
pub fn process_update() {
    let start = Instant::now();
    let mut guard = GATEWAY.lock().expect("[Gateway] Failed to lock");

    if let Some(gateway) = guard.as_mut() {
        let event_count = gateway.pending_events.len();

        // Process all events
        while let Some(event) = gateway.pending_events.pop_front() {
            process_single_event(gateway, &event);
            gateway.metrics.events_processed += 1;
        }

        // Process all commands
        while let Some(command) = gateway.pending_commands.pop_front() {
            execute_single_command(gateway, &command);
            gateway.metrics.commands_executed += 1;
        }

        // Update average processing time
        if event_count > 0 {
            let elapsed = start.elapsed().as_micros() as f32;
            let current_avg = gateway.metrics.avg_process_time_us;

            // Exponential moving average
            gateway.metrics.avg_process_time_us =
                if current_avg == 0.0 {
                    elapsed / event_count as f32
                } else {
                    current_avg * 0.9 + (elapsed / event_count as f32) * 0.1
                };
        }
    }
}

/// Process a single event (internal)
fn process_single_event(gateway: &mut GameGatewayData, event: &GameEvent) {
    if gateway.config.debug_logging {
        log::debug!("[Gateway] Processing event: {:?}", event);
    }

    // In a real implementation, this would call engine operations
    // For now, we just log
    match event {
        GameEvent::BlockBreak { position, block_id, .. } => {
            log::debug!("[Gateway] Block break at {:?}: {:?}", position, block_id);
        }
        GameEvent::BlockPlace { position, block_id, .. } => {
            log::debug!("[Gateway] Block place at {:?}: {:?}", position, block_id);
        }
        _ => {}
    }
}

/// Execute a single command (internal)
fn execute_single_command(gateway: &mut GameGatewayData, command: &GameCommand) {
    if gateway.config.debug_logging {
        log::debug!("[Gateway] Executing command: {:?}", command);
    }

    match command {
        GameCommand::Shutdown => {
            log::info!("[Gateway] Shutdown command received");
        }
        _ => {}
    }
}

// ============================================================================
// BLOCK REGISTRATION
// ============================================================================

/// Register blocks with the engine
pub fn register_blocks(registry: &mut BlockRegistry) {
    let guard = GATEWAY.lock().expect("[Gateway] Failed to lock");

    if let Some(gateway) = guard.as_ref() {
        for block_reg in &gateway.registered_blocks {
            // Register block with engine
            log::debug!("[Gateway] Registering block: {}", block_reg.name);
            // TODO: Actually register with BlockRegistry
        }
    }
}

/// Add a block registration to the gateway
pub fn add_block_registration(registration: BlockRegistration) {
    let mut guard = GATEWAY.lock().expect("[Gateway] Failed to lock");

    if let Some(gateway) = guard.as_mut() {
        gateway.registered_blocks.push(registration);
    }
}

/// Get active block for placement
pub fn get_active_block() -> BlockId {
    let guard = GATEWAY.lock().expect("[Gateway] Failed to lock");

    if let Some(gateway) = guard.as_ref() {
        gateway.active_block
    } else {
        BlockId(1) // Default
    }
}

/// Set active block for placement
pub fn set_active_block(block_id: BlockId) {
    let mut guard = GATEWAY.lock().expect("[Gateway] Failed to lock");

    if let Some(gateway) = guard.as_mut() {
        gateway.active_block = block_id;
    }
}

// ============================================================================
// PERSISTENCE
// ============================================================================

/// Save game state (stub - to be implemented)
pub fn save_game_state(path: &str) -> Result<(), String> {
    log::info!("[Gateway] Saving game state to: {}", path);
    // TODO: Implement actual save logic
    Ok(())
}

/// Load game state (stub - to be implemented)
pub fn load_game_state(path: &str) -> Result<(), String> {
    log::info!("[Gateway] Loading game state from: {}", path);
    // TODO: Implement actual load logic
    Ok(())
}

// ============================================================================
// METRICS & CONFIGURATION
// ============================================================================

/// Get gateway metrics
pub fn get_metrics() -> GatewayMetrics {
    let guard = GATEWAY.lock().expect("[Gateway] Failed to lock");

    if let Some(gateway) = guard.as_ref() {
        gateway.metrics
    } else {
        GatewayMetrics::default()
    }
}

/// Reset metrics
pub fn reset_metrics() {
    let mut guard = GATEWAY.lock().expect("[Gateway] Failed to lock");

    if let Some(gateway) = guard.as_mut() {
        gateway.metrics = GatewayMetrics::default();
    }
}

/// Get gateway configuration
pub fn get_gateway_config() -> Option<GatewayConfig> {
    let guard = GATEWAY.lock().expect("[Gateway] Failed to lock");

    guard.as_ref().map(|g| g.config.clone())
}

/// Update gateway configuration
pub fn update_gateway_config(config: GatewayConfig) {
    let mut guard = GATEWAY.lock().expect("[Gateway] Failed to lock");

    if let Some(gateway) = guard.as_mut() {
        gateway.config = config;
    }
}

// ============================================================================
// MESSAGING
// ============================================================================

/// Post a message to the gateway
pub fn post_message(message: MessageType) {
    let mut guard = GATEWAY.lock().expect("[Gateway] Failed to lock");

    if let Some(gateway) = guard.as_mut() {
        gateway.messages.push_back(message);
    }
}

/// Get and clear all messages
pub fn drain_messages() -> Vec<MessageType> {
    let mut guard = GATEWAY.lock().expect("[Gateway] Failed to lock");

    if let Some(gateway) = guard.as_mut() {
        gateway.messages.drain(..).collect()
    } else {
        Vec::new()
    }
}

// ============================================================================
// DEBUG / DIAGNOSTICS
// ============================================================================

/// Get queue status for debugging
pub fn get_queue_status() -> (usize, usize) {
    let guard = GATEWAY.lock().expect("[Gateway] Failed to lock");

    if let Some(gateway) = guard.as_ref() {
        (gateway.pending_events.len(), gateway.pending_commands.len())
    } else {
        (0, 0)
    }
}

/// Log gateway status
pub fn log_gateway_status() {
    let guard = GATEWAY.lock().expect("[Gateway] Failed to lock");

    if let Some(gateway) = guard.as_ref() {
        log::info!("[Gateway] Status:");
        log::info!("  Events in queue: {}", gateway.pending_events.len());
        log::info!("  Commands in queue: {}", gateway.pending_commands.len());
        log::info!("  Events processed: {}", gateway.metrics.events_processed);
        log::info!("  Events dropped: {}", gateway.metrics.events_dropped);
        log::info!("  Peak queue size: {}", gateway.metrics.peak_queue_size);
        log::info!("  Avg process time: {:.2}μs", gateway.metrics.avg_process_time_us);
    } else {
        log::warn!("[Gateway] Not initialized");
    }
}
