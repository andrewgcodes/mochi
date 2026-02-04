//! Terminal Cursor
//!
//! Manages cursor position, style, and saved state.

use serde::{Deserialize, Serialize};

use super::{Color, Style};

/// Cursor state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    /// Column position (0-indexed)
    pub col: usize,
    /// Row position (0-indexed)
    pub row: usize,
    /// Cursor style
    pub style: CursorStyle,
    /// Whether cursor is visible
    pub visible: bool,
    /// Current text attributes for new characters
    pub attrs: CursorAttrs,
    /// Pending wrap: cursor is at the end of line and next char should wrap
    pub pending_wrap: bool,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            col: 0,
            row: 0,
            style: CursorStyle::Block,
            visible: true,
            attrs: CursorAttrs::default(),
            pending_wrap: false,
        }
    }
}

impl Cursor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Move cursor to absolute position, clamping to bounds
    pub fn move_to(&mut self, col: usize, row: usize, cols: usize, rows: usize) {
        self.col = col.min(cols.saturating_sub(1));
        self.row = row.min(rows.saturating_sub(1));
        self.pending_wrap = false;
    }

    /// Move cursor relative to current position
    pub fn move_relative(&mut self, dcol: i32, drow: i32, cols: usize, rows: usize) {
        let new_col = (self.col as i32 + dcol).max(0) as usize;
        let new_row = (self.row as i32 + drow).max(0) as usize;
        self.move_to(new_col, new_row, cols, rows);
    }

    /// Move cursor to the beginning of the current line
    pub fn carriage_return(&mut self) {
        self.col = 0;
        self.pending_wrap = false;
    }

    /// Move cursor down one line (does not scroll)
    pub fn line_feed(&mut self, rows: usize) {
        if self.row < rows.saturating_sub(1) {
            self.row += 1;
        }
        self.pending_wrap = false;
    }

    /// Move cursor up one line
    pub fn reverse_index(&mut self) {
        self.row = self.row.saturating_sub(1);
        self.pending_wrap = false;
    }

    /// Save cursor state for later restoration
    pub fn save(&self) -> SavedCursor {
        SavedCursor {
            col: self.col,
            row: self.row,
            attrs: self.attrs.clone(),
            pending_wrap: self.pending_wrap,
        }
    }

    /// Restore cursor state from saved state
    pub fn restore(&mut self, saved: &SavedCursor, cols: usize, rows: usize) {
        self.col = saved.col.min(cols.saturating_sub(1));
        self.row = saved.row.min(rows.saturating_sub(1));
        self.attrs = saved.attrs.clone();
        self.pending_wrap = saved.pending_wrap;
    }
}

/// Cursor visual style
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorStyle {
    /// Filled block cursor
    #[default]
    Block,
    /// Underline cursor
    Underline,
    /// Vertical bar cursor
    Bar,
}

/// Current text attributes that will be applied to new characters
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CursorAttrs {
    pub fg: Color,
    pub bg: Color,
    pub style: Style,
    pub hyperlink_id: u32,
}

impl CursorAttrs {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Saved cursor state (for DECSC/DECRC)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SavedCursor {
    pub col: usize,
    pub row: usize,
    pub attrs: CursorAttrs,
    pub pending_wrap: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_default() {
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

        // Test clamping
        cursor.move_to(100, 50, 80, 24);
        assert_eq!(cursor.col, 79);
        assert_eq!(cursor.row, 23);
    }

    #[test]
    fn test_cursor_move_relative() {
        let mut cursor = Cursor::new();
        cursor.move_to(10, 10, 80, 24);

        cursor.move_relative(5, 2, 80, 24);
        assert_eq!(cursor.col, 15);
        assert_eq!(cursor.row, 12);

        cursor.move_relative(-20, -20, 80, 24);
        assert_eq!(cursor.col, 0);
        assert_eq!(cursor.row, 0);
    }

    #[test]
    fn test_cursor_carriage_return() {
        let mut cursor = Cursor::new();
        cursor.move_to(50, 10, 80, 24);
        cursor.carriage_return();
        assert_eq!(cursor.col, 0);
        assert_eq!(cursor.row, 10);
    }

    #[test]
    fn test_cursor_save_restore() {
        let mut cursor = Cursor::new();
        cursor.move_to(10, 5, 80, 24);
        cursor.attrs.fg = Color::Indexed(1);
        cursor.attrs.style.bold = true;

        let saved = cursor.save();

        cursor.move_to(0, 0, 80, 24);
        cursor.attrs.reset();

        cursor.restore(&saved, 80, 24);
        assert_eq!(cursor.col, 10);
        assert_eq!(cursor.row, 5);
        assert_eq!(cursor.attrs.fg, Color::Indexed(1));
        assert!(cursor.attrs.style.bold);
    }
}
