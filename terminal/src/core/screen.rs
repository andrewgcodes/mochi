//! Terminal screen implementation
//!
//! The screen manages the visible terminal grid, cursor, modes, and scrollback.

use serde::{Deserialize, Serialize};

use super::cell::{Cell, CellAttributes, Color};
use super::cursor::{Cursor, SavedCursor};
use super::line::Line;
use super::modes::Modes;
use super::scrollback::Scrollback;
use crate::parser::TerminalAction;

/// Which buffer is currently active
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BufferType {
    #[default]
    Primary,
    Alternate,
}

/// A grid of lines representing the terminal buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    lines: Vec<Line>,
    cols: usize,
    rows: usize,
}

impl Grid {
    /// Create a new grid with the specified dimensions
    pub fn new(cols: usize, rows: usize) -> Self {
        let lines = (0..rows).map(|_| Line::new(cols)).collect();
        Self { lines, cols, rows }
    }

    /// Get the number of columns
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Get the number of rows
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Get a reference to a line
    pub fn line(&self, row: usize) -> Option<&Line> {
        self.lines.get(row)
    }

    /// Get a mutable reference to a line
    pub fn line_mut(&mut self, row: usize) -> Option<&mut Line> {
        self.lines.get_mut(row)
    }

    /// Get a reference to a cell
    pub fn cell(&self, row: usize, col: usize) -> Option<&Cell> {
        self.lines.get(row).and_then(|line| line.cell(col))
    }

    /// Get a mutable reference to a cell
    pub fn cell_mut(&mut self, row: usize, col: usize) -> Option<&mut Cell> {
        self.lines.get_mut(row).and_then(|line| line.cell_mut(col))
    }

    /// Resize the grid
    pub fn resize(&mut self, cols: usize, rows: usize) {
        // Resize existing lines
        for line in &mut self.lines {
            line.resize(cols);
        }

        // Add or remove rows
        use std::cmp::Ordering;
        match rows.cmp(&self.rows) {
            Ordering::Greater => {
                for _ in self.rows..rows {
                    self.lines.push(Line::new(cols));
                }
            },
            Ordering::Less => {
                self.lines.truncate(rows);
            },
            Ordering::Equal => {},
        }

        self.cols = cols;
        self.rows = rows;
    }

    /// Clear the entire grid
    pub fn clear(&mut self, bg: Color) {
        for line in &mut self.lines {
            line.clear_with_bg(bg);
        }
    }

    /// Scroll the grid up by one line within the given region
    /// Returns the line that was scrolled out (for scrollback)
    pub fn scroll_up(&mut self, top: usize, bottom: usize, bg: Color) -> Option<Line> {
        if top >= bottom || bottom > self.rows {
            return None;
        }

        // Remove the top line
        let scrolled_out = self.lines.remove(top);

        // Insert a new blank line at the bottom of the region
        self.lines.insert(bottom, Line::new(self.cols));
        self.lines[bottom].clear_with_bg(bg);

        Some(scrolled_out)
    }

    /// Scroll the grid down by one line within the given region
    pub fn scroll_down(&mut self, top: usize, bottom: usize, bg: Color) {
        if top >= bottom || bottom > self.rows {
            return;
        }

        // Remove the bottom line
        self.lines.remove(bottom);

        // Insert a new blank line at the top of the region
        let mut new_line = Line::new(self.cols);
        new_line.clear_with_bg(bg);
        self.lines.insert(top, new_line);
    }

    /// Insert blank lines at the given row
    pub fn insert_lines(&mut self, row: usize, count: usize, bottom: usize, bg: Color) {
        if row >= bottom {
            return;
        }

        let count = count.min(bottom - row);

        for _ in 0..count {
            // Remove line at bottom of scroll region
            if bottom < self.lines.len() {
                self.lines.remove(bottom);
            }
            // Insert blank line at current row
            let mut new_line = Line::new(self.cols);
            new_line.clear_with_bg(bg);
            self.lines.insert(row, new_line);
        }
    }

    /// Delete lines at the given row
    pub fn delete_lines(&mut self, row: usize, count: usize, bottom: usize, bg: Color) {
        if row >= bottom {
            return;
        }

        let count = count.min(bottom - row);

        for _ in 0..count {
            // Remove line at current row
            self.lines.remove(row);
            // Insert blank line at bottom of scroll region
            let mut new_line = Line::new(self.cols);
            new_line.clear_with_bg(bg);
            self.lines.insert(bottom, new_line);
        }
    }
}

