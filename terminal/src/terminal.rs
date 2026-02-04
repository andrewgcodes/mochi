//! Terminal Executor
//!
//! Ties together the parser, screen model, and applies parsed actions
//! to update the terminal state. This is the main integration point
//! between parsing and the screen model.

use crate::core::{Cell, Color, Screen, Style};
use crate::parser::{Action, CsiAction, EscAction, OscAction, Parser, SgrAttribute};

/// Terminal executor that processes parsed actions and updates the screen
pub struct Terminal {
    /// The terminal screen
    screen: Screen,
    /// The escape sequence parser
    parser: Parser,
}

impl Terminal {
    /// Create a new terminal with the given dimensions
    pub fn new(cols: usize, rows: usize, scrollback_capacity: usize) -> Self {
        Self {
            screen: Screen::new(cols, rows, scrollback_capacity),
            parser: Parser::new(),
        }
    }

    /// Get a reference to the screen
    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    /// Get a mutable reference to the screen
    pub fn screen_mut(&mut self) -> &mut Screen {
        &mut self.screen
    }

    /// Process input bytes from the PTY
    pub fn process(&mut self, data: &[u8]) {
        let actions = self.parser.parse(data);
        for action in actions {
            self.apply_action(action);
        }
    }

    /// Apply a single parsed action to the screen
    fn apply_action(&mut self, action: Action) {
        match action {
            Action::Print(c) => {
                self.screen.print_char(c);
            }
            Action::Execute(byte) => {
                self.execute_c0(byte);
            }
            Action::CsiDispatch(csi) => {
                self.execute_csi(csi);
            }
            Action::EscDispatch(esc) => {
                self.execute_esc(esc);
            }
            Action::OscDispatch(osc) => {
                self.execute_osc(osc);
            }
            Action::DcsDispatch(_) => {
                // DCS sequences are currently not implemented
                log::debug!("DCS sequence ignored");
            }
            Action::ApcDispatch(_) | Action::PmDispatch(_) | Action::SosDispatch(_) => {
                // These are ignored
            }
            Action::Invalid(bytes) => {
                log::warn!("Invalid sequence: {:?}", bytes);
            }
        }
    }

    /// Execute a C0 control character
    fn execute_c0(&mut self, byte: u8) {
        match byte {
            0x07 => {
                // BEL - Bell
                log::debug!("Bell");
                // Could trigger audio/visual bell here
            }
            0x08 => {
                // BS - Backspace
                self.screen.backspace();
            }
            0x09 => {
                // HT - Horizontal Tab
                self.screen.tab();
            }
            0x0A | 0x0B | 0x0C => {
                // LF, VT, FF - Line Feed (VT and FF treated as LF)
                self.screen.linefeed();
            }
            0x0D => {
                // CR - Carriage Return
                self.screen.carriage_return();
            }
            0x0E => {
                // SO - Shift Out (switch to G1 charset)
                // Not fully implemented, but acknowledged
                log::debug!("Shift Out (G1)");
            }
            0x0F => {
                // SI - Shift In (switch to G0 charset)
                log::debug!("Shift In (G0)");
            }
            _ => {
                // Other C0 controls are ignored
            }
        }
    }

