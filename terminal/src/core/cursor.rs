//! Cursor state management
//!
//! The cursor tracks position, visibility, and style. It also supports
//! save/restore operations (DECSC/DECRC and CSI s/u).

use serde::{Deserialize, Serialize};

use super::{Color, Style};

/// Cursor shape/style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CursorShape {
    /// Block cursor (filled rectangle)
    #[default]
    Block,
    /// Underline cursor
    Underline,
    /// Vertical bar cursor
    Bar,
}

/// Cursor state including position, visibility, and saved state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    /// Column position (0-indexed)
    pub col: usize,
    /// Row position (0-indexed)
    pub row: usize,
    /// Whether the cursor is visible (DECTCEM)
    pub visible: bool,
    /// Cursor shape
    pub shape: CursorShape,
    /// Whether cursor is blinking
    pub blinking: bool,
    /// Current text attributes (applied to new characters)
    pub style: Style,
    /// Current foreground color
    pub fg: Color,
    /// Current background color
    pub bg: Color,
    /// Origin mode (DECOM) - cursor addressing relative to scroll region
    pub origin_mode: bool,
    /// Autowrap mode (DECAWM) - wrap at end of line
    pub autowrap: bool,
    /// Pending wrap - cursor is at the right margin, next char will wrap
    pub pending_wrap: bool,
    /// Insert mode (IRM) - insert characters instead of overwriting
    pub insert_mode: bool,
    /// Current hyperlink ID (for OSC 8)
    pub hyperlink_id: Option<u32>,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            col: 0,
            row: 0,
            visible: true,
            shape: CursorShape::Block,
            blinking: true,
            style: Style::default(),
            fg: Color::Default,
            bg: Color::Default,
            origin_mode: false,
            autowrap: true,
            pending_wrap: false,
            insert_mode: false,
            hyperlink_id: None,
        }
    }
}

/// Saved cursor state for DECSC/DECRC
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedCursor {
    pub col: usize,
    pub row: usize,
    pub style: Style,
    pub fg: Color,
    pub bg: Color,
    pub origin_mode: bool,
    pub autowrap: bool,
}

impl Default for SavedCursor {
    fn default() -> Self {
        Self {
            col: 0,
            row: 0,
            style: Style::default(),
            fg: Color::Default,
            bg: Color::Default,
            origin_mode: false,
            autowrap: true,
        }
    }
}

impl Cursor {
    /// Create a new cursor at the home position
    pub fn new() -> Self {
        Self::default()
    }

    /// Move cursor to absolute position, clamping to bounds
    pub fn move_to(&mut self, col: usize, row: usize, cols: usize, rows: usize) {
        self.col = col.min(cols.saturating_sub(1));
        self.row = row.min(rows.saturating_sub(1));
        self.pending_wrap = false;
    }

    /// Move cursor to home position (0, 0) or top of scroll region if origin mode
    pub fn home(&mut self, scroll_top: usize) {
        self.col = 0;
        self.row = if self.origin_mode { scroll_top } else { 0 };
        self.pending_wrap = false;
    }

    /// Move cursor up by n rows, stopping at top margin
    pub fn move_up(&mut self, n: usize, top_margin: usize) {
        let min_row = if self.origin_mode { top_margin } else { 0 };
        self.row = self.row.saturating_sub(n).max(min_row);
        self.pending_wrap = false;
    }

    /// Move cursor down by n rows, stopping at bottom margin
    pub fn move_down(&mut self, n: usize, bottom_margin: usize, rows: usize) {
        let max_row = if self.origin_mode {
            bottom_margin
        } else {
            rows.saturating_sub(1)
        };
        self.row = (self.row + n).min(max_row);
        self.pending_wrap = false;
    }

    /// Move cursor left by n columns, stopping at column 0
    pub fn move_left(&mut self, n: usize) {
        self.col = self.col.saturating_sub(n);
        self.pending_wrap = false;
    }

    /// Move cursor right by n columns, stopping at right margin
    pub fn move_right(&mut self, n: usize, cols: usize) {
        self.col = (self.col + n).min(cols.saturating_sub(1));
        self.pending_wrap = false;
    }

    /// Move cursor to column (0-indexed)
    pub fn set_col(&mut self, col: usize, cols: usize) {
        self.col = col.min(cols.saturating_sub(1));
        self.pending_wrap = false;
    }

