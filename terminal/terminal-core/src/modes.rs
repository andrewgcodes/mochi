//! Terminal mode flags
//!
//! Various modes that affect terminal behavior, including DEC private modes.

use serde::{Deserialize, Serialize};

/// Terminal mode flags
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modes {
    // Standard modes (ANSI)
    /// Insert mode (IRM) - characters shift right instead of overwriting
    pub insert_mode: bool,
    /// Automatic newline mode (LNM) - LF also does CR
    pub linefeed_mode: bool,

    // DEC private modes
    /// DECCKM - Cursor key mode (application vs normal)
    pub cursor_keys_application: bool,
    /// DECANM - ANSI mode (vs VT52)
    pub ansi_mode: bool,
    /// DECCOLM - 132 column mode (vs 80)
    pub column_132: bool,
    /// DECSCLM - Smooth scroll mode
    pub smooth_scroll: bool,
    /// DECSCNM - Reverse video mode
    pub reverse_video: bool,
    /// DECOM - Origin mode (cursor relative to scroll region)
    pub origin_mode: bool,
    /// DECAWM - Auto-wrap mode
    pub auto_wrap: bool,
    /// DECARM - Auto-repeat mode
    pub auto_repeat: bool,
    /// DECTCEM - Cursor visible
    pub cursor_visible: bool,

    // xterm extensions
    /// Mouse tracking: X10 mode (button press only)
    pub mouse_x10: bool,
    /// Mouse tracking: VT200 mode (button press/release)
    pub mouse_vt200: bool,
    /// Mouse tracking: button event tracking
    pub mouse_button_event: bool,
    /// Mouse tracking: any event tracking
    pub mouse_any_event: bool,
    /// Mouse tracking: UTF-8 extended coordinates (mode 1005)
    pub mouse_utf8: bool,
    /// Mouse tracking: SGR extended coordinates
    pub mouse_sgr: bool,
    /// Focus in/out events
    pub focus_events: bool,
    /// Cursor blink (mode 12)
    pub cursor_blink: bool,
    /// Alternate screen buffer
    pub alternate_screen: bool,
    /// Alternate scroll mode (mode 1007) - scroll wheel sends arrow keys in alt screen
    pub alternate_scroll: bool,
    /// Bracketed paste mode
    pub bracketed_paste: bool,
    /// Synchronized output mode (DEC 2026) - used by TUI apps like Claude Code
    /// When enabled, the terminal should buffer output until the mode is disabled
    pub synchronized_output: bool,
}

impl Modes {
    /// Create new modes with default values
    pub fn new() -> Self {
        Self {
            // Standard modes
            insert_mode: false,
            linefeed_mode: false,

            // DEC private modes
            cursor_keys_application: false,
            ansi_mode: true,
            column_132: false,
            smooth_scroll: false,
            reverse_video: false,
            origin_mode: false,
            auto_wrap: true, // Usually enabled by default
            auto_repeat: true,
            cursor_visible: true,

            // xterm extensions
            mouse_x10: false,
            mouse_vt200: false,
            mouse_button_event: false,
            mouse_any_event: false,
            mouse_utf8: false,
            mouse_sgr: false,
            focus_events: false,
            cursor_blink: true,
            alternate_screen: false,
            alternate_scroll: false,
            bracketed_paste: false,
            synchronized_output: false,
        }
    }

    /// Reset all modes to default
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Set a DEC private mode by number
    pub fn set_dec_mode(&mut self, mode: u16, value: bool) {
        match mode {
            1 => self.cursor_keys_application = value,
            2 => self.ansi_mode = value,
            3 => self.column_132 = value,
            4 => self.smooth_scroll = value,
            5 => self.reverse_video = value,
            6 => self.origin_mode = value,
            7 => self.auto_wrap = value,
            8 => self.auto_repeat = value,
            9 => self.mouse_x10 = value,
            12 => self.cursor_blink = value,
            25 => self.cursor_visible = value,
            47 => self.alternate_screen = value,
            1000 => self.mouse_vt200 = value,
            1002 => self.mouse_button_event = value,
            1003 => self.mouse_any_event = value,
            1004 => self.focus_events = value,
            1005 => self.mouse_utf8 = value,
            1006 => self.mouse_sgr = value,
            1007 => self.alternate_scroll = value,
            1047 => self.alternate_screen = value,
            1049 => self.alternate_screen = value,
            2004 => self.bracketed_paste = value,
            2026 => self.synchronized_output = value,
            _ => {
                log::debug!("Unknown DEC private mode: {}", mode);
            }
        }
    }

    /// Get a DEC private mode by number
    pub fn get_dec_mode(&self, mode: u16) -> bool {
        match mode {
            1 => self.cursor_keys_application,
            2 => self.ansi_mode,
            3 => self.column_132,
            4 => self.smooth_scroll,
            5 => self.reverse_video,
            6 => self.origin_mode,
            7 => self.auto_wrap,
            8 => self.auto_repeat,
            9 => self.mouse_x10,
            12 => self.cursor_blink,
            25 => self.cursor_visible,
            47 => self.alternate_screen,
            1000 => self.mouse_vt200,
            1002 => self.mouse_button_event,
            1003 => self.mouse_any_event,
            1004 => self.focus_events,
            1005 => self.mouse_utf8,
            1006 => self.mouse_sgr,
            1007 => self.alternate_scroll,
            1047 => self.alternate_screen,
            1049 => self.alternate_screen,
            2004 => self.bracketed_paste,
            2026 => self.synchronized_output,
            _ => false,
        }
    }

    /// Set a standard (non-DEC) mode by number
    pub fn set_mode(&mut self, mode: u16, value: bool) {
        match mode {
            4 => self.insert_mode = value,
            20 => self.linefeed_mode = value,
            _ => {
                log::debug!("Unknown standard mode: {}", mode);
            }
        }
    }

    /// Check if any mouse mode is active
    pub fn mouse_tracking_enabled(&self) -> bool {
        self.mouse_x10 || self.mouse_vt200 || self.mouse_button_event || self.mouse_any_event
    }
}

impl Default for Modes {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modes_default() {
        let modes = Modes::new();
        assert!(modes.auto_wrap);
        assert!(modes.cursor_visible);
        assert!(!modes.alternate_screen);
        assert!(!modes.bracketed_paste);
    }

    #[test]
    fn test_set_dec_mode() {
        let mut modes = Modes::new();

        modes.set_dec_mode(25, false);
        assert!(!modes.cursor_visible);

        modes.set_dec_mode(1049, true);
        assert!(modes.alternate_screen);

        modes.set_dec_mode(2004, true);
        assert!(modes.bracketed_paste);
    }

    #[test]
    fn test_get_dec_mode() {
        let modes = Modes::new();
        assert!(modes.get_dec_mode(25)); // cursor visible
        assert!(!modes.get_dec_mode(1049)); // alternate screen
    }

    #[test]
    fn test_mouse_tracking() {
        let mut modes = Modes::new();
        assert!(!modes.mouse_tracking_enabled());

        modes.mouse_vt200 = true;
        assert!(modes.mouse_tracking_enabled());
    }

    #[test]
    fn test_modes_reset() {
        let mut modes = Modes::new();
        modes.cursor_visible = false;
        modes.alternate_screen = true;

        modes.reset();

        assert!(modes.cursor_visible);
        assert!(!modes.alternate_screen);
    }
}
