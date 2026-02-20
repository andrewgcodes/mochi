//! Configuration for Mochi Terminal
//!
//! This module provides a comprehensive configuration system with:
//! - XDG-compliant config file location
//! - CLI argument overrides
//! - Environment variable support
//! - Config precedence: CLI > env > file > defaults
//! - Detailed validation and error messages

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
#[command(about = "A modern, customizable terminal emulator", long_about = None)]
pub struct CliArgs {
    /// Path to custom config file
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Font family name
    #[arg(long, value_name = "FONT")]
    pub font_family: Option<String>,

    /// Font size in points
    #[arg(long, value_name = "SIZE")]
    pub font_size: Option<f32>,

    /// Theme name (dark, light, solarized-dark, solarized-light, dracula, nord)
    #[arg(short, long, value_name = "THEME")]
    pub theme: Option<String>,

    /// Shell command to run
    #[arg(short, long, value_name = "SHELL")]
    pub shell: Option<String>,

    /// Number of scrollback lines
    #[arg(long, value_name = "LINES")]
    pub scrollback: Option<usize>,

    /// Initial window columns
    #[arg(long, value_name = "COLS")]
    pub columns: Option<u16>,

    /// Initial window rows
    #[arg(long, value_name = "ROWS")]
    pub rows: Option<u16>,

    /// Enable OSC 52 clipboard (security risk)
    #[arg(long)]
    pub enable_osc52: bool,
}

/// Available theme names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeName {
    /// Mochi theme - cute pink kawaii aesthetic (default)
    #[default]
    Mochi,
    /// Dark theme
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
    /// Parse theme name from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mochi" => Some(ThemeName::Mochi),
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
    #[allow(dead_code)] // Will be used for theme listing UI
    pub fn all_names() -> &'static [&'static str] {
        &[
            "mochi",
            "dark",
            "light",
            "solarized-dark",
            "solarized-light",
            "dracula",
            "nord",
            "custom",
        ]
    }

    pub fn display_name(self) -> &'static str {
        match self {
            ThemeName::Mochi => "mochi",
            ThemeName::Dark => "dark",
            ThemeName::Light => "light",
            ThemeName::SolarizedDark => "sol-dark",
            ThemeName::SolarizedLight => "sol-light",
            ThemeName::Dracula => "dracula",
            ThemeName::Nord => "nord",
            ThemeName::Custom => "custom",
        }
    }

    /// Cycle to the next theme (for toggle keybinding)
    pub fn next(self) -> Self {
        match self {
            ThemeName::Mochi => ThemeName::Dark,
            ThemeName::Dark => ThemeName::Light,
            ThemeName::Light => ThemeName::SolarizedDark,
            ThemeName::SolarizedDark => ThemeName::SolarizedLight,
            ThemeName::SolarizedLight => ThemeName::Dracula,
            ThemeName::Dracula => ThemeName::Nord,
            ThemeName::Nord => ThemeName::Mochi,
            ThemeName::Custom => ThemeName::Mochi,
        }
    }
}

/// Keybinding action
#[allow(dead_code)] // Will be used when keybinding parsing is implemented
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyAction {
    Copy,
    Paste,
    Find,
    ReloadConfig,
    ToggleTheme,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ScrollUp,
    ScrollDown,
    ScrollPageUp,
    ScrollPageDown,
    ScrollToTop,
    ScrollToBottom,
    ClearScrollback,
}

/// Keybinding configuration
#[allow(dead_code)] // Will be used when keybinding parsing is implemented
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybinding {
    /// Key combination (e.g., "ctrl+shift+c")
    pub key: String,
    /// Action to perform
    pub action: KeyAction,
}

impl Default for Keybinding {
    fn default() -> Self {
        Self {
            key: String::new(),
            action: KeyAction::Copy,
        }
    }
}

/// Keybindings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    /// Copy selection to clipboard
    #[serde(default = "default_copy_key")]
    pub copy: String,
    /// Paste from clipboard
    #[serde(default = "default_paste_key")]
    pub paste: String,
    /// Open search/find bar
    #[serde(default = "default_find_key")]
    pub find: String,
    /// Reload configuration
    #[serde(default = "default_reload_key")]
    pub reload_config: String,
    /// Toggle theme (cycle through themes)
    #[serde(default = "default_toggle_theme_key")]
    pub toggle_theme: String,
    /// Zoom in (increase font size)
    #[serde(default = "default_zoom_in_key")]
    pub zoom_in: String,
    /// Zoom out (decrease font size)
    #[serde(default = "default_zoom_out_key")]
    pub zoom_out: String,
    /// Reset zoom to default
    #[serde(default = "default_zoom_reset_key")]
    pub zoom_reset: String,
}

