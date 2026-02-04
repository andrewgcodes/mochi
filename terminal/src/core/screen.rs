//! Terminal Screen
//!
//! The main screen model that combines the grid, cursor, scrollback,
//! and terminal modes into a complete terminal state.

use serde::{Deserialize, Serialize};

use super::cursor::{Cursor, SavedCursor};
use super::grid::Grid;
use super::scrollback::Scrollback;
use super::{Modes, TabStops};

/// The complete terminal screen state
#[derive(Debug, Clone)]
pub struct Screen {
    /// Primary screen grid
    primary: Grid,
    /// Alternate screen grid (for full-screen apps)
    alternate: Grid,
    /// Whether we're currently on the alternate screen
    using_alternate: bool,
    /// Scrollback buffer (only for primary screen)
    scrollback: Scrollback,
    /// Cursor state
    pub cursor: Cursor,
    /// Saved cursor for primary screen
    saved_cursor_primary: SavedCursor,
    /// Saved cursor for alternate screen
    saved_cursor_alternate: SavedCursor,
    /// Terminal modes
    pub modes: Modes,
    /// Tab stops
    pub tabs: TabStops,
    /// Scroll region top (0-indexed, inclusive)
    scroll_top: usize,
    /// Scroll region bottom (0-indexed, inclusive)
    scroll_bottom: usize,
    /// Number of columns
    cols: usize,
    /// Number of rows
    rows: usize,
    /// Window title
    pub title: String,
    /// Current hyperlink (if any)
    pub current_hyperlink: Option<super::Hyperlink>,
    /// Hyperlink registry
    hyperlinks: Vec<super::Hyperlink>,
    /// Next hyperlink ID
    next_hyperlink_id: u32,
}

