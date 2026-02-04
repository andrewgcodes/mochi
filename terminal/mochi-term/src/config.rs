//! Configuration for Mochi Terminal
//!
//! Configuration is loaded with the following precedence (highest to lowest):
//! 1. CLI flags
//! 2. Environment variables (MOCHI_*)
//! 3. Config file (~/.config/mochi/config.toml or --config path)
//! 4. Built-in defaults

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(String),
    #[error("Failed to parse config file: {0}")]
    ParseError(String),
    #[error("Invalid configuration: {0}")]
    ValidationError(String),
    #[error("Invalid color format '{0}': expected hex color like #rrggbb")]
    InvalidColor(String),
    #[error("Invalid theme name '{0}': expected one of dark, light, solarized-dark, solarized-light, dracula, nord, gruvbox, custom")]
    InvalidTheme(String),
}

#[derive(Parser, Debug)]
#[command(name = "mochi")]
#[command(author = "Mochi Terminal Authors")]
#[command(version)]
#[command(about = "A VT/xterm-compatible terminal emulator", long_about = None)]
pub struct CliArgs {
    #[arg(short, long, value_name = "FILE", help = "Path to config file")]
    pub config: Option<PathBuf>,

    #[arg(long, value_name = "FAMILY", help = "Font family name")]
    pub font_family: Option<String>,

    #[arg(long, value_name = "SIZE", help = "Font size in points")]
    pub font_size: Option<f32>,

    #[arg(long, value_name = "THEME", help = "Theme name (dark, light, solarized-dark, solarized-light, dracula, nord, gruvbox)")]
    pub theme: Option<String>,

    #[arg(long, value_name = "COLS", help = "Initial columns")]
    pub cols: Option<u16>,

    #[arg(long, value_name = "ROWS", help = "Initial rows")]
    pub rows: Option<u16>,

    #[arg(long, value_name = "SHELL", help = "Shell command to run")]
    pub shell: Option<String>,

    #[arg(long, help = "Enable OSC 52 clipboard support (security risk)")]
    pub enable_osc52: bool,
}

impl CliArgs {
    pub fn parse_args() -> Self {
        Self::parse()
    }
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
            "custom" => Ok(ThemeName::Custom),
            _ => Err(ConfigError::InvalidTheme(s.to_string())),
        }
    }

    #[allow(dead_code)]
    pub fn all_names() -> &'static [&'static str] {
        &["dark", "light", "solarized-dark", "solarized-light", "dracula", "nord", "gruvbox", "custom"]
    }
}

