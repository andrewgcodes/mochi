//! Terminal mode flags
//!
//! Manages various terminal modes that affect behavior.

use serde::{Deserialize, Serialize};

/// Mouse reporting mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MouseMode {
    /// No mouse reporting
    #[default]
    None,
    /// X10 compatibility mode - report button press only
    X10,
    /// Normal tracking mode - report button press and release
    Normal,
    /// Button-event tracking - report press, release, and motion while button pressed
    ButtonMotion,
    /// Any-event tracking - report all motion events
    AnyMotion,
}

/// Mouse encoding format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MouseEncoding {
    /// Default X10 encoding (limited to 223 columns/rows)
    #[default]
    X10,
    /// UTF-8 encoding (extends range)
    Utf8,
    /// SGR encoding (CSI < ... M/m) - recommended
    Sgr,
    /// URXVT encoding
    Urxvt,
}

/// Terminal mode flags
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modes {
    /// DECAWM - Auto wrap mode
    /// When enabled, characters written past the right margin wrap to the next line
    pub autowrap: bool,

    /// DECOM - Origin mode
    /// When enabled, cursor positions are relative to the scroll region
    pub origin: bool,

    /// IRM - Insert/Replace mode
    /// When enabled, characters are inserted rather than overwriting
    pub insert: bool,

    /// LNM - Line feed/new line mode
    /// When enabled, LF also performs CR
    pub linefeed_newline: bool,

    /// DECCKM - Cursor key mode
    /// When enabled, cursor keys send application sequences (ESC O ...) instead of (ESC [ ...)
    pub cursor_keys_application: bool,

    /// DECKPAM/DECKPNM - Keypad mode
    /// When enabled, keypad sends application sequences
    pub keypad_application: bool,

    /// Bracketed paste mode (DECSET 2004)
    /// When enabled, pasted text is wrapped with escape sequences
    pub bracketed_paste: bool,

    /// Mouse reporting mode
    pub mouse_mode: MouseMode,

    /// Mouse encoding format
    pub mouse_encoding: MouseEncoding,

    /// Focus reporting (DECSET 1004)
    /// When enabled, terminal sends focus in/out events
    pub focus_reporting: bool,

    /// Alternate screen buffer active (DECSET 1049/47)
    pub alternate_screen: bool,

    /// DECSCNM - Screen mode (reverse video)
    /// When enabled, screen colors are inverted
    pub reverse_video: bool,

    /// DECSCLM - Scrolling mode
    /// When enabled, smooth scrolling is used (we ignore this)
    pub smooth_scroll: bool,

    /// DECARM - Auto repeat mode
    /// When enabled, keys auto-repeat (handled by OS, we ignore this)
    pub auto_repeat: bool,

    /// Show cursor (DECTCEM - DECSET 25)
    pub cursor_visible: bool,

    /// Column mode (DECCOLM - 132 vs 80 columns)
    /// We track this but don't automatically resize
    pub column_132: bool,

    /// Send/receive mode (SRM)
    /// When enabled, local echo is disabled
    pub send_receive: bool,
}

impl Default for Modes {
    fn default() -> Self {
        Self {
            autowrap: true,
            origin: false,
            insert: false,
            linefeed_newline: false,
            cursor_keys_application: false,
            keypad_application: false,
            bracketed_paste: false,
            mouse_mode: MouseMode::None,
            mouse_encoding: MouseEncoding::X10,
            focus_reporting: false,
            alternate_screen: false,
            reverse_video: false,
            smooth_scroll: false,
            auto_repeat: true,
            cursor_visible: true,
            column_132: false,
            send_receive: false,
        }
    }
}

impl Modes {
    /// Create new default modes
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all modes to default values
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Set a DEC private mode (CSI ? Ps h)
    /// Returns true if the mode was recognized
    pub fn set_dec_mode(&mut self, mode: u16) -> bool {
        match mode {
            1 => {
                self.cursor_keys_application = true;
                true
            },
            3 => {
                self.column_132 = true;
                true
            },
            5 => {
                self.reverse_video = true;
                true
            },
            6 => {
                self.origin = true;
                true
            },
            7 => {
                self.autowrap = true;
                true
            },
            12 => {
                // Cursor blink - handled separately
                true
            },
            25 => {
                self.cursor_visible = true;
                true
            },
            47 => {
                self.alternate_screen = true;
                true
            },
            1000 => {
                self.mouse_mode = MouseMode::Normal;
                true
            },
            1002 => {
                self.mouse_mode = MouseMode::ButtonMotion;
                true
            },
            1003 => {
                self.mouse_mode = MouseMode::AnyMotion;
                true
            },
            1004 => {
                self.focus_reporting = true;
                true
            },
            1005 => {
                self.mouse_encoding = MouseEncoding::Utf8;
                true
            },
            1006 => {
                self.mouse_encoding = MouseEncoding::Sgr;
                true
            },
            1015 => {
                self.mouse_encoding = MouseEncoding::Urxvt;
                true
            },
            1049 => {
                self.alternate_screen = true;
                true
            },
            2004 => {
                self.bracketed_paste = true;
                true
            },
            _ => false,
        }
    }

