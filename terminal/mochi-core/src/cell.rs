//! Cell representation for terminal grid.
//!
//! Each cell contains:
//! - A character (Unicode scalar value or grapheme cluster)
//! - Foreground and background colors
//! - Text attributes (bold, italic, underline, etc.)
//! - Optional hyperlink ID

use crate::color::Color;
use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthChar;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Attributes {
    pub bold: bool,
    pub faint: bool,
    pub italic: bool,
    pub underline: bool,
    pub blink: bool,
    pub inverse: bool,
    pub hidden: bool,
    pub strikethrough: bool,
}

impl Attributes {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub character: char,
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attributes,
    pub hyperlink_id: Option<u32>,
    wide: CellWidth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CellWidth {
    #[default]
    Normal,
    Wide,
    WideContinuation,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            character: ' ',
            fg: Color::Default,
            bg: Color::Default,
            attrs: Attributes::default(),
            hyperlink_id: None,
            wide: CellWidth::Normal,
        }
    }
}

impl Cell {
    pub fn new(c: char) -> Self {
        let wide = match c.width() {
            Some(2) => CellWidth::Wide,
            _ => CellWidth::Normal,
        };
        Cell {
            character: c,
            wide,
            ..Default::default()
        }
    }

    pub fn with_attrs(c: char, fg: Color, bg: Color, attrs: Attributes) -> Self {
        let wide = match c.width() {
            Some(2) => CellWidth::Wide,
            _ => CellWidth::Normal,
        };
        Cell {
            character: c,
            fg,
            bg,
            attrs,
            hyperlink_id: None,
            wide,
        }
    }

    pub fn is_wide(&self) -> bool {
        matches!(self.wide, CellWidth::Wide)
    }

    pub fn is_wide_continuation(&self) -> bool {
        matches!(self.wide, CellWidth::WideContinuation)
    }

    pub fn set_wide(&mut self) {
        self.wide = CellWidth::Wide;
    }

    pub fn set_wide_continuation(&mut self) {
        self.wide = CellWidth::WideContinuation;
    }

    pub fn clear(&mut self) {
        self.character = ' ';
        self.fg = Color::Default;
        self.bg = Color::Default;
        self.attrs = Attributes::default();
        self.hyperlink_id = None;
        self.wide = CellWidth::Normal;
    }

    pub fn clear_with_bg(&mut self, bg: Color) {
        self.clear();
        self.bg = bg;
    }

    pub fn width(&self) -> usize {
        match self.wide {
            CellWidth::Normal => 1,
            CellWidth::Wide => 2,
            CellWidth::WideContinuation => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cell() {
        let cell = Cell::default();
        assert_eq!(cell.character, ' ');
        assert_eq!(cell.fg, Color::Default);
        assert_eq!(cell.bg, Color::Default);
        assert!(!cell.attrs.bold);
    }

    #[test]
    fn test_wide_character() {
        let cell = Cell::new('中');
        assert!(cell.is_wide());
        assert_eq!(cell.width(), 2);
    }

    #[test]
    fn test_normal_character() {
        let cell = Cell::new('a');
        assert!(!cell.is_wide());
        assert_eq!(cell.width(), 1);
    }

    #[test]
    fn test_clear_cell() {
        let mut cell = Cell::with_attrs('x', Color::RED, Color::BLUE, Attributes { bold: true, ..Default::default() });
        cell.clear();
        assert_eq!(cell.character, ' ');
        assert_eq!(cell.fg, Color::Default);
        assert_eq!(cell.bg, Color::Default);
        assert!(!cell.attrs.bold);
    }
}
