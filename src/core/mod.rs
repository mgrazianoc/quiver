//! Core engine — connection management, Flight SQL client, data layer.
//!
//! This module will house:
//! - `ConnectionProfile` and `ConnectionManager` (§5)
//! - Flight SQL client wrapper around `arrow_flight::sql::FlightSqlServiceClient` (§4)
//! - `ResultStore` holding `Vec<RecordBatch>` per tab (§3)
//! - DataFusion `SessionContext` for local analytics (§8)
//! - Stream lifecycle management (§4.5)
//! - Query history persistence (§10.1)
//!
//! For now, the TUI operates on placeholder data. The integration points
//! are clearly defined in `app.rs`:
//!   - `App::result_headers` / `App::result_rows` → will become `Vec<RecordBatch>`
//!   - `App::schema_tree` → will be populated from Flight SQL catalog RPCs
//!   - `App::tabs[n].state` → will track real query execution state

/// Placeholder for connection profile.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ConnectionProfile {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub tls_enabled: bool,
    // Future: auth method, mTLS certs, metadata headers, etc.
}

/// Placeholder for connection state.
#[allow(dead_code)]
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
