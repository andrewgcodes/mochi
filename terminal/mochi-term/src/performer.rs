//! Action performer
//!
//! This module translates parser actions into terminal state changes.
//! It bridges the parser output to the terminal core.

use log::{debug, trace, warn};
use mochi_core::color::{Color, NamedColor, Rgb};
use mochi_core::cursor::CursorStyle;
use mochi_core::screen::{MouseEncoding, MouseMode};
use mochi_core::term::{Charset, Term};
use mochi_parser::action::{c0, Action, CsiAction, EscAction, OscAction};

/// Performs actions on the terminal
pub struct Performer<'a> {
    term: &'a mut Term,
}

impl<'a> Performer<'a> {
    pub fn new(term: &'a mut Term) -> Self {
        Performer { term }
    }

    /// Perform an action
    pub fn perform(&mut self, action: Action) {
        match action {
            Action::Print(c) => self.print(c),
            Action::Execute(byte) => self.execute(byte),
            Action::CsiDispatch(csi) => self.csi_dispatch(csi),
            Action::EscDispatch(esc) => self.esc_dispatch(esc),
            Action::OscDispatch(osc) => self.osc_dispatch(osc),
            Action::DcsHook(_) => {
                // DCS sequences are not fully implemented
                trace!("DCS hook (ignored)");
            }
            Action::DcsPut(_) => {
                trace!("DCS put (ignored)");
            }
            Action::DcsUnhook => {
                trace!("DCS unhook (ignored)");
            }
            Action::DcsDispatch(_) => {
                trace!("DCS dispatch (ignored)");
            }
            Action::ApcDispatch(_) => {
                trace!("APC dispatch (ignored)");
            }
            Action::PmDispatch(_) => {
                trace!("PM dispatch (ignored)");
            }
            Action::SosDispatch(_) => {
                trace!("SOS dispatch (ignored)");
            }
        }
    }

    /// Print a character
    fn print(&mut self, c: char) {
        self.term.write_char(c);
    }

    /// Execute a C0 control character
    fn execute(&mut self, byte: u8) {
        match byte {
            c0::BEL => self.term.bell(),
            c0::BS => self.term.backspace(),
            c0::HT => self.term.tab(),
            c0::LF | c0::VT | c0::FF => {
                self.term.linefeed();
                // In linefeed mode, LF also does CR
                if self.term.screen_mode().linefeed_mode {
                    self.term.carriage_return();
                }
            }
            c0::CR => self.term.carriage_return(),
            c0::SO => self.term.set_active_charset(1), // G1
            c0::SI => self.term.set_active_charset(0), // G0
            _ => {
                trace!("Unhandled C0 control: 0x{:02X}", byte);
            }
        }
    }

    /// Dispatch a CSI sequence
    fn csi_dispatch(&mut self, csi: CsiAction) {
        if csi.private {
            self.csi_private(&csi);
        } else if !csi.intermediates.is_empty() {
            self.csi_intermediate(&csi);
        } else {
            self.csi_standard(&csi);
        }
    }