/// The terminal screen state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screen {
    /// Primary screen buffer
    primary: Grid,
    /// Alternate screen buffer (for full-screen apps)
    alternate: Grid,
    /// Which buffer is active
    active_buffer: BufferType,
    /// Scrollback history (primary screen only)
    scrollback: Scrollback,
    /// Cursor state
    cursor: Cursor,
    /// Saved cursor for primary screen (DECSC/DECRC)
    saved_cursor_primary: Option<SavedCursor>,
    /// Saved cursor for alternate screen
    saved_cursor_alternate: Option<SavedCursor>,
    /// Terminal modes
    modes: Modes,
    /// Current SGR attributes for new characters
    current_attrs: CellAttributes,
    /// Current foreground color
    current_fg: Color,
    /// Current background color
    current_bg: Color,
    /// Tab stops (true = tab stop at this column)
    tab_stops: Vec<bool>,
    /// Scroll region top (0-indexed, inclusive)
    scroll_top: usize,
    /// Scroll region bottom (0-indexed, inclusive)
    scroll_bottom: usize,
    /// Terminal width in columns
    cols: usize,
    /// Terminal height in rows
    rows: usize,
    /// Window title (set via OSC)
    title: String,
    /// Current hyperlink ID counter
    next_hyperlink_id: u32,
    /// Active hyperlinks (id -> url)
    hyperlinks: std::collections::HashMap<u32, String>,
}

impl Screen {
    /// Create a new screen with the specified dimensions
    pub fn new(cols: usize, rows: usize) -> Self {
        let mut tab_stops = vec![false; cols];
        // Set default tab stops every 8 columns
        for i in (0..cols).step_by(8) {
            tab_stops[i] = true;
        }

        Self {
            primary: Grid::new(cols, rows),
            alternate: Grid::new(cols, rows),
            active_buffer: BufferType::Primary,
            scrollback: Scrollback::default(),
            cursor: Cursor::new(),
            saved_cursor_primary: None,
            saved_cursor_alternate: None,
            modes: Modes::new(),
            current_attrs: CellAttributes::default(),
            current_fg: Color::Default,
            current_bg: Color::Default,
            tab_stops,
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            cols,
            rows,
            title: String::new(),
            next_hyperlink_id: 1,
            hyperlinks: std::collections::HashMap::new(),
        }
    }

    /// Get the current grid (primary or alternate)
    pub fn grid(&self) -> &Grid {
        match self.active_buffer {
            BufferType::Primary => &self.primary,
            BufferType::Alternate => &self.alternate,
        }
    }

    /// Get the current grid mutably
    fn grid_mut(&mut self) -> &mut Grid {
        match self.active_buffer {
            BufferType::Primary => &mut self.primary,
            BufferType::Alternate => &mut self.alternate,
        }
    }

    /// Get the cursor
    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    /// Get the cursor mutably
    pub fn cursor_mut(&mut self) -> &mut Cursor {
        &mut self.cursor
    }

    /// Get the modes
    pub fn modes(&self) -> &Modes {
        &self.modes
    }

    /// Get the modes mutably
    pub fn modes_mut(&mut self) -> &mut Modes {
        &mut self.modes
    }

    /// Get the scrollback buffer
    pub fn scrollback(&self) -> &Scrollback {
        &self.scrollback
    }

    /// Get the number of columns
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Get the number of rows
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Get the window title
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the window title
    pub fn set_title(&mut self, title: String) {
        // Limit title length for security
        self.title = title.chars().take(256).collect();
    }

    /// Get the current foreground color
    pub fn current_fg(&self) -> Color {
        self.current_fg
    }

    /// Get the current background color
    pub fn current_bg(&self) -> Color {
        self.current_bg
    }

    /// Get the current attributes
    pub fn current_attrs(&self) -> &CellAttributes {
        &self.current_attrs
    }

    /// Resize the screen
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.primary.resize(cols, rows);
        self.alternate.resize(cols, rows);

        // Update tab stops
        self.tab_stops.resize(cols, false);
        for i in (0..cols).step_by(8) {
            self.tab_stops[i] = true;
        }

        // Clamp cursor position
        self.cursor
            .set_row(self.cursor.row().min(rows.saturating_sub(1)));
        self.cursor
            .set_col(self.cursor.col().min(cols.saturating_sub(1)));

        // Update scroll region
        self.scroll_top = 0;
        self.scroll_bottom = rows.saturating_sub(1);

