//! Terminal screen - the main interface for terminal state
//!
//! The Screen struct ties together the grid, cursor, scrollback, and modes
//! to provide a complete terminal emulation state machine.

use crate::cell::CellAttributes;
use crate::charset::{parse_charset_designation, CharsetState};
use crate::cursor::{Cursor, SavedCursor};
use crate::grid::Grid;
use crate::line::Line;
use crate::modes::Modes;
use crate::scrollback::Scrollback;
use crate::selection::Selection;
use crate::snapshot::Snapshot;
use crate::Dimensions;

/// Tab stop interval (default)
const DEFAULT_TAB_WIDTH: usize = 8;

/// The complete terminal screen state
#[derive(Debug, Clone)]
pub struct Screen {
    /// Primary screen grid
    primary_grid: Grid,
    /// Alternate screen grid (for full-screen apps)
    alternate_grid: Grid,
    /// Whether we're using the alternate screen
    using_alternate: bool,
    /// Scrollback buffer (only for primary screen)
    scrollback: Scrollback,
    /// Cursor state
    cursor: Cursor,
    /// Saved cursor for primary screen (DECSC/DECRC)
    saved_cursor_primary: SavedCursor,
    /// Saved cursor for alternate screen
    saved_cursor_alternate: SavedCursor,
    /// Terminal modes
    modes: Modes,
    /// Scroll region (top, bottom) - 0-indexed, inclusive
    scroll_region: Option<(usize, usize)>,
    /// Tab stops
    tab_stops: Vec<bool>,
    /// Current selection
    selection: Selection,
    /// Window title
    title: String,
    /// Hyperlink registry (id -> url)
    hyperlinks: Vec<String>,
    /// Next hyperlink ID
    next_hyperlink_id: u32,
    /// Character set state
    charset: CharsetState,
}

impl Screen {
    /// Create a new screen with the specified dimensions
    pub fn new(dims: Dimensions) -> Self {
        let mut tab_stops = vec![false; dims.cols];
        for i in (0..dims.cols).step_by(DEFAULT_TAB_WIDTH) {
            tab_stops[i] = true;
        }

        Self {
            primary_grid: Grid::new(dims),
            alternate_grid: Grid::new(dims),
            using_alternate: false,
            scrollback: Scrollback::default(),
            cursor: Cursor::new(),
            saved_cursor_primary: SavedCursor::default(),
            saved_cursor_alternate: SavedCursor::default(),
            modes: Modes::new(),
            scroll_region: None,
            tab_stops,
            selection: Selection::new(),
            title: String::new(),
            hyperlinks: Vec::new(),
            next_hyperlink_id: 1,
            charset: CharsetState::new(),
        }
    }

    /// Get the current grid (primary or alternate)
    pub fn grid(&self) -> &Grid {
        if self.using_alternate {
            &self.alternate_grid
        } else {
            &self.primary_grid
        }
    }

    /// Get the current grid mutably
    fn grid_mut(&mut self) -> &mut Grid {
        if self.using_alternate {
            &mut self.alternate_grid
        } else {
            &mut self.primary_grid
        }
    }

    /// Get screen dimensions
    pub fn dimensions(&self) -> Dimensions {
        self.grid().dimensions()
    }

    /// Get number of columns
    pub fn cols(&self) -> usize {
        self.grid().cols()
    }

    /// Get number of rows
    pub fn rows(&self) -> usize {
        self.grid().rows()
    }

    /// Get cursor reference
    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    /// Get cursor mutably
    pub fn cursor_mut(&mut self) -> &mut Cursor {
        &mut self.cursor
    }

    /// Get modes reference
    pub fn modes(&self) -> &Modes {
        &self.modes
    }

    /// Get modes mutably
    pub fn modes_mut(&mut self) -> &mut Modes {
        &mut self.modes
    }

    /// Get scrollback reference
    pub fn scrollback(&self) -> &Scrollback {
        &self.scrollback
    }

