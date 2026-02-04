//! Screen model implementation
//!
//! The screen represents the visible terminal grid plus state like scroll
//! regions, tab stops, and mode flags. It supports both primary and alternate
//! screen buffers.

use serde::{Deserialize, Serialize};

use super::cell::{Cell, Color, Style};
use super::cursor::{Cursor, SavedCursor};
use super::scrollback::{Line, Scrollback};

/// Terminal mode flags
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Modes {
    /// Application cursor keys (DECCKM)
    pub application_cursor: bool,
    /// Application keypad mode (DECKPAM/DECKPNM)
    pub application_keypad: bool,
    /// Bracketed paste mode (xterm)
    pub bracketed_paste: bool,
    /// Focus reporting mode
    pub focus_reporting: bool,
    /// Mouse tracking modes
    pub mouse_tracking: MouseMode,
    /// Mouse encoding format
    pub mouse_encoding: MouseEncoding,
    /// Alternate screen active
    pub alternate_screen: bool,
    /// Line feed/new line mode (LNM)
    pub linefeed_mode: bool,
    /// Reverse video mode (DECSCNM)
    pub reverse_video: bool,
}

/// Mouse tracking mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MouseMode {
    /// No mouse tracking
    #[default]
    None,
    /// X10 compatibility mode (button press only)
    X10,
    /// VT200 normal tracking (button press and release)
    Normal,
    /// VT200 highlight tracking
    Highlight,
    /// Button-event tracking (motion while button pressed)
    ButtonEvent,
    /// Any-event tracking (all motion)
    AnyEvent,
}

/// Mouse coordinate encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MouseEncoding {
    /// Default X10 encoding (limited to 223 columns/rows)
    #[default]
    X10,
    /// UTF-8 encoding
    Utf8,
    /// SGR encoding (CSI < ... M/m)
    Sgr,
    /// URXVT encoding
    Urxvt,
}

/// The main screen structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screen {
    /// Number of columns
    cols: usize,
    /// Number of rows
    rows: usize,
    /// Primary screen grid
    primary_grid: Vec<Line>,
    /// Alternate screen grid
    alternate_grid: Vec<Line>,
    /// Scrollback buffer (only for primary screen)
    scrollback: Scrollback,
    /// Primary screen cursor
    primary_cursor: Cursor,
    /// Alternate screen cursor
    alternate_cursor: Cursor,
    /// Saved cursor for primary screen (DECSC/DECRC)
    primary_saved_cursor: SavedCursor,
    /// Saved cursor for alternate screen
    alternate_saved_cursor: SavedCursor,
    /// Scroll region top (0-indexed, inclusive)
    scroll_top: usize,
    /// Scroll region bottom (0-indexed, inclusive)
    scroll_bottom: usize,
    /// Tab stops (column indices)
    tab_stops: Vec<bool>,
    /// Terminal modes
    pub modes: Modes,
    /// Whether we're on the alternate screen
    on_alternate: bool,
    /// Dirty flag for rendering optimization
    dirty: bool,
    /// Dirty lines bitmap (for partial redraws)
    dirty_lines: Vec<bool>,
    /// Window title (set via OSC 0/2)
    pub title: String,
    /// Hyperlink registry (id -> url)
    hyperlinks: Vec<String>,
    /// Next hyperlink ID
    next_hyperlink_id: u32,
}

impl Screen {
    /// Create a new screen with the given dimensions
    pub fn new(cols: usize, rows: usize, scrollback_capacity: usize) -> Self {
        let primary_grid = (0..rows).map(|_| Line::new(cols)).collect();
        let alternate_grid = (0..rows).map(|_| Line::new(cols)).collect();

        // Initialize tab stops every 8 columns
        let mut tab_stops = vec![false; cols];
        for i in (8..cols).step_by(8) {
            tab_stops[i] = true;
        }

        Self {
            cols,
            rows,
            primary_grid,
            alternate_grid,
            scrollback: Scrollback::new(scrollback_capacity),
            primary_cursor: Cursor::new(),
            alternate_cursor: Cursor::new(),
            primary_saved_cursor: SavedCursor::default(),
            alternate_saved_cursor: SavedCursor::default(),
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            tab_stops,
            modes: Modes::default(),
            on_alternate: false,
            dirty: true,
            dirty_lines: vec![true; rows],
            title: String::new(),
            hyperlinks: Vec::new(),
            next_hyperlink_id: 1,
        }
    }

