//! Disconnect Handler Operations - Pure DOP Functions
//!
//! All functions are pure: take data, return results, no side effects.
//! No methods, no self, just transformations.

use super::disconnect_handler_data::{
    ConnectionState, DisconnectConfig, DisconnectHandlerData, DisconnectStats, DisconnectingPlayer,
};
use crate::persistence::{AtomicSaveData, PersistenceError, PersistenceResult, SaveOperation, SavePriority};
use crate::{ChunkPos, World};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Create a new disconnect handler
pub fn create_disconnect_handler(
    save_data: Arc<AtomicSaveData>,
    config: DisconnectConfig,
) -> DisconnectHandlerData {
    DisconnectHandlerData {
        disconnecting_players: Arc::new(Mutex::new(HashMap::new())),
        save_data,
        config,
        stats: Arc::new(Mutex::new(DisconnectStats {
            players_disconnecting: 0,
            successful_saves: 0,
            failed_saves: 0,
            emergency_saves: 0,
            average_save_time: Duration::from_millis(0),
            force_disconnects: 0,
        })),
        worker_thread: None,
        shutdown: Arc::new(Mutex::new(false)),
    }
}

/// Start the background worker thread
pub fn start(data: &mut DisconnectHandlerData) -> PersistenceResult<()> {
    let disconnecting_players = Arc::clone(&data.disconnecting_players);
    let save_data = Arc::clone(&data.save_data);
    let config = data.config.clone();
    let stats = Arc::clone(&data.stats);
    let shutdown = Arc::clone(&data.shutdown);

    data.worker_thread = Some(thread::spawn(move || {
        worker_loop(disconnecting_players, save_data, config, stats, shutdown);
    }));

    Ok(())
}

/// Stop the background worker
pub fn stop(data: &mut DisconnectHandlerData) -> PersistenceResult<()> {
    if let Ok(mut shutdown) = data.shutdown.lock() {
        *shutdown = true;
    }

    if let Some(handle) = data.worker_thread.take() {
        let _ = handle.join();
    }

    Ok(())
}

/// Handle a player disconnect request
pub fn handle_disconnect(
    data: &DisconnectHandlerData,
    player_uuid: String,
    username: String,
    world: &World,
    player_position: (f64, f64, f64),
) -> PersistenceResult<()> {
    let chunks_to_save = get_chunks_around_player(&data.config, player_position);

    let disconnecting_player = DisconnectingPlayer {
        uuid: player_uuid.clone(),
        username,
        position: player_position,
        chunks_to_save,
        disconnect_time: Instant::now(),
        state: ConnectionState::Disconnecting,
    };

    // Add to disconnecting players list
    {
        let mut players = data
            .disconnecting_players
            .lock()
            .map_err(|_| PersistenceError::LockPoisoned("disconnecting_players".to_string()))?;
        players.insert(player_uuid.clone(), disconnecting_player.clone());
    }

    // Queue save operations with critical priority
    queue_player_saves(data, &disconnecting_player, world)?;

    // Update stats
    if let Ok(mut stats) = data.stats.lock() {
        stats.players_disconnecting += 1;
    }

    println!(
        "[DisconnectHandler] Handling disconnect for player {} at {:?}",
        player_uuid, player_position
    );

    Ok(())
}

