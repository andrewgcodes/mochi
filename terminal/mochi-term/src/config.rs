//! Configuration for Mochi Terminal
//!
//! Configuration precedence (highest to lowest):
//! 1. CLI arguments
//! 2. Environment variables (MOCHI_*)
//! 3. Config file (~/.config/mochi/config.toml)
//! 4. Built-in defaults

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

/// CLI arguments for Mochi Terminal
#[derive(Parser, Debug)]
#[command(name = "mochi")]
#[command(author = "Mochi Terminal Authors")]
#[command(version)]
#[command(about = "A modern, customizable terminal emulator", long_about = None)]
pub struct CliArgs {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Theme to use (dark, light, solarized-dark, solarized-light, dracula, nord)
    #[arg(short, long)]
    pub theme: Option<String>,

    /// Font size in points
    #[arg(long)]
    pub font_size: Option<f32>,

    /// Font family name
    #[arg(long)]
    pub font_family: Option<String>,

    /// Shell command to run
    #[arg(short, long)]
    pub shell: Option<String>,

    /// Number of scrollback lines
    #[arg(long)]
    pub scrollback: Option<usize>,

    /// Initial columns
    #[arg(long)]
    pub cols: Option<u16>,

    /// Initial rows
    #[arg(long)]
    pub rows: Option<u16>,
}

impl CliArgs {
    /// Parse CLI arguments
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

/// Available theme names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeName {
    /// Dark theme (default) - also known as mochi-dark
    #[default]
    #[serde(alias = "mochi-dark")]
    Dark,
    /// Light theme - also known as mochi-light
    #[serde(alias = "mochi-light")]
    Light,
    /// Solarized Dark
    SolarizedDark,
    /// Solarized Light
    SolarizedLight,
    /// Dracula theme
    Dracula,
    /// Nord theme
    Nord,
    /// Custom theme (uses colors field)
    Custom,
}

impl FromStr for ThemeName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dark" | "mochi-dark" => Ok(ThemeName::Dark),
            "light" | "mochi-light" => Ok(ThemeName::Light),
            "solarized-dark" | "solarizeddark" => Ok(ThemeName::SolarizedDark),
            "solarized-light" | "solarizedlight" => Ok(ThemeName::SolarizedLight),
            "dracula" => Ok(ThemeName::Dracula),
            "nord" => Ok(ThemeName::Nord),
            "custom" => Ok(ThemeName::Custom),
            _ => Err(format!(
                "Unknown theme '{}'. Valid themes: dark, light, solarized-dark, solarized-light, dracula, nord, custom",
                s
            )),
        }
    }
}

impl std::fmt::Display for ThemeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeName::Dark => write!(f, "dark"),
            ThemeName::Light => write!(f, "light"),
            ThemeName::SolarizedDark => write!(f, "solarized-dark"),
            ThemeName::SolarizedLight => write!(f, "solarized-light"),
            ThemeName::Dracula => write!(f, "dracula"),
            ThemeName::Nord => write!(f, "nord"),
            ThemeName::Custom => write!(f, "custom"),
        }
    }
}

/// Keybinding action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyAction {
    Copy,
    Paste,
    Find,
    ReloadConfig,
    ToggleTheme,
    FontSizeIncrease,
    FontSizeDecrease,
    FontSizeReset,
    ScrollUp,
    ScrollDown,
    ScrollPageUp,
    ScrollPageDown,
    ScrollToTop,
    ScrollToBottom,
}

/// Keybinding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybinding {
    /// Key combination (e.g., "ctrl+shift+c")
    pub key: String,
    /// Action to perform
    pub action: KeyAction,
}

/// Keybindings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    /// List of keybindings
    #[serde(default = "KeybindingsConfig::default_bindings")]
    pub bindings: Vec<Keybinding>,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            bindings: Self::default_bindings(),
        }
    }
}

