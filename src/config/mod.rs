use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::keybindings::KeyMode;
use crate::theme::ThemeKind;

/// Top-level configuration, loadable from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub key_mode: KeyMode,
    pub theme: ThemeKind,
    pub editor: EditorConfig,
    pub results: ResultsConfig,
    pub memory_budget_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub tab_width: usize,
    pub expand_tab: bool,
    pub show_line_numbers: bool,
    pub word_wrap: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ResultsConfig {
    pub max_column_width: usize,
    pub thousands_separator: bool,
    pub null_display: String,
    pub float_precision: usize,
    pub max_materialized_rows: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            key_mode: KeyMode::Normal,
            theme: ThemeKind::TokyoNight,
            editor: EditorConfig::default(),
            results: ResultsConfig::default(),
            memory_budget_mb: 2048,
        }
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_width: 4,
            expand_tab: true,
            show_line_numbers: true,
            word_wrap: false,
        }
    }
}

impl Default for ResultsConfig {
    fn default() -> Self {
        Self {
            max_column_width: 48,
            thousands_separator: true,
            null_display: "NULL".into(),
            float_precision: 4,
            max_materialized_rows: 1_000_000,
        }
    }
}

impl Config {
    /// Standard config directory: ~/.config/quiver/
    pub fn config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("quiver"))
    }

    /// Attempt to load config from ~/.config/quiver/config.toml.
    /// Falls back to defaults if file doesn't exist or is malformed.
    pub fn load() -> Self {
        let path = match Self::config_dir() {
            Some(d) => d.join("config.toml"),
            None => return Self::default(),
        };

        match std::fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save current config to disk.
    #[allow(dead_code)]
    pub fn save(&self) -> anyhow::Result<()> {
        let dir = Self::config_dir().ok_or_else(|| anyhow::anyhow!("No config directory"))?;
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("config.toml");
        let toml_str = toml::to_string_pretty(self)?;
        std::fs::write(path, toml_str)?;
        Ok(())
    }
}
