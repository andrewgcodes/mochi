//! Terminal cell representation
//!
//! A cell represents a single character position in the terminal grid.
//! Each cell contains:
//! - A character (Unicode scalar value or grapheme cluster)
//! - Foreground and background colors
//! - Text attributes (bold, italic, underline, etc.)
//! - Optional hyperlink ID

use serde::{Deserialize, Serialize};

use crate::color::Color;

/// Flags for cell text attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CellFlags {
    bits: u16,
}

impl CellFlags {
    pub const NONE: u16 = 0;
    pub const BOLD: u16 = 1 << 0;
    pub const FAINT: u16 = 1 << 1;
    pub const ITALIC: u16 = 1 << 2;
    pub const UNDERLINE: u16 = 1 << 3;
    pub const BLINK: u16 = 1 << 4;
    pub const INVERSE: u16 = 1 << 5;
    pub const HIDDEN: u16 = 1 << 6;
    pub const STRIKETHROUGH: u16 = 1 << 7;
    pub const DOUBLE_UNDERLINE: u16 = 1 << 8;
    pub const WIDE_CHAR: u16 = 1 << 9;
    pub const WIDE_CHAR_SPACER: u16 = 1 << 10;

    pub const fn empty() -> Self {
        CellFlags { bits: Self::NONE }
    }

    pub const fn new(bits: u16) -> Self {
        CellFlags { bits }
    }

    pub fn contains(&self, flag: u16) -> bool {
        self.bits & flag != 0
    }

    pub fn set(&mut self, flag: u16, value: bool) {
        if value {
            self.bits |= flag;
        } else {
            self.bits &= !flag;
        }
    }

    pub fn insert(&mut self, flag: u16) {
        self.bits |= flag;
    }

    pub fn remove(&mut self, flag: u16) {
        self.bits &= !flag;
    }

    pub fn bits(&self) -> u16 {
        self.bits
    }

    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }
}

/// A single cell in the terminal grid
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    /// The character stored in this cell
    /// We store a String to support grapheme clusters (combining characters, emoji)
    pub c: String,
    /// Foreground color
    pub fg: Color,
    /// Background color
    pub bg: Color,
    /// Text attributes
    pub flags: CellFlags,
    /// Hyperlink ID (0 = no hyperlink)
    pub hyperlink_id: u32,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            c: String::from(" "),
            fg: Color::Default,
            bg: Color::Default,
            flags: CellFlags::empty(),
            hyperlink_id: 0,
        }
    }
}

impl Cell {
    /// Create a new cell with the given character
    pub fn new(c: char) -> Self {
        Cell {
            c: c.to_string(),
            ..Default::default()
        }
    }

    /// Create a new cell with a grapheme cluster
    pub fn new_grapheme(s: &str) -> Self {
        Cell {
            c: s.to_string(),
            ..Default::default()
        }
    }

    /// Check if this cell is empty (contains only a space with default attributes)
    pub fn is_empty(&self) -> bool {
        (self.c == " " || self.c.is_empty())
            && self.fg == Color::Default
            && self.bg == Color::Default
            && self.flags.is_empty()
            && self.hyperlink_id == 0
    }

    /// Reset the cell to default state
    pub fn reset(&mut self) {
        self.c = String::from(" ");
        self.fg = Color::Default;
        self.bg = Color::Default;
        self.flags = CellFlags::empty();
        self.hyperlink_id = 0;
    }

    /// Get the display width of this cell
    /// Wide characters (CJK, some emoji) have width 2
    pub fn width(&self) -> usize {
        if self.flags.contains(CellFlags::WIDE_CHAR_SPACER) {
            0
        } else if self.flags.contains(CellFlags::WIDE_CHAR) {
            2
        } else {
            use unicode_width::UnicodeWidthStr;
            self.c.width().max(1)
        }
    }

    /// Check if this cell is a wide character
    pub fn is_wide(&self) -> bool {
        self.flags.contains(CellFlags::WIDE_CHAR)
    }

    /// Check if this cell is a spacer for a wide character
    pub fn is_wide_spacer(&self) -> bool {
        self.flags.contains(CellFlags::WIDE_CHAR_SPACER)
    }
}

/// Attributes that can be applied to cells (used during parsing)
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CellAttributes {
    pub fg: Color,
    pub bg: Color,
    pub flags: CellFlags,
    pub hyperlink_id: u32,
}

impl CellAttributes {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply these attributes to a cell
    pub fn apply_to(&self, cell: &mut Cell) {
        cell.fg = self.fg;
        cell.bg = self.bg;
        cell.flags = self.flags;
        cell.hyperlink_id = self.hyperlink_id;
    }

    /// Reset all attributes to default
    pub fn reset(&mut self) {
        self.fg = Color::Default;
        self.bg = Color::Default;
        self.flags = CellFlags::empty();
        self.hyperlink_id = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_default() {
        let cell = Cell::default();
        assert_eq!(cell.c, " ");
        assert_eq!(cell.fg, Color::Default);
        assert_eq!(cell.bg, Color::Default);
        assert!(cell.flags.is_empty());
        assert!(cell.is_empty());
    }

    #[test]
    fn test_cell_new() {
        let cell = Cell::new('A');
        assert_eq!(cell.c, "A");
        assert!(!cell.is_empty());
    }

    #[test]
    fn test_cell_flags() {
        let mut flags = CellFlags::empty();
        assert!(!flags.contains(CellFlags::BOLD));

        flags.insert(CellFlags::BOLD);
        assert!(flags.contains(CellFlags::BOLD));

        flags.insert(CellFlags::ITALIC);
        assert!(flags.contains(CellFlags::BOLD));
        assert!(flags.contains(CellFlags::ITALIC));

        flags.remove(CellFlags::BOLD);
        assert!(!flags.contains(CellFlags::BOLD));
        assert!(flags.contains(CellFlags::ITALIC));
    }

    #[test]
    fn test_cell_reset() {
        let mut cell = Cell::new('X');
        cell.fg = Color::Indexed(1);
        cell.flags.insert(CellFlags::BOLD);

        cell.reset();
        assert!(cell.is_empty());
    }
}
