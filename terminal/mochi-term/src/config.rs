//! Configuration for Mochi Terminal
//!
//! Configuration is loaded with the following precedence (highest to lowest):
//! 1. CLI flags (--config, --font-size, --theme, etc.)
//! 2. Environment variables (MOCHI_FONT_SIZE, MOCHI_THEME, etc.)
//! 3. Config file (~/.config/mochi/config.toml or XDG_CONFIG_HOME/mochi/config.toml)
//! 4. Built-in defaults

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("Invalid configuration: {0}")]
    ValidationError(String),
    #[error("Config file not found: {0}")]
    NotFound(PathBuf),
}

/// CLI arguments for configuration overrides
#[derive(Debug, Clone, Default)]
pub struct CliArgs {
    /// Path to config file (overrides XDG default)
    pub config_path: Option<PathBuf>,
    /// Font size override
    pub font_size: Option<f32>,
    /// Theme override
    pub theme: Option<ThemeName>,
    /// Shell command override
    pub shell: Option<String>,
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
    /// Parse a theme name from a string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dark" => Some(ThemeName::Dark),
            "light" => Some(ThemeName::Light),
            "solarized-dark" | "solarizeddark" => Some(ThemeName::SolarizedDark),
            "solarized-light" | "solarizedlight" => Some(ThemeName::SolarizedLight),
            "dracula" => Some(ThemeName::Dracula),
            "nord" => Some(ThemeName::Nord),
            "custom" => Some(ThemeName::Custom),
            _ => None,
        }
    }

    /// Get all available theme names
    pub fn all() -> &'static [ThemeName] {
        &[
            ThemeName::Dark,
            ThemeName::Light,
            ThemeName::SolarizedDark,
            ThemeName::SolarizedLight,
            ThemeName::Dracula,
            ThemeName::Nord,
        ]
    }

    /// Get the next theme in the cycle (for toggle functionality)
    pub fn next(self) -> ThemeName {
        match self {
            ThemeName::Dark => ThemeName::Light,
            ThemeName::Light => ThemeName::SolarizedDark,
            ThemeName::SolarizedDark => ThemeName::SolarizedLight,
            ThemeName::SolarizedLight => ThemeName::Dracula,
            ThemeName::Dracula => ThemeName::Nord,
            ThemeName::Nord => ThemeName::Dark,
            ThemeName::Custom => ThemeName::Dark,
        }
    }
}

/// Terminal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Font family name (currently only "monospace" is supported, uses bundled DejaVu Sans Mono)
    pub font_family: String,
    /// Font size in points
    pub font_size: f32,
    /// Line height multiplier (1.0 = normal, 1.2 = 20% extra spacing)
    #[serde(default = "default_line_height")]
    pub line_height: f32,
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

