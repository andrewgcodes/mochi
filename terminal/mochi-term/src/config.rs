//! Configuration for Mochi Terminal
//!
//! This module provides a comprehensive configuration system with:
//! - XDG Base Directory support for config file location
//! - CLI argument parsing with clap
//! - Environment variable overrides
//! - Strongly typed configuration with validation
//! - Clear precedence: CLI > env vars > config file > defaults

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    #[error("Invalid color format '{0}': expected #RRGGBB")]
    InvalidColor(String),

    #[error("Invalid keybinding format '{0}': expected format like 'ctrl+shift+c'")]
    InvalidKeybinding(String),
}

/// CLI arguments for Mochi Terminal
#[derive(Parser, Debug, Clone)]
#[command(name = "mochi")]
#[command(author = "Mochi Terminal Contributors")]
#[command(version)]
#[command(about = "A modern, customizable terminal emulator", long_about = None)]
#[derive(Default)]
pub struct CliArgs {
    /// Path to configuration file (overrides XDG default)
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

    /// Initial window columns
    #[arg(long)]
    pub columns: Option<u16>,

    /// Initial window rows
    #[arg(long)]
    pub rows: Option<u16>,

    /// Enable debug logging
    #[arg(short, long)]
    pub debug: bool,
}

/// Available theme names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
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
    /// Parse theme name from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dark" | "mochi-dark" => Some(ThemeName::Dark),
            "light" | "mochi-light" => Some(ThemeName::Light),
            "solarized-dark" => Some(ThemeName::SolarizedDark),
            "solarized-light" => Some(ThemeName::SolarizedLight),
            "dracula" => Some(ThemeName::Dracula),
            "nord" => Some(ThemeName::Nord),
            "custom" => Some(ThemeName::Custom),
            _ => None,
        }
    }

    /// Get all available theme names
    #[allow(dead_code)]
    pub fn all() -> &'static [&'static str] {
        &[
            "dark",
            "light",
            "solarized-dark",
            "solarized-light",
            "dracula",
            "nord",
        ]
    }

    /// Cycle to the next theme
    pub fn next(self) -> Self {
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

/// Keybinding action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    ClearScrollback,
}

/// A keybinding specification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Keybinding {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
    pub key: String,
}

impl Keybinding {
    /// Parse a keybinding string like "ctrl+shift+c"
    pub fn parse(s: &str) -> Result<Self, ConfigError> {
        let lower = s.to_lowercase();
        let parts: Vec<&str> = lower.split('+').collect();
        if parts.is_empty() {
            return Err(ConfigError::InvalidKeybinding(s.to_string()));
        }

        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut super_key = false;
        let mut key = String::new();

        for (i, part) in parts.iter().enumerate() {
            let part = part.trim();
            if i == parts.len() - 1 {
                // Last part is the key
                key = part.to_string();
            } else {
                // Modifier
                match part {
                    "ctrl" | "control" => ctrl = true,
                    "alt" | "meta" => alt = true,
                    "shift" => shift = true,
                    "super" | "win" | "cmd" => super_key = true,
                    _ => return Err(ConfigError::InvalidKeybinding(s.to_string())),
                }
            }
        }

        if key.is_empty() {
            return Err(ConfigError::InvalidKeybinding(s.to_string()));
        }

        Ok(Self {
            ctrl,
            alt,
            shift,
            super_key,
            key,
        })
    }

    /// Check if this keybinding matches the given modifiers and key
    pub fn matches(&self, ctrl: bool, alt: bool, shift: bool, super_key: bool, key: &str) -> bool {
        self.ctrl == ctrl
            && self.alt == alt
            && self.shift == shift
            && self.super_key == super_key
            && self.key.eq_ignore_ascii_case(key)
    }
}

impl Serialize for Keybinding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("ctrl");
        }
        if self.alt {
            parts.push("alt");
        }
        if self.shift {
            parts.push("shift");
        }
        if self.super_key {
            parts.push("super");
        }
        parts.push(&self.key);
        serializer.serialize_str(&parts.join("+"))
    }
}

