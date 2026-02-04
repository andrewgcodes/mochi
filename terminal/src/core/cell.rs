//! Terminal Cell
//!
//! Represents a single cell in the terminal grid, containing a character
//! and its associated styling attributes.

use serde::{Deserialize, Serialize};

/// A single cell in the terminal grid
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    /// The character(s) in this cell. May be empty for continuation cells
    /// of wide characters, or contain multiple codepoints for combining marks.
    pub content: String,
    /// Foreground color
    pub fg: Color,
    /// Background color
    pub bg: Color,
    /// Text style attributes
    pub style: Style,
    /// Hyperlink ID (0 = no hyperlink)
    pub hyperlink_id: u32,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            content: String::new(),
            fg: Color::Default,
            bg: Color::Default,
            style: Style::default(),
            hyperlink_id: 0,
        }
    }
}

impl Cell {
    /// Create a new cell with a single character
    pub fn new(c: char) -> Self {
        Self {
            content: c.to_string(),
            ..Default::default()
        }
    }

    /// Create a new cell with content and style
    pub fn with_style(content: String, fg: Color, bg: Color, style: Style) -> Self {
        Self {
            content,
            fg,
            bg,
            style,
            hyperlink_id: 0,
        }
    }

    /// Check if this cell is empty (no content)
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Check if this cell is a wide character placeholder
    /// (the second cell of a double-width character)
    pub fn is_wide_continuation(&self) -> bool {
        self.content.is_empty() && self.style.wide_char_continuation
    }

    /// Get the display width of this cell's content
    pub fn width(&self) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        use unicode_width::UnicodeWidthStr;
        self.content.width()
    }

    /// Clear the cell to default state
    pub fn clear(&mut self) {
        self.content.clear();
        self.fg = Color::Default;
        self.bg = Color::Default;
        self.style = Style::default();
        self.hyperlink_id = 0;
    }

    /// Clear the cell but preserve background color (for erase operations)
    pub fn erase(&mut self, bg: Color) {
        self.content.clear();
        self.fg = Color::Default;
        self.bg = bg;
        self.style = Style::default();
        self.hyperlink_id = 0;
    }
}

/// Color representation supporting indexed and RGB colors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Color {
    /// Default terminal color (foreground or background)
    Default,
    /// Standard 16-color palette (0-15)
    Indexed(u8),
    /// 24-bit RGB color
    Rgb(u8, u8, u8),
}

impl Default for Color {
    fn default() -> Self {
        Color::Default
    }
}

impl Color {
    /// Standard ANSI colors (0-7)
    pub const BLACK: Color = Color::Indexed(0);
    pub const RED: Color = Color::Indexed(1);
    pub const GREEN: Color = Color::Indexed(2);
    pub const YELLOW: Color = Color::Indexed(3);
    pub const BLUE: Color = Color::Indexed(4);
    pub const MAGENTA: Color = Color::Indexed(5);
    pub const CYAN: Color = Color::Indexed(6);
    pub const WHITE: Color = Color::Indexed(7);

    /// Bright ANSI colors (8-15)
    pub const BRIGHT_BLACK: Color = Color::Indexed(8);
    pub const BRIGHT_RED: Color = Color::Indexed(9);
    pub const BRIGHT_GREEN: Color = Color::Indexed(10);
    pub const BRIGHT_YELLOW: Color = Color::Indexed(11);
    pub const BRIGHT_BLUE: Color = Color::Indexed(12);
    pub const BRIGHT_MAGENTA: Color = Color::Indexed(13);
    pub const BRIGHT_CYAN: Color = Color::Indexed(14);
    pub const BRIGHT_WHITE: Color = Color::Indexed(15);

    /// Convert a 256-color index to RGB
    /// This implements the standard xterm 256-color palette
    pub fn indexed_to_rgb(index: u8) -> (u8, u8, u8) {
        match index {
            // Standard colors (0-15) - using typical xterm defaults
            0 => (0, 0, 0),        // Black
            1 => (205, 0, 0),      // Red
            2 => (0, 205, 0),      // Green
            3 => (205, 205, 0),    // Yellow
            4 => (0, 0, 238),      // Blue
            5 => (205, 0, 205),    // Magenta
            6 => (0, 205, 205),    // Cyan
            7 => (229, 229, 229),  // White
            8 => (127, 127, 127),  // Bright Black
            9 => (255, 0, 0),      // Bright Red
            10 => (0, 255, 0),     // Bright Green
            11 => (255, 255, 0),   // Bright Yellow
            12 => (92, 92, 255),   // Bright Blue
            13 => (255, 0, 255),   // Bright Magenta
            14 => (0, 255, 255),   // Bright Cyan
            15 => (255, 255, 255), // Bright White
            // 216 color cube (16-231)
            16..=231 => {
                let n = index - 16;
                let r = n / 36;
                let g = (n % 36) / 6;
                let b = n % 6;
                let to_rgb = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
                (to_rgb(r), to_rgb(g), to_rgb(b))
            }
            // Grayscale (232-255)
            232..=255 => {
                let gray = 8 + (index - 232) * 10;
                (gray, gray, gray)
            }
        }
    }

    /// Convert this color to RGB, using defaults for Default color
    pub fn to_rgb(&self, is_foreground: bool) -> (u8, u8, u8) {
        match self {
            Color::Default => {
                if is_foreground {
                    (229, 229, 229) // Default foreground (light gray)
                } else {
                    (0, 0, 0) // Default background (black)
                }
            }
            Color::Indexed(i) => Self::indexed_to_rgb(*i),
            Color::Rgb(r, g, b) => (*r, *g, *b),
        }
    }
}

/// Text style attributes
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Style {
    pub bold: bool,
    pub faint: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub inverse: bool,
    pub hidden: bool,
    pub strikethrough: bool,
    /// This cell is the continuation of a wide character
    pub wide_char_continuation: bool,
}

impl Style {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Hyperlink information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hyperlink {
    /// Unique ID for this hyperlink
    pub id: u32,
    /// The URL
    pub url: String,
    /// Optional ID parameter from OSC 8
    pub params: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_default() {
        let cell = Cell::default();
        assert!(cell.is_empty());
        assert_eq!(cell.fg, Color::Default);
        assert_eq!(cell.bg, Color::Default);
    }

    #[test]
    fn test_cell_new() {
        let cell = Cell::new('A');
        assert_eq!(cell.content, "A");
        assert!(!cell.is_empty());
    }

    #[test]
    fn test_cell_clear() {
        let mut cell = Cell::new('A');
        cell.fg = Color::RED;
        cell.style.bold = true;
        cell.clear();
        assert!(cell.is_empty());
        assert_eq!(cell.fg, Color::Default);
        assert!(!cell.style.bold);
    }

    #[test]
    fn test_color_indexed_to_rgb() {
        // Test standard colors
        assert_eq!(Color::indexed_to_rgb(0), (0, 0, 0));
        assert_eq!(Color::indexed_to_rgb(15), (255, 255, 255));

        // Test color cube
        assert_eq!(Color::indexed_to_rgb(16), (0, 0, 0));
        assert_eq!(Color::indexed_to_rgb(231), (255, 255, 255));

        // Test grayscale
        assert_eq!(Color::indexed_to_rgb(232), (8, 8, 8));
        assert_eq!(Color::indexed_to_rgb(255), (238, 238, 238));
    }

    #[test]
    fn test_cell_width() {
        let cell = Cell::new('A');
        assert_eq!(cell.width(), 1);

        let mut wide_cell = Cell::default();
        wide_cell.content = "中".to_string();
        assert_eq!(wide_cell.width(), 2);
    }
}
