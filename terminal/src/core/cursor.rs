//! Cursor state and styles
//!
//! Manages the terminal cursor position, visibility, and appearance.

use serde::{Deserialize, Serialize};

/// Cursor visual style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CursorStyle {
    /// Solid block cursor (default)
    #[default]
    Block,
    /// Underline cursor
    Underline,
    /// Vertical bar cursor
    Bar,
}

/// Terminal cursor state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    /// Row position (0-indexed)
    row: usize,
    /// Column position (0-indexed)
    col: usize,
    /// Whether the cursor is visible
    visible: bool,
    /// Cursor visual style
    style: CursorStyle,
    /// Whether the cursor should blink
    blinking: bool,
    /// Pending wrap flag - cursor is at the right margin and next character
    /// should wrap to the next line
    pending_wrap: bool,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            row: 0,
            col: 0,
            visible: true,
            style: CursorStyle::Block,
            blinking: true,
            pending_wrap: false,
        }
    }
}

impl Cursor {
    /// Create a new cursor at position (0, 0)
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the current row (0-indexed)
    pub fn row(&self) -> usize {
        self.row
    }

    /// Get the current column (0-indexed)
    pub fn col(&self) -> usize {
        self.col
    }

    /// Set the row position
    pub fn set_row(&mut self, row: usize) {
        self.row = row;
        self.pending_wrap = false;
    }

    /// Set the column position
    pub fn set_col(&mut self, col: usize) {
        self.col = col;
        self.pending_wrap = false;
    }

    /// Set both row and column position
    pub fn set_position(&mut self, row: usize, col: usize) {
        self.row = row;
        self.col = col;
        self.pending_wrap = false;
    }

    /// Move the cursor up by the given amount, stopping at row 0
    pub fn move_up(&mut self, amount: usize) {
        self.row = self.row.saturating_sub(amount);
        self.pending_wrap = false;
    }

    /// Move the cursor down by the given amount
    pub fn move_down(&mut self, amount: usize, max_row: usize) {
        self.row = (self.row + amount).min(max_row);
        self.pending_wrap = false;
    }

    /// Move the cursor left by the given amount, stopping at column 0
    pub fn move_left(&mut self, amount: usize) {
        self.col = self.col.saturating_sub(amount);
        self.pending_wrap = false;
    }

    /// Move the cursor right by the given amount
    pub fn move_right(&mut self, amount: usize, max_col: usize) {
        self.col = (self.col + amount).min(max_col);
        self.pending_wrap = false;
    }

    /// Check if the cursor is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Set cursor visibility
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Get the cursor style
    pub fn style(&self) -> CursorStyle {
        self.style
    }

    /// Set the cursor style
    pub fn set_style(&mut self, style: CursorStyle) {
        self.style = style;
    }

    /// Check if the cursor is blinking
    pub fn is_blinking(&self) -> bool {
        self.blinking
    }

    /// Set cursor blinking
    pub fn set_blinking(&mut self, blinking: bool) {
        self.blinking = blinking;
    }

    /// Check if there's a pending wrap
    pub fn pending_wrap(&self) -> bool {
        self.pending_wrap
    }

    /// Set the pending wrap flag
    pub fn set_pending_wrap(&mut self, pending: bool) {
        self.pending_wrap = pending;
    }

    /// Reset the cursor to default state
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Saved cursor state (for DECSC/DECRC)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedCursor {
    /// Saved row position
    pub row: usize,
    /// Saved column position
    pub col: usize,
    /// Saved pending wrap flag
    pub pending_wrap: bool,
    /// Saved origin mode flag
    pub origin_mode: bool,
    // Note: Full DECSC also saves:
    // - Character attributes (SGR)
    // - Character set designations
    // - Selective erase attribute
    // These are stored separately in the screen state
}

impl SavedCursor {
    /// Create a saved cursor from the current cursor state
    pub fn from_cursor(cursor: &Cursor, origin_mode: bool) -> Self {
        Self {
            row: cursor.row,
            col: cursor.col,
            pending_wrap: cursor.pending_wrap,
            origin_mode,
        }
    }

    /// Restore cursor position from saved state
    pub fn restore_to(&self, cursor: &mut Cursor) {
        cursor.row = self.row;
        cursor.col = self.col;
        cursor.pending_wrap = self.pending_wrap;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_default() {
        let cursor = Cursor::new();
        assert_eq!(cursor.row(), 0);
        assert_eq!(cursor.col(), 0);
        assert!(cursor.is_visible());
        assert_eq!(cursor.style(), CursorStyle::Block);
        assert!(!cursor.pending_wrap());
    }

    #[test]
    fn test_cursor_movement() {
        let mut cursor = Cursor::new();

        cursor.move_down(5, 23);
        assert_eq!(cursor.row(), 5);

        cursor.move_right(10, 79);
        assert_eq!(cursor.col(), 10);

        cursor.move_up(3);
        assert_eq!(cursor.row(), 2);

        cursor.move_left(5);
        assert_eq!(cursor.col(), 5);
    }

    #[test]
    fn test_cursor_bounds() {
        let mut cursor = Cursor::new();

        // Can't go below 0
        cursor.move_up(100);
        assert_eq!(cursor.row(), 0);

        cursor.move_left(100);
        assert_eq!(cursor.col(), 0);

        // Respects max bounds
        cursor.move_down(100, 23);
        assert_eq!(cursor.row(), 23);

        cursor.move_right(100, 79);
        assert_eq!(cursor.col(), 79);
    }

    #[test]
    fn test_cursor_pending_wrap() {
        let mut cursor = Cursor::new();
        cursor.set_pending_wrap(true);
        assert!(cursor.pending_wrap());

        // Movement clears pending wrap
        cursor.move_right(1, 79);
        assert!(!cursor.pending_wrap());
    }

    #[test]
    fn test_saved_cursor() {
        let mut cursor = Cursor::new();
        cursor.set_position(10, 20);
        cursor.set_pending_wrap(true);

        let saved = SavedCursor::from_cursor(&cursor, true);

        cursor.set_position(0, 0);
        cursor.set_pending_wrap(false);

        saved.restore_to(&mut cursor);

        assert_eq!(cursor.row(), 10);
        assert_eq!(cursor.col(), 20);
        assert!(cursor.pending_wrap());
    }
}
