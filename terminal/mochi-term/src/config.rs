//! Configuration for Mochi Terminal
//!
//! Configuration is loaded with the following precedence (highest to lowest):
//! 1. CLI flags (--config, --font-size, etc.)
//! 2. Environment variables (MOCHI_CONFIG, MOCHI_FONT_SIZE, etc.)
//! 3. Config file (~/.config/mochi/config.toml or --config path)
//! 4. Built-in defaults

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during configuration loading
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file '{path}': {source}")]
    ReadError {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse config file '{path}': {message}")]
    ParseError { path: PathBuf, message: String },

    #[error("Invalid configuration: {0}")]
    ValidationError(String),

    #[error("Invalid color format '{color}': expected #RRGGBB hex format")]
    InvalidColor { color: String },

    #[error("Invalid font size {size}: must be between {min} and {max}")]
    InvalidFontSize { size: f32, min: f32, max: f32 },

    #[error("Invalid scrollback lines {lines}: must be between {min} and {max}")]
    InvalidScrollback { lines: usize, min: usize, max: usize },

    #[error("Invalid dimensions ({cols}, {rows}): columns must be 1-500, rows must be 1-200")]
    InvalidDimensions { cols: u16, rows: u16 },
}

/// CLI arguments for Mochi Terminal
#[derive(Parser, Debug, Clone)]
#[command(name = "mochi")]
#[command(author, version, about = "Mochi Terminal Emulator - A VT/xterm-compatible terminal")]
pub struct CliArgs {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Font size in points
    #[arg(long, value_name = "SIZE")]
    pub font_size: Option<f32>,

    /// Font family name
    #[arg(long, value_name = "FAMILY")]
    pub font_family: Option<String>,

    /// Theme name (dark, light, solarized-dark, solarized-light, dracula, nord)
    #[arg(long, value_name = "THEME")]
    pub theme: Option<String>,

    /// Shell command to run
    #[arg(long, value_name = "SHELL")]
    pub shell: Option<String>,

    /// Number of scrollback lines
    #[arg(long, value_name = "LINES")]
    pub scrollback: Option<usize>,

    /// Initial columns
    #[arg(long, value_name = "COLS")]
    pub cols: Option<u16>,

    /// Initial rows
    #[arg(long, value_name = "ROWS")]
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
#[serde(rename_all = "lowercase")]
pub enum ThemeName {
    /// Dark theme (default)
    #[default]
    Dark,
    /// Light theme
    Light,
    /// Solarized Dark
    #[serde(rename = "solarized-dark")]
    SolarizedDark,
    /// Solarized Light
    #[serde(rename = "solarized-light")]
    SolarizedLight,
    /// Dracula theme
    Dracula,
    /// Nord theme
    Nord,
    /// Custom theme (uses colors field)
    Custom,
}

impl ThemeName {
    /// Parse theme name from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            "solarized-dark" | "solarizeddark" => Some(Self::SolarizedDark),
            "solarized-light" | "solarizedlight" => Some(Self::SolarizedLight),
            "dracula" => Some(Self::Dracula),
            "nord" => Some(Self::Nord),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    /// Get all available theme names
    pub fn all_names() -> &'static [&'static str] {
        &[
            "dark",
            "light",
            "solarized-dark",
            "solarized-light",
            "dracula",
            "nord",
        ]
    }
}