    /// Get selection reference
    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    /// Get selection mutably
    pub fn selection_mut(&mut self) -> &mut Selection {
        &mut self.selection
    }

    /// Get window title
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set window title
    pub fn set_title(&mut self, title: &str) {
        // Limit title length for security
        self.title = title.chars().take(4096).collect();
    }

    /// Get scroll region bounds (top, bottom) or full screen if not set
    pub fn scroll_region(&self) -> (usize, usize) {
        self.scroll_region.unwrap_or((0, self.rows() - 1))
    }

    /// Set scroll region (1-indexed as per VT spec, converted to 0-indexed)
    pub fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        let rows = self.rows();
        let top = top.saturating_sub(1).min(rows - 1);
        let bottom = bottom.saturating_sub(1).min(rows - 1);

        if top < bottom {
            self.scroll_region = Some((top, bottom));
        } else {
            self.scroll_region = None;
        }

        // Move cursor to home position (respecting origin mode)
        if self.modes.origin_mode {
            self.cursor.row = top;
        } else {
            self.cursor.row = 0;
        }
        self.cursor.col = 0;
        self.cursor.pending_wrap = false;
    }

    /// Clear scroll region
    pub fn clear_scroll_region(&mut self) {
        self.scroll_region = None;
    }

    /// Print a character at the current cursor position
    pub fn print(&mut self, c: char) {
        // Translate character through current charset
        let c = self.charset.translate(c);
        // Clear single shift after use
        self.charset.clear_single_shift();

        let cols = self.cols();
        let (_, scroll_bottom) = self.scroll_region();

        // Handle pending wrap
        if self.cursor.pending_wrap {
            self.cursor.pending_wrap = false;
            self.cursor.col = 0;

            if self.cursor.row >= scroll_bottom {
                self.scroll_up(1);
            } else {
                self.cursor.row += 1;
            }

            // Mark previous line as wrapped
            if self.cursor.row > 0 {
                let row = self.cursor.row;
                self.grid_mut().line_mut(row - 1).wrapped = true;
            }
        }

        // Get character width
        let width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);

        // Handle insert mode
        if self.modes.insert_mode && width > 0 {
            let row = self.cursor.row;
            let col = self.cursor.col;
            let attrs = self.cursor.attrs;
            self.grid_mut()
                .line_mut(row)
                .insert_cells(col, width, attrs);
        }

        // Write the character
        if self.cursor.col < cols {
            let row = self.cursor.row;
            let col = self.cursor.col;
            let attrs = self.cursor.attrs;
            let hyperlink_id = self.cursor.hyperlink_id;
            let cell = self.grid_mut().line_mut(row).cell_mut(col);
            cell.set_char(c);
            cell.attrs = attrs;
            cell.hyperlink_id = hyperlink_id;

            // Handle wide characters
            if width == 2 && col + 1 < cols {
                self.grid_mut()
                    .line_mut(row)
                    .cell_mut(col + 1)
                    .set_continuation();
            }
        }

        // Advance cursor
        let new_col = self.cursor.col + width.max(1);
        if new_col >= cols {
            if self.modes.auto_wrap {
                self.cursor.col = cols - 1;
                self.cursor.pending_wrap = true;
            } else {
                self.cursor.col = cols - 1;
            }
        } else {
            self.cursor.col = new_col;
        }
    }

    /// Handle backspace (BS)
    pub fn backspace(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
            self.cursor.pending_wrap = false;
        }
    }

    /// Handle horizontal tab (HT)
    pub fn tab(&mut self) {
        let cols = self.cols();
        let mut col = self.cursor.col + 1;

        while col < cols {
            if self.tab_stops.get(col).copied().unwrap_or(false) {
                break;
            }
            col += 1;
        }

        self.cursor.col = col.min(cols - 1);
        self.cursor.pending_wrap = false;
    }

    /// Handle carriage return (CR)
    pub fn carriage_return(&mut self) {
        self.cursor.col = 0;
        self.cursor.pending_wrap = false;
    }

    /// Handle line feed (LF), vertical tab (VT), form feed (FF)
    pub fn linefeed(&mut self) {
        let (_, scroll_bottom) = self.scroll_region();

        if self.cursor.row >= scroll_bottom {
            self.scroll_up(1);
        } else {
            self.cursor.row += 1;
        }
        self.cursor.pending_wrap = false;

        // In linefeed mode, LF also does CR
        if self.modes.linefeed_mode {
            self.cursor.col = 0;
        }
    }

    /// Handle reverse index (RI) - move cursor up, scroll if at top
    pub fn reverse_index(&mut self) {
        let (scroll_top, _) = self.scroll_region();

        if self.cursor.row <= scroll_top {
            self.scroll_down(1);
        } else {
            self.cursor.row -= 1;
        }
        self.cursor.pending_wrap = false;
    }

    /// Handle index (IND) - move cursor down, scroll if at bottom
    pub fn index(&mut self) {
        self.linefeed();
    }

    /// Handle next line (NEL) - move to start of next line
    pub fn next_line(&mut self) {
        self.linefeed();
        self.cursor.col = 0;
    }

    /// Scroll up by n lines within scroll region
    pub fn scroll_up(&mut self, n: usize) {
        let (top, bottom) = self.scroll_region();
        let attrs = self.cursor.attrs;

        let scrolled = self.grid_mut().scroll_up(top, bottom, n, attrs);

        // Add to scrollback if scrolling primary screen from top
        if !self.using_alternate && top == 0 {
            self.scrollback.push_lines(scrolled);
        }
    }

    /// Scroll down by n lines within scroll region
    pub fn scroll_down(&mut self, n: usize) {
        let (top, bottom) = self.scroll_region();
        let attrs = self.cursor.attrs;
        self.grid_mut().scroll_down(top, bottom, n, attrs);
    }

    /// Move cursor to position (1-indexed as per VT spec)
    pub fn move_cursor_to(&mut self, row: usize, col: usize) {
        let cols = self.cols();
        let rows = self.rows();
        let (scroll_top, scroll_bottom) = self.scroll_region();

        let col = col.saturating_sub(1);
        let row = row.saturating_sub(1);

        self.cursor.col = col.min(cols - 1);

        if self.modes.origin_mode {
            // Origin mode: row is relative to scroll region
            self.cursor.row = (scroll_top + row).min(scroll_bottom);
        } else {
            self.cursor.row = row.min(rows - 1);
        }

        self.cursor.pending_wrap = false;
    }

    /// Move cursor up by n rows
    pub fn move_cursor_up(&mut self, n: usize) {
        let (scroll_top, _) = self.scroll_region();
        let min_row = if self.modes.origin_mode {
            scroll_top
        } else {
            0
        };
        self.cursor.row = self.cursor.row.saturating_sub(n).max(min_row);
        self.cursor.pending_wrap = false;
    }

    /// Move cursor down by n rows
    pub fn move_cursor_down(&mut self, n: usize) {
        let (_, scroll_bottom) = self.scroll_region();
        let max_row = if self.modes.origin_mode {
            scroll_bottom
        } else {
            self.rows() - 1
        };
        self.cursor.row = (self.cursor.row + n).min(max_row);
        self.cursor.pending_wrap = false;
    }

    /// Move cursor left by n columns
    pub fn move_cursor_left(&mut self, n: usize) {
        self.cursor.col = self.cursor.col.saturating_sub(n);
        self.cursor.pending_wrap = false;
    }

    /// Move cursor right by n columns
    pub fn move_cursor_right(&mut self, n: usize) {
        let cols = self.cols();
        self.cursor.col = (self.cursor.col + n).min(cols - 1);
        self.cursor.pending_wrap = false;
    }

    /// Set cursor column (1-indexed)
    pub fn set_cursor_col(&mut self, col: usize) {
        let cols = self.cols();
        self.cursor.col = col.saturating_sub(1).min(cols - 1);
        self.cursor.pending_wrap = false;
    }

    /// Set cursor row (1-indexed)
    pub fn set_cursor_row(&mut self, row: usize) {
        let rows = self.rows();
        let (scroll_top, scroll_bottom) = self.scroll_region();

        let row = row.saturating_sub(1);

        if self.modes.origin_mode {
            self.cursor.row = (scroll_top + row).min(scroll_bottom);
        } else {
            self.cursor.row = row.min(rows - 1);
        }
        self.cursor.pending_wrap = false;
    }

    /// Save cursor state (DECSC)
    pub fn save_cursor(&mut self) {
        let saved = SavedCursor::save(&self.cursor);
        if self.using_alternate {
            self.saved_cursor_alternate = saved;
        } else {
            self.saved_cursor_primary = saved;
        }
    }

    /// Restore cursor state (DECRC)
    pub fn restore_cursor(&mut self) {
        let saved = if self.using_alternate {
            &self.saved_cursor_alternate
        } else {
            &self.saved_cursor_primary
        };
        saved.restore(&mut self.cursor);

        // Clamp to screen bounds
        let cols = self.cols();
        let rows = self.rows();
        self.cursor.col = self.cursor.col.min(cols - 1);
        self.cursor.row = self.cursor.row.min(rows - 1);
    }

    /// Erase display (ED)
    pub fn erase_display(&mut self, mode: u16) {
        let attrs = self.cursor.attrs;
        let row = self.cursor.row;
        let col = self.cursor.col;

        match mode {
            0 => {
                // Erase from cursor to end of display
                self.grid_mut().clear_below(row, col, attrs);
            }
            1 => {
                // Erase from start of display to cursor
                self.grid_mut().clear_above(row, col, attrs);
            }
            2 => {
                // Erase entire display
                // Before clearing, save non-empty lines to scrollback (only for primary screen)
                // This preserves terminal history so users can scroll up to see previous content
                // This matches behavior of terminals like Terminal.app where ED mode=2
                // doesn't completely erase history
                if !self.using_alternate {
                    let rows = self.rows();
                    for i in 0..rows {
                        let line = self.primary_grid.line(i);
                        if !line.is_empty() {
                            self.scrollback.push(line.clone());
                        }
                    }
                }
                self.grid_mut().clear(attrs);
            }
            3 => {
                // Erase scrollback (xterm extension)
                // Note: Many modern TUI apps (Claude Code, Gemini CLI, etc.) send ED mode=3
                // immediately after ED mode=2, which would clear the scrollback we just saved.
                // To preserve terminal history and match Terminal.app behavior, we intentionally
                // do NOT clear the scrollback here. Users can still use Cmd+K or similar to
                // manually clear scrollback if needed.
                // self.scrollback.clear();
                log::debug!("ED mode=3 (clear scrollback) ignored to preserve terminal history");
            }
            _ => {}
        }
    }

    /// Erase line (EL)
    pub fn erase_line(&mut self, mode: u16) {
        let attrs = self.cursor.attrs;
        let row = self.cursor.row;
        let col = self.cursor.col;

        match mode {
            0 => {
                // Erase from cursor to end of line
                self.grid_mut().line_mut(row).clear_from(col, attrs);
            }
            1 => {
                // Erase from start of line to cursor
                self.grid_mut().line_mut(row).clear_to(col, attrs);
            }
            2 => {
                // Erase entire line
                self.grid_mut().line_mut(row).clear(attrs);
            }
            _ => {}
        }
    }

    /// Erase characters (ECH)
    pub fn erase_chars(&mut self, n: usize) {
        let attrs = self.cursor.attrs;
        let row = self.cursor.row;
        let col = self.cursor.col;
        self.grid_mut().line_mut(row).erase_cells(col, n, attrs);
    }

    /// Insert lines (IL)
    pub fn insert_lines(&mut self, n: usize) {
        let (_, bottom) = self.scroll_region();
        let row = self.cursor.row;
        let attrs = self.cursor.attrs;

        if row <= bottom {
            self.grid_mut().insert_lines(row, n, bottom, attrs);
        }
    }

    /// Delete lines (DL)
    pub fn delete_lines(&mut self, n: usize) {
        let (_, bottom) = self.scroll_region();
        let row = self.cursor.row;
        let attrs = self.cursor.attrs;

        if row <= bottom {
            self.grid_mut().delete_lines(row, n, bottom, attrs);
        }
    }

    /// Insert characters (ICH)
    pub fn insert_chars(&mut self, n: usize) {
        let row = self.cursor.row;
        let col = self.cursor.col;
        let attrs = self.cursor.attrs;
        self.grid_mut().line_mut(row).insert_cells(col, n, attrs);
    }

    /// Delete characters (DCH)
    pub fn delete_chars(&mut self, n: usize) {
        let row = self.cursor.row;
        let col = self.cursor.col;
        let attrs = self.cursor.attrs;
        self.grid_mut().line_mut(row).delete_cells(col, n, attrs);
    }

    /// Set tab stop at current column (HTS)
    pub fn set_tab_stop(&mut self) {
        let col = self.cursor.col;
        if col < self.tab_stops.len() {
            self.tab_stops[col] = true;
        }
    }

    /// Clear tab stops (TBC)
    pub fn clear_tab_stop(&mut self, mode: u16) {
        match mode {
            0 => {
                // Clear tab stop at current column
                let col = self.cursor.col;
                if col < self.tab_stops.len() {
                    self.tab_stops[col] = false;
                }
            }
            3 => {
                // Clear all tab stops
                for stop in &mut self.tab_stops {
                    *stop = false;
                }
            }
            _ => {}
        }
    }

    /// Switch to alternate screen
    /// Always clears the alternate grid to ensure a clean slate for TUI applications
    pub fn enter_alternate_screen(&mut self) {
        if !self.using_alternate {
            self.using_alternate = true;
            self.modes.alternate_screen = true;
            self.saved_cursor_primary = SavedCursor::save(&self.cursor);
        }
        // Always clear the alternate grid and reset cursor when entering alternate screen
        // This ensures TUI applications like Claude Code, vim, htop get a clean canvas
        self.cursor.reset();
        self.alternate_grid.clear(CellAttributes::default());
    }

    /// Switch back to primary screen
    pub fn exit_alternate_screen(&mut self) {
        if self.using_alternate {
            self.using_alternate = false;
            self.modes.alternate_screen = false;
            self.saved_cursor_primary.restore(&mut self.cursor);
        }
    }

    /// Resize the screen
    pub fn resize(&mut self, dims: Dimensions) {
        let attrs = self.cursor.attrs;

        self.primary_grid.resize(dims, attrs);
        self.alternate_grid.resize(dims, attrs);

        // Update tab stops
        self.tab_stops.resize(dims.cols, false);
        for i in (0..dims.cols).step_by(DEFAULT_TAB_WIDTH) {
            self.tab_stops[i] = true;
        }

        // Clamp cursor
        self.cursor.col = self.cursor.col.min(dims.cols.saturating_sub(1));
        self.cursor.row = self.cursor.row.min(dims.rows.saturating_sub(1));

        // Clear scroll region on resize
        self.scroll_region = None;
    }

    /// Reset terminal to initial state
    pub fn reset(&mut self) {
        let dims = self.dimensions();
        *self = Self::new(dims);
    }

    /// Create a snapshot of the current state
    pub fn snapshot(&self, include_scrollback: bool) -> Snapshot {
        Snapshot::from_terminal(
            self.grid(),
            &self.cursor,
            &self.modes,
            if include_scrollback {
                Some(&self.scrollback)
            } else {
                None
            },
            self.scroll_region,
            if self.title.is_empty() {
                None
            } else {
                Some(&self.title)
            },
            include_scrollback,
        )
    }

    /// Register a hyperlink and return its ID
    pub fn register_hyperlink(&mut self, url: &str) -> u32 {
        // Check if URL already registered
        for (i, existing) in self.hyperlinks.iter().enumerate() {
            if existing == url {
                return (i + 1) as u32;
            }
        }

        // Register new URL
        let id = self.next_hyperlink_id;
        self.next_hyperlink_id += 1;
        self.hyperlinks.push(url.to_string());
        id
    }

    /// Get hyperlink URL by ID
    pub fn get_hyperlink(&self, id: u32) -> Option<&str> {
        if id == 0 {
            return None;
        }
        self.hyperlinks.get((id - 1) as usize).map(|s| s.as_str())
    }

    /// Get a line from the grid
    pub fn line(&self, row: usize) -> &Line {
        self.grid().line(row)
    }

    /// Get charset state reference
    pub fn charset(&self) -> &CharsetState {
        &self.charset
    }

    /// Get charset state mutably
    pub fn charset_mut(&mut self) -> &mut CharsetState {
        &mut self.charset
    }

    /// Shift In (SI) - select G0 into GL
    pub fn shift_in(&mut self) {
        self.charset.shift_in();
    }

    /// Shift Out (SO) - select G1 into GL
    pub fn shift_out(&mut self) {
        self.charset.shift_out();
    }

    /// Designate a character set to a G-set slot
    pub fn designate_charset(&mut self, slot: u8, designation: char) {
        let charset = parse_charset_designation(designation);
        self.charset.set_slot(slot, charset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_new() {
        let screen = Screen::new(Dimensions::new(80, 24));
        assert_eq!(screen.cols(), 80);
        assert_eq!(screen.rows(), 24);
        assert_eq!(screen.cursor().col, 0);
        assert_eq!(screen.cursor().row, 0);
    }

    #[test]
    fn test_screen_print() {
        let mut screen = Screen::new(Dimensions::new(80, 24));
        screen.print('H');
        screen.print('i');

        assert_eq!(screen.cursor().col, 2);
        assert_eq!(screen.line(0).cell(0).display_char(), 'H');
        assert_eq!(screen.line(0).cell(1).display_char(), 'i');
    }

    #[test]
    fn test_screen_wrap() {
        let mut screen = Screen::new(Dimensions::new(5, 3));

        for c in "Hello World".chars() {
            screen.print(c);
        }

        assert_eq!(screen.line(0).text(), "Hello");
        assert_eq!(screen.line(1).text(), " Worl");
        assert_eq!(screen.line(2).text(), "d");
    }

    #[test]
    fn test_screen_linefeed() {
        let mut screen = Screen::new(Dimensions::new(80, 3));
        screen.print('A');
        screen.linefeed();
        screen.carriage_return();
        screen.print('B');
        screen.linefeed();
        screen.carriage_return();
        screen.print('C');
        screen.linefeed(); // Should scroll
        screen.carriage_return();
        screen.print('D');

        assert_eq!(screen.line(0).cell(0).display_char(), 'B');
        assert_eq!(screen.line(1).cell(0).display_char(), 'C');
        assert_eq!(screen.line(2).cell(0).display_char(), 'D');
    }

    #[test]
    fn test_screen_cursor_movement() {
        let mut screen = Screen::new(Dimensions::new(80, 24));

        screen.move_cursor_to(5, 10); // 1-indexed
        assert_eq!(screen.cursor().row, 4);
        assert_eq!(screen.cursor().col, 9);

        screen.move_cursor_up(2);
        assert_eq!(screen.cursor().row, 2);

        screen.move_cursor_down(5);
        assert_eq!(screen.cursor().row, 7);

        screen.move_cursor_left(3);
        assert_eq!(screen.cursor().col, 6);

        screen.move_cursor_right(10);
        assert_eq!(screen.cursor().col, 16);
    }

    #[test]
    fn test_screen_erase_display() {
        let mut screen = Screen::new(Dimensions::new(10, 3));

        for row in 0..3 {
            for col in 0..10 {
                screen.move_cursor_to(row + 1, col + 1);
                screen.print('X');
            }
        }

        screen.move_cursor_to(2, 5);
        screen.erase_display(0); // Erase from cursor to end

        assert_eq!(screen.line(0).text(), "XXXXXXXXXX");
        assert_eq!(screen.line(1).text(), "XXXX");
        assert!(screen.line(2).is_empty());
    }

    #[test]
    fn test_screen_scroll_region() {
        let mut screen = Screen::new(Dimensions::new(10, 5));

        for row in 0..5 {
            screen.move_cursor_to(row + 1, 1);
            screen.print((b'A' + row as u8) as char);
        }
        // Screen: A, B, C, D, E

        screen.set_scroll_region(2, 4); // Rows 2-4 (B, C, D)
        screen.move_cursor_to(4, 1); // Row 4 (D)
        screen.linefeed(); // Should scroll within region

        assert_eq!(screen.line(0).cell(0).display_char(), 'A');
        assert_eq!(screen.line(1).cell(0).display_char(), 'C');
        assert_eq!(screen.line(2).cell(0).display_char(), 'D');
        assert!(screen.line(3).cell(0).is_empty());
        assert_eq!(screen.line(4).cell(0).display_char(), 'E');
    }

    #[test]
    fn test_screen_alternate() {
        let mut screen = Screen::new(Dimensions::new(80, 24));
        screen.print('A');

        screen.enter_alternate_screen();
        assert!(screen.modes().alternate_screen);
        assert!(screen.line(0).cell(0).is_empty());

        screen.print('B');
        assert_eq!(screen.line(0).cell(0).display_char(), 'B');

        screen.exit_alternate_screen();
        assert!(!screen.modes().alternate_screen);
        assert_eq!(screen.line(0).cell(0).display_char(), 'A');
    }

    #[test]
    fn test_screen_tab() {
        let mut screen = Screen::new(Dimensions::new(80, 24));
        screen.print('A');
        screen.tab();
        assert_eq!(screen.cursor().col, 8);

        screen.print('B');
        screen.tab();
        assert_eq!(screen.cursor().col, 16);
    }

    #[test]
    fn test_screen_insert_delete_lines() {
        let mut screen = Screen::new(Dimensions::new(10, 5));

        for row in 0..5 {
            screen.move_cursor_to(row + 1, 1);
            screen.print((b'A' + row as u8) as char);
        }
        // Screen: A, B, C, D, E

        screen.move_cursor_to(2, 1);
        screen.insert_lines(2);
        // Should be: A, _, _, B, C

        assert_eq!(screen.line(0).cell(0).display_char(), 'A');
        assert!(screen.line(1).cell(0).is_empty());
        assert!(screen.line(2).cell(0).is_empty());
        assert_eq!(screen.line(3).cell(0).display_char(), 'B');
        assert_eq!(screen.line(4).cell(0).display_char(), 'C');
    }

    #[test]
    fn test_screen_save_restore_cursor() {
        let mut screen = Screen::new(Dimensions::new(80, 24));
        screen.move_cursor_to(10, 20);
        screen.cursor_mut().attrs.bold = true;

        screen.save_cursor();

        screen.move_cursor_to(1, 1);
        screen.cursor_mut().attrs.bold = false;

        screen.restore_cursor();

        assert_eq!(screen.cursor().row, 9);
        assert_eq!(screen.cursor().col, 19);
        assert!(screen.cursor().attrs.bold);
    }
}