fn default_copy_key() -> String {
    "ctrl+shift+c".to_string()
}
fn default_paste_key() -> String {
    "ctrl+shift+v".to_string()
}
fn default_find_key() -> String {
    "ctrl+shift+f".to_string()
}
fn default_reload_key() -> String {
    "ctrl+shift+r".to_string()
}
fn default_toggle_theme_key() -> String {
    "ctrl+shift+t".to_string()
}
fn default_zoom_in_key() -> String {
    "ctrl+plus".to_string()
}
fn default_zoom_out_key() -> String {
    "ctrl+minus".to_string()
}
fn default_zoom_reset_key() -> String {
    "ctrl+0".to_string()
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            copy: default_copy_key(),
            paste: default_paste_key(),
            find: default_find_key(),
            reload_config: default_reload_key(),
            toggle_theme: default_toggle_theme_key(),
            zoom_in: default_zoom_in_key(),
            zoom_out: default_zoom_out_key(),
            zoom_reset: default_zoom_reset_key(),
        }
    }
}

/// Font configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    /// Primary font family name
    #[serde(default = "default_font_family")]
    pub family: String,
    /// Font size in points
    #[serde(default = "default_font_size")]
    pub size: f32,
    /// Fallback font families (tried in order if primary is missing)
    #[serde(default = "default_font_fallbacks")]
    pub fallbacks: Vec<String>,
    /// Extra horizontal padding per cell (pixels)
    #[serde(default)]
    pub cell_padding_x: u32,
    /// Extra vertical padding per cell (pixels)
    #[serde(default)]
    pub cell_padding_y: u32,
    /// Line height multiplier (1.0 = normal)
    #[serde(default = "default_line_height")]
    pub line_height: f32,
}

fn default_font_family() -> String {
    "monospace".to_string()
}
fn default_font_size() -> f32 {
    14.0
}
fn default_font_fallbacks() -> Vec<String> {
    vec![
        "DejaVu Sans Mono".to_string(),
        "Liberation Mono".to_string(),
        "Courier New".to_string(),
    ]
}
fn default_line_height() -> f32 {
    1.0
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: default_font_family(),
            size: default_font_size(),
            fallbacks: default_font_fallbacks(),
            cell_padding_x: 0,
            cell_padding_y: 0,
            line_height: default_line_height(),
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable OSC 52 clipboard sequences (disabled by default for security)
    #[serde(default)]
    pub osc52_clipboard: bool,
    /// Maximum OSC 52 payload size in bytes
    #[serde(default = "default_osc52_max_size")]
    pub osc52_max_size: usize,
    /// Show notification when clipboard is modified by escape sequence
    #[serde(default = "default_true")]
    pub osc52_notify: bool,
    /// Maximum title updates per second (throttling)
    #[serde(default = "default_title_update_rate")]
    pub title_update_rate: u32,
}

fn default_osc52_max_size() -> usize {
    100_000
}
fn default_true() -> bool {
    true
}
fn default_title_update_rate() -> u32 {
    10
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            osc52_clipboard: false,
            osc52_max_size: default_osc52_max_size(),
            osc52_notify: true,
            title_update_rate: default_title_update_rate(),
        }
    }
}

/// Terminal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Font configuration
    #[serde(default)]
    pub font: FontConfig,

    /// Number of scrollback lines
    #[serde(default = "default_scrollback_lines")]
    pub scrollback_lines: usize,

    /// Window dimensions (columns, rows)
    #[serde(default = "default_dimensions")]
    pub dimensions: (u16, u16),

    /// Theme name
    #[serde(default)]
    pub theme: ThemeName,

    /// Custom color scheme (used when theme is "custom")
    #[serde(default)]
    pub colors: ColorScheme,

    /// Shell command (None = use $SHELL)
    #[serde(default)]
    pub shell: Option<String>,

    /// Cursor style (block, underline, bar)
    #[serde(default = "default_cursor_style")]
    pub cursor_style: String,

    /// Cursor blink
    #[serde(default = "default_true")]
    pub cursor_blink: bool,

    /// Keybindings
    #[serde(default)]
    pub keybindings: KeybindingsConfig,

    /// Security settings
    #[serde(default)]
    pub security: SecurityConfig,

    // Legacy fields for backwards compatibility
    #[serde(skip_serializing, default)]
    font_family: Option<String>,
    #[serde(skip_serializing, default)]
    font_size: Option<f32>,
    #[serde(skip_serializing, default)]
    osc52_clipboard: Option<bool>,
    #[serde(skip_serializing, default)]
    osc52_max_size: Option<usize>,
}

