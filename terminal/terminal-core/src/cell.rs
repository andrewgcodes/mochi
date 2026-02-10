//! Terminal cell representation
//!
//! Each cell in the terminal grid contains:
//! - A character (Unicode scalar value or empty)
//! - Display attributes (colors, bold, italic, etc.)
//! - Optional hyperlink reference

use serde::{Deserialize, Serialize};

use crate::color::Color;

/// Underline style variants (SGR 4:x subparameters)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UnderlineStyle {
    #[default]
    None,
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}

/// Attributes that affect how a cell is rendered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CellAttributes {
    /// Foreground color
    pub fg: Color,
    /// Background color
    pub bg: Color,
    /// Bold text (SGR 1)
    pub bold: bool,
    /// Faint/dim text (SGR 2)
    pub faint: bool,
    /// Italic text (SGR 3)
    pub italic: bool,
    /// Underlined text (SGR 4)
    pub underline: bool,
    /// Underline style (SGR 4:0-4:5)
    pub underline_style: UnderlineStyle,
    /// Underline color (SGR 58)
    pub underline_color: Color,
    /// Blinking text (SGR 5) - typically rendered as bold or ignored
    pub blink: bool,
    /// Inverse/reverse video (SGR 7)
    pub inverse: bool,
    /// Hidden/invisible text (SGR 8)
    pub hidden: bool,
    /// Strikethrough text (SGR 9)
    pub strikethrough: bool,
}

impl CellAttributes {
    /// Create new default attributes
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all attributes to default
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Get effective foreground color (accounting for inverse)
    pub fn effective_fg(&self) -> Color {
        if self.inverse {
            self.bg
        } else {
            self.fg
        }
    }

    /// Get effective background color (accounting for inverse)
    pub fn effective_bg(&self) -> Color {
        if self.inverse {
            self.fg
        } else {
            self.bg
        }
    }
}

/// A single cell in the terminal grid
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    /// The character stored in this cell
    /// Empty string means the cell is empty (space)
    /// Can contain a grapheme cluster for combining characters
    content: String,
    /// Display attributes
    pub attrs: CellAttributes,
    /// Width of this cell (1 for normal, 2 for wide chars, 0 for continuation)
    /// A wide character occupies two cells: the first has width=2, the second has width=0
    width: u8,
    /// Hyperlink ID (0 means no hyperlink)
    pub hyperlink_id: u32,
}

impl Cell {
    /// Create a new empty cell
    pub fn new() -> Self {
        Self {
            content: String::new(),
            attrs: CellAttributes::default(),
            width: 1,
            hyperlink_id: 0,
        }
    }

    /// Create a cell with a character
    pub fn with_char(c: char) -> Self {
        let width = unicode_display_width(c);
        Self {
            content: c.to_string(),
            attrs: CellAttributes::default(),
            width,
            hyperlink_id: 0,
        }
    }

    /// Create a cell with a character and attributes
    pub fn with_char_and_attrs(c: char, attrs: CellAttributes) -> Self {
        let width = unicode_display_width(c);
        Self {
            content: c.to_string(),
            attrs,
            width,
            hyperlink_id: 0,
        }
    }

    /// Set the character content
    pub fn set_char(&mut self, c: char) {
        self.content = c.to_string();
        self.width = unicode_display_width(c);
    }

    /// Set content from a string (for grapheme clusters)
    pub fn set_content(&mut self, s: &str) {
        self.content = s.to_string();
        // Calculate width from first char, or 1 if empty
        self.width = s.chars().next().map(unicode_display_width).unwrap_or(1);
    }

    /// Get the character content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the display character (space if empty)
    pub fn display_char(&self) -> char {
        self.content.chars().next().unwrap_or(' ')
    }

    /// Check if cell is empty (no content or just space)
    pub fn is_empty(&self) -> bool {
        self.content.is_empty() || self.content == " "
    }

    /// Get the display width of this cell
    pub fn width(&self) -> u8 {
        self.width
    }

    /// Set this cell as a wide character continuation (width=0)
    pub fn set_continuation(&mut self) {
        self.content.clear();
        self.width = 0;
    }

    /// Check if this is a continuation cell
    pub fn is_continuation(&self) -> bool {
        self.width == 0
    }

    /// Clear the cell (reset to empty with given attributes)
    pub fn clear(&mut self, attrs: CellAttributes) {
        self.content.clear();
        self.attrs = attrs;
        self.width = 1;
        self.hyperlink_id = 0;
    }

    /// Reset cell to default state
    pub fn reset(&mut self) {
        self.content.clear();
        self.attrs = CellAttributes::default();
        self.width = 1;
        self.hyperlink_id = 0;
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate the display width of a Unicode character
/// Returns 2 for wide characters (CJK, etc.), 1 for normal, 0 for combining marks
fn unicode_display_width(c: char) -> u8 {
    use unicode_width::UnicodeWidthChar;
    match c.width() {
        Some(0) => 0, // Combining characters
        Some(1) => 1, // Normal width
        Some(2) => 2, // Wide characters (CJK, etc.)
        Some(w) => w as u8,
        None => 1, // Control characters - treat as 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_new() {
        let cell = Cell::new();
        assert!(cell.is_empty());
        assert_eq!(cell.width(), 1);
    }

    #[test]
    fn test_cell_with_char() {
        let cell = Cell::with_char('A');
        assert_eq!(cell.display_char(), 'A');
        assert_eq!(cell.width(), 1);
        assert!(!cell.is_empty());
    }

    #[test]
    fn test_cell_wide_char() {
        // CJK character should have width 2
        let cell = Cell::with_char('中');
        assert_eq!(cell.display_char(), '中');
        assert_eq!(cell.width(), 2);
    }

    #[test]
    fn test_cell_clear() {
        let mut cell = Cell::with_char('X');
        cell.clear(CellAttributes::default());
        assert!(cell.is_empty());
    }

    #[test]
    fn test_attributes_inverse() {
        let mut attrs = CellAttributes::new();
        attrs.fg = Color::Indexed(1); // Red
        attrs.bg = Color::Indexed(0); // Black
        attrs.inverse = true;

        assert_eq!(attrs.effective_fg(), Color::Indexed(0)); // Black
        assert_eq!(attrs.effective_bg(), Color::Indexed(1)); // Red
    }

    #[test]
    fn test_attributes_reset() {
        let mut attrs = CellAttributes::new();
        attrs.bold = true;
        attrs.italic = true;
        attrs.fg = Color::Indexed(1);

        attrs.reset();

        assert!(!attrs.bold);
        assert!(!attrs.italic);
        assert_eq!(attrs.fg, Color::Default);
    }
}
