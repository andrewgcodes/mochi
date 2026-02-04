//! Configuration for Mochi Terminal
//!
//! Configuration is loaded with the following precedence (highest to lowest):
//! 1. CLI arguments (--config, --font-size, etc.)
//! 2. Environment variables (MOCHI_FONT_SIZE, MOCHI_THEME, etc.)
//! 3. Config file (~/.config/mochi/config.toml or --config path)
//! 4. Built-in defaults

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("Invalid configuration: {0}")]
    ValidationError(String),

    #[error("Invalid color format '{0}': expected #RRGGBB")]
    InvalidColor(String),

    #[error("Invalid theme name '{0}': expected one of dark, light, solarized-dark, solarized-light, dracula, nord, gruvbox, onedark, custom")]
    InvalidTheme(String),

    #[error("Config file not found: {0}")]
    NotFound(PathBuf),
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
    Gruvbox,
    #[serde(rename = "onedark")]
    OneDark,
    Custom,
}

impl ThemeName {
    pub fn from_str(s: &str) -> Result<Self, ConfigError> {
        match s.to_lowercase().as_str() {
            "dark" => Ok(ThemeName::Dark),
            "light" => Ok(ThemeName::Light),
            "solarized-dark" | "solarizeddark" => Ok(ThemeName::SolarizedDark),
            "solarized-light" | "solarizedlight" => Ok(ThemeName::SolarizedLight),
            "dracula" => Ok(ThemeName::Dracula),
            "nord" => Ok(ThemeName::Nord),
            "gruvbox" => Ok(ThemeName::Gruvbox),
            "onedark" | "one-dark" => Ok(ThemeName::OneDark),
            "custom" => Ok(ThemeName::Custom),
            _ => Err(ConfigError::InvalidTheme(s.to_string())),
        }
    }

