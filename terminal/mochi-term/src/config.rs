//! Configuration module for Mochi terminal emulator.
//!
//! This module provides configuration loading and management for the terminal.
//! Configuration can be loaded from a TOML file or use sensible defaults.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure for the terminal emulator.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Font configuration
    pub font: FontConfig,
    /// Color scheme configuration
    pub colors: ColorConfig,
    /// Terminal behavior configuration
    pub terminal: TerminalConfig,
    /// Security configuration
    pub security: SecurityConfig,
}

/// Font configuration options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FontConfig {
    /// Path to the font file (TTF/OTF)
    pub path: String,
    /// Font size in pixels
    pub size: f32,
    /// Cell padding in pixels
    pub cell_padding: f32,
}

impl Default for FontConfig {
    fn default() -> Self {
        FontConfig {
            path: "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf".to_string(),
            size: 16.0,
            cell_padding: 2.0,
        }
    }
}

/// Color scheme configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorConfig {
    /// Default foreground color (hex format: "#RRGGBB")
    pub foreground: String,
    /// Default background color (hex format: "#RRGGBB")
    pub background: String,
    /// Cursor color (hex format: "#RRGGBB")
    pub cursor: String,
    /// Selection background color (hex format: "#RRGGBB")
    pub selection: String,
    /// ANSI color palette (16 colors in hex format)
    pub palette: Vec<String>,
}

impl Default for ColorConfig {
    fn default() -> Self {
        ColorConfig {
            foreground: "#D4D4D4".to_string(),
            background: "#1E1E1E".to_string(),
            cursor: "#FFFFFF".to_string(),
            selection: "#264F78".to_string(),
            palette: vec![
                "#000000".to_string(), // Black
                "#CD3131".to_string(), // Red
                "#0DBC79".to_string(), // Green
                "#E5E510".to_string(), // Yellow
                "#2472C8".to_string(), // Blue
                "#BC3FBC".to_string(), // Magenta
                "#11A8CD".to_string(), // Cyan
                "#E5E5E5".to_string(), // White
                "#666666".to_string(), // Bright Black
                "#F14C4C".to_string(), // Bright Red
                "#23D18B".to_string(), // Bright Green
                "#F5F543".to_string(), // Bright Yellow
                "#3B8EEA".to_string(), // Bright Blue
                "#D670D6".to_string(), // Bright Magenta
                "#29B8DB".to_string(), // Bright Cyan
                "#FFFFFF".to_string(), // Bright White
            ],
        }
    }
}

/// Terminal behavior configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalConfig {
    /// Maximum number of scrollback lines
    pub scrollback_lines: usize,
    /// Shell command to execute (defaults to $SHELL or /bin/bash)
    pub shell: Option<String>,
    /// Initial terminal width in columns
    pub columns: u16,
    /// Initial terminal height in rows
    pub rows: u16,
    /// Enable cursor blinking
    pub cursor_blink: bool,
    /// Cursor style: "block", "underline", or "bar"
    pub cursor_style: String,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        TerminalConfig {
            scrollback_lines: 10000,
            shell: None,
            columns: 80,
            rows: 24,
            cursor_blink: false,
            cursor_style: "block".to_string(),
        }
    }
}

/// Security configuration options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// Enable OSC 52 clipboard operations (default: false for security)
    pub osc52_enabled: bool,
    /// Maximum size for OSC 52 clipboard data in bytes
    pub osc52_max_size: usize,
    /// Allow applications to set window title
    pub allow_title_change: bool,
    /// Maximum window title length
    pub max_title_length: usize,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        SecurityConfig {
            osc52_enabled: false,
            osc52_max_size: 100_000,
            allow_title_change: true,
            max_title_length: 256,
        }
    }
}

impl Config {
    /// Load configuration from the default config file location.
    ///
    /// Searches for config in the following order:
    /// 1. $XDG_CONFIG_HOME/mochi/config.toml
    /// 2. ~/.config/mochi/config.toml
    /// 3. Falls back to defaults if no config file found
    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                return Self::load_from_file(&path).unwrap_or_default();
            }
        }
        Config::default()
    }

    /// Load configuration from a specific file path.
    pub fn load_from_file(path: &PathBuf) -> Result<Self, ConfigError> {
        let contents =
            std::fs::read_to_string(path).map_err(|e| ConfigError::IoError(e.to_string()))?;
        toml::from_str(&contents).map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// Get the default config file path.
    pub fn config_path() -> Option<PathBuf> {
        if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg_config).join("mochi").join("config.toml"));
        }
        if let Ok(home) = std::env::var("HOME") {
            return Some(
                PathBuf::from(home)
                    .join(".config")
                    .join("mochi")
                    .join("config.toml"),
            );
        }
        None
    }

    /// Generate a default config file as a TOML string.
    pub fn default_config_toml() -> String {
        let config = Config::default();
        toml::to_string_pretty(&config).unwrap_or_default()
    }
}

/// Configuration loading errors.
#[derive(Debug)]
pub enum ConfigError {
    IoError(String),
    ParseError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.font.size, 16.0);
        assert_eq!(config.terminal.scrollback_lines, 10000);
        assert!(!config.security.osc52_enabled);
    }

    #[test]
    fn test_config_toml_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.font.size, config.font.size);
        assert_eq!(parsed.terminal.columns, config.terminal.columns);
    }

    #[test]
    fn test_default_config_toml() {
        let toml_str = Config::default_config_toml();
        assert!(toml_str.contains("[font]"));
        assert!(toml_str.contains("[colors]"));
        assert!(toml_str.contains("[terminal]"));
        assert!(toml_str.contains("[security]"));
    }
}