impl KeybindingsConfig {
    fn default_bindings() -> Vec<Keybinding> {
        vec![
            Keybinding {
                key: "ctrl+shift+c".to_string(),
                action: KeyAction::Copy,
            },
            Keybinding {
                key: "ctrl+shift+v".to_string(),
                action: KeyAction::Paste,
            },
            Keybinding {
                key: "ctrl+shift+f".to_string(),
                action: KeyAction::Find,
            },
            Keybinding {
                key: "ctrl+shift+r".to_string(),
                action: KeyAction::ReloadConfig,
            },
            Keybinding {
                key: "ctrl+shift+t".to_string(),
                action: KeyAction::ToggleTheme,
            },
            Keybinding {
                key: "ctrl+equal".to_string(),
                action: KeyAction::FontSizeIncrease,
            },
            Keybinding {
                key: "ctrl+plus".to_string(),
                action: KeyAction::FontSizeIncrease,
            },
            Keybinding {
                key: "ctrl+minus".to_string(),
                action: KeyAction::FontSizeDecrease,
            },
            Keybinding {
                key: "ctrl+0".to_string(),
                action: KeyAction::FontSizeReset,
            },
        ]
    }
}

/// Terminal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Font family name
    #[serde(default = "default_font_family")]
    pub font_family: String,
    /// Font size in points
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    /// Number of scrollback lines
    #[serde(default = "default_scrollback_lines")]
    pub scrollback_lines: usize,
    /// Window dimensions (columns, rows)
    #[serde(default = "default_dimensions")]
    pub dimensions: (u16, u16),
    /// Theme name (dark, light, solarized-dark, solarized-light, dracula, nord, custom)
    #[serde(default)]
    pub theme: ThemeName,
    /// Color scheme (used when theme is "custom", otherwise ignored)
    #[serde(default)]
    pub colors: ColorScheme,
    /// Enable OSC 52 clipboard
    #[serde(default)]
    pub osc52_clipboard: bool,
    /// Maximum OSC 52 payload size
    #[serde(default = "default_osc52_max_size")]
    pub osc52_max_size: usize,
    /// Shell command (None = use $SHELL)
    #[serde(default)]
    pub shell: Option<String>,
    /// Cursor style
    #[serde(default = "default_cursor_style")]
    pub cursor_style: String,
    /// Cursor blink
    #[serde(default = "default_cursor_blink")]
    pub cursor_blink: bool,
    /// Keybindings configuration
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
}

fn default_font_family() -> String {
    "monospace".to_string()
}

fn default_font_size() -> f32 {
    14.0
}

fn default_scrollback_lines() -> usize {
    10000
}

fn default_dimensions() -> (u16, u16) {
    (80, 24)
}

fn default_osc52_max_size() -> usize {
    100000
}

fn default_cursor_style() -> String {
    "block".to_string()
}

fn default_cursor_blink() -> bool {
    true
}

/// Color scheme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    /// Foreground color (hex)
    pub foreground: String,
    /// Background color (hex)
    pub background: String,
    /// Cursor color (hex)
    pub cursor: String,
    /// Selection color (hex)
    pub selection: String,
    /// ANSI colors 0-15 (hex)
    pub ansi: [String; 16],
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_family: default_font_family(),
            font_size: default_font_size(),
            scrollback_lines: default_scrollback_lines(),
            dimensions: default_dimensions(),
            theme: ThemeName::Dark,
            colors: ColorScheme::default(),
            osc52_clipboard: false, // Disabled by default for security
            osc52_max_size: default_osc52_max_size(),
            shell: None,
            cursor_style: default_cursor_style(),
            cursor_blink: default_cursor_blink(),
            keybindings: KeybindingsConfig::default(),
        }
    }
}

/// Configuration error
#[derive(Debug)]
pub enum ConfigError {
    /// IO error reading config file
    Io(std::io::Error),
    /// TOML parsing error
    Parse(toml::de::Error),
    /// Invalid theme name
    InvalidTheme(String),
    /// Invalid config value
    InvalidValue(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "Failed to read config file: {}", e),
            ConfigError::Parse(e) => write!(f, "Failed to parse config file: {}", e),
            ConfigError::InvalidTheme(t) => write!(f, "Invalid theme: {}", t),
            ConfigError::InvalidValue(v) => write!(f, "Invalid config value: {}", v),
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// Get the effective color scheme based on the theme setting
    pub fn effective_colors(&self) -> ColorScheme {
        match self.theme {
            ThemeName::Custom => self.colors.clone(),
            ThemeName::Dark => ColorScheme::dark(),
            ThemeName::Light => ColorScheme::light(),
            ThemeName::SolarizedDark => ColorScheme::solarized_dark(),
            ThemeName::SolarizedLight => ColorScheme::solarized_light(),
            ThemeName::Dracula => ColorScheme::dracula(),
            ThemeName::Nord => ColorScheme::nord(),
        }
    }