    pub fn all_names() -> &'static [&'static str] {
        &["dark", "light", "solarized-dark", "solarized-light", "dracula", "nord", "gruvbox", "onedark", "custom"]
    }

    pub fn next(self) -> Self {
        match self {
            ThemeName::Dark => ThemeName::Light,
            ThemeName::Light => ThemeName::SolarizedDark,
            ThemeName::SolarizedDark => ThemeName::SolarizedLight,
            ThemeName::SolarizedLight => ThemeName::Dracula,
            ThemeName::Dracula => ThemeName::Nord,
            ThemeName::Nord => ThemeName::Gruvbox,
            ThemeName::Gruvbox => ThemeName::OneDark,
            ThemeName::OneDark => ThemeName::Dark,
            ThemeName::Custom => ThemeName::Dark,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            ThemeName::Dark => "Dark",
            ThemeName::Light => "Light",
            ThemeName::SolarizedDark => "Solarized Dark",
            ThemeName::SolarizedLight => "Solarized Light",
            ThemeName::Dracula => "Dracula",
            ThemeName::Nord => "Nord",
            ThemeName::Gruvbox => "Gruvbox",
            ThemeName::OneDark => "One Dark",
            ThemeName::Custom => "Custom",
        }
    }

    /// Convert theme name to its corresponding color scheme
    pub fn to_color_scheme(self) -> ColorScheme {
        match self {
            ThemeName::Dark => ColorScheme::default(),
            ThemeName::Light => ColorScheme::light(),
            ThemeName::SolarizedDark => ColorScheme::solarized_dark(),
            ThemeName::SolarizedLight => ColorScheme::solarized_light(),
            ThemeName::Dracula => ColorScheme::dracula(),
            ThemeName::Nord => ColorScheme::nord(),
            ThemeName::Gruvbox => ColorScheme::gruvbox(),
            ThemeName::OneDark => ColorScheme::onedark(),
            ThemeName::Custom => ColorScheme::default(), // Custom uses default as fallback
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyAction {
    Copy,
    Paste,
    Find,
    ReloadConfig,
    ToggleTheme,
    IncreaseFontSize,
    DecreaseFontSize,
    ResetFontSize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    #[serde(default = "default_copy_key")]
    pub copy: String,
    #[serde(default = "default_paste_key")]
    pub paste: String,
    #[serde(default = "default_find_key")]
    pub find: String,
    #[serde(default = "default_reload_key")]
    pub reload_config: String,
    #[serde(default = "default_toggle_theme_key")]
    pub toggle_theme: String,
    #[serde(default = "default_increase_font_key")]
    pub increase_font_size: String,
    #[serde(default = "default_decrease_font_key")]
    pub decrease_font_size: String,
    #[serde(default = "default_reset_font_key")]
    pub reset_font_size: String,
}

fn default_copy_key() -> String { "ctrl+shift+c".to_string() }
fn default_paste_key() -> String { "ctrl+shift+v".to_string() }
fn default_find_key() -> String { "ctrl+shift+f".to_string() }
fn default_reload_key() -> String { "ctrl+shift+r".to_string() }
fn default_toggle_theme_key() -> String { "ctrl+shift+t".to_string() }
fn default_increase_font_key() -> String { "ctrl+plus".to_string() }
fn default_decrease_font_key() -> String { "ctrl+minus".to_string() }
fn default_reset_font_key() -> String { "ctrl+0".to_string() }

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            copy: default_copy_key(),
            paste: default_paste_key(),
            find: default_find_key(),
            reload_config: default_reload_key(),
            toggle_theme: default_toggle_theme_key(),
            increase_font_size: default_increase_font_key(),
            decrease_font_size: default_decrease_font_key(),
            reset_font_size: default_reset_font_key(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontConfig {
    #[serde(default = "default_font_family")]
    pub family: String,
    #[serde(default = "default_font_size")]
    pub size: f32,
    #[serde(default)]
    pub fallback: Vec<String>,
    #[serde(default = "default_line_height")]
    pub line_height: f32,
    #[serde(default)]
    pub cell_padding: (f32, f32),
}

fn default_font_family() -> String { "monospace".to_string() }
fn default_font_size() -> f32 { 14.0 }
fn default_line_height() -> f32 { 1.2 }

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: default_font_family(),
            size: default_font_size(),
            fallback: vec![],
            line_height: default_line_height(),
            cell_padding: (0.0, 0.0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default)]
    pub osc52_clipboard: bool,
    #[serde(default = "default_osc52_max_size")]
    pub osc52_max_size: usize,
    #[serde(default = "default_true")]
    pub clipboard_notification: bool,
    #[serde(default = "default_max_title_length")]
    pub max_title_length: usize,
    #[serde(default = "default_title_throttle_ms")]
    pub title_throttle_ms: u64,
}

fn default_osc52_max_size() -> usize { 100_000 }
fn default_true() -> bool { true }
fn default_max_title_length() -> usize { 4096 }
fn default_title_throttle_ms() -> u64 { 100 }

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            osc52_clipboard: false,
            osc52_max_size: default_osc52_max_size(),
            clipboard_notification: true,
            max_title_length: default_max_title_length(),
            title_throttle_ms: default_title_throttle_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub font: FontConfig,
    #[serde(default = "default_scrollback_lines")]
    pub scrollback_lines: usize,
    #[serde(default = "default_dimensions")]
    pub dimensions: (u16, u16),
    #[serde(default)]
    pub theme: ThemeName,
    #[serde(default)]
    pub colors: ColorScheme,
    #[serde(default)]
    pub shell: Option<String>,
    #[serde(default = "default_cursor_style")]
    pub cursor_style: String,
    #[serde(default = "default_true")]
    pub cursor_blink: bool,
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub auto_reload: bool,
}

fn default_scrollback_lines() -> usize { 10000 }
fn default_dimensions() -> (u16, u16) { (80, 24) }
fn default_cursor_style() -> String { "block".to_string() }

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
            keybindings: KeybindingsConfig::default(),
            security: SecurityConfig::default(),
            auto_reload: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    pub foreground: String,
    pub background: String,
    pub cursor: String,
    pub selection: String,
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
                "#000000".to_string(), "#cd3131".to_string(), "#0dbc79".to_string(), "#e5e510".to_string(),
                "#2472c8".to_string(), "#bc3fbc".to_string(), "#11a8cd".to_string(), "#e5e5e5".to_string(),
                "#666666".to_string(), "#f14c4c".to_string(), "#23d18b".to_string(), "#f5f543".to_string(),
                "#3b8eea".to_string(), "#d670d6".to_string(), "#29b8db".to_string(), "#ffffff".to_string(),
            ],
        }
    }
}

