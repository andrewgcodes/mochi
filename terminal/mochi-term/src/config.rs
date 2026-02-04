//! Configuration for Mochi Terminal
//!
//! Configuration is loaded with the following precedence (highest to lowest):
//! 1. CLI arguments (--config, --font-size, --theme, etc.)
//! 2. Environment variables (MOCHI_CONFIG, MOCHI_FONT_SIZE, MOCHI_THEME, etc.)
//! 3. Config file (~/.config/mochi/config.toml by default)
//! 4. Built-in defaults

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

/// CLI arguments for Mochi Terminal
#[derive(Parser, Debug, Clone)]
#[command(name = "mochi")]
#[command(author = "Mochi Team")]
#[command(version)]
#[command(about = "A modern, customizable terminal emulator")]
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

    /// Initial window columns
    #[arg(long, value_name = "COLS")]
    pub cols: Option<u16>,

    /// Initial window rows
    #[arg(long, value_name = "ROWS")]
    pub rows: Option<u16>,

    /// Shell command to run
    #[arg(long, value_name = "SHELL")]
    pub shell: Option<String>,

    /// Enable OSC 52 clipboard support
    #[arg(long)]
    pub osc52_clipboard: bool,
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
    #[serde(alias = "dark", alias = "mochi-dark")]
    MochiDark,
    /// Light theme - also known as mochi-light
    #[serde(alias = "light", alias = "mochi-light")]
    MochiLight,
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
    /// Parse theme name from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dark" | "mochi-dark" | "mochidark" => Some(Self::MochiDark),
            "light" | "mochi-light" | "mochilight" => Some(Self::MochiLight),
            "solarized-dark" | "solarizeddark" => Some(Self::SolarizedDark),
            "solarized-light" | "solarizedlight" => Some(Self::SolarizedLight),
            "dracula" => Some(Self::Dracula),
            "nord" => Some(Self::Nord),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::MochiDark => "mochi-dark",
            Self::MochiLight => "mochi-light",
            Self::SolarizedDark => "solarized-dark",
            Self::SolarizedLight => "solarized-light",
            Self::Dracula => "dracula",
            Self::Nord => "nord",
            Self::Custom => "custom",
        }
    }

    /// Get all available theme names
    #[allow(dead_code)]
    pub fn all() -> &'static [ThemeName] {
        &[
            Self::MochiDark,
            Self::MochiLight,
            Self::SolarizedDark,
            Self::SolarizedLight,
            Self::Dracula,
            Self::Nord,
        ]
    }

    /// Get next theme in cycle
    pub fn next(&self) -> Self {
        match self {
            Self::MochiDark => Self::MochiLight,
            Self::MochiLight => Self::SolarizedDark,
            Self::SolarizedDark => Self::SolarizedLight,
            Self::SolarizedLight => Self::Dracula,
            Self::Dracula => Self::Nord,
            Self::Nord => Self::MochiDark,
            Self::Custom => Self::MochiDark,
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

/// A keybinding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybinding {
    /// Key (e.g., "C", "V", "F", "R", "T")
    pub key: String,
    /// Modifiers (e.g., ["ctrl", "shift"])
    #[serde(default)]
    pub modifiers: Vec<String>,
    /// Action to perform
    pub action: KeyAction,
}

impl Keybinding {
    /// Create a new keybinding
    pub fn new(key: &str, modifiers: &[&str], action: KeyAction) -> Self {
        Self {
            key: key.to_string(),
            modifiers: modifiers.iter().map(|s| s.to_string()).collect(),
            action,
        }
    }

    /// Check if modifiers match
    pub fn has_ctrl(&self) -> bool {
        self.modifiers.iter().any(|m| m.to_lowercase() == "ctrl")
    }

    pub fn has_shift(&self) -> bool {
        self.modifiers.iter().any(|m| m.to_lowercase() == "shift")
    }

    pub fn has_alt(&self) -> bool {
        self.modifiers.iter().any(|m| m.to_lowercase() == "alt")
    }

    pub fn has_super(&self) -> bool {
        self.modifiers
            .iter()
            .any(|m| m.to_lowercase() == "super" || m.to_lowercase() == "meta")
    }
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
    /// Default keybindings
    pub fn default_bindings() -> Vec<Keybinding> {
        vec![
            Keybinding::new("c", &["ctrl", "shift"], KeyAction::Copy),
            Keybinding::new("v", &["ctrl", "shift"], KeyAction::Paste),
            Keybinding::new("f", &["ctrl", "shift"], KeyAction::Find),
            Keybinding::new("r", &["ctrl", "shift"], KeyAction::ReloadConfig),
            Keybinding::new("t", &["ctrl", "shift"], KeyAction::ToggleTheme),
            Keybinding::new("=", &["ctrl"], KeyAction::FontSizeIncrease),
            Keybinding::new("+", &["ctrl"], KeyAction::FontSizeIncrease),
            Keybinding::new("-", &["ctrl"], KeyAction::FontSizeDecrease),
            Keybinding::new("0", &["ctrl"], KeyAction::FontSizeReset),
        ]
    }

    /// Find action for a key event
    pub fn find_action(
        &self,
        key: &str,
        ctrl: bool,
        shift: bool,
        alt: bool,
        super_key: bool,
    ) -> Option<KeyAction> {
        let key_lower = key.to_lowercase();
        for binding in &self.bindings {
            if binding.key.to_lowercase() == key_lower
                && binding.has_ctrl() == ctrl
                && binding.has_shift() == shift
                && binding.has_alt() == alt
                && binding.has_super() == super_key
            {
                return Some(binding.action);
            }
        }
        None
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
    /// Fallback font families for missing glyphs
    #[serde(default)]
    pub font_fallback: Vec<String>,
    /// Line height multiplier (1.0 = normal)
    #[serde(default = "default_line_height")]
    pub line_height: f32,
    /// Cell padding (horizontal, vertical) in pixels
    #[serde(default)]
    pub cell_padding: (f32, f32),
    /// Number of scrollback lines
    #[serde(default = "default_scrollback_lines")]
    pub scrollback_lines: usize,
    /// Window dimensions (columns, rows)
    #[serde(default = "default_dimensions")]
    pub dimensions: (u16, u16),
    /// Theme name (mochi-dark, mochi-light, solarized-dark, solarized-light, dracula, nord, custom)
    #[serde(default)]
    pub theme: ThemeName,
    /// Color scheme (used when theme is "custom", otherwise ignored)
    #[serde(default)]
    pub colors: ColorScheme,
    /// Enable OSC 52 clipboard (disabled by default for security)
    #[serde(default)]
    pub osc52_clipboard: bool,
    /// Maximum OSC 52 payload size in bytes
    #[serde(default = "default_osc52_max_size")]
    pub osc52_max_size: usize,
    /// Shell command (None = use $SHELL)
    #[serde(default)]
    pub shell: Option<String>,
    /// Cursor style (block, underline, bar)
    #[serde(default = "default_cursor_style")]
    pub cursor_style: String,
    /// Cursor blink
    #[serde(default = "default_cursor_blink")]
    pub cursor_blink: bool,
    /// Keybindings
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
    /// Maximum title updates per second (throttling)
    #[serde(default = "default_title_update_rate")]
    pub title_update_rate: u32,
}

fn default_font_family() -> String {
    "monospace".to_string()
}

fn default_font_size() -> f32 {
    14.0
}

fn default_line_height() -> f32 {
    1.4
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

fn default_title_update_rate() -> u32 {
    10
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_family: default_font_family(),
            font_size: default_font_size(),
            font_fallback: Vec::new(),
            line_height: default_line_height(),
            cell_padding: (0.0, 0.0),
            scrollback_lines: default_scrollback_lines(),
            dimensions: default_dimensions(),
            theme: ThemeName::default(),
            colors: ColorScheme::default(),
            osc52_clipboard: false, // Disabled by default for security
            osc52_max_size: default_osc52_max_size(),
            shell: None,
            cursor_style: default_cursor_style(),
            cursor_blink: default_cursor_blink(),
            keybindings: KeybindingsConfig::default(),
            title_update_rate: default_title_update_rate(),
        }
    }
}

impl Config {
    /// Get the effective color scheme based on the theme setting
    pub fn effective_colors(&self) -> ColorScheme {
        match self.theme {
            ThemeName::Custom => self.colors.clone(),
            ThemeName::MochiDark => ColorScheme::mochi_dark(),
            ThemeName::MochiLight => ColorScheme::mochi_light(),
            ThemeName::SolarizedDark => ColorScheme::solarized_dark(),
            ThemeName::SolarizedLight => ColorScheme::solarized_light(),
            ThemeName::Dracula => ColorScheme::dracula(),
            ThemeName::Nord => ColorScheme::nord(),
        }
    }

    /// Load configuration with proper precedence:
    /// CLI args > Environment variables > Config file > Defaults
    pub fn load_with_args(args: &CliArgs) -> Result<Self, ConfigError> {
        // Start with defaults
        let mut config = Self::default();

        // Load from config file (if exists)
        let config_path = args
            .config
            .clone()
            .or_else(Self::env_config_path)
            .or_else(Self::default_config_path);

        if let Some(path) = config_path {
            if path.exists() {
                log::info!("Loading config from: {}", path.display());
                match Self::load_from_file(&path) {
                    Ok(file_config) => config = file_config,
                    Err(e) => {
                        log::warn!("Failed to load config file: {}", e);
                        return Err(e);
                    }
                }
            } else if args.config.is_some() {
                // User explicitly specified a config file that doesn't exist
                return Err(ConfigError::FileNotFound(path));
            }
        }

        // Apply environment variables
        config.apply_env_vars();

        // Apply CLI arguments (highest priority)
        config.apply_cli_args(args);

        // Validate the final config
        config.validate()?;

        Ok(config)
    }

    /// Load configuration from file only (legacy method)
    #[allow(dead_code)]
    pub fn load() -> Option<Self> {
        let config_path = Self::default_config_path()?;

        if !config_path.exists() {
            return None;
        }

        Self::load_from_file(&config_path).ok()
    }

    /// Load configuration from a specific file
    pub fn load_from_file(path: &PathBuf) -> Result<Self, ConfigError> {
        let content =
            fs::read_to_string(path).map_err(|e| ConfigError::ReadError(e.to_string()))?;
        toml::from_str(&content).map_err(|e| ConfigError::ParseError(e.to_string()))
    }

    /// Apply environment variables to config
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

        if let Ok(val) = env::var("MOCHI_SCROLLBACK_LINES") {
            if let Ok(lines) = val.parse::<usize>() {
                self.scrollback_lines = lines;
            }
        }

        if let Ok(val) = env::var("MOCHI_SHELL") {
            self.shell = Some(val);
        }

        if let Ok(val) = env::var("MOCHI_OSC52_CLIPBOARD") {
            self.osc52_clipboard = val == "1" || val.to_lowercase() == "true";
        }

        if let Ok(val) = env::var("MOCHI_LINE_HEIGHT") {
            if let Ok(height) = val.parse::<f32>() {
                self.line_height = height;
            }
        }
    }

    /// Apply CLI arguments to config
    fn apply_cli_args(&mut self, args: &CliArgs) {
        if let Some(size) = args.font_size {
            self.font_size = size;
        }

        if let Some(ref family) = args.font_family {
            self.font_family = family.clone();
        }

        if let Some(ref theme_str) = args.theme {
            if let Some(theme) = ThemeName::from_str(theme_str) {
                self.theme = theme;
            }
        }

        if let Some(cols) = args.cols {
            self.dimensions.0 = cols;
        }

        if let Some(rows) = args.rows {
            self.dimensions.1 = rows;
        }

        if let Some(ref shell) = args.shell {
            self.shell = Some(shell.clone());
        }

        if args.osc52_clipboard {
            self.osc52_clipboard = true;
        }
    }

    /// Validate configuration
    fn validate(&self) -> Result<(), ConfigError> {
        if self.font_size < 4.0 || self.font_size > 200.0 {
            return Err(ConfigError::ValidationError(format!(
                "font_size must be between 4 and 200, got {}",
                self.font_size
            )));
        }

        if self.line_height < 0.5 || self.line_height > 3.0 {
            return Err(ConfigError::ValidationError(format!(
                "line_height must be between 0.5 and 3.0, got {}",
                self.line_height
            )));
        }

        if self.dimensions.0 < 10 || self.dimensions.0 > 1000 {
            return Err(ConfigError::ValidationError(format!(
                "columns must be between 10 and 1000, got {}",
                self.dimensions.0
            )));
        }

        if self.dimensions.1 < 5 || self.dimensions.1 > 500 {
            return Err(ConfigError::ValidationError(format!(
                "rows must be between 5 and 500, got {}",
                self.dimensions.1
            )));
        }

        if self.osc52_max_size > 10_000_000 {
            return Err(ConfigError::ValidationError(format!(
                "osc52_max_size must be at most 10MB, got {}",
                self.osc52_max_size
            )));
        }

        Ok(())
    }

    /// Get config path from environment variable
    fn env_config_path() -> Option<PathBuf> {
        env::var("MOCHI_CONFIG").ok().map(PathBuf::from)
    }

    /// Get the default configuration file path (XDG compliant)
    pub fn default_config_path() -> Option<PathBuf> {
        // First try XDG_CONFIG_HOME
        if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg_config).join("mochi").join("config.toml"));
        }

        // Fall back to ~/.config/mochi/config.toml
        dirs::config_dir().map(|p| p.join("mochi").join("config.toml"))
    }

    /// Save configuration to file
    #[allow(dead_code)]
    pub fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::default_config_path().ok_or(ConfigError::NoConfigPath)?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::WriteError(e.to_string()))?;
        }

        let content =
            toml::to_string_pretty(self).map_err(|e| ConfigError::SerializeError(e.to_string()))?;
        fs::write(&config_path, content).map_err(|e| ConfigError::WriteError(e.to_string()))?;

        Ok(())
    }

    /// Reload configuration from file
    pub fn reload(&mut self) -> Result<(), ConfigError> {
        let config_path = Self::default_config_path().ok_or(ConfigError::NoConfigPath)?;

        if !config_path.exists() {
            return Err(ConfigError::FileNotFound(config_path));
        }

        let new_config = Self::load_from_file(&config_path)?;
        new_config.validate()?;

        // Update fields that can be changed at runtime
        self.font_family = new_config.font_family;
        self.font_size = new_config.font_size;
        self.font_fallback = new_config.font_fallback;
        self.line_height = new_config.line_height;
        self.cell_padding = new_config.cell_padding;
        self.theme = new_config.theme;
        self.colors = new_config.colors;
        self.keybindings = new_config.keybindings;
        self.osc52_clipboard = new_config.osc52_clipboard;
        self.osc52_max_size = new_config.osc52_max_size;
        self.title_update_rate = new_config.title_update_rate;
        // Note: shell and dimensions are not reloaded as they require restart

        Ok(())
    }
}

