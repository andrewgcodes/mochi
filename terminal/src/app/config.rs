//! Configuration for the terminal emulator

use serde::{Deserialize, Serialize};

use crate::core::Color;

/// Terminal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Font family name
    pub font_family: String,
    /// Font size in points
    pub font_size: f32,
    /// Maximum scrollback lines
    pub scrollback_lines: usize,
    /// Color palette (256 colors)
    pub colors: ColorPalette,
    /// Security settings
    pub security: SecurityConfig,
    /// Window settings
    pub window: WindowConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_family: "monospace".to_string(),
            font_size: 12.0,
            scrollback_lines: 10000,
            colors: ColorPalette::default(),
            security: SecurityConfig::default(),
            window: WindowConfig::default(),
        }
    }
}

/// Color palette configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPalette {
    /// Default foreground color
    pub foreground: (u8, u8, u8),
    /// Default background color
    pub background: (u8, u8, u8),
    /// Cursor color
    pub cursor: (u8, u8, u8),
    /// Selection color
    pub selection: (u8, u8, u8),
    /// The 16 ANSI colors (0-15)
    pub ansi: [(u8, u8, u8); 16],
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            foreground: (255, 255, 255),
            background: (0, 0, 0),
            cursor: (255, 255, 255),
            selection: (68, 68, 68),
            // Default ANSI colors (similar to xterm)
            ansi: [
                (0, 0, 0),       // 0: Black
                (205, 0, 0),     // 1: Red
                (0, 205, 0),     // 2: Green
                (205, 205, 0),   // 3: Yellow
                (0, 0, 238),     // 4: Blue
                (205, 0, 205),   // 5: Magenta
                (0, 205, 205),   // 6: Cyan
                (229, 229, 229), // 7: White
                (127, 127, 127), // 8: Bright Black
                (255, 0, 0),     // 9: Bright Red
                (0, 255, 0),     // 10: Bright Green
                (255, 255, 0),   // 11: Bright Yellow
                (92, 92, 255),   // 12: Bright Blue
                (255, 0, 255),   // 13: Bright Magenta
                (0, 255, 255),   // 14: Bright Cyan
                (255, 255, 255), // 15: Bright White
            ],
        }
    }
}

impl ColorPalette {
    /// Get the RGB color for an indexed color (0-255)
    pub fn get_indexed(&self, index: u8) -> (u8, u8, u8) {
        match index {
            // ANSI colors
            0..=15 => self.ansi[index as usize],
            // 216 color cube (16-231)
            16..=231 => {
                let n = index - 16;
                let b = n % 6;
                let g = (n / 6) % 6;
                let r = n / 36;
                let to_component = |c: u8| if c == 0 { 0 } else { 55 + c * 40 };
                (to_component(r), to_component(g), to_component(b))
            },
            // Grayscale (232-255)
            232..=255 => {
                let gray = 8 + (index - 232) * 10;
                (gray, gray, gray)
            },
        }
    }

    /// Convert a Color to RGB
    pub fn color_to_rgb(&self, color: Color, is_foreground: bool) -> (u8, u8, u8) {
        match color {
            Color::Default => {
                if is_foreground {
                    self.foreground
                } else {
                    self.background
                }
            },
            Color::Indexed(i) => self.get_indexed(i),
            Color::Rgb(r, g, b) => (r, g, b),
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Allow OSC 52 clipboard read
    pub osc52_read: bool,
    /// Allow OSC 52 clipboard write
    pub osc52_write: bool,
    /// Maximum OSC 52 payload size in bytes
    pub osc52_max_bytes: usize,
    /// Allow file:// URLs in hyperlinks
    pub allow_file_urls: bool,
    /// Confirm large pastes (bytes threshold, 0 = disabled)
    pub paste_confirm_threshold: usize,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            osc52_read: false,
            osc52_write: false,
            osc52_max_bytes: 102400, // 100KB
            allow_file_urls: false,
            paste_confirm_threshold: 0,
        }
    }
}

/// Window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Initial window width in columns
    pub columns: u16,
    /// Initial window height in rows
    pub rows: u16,
    /// Window title
    pub title: String,
    /// Enable window decorations
    pub decorations: bool,
    /// Window opacity (0.0 - 1.0)
    pub opacity: f32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            columns: 80,
            rows: 24,
            title: "Mochi Terminal".to_string(),
            decorations: true,
            opacity: 1.0,
        }
    }
}

impl Config {
    /// Load configuration from a file
    pub fn load(path: &std::path::Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a file
    pub fn save(&self, path: &std::path::Path) -> Result<(), ConfigError> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Load configuration from default location or return default config
    pub fn load_or_default() -> Self {
        // Try to load from ~/.config/mochi/config.json
        if let Some(config_dir) = dirs_config_path() {
            let config_path = config_dir.join("config.json");
            if config_path.exists() {
                if let Ok(config) = Self::load(&config_path) {
                    return config;
                }
            }
        }
        Self::default()
    }
}

/// Get the configuration directory path
fn dirs_config_path() -> Option<std::path::PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|home| std::path::PathBuf::from(home).join(".config").join("mochi"))
}

/// Configuration error
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.font_size, 12.0);
        assert_eq!(config.scrollback_lines, 10000);
        assert!(!config.security.osc52_read);
    }

    #[test]
    fn test_color_palette_indexed() {
        let palette = ColorPalette::default();

        // ANSI colors
        assert_eq!(palette.get_indexed(0), (0, 0, 0)); // Black
        assert_eq!(palette.get_indexed(1), (205, 0, 0)); // Red

        // Color cube
        assert_eq!(palette.get_indexed(16), (0, 0, 0)); // First color cube entry
        assert_eq!(palette.get_indexed(231), (255, 255, 255)); // Last color cube entry

        // Grayscale
        assert_eq!(palette.get_indexed(232), (8, 8, 8)); // First grayscale
        assert_eq!(palette.get_indexed(255), (238, 238, 238)); // Last grayscale
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        let restored: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(config.font_size, restored.font_size);
    }
}
