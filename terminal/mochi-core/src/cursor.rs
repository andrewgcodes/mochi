//! Cursor state management
//!
//! The cursor tracks:
//! - Position (row, column)
//! - Style (block, bar, underline)
//! - Visibility
//! - Saved state for DECSC/DECRC

use serde::{Deserialize, Serialize};

use crate::cell::CellAttributes;

/// Cursor style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorStyle {
    /// Block cursor (default)
    Block,
    /// Underline cursor
    Underline,
    /// Bar/beam cursor
    Bar,
}

impl Default for CursorStyle {
    fn default() -> Self {
        CursorStyle::Block
    }
}

/// Cursor state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    /// Row position (0-indexed from top of visible area)
    pub row: usize,
    /// Column position (0-indexed)
    pub col: usize,
    /// Cursor style
    pub style: CursorStyle,
    /// Whether the cursor is visible
    pub visible: bool,
    /// Whether the cursor is blinking
    pub blinking: bool,
    /// Current cell attributes (applied to new characters)
    pub attrs: CellAttributes,
    /// Whether we're in the "pending wrap" state
    /// This happens when we write to the last column and autowrap is on
    pub pending_wrap: bool,
    /// Origin mode: if true, cursor positions are relative to scroll region
    pub origin_mode: bool,
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor {
            row: 0,
            col: 0,
            style: CursorStyle::Block,
            visible: true,
            blinking: true,
            attrs: CellAttributes::default(),
            pending_wrap: false,
            origin_mode: false,
        }
    }
}

impl Cursor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Move cursor to absolute position, clamping to bounds
    pub fn goto(&mut self, row: usize, col: usize, rows: usize, cols: usize) {
        self.row = row.min(rows.saturating_sub(1));
        self.col = col.min(cols.saturating_sub(1));
        self.pending_wrap = false;
    }

    /// Move cursor up by n rows
    pub fn move_up(&mut self, n: usize) {
        self.row = self.row.saturating_sub(n);
        self.pending_wrap = false;
    }

    /// Move cursor down by n rows, clamping to max_row
    pub fn move_down(&mut self, n: usize, max_row: usize) {
        self.row = (self.row + n).min(max_row);
        self.pending_wrap = false;
    }

    /// Move cursor left by n columns
    pub fn move_left(&mut self, n: usize) {
        self.col = self.col.saturating_sub(n);
        self.pending_wrap = false;
    }

    /// Move cursor right by n columns, clamping to max_col
    pub fn move_right(&mut self, n: usize, max_col: usize) {
        self.col = (self.col + n).min(max_col);
        self.pending_wrap = false;
    }

    /// Move cursor to beginning of current line
    pub fn carriage_return(&mut self) {
        self.col = 0;
        self.pending_wrap = false;
    }

    /// Move cursor to specific column
    pub fn goto_col(&mut self, col: usize, max_col: usize) {
        self.col = col.min(max_col);
        self.pending_wrap = false;
    }

    /// Move cursor to specific row
    pub fn goto_row(&mut self, row: usize, max_row: usize) {
        self.row = row.min(max_row);
        self.pending_wrap = false;
    }
}

/// Saved cursor state for DECSC/DECRC
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SavedCursor {
    pub row: usize,
    pub col: usize,
    pub attrs: CellAttributes,
    pub origin_mode: bool,
    pub pending_wrap: bool,
}

impl From<&Cursor> for SavedCursor {
    fn from(cursor: &Cursor) -> Self {
        SavedCursor {
            row: cursor.row,
            col: cursor.col,
            attrs: cursor.attrs.clone(),
            origin_mode: cursor.origin_mode,
            pending_wrap: cursor.pending_wrap,
        }
    }
}

impl SavedCursor {
    /// Restore saved state to cursor
    pub fn restore_to(&self, cursor: &mut Cursor) {
        cursor.row = self.row;
        cursor.col = self.col;
        cursor.attrs = self.attrs.clone();
        cursor.origin_mode = self.origin_mode;
        cursor.pending_wrap = self.pending_wrap;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_default() {
        let cursor = Cursor::default();
        assert_eq!(cursor.row, 0);
        assert_eq!(cursor.col, 0);
        assert!(cursor.visible);
        assert_eq!(cursor.style, CursorStyle::Block);
    }

    #[test]
    fn test_cursor_goto() {
        let mut cursor = Cursor::new();
        cursor.goto(5, 10, 24, 80);
        assert_eq!(cursor.row, 5);
        assert_eq!(cursor.col, 10);

        // Test clamping
        cursor.goto(100, 200, 24, 80);
        assert_eq!(cursor.row, 23);
        assert_eq!(cursor.col, 79);
    }

    #[test]
    fn test_cursor_movement() {
        let mut cursor = Cursor::new();
        cursor.goto(10, 10, 24, 80);

        cursor.move_up(5);
        assert_eq!(cursor.row, 5);

        cursor.move_down(3, 23);
        assert_eq!(cursor.row, 8);

        cursor.move_left(5);
        assert_eq!(cursor.col, 5);

        cursor.move_right(10, 79);
        assert_eq!(cursor.col, 15);
    }

    #[test]
    fn test_cursor_save_restore() {
        let mut cursor = Cursor::new();
        cursor.goto(5, 10, 24, 80);
        cursor.attrs.flags.insert(crate::cell::CellFlags::BOLD);

        let saved = SavedCursor::from(&cursor);

        cursor.goto(0, 0, 24, 80);
        cursor.attrs.reset();

        saved.restore_to(&mut cursor);
        assert_eq!(cursor.row, 5);
        assert_eq!(cursor.col, 10);
        assert!(cursor.attrs.flags.contains(crate::cell::CellFlags::BOLD));
    }
}