    /// Get the number of columns
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Get the number of rows
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Get a reference to the current grid
    fn grid(&self) -> &Vec<Line> {
        if self.on_alternate {
            &self.alternate_grid
        } else {
            &self.primary_grid
        }
    }

    /// Get a mutable reference to the current grid
    fn grid_mut(&mut self) -> &mut Vec<Line> {
        if self.on_alternate {
            &mut self.alternate_grid
        } else {
            &mut self.primary_grid
        }
    }

    /// Get a reference to the current cursor
    pub fn cursor(&self) -> &Cursor {
        if self.on_alternate {
            &self.alternate_cursor
        } else {
            &self.primary_cursor
        }
    }

    /// Get a mutable reference to the current cursor
    pub fn cursor_mut(&mut self) -> &mut Cursor {
        if self.on_alternate {
            &mut self.alternate_cursor
        } else {
            &mut self.primary_cursor
        }
    }

    /// Get the scrollback buffer
    pub fn scrollback(&self) -> &Scrollback {
        &self.scrollback
    }

    /// Get scroll region top
    pub fn scroll_top(&self) -> usize {
        self.scroll_top
    }

    /// Get scroll region bottom
    pub fn scroll_bottom(&self) -> usize {
        self.scroll_bottom
    }

    /// Check if the screen is dirty (needs redraw)
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the screen as clean
    pub fn mark_clean(&mut self) {
        self.dirty = false;
        for d in &mut self.dirty_lines {
            *d = false;
        }
    }

    /// Mark a line as dirty
    fn mark_line_dirty(&mut self, row: usize) {
        if row < self.dirty_lines.len() {
            self.dirty_lines[row] = true;
        }
        self.dirty = true;
    }

    /// Mark all lines as dirty
    fn mark_all_dirty(&mut self) {
        for d in &mut self.dirty_lines {
            *d = true;
        }
        self.dirty = true;
    }

    /// Get a cell at the given position
    pub fn get_cell(&self, col: usize, row: usize) -> Option<&Cell> {
        self.grid().get(row).and_then(|line| line.get(col))
    }

    /// Get a mutable cell at the given position
    pub fn get_cell_mut(&mut self, col: usize, row: usize) -> Option<&mut Cell> {
        self.grid_mut()
            .get_mut(row)
            .and_then(|line| line.get_mut(col))
    }

    /// Get a line at the given row
    pub fn get_line(&self, row: usize) -> Option<&Line> {
        self.grid().get(row)
    }

    /// Print a character at the current cursor position
    pub fn print_char(&mut self, c: char) {
        let width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);

        // Handle zero-width characters (combining marks)
        if width == 0 {
            // Append to previous cell if possible
            let col = self.cursor().col;
            let row = self.cursor().row;
            if col > 0 {
                if let Some(cell) = self.get_cell_mut(col - 1, row) {
                    cell.append_combining(c);
                    self.mark_line_dirty(row);
                }
            }
            return;
        }

        // Handle pending wrap
        if self.cursor().pending_wrap {
            self.wrap_cursor();
        }

        let col = self.cursor().col;
        let row = self.cursor().row;
        let cols = self.cols;

        // Check if we need to handle wide character at end of line
        if width == 2 && col == cols - 1 {
            // Wide char at last column - clear the cell and wrap
            if let Some(cell) = self.get_cell_mut(col, row) {
                cell.clear();
            }
            self.wrap_cursor();
        }

        // Get current cursor state for cell attributes
        let fg = self.cursor().fg;
        let bg = self.cursor().bg;
        let style = self.cursor().style;
        let hyperlink_id = self.cursor().hyperlink_id;
        let insert_mode = self.cursor().insert_mode;

        let col = self.cursor().col;
        let row = self.cursor().row;

        // Insert mode: shift characters right
        if insert_mode {
            self.insert_blank_cells(col, row, width);
        }

        // Write the character
        if let Some(cell) = self.get_cell_mut(col, row) {
            cell.content.clear();
            cell.content.push(c);
            cell.fg = fg;
            cell.bg = bg;
            cell.style = style;
            cell.hyperlink_id = hyperlink_id;
            cell.width = width as u8;
        }

        // For wide characters, mark the next cell as continuation
        if width == 2 && col + 1 < cols {
            if let Some(cell) = self.get_cell_mut(col + 1, row) {
                cell.content.clear();
                cell.fg = fg;
                cell.bg = bg;
                cell.style = style;
                cell.hyperlink_id = hyperlink_id;
                cell.width = 0; // Continuation cell
            }
        }