/// Terminal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub font_family: String,
    pub font_size: f32,
    pub scrollback_lines: usize,
    pub dimensions: (u16, u16),
    pub theme: ThemeName,
    pub colors: ColorScheme,
    pub osc52_clipboard: bool,
    pub osc52_max_size: usize,
    pub shell: Option<String>,
    pub cursor_style: String,
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
        }
    }

    pub fn load_with_args(args: &CliArgs) -> Result<Self, ConfigError> {
        let config_path = args.config.clone().or_else(Self::default_config_path);
        let mut config = if let Some(path) = config_path {
            if path.exists() {
                Self::load_from_path(&path)?
            } else if args.config.is_some() {
                return Err(ConfigError::ReadError(format!(
                    "Config file not found: {}",
                    path.display()
                )));
            } else {
                Config::default()
            }
        } else {
            Config::default()
        };

        config.apply_env_vars();
        config.apply_cli_args(args);
        config.validate()?;

        Ok(config)
    }

    pub fn load_from_path(path: &PathBuf) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)
            .map_err(|e| ConfigError::ReadError(format!("{}: {}", path.display(), e)))?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(format!("{}", e)))?;

        Ok(config)
    }

    pub fn reload(&mut self) -> Result<(), ConfigError> {
        let config_path = Self::default_config_path()
            .ok_or_else(|| ConfigError::ReadError("Could not determine config path".to_string()))?;

        if !config_path.exists() {
            return Err(ConfigError::ReadError(format!(
                "Config file not found: {}",
                config_path.display()
            )));
        }

        let new_config = Self::load_from_path(&config_path)?;
        new_config.validate()?;

        *self = new_config;
        Ok(())
    }

    fn apply_env_vars(&mut self) {
        if let Ok(val) = env::var("MOCHI_FONT_FAMILY") {
            self.font_family = val;
        }
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
        if let Ok(val) = env::var("MOCHI_OSC52_CLIPBOARD") {
            self.osc52_clipboard = val == "1" || val.to_lowercase() == "true";
        }
    }

    fn apply_cli_args(&mut self, args: &CliArgs) {
        if let Some(ref family) = args.font_family {
            self.font_family = family.clone();
        }
        if let Some(size) = args.font_size {
            self.font_size = size;
        }
        if let Some(ref theme_str) = args.theme {
            if let Ok(theme) = ThemeName::from_str(theme_str) {
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
        if args.enable_osc52 {
            self.osc52_clipboard = true;
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.font_size < 4.0 || self.font_size > 144.0 {
            return Err(ConfigError::ValidationError(format!(
                "font_size must be between 4 and 144, got {}",
                self.font_size
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

        if self.theme == ThemeName::Custom {
            self.colors.validate()?;
        }

        Ok(())
    }

    pub fn default_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("mochi").join("config.toml"))
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

    fn validate_color(color: &str, field_name: &str) -> Result<(), ConfigError> {
        if Self::parse_hex(color).is_none() {
            return Err(ConfigError::InvalidColor(format!(
                "{}: '{}'",
                field_name, color
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

    pub fn gruvbox() -> Self {
        Self {
            foreground: "#ebdbb2".to_string(),
            background: "#282828".to_string(),
            cursor: "#ebdbb2".to_string(),
            selection: "#504945".to_string(),
            ansi: [
                "#282828".to_string(),
                "#cc241d".to_string(),
                "#98971a".to_string(),
                "#d79921".to_string(),
                "#458588".to_string(),
                "#b16286".to_string(),
                "#689d6a".to_string(),
                "#a89984".to_string(),
                "#928374".to_string(),
                "#fb4934".to_string(),
                "#b8bb26".to_string(),
                "#fabd2f".to_string(),
                "#83a598".to_string(),
                "#d3869b".to_string(),
                "#8ec07c".to_string(),
                "#ebdbb2".to_string(),
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
        assert_eq!(config.theme, ThemeName::Dark);
        assert_eq!(config.font_family, "monospace");
    }

    #[test]
    fn test_parse_hex() {
        assert_eq!(ColorScheme::parse_hex("#ff0000"), Some((255, 0, 0)));
        assert_eq!(ColorScheme::parse_hex("00ff00"), Some((0, 255, 0)));
        assert_eq!(ColorScheme::parse_hex("#invalid"), None);
        assert_eq!(ColorScheme::parse_hex("#fff"), None);
        assert_eq!(ColorScheme::parse_hex(""), None);
    }

    #[test]
    fn test_color_scheme_default() {
        let scheme = ColorScheme::default();
        assert_eq!(scheme.ansi.len(), 16);
    }

    #[test]
    fn test_theme_name_from_str() {
        assert_eq!(ThemeName::from_str("dark").unwrap(), ThemeName::Dark);
        assert_eq!(ThemeName::from_str("LIGHT").unwrap(), ThemeName::Light);
        assert_eq!(ThemeName::from_str("Dark").unwrap(), ThemeName::Dark);
        assert_eq!(ThemeName::from_str("solarized-dark").unwrap(), ThemeName::SolarizedDark);
        assert_eq!(ThemeName::from_str("solarizeddark").unwrap(), ThemeName::SolarizedDark);
        assert_eq!(ThemeName::from_str("solarized-light").unwrap(), ThemeName::SolarizedLight);
        assert_eq!(ThemeName::from_str("dracula").unwrap(), ThemeName::Dracula);
        assert_eq!(ThemeName::from_str("nord").unwrap(), ThemeName::Nord);
        assert_eq!(ThemeName::from_str("gruvbox").unwrap(), ThemeName::Gruvbox);
        assert_eq!(ThemeName::from_str("custom").unwrap(), ThemeName::Custom);
        assert!(ThemeName::from_str("invalid").is_err());
        assert!(ThemeName::from_str("").is_err());
    }

    #[test]
    fn test_config_validation_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_font_size_too_small() {
        let mut config = Config::default();
        config.font_size = 2.0;
        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::ValidationError(_)));
    }

    #[test]
    fn test_config_validation_font_size_too_large() {
        let mut config = Config::default();
        config.font_size = 200.0;
        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::ValidationError(_)));
    }

    #[test]
    fn test_config_validation_columns_too_small() {
        let mut config = Config::default();
        config.dimensions = (5, 24);
        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::ValidationError(_)));
    }

    #[test]
    fn test_config_validation_rows_too_small() {
        let mut config = Config::default();
        config.dimensions = (80, 2);
        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::ValidationError(_)));
    }

    #[test]
    fn test_config_validation_scrollback_too_large() {
        let mut config = Config::default();
        config.scrollback_lines = 2_000_000;
        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::ValidationError(_)));
    }

    #[test]
    fn test_color_scheme_validation_valid() {
        let scheme = ColorScheme::default();
        assert!(scheme.validate().is_ok());
    }

    #[test]
    fn test_color_scheme_validation_invalid_foreground() {
        let mut scheme = ColorScheme::default();
        scheme.foreground = "not-a-color".to_string();
        let err = scheme.validate().unwrap_err();
        assert!(matches!(err, ConfigError::InvalidColor(_)));
    }

    #[test]
    fn test_color_scheme_validation_invalid_ansi() {
        let mut scheme = ColorScheme::default();
        scheme.ansi[5] = "invalid".to_string();
        let err = scheme.validate().unwrap_err();
        assert!(matches!(err, ConfigError::InvalidColor(_)));
    }

    #[test]
    fn test_all_builtin_themes_valid() {
        assert!(ColorScheme::dark().validate().is_ok());
        assert!(ColorScheme::light().validate().is_ok());
        assert!(ColorScheme::solarized_dark().validate().is_ok());
        assert!(ColorScheme::solarized_light().validate().is_ok());
        assert!(ColorScheme::dracula().validate().is_ok());
        assert!(ColorScheme::nord().validate().is_ok());
        assert!(ColorScheme::gruvbox().validate().is_ok());
    }

    #[test]
    fn test_effective_colors_dark() {
        let mut config = Config::default();
        config.theme = ThemeName::Dark;
        let colors = config.effective_colors();
        assert_eq!(colors.background, "#1e1e1e");
    }

    #[test]
    fn test_effective_colors_light() {
        let mut config = Config::default();
        config.theme = ThemeName::Light;
        let colors = config.effective_colors();
        assert_eq!(colors.background, "#ffffff");
    }

    #[test]
    fn test_effective_colors_gruvbox() {
        let mut config = Config::default();
        config.theme = ThemeName::Gruvbox;
        let colors = config.effective_colors();
        assert_eq!(colors.background, "#282828");
        assert_eq!(colors.foreground, "#ebdbb2");
    }

    #[test]
    fn test_effective_colors_custom() {
        let mut config = Config::default();
        config.theme = ThemeName::Custom;
        config.colors.background = "#123456".to_string();
        let colors = config.effective_colors();
        assert_eq!(colors.background, "#123456");
    }

    #[test]
    fn test_parse_toml_config_full() {
        let toml_str = r#"
font_family = "JetBrains Mono"
font_size = 16.0
theme = "nord"
scrollback_lines = 5000
dimensions = [100, 30]
osc52_clipboard = true
osc52_max_size = 50000
cursor_style = "underline"
cursor_blink = false
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.font_family, "JetBrains Mono");
        assert_eq!(config.font_size, 16.0);
        assert_eq!(config.theme, ThemeName::Nord);
        assert_eq!(config.scrollback_lines, 5000);
        assert_eq!(config.dimensions, (100, 30));
        assert!(config.osc52_clipboard);
        assert_eq!(config.osc52_max_size, 50000);
        assert_eq!(config.cursor_style, "underline");
        assert!(!config.cursor_blink);
    }

    #[test]
    fn test_parse_toml_config_partial() {
        let toml_str = r#"
font_size = 18.0
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.font_size, 18.0);
        assert_eq!(config.font_family, "monospace");
        assert_eq!(config.theme, ThemeName::Dark);
        assert_eq!(config.dimensions, (80, 24));
    }

    #[test]
    fn test_parse_toml_config_with_custom_colors() {
        let toml_str = r##"
theme = "custom"

[colors]
foreground = "#aabbcc"
background = "#112233"
cursor = "#ffffff"
selection = "#445566"
ansi = [
    "#000000", "#ff0000", "#00ff00", "#ffff00",
    "#0000ff", "#ff00ff", "#00ffff", "#ffffff",
    "#808080", "#ff8080", "#80ff80", "#ffff80",
    "#8080ff", "#ff80ff", "#80ffff", "#ffffff"
]
"##;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.theme, ThemeName::Custom);
        assert_eq!(config.colors.foreground, "#aabbcc");
        assert_eq!(config.colors.background, "#112233");
    }

    #[test]
    fn test_parse_invalid_toml() {
        let toml_str = "this is not valid toml {{{";
        let result: Result<Config, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_color_rgb_methods() {
        let scheme = ColorScheme::default();
        let (r, g, b) = scheme.foreground_rgb();
        assert_eq!((r, g, b), (212, 212, 212));

        let (r, g, b) = scheme.background_rgb();
        assert_eq!((r, g, b), (30, 30, 30));

        let (r, g, b) = scheme.cursor_rgb();
        assert_eq!((r, g, b), (255, 255, 255));

        let (r, g, b) = scheme.ansi_rgb(0);
        assert_eq!((r, g, b), (0, 0, 0));

        let (r, g, b) = scheme.ansi_rgb(16);
        assert_eq!((r, g, b), (128, 128, 128));
    }

    #[test]
    fn test_default_config_path() {
        let path = Config::default_config_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("mochi"));
        assert!(path.to_string_lossy().contains("config.toml"));
    }
}