    /// Cycle to the next theme
    pub fn next_theme(&mut self) {
        self.theme = match self.theme {
            ThemeName::Dark => ThemeName::Light,
            ThemeName::Light => ThemeName::SolarizedDark,
            ThemeName::SolarizedDark => ThemeName::SolarizedLight,
            ThemeName::SolarizedLight => ThemeName::Dracula,
            ThemeName::Dracula => ThemeName::Nord,
            ThemeName::Nord => ThemeName::Dark,
            ThemeName::Custom => ThemeName::Dark,
        };
    }

    /// Get the list of available themes
    pub fn available_themes() -> &'static [&'static str] {
        &[
            "dark",
            "light",
            "solarized-dark",
            "solarized-light",
            "dracula",
            "nord",
        ]
    }

    /// Load configuration with full precedence:
    /// CLI args > environment variables > config file > defaults
    pub fn load_with_args(args: &CliArgs) -> Result<Self, ConfigError> {
        // Start with defaults
        let mut config = Config::default();

        // Layer 1: Load from config file
        let config_path = args
            .config
            .clone()
            .or_else(|| env::var("MOCHI_CONFIG").ok().map(PathBuf::from))
            .or_else(Self::default_config_path);

        if let Some(path) = config_path {
            if path.exists() {
                match Self::load_from_path(&path) {
                    Ok(file_config) => config = file_config,
                    Err(e) => {
                        log::warn!("Failed to load config from {:?}: {}", path, e);
                        // Continue with defaults
                    }
                }
            }
        }

        // Layer 2: Apply environment variables
        config.apply_env_vars();

        // Layer 3: Apply CLI arguments (highest priority)
        config.apply_cli_args(args)?;

        Ok(config)
    }

    /// Load configuration from a specific path
    pub fn load_from_path(path: &PathBuf) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path).map_err(ConfigError::Io)?;
        toml::from_str(&content).map_err(ConfigError::Parse)
    }

    /// Apply environment variables to config
    fn apply_env_vars(&mut self) {
        if let Ok(theme) = env::var("MOCHI_THEME") {
            if let Ok(t) = ThemeName::from_str(&theme) {
                self.theme = t;
            } else {
                log::warn!("Invalid MOCHI_THEME value: {}", theme);
            }
        }

        if let Ok(font_size) = env::var("MOCHI_FONT_SIZE") {
            if let Ok(size) = font_size.parse::<f32>() {
                if (6.0..=72.0).contains(&size) {
                    self.font_size = size;
                } else {
                    log::warn!("MOCHI_FONT_SIZE out of range (6-72): {}", size);
                }
            }
        }

        if let Ok(font_family) = env::var("MOCHI_FONT_FAMILY") {
            self.font_family = font_family;
        }

        if let Ok(shell) = env::var("MOCHI_SHELL") {
            self.shell = Some(shell);
        }

        if let Ok(scrollback) = env::var("MOCHI_SCROLLBACK") {
            if let Ok(lines) = scrollback.parse::<usize>() {
                self.scrollback_lines = lines;
            }
        }

        if let Ok(osc52) = env::var("MOCHI_OSC52_CLIPBOARD") {
            self.osc52_clipboard = osc52 == "1" || osc52.to_lowercase() == "true";
        }
    }

    /// Apply CLI arguments to config
    fn apply_cli_args(&mut self, args: &CliArgs) -> Result<(), ConfigError> {
        if let Some(ref theme) = args.theme {
            self.theme =
                ThemeName::from_str(theme).map_err(|e| ConfigError::InvalidTheme(e.to_string()))?;
        }

        if let Some(font_size) = args.font_size {
            if !(6.0..=72.0).contains(&font_size) {
                return Err(ConfigError::InvalidValue(format!(
                    "Font size must be between 6 and 72, got {}",
                    font_size
                )));
            }
            self.font_size = font_size;
        }

        if let Some(ref font_family) = args.font_family {
            self.font_family = font_family.clone();
        }

        if let Some(ref shell) = args.shell {
            self.shell = Some(shell.clone());
        }

        if let Some(scrollback) = args.scrollback {
            self.scrollback_lines = scrollback;
        }

        if let Some(cols) = args.cols {
            self.dimensions.0 = cols;
        }

        if let Some(rows) = args.rows {
            self.dimensions.1 = rows;
        }

        Ok(())
    }

    /// Get the default configuration file path (XDG compliant)
    pub fn default_config_path() -> Option<PathBuf> {
        // First check XDG_CONFIG_HOME
        if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg_config).join("mochi").join("config.toml"));
        }

        // Fall back to ~/.config/mochi/config.toml
        dirs::config_dir().map(|p| p.join("mochi").join("config.toml"))
    }

    /// Reload configuration from file
    pub fn reload(&mut self) -> Result<(), ConfigError> {
        let path = Self::default_config_path().ok_or_else(|| {
            ConfigError::InvalidValue("Could not determine config path".to_string())
        })?;

        if path.exists() {
            let new_config = Self::load_from_path(&path)?;
            *self = new_config;
        }

        Ok(())
    }
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            foreground: "#d4d4d4".to_string(),
            background: "#1e1e1e".to_string(),
            cursor: "#ffffff".to_string(),
            selection: "#264f78".to_string(),
            ansi: [
                "#000000".to_string(), // Black
                "#cd3131".to_string(), // Red
                "#0dbc79".to_string(), // Green
                "#e5e510".to_string(), // Yellow
                "#2472c8".to_string(), // Blue
                "#bc3fbc".to_string(), // Magenta
                "#11a8cd".to_string(), // Cyan
                "#e5e5e5".to_string(), // White
                "#666666".to_string(), // Bright Black
                "#f14c4c".to_string(), // Bright Red
                "#23d18b".to_string(), // Bright Green
                "#f5f543".to_string(), // Bright Yellow
                "#3b8eea".to_string(), // Bright Blue
                "#d670d6".to_string(), // Bright Magenta
                "#29b8db".to_string(), // Bright Cyan
                "#ffffff".to_string(), // Bright White
            ],
        }
    }
}

