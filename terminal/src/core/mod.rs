//! Terminal Core Module
//!
//! This module contains the platform-independent terminal emulation logic:
//! - Screen model (grid, cells, cursor)
//! - Scrollback buffer
//! - Terminal state and modes
//! - Attribute handling (SGR)

mod cell;
mod cursor;
mod grid;
mod screen;
mod scrollback;

// Re-exports for library users - some may appear unused in binary crates
#[allow(unused_imports)]
pub use cell::{Cell, Color, Hyperlink, Style};
#[allow(unused_imports)]
pub use cursor::{Cursor, CursorStyle};
#[allow(unused_imports)]
pub use grid::Grid;
#[allow(unused_imports)]
pub use screen::{EraseMode, Screen, ScreenSnapshot, TabClearMode};
#[allow(unused_imports)]
pub use scrollback::Scrollback;

/// Terminal modes that can be set/reset via escape sequences
#[derive(Debug, Clone, Default)]
pub struct Modes {
    /// DECCKM: Application cursor keys mode
    pub application_cursor_keys: bool,
    /// DECAWM: Auto-wrap mode (wrap at end of line)
    pub auto_wrap: bool,
    /// DECOM: Origin mode (cursor relative to scroll region)
    pub origin_mode: bool,
    /// IRM: Insert mode (insert characters instead of overwrite)
    pub insert_mode: bool,
    /// LNM: Line feed/new line mode
    pub linefeed_mode: bool,
    /// DECTCEM: Cursor visible
    pub cursor_visible: bool,
    /// Alternate screen buffer active
    pub alternate_screen: bool,
    /// Bracketed paste mode
    pub bracketed_paste: bool,
    /// Mouse tracking modes
    pub mouse_tracking: MouseMode,
    /// Mouse encoding format
    pub mouse_encoding: MouseEncoding,
    /// Focus reporting
    pub focus_reporting: bool,
}

impl Modes {
    pub fn new() -> Self {
        Self {
            auto_wrap: true,
            cursor_visible: true,
            ..Default::default()
        }
    }
}

/// Mouse tracking mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MouseMode {
    #[default]
    None,
    /// X10 mouse reporting (button press only)
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

/// Mouse encoding format
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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

/// Tab stops management
#[derive(Debug, Clone)]
pub struct TabStops {
    stops: Vec<bool>,
    default_interval: usize,
}

impl TabStops {
    pub fn new(cols: usize) -> Self {
        let mut stops = vec![false; cols];
        // Default tab stops every 8 columns
        for i in (8..cols).step_by(8) {
            stops[i] = true;
        }
        Self {
            stops,
            default_interval: 8,
        }
    }

    pub fn resize(&mut self, cols: usize) {
        let old_len = self.stops.len();
        self.stops.resize(cols, false);
        // Set default tab stops for new columns
        for i in (old_len..cols).filter(|&i| i % self.default_interval == 0) {
            self.stops[i] = true;
        }
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
        self.stops.fill(false);
    }

    /// Find the next tab stop after the given column
    pub fn next_stop(&self, col: usize) -> usize {
        for i in (col + 1)..self.stops.len() {
            if self.stops[i] {
                return i;
            }
        }
        // If no tab stop found, go to last column
        self.stops.len().saturating_sub(1)
    }

    /// Find the previous tab stop before the given column
    pub fn prev_stop(&self, col: usize) -> usize {
        for i in (0..col).rev() {
            if self.stops[i] {
                return i;
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_stops_default() {
        let tabs = TabStops::new(80);
        assert_eq!(tabs.next_stop(0), 8);
        assert_eq!(tabs.next_stop(7), 8);
        assert_eq!(tabs.next_stop(8), 16);
        assert_eq!(tabs.next_stop(15), 16);
    }

    #[test]
    fn test_tab_stops_set_clear() {
        let mut tabs = TabStops::new(80);
        tabs.set(5);
        assert_eq!(tabs.next_stop(0), 5);
        tabs.clear(5);
        assert_eq!(tabs.next_stop(0), 8);
    }

    #[test]
    fn test_tab_stops_clear_all() {
        let mut tabs = TabStops::new(80);
        tabs.clear_all();
        assert_eq!(tabs.next_stop(0), 79);
    }
}
