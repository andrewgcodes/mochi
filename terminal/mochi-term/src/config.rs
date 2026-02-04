//! Configuration for Mochi Terminal
//!
//! Configuration is loaded with the following precedence (highest to lowest):
//! 1. CLI flags (--config, --font-size, --theme, etc.)
//! 2. Environment variables (MOCHI_CONFIG, MOCHI_FONT_SIZE, MOCHI_THEME, etc.)
//! 3. Config file (~/.config/mochi/config.toml or XDG_CONFIG_HOME/mochi/config.toml)
//! 4. Built-in defaults

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

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

/// Configuration error types
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("Invalid configuration: {0}")]
    ValidationError(String),

    #[error("Invalid color format '{color}': expected hex color like #rrggbb")]
    InvalidColor { color: String },

    #[error("Font size {size} is out of range: must be between {min} and {max}")]
    FontSizeOutOfRange { size: f32, min: f32, max: f32 },
}

/// CLI arguments for the terminal
#[derive(Debug, Default)]
pub struct CliArgs {
    /// Custom config file path
    pub config_path: Option<PathBuf>,
    /// Override font size
    pub font_size: Option<f32>,
    /// Override theme
    pub theme: Option<ThemeName>,
    /// Override shell command
    pub shell: Option<String>,
    /// Show help
    pub help: bool,
    /// Show version
    pub version: bool,
}