    /// Execute a CSI sequence
    fn execute_csi(&mut self, csi: CsiAction) {
        if csi.private {
            self.execute_csi_private(&csi);
            return;
        }

        match csi.final_byte {
            // Cursor movement
            b'A' => {
                // CUU - Cursor Up
                let n = csi.param_or_default(0, 1) as usize;
                self.screen.move_cursor_up(n);
            }
            b'B' | b'e' => {
                // CUD - Cursor Down, VPR - Vertical Position Relative
                let n = csi.param_or_default(0, 1) as usize;
                self.screen.move_cursor_down(n);
            }
            b'C' | b'a' => {
                // CUF - Cursor Forward, HPR - Horizontal Position Relative
                let n = csi.param_or_default(0, 1) as usize;
                self.screen.move_cursor_forward(n);
            }
            b'D' => {
                // CUB - Cursor Backward
                let n = csi.param_or_default(0, 1) as usize;
                self.screen.move_cursor_backward(n);
            }
            b'E' => {
                // CNL - Cursor Next Line
                let n = csi.param_or_default(0, 1) as usize;
                self.screen.move_cursor_down(n);
                self.screen.cursor_mut().col = 0;
            }
            b'F' => {
                // CPL - Cursor Previous Line
                let n = csi.param_or_default(0, 1) as usize;
                self.screen.move_cursor_up(n);
                self.screen.cursor_mut().col = 0;
            }
            b'G' | b'`' => {
                // CHA - Cursor Character Absolute, HPA
                let col = csi.param_or_default(0, 1).saturating_sub(1) as usize;
                self.screen.move_cursor_to_col(col);
            }
            b'H' | b'f' => {
                // CUP - Cursor Position, HVP
                let row = csi.param_or_default(0, 1).saturating_sub(1) as usize;
                let col = csi.param_or_default(1, 1).saturating_sub(1) as usize;
                self.screen.move_cursor_to(row, col);
            }
            b'd' => {
                // VPA - Vertical Position Absolute
                let row = csi.param_or_default(0, 1).saturating_sub(1) as usize;
                self.screen.move_cursor_to_row(row);
            }

            // Erase operations
            b'J' => {
                // ED - Erase in Display
                let mode = csi.param(0, 0);
                self.screen.erase_in_display(mode);
            }
            b'K' => {
                // EL - Erase in Line
                let mode = csi.param(0, 0);
                self.screen.erase_in_line(mode);
            }
            b'X' => {
                // ECH - Erase Characters
                let n = csi.param_or_default(0, 1) as usize;
                self.screen.erase_chars(n);
            }

            // Insert/Delete
            b'L' => {
                // IL - Insert Lines
                let n = csi.param_or_default(0, 1) as usize;
                self.screen.insert_lines(n);
            }
            b'M' => {
                // DL - Delete Lines
                let n = csi.param_or_default(0, 1) as usize;
                self.screen.delete_lines(n);
            }
            b'@' => {
                // ICH - Insert Characters
                let n = csi.param_or_default(0, 1) as usize;
                self.screen.insert_chars(n);
            }
            b'P' => {
                // DCH - Delete Characters
                let n = csi.param_or_default(0, 1) as usize;
                self.screen.delete_chars(n);
            }

            // Scroll
            b'S' => {
                // SU - Scroll Up
                let n = csi.param_or_default(0, 1) as usize;
                self.screen.scroll_up(n);
            }
            b'T' => {
                // SD - Scroll Down
                let n = csi.param_or_default(0, 1) as usize;
                self.screen.scroll_down(n);
            }

            // Scroll region
            b'r' => {
                // DECSTBM - Set Top and Bottom Margins
                let top = csi.param_or_default(0, 1).saturating_sub(1) as usize;
                let bottom = csi
                    .param_or_default(1, self.screen.rows() as u32)
                    .saturating_sub(1) as usize;
                self.screen.set_scroll_region(top, bottom);
            }

            // SGR - Select Graphic Rendition
            b'm' => {
                self.execute_sgr(&csi);
            }

            // Tab operations
            b'g' => {
                // TBC - Tab Clear
                match csi.param(0, 0) {
                    0 => self.screen.clear_tab_stop(),
                    3 => self.screen.clear_all_tab_stops(),
                    _ => {}
                }
            }

            // Cursor save/restore (ANSI)
            b's' => {
                // SCP - Save Cursor Position
                if csi.params.is_empty() {
                    self.screen.save_cursor();
                }
            }
            b'u' => {
                // RCP - Restore Cursor Position
                if csi.params.is_empty() {
                    self.screen.restore_cursor();
                }
            }

            // Mode set/reset
            b'h' => {
                // SM - Set Mode
                self.set_mode(&csi, true);
            }
            b'l' => {
                // RM - Reset Mode
                self.set_mode(&csi, false);
            }

            // Device status reports
            b'n' => {
                // DSR - Device Status Report
                match csi.param(0, 0) {
                    5 => {
                        // Status report - we're OK
                        log::debug!("DSR: Status report requested");
                    }
                    6 => {
                        // Cursor position report
                        log::debug!("DSR: Cursor position report requested");
                    }
                    _ => {}
                }
            }

            // Repeat
            b'b' => {
                // REP - Repeat preceding character
                // Not commonly used, skip for now
            }

            // Soft terminal reset
            b'p' if csi.intermediates.contains(&b'!') => {
                // DECSTR - Soft Terminal Reset
                self.soft_reset();
            }

            _ => {
                log::debug!(
                    "Unhandled CSI: params={:?} intermediates={:?} final={}",
                    csi.params,
                    csi.intermediates,
                    csi.final_byte as char
                );
            }
        }
    }