        self.cols = cols;
        self.rows = rows;
    }

    /// Apply a terminal action to the screen
    pub fn apply(&mut self, action: TerminalAction) {
        match action {
            TerminalAction::Print(c) => self.print(c),
            TerminalAction::Execute(byte) => self.execute(byte),
            TerminalAction::CsiDispatch {
                params,
                intermediates,
                final_byte,
            } => {
                self.csi_dispatch(&params, &intermediates, final_byte);
            },
            TerminalAction::EscDispatch {
                intermediates,
                final_byte,
            } => {
                self.esc_dispatch(&intermediates, final_byte);
            },
            TerminalAction::OscDispatch { params } => {
                self.osc_dispatch(&params);
            },
            _ => {
                // DCS and other sequences - log and ignore for now
                tracing::debug!("Unhandled action: {:?}", action);
            },
        }
    }

    /// Print a character at the current cursor position
    fn print(&mut self, c: char) {
        // Handle pending wrap
        if self.cursor.pending_wrap() && self.modes.autowrap {
            self.cursor.set_col(0);
            self.linefeed();
            self.cursor.set_pending_wrap(false);
        }

        let row = self.cursor.row();
        let col = self.cursor.col();

        // Get character width
        let width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);

        // Copy values before mutable borrow
        let current_fg = self.current_fg;
        let current_bg = self.current_bg;
        let current_attrs = self.current_attrs;
        let cols = self.cols;
        let insert_mode = self.modes.insert;
        let autowrap = self.modes.autowrap;

        // Handle insert mode
        if insert_mode && width > 0 {
            if let Some(line) = self.grid_mut().line_mut(row) {
                line.insert_cells(col, width, current_bg);
            }
        }

        // Write the character
        if let Some(cell) = self.grid_mut().cell_mut(row, col) {
            cell.set_content(c);
            cell.set_fg(current_fg);
            cell.set_bg(current_bg);
            cell.set_attrs(current_attrs);
            cell.set_width(width as u8);
        }

        // For wide characters, clear the next cell
        if width == 2 && col + 1 < cols {
            if let Some(cell) = self.grid_mut().cell_mut(row, col + 1) {
                cell.clear();
                cell.set_width(0); // Continuation cell
                cell.set_bg(current_bg);
            }
        }

        // Move cursor
        let new_col = col + width;
        if new_col >= cols {
            // At right margin
            if autowrap {
                self.cursor.set_col(cols - 1);
                self.cursor.set_pending_wrap(true);
            }
        } else {
            self.cursor.set_col(new_col);
        }
    }

    /// Execute a C0 control character
    fn execute(&mut self, byte: u8) {
        match byte {
            0x07 => {
                // BEL - Bell
                tracing::debug!("Bell");
            },
            0x08 => {
                // BS - Backspace
                self.cursor.move_left(1);
            },
            0x09 => {
                // HT - Horizontal Tab
                self.tab();
            },
            0x0A..=0x0C => {
                // LF, VT, FF - Line Feed
                self.linefeed();
                if self.modes.linefeed_newline {
                    self.cursor.set_col(0);
                }
            },
            0x0D => {
                // CR - Carriage Return
                self.cursor.set_col(0);
                self.cursor.set_pending_wrap(false);
            },
            0x0E => {
                // SO - Shift Out (switch to G1)
                // TODO: Character set switching
            },
            0x0F => {
                // SI - Shift In (switch to G0)
                // TODO: Character set switching
            },
            _ => {
                // Ignore other control characters
            },
        }
    }

    /// Move to the next tab stop
    fn tab(&mut self) {
        let col = self.cursor.col();
        for i in (col + 1)..self.cols {
            if self.tab_stops.get(i).copied().unwrap_or(false) {
                self.cursor.set_col(i);
                return;
            }
        }
        // No tab stop found, move to last column
        self.cursor.set_col(self.cols - 1);
    }

    /// Perform a line feed
    fn linefeed(&mut self) {
        let row = self.cursor.row();

        if row == self.scroll_bottom {
            // At bottom of scroll region, scroll up
            self.scroll_up();
        } else if row < self.rows - 1 {
            // Not at bottom, just move down
            self.cursor.set_row(row + 1);
        }
    }

    /// Scroll the screen up by one line
    fn scroll_up(&mut self) {
        // Copy values before mutable borrow
        let scroll_top = self.scroll_top;
        let scroll_bottom = self.scroll_bottom;
        let current_bg = self.current_bg;
        let is_primary = self.active_buffer == BufferType::Primary;

        if let Some(line) = self
            .grid_mut()
            .scroll_up(scroll_top, scroll_bottom, current_bg)
        {
            // Add to scrollback if on primary screen
            if is_primary {
                self.scrollback.push(line);
            }
        }
    }

    /// Scroll the screen down by one line
    fn scroll_down(&mut self) {
        // Copy values before mutable borrow
        let scroll_top = self.scroll_top;
        let scroll_bottom = self.scroll_bottom;
        let current_bg = self.current_bg;

        self.grid_mut()
            .scroll_down(scroll_top, scroll_bottom, current_bg);
    }

    /// Handle CSI sequence
    fn csi_dispatch(&mut self, params: &[u16], intermediates: &[u8], final_byte: u8) {
        let is_private = intermediates.first() == Some(&b'?');

        match final_byte {
            b'A' => {
                // CUU - Cursor Up
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                let min_row = if self.modes.origin {
                    self.scroll_top
                } else {
                    0
                };
                let new_row = self.cursor.row().saturating_sub(n).max(min_row);
                self.cursor.set_row(new_row);
            },
            b'B' => {
                // CUD - Cursor Down
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                let max_row = if self.modes.origin {
                    self.scroll_bottom
                } else {
                    self.rows - 1
                };
                let new_row = (self.cursor.row() + n).min(max_row);
                self.cursor.set_row(new_row);
            },
            b'C' => {
                // CUF - Cursor Forward
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                self.cursor.move_right(n, self.cols - 1);
            },
            b'D' => {
                // CUB - Cursor Back
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                self.cursor.move_left(n);
            },
            b'E' => {
                // CNL - Cursor Next Line
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                self.cursor.set_col(0);
                let max_row = if self.modes.origin {
                    self.scroll_bottom
                } else {
                    self.rows - 1
                };
                let new_row = (self.cursor.row() + n).min(max_row);
                self.cursor.set_row(new_row);
            },
            b'F' => {
                // CPL - Cursor Previous Line
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                self.cursor.set_col(0);
                let min_row = if self.modes.origin {
                    self.scroll_top
                } else {
                    0
                };
                let new_row = self.cursor.row().saturating_sub(n).max(min_row);
                self.cursor.set_row(new_row);
            },
            b'G' => {
                // CHA - Cursor Horizontal Absolute
                let col = params.first().copied().unwrap_or(1).max(1) as usize - 1;
                self.cursor.set_col(col.min(self.cols - 1));
            },
            b'H' | b'f' => {
                // CUP/HVP - Cursor Position
                let row = params.first().copied().unwrap_or(1).max(1) as usize - 1;
                let col = params.get(1).copied().unwrap_or(1).max(1) as usize - 1;

                let (_min_row, max_row) = if self.modes.origin {
                    (self.scroll_top, self.scroll_bottom)
                } else {
                    (0, self.rows - 1)
                };

                let actual_row = if self.modes.origin {
                    (self.scroll_top + row).min(max_row)
                } else {
                    row.min(max_row)
                };

                self.cursor.set_position(actual_row, col.min(self.cols - 1));
            },
            b'J' => {
                // ED - Erase in Display
                let mode = params.first().copied().unwrap_or(0);
                self.erase_in_display(mode);
            },
            b'K' => {
                // EL - Erase in Line
                let mode = params.first().copied().unwrap_or(0);
                self.erase_in_line(mode);
            },
            b'L' => {
                // IL - Insert Lines
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                let row = self.cursor.row();
                let scroll_bottom = self.scroll_bottom;
                let current_bg = self.current_bg;
                if row >= self.scroll_top && row <= scroll_bottom {
                    self.grid_mut()
                        .insert_lines(row, n, scroll_bottom, current_bg);
                }
            },
            b'M' => {
                // DL - Delete Lines
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                let row = self.cursor.row();
                let scroll_bottom = self.scroll_bottom;
                let current_bg = self.current_bg;
                if row >= self.scroll_top && row <= scroll_bottom {
                    self.grid_mut()
                        .delete_lines(row, n, scroll_bottom, current_bg);
                }
            },
            b'P' => {
                // DCH - Delete Characters
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                let row = self.cursor.row();
                let col = self.cursor.col();
                let current_bg = self.current_bg;
                if let Some(line) = self.grid_mut().line_mut(row) {
                    line.delete_cells(col, n, current_bg);
                }
            },
            b'S' => {
                // SU - Scroll Up
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                for _ in 0..n {
                    self.scroll_up();
                }
            },
            b'T' => {
                // SD - Scroll Down
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                for _ in 0..n {
                    self.scroll_down();
                }
            },
            b'X' => {
                // ECH - Erase Characters
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                let row = self.cursor.row();
                let col = self.cursor.col();
                let current_bg = self.current_bg;
                if let Some(line) = self.grid_mut().line_mut(row) {
                    line.erase_chars(col, n, current_bg);
                }
            },
            b'@' => {
                // ICH - Insert Characters
                let n = params.first().copied().unwrap_or(1).max(1) as usize;
                let row = self.cursor.row();
                let col = self.cursor.col();
                let current_bg = self.current_bg;
                if let Some(line) = self.grid_mut().line_mut(row) {
                    line.insert_cells(col, n, current_bg);
                }
            },
            b'd' => {
                // VPA - Vertical Position Absolute
                let row = params.first().copied().unwrap_or(1).max(1) as usize - 1;
                let (_min_row, max_row) = if self.modes.origin {
                    (self.scroll_top, self.scroll_bottom)
                } else {
                    (0, self.rows - 1)
                };
                let actual_row = if self.modes.origin {
                    (self.scroll_top + row).min(max_row)
                } else {
                    row.min(max_row)
                };
                self.cursor.set_row(actual_row);
            },
            b'g' => {
                // TBC - Tab Clear
                let mode = params.first().copied().unwrap_or(0);
                match mode {
                    0 => {
                        // Clear tab stop at current column
                        let col = self.cursor.col();
                        if col < self.tab_stops.len() {
                            self.tab_stops[col] = false;
                        }
                    },
                    3 => {
                        // Clear all tab stops
                        self.tab_stops.fill(false);
                    },
                    _ => {},
                }
            },
            b'h' => {
                // SM - Set Mode
                for &param in params {
                    if is_private {
                        self.set_dec_mode(param);
                    } else {
                        self.modes.set_ansi_mode(param);
                    }
                }
            },
            b'l' => {
                // RM - Reset Mode
                for &param in params {
                    if is_private {
                        self.reset_dec_mode(param);
                    } else {
                        self.modes.reset_ansi_mode(param);
                    }
                }
            },
            b'm' => {
                // SGR - Select Graphic Rendition
                self.sgr(params);
            },
            b'n' => {
                // DSR - Device Status Report
                // This requires writing back to the PTY, handled by caller
                tracing::debug!("DSR request: {:?}", params);
            },
            b'r' => {
                // DECSTBM - Set Top and Bottom Margins
                let top = params.first().copied().unwrap_or(1).max(1) as usize - 1;
                let bottom = params.get(1).copied().unwrap_or(self.rows as u16).max(1) as usize - 1;

                if top < bottom && bottom < self.rows {
                    self.scroll_top = top;
                    self.scroll_bottom = bottom;

                    // Move cursor to home position
                    if self.modes.origin {
                        self.cursor.set_position(self.scroll_top, 0);
                    } else {
                        self.cursor.set_position(0, 0);
                    }
                }
            },
            b's' => {
                // SCOSC - Save Cursor Position
                self.save_cursor();
            },
            b'u' => {
                // SCORC - Restore Cursor Position
                self.restore_cursor();
            },
            b'c' => {
                // DA - Device Attributes
                // This requires writing back to the PTY, handled by caller
                tracing::debug!("DA request");
            },
            _ => {
                tracing::debug!(
                    "Unhandled CSI: params={:?}, intermediates={:?}, final={}",
                    params,
                    intermediates,
                    final_byte as char
                );
            },
        }
    }

    /// Handle ESC sequence
    fn esc_dispatch(&mut self, intermediates: &[u8], final_byte: u8) {
        match (intermediates, final_byte) {
            ([], b'7') => {
                // DECSC - Save Cursor
                self.save_cursor();
            },
            ([], b'8') => {
                // DECRC - Restore Cursor
                self.restore_cursor();
            },
            ([], b'D') => {
                // IND - Index (move down, scroll if at bottom)
                self.linefeed();
            },
            ([], b'E') => {
                // NEL - Next Line
                self.cursor.set_col(0);
                self.linefeed();
            },
            ([], b'H') => {
                // HTS - Horizontal Tab Set
                let col = self.cursor.col();
                if col < self.tab_stops.len() {
                    self.tab_stops[col] = true;
                }
            },
            ([], b'M') => {
                // RI - Reverse Index (move up, scroll if at top)
                let row = self.cursor.row();
                if row == self.scroll_top {
                    self.scroll_down();
                } else if row > 0 {
                    self.cursor.set_row(row - 1);
                }
            },
            ([], b'c') => {
                // RIS - Reset to Initial State
                self.reset();
            },
            ([], b'=') => {
                // DECKPAM - Keypad Application Mode
                self.modes.keypad_application = true;
            },
            ([], b'>') => {
                // DECKPNM - Keypad Numeric Mode
                self.modes.keypad_application = false;
            },
            ([b'('], c) => {
                // Designate G0 character set
                tracing::debug!("G0 charset: {}", c as char);
            },
            ([b')'], c) => {
                // Designate G1 character set
                tracing::debug!("G1 charset: {}", c as char);
            },
            _ => {
                tracing::debug!(
                    "Unhandled ESC: intermediates={:?}, final={}",
                    intermediates,
                    final_byte as char
                );
            },
        }
    }

    /// Handle OSC sequence
    fn osc_dispatch(&mut self, params: &[Vec<u8>]) {
        if params.is_empty() {
            return;
        }

        // First param is the command number
        let cmd = std::str::from_utf8(&params[0])
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(0);

        match cmd {
            0 | 2 => {
                // Set window title
                if let Some(title) = params.get(1) {
                    if let Ok(title) = std::str::from_utf8(title) {
                        self.set_title(title.to_string());
                    }
                }
            },
            8 => {
                // Hyperlink
                // OSC 8 ; params ; uri ST
                // params can include id=xxx
                tracing::debug!("Hyperlink: {:?}", params);
            },
            52 => {
                // Clipboard
                // Security: This is disabled by default
                tracing::debug!("OSC 52 clipboard request (disabled by default)");
            },
            _ => {
                tracing::debug!("Unhandled OSC {}: {:?}", cmd, params);
            },
        }
    }

    /// Set a DEC private mode
    fn set_dec_mode(&mut self, mode: u16) {
        match mode {
            25 => {
                self.cursor.set_visible(true);
            },
            47 => {
                // Switch to alternate screen (without save/restore)
                self.switch_to_alternate_screen(false);
            },
            1049 => {
                // Switch to alternate screen with save cursor
                self.switch_to_alternate_screen(true);
            },
            _ => {
                self.modes.set_dec_mode(mode);
            },
        }
    }

    /// Reset a DEC private mode
    fn reset_dec_mode(&mut self, mode: u16) {
        match mode {
            25 => {
                self.cursor.set_visible(false);
            },
            47 => {
                // Switch to primary screen (without restore)
                self.switch_to_primary_screen(false);
            },
            1049 => {
                // Switch to primary screen with restore cursor
                self.switch_to_primary_screen(true);
            },
            _ => {
                self.modes.reset_dec_mode(mode);
            },
        }
    }

    /// Switch to alternate screen buffer
    fn switch_to_alternate_screen(&mut self, save_cursor: bool) {
        if self.active_buffer == BufferType::Alternate {
            return;
        }

        if save_cursor {
            self.save_cursor();
        }

        self.active_buffer = BufferType::Alternate;
        self.modes.alternate_screen = true;

        // Clear alternate screen
        self.alternate.clear(self.current_bg);
    }

    /// Switch to primary screen buffer
    fn switch_to_primary_screen(&mut self, restore_cursor: bool) {
        if self.active_buffer == BufferType::Primary {
            return;
        }

        self.active_buffer = BufferType::Primary;
        self.modes.alternate_screen = false;

        if restore_cursor {
            self.restore_cursor();
        }
    }

    /// Save cursor position and attributes
    fn save_cursor(&mut self) {
        let saved = SavedCursor::from_cursor(&self.cursor, self.modes.origin);
        match self.active_buffer {
            BufferType::Primary => self.saved_cursor_primary = Some(saved),
            BufferType::Alternate => self.saved_cursor_alternate = Some(saved),
        }
    }

    /// Restore cursor position and attributes
    fn restore_cursor(&mut self) {
        let saved = match self.active_buffer {
            BufferType::Primary => self.saved_cursor_primary.clone(),
            BufferType::Alternate => self.saved_cursor_alternate.clone(),
        };

        if let Some(saved) = saved {
            saved.restore_to(&mut self.cursor);
            // Clamp to valid range
            self.cursor.set_row(self.cursor.row().min(self.rows - 1));
            self.cursor.set_col(self.cursor.col().min(self.cols - 1));
        }
    }

    /// Erase in display
    fn erase_in_display(&mut self, mode: u16) {
        let row = self.cursor.row();
        let col = self.cursor.col();
        let current_bg = self.current_bg;
        let rows = self.rows;

        match mode {
            0 => {
                // Erase from cursor to end of display
                // Erase rest of current line
                if let Some(line) = self.grid_mut().line_mut(row) {
                    line.clear_from(col, current_bg);
                }
                // Erase all lines below
                for r in (row + 1)..rows {
                    if let Some(line) = self.grid_mut().line_mut(r) {
                        line.clear_with_bg(current_bg);
                    }
                }
            },
            1 => {
                // Erase from start of display to cursor
                // Erase all lines above
                for r in 0..row {
                    if let Some(line) = self.grid_mut().line_mut(r) {
                        line.clear_with_bg(current_bg);
                    }
                }
                // Erase start of current line
                if let Some(line) = self.grid_mut().line_mut(row) {
                    line.clear_to(col, current_bg);
                }
            },
            2 => {
                // Erase entire display
                self.grid_mut().clear(current_bg);
            },
            3 => {
                // Erase scrollback (xterm extension)
                self.scrollback.clear();
            },
            _ => {},
        }
    }

    /// Erase in line
    fn erase_in_line(&mut self, mode: u16) {
        let row = self.cursor.row();
        let col = self.cursor.col();
        let current_bg = self.current_bg;

        if let Some(line) = self.grid_mut().line_mut(row) {
            match mode {
                0 => {
                    // Erase from cursor to end of line
                    line.clear_from(col, current_bg);
                },
                1 => {
                    // Erase from start of line to cursor
                    line.clear_to(col, current_bg);
                },
                2 => {
                    // Erase entire line
                    line.clear_with_bg(current_bg);
                },
                _ => {},
            }
        }
    }

    /// Handle SGR (Select Graphic Rendition)
    fn sgr(&mut self, params: &[u16]) {
        if params.is_empty() {
            // No params means reset
            self.current_attrs.reset();
            self.current_fg = Color::Default;
            self.current_bg = Color::Default;
            return;
        }

        let mut i = 0;
        while i < params.len() {
            let param = params[i];
            match param {
                0 => {
                    // Reset
                    self.current_attrs.reset();
                    self.current_fg = Color::Default;
                    self.current_bg = Color::Default;
                },
                1 => self.current_attrs.bold = true,
                2 => self.current_attrs.faint = true,
                3 => self.current_attrs.italic = true,
                4 => self.current_attrs.underline = true,
                5 => self.current_attrs.blink = true,
                7 => self.current_attrs.inverse = true,
                8 => self.current_attrs.hidden = true,
                9 => self.current_attrs.strikethrough = true,
                22 => {
                    self.current_attrs.bold = false;
                    self.current_attrs.faint = false;
                },
                23 => self.current_attrs.italic = false,
                24 => self.current_attrs.underline = false,
                25 => self.current_attrs.blink = false,
                27 => self.current_attrs.inverse = false,
                28 => self.current_attrs.hidden = false,
                29 => self.current_attrs.strikethrough = false,
                30..=37 => {
                    // Standard foreground colors
                    self.current_fg = Color::Indexed((param - 30) as u8);
                },
                38 => {
                    // Extended foreground color
                    if let Some(color) = self.parse_extended_color(params, &mut i) {
                        self.current_fg = color;
                    }
                },
                39 => self.current_fg = Color::Default,
                40..=47 => {
                    // Standard background colors
                    self.current_bg = Color::Indexed((param - 40) as u8);
                },
                48 => {
                    // Extended background color
                    if let Some(color) = self.parse_extended_color(params, &mut i) {
                        self.current_bg = color;
                    }
                },
                49 => self.current_bg = Color::Default,
                90..=97 => {
                    // Bright foreground colors
                    self.current_fg = Color::Indexed((param - 90 + 8) as u8);
                },
                100..=107 => {
                    // Bright background colors
                    self.current_bg = Color::Indexed((param - 100 + 8) as u8);
                },
                _ => {
                    tracing::debug!("Unknown SGR parameter: {}", param);
                },
            }
            i += 1;
        }
    }

    /// Parse extended color (38;5;N or 38;2;R;G;B)
    fn parse_extended_color(&self, params: &[u16], i: &mut usize) -> Option<Color> {
        if *i + 1 >= params.len() {
            return None;
        }

        let mode = params[*i + 1];
        match mode {
            5 => {
                // 256-color: 38;5;N
                if *i + 2 < params.len() {
                    *i += 2;
                    Some(Color::Indexed(params[*i] as u8))
                } else {
                    None
                }
            },
            2 => {
                // True color: 38;2;R;G;B
                if *i + 4 < params.len() {
                    let r = params[*i + 2] as u8;
                    let g = params[*i + 3] as u8;
                    let b = params[*i + 4] as u8;
                    *i += 4;
                    Some(Color::Rgb(r, g, b))
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    /// Reset the screen to initial state
    pub fn reset(&mut self) {
        self.primary = Grid::new(self.cols, self.rows);
        self.alternate = Grid::new(self.cols, self.rows);
        self.active_buffer = BufferType::Primary;
        self.scrollback.clear();
        self.cursor = Cursor::new();
        self.saved_cursor_primary = None;
        self.saved_cursor_alternate = None;
        self.modes = Modes::new();
        self.current_attrs = CellAttributes::default();
        self.current_fg = Color::Default;
        self.current_bg = Color::Default;

        // Reset tab stops
        self.tab_stops.fill(false);
        for i in (0..self.cols).step_by(8) {
            self.tab_stops[i] = true;
        }

        self.scroll_top = 0;
        self.scroll_bottom = self.rows - 1;
        self.title.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_new() {
        let screen = Screen::new(80, 24);
        assert_eq!(screen.cols(), 80);
        assert_eq!(screen.rows(), 24);
        assert_eq!(screen.cursor().row(), 0);
        assert_eq!(screen.cursor().col(), 0);
    }

    #[test]
    fn test_screen_print() {
        let mut screen = Screen::new(80, 24);
        screen.print('H');
        screen.print('i');

        assert_eq!(screen.grid().cell(0, 0).unwrap().content(), "H");
        assert_eq!(screen.grid().cell(0, 1).unwrap().content(), "i");
        assert_eq!(screen.cursor().col(), 2);
    }

    #[test]
    fn test_screen_linefeed() {
        let mut screen = Screen::new(80, 24);
        screen.cursor_mut().set_row(23);
        screen.print('A');
        screen.linefeed();

        // Should have scrolled, cursor still at row 23
        assert_eq!(screen.cursor().row(), 23);
        // First row should be in scrollback
        assert_eq!(screen.scrollback().len(), 1);
    }

    #[test]
    fn test_screen_cursor_movement() {
        let mut screen = Screen::new(80, 24);
        screen.cursor_mut().set_position(10, 10);

        screen.csi_dispatch(&[5], &[], b'A'); // Up 5
        assert_eq!(screen.cursor().row(), 5);

        screen.csi_dispatch(&[3], &[], b'B'); // Down 3
        assert_eq!(screen.cursor().row(), 8);

        screen.csi_dispatch(&[2], &[], b'C'); // Right 2
        assert_eq!(screen.cursor().col(), 12);

        screen.csi_dispatch(&[4], &[], b'D'); // Left 4
        assert_eq!(screen.cursor().col(), 8);
    }

    #[test]
    fn test_screen_erase() {
        let mut screen = Screen::new(10, 5);

        // Fill screen with 'X'
        for row in 0..5 {
            for col in 0..10 {
                screen
                    .grid_mut()
                    .cell_mut(row, col)
                    .unwrap()
                    .set_content('X');
            }
        }

        screen.cursor_mut().set_position(2, 5);
        screen.erase_in_line(0); // Erase to end of line

        assert_eq!(screen.grid().cell(2, 4).unwrap().content(), "X");
        assert!(screen.grid().cell(2, 5).unwrap().is_empty());
        assert!(screen.grid().cell(2, 9).unwrap().is_empty());
    }

    #[test]
    fn test_screen_sgr() {
        let mut screen = Screen::new(80, 24);

        screen.sgr(&[1]); // Bold
        assert!(screen.current_attrs().bold);

        screen.sgr(&[31]); // Red foreground
        assert_eq!(screen.current_fg(), Color::Indexed(1));

        screen.sgr(&[0]); // Reset
        assert!(!screen.current_attrs().bold);
        assert_eq!(screen.current_fg(), Color::Default);
    }

    #[test]
    fn test_screen_alternate_buffer() {
        let mut screen = Screen::new(80, 24);
        screen.print('A');

        screen.set_dec_mode(1049); // Switch to alternate
        assert_eq!(screen.active_buffer, BufferType::Alternate);
        assert!(screen.grid().cell(0, 0).unwrap().is_empty());

        screen.print('B');

        screen.reset_dec_mode(1049); // Switch back to primary
        assert_eq!(screen.active_buffer, BufferType::Primary);
        assert_eq!(screen.grid().cell(0, 0).unwrap().content(), "A");
    }
}