impl Screen {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            primary: Grid::new(cols, rows),
            alternate: Grid::new(cols, rows),
            using_alternate: false,
            scrollback: Scrollback::new(10000),
            cursor: Cursor::new(),
            saved_cursor_primary: SavedCursor::default(),
            saved_cursor_alternate: SavedCursor::default(),
            modes: Modes::new(),
            tabs: TabStops::new(cols),
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            cols,
            rows,
            title: String::new(),
            current_hyperlink: None,
            hyperlinks: Vec::new(),
            next_hyperlink_id: 1,
        }
    }

    /// Get the current grid (primary or alternate)
    pub fn grid(&self) -> &Grid {
        if self.using_alternate {
            &self.alternate
        } else {
            &self.primary
        }
    }

    /// Get the current grid mutably
    pub fn grid_mut(&mut self) -> &mut Grid {
        if self.using_alternate {
            &mut self.alternate
        } else {
            &mut self.primary
        }
    }

    /// Get the scrollback buffer
    pub fn scrollback(&self) -> &Scrollback {
        &self.scrollback
    }

    /// Get screen dimensions
    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Get scroll region
    pub fn scroll_region(&self) -> (usize, usize) {
        (self.scroll_top, self.scroll_bottom)
    }

    /// Set scroll region
    pub fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        let top = top.min(self.rows.saturating_sub(1));
        let bottom = bottom.min(self.rows.saturating_sub(1));
        if top < bottom {
            self.scroll_top = top;
            self.scroll_bottom = bottom;
        }
    }

    /// Reset scroll region to full screen
    pub fn reset_scroll_region(&mut self) {
        self.scroll_top = 0;
        self.scroll_bottom = self.rows.saturating_sub(1);
    }

    /// Check if cursor is in scroll region
    pub fn cursor_in_scroll_region(&self) -> bool {
        self.cursor.row >= self.scroll_top && self.cursor.row <= self.scroll_bottom
    }

    /// Resize the screen
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.primary.resize(cols, rows);
        self.alternate.resize(cols, rows);
        self.tabs.resize(cols);

        // Adjust cursor position
        self.cursor.col = self.cursor.col.min(cols.saturating_sub(1));
        self.cursor.row = self.cursor.row.min(rows.saturating_sub(1));

        // Reset scroll region
        self.scroll_top = 0;
        self.scroll_bottom = rows.saturating_sub(1);

        self.cols = cols;
        self.rows = rows;
    }

    /// Switch to alternate screen
    pub fn enter_alternate_screen(&mut self) {
        if !self.using_alternate {
            self.saved_cursor_primary = self.cursor.save();
            self.using_alternate = true;
            self.alternate.clear();
            self.cursor = Cursor::new();
        }
    }

    /// Switch back to primary screen
    pub fn exit_alternate_screen(&mut self) {
        if self.using_alternate {
            self.saved_cursor_alternate = self.cursor.save();
            self.using_alternate = false;
            self.cursor
                .restore(&self.saved_cursor_primary, self.cols, self.rows);
        }
    }

    /// Save cursor state (DECSC)
    pub fn save_cursor(&mut self) {
        if self.using_alternate {
            self.saved_cursor_alternate = self.cursor.save();
        } else {
            self.saved_cursor_primary = self.cursor.save();
        }
    }

    /// Restore cursor state (DECRC)
    pub fn restore_cursor(&mut self) {
        let saved = if self.using_alternate {
            &self.saved_cursor_alternate
        } else {
            &self.saved_cursor_primary
        };
        self.cursor.restore(saved, self.cols, self.rows);
    }

    /// Print a character at the current cursor position
    pub fn print(&mut self, c: char) {
        use unicode_width::UnicodeWidthChar;

        // Handle pending wrap
        if self.cursor.pending_wrap && self.modes.auto_wrap {
            self.cursor.col = 0;
            self.linefeed();
            let cursor_row = self.cursor.row;
            if let Some(row) = self.grid_mut().row_mut(cursor_row) {
                row.wrapped = true;
            }
        }
        self.cursor.pending_wrap = false;

        let char_width = c.width().unwrap_or(0);
        if char_width == 0 {
            // Combining character - append to previous cell
            if self.cursor.col > 0 {
                let col = self.cursor.col - 1;
                let row = self.cursor.row;
                if let Some(cell) = self.grid_mut().cell_mut(col, row) {
                    cell.content.push(c);
                }
            }
            return;
        }

        // Check if we need to handle wide characters
        if char_width == 2 && self.cursor.col >= self.cols.saturating_sub(1) {
            // Wide char at last column - wrap first
            if self.modes.auto_wrap {
                self.cursor.col = 0;
                self.linefeed();
                let cursor_row = self.cursor.row;
                if let Some(row) = self.grid_mut().row_mut(cursor_row) {
                    row.wrapped = true;
                }
            }
        }

        // Insert mode: shift characters right
        if self.modes.insert_mode {
            let cursor_col = self.cursor.col;
            let cursor_row = self.cursor.row;
            let bg = self.cursor.attrs.bg;
            self.grid_mut()
                .insert_chars(cursor_col, cursor_row, char_width, bg);
        }

        // Write the character
        let cursor_col = self.cursor.col;
        let cursor_row = self.cursor.row;
        let fg = self.cursor.attrs.fg;
        let bg = self.cursor.attrs.bg;
        let style = self.cursor.attrs.style;
        let hyperlink_id = self.cursor.attrs.hyperlink_id;
        if let Some(cell) = self.grid_mut().cell_mut(cursor_col, cursor_row) {
            cell.content = c.to_string();
            cell.fg = fg;
            cell.bg = bg;
            cell.style = style;
            cell.hyperlink_id = hyperlink_id;
        }

        // For wide characters, mark the next cell as a continuation
        if char_width == 2 && self.cursor.col + 1 < self.cols {
            let next_col = self.cursor.col + 1;
            let cursor_row = self.cursor.row;
            let fg = self.cursor.attrs.fg;
            let bg = self.cursor.attrs.bg;
            let mut style = self.cursor.attrs.style;
            style.wide_char_continuation = true;
            if let Some(cell) = self.grid_mut().cell_mut(next_col, cursor_row) {
                cell.content.clear();
                cell.fg = fg;
                cell.bg = bg;
                cell.style = style;
            }
        }

        // Advance cursor
        self.cursor.col += char_width;
        if self.cursor.col >= self.cols {
            if self.modes.auto_wrap {
                self.cursor.col = self.cols - 1;
                self.cursor.pending_wrap = true;
            } else {
                self.cursor.col = self.cols - 1;
            }
        }
    }

    /// Handle carriage return
    pub fn carriage_return(&mut self) {
        self.cursor.carriage_return();
    }

    /// Handle line feed (and vertical tab, form feed)
    pub fn linefeed(&mut self) {
        self.cursor.pending_wrap = false;

        if self.cursor.row == self.scroll_bottom {
            // At bottom of scroll region - scroll up
            let bg = self.cursor.attrs.bg;
            let scroll_top = self.scroll_top;
            let scroll_bottom = self.scroll_bottom;
            let scrolled = self.grid_mut().scroll_up(1, scroll_top, scroll_bottom, bg);
            if !self.using_alternate {
                self.scrollback.push(scrolled);
            }
        } else if self.cursor.row < self.rows - 1 {
            self.cursor.row += 1;
        }

        // In newline mode, also do carriage return
        if self.modes.linefeed_mode {
            self.cursor.col = 0;
        }
    }

    /// Handle reverse index (move up, scroll if at top)
    pub fn reverse_index(&mut self) {
        self.cursor.pending_wrap = false;

        if self.cursor.row == self.scroll_top {
            // At top of scroll region - scroll down
            let bg = self.cursor.attrs.bg;
            let scroll_top = self.scroll_top;
            let scroll_bottom = self.scroll_bottom;
            self.grid_mut()
                .scroll_down(1, scroll_top, scroll_bottom, bg);
        } else if self.cursor.row > 0 {
            self.cursor.row -= 1;
        }
    }

    /// Handle backspace
    pub fn backspace(&mut self) {
        self.cursor.pending_wrap = false;
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        }
    }

    /// Handle horizontal tab
    pub fn tab(&mut self) {
        self.cursor.pending_wrap = false;
        self.cursor.col = self.tabs.next_stop(self.cursor.col);
    }

    /// Handle bell
    pub fn bell(&mut self) {
        // Visual bell or audio bell would be handled by the frontend
        // This is just a notification that a bell occurred
    }

    /// Move cursor to position (1-indexed input, converted to 0-indexed)
    pub fn move_cursor_to(&mut self, row: usize, col: usize) {
        self.cursor.pending_wrap = false;
        let row = row.saturating_sub(1);
        let col = col.saturating_sub(1);

        let (effective_row, max_row) = if self.modes.origin_mode {
            (self.scroll_top + row, self.scroll_bottom)
        } else {
            (row, self.rows.saturating_sub(1))
        };

        self.cursor.row = effective_row.min(max_row);
        self.cursor.col = col.min(self.cols.saturating_sub(1));
    }

    /// Move cursor up
    pub fn move_cursor_up(&mut self, n: usize) {
        self.cursor.pending_wrap = false;
        let min_row = if self.modes.origin_mode {
            self.scroll_top
        } else {
            0
        };
        self.cursor.row = self.cursor.row.saturating_sub(n).max(min_row);
    }

    /// Move cursor down
    pub fn move_cursor_down(&mut self, n: usize) {
        self.cursor.pending_wrap = false;
        let max_row = if self.modes.origin_mode {
            self.scroll_bottom
        } else {
            self.rows.saturating_sub(1)
        };
        self.cursor.row = (self.cursor.row + n).min(max_row);
    }

    /// Move cursor forward (right)
    pub fn move_cursor_forward(&mut self, n: usize) {
        self.cursor.pending_wrap = false;
        self.cursor.col = (self.cursor.col + n).min(self.cols.saturating_sub(1));
    }

    /// Move cursor backward (left)
    pub fn move_cursor_backward(&mut self, n: usize) {
        self.cursor.pending_wrap = false;
        self.cursor.col = self.cursor.col.saturating_sub(n);
    }

    /// Move cursor to column (1-indexed)
    pub fn move_cursor_to_column(&mut self, col: usize) {
        self.cursor.pending_wrap = false;
        self.cursor.col = col.saturating_sub(1).min(self.cols.saturating_sub(1));
    }

    /// Move cursor to row (1-indexed)
    pub fn move_cursor_to_row(&mut self, row: usize) {
        self.cursor.pending_wrap = false;
        let row = row.saturating_sub(1);
        let (effective_row, max_row) = if self.modes.origin_mode {
            (self.scroll_top + row, self.scroll_bottom)
        } else {
            (row, self.rows.saturating_sub(1))
        };
        self.cursor.row = effective_row.min(max_row);
    }

    /// Erase in display
    pub fn erase_in_display(&mut self, mode: EraseMode) {
        let bg = self.cursor.attrs.bg;
        let cursor_row = self.cursor.row;
        let cursor_col = self.cursor.col;
        let cols = self.cols;
        let rows = self.rows;
        match mode {
            EraseMode::ToEnd => {
                // Erase from cursor to end of screen
                if let Some(row) = self.grid_mut().row_mut(cursor_row) {
                    row.erase_range(cursor_col, cols - 1, bg);
                }
                for r in (cursor_row + 1)..rows {
                    if let Some(row) = self.grid_mut().row_mut(r) {
                        row.erase(bg);
                    }
                }
            }
            EraseMode::ToBeginning => {
                // Erase from beginning to cursor
                for r in 0..cursor_row {
                    if let Some(row) = self.grid_mut().row_mut(r) {
                        row.erase(bg);
                    }
                }
                if let Some(row) = self.grid_mut().row_mut(cursor_row) {
                    row.erase_range(0, cursor_col, bg);
                }
            }
            EraseMode::All => {
                // Erase entire screen
                self.grid_mut().erase(bg);
            }
            EraseMode::Scrollback => {
                // Erase scrollback buffer
                self.scrollback.clear();
            }
        }
    }

    /// Erase in line
    pub fn erase_in_line(&mut self, mode: EraseMode) {
        let bg = self.cursor.attrs.bg;
        let cursor_row = self.cursor.row;
        let cursor_col = self.cursor.col;
        let cols = self.cols;
        if let Some(row) = self.grid_mut().row_mut(cursor_row) {
            match mode {
                EraseMode::ToEnd => {
                    row.erase_range(cursor_col, cols - 1, bg);
                }
                EraseMode::ToBeginning => {
                    row.erase_range(0, cursor_col, bg);
                }
                EraseMode::All | EraseMode::Scrollback => {
                    row.erase(bg);
                }
            }
        }
    }

    /// Erase characters at cursor position
    pub fn erase_chars(&mut self, n: usize) {
        let bg = self.cursor.attrs.bg;
        let cursor_col = self.cursor.col;
        let cursor_row = self.cursor.row;
        self.grid_mut().erase_chars(cursor_col, cursor_row, n, bg);
    }

    /// Insert blank lines at cursor position
    pub fn insert_lines(&mut self, n: usize) {
        if self.cursor_in_scroll_region() {
            let bg = self.cursor.attrs.bg;
            let cursor_row = self.cursor.row;
            let scroll_top = self.scroll_top;
            let scroll_bottom = self.scroll_bottom;
            self.grid_mut()
                .insert_lines(cursor_row, n, scroll_top, scroll_bottom, bg);
        }
    }

    /// Delete lines at cursor position
    pub fn delete_lines(&mut self, n: usize) {
        if self.cursor_in_scroll_region() {
            let bg = self.cursor.attrs.bg;
            let cursor_row = self.cursor.row;
            let scroll_top = self.scroll_top;
            let scroll_bottom = self.scroll_bottom;
            self.grid_mut()
                .delete_lines(cursor_row, n, scroll_top, scroll_bottom, bg);
        }
    }

    /// Insert blank characters at cursor position
    pub fn insert_chars(&mut self, n: usize) {
        let bg = self.cursor.attrs.bg;
        let cursor_col = self.cursor.col;
        let cursor_row = self.cursor.row;
        self.grid_mut().insert_chars(cursor_col, cursor_row, n, bg);
    }

    /// Delete characters at cursor position
    pub fn delete_chars(&mut self, n: usize) {
        let bg = self.cursor.attrs.bg;
        let cursor_col = self.cursor.col;
        let cursor_row = self.cursor.row;
        self.grid_mut().delete_chars(cursor_col, cursor_row, n, bg);
    }

    /// Scroll up within scroll region
    pub fn scroll_up(&mut self, n: usize) {
        let bg = self.cursor.attrs.bg;
        let scroll_top = self.scroll_top;
        let scroll_bottom = self.scroll_bottom;
        let scrolled = self.grid_mut().scroll_up(n, scroll_top, scroll_bottom, bg);
        if !self.using_alternate {
            self.scrollback.push(scrolled);
        }
    }

    /// Scroll down within scroll region
    pub fn scroll_down(&mut self, n: usize) {
        let bg = self.cursor.attrs.bg;
        let scroll_top = self.scroll_top;
        let scroll_bottom = self.scroll_bottom;
        self.grid_mut()
            .scroll_down(n, scroll_top, scroll_bottom, bg);
    }

    /// Set a tab stop at current column
    pub fn set_tab_stop(&mut self) {
        self.tabs.set(self.cursor.col);
    }

    /// Clear tab stop(s)
    pub fn clear_tab_stop(&mut self, mode: TabClearMode) {
        match mode {
            TabClearMode::Current => self.tabs.clear(self.cursor.col),
            TabClearMode::All => self.tabs.clear_all(),
        }
    }

    /// Register a hyperlink and return its ID
    pub fn register_hyperlink(&mut self, url: String, params: Option<String>) -> u32 {
        let id = self.next_hyperlink_id;
        self.next_hyperlink_id += 1;
        self.hyperlinks.push(super::Hyperlink { id, url, params });
        id
    }

    /// Get a hyperlink by ID
    pub fn get_hyperlink(&self, id: u32) -> Option<&super::Hyperlink> {
        self.hyperlinks.iter().find(|h| h.id == id)
    }

    /// Reset terminal to initial state
    pub fn reset(&mut self) {
        self.primary = Grid::new(self.cols, self.rows);
        self.alternate = Grid::new(self.cols, self.rows);
        self.using_alternate = false;
        self.scrollback.clear();
        self.cursor = Cursor::new();
        self.saved_cursor_primary = SavedCursor::default();
        self.saved_cursor_alternate = SavedCursor::default();
        self.modes = Modes::new();
        self.tabs = TabStops::new(self.cols);
        self.scroll_top = 0;
        self.scroll_bottom = self.rows.saturating_sub(1);
        self.title.clear();
        self.current_hyperlink = None;
    }

    /// Create a snapshot of the current screen state for testing/serialization
    pub fn snapshot(&self) -> ScreenSnapshot {
        ScreenSnapshot {
            grid: self.grid().clone(),
            cursor_col: self.cursor.col,
            cursor_row: self.cursor.row,
            cursor_visible: self.cursor.visible,
            cursor_style: self.cursor.style,
            using_alternate: self.using_alternate,
            scroll_top: self.scroll_top,
            scroll_bottom: self.scroll_bottom,
            modes: SnapshotModes {
                auto_wrap: self.modes.auto_wrap,
                origin_mode: self.modes.origin_mode,
                insert_mode: self.modes.insert_mode,
                cursor_visible: self.modes.cursor_visible,
                bracketed_paste: self.modes.bracketed_paste,
            },
            title: self.title.clone(),
        }
    }
}

