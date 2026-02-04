//! Terminal configuration
//!
//! This module handles terminal configuration including:
//! - Window size (rows, columns)
//! - Font settings
//! - Color scheme
//! - Scrollback size
//! - Security settings (OSC 52, etc.)

use serde::{Deserialize, Serialize};

/// Terminal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Number of rows
    pub rows: usize,
    /// Number of columns
    pub cols: usize,
    /// Font family
    pub font_family: String,
    /// Font size in points
    pub font_size: f32,
    /// Maximum scrollback lines
    pub scrollback_lines: usize,
    /// Whether to enable OSC 52 clipboard access
    pub osc52_enabled: bool,
    /// Maximum OSC 52 payload size (bytes)
    pub osc52_max_size: usize,
    /// Color scheme
    pub colors: ColorScheme,
    /// Cell padding in pixels
    pub cell_padding: f32,
    /// Cursor blink interval in milliseconds
    pub cursor_blink_ms: u64,
    /// Whether to show bold text as bright colors
    pub bold_is_bright: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            rows: 24,
            cols: 80,
            font_family: String::from("monospace"),
            font_size: 14.0,
            scrollback_lines: 10000,
            osc52_enabled: false, // Disabled by default for security
            osc52_max_size: 100000,
            colors: ColorScheme::default(),
            cell_padding: 0.0,
            cursor_blink_ms: 500,
            bold_is_bright: false,
        }
    }
}

impl Config {
    /// Load configuration from a file
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a file
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the default config file path
    pub fn default_path() -> Option<String> {
        if let Some(config_dir) = dirs_config_dir() {
            Some(format!("{}/mochi/config.json", config_dir))
        } else {
            None
        }
    }
}

/// Get the config directory (XDG_CONFIG_HOME or ~/.config)
fn dirs_config_dir() -> Option<String> {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|home| format!("{}/.config", home))
        })
}

/// Color scheme for the terminal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    /// Foreground color (default text)
    pub foreground: Rgb,
    /// Background color
    pub background: Rgb,
    /// Cursor color
    pub cursor: Rgb,
    /// Selection background color
    pub selection: Rgb,
    /// 16-color palette (ANSI colors)
    pub palette: [Rgb; 16],
}

impl Default for ColorScheme {
    fn default() -> Self {
        ColorScheme {
            foreground: Rgb::new(229, 229, 229),
            background: Rgb::new(0, 0, 0),
            cursor: Rgb::new(229, 229, 229),
            selection: Rgb::new(68, 68, 68),
            palette: [
                // Normal colors (0-7)
                Rgb::new(0, 0, 0),       // Black
                Rgb::new(205, 0, 0),     // Red
                Rgb::new(0, 205, 0),     // Green
                Rgb::new(205, 205, 0),   // Yellow
                Rgb::new(0, 0, 238),     // Blue
                Rgb::new(205, 0, 205),   // Magenta
                Rgb::new(0, 205, 205),   // Cyan
                Rgb::new(229, 229, 229), // White
                // Bright colors (8-15)
                Rgb::new(127, 127, 127), // Bright Black
                Rgb::new(255, 0, 0),     // Bright Red
                Rgb::new(0, 255, 0),     // Bright Green
                Rgb::new(255, 255, 0),   // Bright Yellow
                Rgb::new(92, 92, 255),   // Bright Blue
                Rgb::new(255, 0, 255),   // Bright Magenta
                Rgb::new(0, 255, 255),   // Bright Cyan
                Rgb::new(255, 255, 255), // Bright White
            ],
        }
    }
}

/// RGB color
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Rgb { r, g, b }
    }

    /// Convert to a normalized float array [0.0, 1.0]
    pub fn to_f32_array(&self) -> [f32; 3] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
        ]
    }

    /// Convert to a u32 (0xRRGGBB)
    pub fn to_u32(&self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Create from a u32 (0xRRGGBB)
    pub fn from_u32(value: u32) -> Self {
        Rgb {
            r: ((value >> 16) & 0xFF) as u8,
            g: ((value >> 8) & 0xFF) as u8,
            b: (value & 0xFF) as u8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.rows, 24);
        assert_eq!(config.cols, 80);
        assert!(!config.osc52_enabled);
    }

    #[test]
    fn test_rgb_conversion() {
        let rgb = Rgb::new(255, 128, 64);
        let arr = rgb.to_f32_array();
        assert!((arr[0] - 1.0).abs() < 0.01);
        assert!((arr[1] - 0.5).abs() < 0.01);
        assert!((arr[2] - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_rgb_u32() {
        let rgb = Rgb::new(0xAB, 0xCD, 0xEF);
        assert_eq!(rgb.to_u32(), 0xABCDEF);
        assert_eq!(Rgb::from_u32(0xABCDEF), rgb);
    }
}