    /// Execute a private CSI sequence (starts with ?)
    fn execute_csi_private(&mut self, csi: &CsiAction) {
        match csi.final_byte {
            b'h' => {
                // DECSET - DEC Private Mode Set
                for &param in &csi.params {
                    self.set_dec_mode(param, true);
                }
            }
            b'l' => {
                // DECRST - DEC Private Mode Reset
                for &param in &csi.params {
                    self.set_dec_mode(param, false);
                }
            }
            b's' => {
                // Save DEC private mode values
                log::debug!("Save DEC mode: {:?}", csi.params);
            }
            b'r' => {
                // Restore DEC private mode values
                log::debug!("Restore DEC mode: {:?}", csi.params);
            }
            _ => {
                log::debug!(
                    "Unhandled private CSI: params={:?} final={}",
                    csi.params,
                    csi.final_byte as char
                );
            }
        }
    }

    /// Set or reset a DEC private mode
    fn set_dec_mode(&mut self, mode: u32, enable: bool) {
        match mode {
            1 => {
                // DECCKM - Application Cursor Keys
                self.screen.modes.application_cursor = enable;
            }
            3 => {
                // DECCOLM - 132 Column Mode (we don't resize, just acknowledge)
                log::debug!("DECCOLM: {} (ignored)", enable);
            }
            5 => {
                // DECSCNM - Reverse Video
                self.screen.modes.reverse_video = enable;
            }
            6 => {
                // DECOM - Origin Mode
                self.screen.cursor_mut().origin_mode = enable;
                if enable {
                    let scroll_top = self.screen.scroll_top();
                    self.screen.cursor_mut().home(scroll_top);
                } else {
                    self.screen.move_cursor_to(0, 0);
                }
            }
            7 => {
                // DECAWM - Autowrap Mode
                self.screen.cursor_mut().autowrap = enable;
            }
            12 => {
                // Cursor blinking (att610)
                self.screen.cursor_mut().blinking = enable;
            }
            25 => {
                // DECTCEM - Text Cursor Enable Mode
                self.screen.cursor_mut().visible = enable;
            }
            47 => {
                // Alternate screen buffer (old xterm)
                if enable {
                    self.screen.enter_alternate_screen();
                } else {
                    self.screen.exit_alternate_screen();
                }
            }
            66 => {
                // DECNKM - Application Keypad Mode
                self.screen.modes.application_keypad = enable;
            }
            1000 => {
                // X10 mouse reporting
                use crate::core::MouseMode;
                self.screen.modes.mouse_tracking = if enable {
                    MouseMode::X10
                } else {
                    MouseMode::None
                };
            }
            1002 => {
                // Button-event mouse tracking
                use crate::core::MouseMode;
                self.screen.modes.mouse_tracking = if enable {
                    MouseMode::ButtonEvent
                } else {
                    MouseMode::None
                };
            }
            1003 => {
                // Any-event mouse tracking
                use crate::core::MouseMode;
                self.screen.modes.mouse_tracking = if enable {
                    MouseMode::AnyEvent
                } else {
                    MouseMode::None
                };
            }
            1004 => {
                // Focus reporting
                self.screen.modes.focus_reporting = enable;
            }
            1005 => {
                // UTF-8 mouse encoding
                use crate::core::MouseEncoding;
                if enable {
                    self.screen.modes.mouse_encoding = MouseEncoding::Utf8;
                }
            }
            1006 => {
                // SGR mouse encoding
                use crate::core::MouseEncoding;
                if enable {
                    self.screen.modes.mouse_encoding = MouseEncoding::Sgr;
                } else {
                    self.screen.modes.mouse_encoding = MouseEncoding::X10;
                }
            }
            1015 => {
                // URXVT mouse encoding
                use crate::core::MouseEncoding;
                if enable {
                    self.screen.modes.mouse_encoding = MouseEncoding::Urxvt;
                }
            }
            1047 => {
                // Alternate screen buffer
                if enable {
                    self.screen.enter_alternate_screen();
                } else {
                    self.screen.exit_alternate_screen();
                }
            }
            1048 => {
                // Save/restore cursor
                if enable {
                    self.screen.save_cursor();
                } else {
                    self.screen.restore_cursor();
                }
            }
            1049 => {
                // Alternate screen buffer with cursor save/restore
                if enable {
                    self.screen.save_cursor();
                    self.screen.enter_alternate_screen();
                    self.screen.erase_in_display(2);
                } else {
                    self.screen.exit_alternate_screen();
                    self.screen.restore_cursor();
                }
            }
            2004 => {
                // Bracketed paste mode
                self.screen.modes.bracketed_paste = enable;
            }
            _ => {
                log::debug!("Unknown DEC mode: {} = {}", mode, enable);
            }
        }
    }

