//! Network Module - Simplified for DOP conversion
//!
//! This module will be properly implemented after DOP conversion is complete.

pub mod anticheat;
pub mod connection;
pub mod disconnect_handler;
pub mod interest;
pub mod interpolation;
pub mod lag_compensation;
pub mod network_data;
pub mod network_operations;
pub mod packet;
pub mod prediction;
pub mod protocol;

// Simple re-exports matching our stub implementations
pub use anticheat::AntiCheat;
pub use connection::Connection;
pub use disconnect_handler::{DisconnectHandler, DisconnectReason, ConnectionState};
pub use interest::InterestManager;
pub use interpolation::Interpolation;
pub use lag_compensation::LagCompensation;
pub use network_data::NetworkData;
pub use packet::Packet;
pub use prediction::Prediction;
pub use protocol::Protocol;

// Network module error (stub)
pub mod error {
    pub type NetworkResult<T> = Result<T, String>;
}

pub use error::NetworkResult;