/// Handle emergency disconnect (e.g., crashed connection)
pub fn handle_emergency_disconnect(
    data: &DisconnectHandlerData,
    player_uuid: String,
    _world: &World,
    player_position: (f64, f64, f64),
) -> PersistenceResult<()> {
    if !data.config.emergency_save_enabled {
        return Ok(());
    }

    println!(
        "[DisconnectHandler] Emergency disconnect for player {}",
        player_uuid
    );

    // Immediately queue critical saves
    let chunks_to_save = get_chunks_around_player(&data.config, player_position);

    // Queue player data save with critical priority
    crate::persistence::queue_operation(
        &data.save_data,
        SaveOperation::Player {
            player_id: 0, // TODO: Get actual player ID
            uuid: player_uuid.clone(),
            position: [
                player_position.0 as f32,
                player_position.1 as f32,
                player_position.2 as f32,
            ],
            priority: SavePriority::Critical,
        },
    )?;

    // Queue chunk saves with critical priority
    if !chunks_to_save.is_empty() {
        let positions: Vec<(i32, i32, i32)> = chunks_to_save
            .iter()
            .map(|pos| (pos.x, pos.y, pos.z))
            .collect();
        let chunks: Vec<u64> = chunks_to_save
            .iter()
            .map(|pos| ((pos.x as u64) << 42) | ((pos.y as u64) << 21) | (pos.z as u64))
            .collect();

        data.save_data.queue_operation(SaveOperation::ChunkBatch {
            chunks,
            positions,
            priority: SavePriority::Critical,
        })?;
    }

    // Update stats
    if let Ok(mut stats) = data.stats.lock() {
        stats.emergency_saves += 1;
    }

    Ok(())
}

/// Check if a player is currently disconnecting
pub fn is_player_disconnecting(data: &DisconnectHandlerData, player_uuid: &str) -> bool {
    if let Ok(players) = data.disconnecting_players.lock() {
        players.contains_key(player_uuid)
    } else {
        false
    }
}

/// Get disconnect status for a player
pub fn get_disconnect_status(
    data: &DisconnectHandlerData,
    player_uuid: &str,
) -> Option<ConnectionState> {
    if let Ok(players) = data.disconnecting_players.lock() {
        players.get(player_uuid).map(|p| p.state.clone())
    } else {
        None
    }
}