    /// Set or reset an ANSI mode
    fn set_mode(&mut self, csi: &CsiAction, enable: bool) {
        for &param in &csi.params {
            match param {
                4 => {
                    // IRM - Insert Mode
                    self.screen.cursor_mut().insert_mode = enable;
                }
                20 => {
                    // LNM - Line Feed/New Line Mode
                    self.screen.modes.linefeed_mode = enable;
                }
                _ => {
                    log::debug!("Unknown ANSI mode: {} = {}", param, enable);
                }
            }
        }
    }

    /// Execute SGR (Select Graphic Rendition)
    fn execute_sgr(&mut self, csi: &CsiAction) {
        let attrs = csi.parse_sgr();

        for attr in attrs {
            match attr {
                SgrAttribute::Reset => {
                    self.screen.cursor_mut().reset_attributes();
                }
                SgrAttribute::Bold => {
                    self.screen.cursor_mut().style.bold = true;
                }
                SgrAttribute::Faint => {
                    self.screen.cursor_mut().style.faint = true;
                }
                SgrAttribute::Italic => {
                    self.screen.cursor_mut().style.italic = true;
                }
                SgrAttribute::Underline => {
                    self.screen.cursor_mut().style.underline = true;
                }
                SgrAttribute::SlowBlink | SgrAttribute::RapidBlink => {
                    self.screen.cursor_mut().style.blink = true;
                }
                SgrAttribute::Inverse => {
                    self.screen.cursor_mut().style.inverse = true;
                }
                SgrAttribute::Hidden => {
                    self.screen.cursor_mut().style.hidden = true;
                }
                SgrAttribute::Strikethrough => {
                    self.screen.cursor_mut().style.strikethrough = true;
                }
                SgrAttribute::NormalIntensity => {
                    self.screen.cursor_mut().style.bold = false;
                    self.screen.cursor_mut().style.faint = false;
                }
                SgrAttribute::NotItalic => {
                    self.screen.cursor_mut().style.italic = false;
                }
                SgrAttribute::NotUnderlined => {
                    self.screen.cursor_mut().style.underline = false;
                }
                SgrAttribute::NotBlinking => {
                    self.screen.cursor_mut().style.blink = false;
                }
                SgrAttribute::NotInverse => {
                    self.screen.cursor_mut().style.inverse = false;
                }
                SgrAttribute::NotHidden => {
                    self.screen.cursor_mut().style.hidden = false;
                }
                SgrAttribute::NotStrikethrough => {
                    self.screen.cursor_mut().style.strikethrough = false;
                }
                SgrAttribute::ForegroundIndexed(i) => {
                    self.screen.cursor_mut().fg = Color::Indexed(i);
                }
                SgrAttribute::BackgroundIndexed(i) => {
                    self.screen.cursor_mut().bg = Color::Indexed(i);
                }
                SgrAttribute::DefaultForeground => {
                    self.screen.cursor_mut().fg = Color::Default;
                }
                SgrAttribute::DefaultBackground => {
                    self.screen.cursor_mut().bg = Color::Default;
                }
                SgrAttribute::Foreground256(i) => {
                    self.screen.cursor_mut().fg = Color::Indexed(i);
                }
                SgrAttribute::Background256(i) => {
                    self.screen.cursor_mut().bg = Color::Indexed(i);
                }
                SgrAttribute::ForegroundRgb(r, g, b) => {
                    self.screen.cursor_mut().fg = Color::Rgb(r, g, b);
                }
                SgrAttribute::BackgroundRgb(r, g, b) => {
                    self.screen.cursor_mut().bg = Color::Rgb(r, g, b);
                }
            }
        }
    }

