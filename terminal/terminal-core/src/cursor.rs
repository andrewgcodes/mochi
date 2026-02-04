//! Cursor state management
//!
//! Handles cursor position, style, visibility, and saved state.

use serde::{Deserialize, Serialize};

use crate::cell::CellAttributes;

/// Cursor visual style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CursorStyle {
    /// Block cursor (filled rectangle)
    #[default]
    Block,
    /// Underline cursor
    Underline,
    /// Vertical bar cursor
    Bar,
}

/// Cursor state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    /// Column position (0-indexed)
    pub col: usize,
    /// Row position (0-indexed)
    pub row: usize,
    /// Visual style
    pub style: CursorStyle,
    /// Whether cursor is visible
    pub visible: bool,
    /// Whether cursor should blink
    pub blinking: bool,
    /// Current cell attributes (used for new characters)
    pub attrs: CellAttributes,
    /// Origin mode: if true, cursor positions are relative to scroll region
    pub origin_mode: bool,
    /// Pending wrap: cursor is at the right margin and next char should wrap
    pub pending_wrap: bool,
    /// Current hyperlink ID (0 means no hyperlink)
    pub hyperlink_id: u32,
}

impl Cursor {
    /// Create a new cursor at position (0, 0)
    pub fn new() -> Self {
        Self {
            col: 0,
            row: 0,
            style: CursorStyle::Block,
            visible: true,
            blinking: true,
            attrs: CellAttributes::default(),
            origin_mode: false,
            pending_wrap: false,
            hyperlink_id: 0,
        }
    }

    /// Move cursor to absolute position, clamping to bounds
    pub fn move_to(&mut self, col: usize, row: usize, max_col: usize, max_row: usize) {
        self.col = col.min(max_col.saturating_sub(1));
        self.row = row.min(max_row.saturating_sub(1));
        self.pending_wrap = false;
    }

    /// Move cursor up by n rows
    pub fn move_up(&mut self, n: usize, top_margin: usize) {
        let min_row = if self.origin_mode { top_margin } else { 0 };
        self.row = self.row.saturating_sub(n).max(min_row);
        self.pending_wrap = false;
    }

    /// Move cursor down by n rows
    pub fn move_down(&mut self, n: usize, bottom_margin: usize, max_row: usize) {
        let max = if self.origin_mode {
            bottom_margin
        } else {
            max_row.saturating_sub(1)
        };
        self.row = (self.row + n).min(max);
        self.pending_wrap = false;
    }

    /// Move cursor left by n columns
    pub fn move_left(&mut self, n: usize) {
        self.col = self.col.saturating_sub(n);
        self.pending_wrap = false;
    }

    /// Move cursor right by n columns
    pub fn move_right(&mut self, n: usize, max_col: usize) {
        self.col = (self.col + n).min(max_col.saturating_sub(1));
        self.pending_wrap = false;
    }

    /// Move cursor to beginning of line
    pub fn carriage_return(&mut self) {
        self.col = 0;
        self.pending_wrap = false;
    }

    /// Move cursor to specific column (0-indexed)
    pub fn set_col(&mut self, col: usize, max_col: usize) {
        self.col = col.min(max_col.saturating_sub(1));
        self.pending_wrap = false;
    }

    /// Move cursor to specific row (0-indexed)
    pub fn set_row(&mut self, row: usize, max_row: usize) {
        self.row = row.min(max_row.saturating_sub(1));
        self.pending_wrap = false;
    }

    /// Reset cursor to default state
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new()
    }
}

/// Saved cursor state for DECSC/DECRC
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SavedCursor {
    pub col: usize,
    pub row: usize,
    pub attrs: CellAttributes,
    pub origin_mode: bool,
    pub pending_wrap: bool,
    pub hyperlink_id: u32,
}

impl SavedCursor {
    /// Save current cursor state
    pub fn save(cursor: &Cursor) -> Self {
        Self {
            col: cursor.col,
            row: cursor.row,
            attrs: cursor.attrs,
            origin_mode: cursor.origin_mode,
            pending_wrap: cursor.pending_wrap,
            hyperlink_id: cursor.hyperlink_id,
        }
    }

    /// Restore cursor state
    pub fn restore(&self, cursor: &mut Cursor) {
        cursor.col = self.col;
        cursor.row = self.row;
        cursor.attrs = self.attrs;
        cursor.origin_mode = self.origin_mode;
        cursor.pending_wrap = self.pending_wrap;
        cursor.hyperlink_id = self.hyperlink_id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_new() {
        let cursor = Cursor::new();
        assert_eq!(cursor.col, 0);
        assert_eq!(cursor.row, 0);
        assert!(cursor.visible);
    }

    #[test]
    fn test_cursor_move_to() {
        let mut cursor = Cursor::new();
        cursor.move_to(10, 5, 80, 24);
        assert_eq!(cursor.col, 10);
        assert_eq!(cursor.row, 5);
    }

    #[test]
    fn test_cursor_move_to_clamp() {
        let mut cursor = Cursor::new();
        cursor.move_to(100, 50, 80, 24);
        assert_eq!(cursor.col, 79);
        assert_eq!(cursor.row, 23);
    }

    #[test]
    fn test_cursor_movement() {
        let mut cursor = Cursor::new();
        cursor.move_to(10, 10, 80, 24);

        cursor.move_up(3, 0);
        assert_eq!(cursor.row, 7);

        cursor.move_down(5, 23, 24);
        assert_eq!(cursor.row, 12);

        cursor.move_left(5);
        assert_eq!(cursor.col, 5);

        cursor.move_right(10, 80);
        assert_eq!(cursor.col, 15);
    }

    #[test]
    fn test_cursor_carriage_return() {
        let mut cursor = Cursor::new();
        cursor.col = 50;
        cursor.carriage_return();
        assert_eq!(cursor.col, 0);
    }

    #[test]
    fn test_saved_cursor() {
        let mut cursor = Cursor::new();
        cursor.col = 10;
        cursor.row = 5;
        cursor.attrs.bold = true;

        let saved = SavedCursor::save(&cursor);

        cursor.col = 0;
        cursor.row = 0;
        cursor.attrs.bold = false;

        saved.restore(&mut cursor);

        assert_eq!(cursor.col, 10);
        assert_eq!(cursor.row, 5);
        assert!(cursor.attrs.bold);
    }
}