/// Force disconnect a player (emergency override)
pub fn force_disconnect(data: &DisconnectHandlerData, player_uuid: &str) -> PersistenceResult<bool> {
    let mut players = data
        .disconnecting_players
        .lock()
        .map_err(|_| PersistenceError::LockPoisoned("disconnecting_players".to_string()))?;

    if let Some(mut player) = players.remove(player_uuid) {
        player.state = ConnectionState::Disconnected;

        // Update stats
        if let Ok(mut stats) = data.stats.lock() {
            stats.force_disconnects += 1;
            stats.players_disconnecting = stats.players_disconnecting.saturating_sub(1);
        }

        println!(
            "[DisconnectHandler] Force disconnected player {}",
            player_uuid
        );
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Get chunks around a player position that need saving
fn get_chunks_around_player(config: &DisconnectConfig, position: (f64, f64, f64)) -> HashSet<ChunkPos> {
    let (x, _y, z) = position;
    let chunk_x = (x as i32) >> 4; // Assuming 16x16 chunks
    let chunk_z = (z as i32) >> 4;

    let mut chunks = HashSet::new();
    let radius = config.chunk_save_radius;

    for dx in -radius..=radius {
        for dz in -radius..=radius {
            chunks.insert(ChunkPos {
                x: chunk_x + dx,
                y: 0, // Assuming y=0 for simplicity
                z: chunk_z + dz,
            });
        }
    }

    chunks
}

/// Queue save operations for a disconnecting player
fn queue_player_saves(
    data: &DisconnectHandlerData,
    player: &DisconnectingPlayer,
    _world: &World,
) -> PersistenceResult<()> {
    // Queue player data save
    crate::persistence::queue_operation(
        &data.save_data,
        SaveOperation::Player {
            player_id: 0, // TODO: Get actual player ID
            uuid: player.uuid.clone(),
            position: [
                player.position.0 as f32,
                player.position.1 as f32,
                player.position.2 as f32,
            ],
            priority: SavePriority::Critical,
        },
    )?;

    // Queue chunk saves if any
    if !player.chunks_to_save.is_empty() {
        let positions: Vec<(i32, i32, i32)> = player
            .chunks_to_save
            .iter()
            .map(|pos| (pos.x, pos.y, pos.z))
            .collect();
        let chunks: Vec<u64> = player
            .chunks_to_save
            .iter()
            .map(|pos| ((pos.x as u64) << 42) | ((pos.y as u64) << 21) | (pos.z as u64))
            .collect();

        data.save_data.queue_operation(SaveOperation::ChunkBatch {
            chunks,
            positions,
            priority: SavePriority::Critical,
        })?;
    }

    Ok(())
}

/// Background worker loop
fn worker_loop(
    disconnecting_players: Arc<Mutex<HashMap<String, DisconnectingPlayer>>>,
    save_data: Arc<AtomicSaveData>,
    config: DisconnectConfig,
    stats: Arc<Mutex<DisconnectStats>>,
    shutdown: Arc<Mutex<bool>>,
) {
    loop {
        // Check shutdown signal
        if let Ok(shutdown_flag) = shutdown.lock() {
            if *shutdown_flag {
                break;
            }
        }

        thread::sleep(Duration::from_millis(100));

        // Process disconnecting players
        let players_to_process: Vec<DisconnectingPlayer> = {
            if let Ok(players) = disconnecting_players.lock() {
                players.values().cloned().collect()
            } else {
                continue;
            }
        };

        for player in players_to_process {
            let now = Instant::now();
            let disconnect_duration = now.duration_since(player.disconnect_time);

            // Check if save timeout exceeded
            if disconnect_duration > config.max_save_timeout {
                println!(
                    "[DisconnectHandler] Save timeout for player {}, forcing disconnect",
                    player.uuid
                );

                if let Ok(mut players) = disconnecting_players.lock() {
                    players.remove(&player.uuid);
                }

                if let Ok(mut stats_lock) = stats.lock() {
                    stats_lock.force_disconnects += 1;
                    stats_lock.players_disconnecting =
                        stats_lock.players_disconnecting.saturating_sub(1);
                }
                continue;
            }

            // Check if grace period passed and saves are complete
            if disconnect_duration > config.reconnect_grace_period
                && are_player_saves_complete(&save_data, &player)
            {
                println!(
                    "[DisconnectHandler] Save complete for player {}",
                    player.uuid
                );

                if let Ok(mut players) = disconnecting_players.lock() {
                    if let Some(p) = players.get_mut(&player.uuid) {
                        p.state = ConnectionState::SaveComplete;
                    }
                }

                if let Ok(mut stats_lock) = stats.lock() {
                    stats_lock.successful_saves += 1;
                    stats_lock.players_disconnecting =
                        stats_lock.players_disconnecting.saturating_sub(1);

                    // Update average save time
                    let total_saves = stats_lock.successful_saves + stats_lock.failed_saves;
                    if total_saves > 0 {
                        let total_time = stats_lock.average_save_time.as_millis() as u64
                            * (total_saves - 1)
                            + disconnect_duration.as_millis() as u64;
                        stats_lock.average_save_time = Duration::from_millis(total_time / total_saves);
                    }
                }
            }
        }
    }
}

/// Check if all saves for a player are complete
fn are_player_saves_complete(
    _save_data: &AtomicSaveData,
    _player: &DisconnectingPlayer,
) -> bool {
    // For now, assume saves complete after grace period
    // In real implementation, would check save manager queue for pending operations
    true
}

/// Get current statistics
pub fn get_stats(data: &DisconnectHandlerData) -> PersistenceResult<DisconnectStats> {
    let stats = data
        .stats
        .lock()
        .map_err(|_| PersistenceError::LockPoisoned("stats".to_string()))?;
    Ok(stats.clone())
}

/// Clear all disconnecting players (emergency shutdown)
pub fn clear_all_disconnecting(data: &DisconnectHandlerData) -> PersistenceResult<usize> {
    let mut players = data
        .disconnecting_players
        .lock()
        .map_err(|_| PersistenceError::LockPoisoned("disconnecting_players".to_string()))?;
    let count = players.len();
    players.clear();

    if let Ok(mut stats) = data.stats.lock() {
        stats.players_disconnecting = 0;
    }

    Ok(count)
}