impl Config {
    pub fn effective_colors(&self) -> ColorScheme {
        match self.theme {
            ThemeName::Custom => self.colors.clone(),
            ThemeName::Dark => ColorScheme::dark(),
            ThemeName::Light => ColorScheme::light(),
            ThemeName::SolarizedDark => ColorScheme::solarized_dark(),
            ThemeName::SolarizedLight => ColorScheme::solarized_light(),
            ThemeName::Dracula => ColorScheme::dracula(),
            ThemeName::Nord => ColorScheme::nord(),
            ThemeName::Gruvbox => ColorScheme::gruvbox(),
            ThemeName::OneDark => ColorScheme::onedark(),
        }
    }

    pub fn load_with_overrides(config_path: Option<PathBuf>, cli_overrides: CliOverrides) -> Result<Self, ConfigError> {
        let mut config = Config::default();
        let file_path = config_path.clone().or_else(Self::default_config_path);
        if let Some(path) = file_path {
            if path.exists() {
                let content = fs::read_to_string(&path)?;
                config = toml::from_str(&content)?;
                log::info!("Loaded config from: {}", path.display());
            } else if config_path.is_some() {
                return Err(ConfigError::NotFound(path));
            }
        }
        config.apply_env_overrides();
        config.apply_cli_overrides(cli_overrides);
        config.validate()?;
        Ok(config)
    }

    pub fn load() -> Option<Self> {
        Self::load_with_overrides(None, CliOverrides::default()).ok()
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(val) = env::var("MOCHI_THEME") {
            if let Ok(theme) = ThemeName::from_str(&val) { self.theme = theme; }
        }
        if let Ok(val) = env::var("MOCHI_FONT_SIZE") {
            if let Ok(size) = val.parse::<f32>() { self.font.size = size; }
        }
        if let Ok(val) = env::var("MOCHI_FONT_FAMILY") { self.font.family = val; }
        if let Ok(val) = env::var("MOCHI_SCROLLBACK") {
            if let Ok(lines) = val.parse::<usize>() { self.scrollback_lines = lines; }
        }
        if let Ok(val) = env::var("MOCHI_SHELL") { self.shell = Some(val); }
        if let Ok(val) = env::var("MOCHI_OSC52_CLIPBOARD") {
            self.security.osc52_clipboard = val == "1" || val.to_lowercase() == "true";
        }
    }

    fn apply_cli_overrides(&mut self, overrides: CliOverrides) {
        if let Some(theme) = overrides.theme { self.theme = theme; }
        if let Some(size) = overrides.font_size { self.font.size = size; }
        if let Some(family) = overrides.font_family { self.font.family = family; }
        if let Some(shell) = overrides.shell { self.shell = Some(shell); }
        if let Some(scrollback) = overrides.scrollback { self.scrollback_lines = scrollback; }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.font.size < 4.0 || self.font.size > 144.0 {
            return Err(ConfigError::ValidationError(format!("Font size {} is out of range (4.0-144.0)", self.font.size)));
        }
        if self.font.line_height < 0.5 || self.font.line_height > 3.0 {
            return Err(ConfigError::ValidationError(format!("Line height {} is out of range (0.5-3.0)", self.font.line_height)));
        }
        if self.scrollback_lines > 1_000_000 {
            return Err(ConfigError::ValidationError(format!("Scrollback lines {} exceeds maximum (1,000,000)", self.scrollback_lines)));
        }
        if self.dimensions.0 < 10 || self.dimensions.0 > 1000 {
            return Err(ConfigError::ValidationError(format!("Column count {} is out of range (10-1000)", self.dimensions.0)));
        }
        if self.dimensions.1 < 5 || self.dimensions.1 > 500 {
            return Err(ConfigError::ValidationError(format!("Row count {} is out of range (5-500)", self.dimensions.1)));
        }
        self.colors.validate()?;
        let valid_styles = ["block", "underline", "bar"];
        if !valid_styles.contains(&self.cursor_style.as_str()) {
            return Err(ConfigError::ValidationError(format!("Invalid cursor style '{}': expected one of {:?}", self.cursor_style, valid_styles)));
        }
        Ok(())
    }

    pub fn reload(&mut self, config_path: Option<PathBuf>) -> Result<(), ConfigError> {
        let new_config = Self::load_with_overrides(config_path, CliOverrides::default())?;
        *self = new_config;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::default_config_path().ok_or("Could not determine config path")?;
        if let Some(parent) = config_path.parent() { fs::create_dir_all(parent)?; }
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }

