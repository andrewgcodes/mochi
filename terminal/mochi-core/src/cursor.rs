//! Cursor state and styles for terminal emulation.
//!
//! The cursor tracks:
//! - Current position (row, column)
//! - Visual style (block, underline, bar)
//! - Visibility
//! - Saved state for DECSC/DECRC

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CursorStyle {
    #[default]
    Block,
    Underline,
    Bar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
    pub style: CursorStyle,
    pub visible: bool,
    pub blinking: bool,
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor {
            row: 0,
            col: 0,
            style: CursorStyle::Block,
            visible: true,
            blinking: true,
        }
    }
}

impl Cursor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn move_to(&mut self, row: usize, col: usize) {
        self.row = row;
        self.col = col;
    }

    pub fn move_up(&mut self, n: usize) {
        self.row = self.row.saturating_sub(n);
    }

    pub fn move_down(&mut self, n: usize, max_row: usize) {
        self.row = (self.row + n).min(max_row.saturating_sub(1));
    }

    pub fn move_left(&mut self, n: usize) {
        self.col = self.col.saturating_sub(n);
    }

    pub fn move_right(&mut self, n: usize, max_col: usize) {
        self.col = (self.col + n).min(max_col.saturating_sub(1));
    }

    pub fn move_to_col(&mut self, col: usize, max_col: usize) {
        self.col = col.min(max_col.saturating_sub(1));
    }

    pub fn move_to_row(&mut self, row: usize, max_row: usize) {
        self.row = row.min(max_row.saturating_sub(1));
    }

    pub fn carriage_return(&mut self) {
        self.col = 0;
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SavedCursor {
    pub row: usize,
    pub col: usize,
    pub attrs: crate::cell::Attributes,
    pub fg: crate::color::Color,
    pub bg: crate::color::Color,
    pub origin_mode: bool,
    pub autowrap: bool,
}

impl SavedCursor {
    pub fn from_cursor(
        cursor: &Cursor,
        attrs: &crate::cell::Attributes,
        fg: crate::color::Color,
        bg: crate::color::Color,
        origin_mode: bool,
        autowrap: bool,
    ) -> Self {
        SavedCursor {
            row: cursor.row,
            col: cursor.col,
            attrs: *attrs,
            fg,
            bg,
            origin_mode,
            autowrap,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_movement() {
        let mut cursor = Cursor::new();
        assert_eq!(cursor.row, 0);
        assert_eq!(cursor.col, 0);

        cursor.move_to(5, 10);
        assert_eq!(cursor.row, 5);
        assert_eq!(cursor.col, 10);

        cursor.move_up(3);
        assert_eq!(cursor.row, 2);

        cursor.move_down(10, 24);
        assert_eq!(cursor.row, 12);

        cursor.move_left(5);
        assert_eq!(cursor.col, 5);

        cursor.move_right(100, 80);
        assert_eq!(cursor.col, 79);
    }

    #[test]
    fn test_cursor_bounds() {
        let mut cursor = Cursor::new();

        cursor.move_up(100);
        assert_eq!(cursor.row, 0);

        cursor.move_left(100);
        assert_eq!(cursor.col, 0);
    }

    #[test]
    fn test_carriage_return() {
        let mut cursor = Cursor::new();
        cursor.move_to(5, 40);
        cursor.carriage_return();
        assert_eq!(cursor.row, 5);
        assert_eq!(cursor.col, 0);
    }
}
