//! Disconnect Handler Data - Pure DOP
//!
//! NO METHODS. Just data.
//! All transformations happen in disconnect_handler_operations.rs

use crate::persistence::AtomicSaveData;
use crate::ChunkPos;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Player connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Connected,
    Disconnecting,
    Disconnected,
    SaveComplete,
}

/// Reason for player disconnection
#[derive(Debug, Clone, PartialEq)]
pub enum DisconnectReason {
    ClientQuit,
    Timeout,
    Kicked,
    Error,
}

/// Information about a disconnecting player
#[derive(Debug, Clone)]
pub struct DisconnectingPlayer {
    pub uuid: String,
    pub username: String,
    pub position: (f64, f64, f64),
    pub chunks_to_save: HashSet<ChunkPos>,
    pub disconnect_time: Instant,
    pub state: ConnectionState,
}

/// Configuration for disconnect handling
#[derive(Debug, Clone)]
pub struct DisconnectConfig {
    /// Maximum time to wait for save completion before force disconnect
    pub max_save_timeout: Duration,
    /// Radius around player to save chunks
    pub chunk_save_radius: i32,
    /// Enable emergency save mode for critical failures
    pub emergency_save_enabled: bool,
    /// Grace period for reconnection before save
    pub reconnect_grace_period: Duration,
}

impl Default for DisconnectConfig {
    fn default() -> Self {
        Self {
            max_save_timeout: Duration::from_secs(30),
            chunk_save_radius: 3,
            emergency_save_enabled: true,
            reconnect_grace_period: Duration::from_secs(5),
        }
    }
}

/// Statistics for disconnect handling
#[derive(Debug, Clone, Default)]
pub struct DisconnectStats {
    pub players_disconnecting: usize,
    pub successful_saves: u64,
    pub failed_saves: u64,
    pub emergency_saves: u64,
    pub average_save_time: Duration,
    pub force_disconnects: u64,
}

/// Disconnect handler data
pub struct DisconnectHandlerData {
    /// Players currently disconnecting
    pub disconnecting_players: Arc<Mutex<HashMap<String, DisconnectingPlayer>>>,

    /// Atomic save data for safe operations
    pub save_data: Arc<AtomicSaveData>,

    /// Configuration
    pub config: DisconnectConfig,

    /// Statistics
    pub stats: Arc<Mutex<DisconnectStats>>,

    /// Background thread handle
    pub worker_thread: Option<thread::JoinHandle<()>>,

    /// Shutdown signal
    pub shutdown: Arc<Mutex<bool>>,
}
