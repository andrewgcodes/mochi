//! Configuration for Mochi Terminal
//!
//! Configuration is loaded with the following precedence (highest to lowest):
//! 1. CLI arguments
//! 2. Environment variables (MOCHI_*)
//! 3. Configuration file (~/.config/mochi/config.toml or --config path)
//! 4. Built-in defaults

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

use crate::Args;

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

impl ThemeName {
    /// Get the next theme in the cycle (skips Custom)
    pub fn next(self) -> Self {
        match self {
            ThemeName::Dark => ThemeName::Light,
            ThemeName::Light => ThemeName::SolarizedDark,
            ThemeName::SolarizedDark => ThemeName::SolarizedLight,
            ThemeName::SolarizedLight => ThemeName::Dracula,
            ThemeName::Dracula => ThemeName::Nord,
            ThemeName::Nord => ThemeName::Dark,
            ThemeName::Custom => ThemeName::Dark, // Custom cycles back to Dark
        }
    }

    /// Get the display name for the theme
    pub fn display_name(self) -> &'static str {
        match self {
            ThemeName::Dark => "Dark",
            ThemeName::Light => "Light",
            ThemeName::SolarizedDark => "Solarized Dark",
            ThemeName::SolarizedLight => "Solarized Light",
            ThemeName::Dracula => "Dracula",
            ThemeName::Nord => "Nord",
            ThemeName::Custom => "Custom",
        }
    }

    /// Get the color scheme for this theme
    pub fn color_scheme(self) -> ColorScheme {
        match self {
            ThemeName::Dark => ColorScheme::dark(),
            ThemeName::Light => ColorScheme::light(),
            ThemeName::SolarizedDark => ColorScheme::solarized_dark(),
            ThemeName::SolarizedLight => ColorScheme::solarized_light(),
            ThemeName::Dracula => ColorScheme::dracula(),
            ThemeName::Nord => ColorScheme::nord(),
            ThemeName::Custom => ColorScheme::default(), // Custom uses config colors
        }
    }
}

/// Actions that can be bound to keybindings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Copy selected text to clipboard
    Copy,
    /// Paste from clipboard
    Paste,
    /// Toggle theme (cycle through built-in themes)
    ToggleTheme,
    /// Reload configuration
    ReloadConfig,
    /// Open search/find bar
    Find,
    /// Increase font size
    FontSizeIncrease,
    /// Decrease font size
    FontSizeDecrease,
    /// Reset font size to default
    FontSizeReset,
    /// Scroll up one page
    ScrollPageUp,
    /// Scroll down one page
    ScrollPageDown,
    /// Scroll to top of scrollback
    ScrollToTop,
    /// Scroll to bottom (current output)
    ScrollToBottom,
}

/// A keybinding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    /// The key (e.g., "c", "v", "t", "f", "r")
    pub key: String,
    /// Modifier keys
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    /// The action to perform
    pub action: Action,
}

impl KeyBinding {
    /// Create a new keybinding
    pub fn new(key: &str, ctrl: bool, shift: bool, alt: bool, action: Action) -> Self {
        Self {
            key: key.to_lowercase(),
            ctrl,
            shift,
            alt,
            action,
        }
    }

    /// Check if this keybinding matches the given key and modifiers
    pub fn matches(&self, key: &str, ctrl: bool, shift: bool, alt: bool) -> bool {
        self.key == key.to_lowercase() && self.ctrl == ctrl && self.shift == shift && self.alt == alt
    }
}

/// Keybindings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybindings {
    /// List of keybindings
    #[serde(default = "Keybindings::default_bindings")]
    pub bindings: Vec<KeyBinding>,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            bindings: Self::default_bindings(),
        }
    }
}

impl Keybindings {
    /// Get the default keybindings
    pub fn default_bindings() -> Vec<KeyBinding> {
        vec![
            KeyBinding::new("c", true, true, false, Action::Copy),
            KeyBinding::new("v", true, true, false, Action::Paste),
            KeyBinding::new("t", true, true, false, Action::ToggleTheme),
            KeyBinding::new("r", true, true, false, Action::ReloadConfig),
            KeyBinding::new("f", true, true, false, Action::Find),
            KeyBinding::new("=", true, false, false, Action::FontSizeIncrease),
            KeyBinding::new("+", true, false, false, Action::FontSizeIncrease),
            KeyBinding::new("-", true, false, false, Action::FontSizeDecrease),
            KeyBinding::new("0", true, false, false, Action::FontSizeReset),
        ]
    }