        self.mark_line_dirty(row);

        // Advance cursor
        let new_col = col + width;
        if new_col >= cols {
            if self.cursor().autowrap {
                self.cursor_mut().col = cols - 1;
                self.cursor_mut().pending_wrap = true;
            } else {
                self.cursor_mut().col = cols - 1;
            }
        } else {
            self.cursor_mut().col = new_col;
        }
    }

    /// Handle cursor wrap (newline at end of line)
    fn wrap_cursor(&mut self) {
        self.cursor_mut().pending_wrap = false;
        self.cursor_mut().col = 0;

        let row = self.cursor().row;
        if row == self.scroll_bottom {
            self.scroll_up(1);
        } else if row < self.rows - 1 {
            self.cursor_mut().row += 1;
        }

        // Mark the line as wrapped
        if let Some(line) = self.grid_mut().get_mut(row) {
            line.wrapped = true;
        }
    }

    /// Insert blank cells at position, shifting existing cells right
    fn insert_blank_cells(&mut self, col: usize, row: usize, count: usize) {
        let cols = self.cols;
        let grid = if self.on_alternate {
            &mut self.alternate_grid
        } else {
            &mut self.primary_grid
        };

        if let Some(line) = grid.get_mut(row) {
            // Shift cells right
            for i in (col + count..cols).rev() {
                if i >= count {
                    line.cells[i] = line.cells[i - count].clone();
                }
            }
            // Clear inserted cells
            for i in col..(col + count).min(cols) {
                line.cells[i].clear();
            }
        }
    }

    /// Handle linefeed (LF)
    pub fn linefeed(&mut self) {
        self.cursor_mut().pending_wrap = false;
        let row = self.cursor().row;

        if row == self.scroll_bottom {
            self.scroll_up(1);
        } else if row < self.rows - 1 {
            self.cursor_mut().row += 1;
        }

        // In linefeed mode, LF also does CR
        if self.modes.linefeed_mode {
            self.cursor_mut().col = 0;
        }
    }

    /// Handle carriage return (CR)
    pub fn carriage_return(&mut self) {
        self.cursor_mut().carriage_return();
    }

    /// Handle backspace (BS)
    pub fn backspace(&mut self) {
        self.cursor_mut().pending_wrap = false;
        if self.cursor().col > 0 {
            self.cursor_mut().col -= 1;
        }
    }

    /// Handle horizontal tab (HT)
    pub fn tab(&mut self) {
        self.cursor_mut().pending_wrap = false;
        let col = self.cursor().col;

        // Find next tab stop
        for i in (col + 1)..self.cols {
            if self.tab_stops.get(i).copied().unwrap_or(false) {
                self.cursor_mut().col = i;
                return;
            }
        }

        // No tab stop found, go to last column
        self.cursor_mut().col = self.cols - 1;
    }

    /// Handle reverse index (RI) - move cursor up, scroll if at top
    pub fn reverse_index(&mut self) {
        self.cursor_mut().pending_wrap = false;
        let row = self.cursor().row;

        if row == self.scroll_top {
            self.scroll_down(1);
        } else if row > 0 {
            self.cursor_mut().row -= 1;
        }
    }

    /// Handle index (IND) - move cursor down, scroll if at bottom
    pub fn index(&mut self) {
        self.linefeed();
    }

    /// Handle next line (NEL) - move to start of next line
    pub fn next_line(&mut self) {
        self.linefeed();
        self.cursor_mut().col = 0;
    }

    /// Scroll the screen up by n lines (content moves up, new lines at bottom)
    pub fn scroll_up(&mut self, n: usize) {
        if n == 0 {
            return;
        }

        let top = self.scroll_top;
        let bottom = self.scroll_bottom;
        let cols = self.cols;

        // Move lines to scrollback (only if on primary screen and scrolling from top)
        if !self.on_alternate && top == 0 {
            for i in 0..n.min(bottom - top + 1) {
                if let Some(line) = self.primary_grid.get(top + i) {
                    self.scrollback.push(line.clone());
                }
            }
        }

        // Shift lines up within scroll region
        let grid = if self.on_alternate {
            &mut self.alternate_grid
        } else {
            &mut self.primary_grid
        };
        for i in top..=bottom {
            if i + n <= bottom {
                grid[i] = grid[i + n].clone();
            } else {
                grid[i] = Line::new(cols);
            }
        }

        // Mark affected lines dirty
        for i in top..=bottom {
            self.dirty_lines[i] = true;
        }
        self.dirty = true;
    }

    /// Scroll the screen down by n lines (content moves down, new lines at top)
    pub fn scroll_down(&mut self, n: usize) {
        if n == 0 {
            return;
        }

        let top = self.scroll_top;
        let bottom = self.scroll_bottom;
        let cols = self.cols;

        // Shift lines down within scroll region
        let grid = if self.on_alternate {
            &mut self.alternate_grid
        } else {
            &mut self.primary_grid
        };
        for i in (top..=bottom).rev() {
            if i >= top + n {
                grid[i] = grid[i - n].clone();
            } else {
                grid[i] = Line::new(cols);
            }
        }

        // Mark affected lines dirty
        for i in top..=bottom {
            self.dirty_lines[i] = true;
        }
        self.dirty = true;
    }

    /// Set scroll region (DECSTBM)
    pub fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        let top = top.min(self.rows - 1);
        let bottom = bottom.min(self.rows - 1);

        if top < bottom {
            self.scroll_top = top;
            self.scroll_bottom = bottom;
        } else {
            // Invalid region, reset to full screen
            self.scroll_top = 0;
            self.scroll_bottom = self.rows - 1;
        }

        // Move cursor to home position
        let scroll_top = self.scroll_top;
        if self.on_alternate {
            self.alternate_cursor.home(scroll_top);
        } else {
            self.primary_cursor.home(scroll_top);
        }
    }

    /// Reset scroll region to full screen
    pub fn reset_scroll_region(&mut self) {
        self.scroll_top = 0;
        self.scroll_bottom = self.rows - 1;
    }

    /// Set a tab stop at the current column
    pub fn set_tab_stop(&mut self) {
        let col = self.cursor().col;
        if col < self.tab_stops.len() {
            self.tab_stops[col] = true;
        }
    }

    /// Clear tab stop at current column
    pub fn clear_tab_stop(&mut self) {
        let col = self.cursor().col;
        if col < self.tab_stops.len() {
            self.tab_stops[col] = false;
        }
    }

    /// Clear all tab stops
    pub fn clear_all_tab_stops(&mut self) {
        for t in &mut self.tab_stops {
            *t = false;
        }
    }

    /// Erase in display (ED)
    pub fn erase_in_display(&mut self, mode: u32) {
        let bg = self.cursor().bg;
        let row = self.cursor().row;
        let col = self.cursor().col;
        let cols = self.cols;
        let rows = self.rows;

        let grid = if self.on_alternate {
            &mut self.alternate_grid
        } else {
            &mut self.primary_grid
        };

        match mode {
            0 => {
                // Erase from cursor to end of screen
                // Erase from cursor to end of current line
                if let Some(line) = grid.get_mut(row) {
                    for i in col..cols {
                        line.cells[i].erase(bg);
                    }
                }
                // Erase all lines below
                for r in (row + 1)..rows {
                    if let Some(line) = grid.get_mut(r) {
                        for cell in &mut line.cells {
                            cell.erase(bg);
                        }
                    }
                }
            }
            1 => {
                // Erase from start of screen to cursor
                // Erase all lines above
                for r in 0..row {
                    if let Some(line) = grid.get_mut(r) {
                        for cell in &mut line.cells {
                            cell.erase(bg);
                        }
                    }
                }
                // Erase from start of current line to cursor
                if let Some(line) = grid.get_mut(row) {
                    for i in 0..=col.min(cols.saturating_sub(1)) {
                        line.cells[i].erase(bg);
                    }
                }
            }
            2 => {
                // Erase entire screen
                for line in grid.iter_mut() {
                    for cell in &mut line.cells {
                        cell.erase(bg);
                    }
                }
            }
            3 => {
                // Erase scrollback (xterm extension)
                // Also erase screen
                for line in grid.iter_mut() {
                    for cell in &mut line.cells {
                        cell.erase(bg);
                    }
                }
            }
            _ => {}
        }

        // Handle scrollback clear separately
        if mode == 3 {
            self.scrollback.clear();
        }

        self.mark_all_dirty();
    }

    /// Erase in line (EL)
    pub fn erase_in_line(&mut self, mode: u32) {
        let bg = self.cursor().bg;
        let row = self.cursor().row;
        let col = self.cursor().col;
        let cols = self.cols;

        let grid = if self.on_alternate {
            &mut self.alternate_grid
        } else {
            &mut self.primary_grid
        };

        if let Some(line) = grid.get_mut(row) {
            match mode {
                0 => {
                    // Erase from cursor to end of line
                    for i in col..cols {
                        line.cells[i].erase(bg);
                    }
                }
                1 => {
                    // Erase from start of line to cursor
                    for i in 0..=col.min(cols.saturating_sub(1)) {
                        line.cells[i].erase(bg);
                    }
                }
                2 => {
                    // Erase entire line
                    for cell in &mut line.cells {
                        cell.erase(bg);
                    }
                }
                _ => {}
            }
        }
        self.mark_line_dirty(row);
    }

    /// Erase characters (ECH)
    pub fn erase_chars(&mut self, n: usize) {
        let bg = self.cursor().bg;
        let row = self.cursor().row;
        let col = self.cursor().col;
        let cols = self.cols;

        let grid = if self.on_alternate {
            &mut self.alternate_grid
        } else {
            &mut self.primary_grid
        };

        if let Some(line) = grid.get_mut(row) {
            for i in col..(col + n).min(cols) {
                line.cells[i].erase(bg);
            }
        }
        self.mark_line_dirty(row);
    }

    /// Insert lines (IL)
    pub fn insert_lines(&mut self, n: usize) {
        if n == 0 {
            return;
        }

        let row = self.cursor().row;
        let scroll_bottom = self.scroll_bottom;
        let cols = self.cols;

        // Only works within scroll region
        if row < self.scroll_top || row > scroll_bottom {
            return;
        }

        // Shift lines down
        let grid = if self.on_alternate {
            &mut self.alternate_grid
        } else {
            &mut self.primary_grid
        };
        for i in (row..=scroll_bottom).rev() {
            if i >= row + n {
                grid[i] = grid[i - n].clone();
            } else {
                grid[i] = Line::new(cols);
            }
        }

        for i in row..=scroll_bottom {
            self.dirty_lines[i] = true;
        }
        self.dirty = true;
    }

    /// Delete lines (DL)
    pub fn delete_lines(&mut self, n: usize) {
        if n == 0 {
            return;
        }

        let row = self.cursor().row;
        let scroll_bottom = self.scroll_bottom;
        let cols = self.cols;

        // Only works within scroll region
        if row < self.scroll_top || row > scroll_bottom {
            return;
        }

        // Shift lines up
        let grid = if self.on_alternate {
            &mut self.alternate_grid
        } else {
            &mut self.primary_grid
        };
        for i in row..=scroll_bottom {
            if i + n <= scroll_bottom {
                grid[i] = grid[i + n].clone();
            } else {
                grid[i] = Line::new(cols);
            }
        }

        for i in row..=scroll_bottom {
            self.dirty_lines[i] = true;
        }
        self.dirty = true;
    }

    /// Insert blank characters (ICH)
    pub fn insert_chars(&mut self, n: usize) {
        if n == 0 {
            return;
        }

        let row = self.cursor().row;
        let col = self.cursor().col;

        self.insert_blank_cells(col, row, n);
        self.mark_line_dirty(row);
    }

    /// Delete characters (DCH)
    pub fn delete_chars(&mut self, n: usize) {
        if n == 0 {
            return;
        }

        let row = self.cursor().row;
        let col = self.cursor().col;
        let cols = self.cols;

        let grid = if self.on_alternate {
            &mut self.alternate_grid
        } else {
            &mut self.primary_grid
        };

        if let Some(line) = grid.get_mut(row) {
            // Shift cells left
            for i in col..cols {
                if i + n < cols {
                    line.cells[i] = line.cells[i + n].clone();
                } else {
                    line.cells[i].clear();
                }
            }
        }
        self.mark_line_dirty(row);
    }

    /// Move cursor to position (CUP/HVP)
    pub fn move_cursor_to(&mut self, row: usize, col: usize) {
        let row = if self.cursor().origin_mode {
            (self.scroll_top + row).min(self.scroll_bottom)
        } else {
            row.min(self.rows - 1)
        };
        let col = col.min(self.cols - 1);

        self.cursor_mut().row = row;
        self.cursor_mut().col = col;
        self.cursor_mut().pending_wrap = false;
    }

    /// Move cursor up (CUU)
    pub fn move_cursor_up(&mut self, n: usize) {
        let scroll_top = self.scroll_top;
        if self.on_alternate {
            self.alternate_cursor.move_up(n, scroll_top);
        } else {
            self.primary_cursor.move_up(n, scroll_top);
        }
    }

    /// Move cursor down (CUD)
    pub fn move_cursor_down(&mut self, n: usize) {
        let scroll_bottom = self.scroll_bottom;
        let rows = self.rows;
        if self.on_alternate {
            self.alternate_cursor.move_down(n, scroll_bottom, rows);
        } else {
            self.primary_cursor.move_down(n, scroll_bottom, rows);
        }
    }

    /// Move cursor forward/right (CUF)
    pub fn move_cursor_forward(&mut self, n: usize) {
        let cols = self.cols;
        if self.on_alternate {
            self.alternate_cursor.move_right(n, cols);
        } else {
            self.primary_cursor.move_right(n, cols);
        }
    }

    /// Move cursor backward/left (CUB)
    pub fn move_cursor_backward(&mut self, n: usize) {
        if self.on_alternate {
            self.alternate_cursor.move_left(n);
        } else {
            self.primary_cursor.move_left(n);
        }
    }

    /// Move cursor to column (CHA)
    pub fn move_cursor_to_col(&mut self, col: usize) {
        let cols = self.cols;
        if self.on_alternate {
            self.alternate_cursor.set_col(col, cols);
        } else {
            self.primary_cursor.set_col(col, cols);
        }
    }

    /// Move cursor to row (VPA)
    pub fn move_cursor_to_row(&mut self, row: usize) {
        let rows = self.rows;
        let scroll_top = self.scroll_top;
        let scroll_bottom = self.scroll_bottom;
        if self.on_alternate {
            self.alternate_cursor
                .set_row(row, rows, scroll_top, scroll_bottom);
        } else {
            self.primary_cursor
                .set_row(row, rows, scroll_top, scroll_bottom);
        }
    }

    /// Save cursor state (DECSC)
    pub fn save_cursor(&mut self) {
        let saved = self.cursor().save();
        if self.on_alternate {
            self.alternate_saved_cursor = saved;
        } else {
            self.primary_saved_cursor = saved;
        }
    }

    /// Restore cursor state (DECRC)
    pub fn restore_cursor(&mut self) {
        let saved = if self.on_alternate {
            self.alternate_saved_cursor.clone()
        } else {
            self.primary_saved_cursor.clone()
        };
        let cols = self.cols;
        let rows = self.rows;
        if self.on_alternate {
            self.alternate_cursor.restore(&saved, cols, rows);
        } else {
            self.primary_cursor.restore(&saved, cols, rows);
        }
    }

    /// Switch to alternate screen
    pub fn enter_alternate_screen(&mut self) {
        if self.on_alternate {
            return;
        }

        self.on_alternate = true;
        self.modes.alternate_screen = true;

        // Clear alternate screen
        for line in &mut self.alternate_grid {
            line.clear();
        }

        self.mark_all_dirty();
    }

    /// Switch back to primary screen
    pub fn exit_alternate_screen(&mut self) {
        if !self.on_alternate {
            return;
        }

        self.on_alternate = false;
        self.modes.alternate_screen = false;
        self.mark_all_dirty();
    }

    /// Resize the screen
    pub fn resize(&mut self, new_cols: usize, new_rows: usize) {
        if new_cols == self.cols && new_rows == self.rows {
            return;
        }

        // Resize grids
        resize_grid_helper(&mut self.primary_grid, new_cols, new_rows);
        resize_grid_helper(&mut self.alternate_grid, new_cols, new_rows);

        // Update dimensions
        self.cols = new_cols;
        self.rows = new_rows;

        // Reset scroll region
        self.scroll_top = 0;
        self.scroll_bottom = new_rows.saturating_sub(1);

        // Resize tab stops
        self.tab_stops.resize(new_cols, false);
        for i in (8..new_cols).step_by(8) {
            self.tab_stops[i] = true;
        }

        // Clamp cursor positions
        self.primary_cursor.col = self.primary_cursor.col.min(new_cols.saturating_sub(1));
        self.primary_cursor.row = self.primary_cursor.row.min(new_rows.saturating_sub(1));
        self.alternate_cursor.col = self.alternate_cursor.col.min(new_cols.saturating_sub(1));
        self.alternate_cursor.row = self.alternate_cursor.row.min(new_rows.saturating_sub(1));

        // Resize dirty lines
        self.dirty_lines.resize(new_rows, true);
        self.mark_all_dirty();
    }

    /// Register a hyperlink and return its ID
    pub fn register_hyperlink(&mut self, url: String) -> u32 {
        let id = self.next_hyperlink_id;
        self.next_hyperlink_id += 1;
        self.hyperlinks.push(url);
        id
    }

    /// Get a hyperlink URL by ID
    pub fn get_hyperlink(&self, id: u32) -> Option<&str> {
        if id == 0 || id as usize > self.hyperlinks.len() {
            None
        } else {
            self.hyperlinks.get(id as usize - 1).map(|s| s.as_str())
        }
    }
}

