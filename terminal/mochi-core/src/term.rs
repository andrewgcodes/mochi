//! Terminal state machine
//!
//! The Term struct is the main entry point for the terminal emulator core.
//! It manages primary and alternate screens, scrollback, and processes
//! terminal actions from the parser.

use serde::{Deserialize, Serialize};

use crate::cell::CellAttributes;
use crate::color::{Color, NamedColor, Rgb};
use crate::cursor::CursorStyle;
use crate::line::Line;
use crate::screen::{MouseEncoding, MouseMode, Screen, ScreenMode};
use crate::scrollback::Scrollback;
use crate::selection::Selection;
use crate::snapshot::Snapshot;

/// Terminal mode flags (global, not per-screen)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TermMode {
    /// Using alternate screen buffer
    pub alt_screen: bool,
    /// Application cursor keys mode (DECCKM)
    pub app_cursor: bool,
    /// Application keypad mode (DECNKM)
    pub app_keypad: bool,
}

/// Hyperlink information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hyperlink {
    pub id: u32,
    pub uri: String,
    pub params: String,
}

/// The main terminal state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Term {
    /// Primary screen
    primary: Screen,
    /// Alternate screen (for full-screen apps like vim)
    alternate: Screen,
    /// Which screen is active
    pub mode: TermMode,
    /// Scrollback buffer (only for primary screen)
    scrollback: Scrollback,
    /// Current selection
    pub selection: Selection,
    /// Window title
    pub title: String,
    /// Icon name
    pub icon_name: String,
    /// Active hyperlinks (id -> hyperlink)
    hyperlinks: Vec<Hyperlink>,
    /// Next hyperlink ID
    next_hyperlink_id: u32,
    /// Saved cursor for primary screen (separate from screen's saved cursor)
    saved_cursor_primary: Option<crate::cursor::SavedCursor>,
    /// Saved cursor for alternate screen
    saved_cursor_alternate: Option<crate::cursor::SavedCursor>,
    /// Charset designations (G0-G3)
    charsets: [Charset; 4],
    /// Active charset (index into charsets)
    active_charset: usize,
}

/// Character set for special graphics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Charset {
    #[default]
    Ascii,
    /// DEC Special Graphics (line drawing)
    DecSpecialGraphics,
    /// UK charset
    Uk,
}

impl Term {
    /// Create a new terminal with the given dimensions
    pub fn new(rows: usize, cols: usize) -> Self {
        Term {
            primary: Screen::new(rows, cols),
            alternate: Screen::new(rows, cols),
            mode: TermMode::default(),
            scrollback: Scrollback::default(),
            selection: Selection::default(),
            title: String::new(),
            icon_name: String::new(),
            hyperlinks: Vec::new(),
            next_hyperlink_id: 1,
            saved_cursor_primary: None,
            saved_cursor_alternate: None,
            charsets: [Charset::Ascii; 4],
            active_charset: 0,
        }
    }

    /// Get the active screen
    pub fn screen(&self) -> &Screen {
        if self.mode.alt_screen {
            &self.alternate
        } else {
            &self.primary
        }
    }

    /// Get the active screen mutably
    pub fn screen_mut(&mut self) -> &mut Screen {
        if self.mode.alt_screen {
            &mut self.alternate
        } else {
            &mut self.primary
        }
    }

    /// Get the number of rows
    pub fn rows(&self) -> usize {
        self.screen().rows()
    }

    /// Get the number of columns
    pub fn cols(&self) -> usize {
        self.screen().cols()
    }

    /// Get the scrollback buffer
    pub fn scrollback(&self) -> &Scrollback {
        &self.scrollback
    }

