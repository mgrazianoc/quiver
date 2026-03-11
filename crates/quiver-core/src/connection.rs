//! Connection profile and state management.

use serde::{Deserialize, Serialize};

/// A saved connection profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProfile {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub tls_enabled: bool,
    // Future: auth method, mTLS certs, metadata headers, etc.
}

/// Tracks the lifecycle of a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

impl ConnectionState {
    pub fn label(&self) -> &'static str {
        match self {
            ConnectionState::Disconnected => "Disconnected",
            ConnectionState::Connecting => "Connecting…",
            ConnectionState::Connected => "Connected",
            ConnectionState::Error => "Error",
        }
    }

    pub fn dot(&self) -> &'static str {
        match self {
            ConnectionState::Disconnected => "○",
            ConnectionState::Connecting => "◐",
            ConnectionState::Connected => "●",
            ConnectionState::Error => "✗",
        }
    }
}