/// Helper function to resize a grid (outside impl to avoid borrow checker issues)
fn resize_grid_helper(grid: &mut Vec<Line>, new_cols: usize, new_rows: usize) {
    // Resize existing lines
    for line in grid.iter_mut() {
        line.resize(new_cols);
    }

    // Add or remove lines
    if new_rows > grid.len() {
        for _ in grid.len()..new_rows {
            grid.push(Line::new(new_cols));
        }
    } else {
        grid.truncate(new_rows);
    }
}

impl Screen {
    /// Reset the terminal to initial state
    pub fn reset(&mut self) {
        // Clear grids
        for line in &mut self.primary_grid {
            line.clear();
        }
        for line in &mut self.alternate_grid {
            line.clear();
        }

        // Reset cursors
        self.primary_cursor.reset();
        self.alternate_cursor.reset();
        self.primary_saved_cursor = SavedCursor::default();
        self.alternate_saved_cursor = SavedCursor::default();

        // Reset scroll region
        self.scroll_top = 0;
        self.scroll_bottom = self.rows - 1;

        // Reset tab stops
        for (i, t) in self.tab_stops.iter_mut().enumerate() {
            *t = i > 0 && i % 8 == 0;
        }

        // Reset modes
        self.modes = Modes::default();
        self.on_alternate = false;

        // Clear title
        self.title.clear();

        // Clear scrollback
        self.scrollback.clear();

        self.mark_all_dirty();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_new() {
        let screen = Screen::new(80, 24, 1000);
        assert_eq!(screen.cols(), 80);
        assert_eq!(screen.rows(), 24);
        assert_eq!(screen.cursor().col, 0);
        assert_eq!(screen.cursor().row, 0);
    }

    #[test]
    fn test_print_char() {
        let mut screen = Screen::new(80, 24, 1000);
        screen.print_char('H');
        screen.print_char('i');

        assert_eq!(screen.get_cell(0, 0).unwrap().display_char(), 'H');
        assert_eq!(screen.get_cell(1, 0).unwrap().display_char(), 'i');
        assert_eq!(screen.cursor().col, 2);
    }

    #[test]
    fn test_autowrap() {
        let mut screen = Screen::new(5, 3, 1000);
        for c in "Hello World".chars() {
            screen.print_char(c);
        }

        // "Hello" on first line, " Worl" on second, "d" on third
        assert_eq!(screen.get_line(0).unwrap().to_string(), "Hello");
        assert_eq!(screen.get_line(1).unwrap().to_string(), " Worl");
        assert_eq!(screen.get_line(2).unwrap().to_string(), "d");
    }

    #[test]
    fn test_linefeed_and_scroll() {
        let mut screen = Screen::new(80, 3, 1000);

        // Fill screen
        screen.print_char('1');
        screen.linefeed();
        screen.carriage_return();
        screen.print_char('2');
        screen.linefeed();
        screen.carriage_return();
        screen.print_char('3');

        // Now at bottom, linefeed should scroll
        screen.linefeed();
        screen.carriage_return();
        screen.print_char('4');

        assert_eq!(screen.get_line(0).unwrap().to_string(), "2");
        assert_eq!(screen.get_line(1).unwrap().to_string(), "3");
        assert_eq!(screen.get_line(2).unwrap().to_string(), "4");

        // Check scrollback
        assert_eq!(screen.scrollback().len(), 1);
        assert_eq!(screen.scrollback().get(0).unwrap().to_string(), "1");
    }

    #[test]
    fn test_erase_in_display() {
        let mut screen = Screen::new(10, 3, 1000);

        // Fill with X's
        for _ in 0..3 {
            for _ in 0..10 {
                screen.print_char('X');
            }
        }

        // Move to middle and erase to end
        screen.move_cursor_to(1, 5);
        screen.erase_in_display(0);

        assert_eq!(screen.get_line(0).unwrap().to_string(), "XXXXXXXXXX");
        assert_eq!(screen.get_line(1).unwrap().to_string(), "XXXXX");
        assert_eq!(screen.get_line(2).unwrap().to_string(), "");
    }

    #[test]
    fn test_erase_in_line() {
        let mut screen = Screen::new(10, 1, 1000);

        for c in "ABCDEFGHIJ".chars() {
            screen.print_char(c);
        }

        screen.move_cursor_to(0, 5);
        screen.erase_in_line(0); // Erase to end

        assert_eq!(screen.get_line(0).unwrap().to_string(), "ABCDE");
    }

    #[test]
    fn test_scroll_region() {
        let mut screen = Screen::new(80, 5, 1000);

        // Set scroll region to lines 1-3 (0-indexed)
        screen.set_scroll_region(1, 3);

        // Move to bottom of scroll region
        screen.move_cursor_to(3, 0);
        screen.print_char('A');
        screen.linefeed();
        screen.carriage_return();
        screen.print_char('B');

        // Line 0 should be unaffected
        // Lines 1-3 should have scrolled
        assert_eq!(screen.scroll_top(), 1);
        assert_eq!(screen.scroll_bottom(), 3);
    }

    #[test]
    fn test_insert_delete_lines() {
        let mut screen = Screen::new(10, 5, 1000);

        // Put content on each line
        for i in 0..5 {
            screen.move_cursor_to(i, 0);
            screen.print_char(char::from_digit(i as u32, 10).unwrap());
        }

        // Insert 2 lines at row 2
        screen.move_cursor_to(2, 0);
        screen.insert_lines(2);

        assert_eq!(screen.get_line(0).unwrap().to_string(), "0");
        assert_eq!(screen.get_line(1).unwrap().to_string(), "1");
        assert_eq!(screen.get_line(2).unwrap().to_string(), "");
        assert_eq!(screen.get_line(3).unwrap().to_string(), "");
        assert_eq!(screen.get_line(4).unwrap().to_string(), "2");
    }

    #[test]
    fn test_insert_delete_chars() {
        let mut screen = Screen::new(10, 1, 1000);

        for c in "ABCDEFGHIJ".chars() {
            screen.print_char(c);
        }

        // Insert 2 chars at position 3
        screen.move_cursor_to(0, 3);
        screen.insert_chars(2);

        assert_eq!(screen.get_line(0).unwrap().to_string(), "ABC  DEFGH");

        // Delete 2 chars at position 3
        screen.delete_chars(2);

        assert_eq!(screen.get_line(0).unwrap().to_string(), "ABCDEFGH");
    }

    #[test]
    fn test_alternate_screen() {
        let mut screen = Screen::new(80, 24, 1000);

        screen.print_char('P'); // Primary screen

        screen.enter_alternate_screen();
        screen.print_char('A'); // Alternate screen

        assert_eq!(screen.get_cell(0, 0).unwrap().display_char(), 'A');

        screen.exit_alternate_screen();

        assert_eq!(screen.get_cell(0, 0).unwrap().display_char(), 'P');
    }

    #[test]
    fn test_resize() {
        let mut screen = Screen::new(80, 24, 1000);
        screen.print_char('X');
        screen.move_cursor_to(23, 79);

        screen.resize(40, 12);

        assert_eq!(screen.cols(), 40);
        assert_eq!(screen.rows(), 12);
        assert!(screen.cursor().col < 40);
        assert!(screen.cursor().row < 12);
        assert_eq!(screen.get_cell(0, 0).unwrap().display_char(), 'X');
    }

    #[test]
    fn test_tab_stops() {
        let mut screen = Screen::new(80, 24, 1000);

        screen.tab();
        assert_eq!(screen.cursor().col, 8);

        screen.tab();
        assert_eq!(screen.cursor().col, 16);

        // Clear all and set custom
        screen.clear_all_tab_stops();
        screen.move_cursor_to(0, 5);
        screen.set_tab_stop();

        screen.move_cursor_to(0, 0);
        screen.tab();
        assert_eq!(screen.cursor().col, 5);
    }

    #[test]
    fn test_save_restore_cursor() {
        let mut screen = Screen::new(80, 24, 1000);

        screen.move_cursor_to(10, 20);
        screen.cursor_mut().style.bold = true;
        screen.save_cursor();

        screen.move_cursor_to(0, 0);
        screen.cursor_mut().style.bold = false;

        screen.restore_cursor();

        assert_eq!(screen.cursor().row, 10);
        assert_eq!(screen.cursor().col, 20);
        assert!(screen.cursor().style.bold);
    }
}