    /// Reset a DEC private mode (CSI ? Ps l)
    /// Returns true if the mode was recognized
    pub fn reset_dec_mode(&mut self, mode: u16) -> bool {
        match mode {
            1 => {
                self.cursor_keys_application = false;
                true
            },
            3 => {
                self.column_132 = false;
                true
            },
            5 => {
                self.reverse_video = false;
                true
            },
            6 => {
                self.origin = false;
                true
            },
            7 => {
                self.autowrap = false;
                true
            },
            12 => {
                // Cursor blink - handled separately
                true
            },
            25 => {
                self.cursor_visible = false;
                true
            },
            47 => {
                self.alternate_screen = false;
                true
            },
            1000 | 1002 | 1003 => {
                self.mouse_mode = MouseMode::None;
                true
            },
            1004 => {
                self.focus_reporting = false;
                true
            },
            1005 | 1006 | 1015 => {
                self.mouse_encoding = MouseEncoding::X10;
                true
            },
            1049 => {
                self.alternate_screen = false;
                true
            },
            2004 => {
                self.bracketed_paste = false;
                true
            },
            _ => false,
        }
    }

    /// Set an ANSI mode (CSI Ps h)
    /// Returns true if the mode was recognized
    pub fn set_ansi_mode(&mut self, mode: u16) -> bool {
        match mode {
            4 => {
                self.insert = true;
                true
            },
            12 => {
                self.send_receive = false;
                true
            },
            20 => {
                self.linefeed_newline = true;
                true
            },
            _ => false,
        }
    }

    /// Reset an ANSI mode (CSI Ps l)
    /// Returns true if the mode was recognized
    pub fn reset_ansi_mode(&mut self, mode: u16) -> bool {
        match mode {
            4 => {
                self.insert = false;
                true
            },
            12 => {
                self.send_receive = true;
                true
            },
            20 => {
                self.linefeed_newline = false;
                true
            },
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modes_default() {
        let modes = Modes::new();
        assert!(modes.autowrap);
        assert!(!modes.origin);
        assert!(!modes.insert);
        assert!(modes.cursor_visible);
        assert!(!modes.bracketed_paste);
        assert_eq!(modes.mouse_mode, MouseMode::None);
    }

    #[test]
    fn test_dec_mode_set_reset() {
        let mut modes = Modes::new();

        // Set bracketed paste
        assert!(modes.set_dec_mode(2004));
        assert!(modes.bracketed_paste);

        // Reset bracketed paste
        assert!(modes.reset_dec_mode(2004));
        assert!(!modes.bracketed_paste);
    }

    #[test]
    fn test_mouse_modes() {
        let mut modes = Modes::new();

        modes.set_dec_mode(1000);
        assert_eq!(modes.mouse_mode, MouseMode::Normal);

        modes.set_dec_mode(1002);
        assert_eq!(modes.mouse_mode, MouseMode::ButtonMotion);

        modes.set_dec_mode(1003);
        assert_eq!(modes.mouse_mode, MouseMode::AnyMotion);

        modes.reset_dec_mode(1003);
        assert_eq!(modes.mouse_mode, MouseMode::None);
    }

    #[test]
    fn test_mouse_encoding() {
        let mut modes = Modes::new();

        modes.set_dec_mode(1006);
        assert_eq!(modes.mouse_encoding, MouseEncoding::Sgr);

        modes.reset_dec_mode(1006);
        assert_eq!(modes.mouse_encoding, MouseEncoding::X10);
    }

    #[test]
    fn test_ansi_modes() {
        let mut modes = Modes::new();

        modes.set_ansi_mode(4);
        assert!(modes.insert);

        modes.reset_ansi_mode(4);
        assert!(!modes.insert);
    }

    #[test]
    fn test_unknown_mode() {
        let mut modes = Modes::new();
        assert!(!modes.set_dec_mode(9999));
        assert!(!modes.reset_dec_mode(9999));
    }
}