    pub fn default_config_path() -> Option<PathBuf> {
        if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(xdg_config).join("mochi").join("config.toml"));
        }
        dirs::config_dir().map(|p| p.join("mochi").join("config.toml"))
    }

    pub fn font_family(&self) -> &str { &self.font.family }
    pub fn font_size(&self) -> f32 { self.font.size }
    pub fn osc52_clipboard(&self) -> bool { self.security.osc52_clipboard }
    pub fn osc52_max_size(&self) -> usize { self.security.osc52_max_size }
}

#[derive(Debug, Default)]
pub struct CliOverrides {
    pub theme: Option<ThemeName>,
    pub font_size: Option<f32>,
    pub font_family: Option<String>,
    pub shell: Option<String>,
    pub scrollback: Option<usize>,
}

impl ColorScheme {
    pub fn validate(&self) -> Result<(), ConfigError> {
        Self::validate_color(&self.foreground)?;
        Self::validate_color(&self.background)?;
        Self::validate_color(&self.cursor)?;
        Self::validate_color(&self.selection)?;
        for (i, color) in self.ansi.iter().enumerate() {
            Self::validate_color(color).map_err(|_| ConfigError::InvalidColor(format!("ansi[{}]: {}", i, color)))?;
        }
        Ok(())
    }

    fn validate_color(color: &str) -> Result<(), ConfigError> {
        if Self::parse_hex(color).is_none() { return Err(ConfigError::InvalidColor(color.to_string())); }
        Ok(())
    }

    pub fn dark() -> Self { Self::default() }

    pub fn light() -> Self {
        Self {
            foreground: "#333333".to_string(),
            background: "#ffffff".to_string(),
            cursor: "#000000".to_string(),
            selection: "#add6ff".to_string(),
            ansi: [
                "#000000".to_string(), "#cd3131".to_string(), "#00bc00".to_string(), "#949800".to_string(),
                "#0451a5".to_string(), "#bc05bc".to_string(), "#0598bc".to_string(), "#555555".to_string(),
                "#666666".to_string(), "#cd3131".to_string(), "#14ce14".to_string(), "#b5ba00".to_string(),
                "#0451a5".to_string(), "#bc05bc".to_string(), "#0598bc".to_string(), "#a5a5a5".to_string(),
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
                "#073642".to_string(), "#dc322f".to_string(), "#859900".to_string(), "#b58900".to_string(),
                "#268bd2".to_string(), "#d33682".to_string(), "#2aa198".to_string(), "#eee8d5".to_string(),
                "#002b36".to_string(), "#cb4b16".to_string(), "#586e75".to_string(), "#657b83".to_string(),
                "#839496".to_string(), "#6c71c4".to_string(), "#93a1a1".to_string(), "#fdf6e3".to_string(),
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
                "#073642".to_string(), "#dc322f".to_string(), "#859900".to_string(), "#b58900".to_string(),
                "#268bd2".to_string(), "#d33682".to_string(), "#2aa198".to_string(), "#eee8d5".to_string(),
                "#002b36".to_string(), "#cb4b16".to_string(), "#586e75".to_string(), "#657b83".to_string(),
                "#839496".to_string(), "#6c71c4".to_string(), "#93a1a1".to_string(), "#fdf6e3".to_string(),
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
                "#21222c".to_string(), "#ff5555".to_string(), "#50fa7b".to_string(), "#f1fa8c".to_string(),
                "#bd93f9".to_string(), "#ff79c6".to_string(), "#8be9fd".to_string(), "#f8f8f2".to_string(),
                "#6272a4".to_string(), "#ff6e6e".to_string(), "#69ff94".to_string(), "#ffffa5".to_string(),
                "#d6acff".to_string(), "#ff92df".to_string(), "#a4ffff".to_string(), "#ffffff".to_string(),
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
                "#3b4252".to_string(), "#bf616a".to_string(), "#a3be8c".to_string(), "#ebcb8b".to_string(),
                "#81a1c1".to_string(), "#b48ead".to_string(), "#88c0d0".to_string(), "#e5e9f0".to_string(),
                "#4c566a".to_string(), "#bf616a".to_string(), "#a3be8c".to_string(), "#ebcb8b".to_string(),
                "#81a1c1".to_string(), "#b48ead".to_string(), "#8fbcbb".to_string(), "#eceff4".to_string(),
            ],
        }
    }