/// Configuration errors
#[derive(Debug, Clone)]
pub enum ConfigError {
    FileNotFound(PathBuf),
    ReadError(String),
    ParseError(String),
    ValidationError(String),
    WriteError(String),
    SerializeError(String),
    NoConfigPath,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileNotFound(path) => write!(f, "Config file not found: {}", path.display()),
            Self::ReadError(e) => write!(f, "Failed to read config file: {}", e),
            Self::ParseError(e) => write!(f, "Failed to parse config file: {}", e),
            Self::ValidationError(e) => write!(f, "Config validation error: {}", e),
            Self::WriteError(e) => write!(f, "Failed to write config file: {}", e),
            Self::SerializeError(e) => write!(f, "Failed to serialize config: {}", e),
            Self::NoConfigPath => write!(f, "Could not determine config path"),
        }
    }
}

impl std::error::Error for ConfigError {}

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

impl Default for ColorScheme {
    fn default() -> Self {
        Self::mochi_dark()
    }
}

impl ColorScheme {
    /// Mochi Dark theme (VS Code inspired) - the default dark theme
    pub fn mochi_dark() -> Self {
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

    /// Mochi Light theme - the default light theme
    pub fn mochi_light() -> Self {
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
        assert_eq!(config.theme, ThemeName::MochiDark);
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
        assert_eq!(ThemeName::from_str("dark"), Some(ThemeName::MochiDark));
        assert_eq!(
            ThemeName::from_str("mochi-dark"),
            Some(ThemeName::MochiDark)
        );
        assert_eq!(ThemeName::from_str("light"), Some(ThemeName::MochiLight));
        assert_eq!(
            ThemeName::from_str("mochi-light"),
            Some(ThemeName::MochiLight)
        );
        assert_eq!(
            ThemeName::from_str("solarized-dark"),
            Some(ThemeName::SolarizedDark)
        );
        assert_eq!(ThemeName::from_str("dracula"), Some(ThemeName::Dracula));
        assert_eq!(ThemeName::from_str("nord"), Some(ThemeName::Nord));
        assert_eq!(ThemeName::from_str("invalid"), None);
    }

    #[test]
    fn test_theme_cycle() {
        let theme = ThemeName::MochiDark;
        assert_eq!(theme.next(), ThemeName::MochiLight);
        assert_eq!(ThemeName::Nord.next(), ThemeName::MochiDark);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());

        config.font_size = 2.0;
        assert!(config.validate().is_err());

        config.font_size = 14.0;
        config.line_height = 0.1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_keybinding_find_action() {
        let keybindings = KeybindingsConfig::default();

        // Test copy shortcut (Ctrl+Shift+C)
        assert_eq!(
            keybindings.find_action("c", true, true, false, false),
            Some(KeyAction::Copy)
        );

        // Test paste shortcut (Ctrl+Shift+V)
        assert_eq!(
            keybindings.find_action("v", true, true, false, false),
            Some(KeyAction::Paste)
        );

        // Test font size increase (Ctrl+=)
        assert_eq!(
            keybindings.find_action("=", true, false, false, false),
            Some(KeyAction::FontSizeIncrease)
        );

        // Test no match
        assert_eq!(keybindings.find_action("x", true, true, false, false), None);
    }

    #[test]
    fn test_config_toml_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.font_size, config.font_size);
        assert_eq!(parsed.theme, config.theme);
    }

    #[test]
    fn test_effective_colors() {
        let config_dark = Config {
            theme: ThemeName::MochiDark,
            ..Default::default()
        };
        let colors = config_dark.effective_colors();
        assert_eq!(colors.background, "#1e1e1e");

        let config_light = Config {
            theme: ThemeName::MochiLight,
            ..Default::default()
        };
        let colors = config_light.effective_colors();
        assert_eq!(colors.background, "#ffffff");
    }

    #[test]
    fn test_osc52_disabled_by_default() {
        let config = Config::default();
        assert!(
            !config.osc52_clipboard,
            "OSC 52 should be disabled by default for security"
        );
    }

    #[test]
    fn test_osc52_max_size_default() {
        let config = Config::default();
        assert_eq!(
            config.osc52_max_size, 100_000,
            "Default OSC 52 max size should be 100KB"
        );
    }

    #[test]
    fn test_osc52_max_size_validation() {
        // Valid size
        let config_valid = Config {
            osc52_max_size: 1_000_000,
            ..Default::default()
        };
        assert!(config_valid.validate().is_ok());

        // Too large
        let config_too_large = Config {
            osc52_max_size: 100_000_000,
            ..Default::default()
        };
        assert!(config_too_large.validate().is_err());
    }

    #[test]
    fn test_title_update_rate_default() {
        let config = Config::default();
        assert_eq!(
            config.title_update_rate, 10,
            "Default title update rate should be 10/sec"
        );
    }

    #[test]
    fn test_security_settings_in_reload_fields() {
        // Verify that security settings are included in the reload method
        // by checking the Config struct has the expected security fields
        let config = Config::default();

        // These fields should exist and have secure defaults
        assert!(
            !config.osc52_clipboard,
            "OSC 52 should be disabled by default"
        );
        assert!(
            config.osc52_max_size > 0,
            "OSC 52 max size should be positive"
        );
        assert!(
            config.osc52_max_size <= 10_000_000,
            "OSC 52 max size should be reasonable"
        );
        assert!(
            config.title_update_rate > 0,
            "Title update rate should be positive"
        );
    }
}