impl<'de> Deserialize<'de> for Keybinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Keybinding::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// Keybinding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingConfig {
    pub copy: Keybinding,
    pub paste: Keybinding,
    pub find: Keybinding,
    pub reload_config: Keybinding,
    pub toggle_theme: Keybinding,
    pub font_size_increase: Keybinding,
    pub font_size_decrease: Keybinding,
    pub font_size_reset: Keybinding,
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            copy: Keybinding::parse("ctrl+shift+c").unwrap(),
            paste: Keybinding::parse("ctrl+shift+v").unwrap(),
            find: Keybinding::parse("ctrl+shift+f").unwrap(),
            reload_config: Keybinding::parse("ctrl+shift+r").unwrap(),
            toggle_theme: Keybinding::parse("ctrl+shift+t").unwrap(),
            font_size_increase: Keybinding::parse("ctrl+=").unwrap(),
            font_size_decrease: Keybinding::parse("ctrl+-").unwrap(),
            font_size_reset: Keybinding::parse("ctrl+0").unwrap(),
        }
    }
}

impl KeybindingConfig {
    /// Get the action for a key event, if any
    pub fn get_action(
        &self,
        ctrl: bool,
        alt: bool,
        shift: bool,
        super_key: bool,
        key: &str,
    ) -> Option<KeyAction> {
        if self.copy.matches(ctrl, alt, shift, super_key, key) {
            return Some(KeyAction::Copy);
        }
        if self.paste.matches(ctrl, alt, shift, super_key, key) {
            return Some(KeyAction::Paste);
        }
        if self.find.matches(ctrl, alt, shift, super_key, key) {
            return Some(KeyAction::Find);
        }
        if self.reload_config.matches(ctrl, alt, shift, super_key, key) {
            return Some(KeyAction::ReloadConfig);
        }
        if self.toggle_theme.matches(ctrl, alt, shift, super_key, key) {
            return Some(KeyAction::ToggleTheme);
        }
        if self
            .font_size_increase
            .matches(ctrl, alt, shift, super_key, key)
        {
            return Some(KeyAction::FontSizeIncrease);
        }
        if self
            .font_size_decrease
            .matches(ctrl, alt, shift, super_key, key)
        {
            return Some(KeyAction::FontSizeDecrease);
        }
        if self
            .font_size_reset
            .matches(ctrl, alt, shift, super_key, key)
        {
            return Some(KeyAction::FontSizeReset);
        }
        None
    }

    /// Build a map from keybindings to actions for quick lookup
    #[allow(dead_code)]
    pub fn to_action_map(&self) -> HashMap<Keybinding, KeyAction> {
        let mut map = HashMap::new();
        map.insert(self.copy.clone(), KeyAction::Copy);
        map.insert(self.paste.clone(), KeyAction::Paste);
        map.insert(self.find.clone(), KeyAction::Find);
        map.insert(self.reload_config.clone(), KeyAction::ReloadConfig);
        map.insert(self.toggle_theme.clone(), KeyAction::ToggleTheme);
        map.insert(self.font_size_increase.clone(), KeyAction::FontSizeIncrease);
        map.insert(self.font_size_decrease.clone(), KeyAction::FontSizeDecrease);
        map.insert(self.font_size_reset.clone(), KeyAction::FontSizeReset);
        map
    }
}

/// Font configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    /// Primary font family name
    pub family: String,
    /// Font size in points
    pub size: f32,
    /// Fallback font families (tried in order if primary lacks glyphs)
    #[serde(default)]
    pub fallback: Vec<String>,
    /// Cell padding (horizontal, vertical) in pixels
    #[serde(default)]
    pub cell_padding: (f32, f32),
    /// Line height multiplier (1.0 = normal)
    #[serde(default = "default_line_height")]
    pub line_height: f32,
    /// Enable font ligatures (if supported)
    #[serde(default)]
    pub ligatures: bool,
}