    /// Handle standard CSI sequences
    fn csi_standard(&mut self, csi: &CsiAction) {
        match csi.final_byte {
            // Cursor movement
            b'A' => {
                // CUU - Cursor Up
                let n = csi.param_or_one(0) as usize;
                self.term.move_up(n);
            }
            b'B' | b'e' => {
                // CUD - Cursor Down / VPR - Vertical Position Relative
                let n = csi.param_or_one(0) as usize;
                self.term.move_down(n);
            }
            b'C' | b'a' => {
                // CUF - Cursor Forward / HPR - Horizontal Position Relative
                let n = csi.param_or_one(0) as usize;
                self.term.move_right(n);
            }
            b'D' => {
                // CUB - Cursor Back
                let n = csi.param_or_one(0) as usize;
                self.term.move_left(n);
            }
            b'E' => {
                // CNL - Cursor Next Line
                let n = csi.param_or_one(0) as usize;
                self.term.move_down(n);
                self.term.carriage_return();
            }
            b'F' => {
                // CPL - Cursor Previous Line
                let n = csi.param_or_one(0) as usize;
                self.term.move_up(n);
                self.term.carriage_return();
            }
            b'G' | b'`' => {
                // CHA - Cursor Horizontal Absolute / HPA
                let col = csi.param_or_one(0) as usize;
                self.term.goto_col(col.saturating_sub(1));
            }
            b'H' | b'f' => {
                // CUP - Cursor Position / HVP
                let row = csi.param_or_one(0) as usize;
                let col = csi.param_or_one(1) as usize;
                self.term.goto(row.saturating_sub(1), col.saturating_sub(1));
            }
            b'd' => {
                // VPA - Vertical Position Absolute
                let row = csi.param_or_one(0) as usize;
                self.term.goto_row(row.saturating_sub(1));
            }

            // Erase
            b'J' => {
                // ED - Erase in Display
                match csi.param(0, 0) {
                    0 => self.term.erase_below(),
                    1 => self.term.erase_above(),
                    2 | 3 => self.term.erase_screen(),
                    _ => {}
                }
            }
            b'K' => {
                // EL - Erase in Line
                match csi.param(0, 0) {
                    0 => self.term.erase_to_eol(),
                    1 => self.term.erase_to_bol(),
                    2 => self.term.erase_line(),
                    _ => {}
                }
            }
            b'X' => {
                // ECH - Erase Character
                let n = csi.param_or_one(0) as usize;
                self.term.erase_chars(n);
            }

            // Insert/Delete
            b'@' => {
                // ICH - Insert Character
                let n = csi.param_or_one(0) as usize;
                self.term.insert_chars(n);
            }
            b'P' => {
                // DCH - Delete Character
                let n = csi.param_or_one(0) as usize;
                self.term.delete_chars(n);
            }
            b'L' => {
                // IL - Insert Line
                let n = csi.param_or_one(0) as usize;
                self.term.insert_lines(n);
            }
            b'M' => {
                // DL - Delete Line
                let n = csi.param_or_one(0) as usize;
                self.term.delete_lines(n);
            }

            // Scroll
            b'S' => {
                // SU - Scroll Up
                let n = csi.param_or_one(0) as usize;
                self.term.scroll_up(n);
            }
            b'T' => {
                // SD - Scroll Down
                let n = csi.param_or_one(0) as usize;
                self.term.scroll_down(n);
            }

            // Scroll region
            b'r' => {
                // DECSTBM - Set Top and Bottom Margins
                let top = csi.param_or_one(0) as usize;
                let bottom = csi.param(1, self.term.rows() as u16) as usize;
                self.term.set_scroll_region(top.saturating_sub(1), bottom.saturating_sub(1));
            }

            // SGR - Select Graphic Rendition
            b'm' => {
                self.sgr(&csi.params);
            }

            // Mode set/reset
            b'h' => {
                // SM - Set Mode
                for &param in &csi.params {
                    self.set_mode(param, true);
                }
            }
            b'l' => {
                // RM - Reset Mode
                for &param in &csi.params {
                    self.set_mode(param, false);
                }
            }

            // Cursor save/restore (ANSI)
            b's' => {
                // SCP - Save Cursor Position
                self.term.save_cursor();
            }
            b'u' => {
                // RCP - Restore Cursor Position
                self.term.restore_cursor();
            }

            // Tab clear
            b'g' => {
                // TBC - Tab Clear
                match csi.param(0, 0) {
                    0 => self.term.clear_tab_stop(),
                    3 => self.term.clear_all_tab_stops(),
                    _ => {}
                }
            }

            // Device status
            b'n' => {
                // DSR - Device Status Report
                // We don't respond to these in the performer
                trace!("DSR request: {:?}", csi.params);
            }

            // Cursor style (DECSCUSR)
            b'q' if csi.intermediates == [b' '] => {
                self.set_cursor_style(csi.param(0, 0));
            }

            _ => {
                debug!(
                    "Unhandled CSI: params={:?} intermediates={:?} final={}",
                    csi.params,
                    csi.intermediates,
                    csi.final_byte as char
                );
            }
        }
    }

