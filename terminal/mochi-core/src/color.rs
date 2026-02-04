//! Color representation for terminal cells.
//!
//! Supports:
//! - Default foreground/background colors
//! - 16 standard ANSI colors (0-15)
//! - 256 color palette (0-255)
//! - 24-bit true color (RGB)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Color {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

impl Default for Color {
    fn default() -> Self {
        Color::Default
    }
}

impl Color {
    pub const BLACK: Color = Color::Indexed(0);
    pub const RED: Color = Color::Indexed(1);
    pub const GREEN: Color = Color::Indexed(2);
    pub const YELLOW: Color = Color::Indexed(3);
    pub const BLUE: Color = Color::Indexed(4);
    pub const MAGENTA: Color = Color::Indexed(5);
    pub const CYAN: Color = Color::Indexed(6);
    pub const WHITE: Color = Color::Indexed(7);

    pub const BRIGHT_BLACK: Color = Color::Indexed(8);
    pub const BRIGHT_RED: Color = Color::Indexed(9);
    pub const BRIGHT_GREEN: Color = Color::Indexed(10);
    pub const BRIGHT_YELLOW: Color = Color::Indexed(11);
    pub const BRIGHT_BLUE: Color = Color::Indexed(12);
    pub const BRIGHT_MAGENTA: Color = Color::Indexed(13);
    pub const BRIGHT_CYAN: Color = Color::Indexed(14);
    pub const BRIGHT_WHITE: Color = Color::Indexed(15);

    pub fn from_ansi_fg(code: u8) -> Option<Color> {
        match code {
            30..=37 => Some(Color::Indexed(code - 30)),
            90..=97 => Some(Color::Indexed(code - 90 + 8)),
            39 => Some(Color::Default),
            _ => None,
        }
    }

    pub fn from_ansi_bg(code: u8) -> Option<Color> {
        match code {
            40..=47 => Some(Color::Indexed(code - 40)),
            100..=107 => Some(Color::Indexed(code - 100 + 8)),
            49 => Some(Color::Default),
            _ => None,
        }
    }

    pub fn to_rgb(&self, palette: &ColorPalette) -> (u8, u8, u8) {
        match *self {
            Color::Default => palette.default_fg,
            Color::Indexed(idx) => palette.get(idx),
            Color::Rgb(r, g, b) => (r, g, b),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColorPalette {
    colors: [(u8, u8, u8); 256],
    pub default_fg: (u8, u8, u8),
    pub default_bg: (u8, u8, u8),
}

impl Default for ColorPalette {
    fn default() -> Self {
        let mut colors = [(0u8, 0u8, 0u8); 256];

        colors[0] = (0, 0, 0);
        colors[1] = (205, 49, 49);
        colors[2] = (13, 188, 121);
        colors[3] = (229, 229, 16);
        colors[4] = (36, 114, 200);
        colors[5] = (188, 63, 188);
        colors[6] = (17, 168, 205);
        colors[7] = (229, 229, 229);

        colors[8] = (102, 102, 102);
        colors[9] = (241, 76, 76);
        colors[10] = (35, 209, 139);
        colors[11] = (245, 245, 67);
        colors[12] = (59, 142, 234);
        colors[13] = (214, 112, 214);
        colors[14] = (41, 184, 219);
        colors[15] = (255, 255, 255);

        for i in 0..216 {
            let r = (i / 36) % 6;
            let g = (i / 6) % 6;
            let b = i % 6;
            let r = if r == 0 { 0 } else { 55 + r * 40 };
            let g = if g == 0 { 0 } else { 55 + g * 40 };
            let b = if b == 0 { 0 } else { 55 + b * 40 };
            colors[16 + i as usize] = (r, g, b);
        }

        for i in 0..24 {
            let gray = 8 + i * 10;
            colors[232 + i as usize] = (gray, gray, gray);
        }

        ColorPalette {
            colors,
            default_fg: (229, 229, 229),
            default_bg: (0, 0, 0),
        }
    }
}

impl ColorPalette {
    pub fn get(&self, index: u8) -> (u8, u8, u8) {
        self.colors[index as usize]
    }

    pub fn set(&mut self, index: u8, color: (u8, u8, u8)) {
        self.colors[index as usize] = color;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_fg_colors() {
        assert_eq!(Color::from_ansi_fg(30), Some(Color::Indexed(0)));
        assert_eq!(Color::from_ansi_fg(37), Some(Color::Indexed(7)));
        assert_eq!(Color::from_ansi_fg(90), Some(Color::Indexed(8)));
        assert_eq!(Color::from_ansi_fg(97), Some(Color::Indexed(15)));
        assert_eq!(Color::from_ansi_fg(39), Some(Color::Default));
        assert_eq!(Color::from_ansi_fg(50), None);
    }

    #[test]
    fn test_ansi_bg_colors() {
        assert_eq!(Color::from_ansi_bg(40), Some(Color::Indexed(0)));
        assert_eq!(Color::from_ansi_bg(47), Some(Color::Indexed(7)));
        assert_eq!(Color::from_ansi_bg(100), Some(Color::Indexed(8)));
        assert_eq!(Color::from_ansi_bg(107), Some(Color::Indexed(15)));
        assert_eq!(Color::from_ansi_bg(49), Some(Color::Default));
    }

    #[test]
    fn test_256_color_palette() {
        let palette = ColorPalette::default();
        assert_eq!(palette.get(0), (0, 0, 0));
        assert_eq!(palette.get(15), (255, 255, 255));
        assert_eq!(palette.get(232), (8, 8, 8));
        assert_eq!(palette.get(255), (238, 238, 238));
    }
}