/// Terminal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Font family name
    pub font_family: String,
    /// Font size in points
    pub font_size: f32,
    /// Number of scrollback lines
    pub scrollback_lines: usize,
    /// Window dimensions (columns, rows)
    pub dimensions: (u16, u16),
    /// Theme name (dark, light, solarized-dark, solarized-light, dracula, nord, custom)
    #[serde(default)]
    pub theme: ThemeName,
    /// Color scheme (used when theme is "custom", otherwise ignored)
    pub colors: ColorScheme,
    /// Enable OSC 52 clipboard
    pub osc52_clipboard: bool,
    /// Maximum OSC 52 payload size
    pub osc52_max_size: usize,
    /// Shell command (None = use $SHELL)
    pub shell: Option<String>,
    /// Cursor style
    pub cursor_style: String,
    /// Cursor blink
    pub cursor_blink: bool,
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
            font_family: "monospace".to_string(),
            font_size: 14.0,
            scrollback_lines: 10000,
            dimensions: (80, 24),
            theme: ThemeName::Dark,
            colors: ColorScheme::default(),
            osc52_clipboard: false, // Disabled by default for security
            osc52_max_size: 100000,
            shell: None,
            cursor_style: "block".to_string(),
            cursor_blink: true,
        }
    }
}

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

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate font size
        const MIN_FONT_SIZE: f32 = 6.0;
        const MAX_FONT_SIZE: f32 = 128.0;
        if self.font_size < MIN_FONT_SIZE || self.font_size > MAX_FONT_SIZE {
            return Err(ConfigError::InvalidFontSize {
                size: self.font_size,
                min: MIN_FONT_SIZE,
                max: MAX_FONT_SIZE,
            });
        }

        // Validate scrollback lines
        const MAX_SCROLLBACK: usize = 1_000_000;
        if self.scrollback_lines > MAX_SCROLLBACK {
            return Err(ConfigError::InvalidScrollback {
                lines: self.scrollback_lines,
                min: 0,
                max: MAX_SCROLLBACK,
            });
        }

        // Validate dimensions
        let (cols, rows) = self.dimensions;
        if cols == 0 || cols > 500 || rows == 0 || rows > 200 {
            return Err(ConfigError::InvalidDimensions { cols, rows });
        }

        // Validate colors
        self.colors.validate()?;

        Ok(())
    }

    /// Load configuration with full precedence handling
    ///
    /// Precedence (highest to lowest):
    /// 1. CLI flags
    /// 2. Environment variables
    /// 3. Config file
    /// 4. Built-in defaults
    pub fn load_with_args(args: &CliArgs) -> Result<Self, ConfigError> {
        // Start with defaults
        let mut config = Config::default();

        // Determine config file path
        let config_path = Self::resolve_config_path(args);

        // Load from config file if it exists
        if let Some(path) = &config_path {
            if path.exists() {
                config = Self::load_from_file(path)?;
            }
        }

        // Apply environment variables
        config.apply_env_vars();

        // Apply CLI arguments (highest priority)
        config.apply_cli_args(args);

        // Validate final configuration
        config.validate()?;

        Ok(config)
    }

    /// Resolve the config file path based on CLI args and environment
    fn resolve_config_path(args: &CliArgs) -> Option<PathBuf> {
        // CLI --config flag takes highest priority
        if let Some(path) = &args.config {
            return Some(path.clone());
        }

        // Check MOCHI_CONFIG environment variable
        if let Ok(path) = env::var("MOCHI_CONFIG") {
            return Some(PathBuf::from(path));
        }

        // Default XDG config path
        Self::default_config_path()
    }

    /// Get the default configuration file path (XDG convention)
    pub fn default_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("mochi").join("config.toml"))
    }

    /// Load configuration from a specific file
    pub fn load_from_file(path: &PathBuf) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path).map_err(|e| ConfigError::ReadError {
            path: path.clone(),
            source: e,
        })?;

        toml::from_str(&content).map_err(|e| ConfigError::ParseError {
            path: path.clone(),
            message: e.to_string(),
        })
    }

    /// Apply environment variables to configuration
    fn apply_env_vars(&mut self) {
        if let Ok(val) = env::var("MOCHI_FONT_SIZE") {
            if let Ok(size) = val.parse::<f32>() {
                self.font_size = size;
            }
        }

        if let Ok(val) = env::var("MOCHI_FONT_FAMILY") {
            self.font_family = val;
        }

        if let Ok(val) = env::var("MOCHI_THEME") {
            if let Some(theme) = ThemeName::from_str(&val) {
                self.theme = theme;
            }
        }

        if let Ok(val) = env::var("MOCHI_SHELL") {
            self.shell = Some(val);
        }

        if let Ok(val) = env::var("MOCHI_SCROLLBACK") {
            if let Ok(lines) = val.parse::<usize>() {
                self.scrollback_lines = lines;
            }
        }

        if let Ok(val) = env::var("MOCHI_COLS") {
            if let Ok(cols) = val.parse::<u16>() {
                self.dimensions.0 = cols;
            }
        }

        if let Ok(val) = env::var("MOCHI_ROWS") {
            if let Ok(rows) = val.parse::<u16>() {
                self.dimensions.1 = rows;
            }
        }

        if let Ok(val) = env::var("MOCHI_OSC52_CLIPBOARD") {
            self.osc52_clipboard = val == "1" || val.to_lowercase() == "true";
        }
    }

    /// Apply CLI arguments to configuration
    fn apply_cli_args(&mut self, args: &CliArgs) {
        if let Some(size) = args.font_size {
            self.font_size = size;
        }

        if let Some(family) = &args.font_family {
            self.font_family = family.clone();
        }

        if let Some(theme_str) = &args.theme {
            if let Some(theme) = ThemeName::from_str(theme_str) {
                self.theme = theme;
            }
        }

        if let Some(shell) = &args.shell {
            self.shell = Some(shell.clone());
        }

        if let Some(lines) = args.scrollback {
            self.scrollback_lines = lines;
        }

        if let Some(cols) = args.cols {
            self.dimensions.0 = cols;
        }

        if let Some(rows) = args.rows {
            self.dimensions.1 = rows;
        }
    }

    /// Reload configuration from file
    pub fn reload(&mut self, args: &CliArgs) -> Result<(), ConfigError> {
        let new_config = Self::load_with_args(args)?;
        *self = new_config;
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
    /// Load configuration from file
    pub fn load() -> Option<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return None;
        }

        let content = fs::read_to_string(&config_path).ok()?;
        toml::from_str(&content).ok()
    }

    /// Save configuration to file
    #[allow(dead_code)]
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_path().ok_or("Could not determine config path")?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;

        Ok(())
    }

    /// Get the configuration file path
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("mochi").join("config.toml"))
    }
}

