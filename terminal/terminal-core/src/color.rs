//! Color representation for terminal cells
//!
//! Supports:
//! - Default foreground/background
//! - 16 standard ANSI colors (0-15)
//! - 256-color palette (0-255)
//! - 24-bit true color (RGB)

use serde::{Deserialize, Serialize};

/// Color representation supporting all terminal color modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Color {
    /// Default terminal color (foreground or background)
    Default,
    /// Indexed color (0-255)
    /// 0-7: standard colors
    /// 8-15: bright colors
    /// 16-231: 6x6x6 color cube
    /// 232-255: grayscale
    Indexed(u8),
    /// 24-bit RGB color
    Rgb { r: u8, g: u8, b: u8 },
}

impl Color {
    /// Standard ANSI color indices
    pub const BLACK: u8 = 0;
    pub const RED: u8 = 1;
    pub const GREEN: u8 = 2;
    pub const YELLOW: u8 = 3;
    pub const BLUE: u8 = 4;
    pub const MAGENTA: u8 = 5;
    pub const CYAN: u8 = 6;
    pub const WHITE: u8 = 7;

    /// Bright ANSI color indices
    pub const BRIGHT_BLACK: u8 = 8;
    pub const BRIGHT_RED: u8 = 9;
    pub const BRIGHT_GREEN: u8 = 10;
    pub const BRIGHT_YELLOW: u8 = 11;
    pub const BRIGHT_BLUE: u8 = 12;
    pub const BRIGHT_MAGENTA: u8 = 13;
    pub const BRIGHT_CYAN: u8 = 14;
    pub const BRIGHT_WHITE: u8 = 15;

    /// Create a new indexed color
    pub fn indexed(index: u8) -> Self {
        Color::Indexed(index)
    }

    /// Create a new RGB color
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color::Rgb { r, g, b }
    }

    /// Convert indexed color to RGB using standard xterm palette
    pub fn to_rgb(&self) -> (u8, u8, u8) {
        match self {
            Color::Default => (255, 255, 255), // Default to white for foreground
            Color::Indexed(idx) => index_to_rgb(*idx),
            Color::Rgb { r, g, b } => (*r, *g, *b),
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::Default
    }
}

/// Convert a 256-color index to RGB values using xterm palette
fn index_to_rgb(index: u8) -> (u8, u8, u8) {
    match index {
        // Standard colors (0-7)
        0 => (0, 0, 0),       // Black
        1 => (205, 0, 0),     // Red
        2 => (0, 205, 0),     // Green
        3 => (205, 205, 0),   // Yellow
        4 => (0, 0, 238),     // Blue
        5 => (205, 0, 205),   // Magenta
        6 => (0, 205, 205),   // Cyan
        7 => (229, 229, 229), // White

        // Bright colors (8-15)
        8 => (127, 127, 127),  // Bright Black (Gray)
        9 => (255, 0, 0),      // Bright Red
        10 => (0, 255, 0),     // Bright Green
        11 => (255, 255, 0),   // Bright Yellow
        12 => (92, 92, 255),   // Bright Blue
        13 => (255, 0, 255),   // Bright Magenta
        14 => (0, 255, 255),   // Bright Cyan
        15 => (255, 255, 255), // Bright White

        // 6x6x6 color cube (16-231)
        16..=231 => {
            let idx = index - 16;
            let r = idx / 36;
            let g = (idx % 36) / 6;
            let b = idx % 6;
            let to_val = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            (to_val(r), to_val(g), to_val(b))
        }

        // Grayscale (232-255)
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            (gray, gray, gray)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_default() {
        assert_eq!(Color::default(), Color::Default);
    }

    #[test]
    fn test_color_indexed() {
        let color = Color::indexed(1);
        assert_eq!(color, Color::Indexed(1));
    }

    #[test]
    fn test_color_rgb() {
        let color = Color::rgb(255, 128, 64);
        assert_eq!(
            color,
            Color::Rgb {
                r: 255,
                g: 128,
                b: 64
            }
        );
    }

    #[test]
    fn test_standard_colors_to_rgb() {
        assert_eq!(Color::Indexed(0).to_rgb(), (0, 0, 0)); // Black
        assert_eq!(Color::Indexed(1).to_rgb(), (205, 0, 0)); // Red
        assert_eq!(Color::Indexed(7).to_rgb(), (229, 229, 229)); // White
    }

    #[test]
    fn test_bright_colors_to_rgb() {
        assert_eq!(Color::Indexed(8).to_rgb(), (127, 127, 127)); // Bright Black
        assert_eq!(Color::Indexed(15).to_rgb(), (255, 255, 255)); // Bright White
    }

    #[test]
    fn test_color_cube_to_rgb() {
        // First color in cube (black)
        assert_eq!(Color::Indexed(16).to_rgb(), (0, 0, 0));
        // Pure red in cube
        assert_eq!(Color::Indexed(196).to_rgb(), (255, 0, 0));
    }

    #[test]
    fn test_grayscale_to_rgb() {
        // Darkest gray
        assert_eq!(Color::Indexed(232).to_rgb(), (8, 8, 8));
        // Lightest gray
        assert_eq!(Color::Indexed(255).to_rgb(), (238, 238, 238));
    }
}