    /// Find the action for a given key combination
    pub fn find_action(&self, key: &str, ctrl: bool, shift: bool, alt: bool) -> Option<Action> {
        self.bindings
            .iter()
            .find(|b| b.matches(key, ctrl, shift, alt))
            .map(|b| b.action)
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
    /// Keybindings
    #[serde(default)]
    pub keybindings: Keybindings,
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
            keybindings: Keybindings::default(),
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

/// Configuration error type
#[derive(Debug)]
pub struct ConfigError {
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// Load configuration with CLI arguments
    ///
    /// Precedence (highest to lowest):
    /// 1. CLI arguments
    /// 2. Environment variables (MOCHI_*)
    /// 3. Configuration file
    /// 4. Built-in defaults
    pub fn load_with_args(args: &Args) -> Result<Self, ConfigError> {
        // Start with defaults
        let mut config = Config::default();

        // Layer 1: Load from config file (if exists)
        let config_path = args.config.clone().or_else(Self::config_path);

        if let Some(path) = &config_path {
            if path.exists() {
                log::info!("Loading config from: {:?}", path);
                match fs::read_to_string(path) {
                    Ok(content) => match toml::from_str::<Config>(&content) {
                        Ok(file_config) => {
                            config = file_config;
                        }
                        Err(e) => {
                            return Err(ConfigError {
                                message: format!("Failed to parse config file {:?}: {}", path, e),
                            });
                        }
                    },
                    Err(e) => {
                        return Err(ConfigError {
                            message: format!("Failed to read config file {:?}: {}", path, e),
                        });
                    }
                }
            } else if args.config.is_some() {
                // User explicitly specified a config file that doesn't exist
                return Err(ConfigError {
                    message: format!("Config file not found: {:?}", path),
                });
            }
        }

        // Layer 2: Apply environment variables
        Self::apply_env_vars(&mut config);

        // Layer 3: Apply CLI arguments (highest priority)
        Self::apply_cli_args(&mut config, args)?;

        // Validate the final configuration
        config.validate()?;

        Ok(config)
    }

    /// Apply environment variables to config
    fn apply_env_vars(config: &mut Config) {
        if let Ok(val) = env::var("MOCHI_FONT_FAMILY") {
            config.font_family = val;
        }

        if let Ok(val) = env::var("MOCHI_FONT_SIZE") {
            if let Ok(size) = val.parse::<f32>() {
                config.font_size = size;
            }
        }

        if let Ok(val) = env::var("MOCHI_THEME") {
            if let Some(theme) = Self::parse_theme_name(&val) {
                config.theme = theme;
            }
        }

        if let Ok(val) = env::var("MOCHI_SHELL") {
            config.shell = Some(val);
        }

        if let Ok(val) = env::var("MOCHI_SCROLLBACK") {
            if let Ok(lines) = val.parse::<usize>() {
                config.scrollback_lines = lines;
            }
        }

        if let Ok(val) = env::var("MOCHI_COLUMNS") {
            if let Ok(cols) = val.parse::<u16>() {
                config.dimensions.0 = cols;
            }
        }

        if let Ok(val) = env::var("MOCHI_ROWS") {
            if let Ok(rows) = val.parse::<u16>() {
                config.dimensions.1 = rows;
            }
        }

        if let Ok(val) = env::var("MOCHI_OSC52_CLIPBOARD") {
            config.osc52_clipboard = val == "1" || val.to_lowercase() == "true";
        }
    }

    /// Apply CLI arguments to config
    fn apply_cli_args(config: &mut Config, args: &Args) -> Result<(), ConfigError> {
        if let Some(ref font_family) = args.font_family {
            config.font_family = font_family.clone();
        }

        if let Some(font_size) = args.font_size {
            config.font_size = font_size;
        }

        if let Some(ref theme_str) = args.theme {
            match Self::parse_theme_name(theme_str) {
                Some(theme) => config.theme = theme,
                None => {
                    return Err(ConfigError {
                        message: format!(
                            "Invalid theme '{}'. Valid themes: dark, light, solarized-dark, solarized-light, dracula, nord",
                            theme_str
                        ),
                    });
                }
            }
        }

        if let Some(ref shell) = args.shell {
            config.shell = Some(shell.clone());
        }

        if let Some(cols) = args.columns {
            config.dimensions.0 = cols;
        }

        if let Some(rows) = args.rows {
            config.dimensions.1 = rows;
        }

        if let Some(scrollback) = args.scrollback {
            config.scrollback_lines = scrollback;
        }

        if args.osc52_clipboard {
            config.osc52_clipboard = true;
        }

        Ok(())
    }

    /// Parse a theme name string to ThemeName enum
    fn parse_theme_name(name: &str) -> Option<ThemeName> {
        match name.to_lowercase().as_str() {
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

    /// Validate the configuration
    fn validate(&self) -> Result<(), ConfigError> {
        if self.font_size < 4.0 || self.font_size > 128.0 {
            return Err(ConfigError {
                message: format!("Font size {} is out of range (4.0 - 128.0)", self.font_size),
            });
        }

        if self.dimensions.0 < 10 || self.dimensions.0 > 1000 {
            return Err(ConfigError {
                message: format!(
                    "Column count {} is out of range (10 - 1000)",
                    self.dimensions.0
                ),
            });
        }

        if self.dimensions.1 < 5 || self.dimensions.1 > 500 {
            return Err(ConfigError {
                message: format!("Row count {} is out of range (5 - 500)", self.dimensions.1),
            });
        }

        if self.scrollback_lines > 1_000_000 {
            return Err(ConfigError {
                message: format!(
                    "Scrollback lines {} exceeds maximum (1,000,000)",
                    self.scrollback_lines
                ),
            });
        }

        Ok(())
    }

    /// Load configuration from file (legacy method for backward compatibility)
    #[allow(dead_code)]
    pub fn load() -> Option<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return None;
        }

        let content = fs::read_to_string(&config_path).ok()?;
        toml::from_str(&content).ok()
    }

    /// Load configuration from a specific path
    #[allow(dead_code)]
    pub fn load_from_path(path: &PathBuf) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Err(ConfigError {
                message: format!("Config file not found: {:?}", path),
            });
        }

        let content = fs::read_to_string(path).map_err(|e| ConfigError {
            message: format!("Failed to read config file {:?}: {}", path, e),
        })?;

        toml::from_str(&content).map_err(|e| ConfigError {
            message: format!("Failed to parse config file {:?}: {}", path, e),
        })
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

    /// Get the default configuration file path (XDG compliant)
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("mochi").join("config.toml"))
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
    fn test_parse_theme_name() {
        assert_eq!(Config::parse_theme_name("dark"), Some(ThemeName::Dark));
        assert_eq!(Config::parse_theme_name("light"), Some(ThemeName::Light));
        assert_eq!(
            Config::parse_theme_name("solarized-dark"),
            Some(ThemeName::SolarizedDark)
        );
        assert_eq!(
            Config::parse_theme_name("solarized-light"),
            Some(ThemeName::SolarizedLight)
        );
        assert_eq!(
            Config::parse_theme_name("dracula"),
            Some(ThemeName::Dracula)
        );
        assert_eq!(Config::parse_theme_name("nord"), Some(ThemeName::Nord));
        assert_eq!(Config::parse_theme_name("DARK"), Some(ThemeName::Dark)); // case insensitive
        assert_eq!(Config::parse_theme_name("invalid"), None);
    }

