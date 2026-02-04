//! Configuration for Mochi Terminal
//!
//! Configuration is loaded with the following precedence (highest to lowest):
//! 1. CLI arguments
//! 2. Environment variables (MOCHI_*)
//! 3. Config file (~/.config/mochi/config.toml or --config path)
//! 4. Built-in defaults

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

/// Mochi Terminal - A VT/xterm-compatible terminal emulator
#[derive(Parser, Debug)]
#[command(name = "mochi")]
#[command(author, version, about, long_about = None)]
pub struct CliArgs {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Font size in points
    #[arg(long, value_name = "SIZE")]
    pub font_size: Option<f32>,

    /// Theme name (dark, light, solarized-dark, solarized-light, dracula, nord)
    #[arg(short, long, value_name = "THEME")]
    pub theme: Option<String>,

    /// Shell command to run
    #[arg(long, value_name = "SHELL")]
    pub shell: Option<String>,

    /// Number of scrollback lines
    #[arg(long, value_name = "LINES")]
    pub scrollback: Option<usize>,

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
    #[default]
    Dark,
    Light,
    #[serde(rename = "solarized-dark")]
    SolarizedDark,
    #[serde(rename = "solarized-light")]
    SolarizedLight,
    Dracula,
    Nord,
    Custom,
}

impl ThemeName {
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

    pub fn next(&self) -> Self {
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

/// Keybinding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybindings {
    /// Copy selection to clipboard (default: Ctrl+Shift+C)
    #[serde(default = "default_keybind_copy")]
    pub copy: String,
    /// Paste from clipboard (default: Ctrl+Shift+V)
    #[serde(default = "default_keybind_paste")]
    pub paste: String,
    /// Toggle theme (default: Ctrl+Shift+T)
    #[serde(default = "default_keybind_toggle_theme")]
    pub toggle_theme: String,
    /// Reload configuration (default: Ctrl+Shift+R)
    #[serde(default = "default_keybind_reload_config")]
    pub reload_config: String,
    /// Open search/find bar (default: Ctrl+Shift+F)
    #[serde(default = "default_keybind_find")]
    pub find: String,
    /// Increase font size (default: Ctrl+Plus)
    #[serde(default = "default_keybind_zoom_in")]
    pub zoom_in: String,
    /// Decrease font size (default: Ctrl+Minus)
    #[serde(default = "default_keybind_zoom_out")]
    pub zoom_out: String,
    /// Reset font size (default: Ctrl+0)
    #[serde(default = "default_keybind_zoom_reset")]
    pub zoom_reset: String,
}

fn default_keybind_copy() -> String {
    "Ctrl+Shift+C".to_string()
}

fn default_keybind_paste() -> String {
    "Ctrl+Shift+V".to_string()
}

fn default_keybind_toggle_theme() -> String {
    "Ctrl+Shift+T".to_string()
}

fn default_keybind_reload_config() -> String {
    "Ctrl+Shift+R".to_string()
}

fn default_keybind_find() -> String {
    "Ctrl+Shift+F".to_string()
}

fn default_keybind_zoom_in() -> String {
    "Ctrl+Plus".to_string()
}

fn default_keybind_zoom_out() -> String {
    "Ctrl+Minus".to_string()
}

fn default_keybind_zoom_reset() -> String {
    "Ctrl+0".to_string()
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            copy: default_keybind_copy(),
            paste: default_keybind_paste(),
            toggle_theme: default_keybind_toggle_theme(),
            reload_config: default_keybind_reload_config(),
            find: default_keybind_find(),
            zoom_in: default_keybind_zoom_in(),
            zoom_out: default_keybind_zoom_out(),
            zoom_reset: default_keybind_zoom_reset(),
        }
    }
}

/// Parsed keybinding for matching against key events
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedKeybinding {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub key: String,
}

impl ParsedKeybinding {
    /// Parse a keybinding string like "Ctrl+Shift+C" into components
    pub fn parse(s: &str) -> Option<Self> {
        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut key = String::new();

        for part in s.split('+') {
            let part = part.trim();
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "alt" => alt = true,
                "shift" => shift = true,
                "super" | "meta" | "cmd" => {} // Ignore super/meta for now
                _ => key = part.to_uppercase(),
            }
        }

        if key.is_empty() {
            return None;
        }