    pub fn gruvbox() -> Self {
        Self {
            foreground: "#ebdbb2".to_string(),
            background: "#282828".to_string(),
            cursor: "#ebdbb2".to_string(),
            selection: "#504945".to_string(),
            ansi: [
                "#282828".to_string(), "#cc241d".to_string(), "#98971a".to_string(), "#d79921".to_string(),
                "#458588".to_string(), "#b16286".to_string(), "#689d6a".to_string(), "#a89984".to_string(),
                "#928374".to_string(), "#fb4934".to_string(), "#b8bb26".to_string(), "#fabd2f".to_string(),
                "#83a598".to_string(), "#d3869b".to_string(), "#8ec07c".to_string(), "#ebdbb2".to_string(),
            ],
        }
    }

    pub fn onedark() -> Self {
        Self {
            foreground: "#abb2bf".to_string(),
            background: "#282c34".to_string(),
            cursor: "#528bff".to_string(),
            selection: "#3e4451".to_string(),
            ansi: [
                "#282c34".to_string(), "#e06c75".to_string(), "#98c379".to_string(), "#e5c07b".to_string(),
                "#61afef".to_string(), "#c678dd".to_string(), "#56b6c2".to_string(), "#abb2bf".to_string(),
                "#5c6370".to_string(), "#e06c75".to_string(), "#98c379".to_string(), "#e5c07b".to_string(),
                "#61afef".to_string(), "#c678dd".to_string(), "#56b6c2".to_string(), "#ffffff".to_string(),
            ],
        }
    }

    pub fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 { return None; }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some((r, g, b))
    }

    pub fn foreground_rgb(&self) -> (u8, u8, u8) { Self::parse_hex(&self.foreground).unwrap_or((212, 212, 212)) }
    pub fn background_rgb(&self) -> (u8, u8, u8) { Self::parse_hex(&self.background).unwrap_or((30, 30, 30)) }
    pub fn cursor_rgb(&self) -> (u8, u8, u8) { Self::parse_hex(&self.cursor).unwrap_or((255, 255, 255)) }
    pub fn selection_rgb(&self) -> (u8, u8, u8) { Self::parse_hex(&self.selection).unwrap_or((38, 79, 120)) }
    pub fn ansi_rgb(&self, index: usize) -> (u8, u8, u8) {
        if index < 16 { Self::parse_hex(&self.ansi[index]).unwrap_or((128, 128, 128)) } else { (128, 128, 128) }
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
        assert_eq!(ThemeName::from_str("dark").unwrap(), ThemeName::Dark);
        assert_eq!(ThemeName::from_str("LIGHT").unwrap(), ThemeName::Light);
        assert_eq!(ThemeName::from_str("solarized-dark").unwrap(), ThemeName::SolarizedDark);
        assert!(ThemeName::from_str("invalid").is_err());
    }

    #[test]
    fn test_theme_next() {
        assert_eq!(ThemeName::Dark.next(), ThemeName::Light);
        assert_eq!(ThemeName::OneDark.next(), ThemeName::Dark);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());
        config.font.size = 2.0;
        assert!(config.validate().is_err());
        config.font.size = 14.0;
        config.cursor_style = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_keybindings_default() {
        let kb = KeybindingsConfig::default();
        assert_eq!(kb.copy, "ctrl+shift+c");
        assert_eq!(kb.paste, "ctrl+shift+v");
    }

    #[test]
    fn test_config_parse_toml() {
        let toml_str = r#"
            scrollback_lines = 5000
            theme = "nord"
            cursor_style = "bar"
            [font]
            family = "JetBrains Mono"
            size = 12.0
            [keybindings]
            copy = "ctrl+c"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.scrollback_lines, 5000);
        assert_eq!(config.theme, ThemeName::Nord);
        assert_eq!(config.font.family, "JetBrains Mono");
        assert_eq!(config.font.size, 12.0);
        assert_eq!(config.keybindings.copy, "ctrl+c");
    }

    #[test]
    fn test_all_themes_valid() {
        let themes = [
            ColorScheme::dark(), ColorScheme::light(), ColorScheme::solarized_dark(),
            ColorScheme::solarized_light(), ColorScheme::dracula(), ColorScheme::nord(),
            ColorScheme::gruvbox(), ColorScheme::onedark(),
        ];
        for theme in &themes { assert!(theme.validate().is_ok()); }
    }
}