    /// Resize the terminal
    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.primary.resize(rows, cols);
        self.alternate.resize(rows, cols);
        self.selection.clear();
    }

    /// Reset the terminal to initial state
    pub fn reset(&mut self) {
        self.primary.reset();
        self.alternate.reset();
        self.mode = TermMode::default();
        self.scrollback.clear();
        self.selection.clear();
        self.title.clear();
        self.icon_name.clear();
        self.hyperlinks.clear();
        self.next_hyperlink_id = 1;
        self.saved_cursor_primary = None;
        self.saved_cursor_alternate = None;
        self.charsets = [Charset::Ascii; 4];
        self.active_charset = 0;
    }

    /// Enter alternate screen mode
    pub fn enter_alt_screen(&mut self) {
        if !self.mode.alt_screen {
            self.mode.alt_screen = true;
            self.alternate.reset();
        }
    }

    /// Leave alternate screen mode
    pub fn leave_alt_screen(&mut self) {
        if self.mode.alt_screen {
            self.mode.alt_screen = false;
        }
    }

    /// Save cursor (DECSC or CSI s)
    pub fn save_cursor(&mut self) {
        let saved = crate::cursor::SavedCursor::from(&self.screen().cursor);
        if self.mode.alt_screen {
            self.saved_cursor_alternate = Some(saved);
        } else {
            self.saved_cursor_primary = Some(saved);
        }
    }

    /// Restore cursor (DECRC or CSI u)
    pub fn restore_cursor(&mut self) {
        let saved = if self.mode.alt_screen {
            self.saved_cursor_alternate.clone()
        } else {
            self.saved_cursor_primary.clone()
        };

        if let Some(saved) = saved {
            saved.restore_to(&mut self.screen_mut().cursor);
            // Clamp to screen bounds
            let rows = self.rows();
            let cols = self.cols();
            let cursor = &mut self.screen_mut().cursor;
            cursor.row = cursor.row.min(rows.saturating_sub(1));
            cursor.col = cursor.col.min(cols.saturating_sub(1));
        }
    }

    /// Write a character to the terminal
    pub fn write_char(&mut self, c: char) {
        // Apply charset translation
        let c = self.translate_char(c);

        if let Some(line) = self.screen_mut().write_char(c) {
            // Line scrolled off - add to scrollback if on primary screen
            if !self.mode.alt_screen {
                self.scrollback.push(line);
            }
        }
    }

    /// Translate character through active charset
    fn translate_char(&self, c: char) -> char {
        match self.charsets[self.active_charset] {
            Charset::Ascii => c,
            Charset::DecSpecialGraphics => {
                // DEC Special Graphics character set (line drawing)
                match c {
                    'j' => '┘', // Lower right corner
                    'k' => '┐', // Upper right corner
                    'l' => '┌', // Upper left corner
                    'm' => '└', // Lower left corner
                    'n' => '┼', // Crossing lines
                    'q' => '─', // Horizontal line
                    't' => '├', // Left tee
                    'u' => '┤', // Right tee
                    'v' => '┴', // Bottom tee
                    'w' => '┬', // Top tee
                    'x' => '│', // Vertical line
                    'a' => '▒', // Checker board
                    '`' => '◆', // Diamond
                    'f' => '°', // Degree symbol
                    'g' => '±', // Plus/minus
                    'o' => '⎺', // Scan line 1
                    'p' => '⎻', // Scan line 3
                    'r' => '⎼', // Scan line 7
                    's' => '⎽', // Scan line 9
                    '~' => '·', // Bullet
                    _ => c,
                }
            }
            Charset::Uk => {
                // UK charset: # becomes £
                if c == '#' {
                    '£'
                } else {
                    c
                }
            }
        }
    }

    /// Set charset designation
    pub fn set_charset(&mut self, index: usize, charset: Charset) {
        if index < 4 {
            self.charsets[index] = charset;
        }
    }

    /// Set active charset
    pub fn set_active_charset(&mut self, index: usize) {
        if index < 4 {
            self.active_charset = index;
        }
    }

    /// Process a linefeed
    pub fn linefeed(&mut self) {
        if let Some(line) = self.screen_mut().linefeed() {
            if !self.mode.alt_screen {
                self.scrollback.push(line);
            }
        }
    }

    /// Process a carriage return
    pub fn carriage_return(&mut self) {
        self.screen_mut().carriage_return();
    }

    /// Process a backspace
    pub fn backspace(&mut self) {
        self.screen_mut().backspace();
    }

    /// Process a tab
    pub fn tab(&mut self) {
        self.screen_mut().tab();
    }

    /// Process a bell
    pub fn bell(&mut self) {
        // Bell is handled by the frontend
        log::debug!("Bell");
    }

    /// Move cursor to position
    pub fn goto(&mut self, row: usize, col: usize) {
        self.screen_mut().goto(row, col);
    }

    /// Move cursor up
    pub fn move_up(&mut self, n: usize) {
        let screen = self.screen_mut();
        screen.cursor.move_up(n);
    }

    /// Move cursor down
    pub fn move_down(&mut self, n: usize) {
        let max_row = self.rows().saturating_sub(1);
        self.screen_mut().cursor.move_down(n, max_row);
    }

    /// Move cursor left
    pub fn move_left(&mut self, n: usize) {
        self.screen_mut().cursor.move_left(n);
    }

    /// Move cursor right
    pub fn move_right(&mut self, n: usize) {
        let max_col = self.cols().saturating_sub(1);
        self.screen_mut().cursor.move_right(n, max_col);
    }

    /// Move cursor to column
    pub fn goto_col(&mut self, col: usize) {
        let max_col = self.cols().saturating_sub(1);
        self.screen_mut().cursor.goto_col(col, max_col);
    }

    /// Move cursor to row
    pub fn goto_row(&mut self, row: usize) {
        let max_row = self.rows().saturating_sub(1);
        self.screen_mut().cursor.goto_row(row, max_row);
    }

    /// Reverse index (move up, scroll down if at top)
    pub fn reverse_index(&mut self) {
        self.screen_mut().reverse_index();
    }

    /// Index (move down, scroll up if at bottom)
    pub fn index(&mut self) {
        self.linefeed();
    }

    /// Next line (CR + LF)
    pub fn next_line(&mut self) {
        self.carriage_return();
        self.linefeed();
    }

    /// Set scroll region
    pub fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        self.screen_mut().set_scroll_region(top, bottom);
    }

    /// Scroll up
    pub fn scroll_up(&mut self, n: usize) {
        let lines = self.screen_mut().scroll_up(n);
        if !self.mode.alt_screen {
            self.scrollback.push_lines(lines);
        }
    }

    /// Scroll down
    pub fn scroll_down(&mut self, n: usize) {
        self.screen_mut().scroll_down(n);
    }

    /// Erase from cursor to end of line
    pub fn erase_to_eol(&mut self) {
        self.screen_mut().erase_to_eol();
    }

    /// Erase from start of line to cursor
    pub fn erase_to_bol(&mut self) {
        self.screen_mut().erase_to_bol();
    }

    /// Erase entire line
    pub fn erase_line(&mut self) {
        self.screen_mut().erase_line();
    }

    /// Erase from cursor to end of screen
    pub fn erase_below(&mut self) {
        self.screen_mut().erase_below();
    }

    /// Erase from start of screen to cursor
    pub fn erase_above(&mut self) {
        self.screen_mut().erase_above();
    }

    /// Erase entire screen
    pub fn erase_screen(&mut self) {
        self.screen_mut().erase_screen();
    }

    /// Erase n characters at cursor
    pub fn erase_chars(&mut self, n: usize) {
        self.screen_mut().erase_chars(n);
    }

    /// Insert n blank lines
    pub fn insert_lines(&mut self, n: usize) {
        self.screen_mut().insert_lines(n);
    }

    /// Delete n lines
    pub fn delete_lines(&mut self, n: usize) {
        self.screen_mut().delete_lines(n);
    }

    /// Insert n blank characters
    pub fn insert_chars(&mut self, n: usize) {
        self.screen_mut().insert_chars(n);
    }

    /// Delete n characters
    pub fn delete_chars(&mut self, n: usize) {
        self.screen_mut().delete_chars(n);
    }

    /// Set cursor visibility
    pub fn set_cursor_visible(&mut self, visible: bool) {
        self.screen_mut().cursor.visible = visible;
        self.screen_mut().mode.cursor_visible = visible;
    }

    /// Set cursor style
    pub fn set_cursor_style(&mut self, style: CursorStyle) {
        self.screen_mut().cursor.style = style;
    }

    /// Set cursor blinking
    pub fn set_cursor_blinking(&mut self, blinking: bool) {
        self.screen_mut().cursor.blinking = blinking;
    }

    /// Set origin mode
    pub fn set_origin_mode(&mut self, enabled: bool) {
        self.screen_mut().mode.origin_mode = enabled;
        self.screen_mut().cursor.origin_mode = enabled;
        // Reset cursor to home when changing origin mode
        if enabled {
            let top = self.screen().scroll_top;
            self.screen_mut().cursor.row = top;
        } else {
            self.screen_mut().cursor.row = 0;
        }
        self.screen_mut().cursor.col = 0;
    }

    /// Set auto-wrap mode
    pub fn set_auto_wrap(&mut self, enabled: bool) {
        self.screen_mut().mode.auto_wrap = enabled;
    }

    /// Set insert mode
    pub fn set_insert_mode(&mut self, enabled: bool) {
        self.screen_mut().mode.insert_mode = enabled;
    }

    /// Set linefeed mode
    pub fn set_linefeed_mode(&mut self, enabled: bool) {
        self.screen_mut().mode.linefeed_mode = enabled;
    }

    /// Set reverse video mode
    pub fn set_reverse_video(&mut self, enabled: bool) {
        self.screen_mut().mode.reverse_video = enabled;
    }

    /// Set bracketed paste mode
    pub fn set_bracketed_paste(&mut self, enabled: bool) {
        self.screen_mut().mode.bracketed_paste = enabled;
    }

    /// Set focus reporting mode
    pub fn set_focus_reporting(&mut self, enabled: bool) {
        self.screen_mut().mode.focus_reporting = enabled;
    }

    /// Set mouse mode
    pub fn set_mouse_mode(&mut self, mode: MouseMode) {
        self.screen_mut().mode.mouse_mode = mode;
    }

    /// Set mouse encoding
    pub fn set_mouse_encoding(&mut self, encoding: MouseEncoding) {
        self.screen_mut().mode.mouse_encoding = encoding;
    }

    /// Set application cursor keys mode
    pub fn set_app_cursor(&mut self, enabled: bool) {
        self.mode.app_cursor = enabled;
    }

    /// Set application keypad mode
    pub fn set_app_keypad(&mut self, enabled: bool) {
        self.mode.app_keypad = enabled;
    }

    /// Set tab stop at current column
    pub fn set_tab_stop(&mut self) {
        let col = self.screen().cursor.col;
        self.screen_mut().tabs.set(col);
    }

    /// Clear tab stop at current column
    pub fn clear_tab_stop(&mut self) {
        let col = self.screen().cursor.col;
        self.screen_mut().tabs.clear(col);
    }

    /// Clear all tab stops
    pub fn clear_all_tab_stops(&mut self) {
        self.screen_mut().tabs.clear_all();
    }

    /// Reset SGR attributes
    pub fn reset_attrs(&mut self) {
        self.screen_mut().cursor.attrs.reset();
    }

    /// Set foreground color
    pub fn set_fg(&mut self, color: Color) {
        self.screen_mut().cursor.attrs.fg = color;
    }

    /// Set background color
    pub fn set_bg(&mut self, color: Color) {
        self.screen_mut().cursor.attrs.bg = color;
    }

    /// Set bold
    pub fn set_bold(&mut self, enabled: bool) {
        self.screen_mut()
            .cursor
            .attrs
            .flags
            .set(crate::cell::CellFlags::BOLD, enabled);
    }

    /// Set faint
    pub fn set_faint(&mut self, enabled: bool) {
        self.screen_mut()
            .cursor
            .attrs
            .flags
            .set(crate::cell::CellFlags::FAINT, enabled);
    }

    /// Set italic
    pub fn set_italic(&mut self, enabled: bool) {
        self.screen_mut()
            .cursor
            .attrs
            .flags
            .set(crate::cell::CellFlags::ITALIC, enabled);
    }

    /// Set underline
    pub fn set_underline(&mut self, enabled: bool) {
        self.screen_mut()
            .cursor
            .attrs
            .flags
            .set(crate::cell::CellFlags::UNDERLINE, enabled);
    }

    /// Set blink
    pub fn set_blink(&mut self, enabled: bool) {
        self.screen_mut()
            .cursor
            .attrs
            .flags
            .set(crate::cell::CellFlags::BLINK, enabled);
    }

    /// Set inverse
    pub fn set_inverse(&mut self, enabled: bool) {
        self.screen_mut()
            .cursor
            .attrs
            .flags
            .set(crate::cell::CellFlags::INVERSE, enabled);
    }

    /// Set hidden
    pub fn set_hidden(&mut self, enabled: bool) {
        self.screen_mut()
            .cursor
            .attrs
            .flags
            .set(crate::cell::CellFlags::HIDDEN, enabled);
    }

    /// Set strikethrough
    pub fn set_strikethrough(&mut self, enabled: bool) {
        self.screen_mut()
            .cursor
            .attrs
            .flags
            .set(crate::cell::CellFlags::STRIKETHROUGH, enabled);
    }

    /// Set window title
    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    /// Set icon name
    pub fn set_icon_name(&mut self, name: String) {
        self.icon_name = name;
    }

    /// Register a hyperlink and return its ID
    pub fn register_hyperlink(&mut self, uri: String, params: String) -> u32 {
        let id = self.next_hyperlink_id;
        self.next_hyperlink_id += 1;
        self.hyperlinks.push(Hyperlink { id, uri, params });
        id
    }

    /// Get hyperlink by ID
    pub fn get_hyperlink(&self, id: u32) -> Option<&Hyperlink> {
        self.hyperlinks.iter().find(|h| h.id == id)
    }

    /// Set current hyperlink
    pub fn set_hyperlink(&mut self, id: u32) {
        self.screen_mut().cursor.attrs.hyperlink_id = id;
    }

    /// Clear current hyperlink
    pub fn clear_hyperlink(&mut self) {
        self.screen_mut().cursor.attrs.hyperlink_id = 0;
    }

    /// Create a snapshot of the current state
    pub fn snapshot(&self) -> Snapshot {
        let screen = self.screen();
        Snapshot::from_screen(
            &screen.grid,
            &screen.cursor,
            &screen.mode,
            screen.scroll_top,
            screen.scroll_bottom,
            if self.title.is_empty() {
                None
            } else {
                Some(self.title.clone())
            },
        )
    }

    /// Get the current screen mode
    pub fn screen_mode(&self) -> &ScreenMode {
        &self.screen().mode
    }

    /// Check if bracketed paste is enabled
    pub fn bracketed_paste_enabled(&self) -> bool {
        self.screen().mode.bracketed_paste
    }

    /// Get the current mouse mode
    pub fn mouse_mode(&self) -> MouseMode {
        self.screen().mode.mouse_mode
    }

    /// Get the current mouse encoding
    pub fn mouse_encoding(&self) -> MouseEncoding {
        self.screen().mode.mouse_encoding
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_term_new() {
        let term = Term::new(24, 80);
        assert_eq!(term.rows(), 24);
        assert_eq!(term.cols(), 80);
        assert!(!term.mode.alt_screen);
    }

    #[test]
    fn test_term_write() {
        let mut term = Term::new(24, 80);
        term.write_char('H');
        term.write_char('i');

        let snapshot = term.snapshot();
        assert_eq!(snapshot.row_text(0), "Hi");
    }

    #[test]
    fn test_term_alt_screen() {
        let mut term = Term::new(24, 80);
        term.write_char('A');

        term.enter_alt_screen();
        assert!(term.mode.alt_screen);
        term.write_char('B');

        let snapshot = term.snapshot();
        assert_eq!(snapshot.row_text(0), "B");

        term.leave_alt_screen();
        assert!(!term.mode.alt_screen);

        let snapshot = term.snapshot();
        assert_eq!(snapshot.row_text(0), "A");
    }

    #[test]
    fn test_term_scrollback() {
        let mut term = Term::new(3, 10);

        // Fill screen and scroll
        for i in 0..5 {
            for c in format!("Line {}", i).chars() {
                term.write_char(c);
            }
            term.linefeed();
            term.carriage_return();
        }

        // Should have scrolled lines into scrollback
        assert!(term.scrollback().len() > 0);
    }

    #[test]
    fn test_term_dec_special_graphics() {
        let mut term = Term::new(24, 80);
        term.set_charset(0, Charset::DecSpecialGraphics);
        term.set_active_charset(0);

        term.write_char('q'); // Should become horizontal line
        let snapshot = term.snapshot();
        assert_eq!(snapshot.row_text(0), "─");
    }

    #[test]
    fn test_term_save_restore_cursor() {
        let mut term = Term::new(24, 80);
        term.goto(5, 10);
        term.set_bold(true);

        term.save_cursor();

        term.goto(0, 0);
        term.set_bold(false);

        term.restore_cursor();

        assert_eq!(term.screen().cursor.row, 5);
        assert_eq!(term.screen().cursor.col, 10);
        assert!(term
            .screen()
            .cursor
            .attrs
            .flags
            .contains(crate::cell::CellFlags::BOLD));
    }
}