impl Config {
    /// Load configuration from file (legacy method for compatibility)
    pub fn load() -> Option<Self> {
        let config_path = Self::default_config_path()?;

        if !config_path.exists() {
            return None;
        }

        Self::load_from_path(&config_path).ok()
    }

    /// Save configuration to file
    #[allow(dead_code)]
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::default_config_path().ok_or("Could not determine config path")?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;

        Ok(())
    }
}

impl ColorScheme {
    /// Dark theme (VS Code inspired)
    pub fn dark() -> Self {
        Self::default()
    }

    /// Light theme
    pub fn light() -> Self {
        Self {
            foreground: "#333333".to_string(),
            background: "#ffffff".to_string(),
            cursor: "#000000".to_string(),
            selection: "#add6ff".to_string(),
            ansi: [
                "#000000".to_string(), // Black
                "#cd3131".to_string(), // Red
                "#00bc00".to_string(), // Green
                "#949800".to_string(), // Yellow
                "#0451a5".to_string(), // Blue
                "#bc05bc".to_string(), // Magenta
                "#0598bc".to_string(), // Cyan
                "#555555".to_string(), // White
                "#666666".to_string(), // Bright Black
                "#cd3131".to_string(), // Bright Red
                "#14ce14".to_string(), // Bright Green
                "#b5ba00".to_string(), // Bright Yellow
                "#0451a5".to_string(), // Bright Blue
                "#bc05bc".to_string(), // Bright Magenta
                "#0598bc".to_string(), // Bright Cyan
                "#a5a5a5".to_string(), // Bright White
            ],
        }
    }