impl ColorScheme {
    /// Validate all colors in the scheme
    pub fn validate(&self) -> Result<(), ConfigError> {
        Self::validate_color(&self.foreground)?;
        Self::validate_color(&self.background)?;
        Self::validate_color(&self.cursor)?;
        Self::validate_color(&self.selection)?;

        for color in &self.ansi {
            Self::validate_color(color)?;
        }

        Ok(())
    }

    /// Validate a single color string
    fn validate_color(color: &str) -> Result<(), ConfigError> {
        if Self::parse_hex(color).is_none() {
            return Err(ConfigError::InvalidColor {
                color: color.to_string(),
            });
        }
        Ok(())
    }

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
    }

    #[test]
    fn test_parse_hex() {
        assert_eq!(ColorScheme::parse_hex("#ff0000"), Some((255, 0, 0)));
        assert_eq!(ColorScheme::parse_hex("00ff00"), Some((0, 255, 0)));
        assert_eq!(ColorScheme::parse_hex("#invalid"), None);
        assert_eq!(ColorScheme::parse_hex("#fff"), None); // Too short
        assert_eq!(ColorScheme::parse_hex("#fffffff"), None); // Too long
    }

    #[test]
    fn test_color_scheme_default() {
        let scheme = ColorScheme::default();
        assert_eq!(scheme.ansi.len(), 16);
    }

    #[test]
    fn test_theme_name_from_str() {
        assert_eq!(ThemeName::from_str("dark"), Some(ThemeName::Dark));
        assert_eq!(ThemeName::from_str("DARK"), Some(ThemeName::Dark));
        assert_eq!(ThemeName::from_str("light"), Some(ThemeName::Light));
        assert_eq!(
            ThemeName::from_str("solarized-dark"),
            Some(ThemeName::SolarizedDark)
        );
        assert_eq!(
            ThemeName::from_str("solarizeddark"),
            Some(ThemeName::SolarizedDark)
        );
        assert_eq!(ThemeName::from_str("dracula"), Some(ThemeName::Dracula));
        assert_eq!(ThemeName::from_str("nord"), Some(ThemeName::Nord));
        assert_eq!(ThemeName::from_str("invalid"), None);
    }

    #[test]
    fn test_config_validation_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_font_size() {
        let mut config = Config::default();
        config.font_size = 5.0; // Too small
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidFontSize { .. })
        ));

        config.font_size = 200.0; // Too large
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidFontSize { .. })
        ));
    }

    #[test]
    fn test_config_validation_invalid_dimensions() {
        let mut config = Config::default();
        config.dimensions = (0, 24); // Invalid cols
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidDimensions { .. })
        ));

        config.dimensions = (80, 0); // Invalid rows
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidDimensions { .. })
        ));
    }

    #[test]
    fn test_config_validation_invalid_color() {
        let mut config = Config::default();
        config.colors.foreground = "not-a-color".to_string();
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidColor { .. })
        ));
    }

    #[test]
    fn test_color_scheme_validation() {
        let scheme = ColorScheme::default();
        assert!(scheme.validate().is_ok());

        let mut bad_scheme = scheme.clone();
        bad_scheme.ansi[0] = "invalid".to_string();
        assert!(bad_scheme.validate().is_err());
    }

    #[test]
    fn test_cli_args_override() {
        let mut config = Config::default();
        let args = CliArgs {
            config: None,
            font_size: Some(20.0),
            font_family: Some("JetBrains Mono".to_string()),
            theme: Some("light".to_string()),
            shell: Some("/bin/zsh".to_string()),
            scrollback: Some(5000),
            cols: Some(120),
            rows: Some(40),
        };

        config.apply_cli_args(&args);

        assert_eq!(config.font_size, 20.0);
        assert_eq!(config.font_family, "JetBrains Mono");
        assert_eq!(config.theme, ThemeName::Light);
        assert_eq!(config.shell, Some("/bin/zsh".to_string()));
        assert_eq!(config.scrollback_lines, 5000);
        assert_eq!(config.dimensions, (120, 40));
    }

    #[test]
    fn test_env_var_override() {
        // Save original env vars
        let orig_font_size = env::var("MOCHI_FONT_SIZE").ok();
        let orig_theme = env::var("MOCHI_THEME").ok();

        // Set test env vars
        env::set_var("MOCHI_FONT_SIZE", "18.0");
        env::set_var("MOCHI_THEME", "nord");

        let mut config = Config::default();
        config.apply_env_vars();

        assert_eq!(config.font_size, 18.0);
        assert_eq!(config.theme, ThemeName::Nord);

        // Restore original env vars
        match orig_font_size {
            Some(v) => env::set_var("MOCHI_FONT_SIZE", v),
            None => env::remove_var("MOCHI_FONT_SIZE"),
        }
        match orig_theme {
            Some(v) => env::set_var("MOCHI_THEME", v),
            None => env::remove_var("MOCHI_THEME"),
        }
    }

    #[test]
    fn test_effective_colors() {
        let mut config = Config::default();

        config.theme = ThemeName::Dark;
        let dark = config.effective_colors();
        assert_eq!(dark.background, "#1e1e1e");

        config.theme = ThemeName::Light;
        let light = config.effective_colors();
        assert_eq!(light.background, "#ffffff");

        config.theme = ThemeName::Dracula;
        let dracula = config.effective_colors();
        assert_eq!(dracula.background, "#282a36");
    }

    #[test]
    fn test_all_themes_valid() {
        // Verify all built-in themes have valid colors
        assert!(ColorScheme::dark().validate().is_ok());
        assert!(ColorScheme::light().validate().is_ok());
        assert!(ColorScheme::solarized_dark().validate().is_ok());
        assert!(ColorScheme::solarized_light().validate().is_ok());
        assert!(ColorScheme::dracula().validate().is_ok());
        assert!(ColorScheme::nord().validate().is_ok());
    }

    #[test]
    fn test_parse_toml_config() {
        let toml_str = r##"
            font_family = "Fira Code"
            font_size = 16.0
            scrollback_lines = 5000
            dimensions = [100, 30]
            theme = "dracula"
            osc52_clipboard = false
            osc52_max_size = 50000
            cursor_style = "underline"
            cursor_blink = false

            [colors]
            foreground = "#f8f8f2"
            background = "#282a36"
            cursor = "#f8f8f2"
            selection = "#44475a"
            ansi = [
                "#21222c", "#ff5555", "#50fa7b", "#f1fa8c",
                "#bd93f9", "#ff79c6", "#8be9fd", "#f8f8f2",
                "#6272a4", "#ff6e6e", "#69ff94", "#ffffa5",
                "#d6acff", "#ff92df", "#a4ffff", "#ffffff"
            ]
        "##;

        let config: Config = toml::from_str(toml_str).expect("Failed to parse TOML");
        assert_eq!(config.font_family, "Fira Code");
        assert_eq!(config.font_size, 16.0);
        assert_eq!(config.scrollback_lines, 5000);
        assert_eq!(config.dimensions, (100, 30));
        assert_eq!(config.theme, ThemeName::Dracula);
        assert!(!config.cursor_blink);
    }

    #[test]
    fn test_config_precedence_cli_over_env() {
        // Save original env vars
        let orig_font_size = env::var("MOCHI_FONT_SIZE").ok();

        // Set env var
        env::set_var("MOCHI_FONT_SIZE", "18.0");

        let mut config = Config::default();

        // Apply env vars first
        config.apply_env_vars();
        assert_eq!(config.font_size, 18.0);

        // CLI should override env
        let args = CliArgs {
            config: None,
            font_size: Some(24.0),
            font_family: None,
            theme: None,
            shell: None,
            scrollback: None,
            cols: None,
            rows: None,
        };
        config.apply_cli_args(&args);
        assert_eq!(config.font_size, 24.0);

        // Restore original env vars
        match orig_font_size {
            Some(v) => env::set_var("MOCHI_FONT_SIZE", v),
            None => env::remove_var("MOCHI_FONT_SIZE"),
        }
    }

    #[test]
    fn test_theme_all_names() {
        let names = ThemeName::all_names();
        assert!(names.contains(&"dark"));
        assert!(names.contains(&"light"));
        assert!(names.contains(&"solarized-dark"));
        assert!(names.contains(&"solarized-light"));
        assert!(names.contains(&"dracula"));
        assert!(names.contains(&"nord"));
    }
}