/// Erase mode for ED and EL commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EraseMode {
    /// Erase from cursor to end
    ToEnd,
    /// Erase from beginning to cursor
    ToBeginning,
    /// Erase all
    All,
    /// Erase scrollback (ED only)
    Scrollback,
}

/// Tab clear mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabClearMode {
    /// Clear tab stop at current column
    Current,
    /// Clear all tab stops
    All,
}

/// A serializable snapshot of the screen state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenSnapshot {
    pub grid: Grid,
    pub cursor_col: usize,
    pub cursor_row: usize,
    pub cursor_visible: bool,
    pub cursor_style: super::CursorStyle,
    pub using_alternate: bool,
    pub scroll_top: usize,
    pub scroll_bottom: usize,
    pub modes: SnapshotModes,
    pub title: String,
}

/// Subset of modes included in snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotModes {
    pub auto_wrap: bool,
    pub origin_mode: bool,
    pub insert_mode: bool,
    pub cursor_visible: bool,
    pub bracketed_paste: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_new() {
        let screen = Screen::new(80, 24);
        assert_eq!(screen.cols(), 80);
        assert_eq!(screen.rows(), 24);
        assert_eq!(screen.cursor.col, 0);
        assert_eq!(screen.cursor.row, 0);
    }

    #[test]
    fn test_screen_print() {
        let mut screen = Screen::new(80, 24);
        screen.print('A');
        assert_eq!(screen.grid().cell(0, 0).unwrap().content, "A");
        assert_eq!(screen.cursor.col, 1);
    }

    #[test]
    fn test_screen_print_wrap() {
        let mut screen = Screen::new(5, 2);
        for c in "ABCDE".chars() {
            screen.print(c);
        }
        // Cursor should be at end of first line with pending wrap
        assert_eq!(screen.cursor.col, 4);
        assert!(screen.cursor.pending_wrap);

        // Print one more character
        screen.print('F');
        assert_eq!(screen.cursor.col, 1);
        assert_eq!(screen.cursor.row, 1);
        assert_eq!(screen.grid().cell(0, 1).unwrap().content, "F");
    }

    #[test]
    fn test_screen_linefeed() {
        let mut screen = Screen::new(80, 3);
        screen.cursor.row = 2;
        screen.print('A');
        screen.linefeed();
        // Should have scrolled
        assert_eq!(screen.cursor.row, 2);
        assert!(screen.grid().cell(0, 2).unwrap().is_empty());
    }

    #[test]
    fn test_screen_scroll_region() {
        let mut screen = Screen::new(80, 10);
        screen.set_scroll_region(2, 5);
        assert_eq!(screen.scroll_region(), (2, 5));

        screen.cursor.row = 5;
        screen.linefeed();
        // Should scroll within region only
        assert_eq!(screen.cursor.row, 5);
    }

    #[test]
    fn test_screen_alternate() {
        let mut screen = Screen::new(80, 24);
        screen.print('A');

        screen.enter_alternate_screen();
        assert!(screen.using_alternate);
        assert!(screen.grid().cell(0, 0).unwrap().is_empty());

        screen.print('B');
        assert_eq!(screen.grid().cell(0, 0).unwrap().content, "B");

        screen.exit_alternate_screen();
        assert!(!screen.using_alternate);
        assert_eq!(screen.grid().cell(0, 0).unwrap().content, "A");
    }

    #[test]
    fn test_screen_erase_in_display() {
        let mut screen = Screen::new(10, 3);
        for r in 0..3 {
            screen.cursor.row = r;
            screen.cursor.col = 0;
            screen.cursor.pending_wrap = false; // Clear pending wrap when repositioning
            for c in "ABCDEFGHI".chars() {
                // Only print 9 chars to avoid pending_wrap issues
                screen.print(c);
            }
        }

        screen.cursor.row = 1;
        screen.cursor.col = 5;
        screen.cursor.pending_wrap = false;
        screen.erase_in_display(EraseMode::ToEnd);

        // Row 0 should be intact
        assert_eq!(screen.grid().cell(0, 0).unwrap().content, "A");
        // Row 1 should be erased from col 5
        assert_eq!(screen.grid().cell(4, 1).unwrap().content, "E");
        assert!(screen.grid().cell(5, 1).unwrap().is_empty());
        // Row 2 should be erased
        assert!(screen.grid().cell(0, 2).unwrap().is_empty());
    }

    #[test]
    fn test_screen_cursor_movement() {
        let mut screen = Screen::new(80, 24);

        screen.move_cursor_to(5, 10);
        assert_eq!(screen.cursor.row, 4);
        assert_eq!(screen.cursor.col, 9);

        screen.move_cursor_up(2);
        assert_eq!(screen.cursor.row, 2);

        screen.move_cursor_down(5);
        assert_eq!(screen.cursor.row, 7);

        screen.move_cursor_forward(10);
        assert_eq!(screen.cursor.col, 19);

        screen.move_cursor_backward(5);
        assert_eq!(screen.cursor.col, 14);
    }

    #[test]
    fn test_screen_insert_delete_lines() {
        let mut screen = Screen::new(10, 5);

        // Fill with content
        for r in 0..5 {
            screen.cursor.row = r;
            screen.cursor.col = 0;
            screen.print(('A' as u8 + r as u8) as char);
        }

        screen.cursor.row = 2;
        screen.insert_lines(1);

        assert_eq!(screen.grid().cell(0, 0).unwrap().content, "A");
        assert_eq!(screen.grid().cell(0, 1).unwrap().content, "B");
        assert!(screen.grid().cell(0, 2).unwrap().is_empty());
        assert_eq!(screen.grid().cell(0, 3).unwrap().content, "C");
        assert_eq!(screen.grid().cell(0, 4).unwrap().content, "D");
    }
}