    /// Solarized Dark theme
    pub fn solarized_dark() -> Self {
        Self {
            foreground: "#839496".to_string(),
            background: "#002b36".to_string(),
            cursor: "#93a1a1".to_string(),
            selection: "#073642".to_string(),
            ansi: [
                "#073642".to_string(), // Black
                "#dc322f".to_string(), // Red
                "#859900".to_string(), // Green
                "#b58900".to_string(), // Yellow
                "#268bd2".to_string(), // Blue
                "#d33682".to_string(), // Magenta
                "#2aa198".to_string(), // Cyan
                "#eee8d5".to_string(), // White
                "#002b36".to_string(), // Bright Black
                "#cb4b16".to_string(), // Bright Red
                "#586e75".to_string(), // Bright Green
                "#657b83".to_string(), // Bright Yellow
                "#839496".to_string(), // Bright Blue
                "#6c71c4".to_string(), // Bright Magenta
                "#93a1a1".to_string(), // Bright Cyan
                "#fdf6e3".to_string(), // Bright White
            ],
        }
    }

    /// Solarized Light theme
    pub fn solarized_light() -> Self {
        Self {
            foreground: "#657b83".to_string(),
            background: "#fdf6e3".to_string(),
            cursor: "#586e75".to_string(),
            selection: "#eee8d5".to_string(),
            ansi: [
                "#073642".to_string(), // Black
                "#dc322f".to_string(), // Red
                "#859900".to_string(), // Green
                "#b58900".to_string(), // Yellow
                "#268bd2".to_string(), // Blue
                "#d33682".to_string(), // Magenta
                "#2aa198".to_string(), // Cyan
                "#eee8d5".to_string(), // White
                "#002b36".to_string(), // Bright Black
                "#cb4b16".to_string(), // Bright Red
                "#586e75".to_string(), // Bright Green
                "#657b83".to_string(), // Bright Yellow
                "#839496".to_string(), // Bright Blue
                "#6c71c4".to_string(), // Bright Magenta
                "#93a1a1".to_string(), // Bright Cyan
                "#fdf6e3".to_string(), // Bright White
            ],
        }
    }

    /// Dracula theme
    pub fn dracula() -> Self {
        Self {
            foreground: "#f8f8f2".to_string(),
            background: "#282a36".to_string(),
            cursor: "#f8f8f2".to_string(),
            selection: "#44475a".to_string(),
            ansi: [
                "#21222c".to_string(), // Black
                "#ff5555".to_string(), // Red
                "#50fa7b".to_string(), // Green
                "#f1fa8c".to_string(), // Yellow
                "#bd93f9".to_string(), // Blue
                "#ff79c6".to_string(), // Magenta
                "#8be9fd".to_string(), // Cyan
                "#f8f8f2".to_string(), // White
                "#6272a4".to_string(), // Bright Black
                "#ff6e6e".to_string(), // Bright Red
                "#69ff94".to_string(), // Bright Green
                "#ffffa5".to_string(), // Bright Yellow
                "#d6acff".to_string(), // Bright Blue
                "#ff92df".to_string(), // Bright Magenta
                "#a4ffff".to_string(), // Bright Cyan
                "#ffffff".to_string(), // Bright White
            ],
        }
    }

    /// Nord theme
    pub fn nord() -> Self {
        Self {
            foreground: "#d8dee9".to_string(),
            background: "#2e3440".to_string(),
            cursor: "#d8dee9".to_string(),
            selection: "#434c5e".to_string(),
            ansi: [
                "#3b4252".to_string(), // Black
                "#bf616a".to_string(), // Red
                "#a3be8c".to_string(), // Green
                "#ebcb8b".to_string(), // Yellow
                "#81a1c1".to_string(), // Blue
                "#b48ead".to_string(), // Magenta
                "#88c0d0".to_string(), // Cyan
                "#e5e9f0".to_string(), // White
                "#4c566a".to_string(), // Bright Black
                "#bf616a".to_string(), // Bright Red
                "#a3be8c".to_string(), // Bright Green
                "#ebcb8b".to_string(), // Bright Yellow
                "#81a1c1".to_string(), // Bright Blue
                "#b48ead".to_string(), // Bright Magenta
                "#8fbcbb".to_string(), // Bright Cyan
                "#eceff4".to_string(), // Bright White
            ],
        }
    }

    /// Parse a hex color string to RGB
    pub fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