fn default_scrollback_lines() -> usize {
    10000
}
fn default_dimensions() -> (u16, u16) {
    (80, 24)
}
fn default_cursor_style() -> String {
    "block".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font: FontConfig::default(),
            scrollback_lines: default_scrollback_lines(),
            dimensions: default_dimensions(),
            theme: ThemeName::Mochi,
            colors: ColorScheme::default(),
            shell: None,
            cursor_style: default_cursor_style(),
            cursor_blink: true,
            keybindings: KeybindingsConfig::default(),
            security: SecurityConfig::default(),
            font_family: None,
            font_size: None,
            osc52_clipboard: None,
            osc52_max_size: None,
        }
    }
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

/// Configuration error
#[derive(Debug, Clone)]
pub struct ConfigError {
    pub message: String,
    pub field: Option<String>,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(field) = &self.field {
            write!(f, "Config error in '{}': {}", field, self.message)
        } else {
            write!(f, "Config error: {}", self.message)
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// Load configuration with full precedence:
    /// CLI args > environment variables > config file > defaults
    pub fn load_with_args(args: &CliArgs) -> Result<Self, ConfigError> {
        // Start with defaults
        let mut config = Config::default();

        // Load from config file (if exists)
        let config_path = args.config.clone().or_else(Self::default_config_path);
        if let Some(path) = &config_path {
            if path.exists() {
                match Self::load_from_file(path) {
                    Ok(file_config) => config = file_config,
                    Err(e) => {
                        log::warn!("Failed to load config from {:?}: {}", path, e);
                        // Continue with defaults if config file is invalid
                    }
                }
            }
        }

        // Apply environment variables
        config.apply_env_vars();

        // Apply CLI arguments (highest priority)
        config.apply_cli_args(args);

        // Validate the final configuration
        config.validate()?;

        Ok(config)
    }

    /// Load configuration from file only (legacy method)
    pub fn load() -> Option<Self> {
        let config_path = Self::default_config_path()?;
        if !config_path.exists() {
            return None;
        }
        Self::load_from_file(&config_path).ok()
    }

    /// Load configuration from a specific file
    pub fn load_from_file(path: &PathBuf) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path).map_err(|e| ConfigError {
            message: format!("Failed to read config file: {}", e),
            field: None,
        })?;

        let mut config: Config = toml::from_str(&content).map_err(|e| ConfigError {
            message: format!("Failed to parse config file: {}", e),
            field: None,
        })?;

        // Handle legacy fields
        config.migrate_legacy_fields();