fn default_line_height() -> f32 {
    1.0
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "monospace".to_string(),
            size: 14.0,
            fallback: vec![
                "DejaVu Sans Mono".to_string(),
                "Noto Mono".to_string(),
                "Liberation Mono".to_string(),
            ],
            cell_padding: (0.0, 0.0),
            line_height: 1.0,
            ligatures: false,
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
    pub clipboard_notification: bool,
    /// Maximum title update rate (updates per second, 0 = unlimited)
    #[serde(default = "default_title_rate_limit")]
    pub title_rate_limit: u32,
    /// Maximum title length
    #[serde(default = "default_title_max_length")]
    pub title_max_length: usize,
}

fn default_osc52_max_size() -> usize {
    100_000
}

fn default_true() -> bool {
    true
}

fn default_title_rate_limit() -> u32 {
    10
}

fn default_title_max_length() -> usize {
    256
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            osc52_clipboard: false,
            osc52_max_size: default_osc52_max_size(),
            clipboard_notification: true,
            title_rate_limit: default_title_rate_limit(),
            title_max_length: default_title_max_length(),
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

    /// Theme name (dark, light, solarized-dark, solarized-light, dracula, nord, custom)
    #[serde(default)]
    pub theme: ThemeName,

    /// Color scheme (used when theme is "custom", otherwise ignored)
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

    /// Keybinding configuration
    #[serde(default)]
    pub keybindings: KeybindingConfig,

    /// Security configuration
    #[serde(default)]
    pub security: SecurityConfig,
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
            font: FontConfig::default(),
            scrollback_lines: default_scrollback_lines(),
            dimensions: default_dimensions(),
            theme: ThemeName::Dark,
            colors: ColorScheme::default(),
            shell: None,
            cursor_style: default_cursor_style(),
            cursor_blink: true,
            keybindings: KeybindingConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration with full precedence handling
    ///
    /// Precedence (highest to lowest):
    /// 1. CLI arguments
    /// 2. Environment variables
    /// 3. Config file
    /// 4. Built-in defaults
    pub fn load_with_args(args: &CliArgs) -> Result<Self, ConfigError> {
        // Start with defaults
        let mut config = Config::default();

        // Load from config file (if exists)
        let config_path = args.config.clone().or_else(Self::default_config_path);

        if let Some(path) = config_path {
            if path.exists() {
                log::info!("Loading config from: {}", path.display());
                let content = fs::read_to_string(&path)?;
                config = toml::from_str(&content)?;
            } else if args.config.is_some() {
                // User explicitly specified a config file that doesn't exist
                return Err(ConfigError::ValidationError(format!(
                    "Config file not found: {}",
                    path.display()
                )));
            }
        }

        // Apply environment variable overrides
        config.apply_env_overrides();

        // Apply CLI argument overrides
        config.apply_cli_overrides(args);

        // Validate the final configuration
        config.validate()?;

        Ok(config)
    }

    /// Load configuration from default location (for backwards compatibility)
    #[allow(dead_code)]
    pub fn load() -> Option<Self> {
        let args = CliArgs::default();
        Self::load_with_args(&args).ok()
    }

    /// Get the default XDG config path
    pub fn default_config_path() -> Option<PathBuf> {
        // Check XDG_CONFIG_HOME first, then fall back to ~/.config
        if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
            let path = PathBuf::from(xdg_config).join("mochi").join("config.toml");
            return Some(path);
        }

        dirs::config_dir().map(|p| p.join("mochi").join("config.toml"))
    }

    /// Get the config directory path
    #[allow(dead_code)]
    pub fn config_dir() -> Option<PathBuf> {
        if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg_config).join("mochi"));
        }

        dirs::config_dir().map(|p| p.join("mochi"))
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        if let Ok(theme) = env::var("MOCHI_THEME") {
            if let Some(t) = ThemeName::from_str(&theme) {
                self.theme = t;
            }
        }

        if let Ok(font_size) = env::var("MOCHI_FONT_SIZE") {
            if let Ok(size) = font_size.parse::<f32>() {
                self.font.size = size;
            }
        }

        if let Ok(font_family) = env::var("MOCHI_FONT_FAMILY") {
            self.font.family = font_family;
        }

        if let Ok(shell) = env::var("MOCHI_SHELL") {
            self.shell = Some(shell);
        }

        if let Ok(osc52) = env::var("MOCHI_OSC52_CLIPBOARD") {
            self.security.osc52_clipboard = osc52 == "1" || osc52.to_lowercase() == "true";
        }
    }

    /// Apply CLI argument overrides
    fn apply_cli_overrides(&mut self, args: &CliArgs) {
        if let Some(ref theme) = args.theme {
            if let Some(t) = ThemeName::from_str(theme) {
                self.theme = t;
            }
        }

        if let Some(font_size) = args.font_size {
            self.font.size = font_size;
        }

        if let Some(ref font_family) = args.font_family {
            self.font.family = font_family.clone();
        }

        if let Some(ref shell) = args.shell {
            self.shell = Some(shell.clone());
        }

        if let Some(columns) = args.columns {
            self.dimensions.0 = columns;
        }

        if let Some(rows) = args.rows {
            self.dimensions.1 = rows;
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate font size
        if self.font.size < 4.0 || self.font.size > 144.0 {
            return Err(ConfigError::ValidationError(format!(
                "Font size must be between 4 and 144, got {}",
                self.font.size
            )));
        }

        // Validate dimensions
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

        // Validate scrollback lines
        if self.scrollback_lines > 1_000_000 {
            return Err(ConfigError::ValidationError(format!(
                "Scrollback lines must be at most 1000000, got {}",
                self.scrollback_lines
            )));
        }

        // Validate colors if using custom theme
        if self.theme == ThemeName::Custom {
            self.colors.validate()?;
        }

        // Validate line height
        if self.font.line_height < 0.5 || self.font.line_height > 3.0 {
            return Err(ConfigError::ValidationError(format!(
                "Line height must be between 0.5 and 3.0, got {}",
                self.font.line_height
            )));
        }

        Ok(())
    }

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

    /// Reload configuration from file, keeping CLI overrides
    pub fn reload(&mut self, args: &CliArgs) -> Result<(), ConfigError> {
        let new_config = Self::load_with_args(args)?;
        *self = new_config;
        Ok(())
    }

    // Legacy accessors for backwards compatibility
    #[allow(dead_code)]
    pub fn font_family(&self) -> &str {
        &self.font.family
    }

    #[allow(dead_code)]
    pub fn font_size(&self) -> f32 {
        self.font.size
    }

    #[allow(dead_code)]
    pub fn osc52_clipboard(&self) -> bool {
        self.security.osc52_clipboard
    }

    #[allow(dead_code)]
    pub fn osc52_max_size(&self) -> usize {
        self.security.osc52_max_size
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

impl ColorScheme {
    /// Validate color scheme
    pub fn validate(&self) -> Result<(), ConfigError> {
        Self::validate_color(&self.foreground)?;
        Self::validate_color(&self.background)?;
        Self::validate_color(&self.cursor)?;
        Self::validate_color(&self.selection)?;
        for (i, color) in self.ansi.iter().enumerate() {
            Self::validate_color(color)
                .map_err(|_| ConfigError::InvalidColor(format!("ansi[{}]: {}", i, color)))?;
        }
        Ok(())
    }

    /// Validate a single color string
    fn validate_color(color: &str) -> Result<(), ConfigError> {
        if Self::parse_hex(color).is_none() {
            return Err(ConfigError::InvalidColor(color.to_string()));
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
    fn test_keybinding_parse() {
        let kb = Keybinding::parse("ctrl+shift+c").unwrap();
        assert!(kb.ctrl);
        assert!(kb.shift);
        assert!(!kb.alt);
        assert!(!kb.super_key);
        assert_eq!(kb.key, "c");

        let kb2 = Keybinding::parse("alt+f").unwrap();
        assert!(!kb2.ctrl);
        assert!(!kb2.shift);
        assert!(kb2.alt);
        assert_eq!(kb2.key, "f");

        let kb3 = Keybinding::parse("ctrl+=").unwrap();
        assert!(kb3.ctrl);
        assert_eq!(kb3.key, "=");
    }

    #[test]
    fn test_keybinding_matches() {
        let kb = Keybinding::parse("ctrl+shift+c").unwrap();
        assert!(kb.matches(true, false, true, false, "c"));
        assert!(kb.matches(true, false, true, false, "C"));
        assert!(!kb.matches(true, false, false, false, "c"));
        assert!(!kb.matches(false, false, true, false, "c"));
    }

    #[test]
    fn test_keybinding_config_get_action() {
        let config = KeybindingConfig::default();
        assert_eq!(
            config.get_action(true, false, true, false, "c"),
            Some(KeyAction::Copy)
        );
        assert_eq!(
            config.get_action(true, false, true, false, "v"),
            Some(KeyAction::Paste)
        );
        assert_eq!(
            config.get_action(true, false, true, false, "f"),
            Some(KeyAction::Find)
        );
        assert_eq!(
            config.get_action(true, false, true, false, "r"),
            Some(KeyAction::ReloadConfig)
        );
        assert_eq!(
            config.get_action(true, false, true, false, "t"),
            Some(KeyAction::ToggleTheme)
        );
        assert_eq!(config.get_action(false, false, false, false, "a"), None);
    }

    #[test]
    fn test_theme_name_from_str() {
        assert_eq!(ThemeName::from_str("dark"), Some(ThemeName::Dark));
        assert_eq!(ThemeName::from_str("mochi-dark"), Some(ThemeName::Dark));
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
    fn test_theme_name_next() {
        assert_eq!(ThemeName::Dark.next(), ThemeName::Light);
        assert_eq!(ThemeName::Light.next(), ThemeName::SolarizedDark);
        assert_eq!(ThemeName::Nord.next(), ThemeName::Dark);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());

        config.font.size = 2.0;
        assert!(config.validate().is_err());

        config.font.size = 14.0;
        config.dimensions.0 = 5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_parse_toml() {
        let toml_str = r#"
            scrollback_lines = 5000
            theme = "light"
            
            [font]
            family = "JetBrains Mono"
            size = 16.0
            
            [security]
            osc52_clipboard = true
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.scrollback_lines, 5000);
        assert_eq!(config.theme, ThemeName::Light);
        assert_eq!(config.font.family, "JetBrains Mono");
        assert_eq!(config.font.size, 16.0);
        assert!(config.security.osc52_clipboard);
    }

    #[test]
    fn test_color_scheme_validation() {
        let mut scheme = ColorScheme::default();
        assert!(scheme.validate().is_ok());

        scheme.foreground = "invalid".to_string();
        assert!(scheme.validate().is_err());
    }

    #[test]
    fn test_keybinding_serialization() {
        let kb = Keybinding::parse("ctrl+shift+c").unwrap();

        // Test that the keybinding can be serialized as part of a struct
        #[derive(Serialize, Deserialize)]
        struct TestWrapper {
            binding: Keybinding,
        }

        let wrapper = TestWrapper {
            binding: kb.clone(),
        };
        let serialized = toml::to_string(&wrapper).unwrap();
        assert!(serialized.contains("ctrl+shift+c"));

        let deserialized: TestWrapper = toml::from_str(&serialized).unwrap();
        assert_eq!(kb, deserialized.binding);
    }

    #[test]
    fn test_cli_args_default() {
        let args = CliArgs::default();
        assert!(args.config.is_none());
        assert!(args.theme.is_none());
        assert!(args.font_size.is_none());
    }

    #[test]
    fn test_effective_colors() {
        let config = Config::default();
        let colors = config.effective_colors();
        assert_eq!(colors.background, "#1e1e1e");

        let light_config = Config {
            theme: ThemeName::Light,
            ..Default::default()
        };
        let light_colors = light_config.effective_colors();
        assert_eq!(light_colors.background, "#ffffff");
    }
}