    #[test]
    fn test_config_validation_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_font_size_too_small() {
        let config = Config {
            font_size: 2.0,
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_font_size_too_large() {
        let config = Config {
            font_size: 200.0,
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_columns_too_small() {
        let config = Config {
            dimensions: (5, 24),
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_rows_too_small() {
        let config = Config {
            dimensions: (80, 2),
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_scrollback_too_large() {
        let config = Config {
            scrollback_lines: 2_000_000,
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_effective_colors_dark() {
        let config = Config {
            theme: ThemeName::Dark,
            ..Config::default()
        };
        let colors = config.effective_colors();
        assert_eq!(colors.background, "#1e1e1e");
    }

    #[test]
    fn test_effective_colors_light() {
        let config = Config {
            theme: ThemeName::Light,
            ..Config::default()
        };
        let colors = config.effective_colors();
        assert_eq!(colors.background, "#ffffff");
    }

    #[test]
    fn test_effective_colors_custom() {
        let custom_colors = ColorScheme {
            background: "#123456".to_string(),
            ..ColorScheme::default()
        };
        let config = Config {
            theme: ThemeName::Custom,
            colors: custom_colors,
            ..Config::default()
        };
        let colors = config.effective_colors();
        assert_eq!(colors.background, "#123456");
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
        ];

        for theme in themes {
            let config = Config {
                theme,
                ..Config::default()
            };
            let colors = config.effective_colors();

            // Verify all colors are valid hex
            assert!(
                ColorScheme::parse_hex(&colors.foreground).is_some(),
                "Invalid foreground for {:?}",
                theme
            );
            assert!(
                ColorScheme::parse_hex(&colors.background).is_some(),
                "Invalid background for {:?}",
                theme
            );
            assert!(
                ColorScheme::parse_hex(&colors.cursor).is_some(),
                "Invalid cursor for {:?}",
                theme
            );
            assert!(
                ColorScheme::parse_hex(&colors.selection).is_some(),
                "Invalid selection for {:?}",
                theme
            );

            for (i, ansi) in colors.ansi.iter().enumerate() {
                assert!(
                    ColorScheme::parse_hex(ansi).is_some(),
                    "Invalid ANSI color {} for {:?}",
                    i,
                    theme
                );
            }
        }
    }

    #[test]
    fn test_theme_name_next_cycles_through_all() {
        // Test that next() cycles through all built-in themes
        let mut theme = ThemeName::Dark;
        let mut visited = vec![theme];

        for _ in 0..6 {
            theme = theme.next();
            if theme == ThemeName::Dark {
                break;
            }
            visited.push(theme);
        }

        // Should have visited all 6 built-in themes
        assert_eq!(visited.len(), 6);
        assert!(visited.contains(&ThemeName::Dark));
        assert!(visited.contains(&ThemeName::Light));
        assert!(visited.contains(&ThemeName::SolarizedDark));
        assert!(visited.contains(&ThemeName::SolarizedLight));
        assert!(visited.contains(&ThemeName::Dracula));
        assert!(visited.contains(&ThemeName::Nord));
    }

    #[test]
    fn test_theme_name_custom_cycles_to_dark() {
        // Custom theme should cycle back to Dark
        let theme = ThemeName::Custom;
        assert_eq!(theme.next(), ThemeName::Dark);
    }

    #[test]
    fn test_theme_name_display_name() {
        assert_eq!(ThemeName::Dark.display_name(), "Dark");
        assert_eq!(ThemeName::Light.display_name(), "Light");
        assert_eq!(ThemeName::SolarizedDark.display_name(), "Solarized Dark");
        assert_eq!(ThemeName::SolarizedLight.display_name(), "Solarized Light");
        assert_eq!(ThemeName::Dracula.display_name(), "Dracula");
        assert_eq!(ThemeName::Nord.display_name(), "Nord");
        assert_eq!(ThemeName::Custom.display_name(), "Custom");
    }

    #[test]
    fn test_theme_name_color_scheme() {
        // Each theme should return a distinct color scheme
        let dark = ThemeName::Dark.color_scheme();
        let light = ThemeName::Light.color_scheme();

        // Dark and light should have different backgrounds
        assert_ne!(dark.background, light.background);

        // Verify each theme returns valid colors
        for theme in [
            ThemeName::Dark,
            ThemeName::Light,
            ThemeName::SolarizedDark,
            ThemeName::SolarizedLight,
            ThemeName::Dracula,
            ThemeName::Nord,
        ] {
            let colors = theme.color_scheme();
            assert!(
                ColorScheme::parse_hex(&colors.background).is_some(),
                "Invalid background for {:?}",
                theme
            );
        }
    }

    #[test]
    fn test_keybinding_new() {
        let binding = KeyBinding::new("c", true, true, false, Action::Copy);
        assert_eq!(binding.key, "c");
        assert!(binding.ctrl);
        assert!(binding.shift);
        assert!(!binding.alt);
        assert_eq!(binding.action, Action::Copy);
    }

    #[test]
    fn test_keybinding_matches() {
        let binding = KeyBinding::new("c", true, true, false, Action::Copy);

        // Should match exact combination
        assert!(binding.matches("c", true, true, false));
        assert!(binding.matches("C", true, true, false)); // Case insensitive

        // Should not match different combinations
        assert!(!binding.matches("c", true, false, false)); // Missing shift
        assert!(!binding.matches("c", false, true, false)); // Missing ctrl
        assert!(!binding.matches("v", true, true, false)); // Wrong key
        assert!(!binding.matches("c", true, true, true)); // Extra alt
    }

    #[test]
    fn test_keybindings_default() {
        let keybindings = Keybindings::default();

        // Should have default bindings
        assert!(!keybindings.bindings.is_empty());

        // Should include copy, paste, toggle theme
        assert!(keybindings
            .find_action("c", true, true, false)
            .is_some_and(|a| a == Action::Copy));
        assert!(keybindings
            .find_action("v", true, true, false)
            .is_some_and(|a| a == Action::Paste));
        assert!(keybindings
            .find_action("t", true, true, false)
            .is_some_and(|a| a == Action::ToggleTheme));
    }

    #[test]
    fn test_keybindings_find_action() {
        let keybindings = Keybindings::default();

        // Should find configured actions
        assert_eq!(
            keybindings.find_action("c", true, true, false),
            Some(Action::Copy)
        );
        assert_eq!(
            keybindings.find_action("=", true, false, false),
            Some(Action::FontSizeIncrease)
        );

        // Should return None for unconfigured combinations
        assert_eq!(keybindings.find_action("x", true, true, false), None);
        assert_eq!(keybindings.find_action("c", false, false, false), None);
    }

    #[test]
    fn test_config_has_default_keybindings() {
        let config = Config::default();
        assert!(!config.keybindings.bindings.is_empty());
    }
}