        Ok(config)
    }

    /// Migrate legacy config fields to new structure
    fn migrate_legacy_fields(&mut self) {
        if let Some(family) = self.font_family.take() {
            self.font.family = family;
        }
        if let Some(size) = self.font_size.take() {
            self.font.size = size;
        }
        if let Some(osc52) = self.osc52_clipboard.take() {
            self.security.osc52_clipboard = osc52;
        }
        if let Some(max_size) = self.osc52_max_size.take() {
            self.security.osc52_max_size = max_size;
        }
    }

    /// Apply environment variables to config
    fn apply_env_vars(&mut self) {
        if let Ok(val) = env::var("MOCHI_FONT_FAMILY") {
            self.font.family = val;
        }
        if let Ok(val) = env::var("MOCHI_FONT_SIZE") {
            if let Ok(size) = val.parse() {
                self.font.size = size;
            }
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
            if let Ok(lines) = val.parse() {
                self.scrollback_lines = lines;
            }
        }
        if let Ok(val) = env::var("MOCHI_OSC52_CLIPBOARD") {
            self.security.osc52_clipboard = val == "1" || val.to_lowercase() == "true";
        }
    }

    /// Apply CLI arguments to config
    fn apply_cli_args(&mut self, args: &CliArgs) {
        if let Some(family) = &args.font_family {
            self.font.family = family.clone();
        }
        if let Some(size) = args.font_size {
            self.font.size = size;
        }
        if let Some(theme_str) = &args.theme {
            if let Some(theme) = ThemeName::from_str(theme_str) {
                self.theme = theme;
            }
        }
        if let Some(shell) = &args.shell {
            self.shell = Some(shell.clone());
        }
        if let Some(scrollback) = args.scrollback {
            self.scrollback_lines = scrollback;
        }
        if let Some(cols) = args.columns {
            self.dimensions.0 = cols;
        }
        if let Some(rows) = args.rows {
            self.dimensions.1 = rows;
        }
        if args.enable_osc52 {
            self.security.osc52_clipboard = true;
        }
    }

    /// Validate configuration
    fn validate(&self) -> Result<(), ConfigError> {
        // Validate font size
        if self.font.size < 4.0 {
            return Err(ConfigError {
                message: "Font size must be at least 4.0".to_string(),
                field: Some("font.size".to_string()),
            });
        }
        if self.font.size > 200.0 {
            return Err(ConfigError {
                message: "Font size must be at most 200.0".to_string(),
                field: Some("font.size".to_string()),
            });
        }

        // Validate dimensions
        if self.dimensions.0 < 10 {
            return Err(ConfigError {
                message: "Window columns must be at least 10".to_string(),
                field: Some("dimensions".to_string()),
            });
        }
        if self.dimensions.1 < 3 {
            return Err(ConfigError {
                message: "Window rows must be at least 3".to_string(),
                field: Some("dimensions".to_string()),
            });
        }

        // Validate scrollback
        if self.scrollback_lines > 10_000_000 {
            return Err(ConfigError {
                message: "Scrollback lines must be at most 10,000,000".to_string(),
                field: Some("scrollback_lines".to_string()),
            });
        }

        // Validate line height
        if self.font.line_height < 0.5 {
            return Err(ConfigError {
                message: "Line height must be at least 0.5".to_string(),
                field: Some("font.line_height".to_string()),
            });
        }
        if self.font.line_height > 3.0 {
            return Err(ConfigError {
                message: "Line height must be at most 3.0".to_string(),
                field: Some("font.line_height".to_string()),
            });
        }

        // Validate colors
        self.validate_color(&self.colors.foreground, "colors.foreground")?;
        self.validate_color(&self.colors.background, "colors.background")?;
        self.validate_color(&self.colors.cursor, "colors.cursor")?;
        self.validate_color(&self.colors.selection, "colors.selection")?;
        for (i, color) in self.colors.ansi.iter().enumerate() {
            self.validate_color(color, &format!("colors.ansi[{}]", i))?;
        }

        Ok(())
    }

    /// Validate a hex color string
    fn validate_color(&self, color: &str, field: &str) -> Result<(), ConfigError> {
        if ColorScheme::parse_hex(color).is_none() {
            return Err(ConfigError {
                message: format!("Invalid hex color '{}'. Expected format: #RRGGBB", color),
                field: Some(field.to_string()),
            });
        }
        Ok(())
    }

    /// Get the default configuration file path
    pub fn default_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("mochi").join("config.toml"))
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

    /// Get the effective color scheme based on the theme setting
    pub fn effective_colors(&self) -> ColorScheme {
        match self.theme {
            ThemeName::Custom => self.colors.clone(),
            ThemeName::Mochi => ColorScheme::mochi(),
            ThemeName::Dark => ColorScheme::dark(),
            ThemeName::Light => ColorScheme::light(),
            ThemeName::SolarizedDark => ColorScheme::solarized_dark(),
            ThemeName::SolarizedLight => ColorScheme::solarized_light(),
            ThemeName::Dracula => ColorScheme::dracula(),
            ThemeName::Nord => ColorScheme::nord(),
        }
    }

    // Legacy accessors for backwards compatibility
    #[allow(dead_code)] // Will be used when font rendering is updated
    pub fn font_family(&self) -> &str {
        &self.font.family
    }

    pub fn font_size(&self) -> f32 {
        self.font.size
    }

    #[allow(dead_code)] // Will be used when OSC 52 handling is implemented
    pub fn osc52_clipboard(&self) -> bool {
        self.security.osc52_clipboard
    }

    #[allow(dead_code)] // Will be used when OSC 52 handling is implemented
    pub fn osc52_max_size(&self) -> usize {
        self.security.osc52_max_size
    }
}

