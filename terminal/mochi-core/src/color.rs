//! Terminal color representation
//!
//! Supports:
//! - Named 16-color palette (standard ANSI colors)
//! - 256-color indexed palette
//! - 24-bit true color (RGB)

use serde::{Deserialize, Serialize};

/// Represents a terminal color
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Color {
    /// Default foreground or background color
    Default,
    /// Named color from the 16-color palette (0-15)
    Named(NamedColor),
    /// 256-color palette index (0-255)
    Indexed(u8),
    /// 24-bit RGB color
    Rgb(Rgb),
}

impl Default for Color {
    fn default() -> Self {
        Color::Default
    }
}

/// Named colors from the standard 16-color ANSI palette
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum NamedColor {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
    BrightBlack = 8,
    BrightRed = 9,
    BrightGreen = 10,
    BrightYellow = 11,
    BrightBlue = 12,
    BrightMagenta = 13,
    BrightCyan = 14,
    BrightWhite = 15,
}

impl NamedColor {
    /// Convert from SGR color code (30-37 for fg, 40-47 for bg)
    pub fn from_sgr_normal(code: u8) -> Option<Self> {
        match code {
            0 => Some(NamedColor::Black),
            1 => Some(NamedColor::Red),
            2 => Some(NamedColor::Green),
            3 => Some(NamedColor::Yellow),
            4 => Some(NamedColor::Blue),
            5 => Some(NamedColor::Magenta),
            6 => Some(NamedColor::Cyan),
            7 => Some(NamedColor::White),
            _ => None,
        }
    }

    /// Convert from SGR bright color code (90-97 for fg, 100-107 for bg)
    pub fn from_sgr_bright(code: u8) -> Option<Self> {
        match code {
            0 => Some(NamedColor::BrightBlack),
            1 => Some(NamedColor::BrightRed),
            2 => Some(NamedColor::BrightGreen),
            3 => Some(NamedColor::BrightYellow),
            4 => Some(NamedColor::BrightBlue),
            5 => Some(NamedColor::BrightMagenta),
            6 => Some(NamedColor::BrightCyan),
            7 => Some(NamedColor::BrightWhite),
            _ => None,
        }
    }

    /// Get the index in the 256-color palette
    pub fn to_index(self) -> u8 {
        self as u8
    }
}

/// 24-bit RGB color
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
}

/// Default color palette for 256-color mode
/// Colors 0-15: Standard ANSI colors
/// Colors 16-231: 6x6x6 color cube
/// Colors 232-255: Grayscale ramp
pub fn default_256_palette() -> [Rgb; 256] {
    let mut palette = [Rgb::new(0, 0, 0); 256];

    // Standard colors (0-7)
    palette[0] = Rgb::new(0, 0, 0);       // Black
    palette[1] = Rgb::new(205, 0, 0);     // Red
    palette[2] = Rgb::new(0, 205, 0);     // Green
    palette[3] = Rgb::new(205, 205, 0);   // Yellow
    palette[4] = Rgb::new(0, 0, 238);     // Blue
    palette[5] = Rgb::new(205, 0, 205);   // Magenta
    palette[6] = Rgb::new(0, 205, 205);   // Cyan
    palette[7] = Rgb::new(229, 229, 229); // White

    // Bright colors (8-15)
    palette[8] = Rgb::new(127, 127, 127);  // Bright Black
    palette[9] = Rgb::new(255, 0, 0);      // Bright Red
    palette[10] = Rgb::new(0, 255, 0);     // Bright Green
    palette[11] = Rgb::new(255, 255, 0);   // Bright Yellow
    palette[12] = Rgb::new(92, 92, 255);   // Bright Blue
    palette[13] = Rgb::new(255, 0, 255);   // Bright Magenta
    palette[14] = Rgb::new(0, 255, 255);   // Bright Cyan
    palette[15] = Rgb::new(255, 255, 255); // Bright White

    // 6x6x6 color cube (16-231)
    let cube_values = [0u8, 95, 135, 175, 215, 255];
    for r in 0..6 {
        for g in 0..6 {
            for b in 0..6 {
                let index = 16 + r * 36 + g * 6 + b;
                palette[index] = Rgb::new(cube_values[r], cube_values[g], cube_values[b]);
            }
        }
    }

    // Grayscale ramp (232-255)
    for i in 0..24 {
        let gray = (i * 10 + 8) as u8;
        palette[232 + i] = Rgb::new(gray, gray, gray);
    }

    palette
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_color_from_sgr() {
        assert_eq!(NamedColor::from_sgr_normal(0), Some(NamedColor::Black));
        assert_eq!(NamedColor::from_sgr_normal(7), Some(NamedColor::White));
        assert_eq!(NamedColor::from_sgr_normal(8), None);

        assert_eq!(NamedColor::from_sgr_bright(0), Some(NamedColor::BrightBlack));
        assert_eq!(NamedColor::from_sgr_bright(7), Some(NamedColor::BrightWhite));
    }

    #[test]
    fn test_256_palette_size() {
        let palette = default_256_palette();
        assert_eq!(palette.len(), 256);
    }

    #[test]
    fn test_color_cube() {
        let palette = default_256_palette();
        // Color 16 should be black (0,0,0)
        assert_eq!(palette[16], Rgb::new(0, 0, 0));
        // Color 231 should be white (255,255,255)
        assert_eq!(palette[231], Rgb::new(255, 255, 255));
    }
}