impl CliArgs {
    /// Parse command line arguments
    pub fn parse() -> Self {
        let args: Vec<String> = env::args().collect();
        let mut cli = CliArgs::default();
        let mut i = 1;

        while i < args.len() {
            match args[i].as_str() {
                "-c" | "--config" => {
                    if i + 1 < args.len() {
                        cli.config_path = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    }
                }
                "--font-size" => {
                    if i + 1 < args.len() {
                        if let Ok(size) = args[i + 1].parse() {
                            cli.font_size = Some(size);
                        }
                        i += 1;
                    }
                }
                "-t" | "--theme" => {
                    if i + 1 < args.len() {
                        cli.theme = Self::parse_theme_name(&args[i + 1]);
                        i += 1;
                    }
                }
                "-e" | "--shell" => {
                    if i + 1 < args.len() {
                        cli.shell = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "-h" | "--help" => {
                    cli.help = true;
                }
                "-v" | "--version" => {
                    cli.version = true;
                }
                _ => {}
            }
            i += 1;
        }

        cli
    }

    fn parse_theme_name(s: &str) -> Option<ThemeName> {
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

    /// Print help message
    pub fn print_help() {
        println!(
            r#"Mochi Terminal - A modern VT/xterm-compatible terminal emulator

USAGE:
    mochi [OPTIONS]

OPTIONS:
    -c, --config <PATH>     Use custom config file path
    --font-size <SIZE>      Override font size (e.g., 14.0)
    -t, --theme <THEME>     Override theme (dark, light, solarized-dark,
                            solarized-light, dracula, nord)
    -e, --shell <COMMAND>   Override shell command
    -h, --help              Print help information
    -v, --version           Print version information

ENVIRONMENT VARIABLES:
    MOCHI_CONFIG            Path to config file
    MOCHI_FONT_SIZE         Font size override
    MOCHI_THEME             Theme override

CONFIG FILE:
    Default location: ~/.config/mochi/config.toml
    Or: $XDG_CONFIG_HOME/mochi/config.toml

For more information, see: https://github.com/andrewgcodes/mochi"#
        );
    }

    /// Print version
    pub fn print_version() {
        println!("mochi {}", env!("CARGO_PKG_VERSION"));
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
    pub fn load_with_args(cli: &CliArgs) -> Result<Self, ConfigError> {
        let mut config = Self::default();

        // Determine config file path
        let config_path = cli
            .config_path
            .clone()
            .or_else(|| env::var("MOCHI_CONFIG").ok().map(PathBuf::from))
            .or_else(Self::default_config_path);

        // Load from config file if it exists
        if let Some(path) = config_path {
            if path.exists() {
                let content = fs::read_to_string(&path)?;
                config = toml::from_str(&content)?;
                log::info!("Loaded config from: {}", path.display());
            } else if cli.config_path.is_some() {
                return Err(ConfigError::ReadError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Config file not found: {}", path.display()),
                )));
            }
        }

        // Apply environment variable overrides
        Self::apply_env_overrides(&mut config);

        // Apply CLI overrides (highest precedence)
        Self::apply_cli_overrides(&mut config, cli);

        // Validate the final configuration
        config.validate()?;

        Ok(config)
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(config: &mut Config) {
        if let Ok(size) = env::var("MOCHI_FONT_SIZE") {
            if let Ok(size) = size.parse() {
                config.font_size = size;
            }
        }

        if let Ok(theme) = env::var("MOCHI_THEME") {
            if let Some(theme) = CliArgs::parse_theme_name(&theme) {
                config.theme = theme;
            }
        }
    }

    /// Apply CLI argument overrides
    pub fn apply_cli_overrides(config: &mut Config, cli: &CliArgs) {
        if let Some(size) = cli.font_size {
            config.font_size = size;
        }
        if let Some(theme) = cli.theme {
            config.theme = theme;
        }
        if let Some(ref shell) = cli.shell {
            config.shell = Some(shell.clone());
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate font size
        if self.font_size < 6.0 || self.font_size > 72.0 {
            return Err(ConfigError::FontSizeOutOfRange {
                size: self.font_size,
                min: 6.0,
                max: 72.0,
            });
        }

        // Validate colors
        self.validate_color(&self.colors.foreground)?;
        self.validate_color(&self.colors.background)?;
        self.validate_color(&self.colors.cursor)?;
        self.validate_color(&self.colors.selection)?;
        for color in &self.colors.ansi {
            self.validate_color(color)?;
        }

        // Validate scrollback
        if self.scrollback_lines == 0 {
            return Err(ConfigError::ValidationError(
                "scrollback_lines must be greater than 0".to_string(),
            ));
        }

        // Validate dimensions
        if self.dimensions.0 < 10 || self.dimensions.1 < 5 {
            return Err(ConfigError::ValidationError(
                "dimensions must be at least 10x5".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate a color string
    fn validate_color(&self, color: &str) -> Result<(), ConfigError> {
        if ColorScheme::parse_hex(color).is_none() {
            return Err(ConfigError::InvalidColor {
                color: color.to_string(),
            });
        }
        Ok(())
    }

    /// Get the default config file path (XDG compliant)
    pub fn default_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("mochi").join("config.toml"))
    }

    /// Load configuration from file (simple version)
    #[allow(dead_code)]
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

    /// Reload configuration from file
    pub fn reload(&mut self) -> Result<(), ConfigError> {
        let cli = CliArgs::default();
        let new_config = Self::load_with_args(&cli)?;
        *self = new_config;
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
    fn test_config_validation_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_font_size_too_small() {
        let config = Config {
            font_size: 2.0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::FontSizeOutOfRange { .. }
        ));
    }

    #[test]
    fn test_config_validation_font_size_too_large() {
        let config = Config {
            font_size: 100.0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::FontSizeOutOfRange { .. }
        ));
    }

    #[test]
    fn test_config_validation_invalid_color() {
        let config = Config {
            colors: ColorScheme {
                foreground: "not-a-color".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::InvalidColor { .. }
        ));
    }

    #[test]
    fn test_config_validation_zero_scrollback() {
        let config = Config {
            scrollback_lines: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::ValidationError(_)
        ));
    }

    #[test]
    fn test_config_validation_small_dimensions() {
        let config = Config {
            dimensions: (5, 3),
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::ValidationError(_)
        ));
    }

    #[test]
    fn test_cli_args_parse_theme_name() {
        assert_eq!(CliArgs::parse_theme_name("dark"), Some(ThemeName::Dark));
        assert_eq!(CliArgs::parse_theme_name("light"), Some(ThemeName::Light));
        assert_eq!(
            CliArgs::parse_theme_name("solarized-dark"),
            Some(ThemeName::SolarizedDark)
        );
        assert_eq!(
            CliArgs::parse_theme_name("solarized-light"),
            Some(ThemeName::SolarizedLight)
        );
        assert_eq!(
            CliArgs::parse_theme_name("dracula"),
            Some(ThemeName::Dracula)
        );
        assert_eq!(CliArgs::parse_theme_name("nord"), Some(ThemeName::Nord));
        assert_eq!(CliArgs::parse_theme_name("invalid"), None);
    }

    #[test]
    fn test_config_parse_toml() {
        let toml_str = r##"
font_family = "JetBrains Mono"
font_size = 16.0
scrollback_lines = 5000
dimensions = [120, 40]
theme = "dracula"
osc52_clipboard = false
osc52_max_size = 50000
cursor_style = "beam"
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
        let config: Result<Config, _> = toml::from_str(toml_str);
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.font_family, "JetBrains Mono");
        assert_eq!(config.font_size, 16.0);
        assert_eq!(config.theme, ThemeName::Dracula);
        assert_eq!(config.dimensions, (120, 40));
    }

    #[test]
    fn test_effective_colors_by_theme() {
        let config_dark = Config {
            theme: ThemeName::Dark,
            ..Default::default()
        };
        let colors = config_dark.effective_colors();
        assert_eq!(colors.background, "#1e1e1e");

        let config_light = Config {
            theme: ThemeName::Light,
            ..Default::default()
        };
        let colors = config_light.effective_colors();
        assert_eq!(colors.background, "#ffffff");

        let config_dracula = Config {
            theme: ThemeName::Dracula,
            ..Default::default()
        };
        let colors = config_dracula.effective_colors();
        assert_eq!(colors.background, "#282a36");

        let config_nord = Config {
            theme: ThemeName::Nord,
            ..Default::default()
        };
        let colors = config_nord.effective_colors();
        assert_eq!(colors.background, "#2e3440");
    }

    #[test]
    fn test_cli_overrides() {
        let mut config = Config::default();
        let cli = CliArgs {
            font_size: Some(20.0),
            theme: Some(ThemeName::Nord),
            shell: Some("/bin/zsh".to_string()),
            ..Default::default()
        };

        Config::apply_cli_overrides(&mut config, &cli);

        assert_eq!(config.font_size, 20.0);
        assert_eq!(config.theme, ThemeName::Nord);
        assert_eq!(config.shell, Some("/bin/zsh".to_string()));
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