impl ColorScheme {
    /// Mochi theme - cute pink kawaii aesthetic
    /// A soft, gentle color scheme inspired by Japanese mochi rice cakes
    pub fn mochi() -> Self {
        Self {
            foreground: "#5c4d5c".to_string(), // Soft plum for readable text
            background: "#fff5f5".to_string(), // Rose white - very light pink
            cursor: "#ff8fab".to_string(),     // Soft pink cursor
            selection: "#ffd6e0".to_string(),  // Light pink selection
            ansi: [
                "#5c4d5c".to_string(), // Black - dark plum
                "#e64980".to_string(), // Red - soft rose
                "#40a070".to_string(), // Green - soft sage
                "#d9730d".to_string(), // Yellow - warm peach
                "#9775fa".to_string(), // Blue - soft lavender
                "#f06595".to_string(), // Magenta - pink
                "#22b8cf".to_string(), // Cyan - soft teal
                "#fff0f3".to_string(), // White - rose white
                "#8b7a8b".to_string(), // Bright Black - lighter plum
                "#ff8fa3".to_string(), // Bright Red - lighter rose
                "#69db7c".to_string(), // Bright Green - lighter sage
                "#ffa94d".to_string(), // Bright Yellow - lighter peach
                "#b197fc".to_string(), // Bright Blue - lighter lavender
                "#faa2c1".to_string(), // Bright Magenta - lighter pink
                "#66d9e8".to_string(), // Bright Cyan - lighter teal
                "#ffffff".to_string(), // Bright White - pure white
            ],
        }
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
        assert_eq!(config.font.size, 14.0);
        assert_eq!(config.dimensions, (80, 24));
        assert!(!config.security.osc52_clipboard);
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
        assert_eq!(ThemeName::from_str("dark"), Some(ThemeName::Dark));
        assert_eq!(ThemeName::from_str("light"), Some(ThemeName::Light));
        assert_eq!(
            ThemeName::from_str("solarized-dark"),
            Some(ThemeName::SolarizedDark)
        );
        assert_eq!(ThemeName::from_str("dracula"), Some(ThemeName::Dracula));
        assert_eq!(ThemeName::from_str("nord"), Some(ThemeName::Nord));
        assert_eq!(ThemeName::from_str("invalid"), None);
    }

    #[test]
    fn test_theme_next() {
        assert_eq!(ThemeName::Mochi.next(), ThemeName::Dark);
        assert_eq!(ThemeName::Dark.next(), ThemeName::Light);
        assert_eq!(ThemeName::Light.next(), ThemeName::SolarizedDark);
        assert_eq!(ThemeName::Nord.next(), ThemeName::Mochi);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();

        // Valid config should pass
        assert!(config.validate().is_ok());

        // Invalid font size
        config.font.size = 2.0;
        assert!(config.validate().is_err());
        config.font.size = 14.0;

        // Invalid dimensions
        config.dimensions = (5, 24);
        assert!(config.validate().is_err());
        config.dimensions = (80, 24);

        // Invalid color
        config.colors.foreground = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_keybindings_default() {
        let kb = KeybindingsConfig::default();
        assert_eq!(kb.copy, "ctrl+shift+c");
        assert_eq!(kb.paste, "ctrl+shift+v");
        assert_eq!(kb.find, "ctrl+shift+f");
        assert_eq!(kb.reload_config, "ctrl+shift+r");
        assert_eq!(kb.toggle_theme, "ctrl+shift+t");
    }

    #[test]
    fn test_config_toml_parsing() {
        let toml_str = r#"
            scrollback_lines = 5000
            theme = "dracula"
            
            [font]
            family = "JetBrains Mono"
            size = 16.0
            
            [keybindings]
            copy = "ctrl+c"
            paste = "ctrl+v"
            
            [security]
            osc52_clipboard = true
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.scrollback_lines, 5000);
        assert_eq!(config.theme, ThemeName::Dracula);
        assert_eq!(config.font.family, "JetBrains Mono");
        assert_eq!(config.font.size, 16.0);
        assert_eq!(config.keybindings.copy, "ctrl+c");
        assert!(config.security.osc52_clipboard);
    }

    #[test]
    fn test_legacy_config_migration() {
        // Test that old config format still works
        let toml_str = r##"
            font_family = "Fira Code"
            font_size = 12.0
            scrollback_lines = 5000
            dimensions = [100, 30]
            theme = "light"
            osc52_clipboard = true
            osc52_max_size = 50000
            cursor_style = "underline"
            cursor_blink = false
            
            [colors]
            foreground = "#333333"
            background = "#ffffff"
            cursor = "#000000"
            selection = "#add6ff"
            ansi = [
                "#000000", "#cd3131", "#00bc00", "#949800",
                "#0451a5", "#bc05bc", "#0598bc", "#555555",
                "#666666", "#cd3131", "#14ce14", "#b5ba00",
                "#0451a5", "#bc05bc", "#0598bc", "#a5a5a5"
            ]
        "##;

        let mut config: Config = toml::from_str(toml_str).unwrap();
        config.migrate_legacy_fields();

        assert_eq!(config.font.family, "Fira Code");
        assert_eq!(config.font.size, 12.0);
        assert!(config.security.osc52_clipboard);
        assert_eq!(config.security.osc52_max_size, 50000);
    }
}