        Some((r, g, b))
    }

    /// Get foreground color as RGB
    pub fn foreground_rgb(&self) -> (u8, u8, u8) {
        Self::parse_hex(&self.foreground).unwrap_or((212, 212, 212))
    }

    /// Get background color as RGB
    pub fn background_rgb(&self) -> (u8, u8, u8) {
        Self::parse_hex(&self.background).unwrap_or((30, 30, 30))
    }

    /// Get cursor color as RGB
    pub fn cursor_rgb(&self) -> (u8, u8, u8) {
        Self::parse_hex(&self.cursor).unwrap_or((255, 255, 255))
    }

    /// Get selection color as RGB
    pub fn selection_rgb(&self) -> (u8, u8, u8) {
        Self::parse_hex(&self.selection).unwrap_or((38, 79, 120))
    }

    /// Get ANSI color as RGB
    pub fn ansi_rgb(&self, index: usize) -> (u8, u8, u8) {
        if index < 16 {
            Self::parse_hex(&self.ansi[index]).unwrap_or((128, 128, 128))
        } else {
            (128, 128, 128)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.font_size, 14.0);
        assert_eq!(config.dimensions, (80, 24));
        assert!(!config.osc52_clipboard);
        assert!(!config.keybindings.bindings.is_empty());
    }

    #[test]
    fn test_parse_hex() {
        assert_eq!(ColorScheme::parse_hex("#ff0000"), Some((255, 0, 0)));
        assert_eq!(ColorScheme::parse_hex("00ff00"), Some((0, 255, 0)));
        assert_eq!(ColorScheme::parse_hex("#invalid"), None);
    }

    #[test]
    fn test_color_scheme_default() {
        let scheme = ColorScheme::default();
        assert_eq!(scheme.ansi.len(), 16);
    }

    #[test]
    fn test_theme_from_str() {
        assert_eq!(ThemeName::from_str("dark").unwrap(), ThemeName::Dark);
        assert_eq!(ThemeName::from_str("mochi-dark").unwrap(), ThemeName::Dark);
        assert_eq!(ThemeName::from_str("light").unwrap(), ThemeName::Light);
        assert_eq!(ThemeName::from_str("mochi-light").unwrap(), ThemeName::Light);
        assert_eq!(
            ThemeName::from_str("solarized-dark").unwrap(),
            ThemeName::SolarizedDark
        );
        assert_eq!(ThemeName::from_str("dracula").unwrap(), ThemeName::Dracula);
        assert_eq!(ThemeName::from_str("nord").unwrap(), ThemeName::Nord);
        assert!(ThemeName::from_str("invalid").is_err());
    }

    #[test]
    fn test_theme_display() {
        assert_eq!(ThemeName::Dark.to_string(), "dark");
        assert_eq!(ThemeName::Light.to_string(), "light");
        assert_eq!(ThemeName::SolarizedDark.to_string(), "solarized-dark");
    }

    #[test]
    fn test_next_theme() {
        let mut config = Config::default();
        assert_eq!(config.theme, ThemeName::Dark);

        config.next_theme();
        assert_eq!(config.theme, ThemeName::Light);

        config.next_theme();
        assert_eq!(config.theme, ThemeName::SolarizedDark);
    }

    #[test]
    fn test_effective_colors() {
        let config = Config::default();
        let colors = config.effective_colors();
        assert_eq!(colors.background, "#1e1e1e");

        let mut light_config = Config::default();
        light_config.theme = ThemeName::Light;
        let light_colors = light_config.effective_colors();
        assert_eq!(light_colors.background, "#ffffff");
    }

    #[test]
    fn test_default_keybindings() {
        let config = Config::default();
        assert!(!config.keybindings.bindings.is_empty());

        let paste_binding = config
            .keybindings
            .bindings
            .iter()
            .find(|b| b.action == KeyAction::Paste);
        assert!(paste_binding.is_some());
        assert_eq!(paste_binding.unwrap().key, "ctrl+shift+v");
    }

    #[test]
    fn test_config_path_xdg() {
        let path = Config::default_config_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("mochi"));
        assert!(path.to_string_lossy().ends_with("config.toml"));
    }

    #[test]
    fn test_available_themes() {
        let themes = Config::available_themes();
        assert!(themes.contains(&"dark"));
        assert!(themes.contains(&"light"));
        assert!(themes.contains(&"dracula"));
        assert!(themes.contains(&"nord"));
    }
}