    /// Handle private CSI sequences (CSI ?)
    fn csi_private(&mut self, csi: &CsiAction) {
        let set = csi.final_byte == b'h';

        for &param in &csi.params {
            match param {
                1 => {
                    // DECCKM - Application Cursor Keys
                    self.term.set_app_cursor(set);
                }
                6 => {
                    // DECOM - Origin Mode
                    self.term.set_origin_mode(set);
                }
                7 => {
                    // DECAWM - Auto-wrap Mode
                    self.term.set_auto_wrap(set);
                }
                12 => {
                    // Cursor blinking (att610)
                    self.term.set_cursor_blinking(set);
                }
                25 => {
                    // DECTCEM - Text Cursor Enable Mode
                    self.term.set_cursor_visible(set);
                }
                47 => {
                    // Alternate screen buffer (old)
                    if set {
                        self.term.enter_alt_screen();
                    } else {
                        self.term.leave_alt_screen();
                    }
                }
                1000 => {
                    // X10 mouse reporting
                    self.term.set_mouse_mode(if set { MouseMode::X10 } else { MouseMode::None });
                }
                1002 => {
                    // Button event mouse tracking
                    self.term.set_mouse_mode(if set { MouseMode::ButtonEvent } else { MouseMode::None });
                }
                1003 => {
                    // Any event mouse tracking
                    self.term.set_mouse_mode(if set { MouseMode::AnyEvent } else { MouseMode::None });
                }
                1004 => {
                    // Focus reporting
                    self.term.set_focus_reporting(set);
                }
                1005 => {
                    // UTF-8 mouse encoding
                    if set {
                        self.term.set_mouse_encoding(MouseEncoding::Utf8);
                    }
                }
                1006 => {
                    // SGR mouse encoding
                    if set {
                        self.term.set_mouse_encoding(MouseEncoding::Sgr);
                    } else {
                        self.term.set_mouse_encoding(MouseEncoding::X10);
                    }
                }
                1015 => {
                    // URXVT mouse encoding
                    if set {
                        self.term.set_mouse_encoding(MouseEncoding::Urxvt);
                    }
                }
                1047 => {
                    // Alternate screen buffer
                    if set {
                        self.term.enter_alt_screen();
                    } else {
                        self.term.leave_alt_screen();
                    }
                }
                1048 => {
                    // Save/restore cursor
                    if set {
                        self.term.save_cursor();
                    } else {
                        self.term.restore_cursor();
                    }
                }
                1049 => {
                    // Alternate screen buffer with cursor save/restore
                    if set {
                        self.term.save_cursor();
                        self.term.enter_alt_screen();
                        self.term.erase_screen();
                    } else {
                        self.term.leave_alt_screen();
                        self.term.restore_cursor();
                    }
                }
                2004 => {
                    // Bracketed paste mode
                    self.term.set_bracketed_paste(set);
                }
                _ => {
                    debug!("Unhandled private mode: {} (set={})", param, set);
                }
            }
        }
    }

    /// Handle CSI sequences with intermediate bytes
    fn csi_intermediate(&mut self, csi: &CsiAction) {
        match (csi.intermediates.as_slice(), csi.final_byte) {
            ([b' '], b'q') => {
                // DECSCUSR - Set Cursor Style
                self.set_cursor_style(csi.param(0, 0));
            }
            ([b'!'], b'p') => {
                // DECSTR - Soft Terminal Reset
                self.term.reset();
            }
            _ => {
                debug!(
                    "Unhandled CSI with intermediates: {:?} {:?} {}",
                    csi.intermediates,
                    csi.params,
                    csi.final_byte as char
                );
            }
        }
    }

    /// Set cursor style (DECSCUSR)
    fn set_cursor_style(&mut self, style: u16) {
        match style {
            0 | 1 => {
                self.term.set_cursor_style(CursorStyle::Block);
                self.term.set_cursor_blinking(true);
            }
            2 => {
                self.term.set_cursor_style(CursorStyle::Block);
                self.term.set_cursor_blinking(false);
            }
            3 => {
                self.term.set_cursor_style(CursorStyle::Underline);
                self.term.set_cursor_blinking(true);
            }
            4 => {
                self.term.set_cursor_style(CursorStyle::Underline);
                self.term.set_cursor_blinking(false);
            }
            5 => {
                self.term.set_cursor_style(CursorStyle::Bar);
                self.term.set_cursor_blinking(true);
            }
            6 => {
                self.term.set_cursor_style(CursorStyle::Bar);
                self.term.set_cursor_blinking(false);
            }
            _ => {}
        }
    }

    /// Set ANSI mode
    fn set_mode(&mut self, mode: u16, set: bool) {
        match mode {
            4 => {
                // IRM - Insert/Replace Mode
                self.term.set_insert_mode(set);
            }
            20 => {
                // LNM - Linefeed/New Line Mode
                self.term.set_linefeed_mode(set);
            }
            _ => {
                debug!("Unhandled ANSI mode: {} (set={})", mode, set);
            }
        }
    }

