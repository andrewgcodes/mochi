//! Configuration for Mochi Terminal
//!
//! Configuration is loaded with the following precedence (highest to lowest):
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

/// Configuration error type
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("Invalid configuration: {0}")]
    ValidationError(String),
    #[error("Invalid theme name: {0}")]
    InvalidTheme(String),
    #[error("Invalid color format: {0}")]
    InvalidColor(String),
}

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

    /// Font size in points
    #[arg(long, value_name = "SIZE")]
    pub font_size: Option<f32>,

    /// Theme name (dark, light, solarized-dark, solarized-light, dracula, nord, monokai, gruvbox-dark)
    #[arg(short, long, value_name = "THEME")]
    pub theme: Option<String>,

    /// Shell command to run
    #[arg(long, value_name = "SHELL")]
    pub shell: Option<String>,

    /// Number of scrollback lines
    #[arg(long, value_name = "LINES")]
    pub scrollback: Option<usize>,

    /// Enable OSC 52 clipboard support (disabled by default for security)
    #[arg(long)]
    pub osc52_clipboard: bool,

    /// Initial window columns
    #[arg(long, value_name = "COLS")]
    pub cols: Option<u16>,

    /// Initial window rows
    #[arg(long, value_name = "ROWS")]
    pub rows: Option<u16>,
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
    /// Monokai theme
    Monokai,
    /// Gruvbox Dark theme
    #[serde(rename = "gruvbox-dark")]
    GruvboxDark,
    /// Custom theme (uses colors field)
    Custom,
}