    /// Execute an ESC sequence
    fn execute_esc(&mut self, esc: EscAction) {
        match esc {
            EscAction::SaveCursor => {
                self.screen.save_cursor();
            }
            EscAction::RestoreCursor => {
                self.screen.restore_cursor();
            }
            EscAction::Index => {
                self.screen.index();
            }
            EscAction::ReverseIndex => {
                self.screen.reverse_index();
            }
            EscAction::NextLine => {
                self.screen.next_line();
            }
            EscAction::HorizontalTabSet => {
                self.screen.set_tab_stop();
            }
            EscAction::FullReset => {
                self.screen.reset();
                self.parser.reset();
            }
            EscAction::ApplicationKeypad => {
                self.screen.modes.application_keypad = true;
            }
            EscAction::NormalKeypad => {
                self.screen.modes.application_keypad = false;
            }
            EscAction::DesignateG0(charset) => {
                log::debug!("Designate G0: {}", charset as char);
                // Character set handling would go here
            }
            EscAction::DesignateG1(charset) => {
                log::debug!("Designate G1: {}", charset as char);
            }
            EscAction::DesignateG2(charset) => {
                log::debug!("Designate G2: {}", charset as char);
            }
            EscAction::DesignateG3(charset) => {
                log::debug!("Designate G3: {}", charset as char);
            }
            EscAction::SingleShift2 | EscAction::SingleShift3 => {
                // Single shift - not commonly used
            }
            EscAction::Unknown(bytes) => {
                log::debug!("Unknown ESC sequence: {:?}", bytes);
            }
        }
    }

    /// Execute an OSC sequence
    fn execute_osc(&mut self, osc: OscAction) {
        match osc {
            OscAction::SetTitle(title) => {
                self.screen.title = title;
            }
            OscAction::SetIconName(_name) => {
                // Icon name is typically ignored in modern terminals
            }
            OscAction::SetColor { index, color } => {
                log::debug!("Set color {}: {}", index, color);
                // Color setting would go here
            }
            OscAction::Hyperlink { params, uri } => {
                if uri.is_empty() {
                    // End hyperlink
                    self.screen.cursor_mut().hyperlink_id = None;
                } else {
                    // Start hyperlink
                    let id = self.screen.register_hyperlink(uri);
                    self.screen.cursor_mut().hyperlink_id = Some(id);
                }
                log::debug!("Hyperlink params: {}", params);
            }
            OscAction::Clipboard { clipboard, data } => {
                // OSC 52 clipboard - security sensitive!
                log::info!(
                    "Clipboard request: clipboard={}, data_len={}",
                    clipboard,
                    data.len()
                );
                // Actual clipboard handling would require user consent
            }
            OscAction::ResetColor(index) => {
                log::debug!("Reset color {}", index);
            }
            OscAction::Unknown { command, data } => {
                log::debug!("Unknown OSC {}: {}", command, data);
            }
        }
    }

    /// Perform a soft terminal reset
    fn soft_reset(&mut self) {
        // Reset cursor attributes
        self.screen.cursor_mut().reset_attributes();
        self.screen.cursor_mut().origin_mode = false;
        self.screen.cursor_mut().autowrap = true;
        self.screen.cursor_mut().insert_mode = false;

        // Reset scroll region
        self.screen.reset_scroll_region();

        // Reset modes
        self.screen.modes.application_cursor = false;
        self.screen.modes.application_keypad = false;
        self.screen.modes.bracketed_paste = false;

        log::debug!("Soft reset performed");
    }

    /// Resize the terminal
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.screen.resize(cols, rows);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Snapshot;