    /// Handle SGR (Select Graphic Rendition)
    fn sgr(&mut self, params: &[u16]) {
        if params.is_empty() {
            self.term.reset_attrs();
            return;
        }

        let mut i = 0;
        while i < params.len() {
            let param = params[i];
            match param {
                0 => self.term.reset_attrs(),
                1 => self.term.set_bold(true),
                2 => self.term.set_faint(true),
                3 => self.term.set_italic(true),
                4 => self.term.set_underline(true),
                5 | 6 => self.term.set_blink(true),
                7 => self.term.set_inverse(true),
                8 => self.term.set_hidden(true),
                9 => self.term.set_strikethrough(true),
                21 => self.term.set_bold(false), // Double underline or bold off
                22 => {
                    self.term.set_bold(false);
                    self.term.set_faint(false);
                }
                23 => self.term.set_italic(false),
                24 => self.term.set_underline(false),
                25 => self.term.set_blink(false),
                27 => self.term.set_inverse(false),
                28 => self.term.set_hidden(false),
                29 => self.term.set_strikethrough(false),

                // Foreground colors (30-37)
                30..=37 => {
                    if let Some(color) = NamedColor::from_sgr_normal((param - 30) as u8) {
                        self.term.set_fg(Color::Named(color));
                    }
                }
                38 => {
                    // Extended foreground color
                    if let Some((color, consumed)) = self.parse_extended_color(&params[i..]) {
                        self.term.set_fg(color);
                        i += consumed - 1;
                    }
                }
                39 => self.term.set_fg(Color::Default),

                // Background colors (40-47)
                40..=47 => {
                    if let Some(color) = NamedColor::from_sgr_normal((param - 40) as u8) {
                        self.term.set_bg(Color::Named(color));
                    }
                }
                48 => {
                    // Extended background color
                    if let Some((color, consumed)) = self.parse_extended_color(&params[i..]) {
                        self.term.set_bg(color);
                        i += consumed - 1;
                    }
                }
                49 => self.term.set_bg(Color::Default),

                // Bright foreground colors (90-97)
                90..=97 => {
                    if let Some(color) = NamedColor::from_sgr_bright((param - 90) as u8) {
                        self.term.set_fg(Color::Named(color));
                    }
                }

                // Bright background colors (100-107)
                100..=107 => {
                    if let Some(color) = NamedColor::from_sgr_bright((param - 100) as u8) {
                        self.term.set_bg(Color::Named(color));
                    }
                }

                _ => {
                    trace!("Unhandled SGR parameter: {}", param);
                }
            }
            i += 1;
        }
    }