impl FromStr for ThemeName {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dark" => Ok(ThemeName::Dark),
            "light" => Ok(ThemeName::Light),
            "solarized-dark" | "solarizeddark" => Ok(ThemeName::SolarizedDark),
            "solarized-light" | "solarizedlight" => Ok(ThemeName::SolarizedLight),
            "dracula" => Ok(ThemeName::Dracula),
            "nord" => Ok(ThemeName::Nord),
            "monokai" => Ok(ThemeName::Monokai),
            "gruvbox-dark" | "gruvboxdark" | "gruvbox" => Ok(ThemeName::GruvboxDark),
            "custom" => Ok(ThemeName::Custom),
            _ => Err(ConfigError::InvalidTheme(s.to_string())),
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
            ThemeName::Monokai => write!(f, "monokai"),
            ThemeName::GruvboxDark => write!(f, "gruvbox-dark"),
            ThemeName::Custom => write!(f, "custom"),
        }
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
            ThemeName::Monokai => ColorScheme::monokai(),
            ThemeName::GruvboxDark => ColorScheme::gruvbox_dark(),
        }
    }

    /// Load configuration with full precedence:
    /// CLI args > Environment variables > Config file > Defaults
    pub fn load_with_args(args: &CliArgs) -> Result<Self, ConfigError> {
        let mut config = Config::default();

        let config_path = args.config.clone().or_else(Self::default_config_path);
        if let Some(path) = &config_path {
            if path.exists() {
                match Self::load_from_path(path) {
                    Ok(file_config) => {
                        config = file_config;
                    }
                    Err(e) => {
                        log::warn!("Failed to load config from {:?}: {}", path, e);
                    }
                }
            }
        }

        config.apply_env_vars();
        config.apply_cli_args(args)?;
        config.validate()?;

        Ok(config)
    }

    /// Load configuration from a specific path
    pub fn load_from_path(path: &PathBuf) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Apply environment variables to config
    fn apply_env_vars(&mut self) {
        if let Ok(val) = env::var("MOCHI_FONT_SIZE") {
            if let Ok(size) = val.parse::<f32>() {
                self.font_size = size;
            }
        }

        if let Ok(val) = env::var("MOCHI_THEME") {
            if let Ok(theme) = ThemeName::from_str(&val) {
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

        if let Ok(val) = env::var("MOCHI_OSC52_CLIPBOARD") {
            self.osc52_clipboard = val == "1" || val.to_lowercase() == "true";
        }

        if let Ok(val) = env::var("MOCHI_FONT_FAMILY") {
            self.font_family = val;
        }
    }

    /// Apply CLI arguments to config
    fn apply_cli_args(&mut self, args: &CliArgs) -> Result<(), ConfigError> {
        if let Some(size) = args.font_size {
            self.font_size = size;
        }

        if let Some(ref theme_str) = args.theme {
            self.theme = ThemeName::from_str(theme_str)?;
        }

        if let Some(ref shell) = args.shell {
            self.shell = Some(shell.clone());
        }

        if let Some(lines) = args.scrollback {
            self.scrollback_lines = lines;
        }

        if args.osc52_clipboard {
            self.osc52_clipboard = true;
        }

        if let Some(cols) = args.cols {
            self.dimensions.0 = cols;
        }

        if let Some(rows) = args.rows {
            self.dimensions.1 = rows;
        }

        Ok(())
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.font_size < 6.0 || self.font_size > 128.0 {
            return Err(ConfigError::ValidationError(format!(
                "Font size must be between 6 and 128, got {}",
                self.font_size
            )));
        }

        if self.scrollback_lines > 1_000_000 {
            return Err(ConfigError::ValidationError(format!(
                "Scrollback lines must be at most 1,000,000, got {}",
                self.scrollback_lines
            )));
        }

        if self.dimensions.0 < 10 || self.dimensions.0 > 1000 {
            return Err(ConfigError::ValidationError(format!(
                "Columns must be between 10 and 1000, got {}",
                self.dimensions.0
            )));
        }
        if self.dimensions.1 < 5 || self.dimensions.1 > 500 {
            return Err(ConfigError::ValidationError(format!(
                "Rows must be between 5 and 500, got {}",
                self.dimensions.1
            )));
        }

        if self.osc52_max_size > 10_000_000 {
            return Err(ConfigError::ValidationError(format!(
                "OSC 52 max size must be at most 10,000,000, got {}",
                self.osc52_max_size
            )));
        }

        if self.theme == ThemeName::Custom {
            self.colors.validate()?;
        }

        Ok(())
    }

    /// Get the default configuration file path (XDG compliant)
    pub fn default_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("mochi").join("config.toml"))
    }

    /// Get all available theme names
    pub fn available_themes() -> &'static [&'static str] {
        &[
            "dark",
            "light",
            "solarized-dark",
            "solarized-light",
            "dracula",
            "nord",
            "monokai",
            "gruvbox-dark",
        ]
    }

    /// Cycle to the next theme
    pub fn next_theme(&mut self) {
        self.theme = match self.theme {
            ThemeName::Dark => ThemeName::Light,
            ThemeName::Light => ThemeName::SolarizedDark,
            ThemeName::SolarizedDark => ThemeName::SolarizedLight,
            ThemeName::SolarizedLight => ThemeName::Dracula,
            ThemeName::Dracula => ThemeName::Nord,
            ThemeName::Nord => ThemeName::Monokai,
            ThemeName::Monokai => ThemeName::GruvboxDark,
            ThemeName::GruvboxDark => ThemeName::Dark,
            ThemeName::Custom => ThemeName::Dark,
        };
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
    /// Validate the color scheme
    pub fn validate(&self) -> Result<(), ConfigError> {
        Self::validate_color(&self.foreground, "foreground")?;
        Self::validate_color(&self.background, "background")?;
        Self::validate_color(&self.cursor, "cursor")?;
        Self::validate_color(&self.selection, "selection")?;

        for (i, color) in self.ansi.iter().enumerate() {
            Self::validate_color(color, &format!("ansi[{}]", i))?;
        }

        Ok(())
    }

    /// Validate a single color string
    fn validate_color(color: &str, name: &str) -> Result<(), ConfigError> {
        if Self::parse_hex(color).is_none() {
            return Err(ConfigError::InvalidColor(format!(
                "Invalid {} color: '{}' (expected hex format like #RRGGBB)",
                name, color
            )));
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

    /// Monokai theme
    pub fn monokai() -> Self {
        Self {
            foreground: "#f8f8f2".to_string(),
            background: "#272822".to_string(),
            cursor: "#f8f8f0".to_string(),
            selection: "#49483e".to_string(),
            ansi: [
                "#272822".to_string(), // Black
                "#f92672".to_string(), // Red
                "#a6e22e".to_string(), // Green
                "#f4bf75".to_string(), // Yellow
                "#66d9ef".to_string(), // Blue
                "#ae81ff".to_string(), // Magenta
                "#a1efe4".to_string(), // Cyan
                "#f8f8f2".to_string(), // White
                "#75715e".to_string(), // Bright Black
                "#f92672".to_string(), // Bright Red
                "#a6e22e".to_string(), // Bright Green
                "#f4bf75".to_string(), // Bright Yellow
                "#66d9ef".to_string(), // Bright Blue
                "#ae81ff".to_string(), // Bright Magenta
                "#a1efe4".to_string(), // Bright Cyan
                "#f9f8f5".to_string(), // Bright White
            ],
        }
    }

    /// Gruvbox Dark theme
    pub fn gruvbox_dark() -> Self {
        Self {
            foreground: "#ebdbb2".to_string(),
            background: "#282828".to_string(),
            cursor: "#ebdbb2".to_string(),
            selection: "#504945".to_string(),
            ansi: [
                "#282828".to_string(), // Black
                "#cc241d".to_string(), // Red
                "#98971a".to_string(), // Green
                "#d79921".to_string(), // Yellow
                "#458588".to_string(), // Blue
                "#b16286".to_string(), // Magenta
                "#689d6a".to_string(), // Cyan
                "#a89984".to_string(), // White
                "#928374".to_string(), // Bright Black
                "#fb4934".to_string(), // Bright Red
                "#b8bb26".to_string(), // Bright Green
                "#fabd2f".to_string(), // Bright Yellow
                "#83a598".to_string(), // Bright Blue
                "#d3869b".to_string(), // Bright Magenta
                "#8ec07c".to_string(), // Bright Cyan
                "#ebdbb2".to_string(), // Bright White
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
    }

    #[test]
    fn test_color_scheme_default() {
        let scheme = ColorScheme::default();
        assert_eq!(scheme.ansi.len(), 16);
    }

    #[test]
    fn test_theme_from_str() {
        assert_eq!(ThemeName::from_str("dark").unwrap(), ThemeName::Dark);
        assert_eq!(ThemeName::from_str("LIGHT").unwrap(), ThemeName::Light);
        assert_eq!(
            ThemeName::from_str("solarized-dark").unwrap(),
            ThemeName::SolarizedDark
        );
        assert_eq!(ThemeName::from_str("monokai").unwrap(), ThemeName::Monokai);
        assert_eq!(
            ThemeName::from_str("gruvbox-dark").unwrap(),
            ThemeName::GruvboxDark
        );
        assert!(ThemeName::from_str("invalid").is_err());
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();

        // Valid config should pass
        assert!(config.validate().is_ok());

        // Invalid font size
        config.font_size = 2.0;
        assert!(config.validate().is_err());
        config.font_size = 14.0;

        // Invalid scrollback
        config.scrollback_lines = 2_000_000;
        assert!(config.validate().is_err());
        config.scrollback_lines = 10000;

        // Invalid dimensions
        config.dimensions = (5, 24);
        assert!(config.validate().is_err());
        config.dimensions = (80, 24);

        // Valid again
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_color_scheme_validation() {
        let mut scheme = ColorScheme::default();
        assert!(scheme.validate().is_ok());

        scheme.foreground = "invalid".to_string();
        assert!(scheme.validate().is_err());
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

        let mut config = Config::default();
        config.theme = ThemeName::Light;
        let colors = config.effective_colors();
        assert_eq!(colors.background, "#ffffff");
    }

    #[test]
    fn test_all_themes_have_valid_colors() {
        let themes = [
            ThemeName::Dark,
            ThemeName::Light,
            ThemeName::SolarizedDark,
            ThemeName::SolarizedLight,
            ThemeName::Dracula,
            ThemeName::Nord,
            ThemeName::Monokai,
            ThemeName::GruvboxDark,
        ];

        for theme in themes {
            let mut config = Config::default();
            config.theme = theme;
            let colors = config.effective_colors();
            assert!(
                colors.validate().is_ok(),
                "Theme {:?} has invalid colors",
                theme
            );
        }
    }

    #[test]
    fn test_theme_display() {
        assert_eq!(ThemeName::Dark.to_string(), "dark");
        assert_eq!(ThemeName::SolarizedDark.to_string(), "solarized-dark");
        assert_eq!(ThemeName::GruvboxDark.to_string(), "gruvbox-dark");
    }

    #[test]
    fn test_available_themes() {
        let themes = Config::available_themes();
        assert!(themes.contains(&"dark"));
        assert!(themes.contains(&"light"));
        assert!(themes.contains(&"monokai"));
        assert!(themes.contains(&"gruvbox-dark"));
    }
}