    #[test]
    fn test_terminal_print() {
        let mut term = Terminal::new(80, 24, 1000);
        term.process(b"Hello, World!");

        let snapshot = Snapshot::from_screen(term.screen());
        assert!(snapshot.to_text().contains("Hello, World!"));
    }

    #[test]
    fn test_terminal_cursor_movement() {
        let mut term = Terminal::new(80, 24, 1000);

        // Move cursor to position 5,10 and print
        term.process(b"\x1b[10;5HX");

        assert_eq!(term.screen().cursor().row, 9);
        assert_eq!(term.screen().cursor().col, 5); // After printing X
    }

    #[test]
    fn test_terminal_colors() {
        let mut term = Terminal::new(80, 24, 1000);

        // Set red foreground, blue background
        term.process(b"\x1b[31;44mColored");

        assert_eq!(term.screen().cursor().fg, Color::Indexed(1)); // Red
        assert_eq!(term.screen().cursor().bg, Color::Indexed(4)); // Blue
    }

    #[test]
    fn test_terminal_sgr_reset() {
        let mut term = Terminal::new(80, 24, 1000);

        term.process(b"\x1b[1;31mBold Red\x1b[0mNormal");

        // After reset, should be default
        assert_eq!(term.screen().cursor().fg, Color::Default);
        assert!(!term.screen().cursor().style.bold);
    }

    #[test]
    fn test_terminal_erase() {
        let mut term = Terminal::new(10, 3, 1000);

        term.process(b"XXXXXXXXXX");
        term.process(b"\x1b[1;5H"); // Move to row 1, col 5
        term.process(b"\x1b[K"); // Erase to end of line

        let snapshot = Snapshot::from_screen(term.screen());
        let text = snapshot.to_text();
        assert!(text.starts_with("XXXX"));
        assert!(!text.contains("XXXXX"));
    }

    #[test]
    fn test_terminal_scroll_region() {
        let mut term = Terminal::new(80, 5, 1000);

        // Set scroll region to lines 2-4
        term.process(b"\x1b[2;4r");

        assert_eq!(term.screen().scroll_top(), 1);
        assert_eq!(term.screen().scroll_bottom(), 3);
    }

    #[test]
    fn test_terminal_alternate_screen() {
        let mut term = Terminal::new(80, 24, 1000);

        term.process(b"Primary");
        term.process(b"\x1b[?1049h"); // Enter alternate screen
        term.process(b"Alternate");

        assert!(term.screen().modes.alternate_screen);

        term.process(b"\x1b[?1049l"); // Exit alternate screen

        assert!(!term.screen().modes.alternate_screen);
        let snapshot = Snapshot::from_screen(term.screen());
        assert!(snapshot.to_text().contains("Primary"));
    }

    #[test]
    fn test_terminal_bracketed_paste() {
        let mut term = Terminal::new(80, 24, 1000);

        term.process(b"\x1b[?2004h"); // Enable bracketed paste
        assert!(term.screen().modes.bracketed_paste);

        term.process(b"\x1b[?2004l"); // Disable
        assert!(!term.screen().modes.bracketed_paste);
    }

    #[test]
    fn test_terminal_title() {
        let mut term = Terminal::new(80, 24, 1000);

        term.process(b"\x1b]0;My Terminal Title\x07");

        assert_eq!(term.screen().title, "My Terminal Title");
    }

    #[test]
    fn test_terminal_insert_delete_lines() {
        let mut term = Terminal::new(10, 5, 1000);

        // Fill with line numbers
        term.process(b"Line 1\r\nLine 2\r\nLine 3\r\nLine 4\r\nLine 5");

        // Move to line 2 and insert a line
        term.process(b"\x1b[2;1H\x1b[L");

        let snapshot = Snapshot::from_screen(term.screen());
        let text = snapshot.to_text();

        // Line 1 should still be there, then blank, then Line 2 shifted down
        assert!(text.contains("Line 1"));
    }

    #[test]
    fn test_terminal_truecolor() {
        let mut term = Terminal::new(80, 24, 1000);

        // Set RGB foreground
        term.process(b"\x1b[38;2;255;128;64mOrange");

        assert_eq!(term.screen().cursor().fg, Color::Rgb(255, 128, 64));
    }
}
