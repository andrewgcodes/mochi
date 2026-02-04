//! Cell representation for terminal grid
//!
//! Each cell in the terminal grid contains a character (or grapheme cluster)
//! along with styling attributes like colors and text decorations.

use serde::{Deserialize, Serialize};

/// Represents a color in the terminal.
///
/// Supports:
/// - Default (terminal's default fg/bg)
/// - Indexed colors (0-255, includes 16 standard + 216 cube + 24 grayscale)
/// - True color (24-bit RGB)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Color {
    /// Terminal's default color
    Default,
    /// Indexed color (0-15 standard, 16-231 color cube, 232-255 grayscale)
    Indexed(u8),
    /// True color RGB
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
}

/// Text style attributes for a cell.
///
/// These correspond to SGR (Select Graphic Rendition) attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Style {
    /// Bold text (SGR 1)
    pub bold: bool,
    /// Faint/dim text (SGR 2)
    pub faint: bool,
    /// Italic text (SGR 3)
    pub italic: bool,
    /// Underlined text (SGR 4)
    pub underline: bool,
    /// Blinking text (SGR 5) - we may not animate this but track the state
    pub blink: bool,
    /// Inverse/reverse video (SGR 7)
    pub inverse: bool,
    /// Hidden/invisible text (SGR 8)
    pub hidden: bool,
    /// Strikethrough text (SGR 9)
    pub strikethrough: bool,
}

impl Style {
    /// Create a new default style (no attributes set)
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all attributes to default
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Check if any style attribute is set
    pub fn has_any(&self) -> bool {
        self.bold
            || self.faint
            || self.italic
            || self.underline
            || self.blink
            || self.inverse
            || self.hidden
            || self.strikethrough
    }
}

/// A single cell in the terminal grid.
///
/// Each cell contains:
/// - A character (stored as a String to support grapheme clusters)
/// - Foreground and background colors
/// - Style attributes
/// - Optional hyperlink ID for OSC 8 links
///
/// # Unicode Handling
///
/// We store characters as strings to handle:
/// - Basic ASCII and Unicode codepoints
/// - Combining characters (stored with their base character)
/// - Wide characters (CJK) - these occupy two cells, with the second
///   cell marked as a "continuation" (empty string)
///
/// # Width Handling
///
/// - Normal characters have width 1
/// - Wide characters (CJK, some emoji) have width 2
/// - Zero-width characters (combining marks) have width 0 and are
///   appended to the previous cell's content
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    /// The character(s) in this cell. Empty string means this is a
    /// continuation cell for a wide character.
    pub content: String,
    /// Foreground color
    pub fg: Color,
    /// Background color
    pub bg: Color,
    /// Text style attributes
    pub style: Style,
    /// Hyperlink ID (for OSC 8 links), None if no link
    pub hyperlink_id: Option<u32>,
    /// Width of this cell (0 for continuation, 1 for normal, 2 for wide char start)
    pub width: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            content: String::new(),
            fg: Color::Default,
            bg: Color::Default,
            style: Style::default(),
            hyperlink_id: None,
            width: 1,
        }
    }
}

impl Cell {
    /// Create a new empty cell with default attributes
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a cell with a single character
    pub fn with_char(c: char) -> Self {
        Self {
            content: c.to_string(),
            ..Self::default()
        }
    }

    /// Create a cell with content and colors
    pub fn with_content(content: String, fg: Color, bg: Color, style: Style) -> Self {
        Self {
            content,
            fg,
            bg,
            style,
            hyperlink_id: None,
            width: 1,
        }
    }

    /// Check if this cell is empty (no content)
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Check if this is a continuation cell (second cell of a wide character)
    pub fn is_continuation(&self) -> bool {
        self.width == 0
    }

    /// Clear the cell to default state
    pub fn clear(&mut self) {
        self.content.clear();
        self.fg = Color::Default;
        self.bg = Color::Default;
        self.style = Style::default();
        self.hyperlink_id = None;
        self.width = 1;
    }

    /// Clear content but preserve background color (for erase operations)
    pub fn erase(&mut self, bg: Color) {
        self.content.clear();
        self.fg = Color::Default;
        self.bg = bg;
        self.style = Style::default();
        self.hyperlink_id = None;
        self.width = 1;
    }

    /// Set the character content
    pub fn set_char(&mut self, c: char) {
        self.content.clear();
        self.content.push(c);
    }

    /// Append a combining character to this cell
    pub fn append_combining(&mut self, c: char) {
        self.content.push(c);
    }

    /// Get the display character (first char or space if empty)
    pub fn display_char(&self) -> char {
        self.content.chars().next().unwrap_or(' ')
    }
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
        assert!(!cell.style.has_any());
    }

    #[test]
    fn test_cell_with_char() {
        let cell = Cell::with_char('A');
        assert_eq!(cell.content, "A");
        assert_eq!(cell.display_char(), 'A');
    }

    #[test]
    fn test_cell_clear() {
        let mut cell = Cell::with_char('X');
        cell.fg = Color::RED;
        cell.style.bold = true;
        cell.clear();
        assert!(cell.is_empty());
        assert_eq!(cell.fg, Color::Default);
        assert!(!cell.style.bold);
    }

    #[test]
    fn test_cell_erase_preserves_bg() {
        let mut cell = Cell::with_char('X');
        cell.erase(Color::BLUE);
        assert!(cell.is_empty());
        assert_eq!(cell.bg, Color::BLUE);
    }

    #[test]
    fn test_color_constants() {
        assert_eq!(Color::BLACK, Color::Indexed(0));
        assert_eq!(Color::BRIGHT_WHITE, Color::Indexed(15));
    }

    #[test]
    fn test_style_has_any() {
        let mut style = Style::default();
        assert!(!style.has_any());
        style.bold = true;
        assert!(style.has_any());
        style.reset();
        assert!(!style.has_any());
    }

    #[test]
    fn test_combining_characters() {
        let mut cell = Cell::with_char('e');
        cell.append_combining('\u{0301}'); // combining acute accent
        assert_eq!(cell.content, "e\u{0301}");
        assert_eq!(cell.display_char(), 'e');
    }
}