        Some(Self {
            ctrl,
            alt,
            shift,
            key,
        })
    }

    /// Check if this keybinding matches the given modifiers and key
    pub fn matches(&self, ctrl: bool, alt: bool, shift: bool, key: &str) -> bool {
        self.ctrl == ctrl && self.alt == alt && self.shift == shift && self.key == key.to_uppercase()
    }
}

/// Actions that can be triggered by keybindings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Copy,
    Paste,
    ToggleTheme,
    ReloadConfig,
    Find,
    ZoomIn,
    ZoomOut,
    ZoomReset,
}

impl Keybindings {
    /// Match a key event against configured keybindings and return the action
    pub fn match_key(&self, ctrl: bool, alt: bool, shift: bool, key: &str) -> Option<KeyAction> {
        if let Some(kb) = ParsedKeybinding::parse(&self.copy) {
            if kb.matches(ctrl, alt, shift, key) {
                return Some(KeyAction::Copy);
            }
        }
        if let Some(kb) = ParsedKeybinding::parse(&self.paste) {
            if kb.matches(ctrl, alt, shift, key) {
                return Some(KeyAction::Paste);
            }
        }
        if let Some(kb) = ParsedKeybinding::parse(&self.toggle_theme) {
            if kb.matches(ctrl, alt, shift, key) {
                return Some(KeyAction::ToggleTheme);
            }
        }
        if let Some(kb) = ParsedKeybinding::parse(&self.reload_config) {
            if kb.matches(ctrl, alt, shift, key) {
                return Some(KeyAction::ReloadConfig);
            }
        }
        if let Some(kb) = ParsedKeybinding::parse(&self.find) {
            if kb.matches(ctrl, alt, shift, key) {
                return Some(KeyAction::Find);
            }
        }
        if let Some(kb) = ParsedKeybinding::parse(&self.zoom_in) {
            if kb.matches(ctrl, alt, shift, key) {
                return Some(KeyAction::ZoomIn);
            }
        }
        if let Some(kb) = ParsedKeybinding::parse(&self.zoom_out) {
            if kb.matches(ctrl, alt, shift, key) {
                return Some(KeyAction::ZoomOut);
            }
        }
        if let Some(kb) = ParsedKeybinding::parse(&self.zoom_reset) {
            if kb.matches(ctrl, alt, shift, key) {
                return Some(KeyAction::ZoomReset);
            }
        }
        None
    }
}

/// Terminal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    /// Line height multiplier (1.0 = normal, 1.2 = 20% extra spacing)
    #[serde(default = "default_line_height")]
    pub line_height: f32,
    /// Cell padding in pixels (horizontal, vertical)
    #[serde(default = "default_cell_padding")]
    pub cell_padding: (f32, f32),
    #[serde(default = "default_scrollback_lines")]
    pub scrollback_lines: usize,
    #[serde(default = "default_dimensions")]
    pub dimensions: (u16, u16),
    #[serde(default)]
    pub theme: ThemeName,
    #[serde(default)]
    pub theme_file: Option<String>,
    #[serde(default)]
    pub colors: ColorScheme,
    #[serde(default)]
    pub keybindings: Keybindings,
    #[serde(default)]
    pub osc52_clipboard: bool,
    #[serde(default = "default_osc52_max_size")]
    pub osc52_max_size: usize,
    #[serde(default)]
    pub shell: Option<String>,
    #[serde(default = "default_cursor_style")]
    pub cursor_style: String,
    #[serde(default = "default_cursor_blink")]
    pub cursor_blink: bool,
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

fn default_cell_padding() -> (f32, f32) {
    (0.0, 0.0)
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
    #[serde(default = "default_foreground")]
    pub foreground: String,
    #[serde(default = "default_background")]
    pub background: String,
    #[serde(default = "default_cursor_color")]
    pub cursor: String,
    #[serde(default = "default_selection")]
    pub selection: String,
    #[serde(default = "default_ansi")]
    pub ansi: [String; 16],
}

fn default_foreground() -> String {
    "#d4d4d4".to_string()
}

fn default_background() -> String {
    "#1e1e1e".to_string()
}

fn default_cursor_color() -> String {
    "#ffffff".to_string()
}

fn default_selection() -> String {
    "#264f78".to_string()
}