fn default_line_height() -> f32 {
    1.4
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
            line_height: 1.4,
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

        // Layer 3: Load from config file (if exists)
        let config_path = args.config_path.clone().or_else(Self::default_config_path);
        if let Some(path) = config_path {
            if path.exists() {
                let file_config = Self::load_from_path(&path)?;
                config = file_config;
            } else if args.config_path.is_some() {
                // If user explicitly specified a config path that doesn't exist, error
                return Err(ConfigError::NotFound(path));
            }
        }

        // Layer 2: Apply environment variable overrides
        config.apply_env_overrides();

        // Layer 1: Apply CLI overrides (highest precedence)
        config.apply_cli_overrides(args);

        // Validate the final configuration
        config.validate()?;

        Ok(config)
    }

    /// Load configuration from a specific file path
    pub fn load_from_path(path: &PathBuf) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from file (legacy method for backwards compatibility)
    pub fn load() -> Option<Self> {
        let config_path = Self::default_config_path()?;

        if !config_path.exists() {
            return None;
        }

        let content = fs::read_to_string(&config_path).ok()?;
        toml::from_str(&content).ok()
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

    /// Get the default configuration file path following XDG conventions
    ///
    /// Uses XDG_CONFIG_HOME if set, otherwise falls back to ~/.config
    pub fn default_config_path() -> Option<PathBuf> {
        // First check XDG_CONFIG_HOME environment variable
        if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
            let path = PathBuf::from(xdg_config);
            if path.is_absolute() {
                return Some(path.join("mochi").join("config.toml"));
            }
        }

        // Fall back to dirs crate (which also follows XDG on Linux)
        dirs::config_dir().map(|p| p.join("mochi").join("config.toml"))
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        // MOCHI_FONT_SIZE
        if let Ok(val) = env::var("MOCHI_FONT_SIZE") {
            if let Ok(size) = val.parse::<f32>() {
                self.font_size = size;
            }
        }

        // MOCHI_THEME
        if let Ok(val) = env::var("MOCHI_THEME") {
            if let Some(theme) = ThemeName::from_str(&val) {
                self.theme = theme;
            }
        }

        // MOCHI_SHELL
        if let Ok(val) = env::var("MOCHI_SHELL") {
            if !val.is_empty() {
                self.shell = Some(val);
            }
        }

        // MOCHI_SCROLLBACK_LINES
        if let Ok(val) = env::var("MOCHI_SCROLLBACK_LINES") {
            if let Ok(lines) = val.parse::<usize>() {
                self.scrollback_lines = lines;
            }
        }

        // MOCHI_OSC52_CLIPBOARD (security-sensitive, explicit opt-in)
        if let Ok(val) = env::var("MOCHI_OSC52_CLIPBOARD") {
            self.osc52_clipboard = val == "1" || val.to_lowercase() == "true";
        }
    }

    /// Apply CLI argument overrides
    fn apply_cli_overrides(&mut self, args: &CliArgs) {
        if let Some(size) = args.font_size {
            self.font_size = size;
        }
        if let Some(theme) = args.theme {
            self.theme = theme;
        }
        if let Some(ref shell) = args.shell {
            self.shell = Some(shell.clone());
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate font size
        if self.font_size < 6.0 || self.font_size > 128.0 {
            return Err(ConfigError::ValidationError(format!(
                "font_size must be between 6.0 and 128.0, got {}",
                self.font_size
            )));
        }

        // Validate line height
        if self.line_height < 1.0 || self.line_height > 3.0 {
            return Err(ConfigError::ValidationError(format!(
                "line_height must be between 1.0 and 3.0, got {}",
                self.line_height
            )));
        }

        // Validate scrollback lines
        if self.scrollback_lines > 1_000_000 {
            return Err(ConfigError::ValidationError(format!(
                "scrollback_lines must be at most 1,000,000, got {}",
                self.scrollback_lines
            )));
        }

        // Validate OSC52 max size (security limit)
        if self.osc52_max_size > 10_000_000 {
            return Err(ConfigError::ValidationError(format!(
                "osc52_max_size must be at most 10,000,000, got {}",
                self.osc52_max_size
            )));
        }

        // Validate dimensions
        if self.dimensions.0 < 1 || self.dimensions.1 < 1 {
            return Err(ConfigError::ValidationError(
                "dimensions must be at least 1x1".to_string(),
            ));
        }

        // Validate color scheme colors are valid hex
        let colors = &self.colors;
        Self::validate_hex_color(&colors.foreground, "foreground")?;
        Self::validate_hex_color(&colors.background, "background")?;
        Self::validate_hex_color(&colors.cursor, "cursor")?;
        Self::validate_hex_color(&colors.selection, "selection")?;
        for (i, color) in colors.ansi.iter().enumerate() {
            Self::validate_hex_color(color, &format!("ansi[{}]", i))?;
        }

        Ok(())
    }

    /// Validate a hex color string
    fn validate_hex_color(color: &str, field_name: &str) -> Result<(), ConfigError> {
        if ColorScheme::parse_hex(color).is_none() {
            return Err(ConfigError::ValidationError(format!(
                "Invalid hex color for {}: '{}'",
                field_name, color
            )));
        }
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
    use std::io::Write;

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
    fn test_theme_name_from_str() {
        assert_eq!(ThemeName::from_str("dark"), Some(ThemeName::Dark));
        assert_eq!(ThemeName::from_str("LIGHT"), Some(ThemeName::Light));
        assert_eq!(ThemeName::from_str("solarized-dark"), Some(ThemeName::SolarizedDark));
        assert_eq!(ThemeName::from_str("solarized-light"), Some(ThemeName::SolarizedLight));
        assert_eq!(ThemeName::from_str("dracula"), Some(ThemeName::Dracula));
        assert_eq!(ThemeName::from_str("nord"), Some(ThemeName::Nord));
        assert_eq!(ThemeName::from_str("invalid"), None);
    }

    #[test]
    fn test_theme_name_next() {
        assert_eq!(ThemeName::Dark.next(), ThemeName::Light);
        assert_eq!(ThemeName::Light.next(), ThemeName::SolarizedDark);
        assert_eq!(ThemeName::Nord.next(), ThemeName::Dark);
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
        assert!(config.validate().is_err());

        config.font_size = 200.0; // Too large
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_scrollback() {
        let mut config = Config::default();
        config.scrollback_lines = 2_000_000; // Too large
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_line_height() {
        let mut config = Config::default();
        config.line_height = 0.5; // Too small
        assert!(config.validate().is_err());

        config.line_height = 4.0; // Too large
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_default_line_height() {
        let config = Config::default();
        assert_eq!(config.line_height, 1.4);
    }

    #[test]
    fn test_config_validation_invalid_color() {
        let mut config = Config::default();
        config.colors.foreground = "not-a-color".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_cli_args_override() {
        let mut config = Config::default();
        let args = CliArgs {
            config_path: None,
            font_size: Some(20.0),
            theme: Some(ThemeName::Light),
            shell: Some("/bin/zsh".to_string()),
        };
        config.apply_cli_overrides(&args);

        assert_eq!(config.font_size, 20.0);
        assert_eq!(config.theme, ThemeName::Light);
        assert_eq!(config.shell, Some("/bin/zsh".to_string()));
    }

    #[test]
    fn test_load_from_path() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("mochi_test_config.toml");

        let config_content = concat!(
            "font_family = \"JetBrains Mono\"\n",
            "font_size = 16.0\n",
            "scrollback_lines = 5000\n",
            "dimensions = [120, 40]\n",
            "theme = \"light\"\n",
            "osc52_clipboard = false\n",
            "osc52_max_size = 50000\n",
            "cursor_style = \"underline\"\n",
            "cursor_blink = false\n",
            "\n",
            "[colors]\n",
            "foreground = \"#333333\"\n",
            "background = \"#ffffff\"\n",
            "cursor = \"#000000\"\n",
            "selection = \"#add6ff\"\n",
            "ansi = [\n",
            "    \"#000000\", \"#cd3131\", \"#00bc00\", \"#949800\",\n",
            "    \"#0451a5\", \"#bc05bc\", \"#0598bc\", \"#555555\",\n",
            "    \"#666666\", \"#cd3131\", \"#14ce14\", \"#b5ba00\",\n",
            "    \"#0451a5\", \"#bc05bc\", \"#0598bc\", \"#a5a5a5\"\n",
            "]\n",
        );

        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = Config::load_from_path(&config_path).unwrap();
        assert_eq!(config.font_family, "JetBrains Mono");
        assert_eq!(config.font_size, 16.0);
        assert_eq!(config.theme, ThemeName::Light);
        assert_eq!(config.dimensions, (120, 40));

        std::fs::remove_file(&config_path).ok();
    }

    #[test]
    fn test_load_from_path_invalid_toml() {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("mochi_test_invalid.toml");

        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(b"this is not valid toml {{{").unwrap();

        let result = Config::load_from_path(&config_path);
        assert!(result.is_err());

        std::fs::remove_file(&config_path).ok();
    }

    #[test]
    fn test_load_with_args_defaults() {
        let args = CliArgs::default();
        let config = Config::load_with_args(&args).unwrap();

        // Should get defaults when no config file exists
        assert_eq!(config.font_size, 14.0);
        assert_eq!(config.theme, ThemeName::Dark);
    }

    #[test]
    fn test_load_with_args_cli_override() {
        let args = CliArgs {
            config_path: None,
            font_size: Some(18.0),
            theme: Some(ThemeName::Nord),
            shell: None,
        };
        let config = Config::load_with_args(&args).unwrap();

        assert_eq!(config.font_size, 18.0);
        assert_eq!(config.theme, ThemeName::Nord);
    }

    #[test]
    fn test_load_with_args_nonexistent_config() {
        let args = CliArgs {
            config_path: Some(PathBuf::from("/nonexistent/path/config.toml")),
            font_size: None,
            theme: None,
            shell: None,
        };
        let result = Config::load_with_args(&args);
        assert!(matches!(result, Err(ConfigError::NotFound(_))));
    }

    #[test]
    fn test_effective_colors() {
        let mut config = Config::default();

        config.theme = ThemeName::Dark;
        let colors = config.effective_colors();
        assert_eq!(colors.background, "#1e1e1e");

        config.theme = ThemeName::Light;
        let colors = config.effective_colors();
        assert_eq!(colors.background, "#ffffff");

        config.theme = ThemeName::Nord;
        let colors = config.effective_colors();
        assert_eq!(colors.background, "#2e3440");
    }

    #[test]
    fn test_all_themes_have_valid_colors() {
        for theme in ThemeName::all() {
            let mut config = Config::default();
            config.theme = *theme;
            let colors = config.effective_colors();

            // Verify all colors are valid hex
            assert!(ColorScheme::parse_hex(&colors.foreground).is_some());
            assert!(ColorScheme::parse_hex(&colors.background).is_some());
            assert!(ColorScheme::parse_hex(&colors.cursor).is_some());
            assert!(ColorScheme::parse_hex(&colors.selection).is_some());
            for color in &colors.ansi {
                assert!(ColorScheme::parse_hex(color).is_some());
            }
        }
    }

    #[test]
    fn test_theme_dark_colors() {
        let colors = ColorScheme::dark();
        // Dark theme should have dark background
        let (r, g, b) = colors.background_rgb();
        assert!(r < 100 && g < 100 && b < 100, "Dark theme background should be dark");
        // And light foreground
        let (r, g, b) = colors.foreground_rgb();
        assert!(r > 150 || g > 150 || b > 150, "Dark theme foreground should be light");
    }

    #[test]
    fn test_theme_light_colors() {
        let colors = ColorScheme::light();
        // Light theme should have light background
        let (r, g, b) = colors.background_rgb();
        assert!(r > 200 && g > 200 && b > 200, "Light theme background should be light");
        // And dark foreground
        let (r, g, b) = colors.foreground_rgb();
        assert!(r < 100 && g < 100 && b < 100, "Light theme foreground should be dark");
    }

    #[test]
    fn test_theme_solarized_dark() {
        let colors = ColorScheme::solarized_dark();
        // Solarized dark has specific background color #002b36
        assert_eq!(colors.background, "#002b36");
        assert_eq!(colors.foreground, "#839496");
    }

    #[test]
    fn test_theme_solarized_light() {
        let colors = ColorScheme::solarized_light();
        // Solarized light has specific background color #fdf6e3
        assert_eq!(colors.background, "#fdf6e3");
        assert_eq!(colors.foreground, "#657b83");
    }

    #[test]
    fn test_theme_dracula() {
        let colors = ColorScheme::dracula();
        // Dracula has specific background color #282a36
        assert_eq!(colors.background, "#282a36");
        assert_eq!(colors.foreground, "#f8f8f2");
    }

    #[test]
    fn test_theme_nord() {
        let colors = ColorScheme::nord();
        // Nord has specific background color #2e3440
        assert_eq!(colors.background, "#2e3440");
        assert_eq!(colors.foreground, "#d8dee9");
    }

    #[test]
    fn test_ansi_colors_count() {
        // All themes should have exactly 16 ANSI colors
        for theme in ThemeName::all() {
            let mut config = Config::default();
            config.theme = *theme;
            let colors = config.effective_colors();
            assert_eq!(colors.ansi.len(), 16, "Theme {:?} should have 16 ANSI colors", theme);
        }
    }

    #[test]
    fn test_theme_cycle_returns_to_start() {
        // Cycling through all themes should eventually return to the start
        let start = ThemeName::Dark;
        let mut current = start;
        let mut count = 0;
        loop {
            current = current.next();
            count += 1;
            if current == start {
                break;
            }
            // Safety: prevent infinite loop
            assert!(count < 20, "Theme cycle should return to start within 20 iterations");
        }
        // Should cycle through all non-custom themes
        assert_eq!(count, 6, "Should cycle through 6 themes");
    }
}
