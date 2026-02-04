//! Screen state management
//!
//! The screen represents the visible terminal area plus associated state.
//! It manages the grid, cursor, scroll region, and mode flags.

use serde::{Deserialize, Serialize};

use crate::cell::CellAttributes;
use crate::cursor::{Cursor, SavedCursor};
use crate::grid::Grid;
use crate::line::Line;

/// Screen mode flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ScreenMode {
    /// Origin mode (DECOM): cursor positioning relative to scroll region
    pub origin_mode: bool,
    /// Auto-wrap mode (DECAWM): wrap at end of line
    pub auto_wrap: bool,
    /// Insert mode (IRM): insert characters instead of overwriting
    pub insert_mode: bool,
    /// Line feed/new line mode (LNM): LF also does CR
    pub linefeed_mode: bool,
    /// Cursor visible (DECTCEM)
    pub cursor_visible: bool,
    /// Reverse video mode (DECSCNM)
    pub reverse_video: bool,
    /// Bracketed paste mode
    pub bracketed_paste: bool,
    /// Focus reporting mode
    pub focus_reporting: bool,
    /// Mouse tracking modes
    pub mouse_mode: MouseMode,
    /// Mouse encoding format
    pub mouse_encoding: MouseEncoding,
}

impl ScreenMode {
    pub fn new() -> Self {
        ScreenMode {
            origin_mode: false,
            auto_wrap: true,
            insert_mode: false,
            linefeed_mode: false,
            cursor_visible: true,
            reverse_video: false,
            bracketed_paste: false,
            focus_reporting: false,
            mouse_mode: MouseMode::None,
            mouse_encoding: MouseEncoding::X10,
        }
    }
}

/// Mouse tracking mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MouseMode {
    #[default]
    None,
    /// X10 compatibility mode (button press only)
    X10,
    /// VT200 mode (button press and release)
    VT200,
    /// VT200 highlight mode
    VT200Highlight,
    /// Button event tracking
    ButtonEvent,
    /// Any event tracking (motion while button pressed)
    AnyEvent,
}

/// Mouse encoding format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MouseEncoding {
    #[default]
    X10,
    /// UTF-8 encoding
    Utf8,
    /// SGR encoding (CSI < ...)
    Sgr,
    /// URXVT encoding
    Urxvt,
}

/// Tab stops
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabStops {
    /// Bit vector of tab stop positions
    stops: Vec<bool>,
}

impl TabStops {
    /// Create default tab stops (every 8 columns)
    pub fn new(cols: usize) -> Self {
        let mut stops = vec![false; cols];
        for i in (8..cols).step_by(8) {
            stops[i] = true;
        }
        TabStops { stops }
    }

    /// Set a tab stop at the given column
    pub fn set(&mut self, col: usize) {
        if col < self.stops.len() {
            self.stops[col] = true;
        }
    }

    /// Clear a tab stop at the given column
    pub fn clear(&mut self, col: usize) {
        if col < self.stops.len() {
            self.stops[col] = false;
        }
    }

    /// Clear all tab stops
    pub fn clear_all(&mut self) {
        for stop in &mut self.stops {
            *stop = false;
        }
    }

    /// Reset to default tab stops
    pub fn reset(&mut self, cols: usize) {
        self.stops = vec![false; cols];
        for i in (8..cols).step_by(8) {
            self.stops[i] = true;
        }
    }

    /// Find the next tab stop after the given column
    pub fn next(&self, col: usize) -> usize {
        for i in (col + 1)..self.stops.len() {
            if self.stops[i] {
                return i;
            }
        }
        // No tab stop found, go to last column
        self.stops.len().saturating_sub(1)
    }

    /// Find the previous tab stop before the given column
    pub fn prev(&self, col: usize) -> usize {
        for i in (0..col).rev() {
            if self.stops[i] {
                return i;
            }
        }
        0
    }

    /// Resize tab stops
    pub fn resize(&mut self, cols: usize) {
        let old_len = self.stops.len();
        self.stops.resize(cols, false);
        // Set default tab stops for new columns
        for i in ((old_len / 8 + 1) * 8..cols).step_by(8) {
            self.stops[i] = true;
        }
    }
}

/// A terminal screen (primary or alternate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screen {
    /// The grid of cells
    pub grid: Grid,
    /// Cursor state
    pub cursor: Cursor,
    /// Saved cursor state (for DECSC/DECRC)
    pub saved_cursor: SavedCursor,
    /// Scroll region top (inclusive, 0-indexed)
    pub scroll_top: usize,
    /// Scroll region bottom (inclusive, 0-indexed)
    pub scroll_bottom: usize,
    /// Mode flags
    pub mode: ScreenMode,
    /// Tab stops
    pub tabs: TabStops,
}

