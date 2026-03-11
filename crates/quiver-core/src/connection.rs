//! Connection profile, authentication, state, and manager.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ── Authentication ────────────────────────────────────────────

/// How the client authenticates with the Flight SQL server.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthMethod {
    #[default]
    None,
    Basic {
        username: String,
        password: String,
    },
    BearerToken {
        token: String,
    },
}

// ── Connection Profile ────────────────────────────────────────

/// A saved connection profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionProfile {
    pub name: String,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub tls_enabled: bool,
    #[serde(default)]
    pub auth: AuthMethod,
}

impl Default for ConnectionProfile {
    fn default() -> Self {
        Self {
            name: "local".into(),
            host: "localhost".into(),
            port: 8815,
            tls_enabled: false,
            auth: AuthMethod::None,
        }
    }
}

impl ConnectionProfile {
    /// Build the gRPC endpoint URI from this profile.
    pub fn endpoint_uri(&self) -> String {
        let scheme = if self.tls_enabled { "https" } else { "http" };
        format!("{}://{}:{}", scheme, self.host, self.port)
    }
}

// ── Connection State ──────────────────────────────────────────

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

// ── Connection Manager ────────────────────────────────────────

/// Persistent store for connection profiles.
///
/// Profiles are saved as a TOML array in
/// `~/.config/quiver/connections.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionManager {
    #[serde(rename = "connections", default)]
    pub profiles: Vec<ConnectionProfile>,
}

impl ConnectionManager {
    /// Path to the connections file.
    fn file_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("quiver").join("connections.toml"))
    }

    /// Load profiles from disk. Returns empty manager if file is
    /// missing or malformed.
    pub fn load() -> Self {
        let path = match Self::file_path() {
            Some(p) => p,
            None => return Self::default(),
        };
        match std::fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Persist profiles to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::file_path().ok_or_else(|| anyhow::anyhow!("No config directory"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let toml_str = toml::to_string_pretty(self)?;
        std::fs::write(path, toml_str)?;
        Ok(())
    }

    /// Add or update a profile (matched by name).
    pub fn upsert(&mut self, profile: ConnectionProfile) {
        if let Some(existing) = self.profiles.iter_mut().find(|p| p.name == profile.name) {
            *existing = profile;
        } else {
            self.profiles.push(profile);
        }
    }

    /// Remove a profile by name. Returns `true` if found.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.profiles.len();
        self.profiles.retain(|p| p.name != name);
        self.profiles.len() < before
    }

    /// Look up a profile by name.
    pub fn get(&self, name: &str) -> Option<&ConnectionProfile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    /// List all profile names.
    pub fn names(&self) -> Vec<&str> {
        self.profiles.iter().map(|p| p.name.as_str()).collect()
    }
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile() {
        let p = ConnectionProfile::default();
        assert_eq!(p.name, "local");
        assert_eq!(p.host, "localhost");
        assert_eq!(p.port, 8815);
        assert!(!p.tls_enabled);
        assert_eq!(p.auth, AuthMethod::None);
    }

    #[test]
    fn endpoint_uri_http() {
        let p = ConnectionProfile::default();
        assert_eq!(p.endpoint_uri(), "http://localhost:8815");
    }

    #[test]
    fn endpoint_uri_https() {
        let p = ConnectionProfile {
            tls_enabled: true,
            host: "flight.example.com".into(),
            port: 443,
            ..Default::default()
        };
        assert_eq!(p.endpoint_uri(), "https://flight.example.com:443");
    }

    #[test]
    fn profile_serialization_roundtrip_none_auth() {
        let p = ConnectionProfile::default();
        let toml_str = toml::to_string_pretty(&p).unwrap();
        let decoded: ConnectionProfile = toml::from_str(&toml_str).unwrap();
        assert_eq!(p, decoded);
    }

    #[test]
    fn profile_serialization_roundtrip_basic_auth() {
        let p = ConnectionProfile {
            name: "prod".into(),
            host: "flight.example.com".into(),
            port: 443,
            tls_enabled: true,
            auth: AuthMethod::Basic {
                username: "admin".into(),
                password: "secret".into(),
            },
        };
        let toml_str = toml::to_string_pretty(&p).unwrap();
        let decoded: ConnectionProfile = toml::from_str(&toml_str).unwrap();
        assert_eq!(p, decoded);
    }

    #[test]
    fn profile_serialization_roundtrip_bearer_auth() {
        let p = ConnectionProfile {
            auth: AuthMethod::BearerToken {
                token: "abc123".into(),
            },
            ..Default::default()
        };
        let toml_str = toml::to_string_pretty(&p).unwrap();
        let decoded: ConnectionProfile = toml::from_str(&toml_str).unwrap();
        assert_eq!(p, decoded);
    }

    #[test]
    fn manager_upsert_and_get() {
        let mut mgr = ConnectionManager::default();
        assert!(mgr.get("local").is_none());

        mgr.upsert(ConnectionProfile::default());
        assert!(mgr.get("local").is_some());
        assert_eq!(mgr.profiles.len(), 1);

        // Upsert updates existing by name
        let updated = ConnectionProfile {
            port: 9999,
            ..Default::default()
        };
        mgr.upsert(updated);
        assert_eq!(mgr.profiles.len(), 1);
        assert_eq!(mgr.get("local").unwrap().port, 9999);
    }

    #[test]
    fn manager_remove() {
        let mut mgr = ConnectionManager::default();
        mgr.upsert(ConnectionProfile::default());
        assert!(mgr.remove("local"));
        assert!(!mgr.remove("local"));
        assert!(mgr.profiles.is_empty());
    }

    #[test]
    fn manager_names() {
        let mut mgr = ConnectionManager::default();
        mgr.upsert(ConnectionProfile::default());
        mgr.upsert(ConnectionProfile {
            name: "prod".into(),
            ..Default::default()
        });
        let names = mgr.names();
        assert_eq!(names, vec!["local", "prod"]);
    }

    #[test]
    fn manager_serialization_roundtrip() {
        let mut mgr = ConnectionManager::default();
        mgr.upsert(ConnectionProfile::default());
        mgr.upsert(ConnectionProfile {
            name: "prod".into(),
            host: "flight.example.com".into(),
            port: 443,
            tls_enabled: true,
            auth: AuthMethod::Basic {
                username: "admin".into(),
                password: "secret".into(),
            },
        });

        let toml_str = toml::to_string_pretty(&mgr).unwrap();
        let decoded: ConnectionManager = toml::from_str(&toml_str).unwrap();
        assert_eq!(decoded.profiles.len(), 2);
        assert_eq!(decoded.get("prod").unwrap().port, 443);
    }
}