    /// Parse extended color (38;5;N or 38;2;R;G;B)
    fn parse_extended_color(&self, params: &[u16]) -> Option<(Color, usize)> {
        if params.len() < 2 {
            return None;
        }

        match params[1] {
            2 => {
                // True color: 38;2;R;G;B
                if params.len() >= 5 {
                    let r = params[2] as u8;
                    let g = params[3] as u8;
                    let b = params[4] as u8;
                    Some((Color::Rgb(Rgb::new(r, g, b)), 5))
                } else {
                    None
                }
            }
            5 => {
                // 256 color: 38;5;N
                if params.len() >= 3 {
                    let index = params[2] as u8;
                    Some((Color::Indexed(index), 3))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Dispatch an ESC sequence
    fn esc_dispatch(&mut self, esc: EscAction) {
        match (esc.intermediates.as_slice(), esc.final_byte) {
            // Save/restore cursor (DEC)
            ([], b'7') => self.term.save_cursor(),
            ([], b'8') => self.term.restore_cursor(),

            // Index operations
            ([], b'D') => self.term.index(),
            ([], b'E') => self.term.next_line(),
            ([], b'M') => self.term.reverse_index(),

            // Tab set
            ([], b'H') => self.term.set_tab_stop(),

            // Reset
            ([], b'c') => self.term.reset(),

            // Keypad modes
            ([], b'=') => self.term.set_app_keypad(true),
            ([], b'>') => self.term.set_app_keypad(false),

            // Charset designation
            ([b'('], final_byte) => {
                self.term.set_charset(0, self.parse_charset(final_byte));
            }
            ([b')'], final_byte) => {
                self.term.set_charset(1, self.parse_charset(final_byte));
            }
            ([b'*'], final_byte) => {
                self.term.set_charset(2, self.parse_charset(final_byte));
            }
            ([b'+'], final_byte) => {
                self.term.set_charset(3, self.parse_charset(final_byte));
            }

            // Single shift
            ([], b'N') => {
                // SS2 - Single Shift G2
                trace!("SS2 (not fully implemented)");
            }
            ([], b'O') => {
                // SS3 - Single Shift G3
                trace!("SS3 (not fully implemented)");
            }

            _ => {
                debug!(
                    "Unhandled ESC: intermediates={:?} final={}",
                    esc.intermediates, esc.final_byte as char
                );
            }
        }
    }

    /// Parse charset designation
    fn parse_charset(&self, c: u8) -> Charset {
        match c {
            b'0' => Charset::DecSpecialGraphics,
            b'A' => Charset::Uk,
            b'B' | _ => Charset::Ascii,
        }
    }

    /// Dispatch an OSC sequence
    fn osc_dispatch(&mut self, osc: OscAction) {
        match osc.command {
            0 => {
                // Set icon name and window title
                self.term.set_icon_name(osc.payload.clone());
                self.term.set_title(osc.payload);
            }
            1 => {
                // Set icon name
                self.term.set_icon_name(osc.payload);
            }
            2 => {
                // Set window title
                self.term.set_title(osc.payload);
            }
            4 => {
                // Change color palette
                trace!("OSC 4 (change color) not implemented");
            }
            7 => {
                // Set working directory
                trace!("OSC 7 (working directory): {}", osc.payload);
            }
            8 => {
                // Hyperlink
                self.handle_hyperlink(&osc.payload);
            }
            10 => {
                // Query/set foreground color
                trace!("OSC 10 (foreground color) not implemented");
            }
            11 => {
                // Query/set background color
                trace!("OSC 11 (background color) not implemented");
            }
            12 => {
                // Query/set cursor color
                trace!("OSC 12 (cursor color) not implemented");
            }
            52 => {
                // Clipboard access
                self.handle_clipboard(&osc.payload);
            }
            104 => {
                // Reset color
                trace!("OSC 104 (reset color) not implemented");
            }
            112 => {
                // Reset cursor color
                trace!("OSC 112 (reset cursor color) not implemented");
            }
            _ => {
                debug!("Unhandled OSC {}: {}", osc.command, osc.payload);
            }
        }
    }

    /// Handle OSC 8 hyperlink
    fn handle_hyperlink(&mut self, payload: &str) {
        // Format: params;uri
        // Empty uri ends the hyperlink
        let parts: Vec<&str> = payload.splitn(2, ';').collect();
        if parts.len() < 2 {
            self.term.clear_hyperlink();
            return;
        }

        let params = parts[0];
        let uri = parts[1];

        if uri.is_empty() {
            self.term.clear_hyperlink();
        } else {
            let id = self.term.register_hyperlink(uri.to_string(), params.to_string());
            self.term.set_hyperlink(id);
        }
    }

    /// Handle OSC 52 clipboard
    fn handle_clipboard(&mut self, payload: &str) {
        // Format: Pc;Pd where Pc is clipboard selection and Pd is base64 data
        // This is a security-sensitive operation
        warn!("OSC 52 clipboard access attempted (disabled by default)");
        // Implementation would decode base64 and set clipboard
        // But we don't implement this by default for security
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print() {
        let mut term = Term::new(24, 80);
        let mut performer = Performer::new(&mut term);
        performer.perform(Action::Print('A'));
        assert_eq!(term.snapshot().row_text(0), "A");
    }

    #[test]
    fn test_cursor_movement() {
        let mut term = Term::new(24, 80);
        let mut performer = Performer::new(&mut term);

        // Move cursor to (5, 10)
        performer.perform(Action::CsiDispatch(CsiAction {
            params: vec![6, 11],
            intermediates: vec![],
            final_byte: b'H',
            private: false,
        }));

        assert_eq!(term.screen().cursor.row, 5);
        assert_eq!(term.screen().cursor.col, 10);
    }

    #[test]
    fn test_sgr_colors() {
        let mut term = Term::new(24, 80);
        let mut performer = Performer::new(&mut term);

        // Set red foreground
        performer.perform(Action::CsiDispatch(CsiAction {
            params: vec![31],
            intermediates: vec![],
            final_byte: b'm',
            private: false,
        }));

        // Write a character
        performer.perform(Action::Print('X'));

        let cell = term.screen().grid.cell(0, 0);
        assert!(matches!(cell.fg, Color::Named(NamedColor::Red)));
    }

    #[test]
    fn test_alt_screen() {
        let mut term = Term::new(24, 80);

        // Write on primary screen
        {
            let mut performer = Performer::new(&mut term);
            performer.perform(Action::Print('A'));
        }

        // Enter alt screen
        {
            let mut performer = Performer::new(&mut term);
            performer.perform(Action::CsiDispatch(CsiAction {
                params: vec![1049],
                intermediates: vec![],
                final_byte: b'h',
                private: true,
            }));
        }

        // Write on alt screen
        {
            let mut performer = Performer::new(&mut term);
            performer.perform(Action::Print('B'));
        }
        assert_eq!(term.snapshot().row_text(0), "B");

        // Leave alt screen
        {
            let mut performer = Performer::new(&mut term);
            performer.perform(Action::CsiDispatch(CsiAction {
                params: vec![1049],
                intermediates: vec![],
                final_byte: b'l',
                private: true,
            }));
        }

        // Should see primary screen content
        assert_eq!(term.snapshot().row_text(0), "A");
    }
}