impl Screen {
    /// Create a new screen with the given dimensions
    pub fn new(rows: usize, cols: usize) -> Self {
        Screen {
            grid: Grid::new(rows, cols),
            cursor: Cursor::default(),
            saved_cursor: SavedCursor::default(),
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            mode: ScreenMode::new(),
            tabs: TabStops::new(cols),
        }
    }

    /// Get the number of rows
    pub fn rows(&self) -> usize {
        self.grid.rows()
    }

    /// Get the number of columns
    pub fn cols(&self) -> usize {
        self.grid.cols()
    }

    /// Resize the screen
    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.grid.resize(rows, cols);
        self.tabs.resize(cols);

        // Adjust scroll region
        self.scroll_bottom = rows.saturating_sub(1);
        if self.scroll_top >= rows {
            self.scroll_top = 0;
        }

        // Clamp cursor position
        self.cursor.row = self.cursor.row.min(rows.saturating_sub(1));
        self.cursor.col = self.cursor.col.min(cols.saturating_sub(1));
        self.cursor.pending_wrap = false;
    }

    /// Reset the screen to initial state
    pub fn reset(&mut self) {
        let rows = self.rows();
        let cols = self.cols();
        self.grid.clear();
        self.cursor = Cursor::default();
        self.saved_cursor = SavedCursor::default();
        self.scroll_top = 0;
        self.scroll_bottom = rows.saturating_sub(1);
        self.mode = ScreenMode::new();
        self.tabs = TabStops::new(cols);
    }

    /// Set the scroll region
    pub fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        let rows = self.rows();
        let top = top.min(rows.saturating_sub(1));
        let bottom = bottom.min(rows.saturating_sub(1));

        if top < bottom {
            self.scroll_top = top;
            self.scroll_bottom = bottom;
        }

        // Move cursor to home position (respecting origin mode)
        if self.mode.origin_mode {
            self.cursor.row = self.scroll_top;
        } else {
            self.cursor.row = 0;
        }
        self.cursor.col = 0;
        self.cursor.pending_wrap = false;
    }

    /// Scroll the screen up by n lines, returning lines for scrollback
    pub fn scroll_up(&mut self, n: usize) -> Vec<Line> {
        self.grid.scroll_up(self.scroll_top, self.scroll_bottom, n)
    }

    /// Scroll the screen down by n lines
    pub fn scroll_down(&mut self, n: usize) {
        self.grid.scroll_down(self.scroll_top, self.scroll_bottom, n);
    }

    /// Move cursor to position, respecting origin mode and bounds
    pub fn goto(&mut self, row: usize, col: usize) {
        let rows = self.rows();
        let cols = self.cols();

        let (min_row, max_row) = if self.mode.origin_mode {
            (self.scroll_top, self.scroll_bottom)
        } else {
            (0, rows.saturating_sub(1))
        };

        let actual_row = if self.mode.origin_mode {
            (self.scroll_top + row).min(max_row)
        } else {
            row.min(max_row)
        };

        self.cursor.row = actual_row.max(min_row);
        self.cursor.col = col.min(cols.saturating_sub(1));
        self.cursor.pending_wrap = false;
    }

    /// Move cursor down, scrolling if necessary
    pub fn linefeed(&mut self) -> Option<Line> {
        let mut scrolled = None;
        if self.cursor.row == self.scroll_bottom {
            // At bottom of scroll region, scroll up
            let lines = self.scroll_up(1);
            scrolled = lines.into_iter().next();
        } else if self.cursor.row < self.rows() - 1 {
            self.cursor.row += 1;
        }
        self.cursor.pending_wrap = false;
        scrolled
    }

    /// Reverse index: move cursor up, scrolling down if at top
    pub fn reverse_index(&mut self) {
        if self.cursor.row == self.scroll_top {
            self.scroll_down(1);
        } else if self.cursor.row > 0 {
            self.cursor.row -= 1;
        }
        self.cursor.pending_wrap = false;
    }

    /// Carriage return
    pub fn carriage_return(&mut self) {
        self.cursor.col = 0;
        self.cursor.pending_wrap = false;
    }

    /// Tab to next tab stop
    pub fn tab(&mut self) {
        let next = self.tabs.next(self.cursor.col);
        self.cursor.col = next.min(self.cols().saturating_sub(1));
        self.cursor.pending_wrap = false;
    }

    /// Backspace
    pub fn backspace(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        }
        self.cursor.pending_wrap = false;
    }

    /// Save cursor state
    pub fn save_cursor(&mut self) {
        self.saved_cursor = SavedCursor::from(&self.cursor);
    }

    /// Restore cursor state
    pub fn restore_cursor(&mut self) {
        self.saved_cursor.restore_to(&mut self.cursor);
        // Clamp to current screen size
        self.cursor.row = self.cursor.row.min(self.rows().saturating_sub(1));
        self.cursor.col = self.cursor.col.min(self.cols().saturating_sub(1));
    }

    /// Write a character at the cursor position
    pub fn write_char(&mut self, c: char) -> Option<Line> {
        self.write_grapheme(&c.to_string())
    }

    /// Write a grapheme cluster at the cursor position
    pub fn write_grapheme(&mut self, s: &str) -> Option<Line> {
        use unicode_width::UnicodeWidthStr;

        let width = s.width();
        if width == 0 {
            // Combining character: append to previous cell
            if self.cursor.col > 0 {
                let cell = self.grid.cell_mut(self.cursor.row, self.cursor.col - 1);
                cell.c.push_str(s);
            }
            return None;
        }

        let cols = self.cols();
        let mut scrolled = None;

        // Handle pending wrap
        if self.cursor.pending_wrap {
            if self.mode.auto_wrap {
                // Mark current line as wrapped
                self.grid.line_mut(self.cursor.row).wrapped = true;
                self.carriage_return();
                scrolled = self.linefeed();
            }
            self.cursor.pending_wrap = false;
        }

        // Handle insert mode
        if self.mode.insert_mode {
            self.grid.line_mut(self.cursor.row).insert_cells(self.cursor.col, width);
        }

        // Write the character
        let cell = self.grid.cell_mut(self.cursor.row, self.cursor.col);
        cell.c = s.to_string();
        self.cursor.attrs.apply_to(cell);

        // Handle wide characters
        if width == 2 {
            cell.flags.insert(crate::cell::CellFlags::WIDE_CHAR);
            // Set spacer in next cell if within bounds
            if self.cursor.col + 1 < cols {
                let spacer = self.grid.cell_mut(self.cursor.row, self.cursor.col + 1);
                spacer.reset();
                spacer.flags.insert(crate::cell::CellFlags::WIDE_CHAR_SPACER);
                self.cursor.attrs.apply_to(spacer);
            }
        }

        // Advance cursor
        let new_col = self.cursor.col + width;
        if new_col >= cols {
            // At or past last column
            self.cursor.col = cols - 1;
            self.cursor.pending_wrap = true;
        } else {
            self.cursor.col = new_col;
        }

        scrolled
    }

    /// Erase from cursor to end of line
    pub fn erase_to_eol(&mut self) {
        self.grid.line_mut(self.cursor.row).clear_from(self.cursor.col);
    }

    /// Erase from start of line to cursor
    pub fn erase_to_bol(&mut self) {
        self.grid.line_mut(self.cursor.row).clear_to(self.cursor.col);
    }

    /// Erase entire line
    pub fn erase_line(&mut self) {
        self.grid.line_mut(self.cursor.row).clear();
    }

    /// Erase from cursor to end of screen
    pub fn erase_below(&mut self) {
        self.grid.clear_below(self.cursor.row, self.cursor.col);
    }

    /// Erase from start of screen to cursor
    pub fn erase_above(&mut self) {
        self.grid.clear_above(self.cursor.row, self.cursor.col);
    }

    /// Erase entire screen
    pub fn erase_screen(&mut self) {
        self.grid.clear();
    }

    /// Erase n characters at cursor (ECH)
    pub fn erase_chars(&mut self, n: usize) {
        self.grid.line_mut(self.cursor.row).erase_cells(self.cursor.col, n);
    }

    /// Insert n blank lines at cursor row
    pub fn insert_lines(&mut self, n: usize) {
        if self.cursor.row >= self.scroll_top && self.cursor.row <= self.scroll_bottom {
            self.grid.insert_lines(self.cursor.row, self.scroll_bottom, n);
        }
    }

    /// Delete n lines at cursor row
    pub fn delete_lines(&mut self, n: usize) {
        if self.cursor.row >= self.scroll_top && self.cursor.row <= self.scroll_bottom {
            self.grid.delete_lines(self.cursor.row, self.scroll_bottom, n);
        }
    }

    /// Insert n blank characters at cursor
    pub fn insert_chars(&mut self, n: usize) {
        self.grid.line_mut(self.cursor.row).insert_cells(self.cursor.col, n);
    }

    /// Delete n characters at cursor
    pub fn delete_chars(&mut self, n: usize) {
        self.grid.line_mut(self.cursor.row).delete_cells(self.cursor.col, n);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_new() {
        let screen = Screen::new(24, 80);
        assert_eq!(screen.rows(), 24);
        assert_eq!(screen.cols(), 80);
        assert_eq!(screen.cursor.row, 0);
        assert_eq!(screen.cursor.col, 0);
    }

    #[test]
    fn test_screen_write_char() {
        let mut screen = Screen::new(24, 80);
        screen.write_char('A');
        assert_eq!(screen.grid.cell(0, 0).c, "A");
        assert_eq!(screen.cursor.col, 1);
    }

    #[test]
    fn test_screen_wrap() {
        let mut screen = Screen::new(24, 5);
        for c in "Hello".chars() {
            screen.write_char(c);
        }
        // Cursor should be at last column with pending wrap
        assert_eq!(screen.cursor.col, 4);
        assert!(screen.cursor.pending_wrap);

        // Writing another char should wrap
        screen.write_char('!');
        assert_eq!(screen.cursor.row, 1);
        assert_eq!(screen.cursor.col, 1);
        assert_eq!(screen.grid.cell(1, 0).c, "!");
    }

    #[test]
    fn test_screen_scroll() {
        let mut screen = Screen::new(3, 10);
        screen.cursor.row = 2;
        let scrolled = screen.linefeed();
        assert!(scrolled.is_some());
        assert_eq!(screen.cursor.row, 2);
    }

    #[test]
    fn test_screen_scroll_region() {
        let mut screen = Screen::new(5, 10);
        screen.set_scroll_region(1, 3);
        assert_eq!(screen.scroll_top, 1);
        assert_eq!(screen.scroll_bottom, 3);

        // Fill the screen
        for i in 0..5 {
            screen.cursor.row = i;
            screen.cursor.col = 0;
            screen.write_char((b'0' + i as u8) as char);
        }

        // Scroll within region
        screen.cursor.row = 3;
        screen.linefeed();

        // Row 0 and 4 should be unchanged
        assert_eq!(screen.grid.cell(0, 0).c, "0");
        assert_eq!(screen.grid.cell(4, 0).c, "4");
        // Region should have scrolled
        assert_eq!(screen.grid.cell(1, 0).c, "2");
        assert_eq!(screen.grid.cell(2, 0).c, "3");
        assert_eq!(screen.grid.cell(3, 0).c, " ");
    }

    #[test]
    fn test_screen_tab() {
        let mut screen = Screen::new(24, 80);
        screen.cursor.col = 0;
        screen.tab();
        assert_eq!(screen.cursor.col, 8);
        screen.tab();
        assert_eq!(screen.cursor.col, 16);
    }

    #[test]
    fn test_screen_erase() {
        let mut screen = Screen::new(3, 10);
        for row in 0..3 {
            for col in 0..10 {
                screen.grid.cell_mut(row, col).c = "X".to_string();
            }
        }

        screen.cursor.row = 1;
        screen.cursor.col = 5;
        screen.erase_below();

        // Row 0 unchanged
        assert_eq!(screen.grid.cell(0, 0).c, "X");
        // Row 1: cols 0-4 unchanged, 5-9 cleared
        assert_eq!(screen.grid.cell(1, 4).c, "X");
        assert_eq!(screen.grid.cell(1, 5).c, " ");
        // Row 2 cleared
        assert_eq!(screen.grid.cell(2, 0).c, " ");
    }

    #[test]
    fn test_screen_insert_delete_lines() {
        let mut screen = Screen::new(5, 10);
        for i in 0..5 {
            screen.grid.cell_mut(i, 0).c = format!("{}", i);
        }

        screen.cursor.row = 2;
        screen.insert_lines(1);

        assert_eq!(screen.grid.cell(0, 0).c, "0");
        assert_eq!(screen.grid.cell(1, 0).c, "1");
        assert_eq!(screen.grid.cell(2, 0).c, " "); // inserted
        assert_eq!(screen.grid.cell(3, 0).c, "2"); // shifted
        assert_eq!(screen.grid.cell(4, 0).c, "3"); // shifted
        // "4" is lost
    }
}