    /// Move cursor to row (0-indexed), respecting origin mode
    pub fn set_row(&mut self, row: usize, rows: usize, scroll_top: usize, scroll_bottom: usize) {
        if self.origin_mode {
            // In origin mode, row is relative to scroll region
            self.row = (scroll_top + row).min(scroll_bottom);
        } else {
            self.row = row.min(rows.saturating_sub(1));
        }
        self.pending_wrap = false;
    }

    /// Carriage return - move to column 0
    pub fn carriage_return(&mut self) {
        self.col = 0;
        self.pending_wrap = false;
    }

    /// Save cursor state
    pub fn save(&self) -> SavedCursor {
        SavedCursor {
            col: self.col,
            row: self.row,
            style: self.style,
            fg: self.fg,
            bg: self.bg,
            origin_mode: self.origin_mode,
            autowrap: self.autowrap,
        }
    }

    /// Restore cursor state
    pub fn restore(&mut self, saved: &SavedCursor, cols: usize, rows: usize) {
        self.col = saved.col.min(cols.saturating_sub(1));
        self.row = saved.row.min(rows.saturating_sub(1));
        self.style = saved.style;
        self.fg = saved.fg;
        self.bg = saved.bg;
        self.origin_mode = saved.origin_mode;
        self.autowrap = saved.autowrap;
        self.pending_wrap = false;
    }

    /// Reset cursor to default state
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Reset only the text attributes (SGR 0)
    pub fn reset_attributes(&mut self) {
        self.style = Style::default();
        self.fg = Color::Default;
        self.bg = Color::Default;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_default() {
        let cursor = Cursor::default();
        assert_eq!(cursor.col, 0);
        assert_eq!(cursor.row, 0);
        assert!(cursor.visible);
        assert!(cursor.autowrap);
        assert!(!cursor.origin_mode);
    }

    #[test]
    fn test_cursor_move_to() {
        let mut cursor = Cursor::new();
        cursor.move_to(5, 10, 80, 24);
        assert_eq!(cursor.col, 5);
        assert_eq!(cursor.row, 10);

        // Test clamping
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

        cursor.move_left(4);
        assert_eq!(cursor.col, 6);

        cursor.move_right(10, 80);
        assert_eq!(cursor.col, 16);
    }

    #[test]
    fn test_cursor_boundaries() {
        let mut cursor = Cursor::new();

        // Can't go negative
        cursor.move_up(100, 0);
        assert_eq!(cursor.row, 0);

        cursor.move_left(100);
        assert_eq!(cursor.col, 0);

        // Can't exceed bounds
        cursor.move_down(100, 23, 24);
        assert_eq!(cursor.row, 23);

        cursor.move_right(100, 80);
        assert_eq!(cursor.col, 79);
    }

    #[test]
    fn test_cursor_save_restore() {
        let mut cursor = Cursor::new();
        cursor.move_to(15, 8, 80, 24);
        cursor.style.bold = true;
        cursor.fg = Color::RED;

        let saved = cursor.save();

        cursor.move_to(0, 0, 80, 24);
        cursor.style.bold = false;
        cursor.fg = Color::Default;

        cursor.restore(&saved, 80, 24);
        assert_eq!(cursor.col, 15);
        assert_eq!(cursor.row, 8);
        assert!(cursor.style.bold);
        assert_eq!(cursor.fg, Color::RED);
    }

    #[test]
    fn test_cursor_origin_mode() {
        let mut cursor = Cursor::new();
        cursor.origin_mode = true;

        // In origin mode, home goes to scroll region top
        cursor.home(5);
        assert_eq!(cursor.row, 5);
        assert_eq!(cursor.col, 0);

        // Movement respects scroll region
        cursor.move_up(10, 5);
        assert_eq!(cursor.row, 5); // Stops at top margin
    }

    #[test]
    fn test_carriage_return() {
        let mut cursor = Cursor::new();
        cursor.move_to(50, 10, 80, 24);
        cursor.pending_wrap = true;

        cursor.carriage_return();
        assert_eq!(cursor.col, 0);
        assert_eq!(cursor.row, 10); // Row unchanged
        assert!(!cursor.pending_wrap);
    }
}