fn default_ansi() -> [String; 16] {
    [
        "#000000".to_string(),
        "#cd3131".to_string(),
        "#0dbc79".to_string(),
        "#e5e510".to_string(),
        "#2472c8".to_string(),
        "#bc3fbc".to_string(),
        "#11a8cd".to_string(),
        "#e5e5e5".to_string(),
        "#666666".to_string(),
        "#f14c4c".to_string(),
        "#23d18b".to_string(),
        "#f5f543".to_string(),
        "#3b8eea".to_string(),
        "#d670d6".to_string(),
        "#29b8db".to_string(),
        "#ffffff".to_string(),
    ]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_family: default_font_family(),
            font_size: default_font_size(),
            line_height: default_line_height(),
            cell_padding: default_cell_padding(),
            scrollback_lines: default_scrollback_lines(),
            dimensions: default_dimensions(),
            theme: ThemeName::Dark,
            theme_file: None,
            colors: ColorScheme::default(),
            keybindings: Keybindings::default(),
            osc52_clipboard: false,
            osc52_max_size: default_osc52_max_size(),
            shell: None,
            cursor_style: default_cursor_style(),
            cursor_blink: default_cursor_blink(),
        }
    }
}

impl Config {
    pub fn effective_colors(&self) -> ColorScheme {
        // If a theme file is specified, try to load it
        if let Some(ref theme_path) = self.theme_file {
            match ColorScheme::load_from_file(theme_path) {
                Ok(scheme) => return scheme,
                Err(e) => {
                    log::warn!("Failed to load theme file '{}': {}", theme_path, e);
                    // Fall through to built-in themes
                }
            }
        }

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
            foreground: default_foreground(),
            background: default_background(),
            cursor: default_cursor_color(),
            selection: default_selection(),
            ansi: default_ansi(),
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    ReadError(std::io::Error),
    ParseError(toml::de::Error),
    ValidationError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::ReadError(e) => write!(f, "Failed to read config file: {}", e),
            ConfigError::ParseError(e) => write!(f, "Failed to parse config file: {}", e),
            ConfigError::ValidationError(msg) => write!(f, "Invalid configuration: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn load_with_args(args: &CliArgs) -> Result<Self, ConfigError> {
        let mut config = Config::default();

        let config_path = args.config.clone().or_else(Self::default_config_path);
        if let Some(path) = &config_path {
            if path.exists() {
                match Self::load_from_file(path) {
                    Ok(file_config) => config = file_config,
                    Err(e) => {
                        log::warn!("Failed to load config from {:?}: {}", path, e);
                        return Err(e);
                    }
                }
            }
        }

        config.apply_env_vars();
        config.apply_cli_args(args);
        config.validate()?;

        Ok(config)
    }

    pub fn load() -> Option<Self> {
        let config_path = Self::default_config_path()?;
        if !config_path.exists() {
            return None;
        }
        Self::load_from_file(&config_path).ok()
    }

    pub fn load_from_file(path: &PathBuf) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path).map_err(ConfigError::ReadError)?;
        let config: Config = toml::from_str(&content).map_err(ConfigError::ParseError)?;
        Ok(config)
    }

    fn apply_env_vars(&mut self) {
        if let Ok(val) = env::var("MOCHI_FONT_SIZE") {
            if let Ok(size) = val.parse::<f32>() {
                self.font_size = size;
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
            if let Ok(lines) = val.parse::<usize>() {
                self.scrollback_lines = lines;
            }
        }
        if let Ok(val) = env::var("MOCHI_OSC52_CLIPBOARD") {
            self.osc52_clipboard = val == "1" || val.to_lowercase() == "true";
        }
    }

    fn apply_cli_args(&mut self, args: &CliArgs) {
        if let Some(size) = args.font_size {
            self.font_size = size;
        }
        if let Some(ref theme_str) = args.theme {
            if let Some(theme) = ThemeName::from_str(theme_str) {
                self.theme = theme;
            }
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
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.font_size < 6.0 || self.font_size > 128.0 {
            return Err(ConfigError::ValidationError(format!(
                "font_size must be between 6 and 128, got {}",
                self.font_size
            )));
        }
        if self.line_height < 0.5 || self.line_height > 3.0 {
            return Err(ConfigError::ValidationError(format!(
                "line_height must be between 0.5 and 3.0, got {}",
                self.line_height
            )));
        }
        if self.cell_padding.0 < 0.0 || self.cell_padding.0 > 20.0 {
            return Err(ConfigError::ValidationError(format!(
                "cell_padding horizontal must be between 0 and 20, got {}",
                self.cell_padding.0
            )));
        }
        if self.cell_padding.1 < 0.0 || self.cell_padding.1 > 20.0 {
            return Err(ConfigError::ValidationError(format!(
                "cell_padding vertical must be between 0 and 20, got {}",
                self.cell_padding.1
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
        if self.scrollback_lines > 1_000_000 {
            return Err(ConfigError::ValidationError(format!(
                "scrollback_lines must be at most 1000000, got {}",
                self.scrollback_lines
            )));
        }
        if self.osc52_max_size > 10_000_000 {
            return Err(ConfigError::ValidationError(format!(
                "osc52_max_size must be at most 10000000, got {}",
                self.osc52_max_size
            )));
        }
        self.colors.validate()?;
        Ok(())
    }

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

    pub fn default_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("mochi").join("config.toml"))
    }
}

impl ColorScheme {
    /// Load a color scheme from a TOML file
    pub fn load_from_file(path: &str) -> Result<Self, ConfigError> {
        let path = PathBuf::from(path);
        
        // Handle relative paths - look in themes directory
        let full_path = if path.is_absolute() {
            path
        } else {
            // Try XDG config dir first
            if let Some(config_dir) = dirs::config_dir() {
                let theme_path = config_dir.join("mochi").join("themes").join(&path);
                if theme_path.exists() {
                    theme_path
                } else {
                    path
                }
            } else {
                path
            }
        };

        let content = fs::read_to_string(&full_path).map_err(ConfigError::ReadError)?;
        let scheme: ColorScheme = toml::from_str(&content).map_err(ConfigError::ParseError)?;
        scheme.validate()?;
        Ok(scheme)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if Self::parse_hex(&self.foreground).is_none() {
            return Err(ConfigError::ValidationError(format!(
                "Invalid foreground color: {}",
                self.foreground
            )));
        }
        if Self::parse_hex(&self.background).is_none() {
            return Err(ConfigError::ValidationError(format!(
                "Invalid background color: {}",
                self.background
            )));
        }
        if Self::parse_hex(&self.cursor).is_none() {
            return Err(ConfigError::ValidationError(format!(
                "Invalid cursor color: {}",
                self.cursor
            )));
        }
        if Self::parse_hex(&self.selection).is_none() {
            return Err(ConfigError::ValidationError(format!(
                "Invalid selection color: {}",
                self.selection
            )));
        }
        for (i, color) in self.ansi.iter().enumerate() {
            if Self::parse_hex(color).is_none() {
                return Err(ConfigError::ValidationError(format!(
                    "Invalid ANSI color {}: {}",
                    i, color
                )));
            }
        }
        Ok(())
    }

    pub fn dark() -> Self {
        Self::default()
    }

    pub fn light() -> Self {
        Self {
            foreground: "#333333".to_string(),
            background: "#ffffff".to_string(),
            cursor: "#000000".to_string(),
            selection: "#add6ff".to_string(),
            ansi: [
                "#000000".to_string(),
                "#cd3131".to_string(),
                "#00bc00".to_string(),
                "#949800".to_string(),
                "#0451a5".to_string(),
                "#bc05bc".to_string(),
                "#0598bc".to_string(),
                "#555555".to_string(),
                "#666666".to_string(),
                "#cd3131".to_string(),
                "#14ce14".to_string(),
                "#b5ba00".to_string(),
                "#0451a5".to_string(),
                "#bc05bc".to_string(),
                "#0598bc".to_string(),
                "#a5a5a5".to_string(),
            ],
        }
    }

    pub fn solarized_dark() -> Self {
        Self {
            foreground: "#839496".to_string(),
            background: "#002b36".to_string(),
            cursor: "#93a1a1".to_string(),
            selection: "#073642".to_string(),
            ansi: [
                "#073642".to_string(),
                "#dc322f".to_string(),
                "#859900".to_string(),
                "#b58900".to_string(),
                "#268bd2".to_string(),
                "#d33682".to_string(),
                "#2aa198".to_string(),
                "#eee8d5".to_string(),
                "#002b36".to_string(),
                "#cb4b16".to_string(),
                "#586e75".to_string(),
                "#657b83".to_string(),
                "#839496".to_string(),
                "#6c71c4".to_string(),
                "#93a1a1".to_string(),
                "#fdf6e3".to_string(),
            ],
        }
    }

    pub fn solarized_light() -> Self {
        Self {
            foreground: "#657b83".to_string(),
            background: "#fdf6e3".to_string(),
            cursor: "#586e75".to_string(),
            selection: "#eee8d5".to_string(),
            ansi: [
                "#073642".to_string(),
                "#dc322f".to_string(),
                "#859900".to_string(),
                "#b58900".to_string(),
                "#268bd2".to_string(),
                "#d33682".to_string(),
                "#2aa198".to_string(),
                "#eee8d5".to_string(),
                "#002b36".to_string(),
                "#cb4b16".to_string(),
                "#586e75".to_string(),
                "#657b83".to_string(),
                "#839496".to_string(),
                "#6c71c4".to_string(),
                "#93a1a1".to_string(),
                "#fdf6e3".to_string(),
            ],
        }
    }

    pub fn dracula() -> Self {
        Self {
            foreground: "#f8f8f2".to_string(),
            background: "#282a36".to_string(),
            cursor: "#f8f8f2".to_string(),
            selection: "#44475a".to_string(),
            ansi: [
                "#21222c".to_string(),
                "#ff5555".to_string(),
                "#50fa7b".to_string(),
                "#f1fa8c".to_string(),
                "#bd93f9".to_string(),
                "#ff79c6".to_string(),
                "#8be9fd".to_string(),
                "#f8f8f2".to_string(),
                "#6272a4".to_string(),
                "#ff6e6e".to_string(),
                "#69ff94".to_string(),
                "#ffffa5".to_string(),
                "#d6acff".to_string(),
                "#ff92df".to_string(),
                "#a4ffff".to_string(),
                "#ffffff".to_string(),
            ],
        }
    }

    pub fn nord() -> Self {
        Self {
            foreground: "#d8dee9".to_string(),
            background: "#2e3440".to_string(),
            cursor: "#d8dee9".to_string(),
            selection: "#434c5e".to_string(),
            ansi: [
                "#3b4252".to_string(),
                "#bf616a".to_string(),
                "#a3be8c".to_string(),
                "#ebcb8b".to_string(),
                "#81a1c1".to_string(),
                "#b48ead".to_string(),
                "#88c0d0".to_string(),
                "#e5e9f0".to_string(),
                "#4c566a".to_string(),
                "#bf616a".to_string(),
                "#a3be8c".to_string(),
                "#ebcb8b".to_string(),
                "#81a1c1".to_string(),
                "#b48ead".to_string(),
                "#8fbcbb".to_string(),
                "#eceff4".to_string(),
            ],
        }
    }

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

    pub fn foreground_rgb(&self) -> (u8, u8, u8) {
        Self::parse_hex(&self.foreground).unwrap_or((212, 212, 212))
    }

    pub fn background_rgb(&self) -> (u8, u8, u8) {
        Self::parse_hex(&self.background).unwrap_or((30, 30, 30))
    }

    pub fn cursor_rgb(&self) -> (u8, u8, u8) {
        Self::parse_hex(&self.cursor).unwrap_or((255, 255, 255))
    }

    pub fn selection_rgb(&self) -> (u8, u8, u8) {
        Self::parse_hex(&self.selection).unwrap_or((38, 79, 120))
    }

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
        assert_eq!(ColorScheme::parse_hex("#fff"), None);
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
        assert_eq!(
            ThemeName::from_str("solarized-dark"),
            Some(ThemeName::SolarizedDark)
        );
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

        config.font_size = 2.0;
        assert!(config.validate().is_err());
        config.font_size = 14.0;

        config.dimensions = (5, 24);
        assert!(config.validate().is_err());
        config.dimensions = (80, 24);

        config.scrollback_lines = 2_000_000;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_cli_args_override() {
        let mut config = Config::default();
        let args = CliArgs {
            config: None,
            font_size: Some(20.0),
            theme: Some("light".to_string()),
            shell: Some("/bin/zsh".to_string()),
            scrollback: Some(5000),
            cols: Some(120),
            rows: Some(40),
        };

        config.apply_cli_args(&args);

        assert_eq!(config.font_size, 20.0);
        assert_eq!(config.theme, ThemeName::Light);
        assert_eq!(config.shell, Some("/bin/zsh".to_string()));
        assert_eq!(config.scrollback_lines, 5000);
        assert_eq!(config.dimensions, (120, 40));
    }

    #[test]
    fn test_config_toml_parsing() {
        let toml_str = r#"
            font_family = "JetBrains Mono"
            font_size = 16.0
            theme = "dracula"
            scrollback_lines = 20000
            dimensions = [100, 30]
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.font_family, "JetBrains Mono");
        assert_eq!(config.font_size, 16.0);
        assert_eq!(config.theme, ThemeName::Dracula);
        assert_eq!(config.scrollback_lines, 20000);
        assert_eq!(config.dimensions, (100, 30));
    }

    #[test]
    fn test_partial_config_toml() {
        let toml_str = r#"
            font_size = 18.0
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.font_size, 18.0);
        assert_eq!(config.font_family, "monospace");
        assert_eq!(config.theme, ThemeName::Dark);
    }

    #[test]
    fn test_color_scheme_validation() {
        let mut scheme = ColorScheme::default();
        assert!(scheme.validate().is_ok());

        scheme.foreground = "invalid".to_string();
        assert!(scheme.validate().is_err());
    }

    #[test]
    fn test_parsed_keybinding_parse() {
        let kb = ParsedKeybinding::parse("Ctrl+Shift+C").unwrap();
        assert!(kb.ctrl);
        assert!(kb.shift);
        assert!(!kb.alt);
        assert_eq!(kb.key, "C");

        let kb = ParsedKeybinding::parse("Alt+F4").unwrap();
        assert!(!kb.ctrl);
        assert!(!kb.shift);
        assert!(kb.alt);
        assert_eq!(kb.key, "F4");

        let kb = ParsedKeybinding::parse("Ctrl+Plus").unwrap();
        assert!(kb.ctrl);
        assert!(!kb.shift);
        assert!(!kb.alt);
        assert_eq!(kb.key, "PLUS");

        // Empty key should return None
        assert!(ParsedKeybinding::parse("Ctrl+Shift+").is_none());
    }

    #[test]
    fn test_parsed_keybinding_matches() {
        let kb = ParsedKeybinding::parse("Ctrl+Shift+C").unwrap();
        assert!(kb.matches(true, false, true, "C"));
        assert!(kb.matches(true, false, true, "c")); // Case insensitive
        assert!(!kb.matches(true, false, false, "C")); // Missing shift
        assert!(!kb.matches(false, false, true, "C")); // Missing ctrl
        assert!(!kb.matches(true, false, true, "V")); // Wrong key
    }

    #[test]
    fn test_keybindings_match_key() {
        let keybindings = Keybindings::default();

        // Test default keybindings
        assert_eq!(
            keybindings.match_key(true, false, true, "C"),
            Some(KeyAction::Copy)
        );
        assert_eq!(
            keybindings.match_key(true, false, true, "V"),
            Some(KeyAction::Paste)
        );
        assert_eq!(
            keybindings.match_key(true, false, true, "T"),
            Some(KeyAction::ToggleTheme)
        );
        assert_eq!(
            keybindings.match_key(true, false, true, "R"),
            Some(KeyAction::ReloadConfig)
        );
        assert_eq!(
            keybindings.match_key(true, false, true, "F"),
            Some(KeyAction::Find)
        );

        // Test non-matching
        assert_eq!(keybindings.match_key(false, false, false, "C"), None);
        assert_eq!(keybindings.match_key(true, false, false, "X"), None);
    }

    #[test]
    fn test_keybindings_toml_parsing() {
        let toml_str = r#"
            [keybindings]
            copy = "Ctrl+C"
            paste = "Ctrl+V"
            toggle_theme = "Alt+T"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.keybindings.copy, "Ctrl+C");
        assert_eq!(config.keybindings.paste, "Ctrl+V");
        assert_eq!(config.keybindings.toggle_theme, "Alt+T");
        // Defaults should still be used for unspecified keybindings
        assert_eq!(config.keybindings.reload_config, "Ctrl+Shift+R");
    }
}
