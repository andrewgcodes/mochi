//! Terminal cell representation
//!
//! A cell represents a single character position in the terminal grid,
//! containing the character content, colors, and text attributes.

use serde::{Deserialize, Serialize};

/// Represents a color in the terminal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Color {
    /// Default foreground or background color
    #[default]
    Default,
    /// Indexed color from the 256-color palette
    /// 0-7: Standard colors
    /// 8-15: Bright colors
    /// 16-231: 6x6x6 color cube
    /// 232-255: Grayscale
    Indexed(u8),
    /// True color (24-bit RGB)
    Rgb(u8, u8, u8),
}

/// Text attributes for a cell
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CellAttributes {
    /// Bold text (SGR 1)
    pub bold: bool,
    /// Faint/dim text (SGR 2)
    pub faint: bool,
    /// Italic text (SGR 3)
    pub italic: bool,
    /// Underlined text (SGR 4)
    pub underline: bool,
    /// Blinking text (SGR 5)
    pub blink: bool,
    /// Inverse/reverse video (SGR 7)
    pub inverse: bool,
    /// Hidden/invisible text (SGR 8)
    pub hidden: bool,
    /// Strikethrough text (SGR 9)
    pub strikethrough: bool,
}

impl CellAttributes {
    /// Create new default attributes (all false)
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all attributes to default
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Check if any attribute is set
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

/// A single cell in the terminal grid
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    /// The character content of this cell
    /// May be empty (for cells that are part of a wide character),
    /// a single character, or a grapheme cluster (base + combining marks)
    content: String,
    /// Foreground color
    fg: Color,
    /// Background color
    bg: Color,
    /// Text attributes
    attrs: CellAttributes,
    /// Hyperlink ID (for OSC 8), 0 = no link
    hyperlink_id: u32,
    /// Width of this cell (1 for normal, 2 for wide chars, 0 for continuation)
    width: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            content: String::new(),
            fg: Color::Default,
            bg: Color::Default,
            attrs: CellAttributes::default(),
            hyperlink_id: 0,
            width: 1,
        }
    }
}

impl Cell {
    /// Create a new empty cell
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a cell with a single character
    pub fn with_char(c: char) -> Self {
        let width = unicode_display_width(c);
        Self {
            content: c.to_string(),
            width,
            ..Self::default()
        }
    }

    /// Create a cell with full attributes
    pub fn with_attrs(c: char, fg: Color, bg: Color, attrs: CellAttributes) -> Self {
        let width = unicode_display_width(c);
        Self {
            content: c.to_string(),
            fg,
            bg,
            attrs,
            hyperlink_id: 0,
            width,
        }
    }

    /// Get the character content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Set the character content
    pub fn set_content(&mut self, c: char) {
        self.content.clear();
        self.content.push(c);
        self.width = unicode_display_width(c);
    }

    /// Set the content to a string (for grapheme clusters)
    pub fn set_content_str(&mut self, s: &str) {
        self.content.clear();
        self.content.push_str(s);
        // Width is determined by the first character for grapheme clusters
        self.width = s.chars().next().map(unicode_display_width).unwrap_or(1);
    }

    /// Clear the cell content (make it empty/space)
    pub fn clear(&mut self) {
        self.content.clear();
        self.width = 1;
    }

    /// Get the foreground color
    pub fn fg(&self) -> Color {
        self.fg
    }

    /// Set the foreground color
    pub fn set_fg(&mut self, color: Color) {
        self.fg = color;
    }

    /// Get the background color
    pub fn bg(&self) -> Color {
        self.bg
    }

    /// Set the background color
    pub fn set_bg(&mut self, color: Color) {
        self.bg = color;
    }

    /// Get the text attributes
    pub fn attrs(&self) -> &CellAttributes {
        &self.attrs
    }

    /// Get mutable reference to text attributes
    pub fn attrs_mut(&mut self) -> &mut CellAttributes {
        &mut self.attrs
    }

    /// Set the text attributes
    pub fn set_attrs(&mut self, attrs: CellAttributes) {
        self.attrs = attrs;
    }

    /// Get the hyperlink ID
    pub fn hyperlink_id(&self) -> u32 {
        self.hyperlink_id
    }

    /// Set the hyperlink ID
    pub fn set_hyperlink_id(&mut self, id: u32) {
        self.hyperlink_id = id;
    }

    /// Get the display width of this cell
    pub fn width(&self) -> u8 {
        self.width
    }

    /// Set the width (used for wide character handling)
    pub fn set_width(&mut self, width: u8) {
        self.width = width;
    }

    /// Check if this cell is empty (no content)
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Reset the cell to default state
    pub fn reset(&mut self) {
        self.content.clear();
        self.fg = Color::Default;
        self.bg = Color::Default;
        self.attrs.reset();
        self.hyperlink_id = 0;
        self.width = 1;
    }

    /// Reset the cell but keep the background color (for erase operations)
    pub fn erase(&mut self, bg: Color) {
        self.content.clear();
        self.fg = Color::Default;
        self.bg = bg;
        self.attrs.reset();
        self.hyperlink_id = 0;
        self.width = 1;
    }
}

/// Determine the display width of a Unicode character
///
/// Returns:
/// - 0 for combining characters and zero-width characters
/// - 2 for wide characters (CJK, etc.)
/// - 1 for everything else
fn unicode_display_width(c: char) -> u8 {
    use unicode_width::UnicodeWidthChar;
    match c.width() {
        Some(0) => 0,
        Some(1) => 1,
        Some(2) => 2,
        Some(w) => w as u8,
        None => 1, // Control characters, treat as 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_default() {
        let cell = Cell::new();
        assert!(cell.is_empty());
        assert_eq!(cell.fg(), Color::Default);
        assert_eq!(cell.bg(), Color::Default);
        assert!(!cell.attrs().has_any());
        assert_eq!(cell.width(), 1);
    }

    #[test]
    fn test_cell_with_char() {
        let cell = Cell::with_char('A');
        assert_eq!(cell.content(), "A");
        assert_eq!(cell.width(), 1);
    }

    #[test]
    fn test_cell_wide_char() {
        // CJK character should be width 2
        let cell = Cell::with_char('中');
        assert_eq!(cell.content(), "中");
        assert_eq!(cell.width(), 2);
    }

    #[test]
    fn test_cell_attributes() {
        let mut attrs = CellAttributes::new();
        assert!(!attrs.has_any());

        attrs.bold = true;
        assert!(attrs.has_any());

        attrs.reset();
        assert!(!attrs.has_any());
    }

    #[test]
    fn test_cell_colors() {
        let mut cell = Cell::new();
        cell.set_fg(Color::Indexed(1));
        cell.set_bg(Color::Rgb(255, 0, 0));

        assert_eq!(cell.fg(), Color::Indexed(1));
        assert_eq!(cell.bg(), Color::Rgb(255, 0, 0));
    }

    #[test]
    fn test_cell_erase() {
        let mut cell = Cell::with_char('X');
        cell.set_fg(Color::Indexed(1));
        cell.attrs_mut().bold = true;

        cell.erase(Color::Indexed(4));

        assert!(cell.is_empty());
        assert_eq!(cell.fg(), Color::Default);
        assert_eq!(cell.bg(), Color::Indexed(4));
        assert!(!cell.attrs().bold);
    }
}
