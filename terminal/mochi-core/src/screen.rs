//! Screen model for terminal emulation.
//!
//! The Screen maintains:
//! - Primary screen grid (visible area)
//! - Alternate screen grid (for full-screen apps)
//! - Scrollback buffer
//! - Cursor state
//! - Terminal modes and flags
//! - Current text attributes

use crate::cell::{Attributes, Cell};
use crate::color::Color;
use crate::cursor::{Cursor, SavedCursor};
use crate::line::Line;
use crate::scrollback::Scrollback;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScrollRegion {
    pub top: usize,
    pub bottom: usize,
}

impl ScrollRegion {
    pub fn new(top: usize, bottom: usize) -> Self {
        ScrollRegion { top, bottom }
    }

    pub fn full(rows: usize) -> Self {
        ScrollRegion {
            top: 0,
            bottom: rows.saturating_sub(1),
        }
    }

    pub fn contains(&self, row: usize) -> bool {
        row >= self.top && row <= self.bottom
    }

    pub fn height(&self) -> usize {
        self.bottom - self.top + 1
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalModes {
    pub cursor_visible: bool,
    pub origin_mode: bool,
    pub autowrap: bool,
    pub insert_mode: bool,
    pub linefeed_mode: bool,
    pub bracketed_paste: bool,
    pub focus_events: bool,
    pub mouse_tracking: MouseMode,
    pub mouse_encoding: MouseEncoding,
}

impl Default for TerminalModes {
    fn default() -> Self {
        TerminalModes {
            cursor_visible: true,
            origin_mode: false,
            autowrap: true,
            insert_mode: false,
            linefeed_mode: false,
            bracketed_paste: false,
            focus_events: false,
            mouse_tracking: MouseMode::None,
            mouse_encoding: MouseEncoding::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseMode {
    None,
    X10,
    VT200,
    ButtonEvent,
    AnyEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseEncoding {
    Default,
    Utf8,
    Sgr,
    Urxvt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabStops {
    stops: Vec<bool>,
}

impl TabStops {
    pub fn new(cols: usize) -> Self {
        let mut stops = vec![false; cols];
        for i in (0..cols).step_by(8) {
            stops[i] = true;
        }
        TabStops { stops }
    }

    pub fn set(&mut self, col: usize) {
        if col < self.stops.len() {
            self.stops[col] = true;
        }
    }

    pub fn clear(&mut self, col: usize) {
        if col < self.stops.len() {
            self.stops[col] = false;
        }
    }

    pub fn clear_all(&mut self) {
        for stop in &mut self.stops {
            *stop = false;
        }
    }

    pub fn next_stop(&self, col: usize) -> usize {
        for i in (col + 1)..self.stops.len() {
            if self.stops[i] {
                return i;
            }
        }
        self.stops.len().saturating_sub(1)
    }

    pub fn resize(&mut self, new_cols: usize) {
        let old_len = self.stops.len();
        self.stops.resize(new_cols, false);
        for i in (old_len..new_cols).step_by(8) {
            if i % 8 == 0 {
                self.stops[i] = true;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Screen {
    cols: usize,
    rows: usize,

    primary_grid: Vec<Line>,
    alternate_grid: Vec<Line>,
    using_alternate: bool,

    scrollback: Scrollback,

    cursor: Cursor,
    saved_cursor_primary: Option<SavedCursor>,
    saved_cursor_alternate: Option<SavedCursor>,

    scroll_region: ScrollRegion,

    pub attrs: Attributes,
    pub fg: Color,
    pub bg: Color,

    pub modes: TerminalModes,
    tab_stops: TabStops,

    pending_wrap: bool,

    pub title: String,
    pub icon_name: String,
}

impl Screen {
    pub fn new(cols: usize, rows: usize) -> Self {
        let primary_grid = (0..rows).map(|_| Line::new(cols)).collect();
        let alternate_grid = (0..rows).map(|_| Line::new(cols)).collect();

        Screen {
            cols,
            rows,
            primary_grid,
            alternate_grid,
            using_alternate: false,
            scrollback: Scrollback::default(),
            cursor: Cursor::new(),
            saved_cursor_primary: None,
            saved_cursor_alternate: None,
            scroll_region: ScrollRegion::full(rows),
            attrs: Attributes::default(),
            fg: Color::Default,
            bg: Color::Default,
            modes: TerminalModes::default(),
            tab_stops: TabStops::new(cols),
            pending_wrap: false,
            title: String::new(),
            icon_name: String::new(),
        }
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    pub fn cursor_mut(&mut self) -> &mut Cursor {
        &mut self.cursor
    }

    pub fn scrollback(&self) -> &Scrollback {
        &self.scrollback
    }

    pub fn scroll_region(&self) -> &ScrollRegion {
        &self.scroll_region
    }

    pub fn is_using_alternate(&self) -> bool {
        self.using_alternate
    }

    fn grid(&self) -> &Vec<Line> {
        if self.using_alternate {
            &self.alternate_grid
        } else {
            &self.primary_grid
        }
    }

    fn grid_mut(&mut self) -> &mut Vec<Line> {
        if self.using_alternate {
            &mut self.alternate_grid
        } else {
            &mut self.primary_grid
        }
    }

    pub fn get_line(&self, row: usize) -> Option<&Line> {
        self.grid().get(row)
    }

    pub fn get_line_mut(&mut self, row: usize) -> Option<&mut Line> {
        if self.using_alternate {
            self.alternate_grid.get_mut(row)
        } else {
            self.primary_grid.get_mut(row)
        }
    }

    pub fn get_cell(&self, row: usize, col: usize) -> Option<&Cell> {
        self.grid().get(row).and_then(|line| line.get(col))
    }

    pub fn get_cell_mut(&mut self, row: usize, col: usize) -> Option<&mut Cell> {
        self.get_line_mut(row).and_then(|line| line.get_mut(col))
    }

    pub fn put_char(&mut self, c: char) {
        use unicode_width::UnicodeWidthChar;

        let char_width = c.width().unwrap_or(1);

        if self.pending_wrap && self.modes.autowrap {
            self.pending_wrap = false;
            self.cursor.col = 0;
            self.linefeed();
        }

        let cursor_row = self.cursor.row;
        let cursor_col = self.cursor.col;
        let cols = self.cols;
        let fg = self.fg;
        let bg = self.bg;
        let attrs = self.attrs;
        let insert_mode = self.modes.insert_mode;

        if insert_mode && char_width > 0 {
            if let Some(line) = self.get_line_mut(cursor_row) {
                line.insert_cells(cursor_col, char_width);
            }
        }

        let cell = Cell::with_attrs(c, fg, bg, attrs);

        if let Some(line) = self.get_line_mut(cursor_row) {
            if cursor_col < cols {
                line.set(cursor_col, cell);

                if char_width == 2 && cursor_col + 1 < cols {
                    let mut cont = Cell::default();
                    cont.set_wide_continuation();
                    cont.fg = fg;
                    cont.bg = bg;
                    line.set(cursor_col + 1, cont);
                }
            }
        }

        let new_col = self.cursor.col + char_width;
        if new_col >= self.cols {
            self.cursor.col = self.cols - 1;
            self.pending_wrap = true;
        } else {
            self.cursor.col = new_col;
        }
    }

    pub fn linefeed(&mut self) {
        self.pending_wrap = false;

        if self.cursor.row == self.scroll_region.bottom {
            self.scroll_up(1);
        } else if self.cursor.row < self.rows - 1 {
            self.cursor.row += 1;
        }

        if self.modes.linefeed_mode {
            self.cursor.col = 0;
        }
    }

    pub fn reverse_index(&mut self) {
        self.pending_wrap = false;

        if self.cursor.row == self.scroll_region.top {
            self.scroll_down(1);
        } else if self.cursor.row > 0 {
            self.cursor.row -= 1;
        }
    }

    pub fn carriage_return(&mut self) {
        self.pending_wrap = false;
        self.cursor.col = 0;
    }

    pub fn backspace(&mut self) {
        self.pending_wrap = false;
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        }
    }

    pub fn tab(&mut self) {
        self.pending_wrap = false;
        let next = self.tab_stops.next_stop(self.cursor.col);
        self.cursor.col = next.min(self.cols - 1);
    }

    pub fn set_tab_stop(&mut self) {
        self.tab_stops.set(self.cursor.col);
    }

    pub fn clear_tab_stop(&mut self, mode: u16) {
        match mode {
            0 => self.tab_stops.clear(self.cursor.col),
            3 => self.tab_stops.clear_all(),
            _ => {}
        }
    }

    pub fn scroll_up(&mut self, count: usize) {
        let top = self.scroll_region.top;
        let bottom = self.scroll_region.bottom;
        let cols = self.cols;
        let bg = self.bg;
        let using_alternate = self.using_alternate;

        if count == 0 || top > bottom {
            return;
        }

        let count = count.min(bottom - top + 1);

        // Push lines to scrollback before modifying grid
        // We need to clone the lines first to avoid borrow conflicts
        if !using_alternate && top == 0 {
            let lines_to_save: Vec<Line> = (0..count)
                .filter_map(|i| self.grid().get(i).cloned())
                .collect();
            for line in lines_to_save {
                self.scrollback.push(line);
            }
        }

        // Now modify the grid
        let grid = self.grid_mut();
        for i in top..=bottom {
            if i + count <= bottom {
                grid[i] = grid[i + count].clone();
            } else {
                grid[i] = Line::new(cols);
                grid[i].clear_with_bg(bg);
            }
        }
    }

    pub fn scroll_down(&mut self, count: usize) {
        let top = self.scroll_region.top;
        let bottom = self.scroll_region.bottom;
        let cols = self.cols;
        let bg = self.bg;

        if count == 0 || top > bottom {
            return;
        }

        let count = count.min(bottom - top + 1);

        let grid = self.grid_mut();

        for i in (top..=bottom).rev() {
            if i >= top + count {
                grid[i] = grid[i - count].clone();
            } else {
                grid[i] = Line::new(cols);
                grid[i].clear_with_bg(bg);
            }
        }
    }

    pub fn move_cursor_to(&mut self, row: usize, col: usize) {
        self.pending_wrap = false;

        let (min_row, max_row) = if self.modes.origin_mode {
            (self.scroll_region.top, self.scroll_region.bottom)
        } else {
            (0, self.rows - 1)
        };

        let actual_row = if self.modes.origin_mode {
            (self.scroll_region.top + row).min(max_row)
        } else {
            row.min(max_row)
        };

        self.cursor.row = actual_row.max(min_row);
        self.cursor.col = col.min(self.cols - 1);
    }

    pub fn move_cursor_up(&mut self, n: usize) {
        self.pending_wrap = false;
        let min_row = if self.modes.origin_mode {
            self.scroll_region.top
        } else {
            0
        };
        self.cursor.row = self.cursor.row.saturating_sub(n).max(min_row);
    }

    pub fn move_cursor_down(&mut self, n: usize) {
        self.pending_wrap = false;
        let max_row = if self.modes.origin_mode {
            self.scroll_region.bottom
        } else {
            self.rows - 1
        };
        self.cursor.row = (self.cursor.row + n).min(max_row);
    }

    pub fn move_cursor_forward(&mut self, n: usize) {
        self.pending_wrap = false;
        self.cursor.col = (self.cursor.col + n).min(self.cols - 1);
    }

    pub fn move_cursor_backward(&mut self, n: usize) {
        self.pending_wrap = false;
        self.cursor.col = self.cursor.col.saturating_sub(n);
    }

    pub fn move_cursor_to_col(&mut self, col: usize) {
        self.pending_wrap = false;
        self.cursor.col = col.min(self.cols - 1);
    }

    pub fn move_cursor_to_row(&mut self, row: usize) {
        self.pending_wrap = false;
        let (min_row, max_row) = if self.modes.origin_mode {
            (self.scroll_region.top, self.scroll_region.bottom)
        } else {
            (0, self.rows - 1)
        };
        let actual_row = if self.modes.origin_mode {
            self.scroll_region.top + row
        } else {
            row
        };
        self.cursor.row = actual_row.min(max_row).max(min_row);
    }

    pub fn erase_in_display(&mut self, mode: u16) {
        let cursor_row = self.cursor.row;
        let cursor_col = self.cursor.col;
        let cols = self.cols;
        let rows = self.rows;
        let bg = self.bg;

        match mode {
            0 => {
                if let Some(line) = self.get_line_mut(cursor_row) {
                    line.clear_range_with_bg(cursor_col, cols, bg);
                }
                for row in (cursor_row + 1)..rows {
                    if let Some(line) = self.get_line_mut(row) {
                        line.clear_with_bg(bg);
                    }
                }
            }
            1 => {
                for row in 0..cursor_row {
                    if let Some(line) = self.get_line_mut(row) {
                        line.clear_with_bg(bg);
                    }
                }
                if let Some(line) = self.get_line_mut(cursor_row) {
                    line.clear_range_with_bg(0, cursor_col + 1, bg);
                }
            }
            2 => {
                for row in 0..rows {
                    if let Some(line) = self.get_line_mut(row) {
                        line.clear_with_bg(bg);
                    }
                }
            }
            3 => {
                self.scrollback.clear();
                for row in 0..rows {
                    if let Some(line) = self.get_line_mut(row) {
                        line.clear_with_bg(bg);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn erase_in_line(&mut self, mode: u16) {
        let cursor_row = self.cursor.row;
        let cursor_col = self.cursor.col;
        let cols = self.cols;
        let bg = self.bg;

        if let Some(line) = self.get_line_mut(cursor_row) {
            match mode {
                0 => line.clear_range_with_bg(cursor_col, cols, bg),
                1 => line.clear_range_with_bg(0, cursor_col + 1, bg),
                2 => line.clear_with_bg(bg),
                _ => {}
            }
        }
    }

    pub fn erase_chars(&mut self, count: usize) {
        let cursor_row = self.cursor.row;
        let cursor_col = self.cursor.col;
        let cols = self.cols;
        let bg = self.bg;

        if let Some(line) = self.get_line_mut(cursor_row) {
            let end = (cursor_col + count).min(cols);
            line.clear_range_with_bg(cursor_col, end, bg);
        }
    }

    pub fn insert_lines(&mut self, count: usize) {
        self.pending_wrap = false;

        let cursor_row = self.cursor.row;
        let scroll_bottom = self.scroll_region.bottom;
        let cols = self.cols;

        if !self.scroll_region.contains(cursor_row) {
            return;
        }

        let count = count.min(scroll_bottom - cursor_row + 1);

        let grid = self.grid_mut();
        for _ in 0..count {
            if scroll_bottom < grid.len() {
                grid.remove(scroll_bottom);
            }
            grid.insert(cursor_row, Line::new(cols));
        }
    }

    pub fn delete_lines(&mut self, count: usize) {
        self.pending_wrap = false;

        let cursor_row = self.cursor.row;
        let scroll_bottom = self.scroll_region.bottom;
        let cols = self.cols;
        let rows = self.rows;

        if !self.scroll_region.contains(cursor_row) {
            return;
        }

        let count = count.min(scroll_bottom - cursor_row + 1);

        let grid = self.grid_mut();
        for _ in 0..count {
            if cursor_row < grid.len() {
                grid.remove(cursor_row);
            }
            if scroll_bottom < rows {
                grid.insert(scroll_bottom, Line::new(cols));
            }
        }
    }

    pub fn insert_chars(&mut self, count: usize) {
        self.pending_wrap = false;
        let cursor_row = self.cursor.row;
        let cursor_col = self.cursor.col;

        if let Some(line) = self.get_line_mut(cursor_row) {
            line.insert_cells(cursor_col, count);
        }
    }

    pub fn delete_chars(&mut self, count: usize) {
        self.pending_wrap = false;
        let cursor_row = self.cursor.row;
        let cursor_col = self.cursor.col;

        if let Some(line) = self.get_line_mut(cursor_row) {
            line.delete_cells(cursor_col, count);
        }
    }

    pub fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        let top = top.min(self.rows - 1);
        let bottom = bottom.min(self.rows - 1);

        if top < bottom {
            self.scroll_region = ScrollRegion::new(top, bottom);
            self.move_cursor_to(0, 0);
        }
    }

    pub fn reset_scroll_region(&mut self) {
        self.scroll_region = ScrollRegion::full(self.rows);
    }

    pub fn save_cursor(&mut self) {
        let saved = SavedCursor::from_cursor(
            &self.cursor,
            &self.attrs,
            self.fg,
            self.bg,
            self.modes.origin_mode,
            self.modes.autowrap,
        );

        if self.using_alternate {
            self.saved_cursor_alternate = Some(saved);
        } else {
            self.saved_cursor_primary = Some(saved);
        }
    }

    pub fn restore_cursor(&mut self) {
        let saved = if self.using_alternate {
            self.saved_cursor_alternate.clone()
        } else {
            self.saved_cursor_primary.clone()
        };

        if let Some(saved) = saved {
            self.cursor.row = saved.row.min(self.rows - 1);
            self.cursor.col = saved.col.min(self.cols - 1);
            self.attrs = saved.attrs;
            self.fg = saved.fg;
            self.bg = saved.bg;
            self.modes.origin_mode = saved.origin_mode;
            self.modes.autowrap = saved.autowrap;
        }

        self.pending_wrap = false;
    }

    pub fn enter_alternate_screen(&mut self) {
        if !self.using_alternate {
            self.using_alternate = true;
            for line in &mut self.alternate_grid {
                line.clear();
            }
            self.cursor = Cursor::new();
        }
    }

    pub fn exit_alternate_screen(&mut self) {
        if self.using_alternate {
            self.using_alternate = false;
        }
    }

    pub fn resize(&mut self, new_cols: usize, new_rows: usize) {
        if new_cols == self.cols && new_rows == self.rows {
            return;
        }

        for line in &mut self.primary_grid {
            line.resize(new_cols);
        }
        for line in &mut self.alternate_grid {
            line.resize(new_cols);
        }

        while self.primary_grid.len() < new_rows {
            self.primary_grid.push(Line::new(new_cols));
        }
        while self.primary_grid.len() > new_rows {
            let line = self.primary_grid.remove(0);
            if !self.using_alternate {
                self.scrollback.push(line);
            }
        }

        while self.alternate_grid.len() < new_rows {
            self.alternate_grid.push(Line::new(new_cols));
        }
        while self.alternate_grid.len() > new_rows {
            self.alternate_grid.pop();
        }

        self.cols = new_cols;
        self.rows = new_rows;

        self.cursor.row = self.cursor.row.min(new_rows.saturating_sub(1));
        self.cursor.col = self.cursor.col.min(new_cols.saturating_sub(1));

        self.scroll_region = ScrollRegion::full(new_rows);
        self.tab_stops.resize(new_cols);
        self.pending_wrap = false;
    }

    pub fn reset(&mut self) {
        self.cursor = Cursor::new();
        self.attrs = Attributes::default();
        self.fg = Color::Default;
        self.bg = Color::Default;
        self.modes = TerminalModes::default();
        self.scroll_region = ScrollRegion::full(self.rows);
        self.tab_stops = TabStops::new(self.cols);
        self.pending_wrap = false;
        self.saved_cursor_primary = None;
        self.saved_cursor_alternate = None;

        for line in &mut self.primary_grid {
            line.clear();
        }
        for line in &mut self.alternate_grid {
            line.clear();
        }

        self.using_alternate = false;
    }

    pub fn bell(&self) {
        log::debug!("Bell!");
    }
}

impl Default for Screen {
    fn default() -> Self {
        Screen::new(crate::DEFAULT_COLS, crate::DEFAULT_ROWS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_screen() {
        let screen = Screen::new(80, 24);
        assert_eq!(screen.cols(), 80);
        assert_eq!(screen.rows(), 24);
        assert_eq!(screen.cursor().row, 0);
        assert_eq!(screen.cursor().col, 0);
    }

    #[test]
    fn test_put_char() {
        let mut screen = Screen::new(80, 24);
        screen.put_char('A');
        assert_eq!(screen.get_cell(0, 0).unwrap().character, 'A');
        assert_eq!(screen.cursor().col, 1);
    }

    #[test]
    fn test_linefeed() {
        let mut screen = Screen::new(80, 24);
        screen.cursor.row = 5;
        screen.linefeed();
        assert_eq!(screen.cursor().row, 6);
    }

    #[test]
    fn test_scroll_up() {
        let mut screen = Screen::new(80, 24);
        screen.put_char('A');
        screen.cursor.row = 23;
        screen.linefeed();
        assert_eq!(screen.scrollback().len(), 1);
    }

    #[test]
    fn test_erase_in_display() {
        let mut screen = Screen::new(80, 24);
        for i in 0..10 {
            screen.put_char((b'A' + i as u8) as char);
        }
        screen.cursor.col = 5;
        screen.erase_in_display(0);
        assert_eq!(screen.get_cell(0, 4).unwrap().character, 'E');
        assert_eq!(screen.get_cell(0, 5).unwrap().character, ' ');
    }

    #[test]
    fn test_cursor_movement() {
        let mut screen = Screen::new(80, 24);
        screen.move_cursor_to(10, 20);
        assert_eq!(screen.cursor().row, 10);
        assert_eq!(screen.cursor().col, 20);

        screen.move_cursor_up(5);
        assert_eq!(screen.cursor().row, 5);

        screen.move_cursor_down(10);
        assert_eq!(screen.cursor().row, 15);
    }

    #[test]
    fn test_scroll_region() {
        let mut screen = Screen::new(80, 24);
        screen.set_scroll_region(5, 15);
        assert_eq!(screen.scroll_region().top, 5);
        assert_eq!(screen.scroll_region().bottom, 15);
    }

    #[test]
    fn test_alternate_screen() {
        let mut screen = Screen::new(80, 24);
        screen.put_char('A');

        screen.enter_alternate_screen();
        assert!(screen.is_using_alternate());
        assert_eq!(screen.get_cell(0, 0).unwrap().character, ' ');

        screen.put_char('B');
        assert_eq!(screen.get_cell(0, 0).unwrap().character, 'B');

        screen.exit_alternate_screen();
        assert!(!screen.is_using_alternate());
        assert_eq!(screen.get_cell(0, 0).unwrap().character, 'A');
    }

    #[test]
    fn test_save_restore_cursor() {
        let mut screen = Screen::new(80, 24);
        screen.move_cursor_to(10, 20);
        screen.attrs.bold = true;
        screen.save_cursor();

        screen.move_cursor_to(5, 5);
        screen.attrs.bold = false;

        screen.restore_cursor();
        assert_eq!(screen.cursor().row, 10);
        assert_eq!(screen.cursor().col, 20);
        assert!(screen.attrs.bold);
    }

    #[test]
    fn test_resize() {
        let mut screen = Screen::new(80, 24);
        screen.put_char('A');
        screen.resize(100, 30);
        assert_eq!(screen.cols(), 100);
        assert_eq!(screen.rows(), 30);
        assert_eq!(screen.get_cell(0, 0).unwrap().character, 'A');
    }

    #[test]
    fn test_tab_stops() {
        let mut screen = Screen::new(80, 24);
        screen.cursor.col = 0;
        screen.tab();
        assert_eq!(screen.cursor().col, 8);
        screen.tab();
        assert_eq!(screen.cursor().col, 16);
    }

    #[test]
    fn test_autowrap() {
        let mut screen = Screen::new(10, 5);
        for i in 0..15 {
            screen.put_char((b'A' + (i % 26) as u8) as char);
        }
        assert_eq!(screen.cursor().row, 1);
        assert_eq!(screen.cursor().col, 5);
    }
}
