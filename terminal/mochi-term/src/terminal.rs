//! Terminal state management
//!
//! Integrates the parser and screen model to handle terminal emulation.

use terminal_core::{
    CellAttributes, Color, CursorStyle, Dimensions, Screen, Snapshot, UnderlineStyle,
};
use terminal_parser::{Action, CsiAction, EscAction, OscAction, Parser};

/// Terminal emulator state
pub struct Terminal {
    /// Screen state
    screen: Screen,
    /// Parser
    parser: Parser,
    /// Window title
    title: String,
    /// Pending title change
    title_changed: bool,
    /// Bell triggered
    bell: bool,
    /// Track if synchronized output mode has been enabled before
    /// Used to clear screen on first enable for TUI apps like Claude Code
    sync_output_first_enable: bool,
    /// Pending responses to send back to the PTY
    /// Used for DSR (Device Status Report), DA1 (Primary Device Attributes), etc.
    pending_responses: Vec<Vec<u8>>,
    /// Last printed character for REP (CSI b) support
    last_printed_char: Option<char>,
    /// Pending focus events to send (true = focus in, false = focus out)
    /// Used by notify_focus()/take_pending_focus_events() which are called by the GUI layer
    #[allow(dead_code)]
    pending_focus_events: Vec<bool>,
}

impl Terminal {
    /// Create a new terminal with the given dimensions
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            screen: Screen::new(Dimensions::new(cols, rows)),
            parser: Parser::new(),
            title: String::new(),
            title_changed: false,
            bell: false,
            sync_output_first_enable: false,
            pending_responses: Vec::new(),
            last_printed_char: None,
            pending_focus_events: Vec::new(),
        }
    }

    /// Get screen reference
    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    /// Get screen mutably
    #[allow(dead_code)]
    pub fn screen_mut(&mut self) -> &mut Screen {
        &mut self.screen
    }

    /// Get window title
    pub fn title(&self) -> &str {
        if self.title.is_empty() {
            self.screen.title()
        } else {
            &self.title
        }
    }

    /// Check and clear title changed flag
    pub fn take_title_changed(&mut self) -> bool {
        let changed = self.title_changed;
        self.title_changed = false;
        changed
    }

    /// Check and clear bell flag
    pub fn take_bell(&mut self) -> bool {
        let bell = self.bell;
        self.bell = false;
        bell
    }

    /// Process input bytes from the PTY
    pub fn process(&mut self, data: &[u8]) {
        // Collect actions first to avoid borrow checker issues
        let mut actions = Vec::new();
        self.parser.parse(data, |action| {
            actions.push(action);
        });

        // Then handle each action
        for action in actions {
            self.handle_action(action);
        }
    }

    /// Handle a parsed action
    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Print(c) => {
                self.screen.print(c);
                self.last_printed_char = Some(c);
            }
            Action::Control(byte) => {
                self.handle_control(byte);
            }
            Action::Esc(esc) => {
                self.handle_esc(esc);
            }
            Action::Csi(csi) => {
                self.handle_csi(csi);
            }
            Action::Osc(osc) => {
                self.handle_osc(osc);
            }
            Action::Dcs {
                params: _,
                intermediates,
                final_byte,
                data,
            } => {
                self.handle_dcs(&intermediates, final_byte, &data);
            }
            Action::Apc(_) | Action::Pm(_) | Action::Sos(_) => {
                // These are consumed but ignored
            }
            Action::Invalid(data) => {
                log::debug!("Invalid sequence: {:?}", data);
            }
        }
    }

    /// Handle C0 control characters
    fn handle_control(&mut self, byte: u8) {
        match byte {
            0x07 => {
                // BEL
                self.bell = true;
            }
            0x08 => {
                // BS
                self.screen.backspace();
            }
            0x09 => {
                // HT
                self.screen.tab();
            }
            0x0A..=0x0C => {
                // LF, VT, FF
                self.screen.linefeed();
            }
            0x0D => {
                // CR
                self.screen.carriage_return();
            }
            0x0E => {
                // SO - Shift Out (select G1)
                self.screen.shift_out();
            }
            0x0F => {
                // SI - Shift In (select G0)
                self.screen.shift_in();
            }
            _ => {}
        }
    }

    /// Handle ESC sequences
    fn handle_esc(&mut self, esc: EscAction) {
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
                // Application keypad mode - affects key encoding
                log::debug!("Application keypad mode enabled");
            }
            EscAction::NormalKeypad => {
                // Normal keypad mode
                log::debug!("Normal keypad mode enabled");
            }
            EscAction::DesignateG0(c) => {
                // Character set designation for G0
                self.screen.designate_charset(0, c);
            }
            EscAction::DesignateG1(c) => {
                // Character set designation for G1
                self.screen.designate_charset(1, c);
            }
            EscAction::DesignateG2(c) => {
                // Character set designation for G2
                self.screen.designate_charset(2, c);
            }
            EscAction::DesignateG3(c) => {
                // Character set designation for G3
                self.screen.designate_charset(3, c);
            }
            EscAction::DecAlignmentTest => {
                // Fill screen with 'E' characters
                let rows = self.screen.rows();
                let cols = self.screen.cols();
                for row in 0..rows {
                    self.screen.move_cursor_to(row + 1, 1);
                    for _ in 0..cols {
                        self.screen.print('E');
                    }
                }
                self.screen.move_cursor_to(1, 1);
            }
            EscAction::Unknown(data) => {
                log::debug!("Unknown ESC sequence: {:?}", data);
            }
        }
    }

    /// Handle CSI sequences
    fn handle_csi(&mut self, csi: CsiAction) {
        // Handle prefixed sequences (?, >, <, =)
        if csi.prefix != 0 {
            self.handle_csi_private(&csi);
            return;
        }

        // Handle sequences with intermediates
        if !csi.intermediates.is_empty() {
            self.handle_csi_intermediate(&csi);
            return;
        }

        match csi.final_byte {
            b'@' => {
                // ICH - Insert Character
                let n = csi.param(0, 1) as usize;
                self.screen.insert_chars(n);
            }
            b'A' => {
                // CUU - Cursor Up
                let n = csi.param(0, 1) as usize;
                self.screen.move_cursor_up(n);
            }
            b'B' => {
                // CUD - Cursor Down
                let n = csi.param(0, 1) as usize;
                self.screen.move_cursor_down(n);
            }
            b'C' => {
                // CUF - Cursor Forward
                let n = csi.param(0, 1) as usize;
                self.screen.move_cursor_right(n);
            }
            b'D' => {
                // CUB - Cursor Back
                let n = csi.param(0, 1) as usize;
                self.screen.move_cursor_left(n);
            }
            b'E' => {
                // CNL - Cursor Next Line
                let n = csi.param(0, 1) as usize;
                self.screen.move_cursor_down(n);
                self.screen.carriage_return();
            }
            b'F' => {
                // CPL - Cursor Previous Line
                let n = csi.param(0, 1) as usize;
                self.screen.move_cursor_up(n);
                self.screen.carriage_return();
            }
            b'G' => {
                // CHA - Cursor Horizontal Absolute
                let col = csi.param(0, 1) as usize;
                self.screen.set_cursor_col(col);
            }
            b'H' | b'f' => {
                // CUP/HVP - Cursor Position
                let row = csi.param(0, 1) as usize;
                let col = csi.param(1, 1) as usize;
                self.screen.move_cursor_to(row, col);
            }
            b'J' => {
                // ED - Erase in Display
                let mode = csi.param(0, 0);
                self.screen.erase_display(mode);
            }
            b'K' => {
                // EL - Erase in Line
                let mode = csi.param(0, 0);
                self.screen.erase_line(mode);
            }
            b'L' => {
                // IL - Insert Line
                let n = csi.param(0, 1) as usize;
                self.screen.insert_lines(n);
            }
            b'M' => {
                // DL - Delete Line
                let n = csi.param(0, 1) as usize;
                self.screen.delete_lines(n);
            }
            b'P' => {
                // DCH - Delete Character
                let n = csi.param(0, 1) as usize;
                self.screen.delete_chars(n);
            }
            b'S' => {
                // SU - Scroll Up
                let n = csi.param(0, 1) as usize;
                self.screen.scroll_up(n);
            }
            b'T' => {
                // SD - Scroll Down
                let n = csi.param(0, 1) as usize;
                self.screen.scroll_down(n);
            }
            b'X' => {
                // ECH - Erase Character
                let n = csi.param(0, 1) as usize;
                self.screen.erase_chars(n);
            }
            b'Z' => {
                // CBT - Cursor Backward Tabulation
                let n = csi.param(0, 1) as usize;
                for _ in 0..n {
                    self.screen.backtab();
                }
            }
            b'b' => {
                // REP - Repeat preceding character
                if let Some(c) = self.last_printed_char {
                    let n = csi.param(0, 1) as usize;
                    for _ in 0..n {
                        self.screen.print(c);
                    }
                }
            }
            b'd' => {
                // VPA - Vertical Position Absolute
                let row = csi.param(0, 1) as usize;
                self.screen.set_cursor_row(row);
            }
            b'g' => {
                // TBC - Tab Clear
                let mode = csi.param(0, 0);
                self.screen.clear_tab_stop(mode);
            }
            b'h' => {
                // SM - Set Mode
                for param in csi.params.iter() {
                    self.screen.modes_mut().set_mode(param, true);
                }
            }
            b'l' => {
                // RM - Reset Mode
                for param in csi.params.iter() {
                    self.screen.modes_mut().set_mode(param, false);
                }
            }
            b'm' => {
                // SGR - Select Graphic Rendition
                self.handle_sgr(&csi);
            }
            b'n' => {
                // DSR - Device Status Report
                let mode = csi.param(0, 0);
                match mode {
                    5 => {
                        // Status report - respond with "OK"
                        // Response: CSI 0 n
                        self.queue_response(b"\x1b[0n".to_vec());
                        log::debug!("DSR mode 5: status report, responding OK");
                    }
                    6 => {
                        // Cursor position report
                        // Response: CSI row ; col R (1-indexed)
                        let row = self.screen.cursor().row + 1;
                        let col = self.screen.cursor().col + 1;
                        let response = format!("\x1b[{};{}R", row, col);
                        self.queue_response(response.into_bytes());
                        log::debug!(
                            "DSR mode 6: cursor position report, responding row={} col={}",
                            row,
                            col
                        );
                    }
                    _ => {
                        log::debug!("DSR request with unknown mode: {}", mode);
                    }
                }
            }
            b'r' => {
                // DECSTBM - Set Top and Bottom Margins
                let top = csi.param(0, 1) as usize;
                let bottom = csi.param(1, self.screen.rows() as u16) as usize;
                self.screen.set_scroll_region(top, bottom);
            }
            b's' => {
                // Save cursor (ANSI.SYS)
                self.screen.save_cursor();
            }
            b'u' => {
                // Restore cursor (ANSI.SYS)
                self.screen.restore_cursor();
            }
            _ => {
                log::debug!(
                    "Unknown CSI sequence: {:?} {}",
                    csi.params,
                    csi.final_byte as char
                );
            }
        }
    }

    /// Handle CSI sequences with private marker (?) or other prefixes (>, <, =)
    fn handle_csi_private(&mut self, csi: &CsiAction) {
        // Handle non-? prefixes (>, <, =)
        if csi.prefix != b'?' {
            self.handle_csi_prefixed(csi);
            return;
        }

        // Check for DECRQM: CSI ? Ps $ p
        if csi.final_byte == b'p' && csi.intermediates == [b'$'] {
            self.handle_decrqm(csi);
            return;
        }

        match csi.final_byte {
            b'h' => {
                // DECSET - DEC Private Mode Set
                for param in csi.params.iter() {
                    self.set_dec_mode(param, true);
                }
            }
            b'l' => {
                // DECRST - DEC Private Mode Reset
                for param in csi.params.iter() {
                    self.set_dec_mode(param, false);
                }
            }
            b'c' => {
                // DA1 - Primary Device Attributes
                // Respond as VT220 with features: 132 columns, printer port, selective erase,
                // user-defined keys, national replacement charsets, technical chars, ANSI color
                // This richer response helps tmux detect terminal capabilities
                self.queue_response(b"\x1b[?62;1;2;4;6;7;8;9;15;22c".to_vec());
                log::debug!("DA1 request: responding as VT220 with extended capabilities");
            }
            b'n' => {
                // DECDSR - DEC-specific Device Status Report
                let mode = csi.param(0, 0);
                match mode {
                    6 => {
                        // DECXCPR - Extended Cursor Position Report
                        let row = self.screen.cursor().row + 1;
                        let col = self.screen.cursor().col + 1;
                        let response = format!("\x1b[?{};{}R", row, col);
                        self.queue_response(response.into_bytes());
                        log::debug!(
                            "DECXCPR: extended cursor position report row={} col={}",
                            row,
                            col
                        );
                    }
                    _ => {
                        log::debug!("Unknown DECDSR mode: {}", mode);
                    }
                }
            }
            b's' => {
                // XTSAVE - Save private mode values
                for param in csi.params.iter() {
                    self.screen.modes_mut().save_dec_mode(param);
                }
                log::debug!("XTSAVE: saved modes {:?}", csi.params);
            }
            b'r' => {
                // XTRESTORE - Restore private mode values
                // Route through Terminal::set_dec_mode to apply side effects
                // (cursor visibility, alternate screen, origin mode, etc.)
                let params: Vec<u16> = csi.params.iter().collect();
                for param in params {
                    if let Some(value) = self.screen.modes().get_saved_dec_mode(param) {
                        self.set_dec_mode(param, value);
                    }
                }
                log::debug!("XTRESTORE: restored modes {:?}", csi.params);
            }
            b't' => {
                // Window manipulation (XTWINOPS)
                self.handle_window_ops(csi);
            }
            _ => {
                log::debug!(
                    "Unknown private CSI: ?{:?}{}",
                    csi.params,
                    csi.final_byte as char
                );
            }
        }
    }

    /// Handle CSI sequences with non-? prefixes (>, <, =)
    fn handle_csi_prefixed(&mut self, csi: &CsiAction) {
        match (csi.prefix, csi.final_byte) {
            (b'>', b'c') => {
                // DA2 - Secondary Device Attributes
                // Response: CSI > Pp ; Pv ; Pc c
                // Pp=0 (VT100), Pv=136 (firmware version), Pc=0 (ROM cartridge)
                // tmux uses this to identify the terminal and enable features
                self.queue_response(b"\x1b[>0;136;0c".to_vec());
                log::debug!("DA2 request: responding with terminal identification");
            }
            (b'>', b'q') => {
                // XTVERSION - Report terminal version
                // Response: DCS > | terminal-name(version) ST
                // tmux uses this to identify the terminal
                self.queue_response(b"\x1bP>|mochi(0.1.0)\x1b\\".to_vec());
                log::debug!("XTVERSION request: responding with mochi version");
            }
            (b'>', b'm') => {
                // xterm modifyOtherKeys - reset
                // tmux sends CSI > 4 ; 0 m to reset modifyOtherKeys
                log::debug!("modifyOtherKeys reset (ignored)");
            }
            (b'>', b'n') => {
                // Disable key modifier options
                log::debug!("Disable key modifier options (ignored)");
            }
            (b'>', b'u') => {
                // Kitty keyboard protocol - query current flags
                // Response: CSI ? flags u
                // We report 0 (no progressive enhancement active)
                self.queue_response(b"\x1b[?0u".to_vec());
                log::debug!("Kitty keyboard protocol query: responding with flags=0");
            }
            (b'<', b'u') => {
                // Kitty keyboard protocol - pop from stack
                // We don't track the stack, just acknowledge
                log::debug!("Kitty keyboard protocol pop (ignored)");
            }
            (b'=', b'c') => {
                // DA3 - Tertiary Device Attributes
                // Response: DCS ! | device-id ST
                self.queue_response(b"\x1bP!|00000000\x1b\\".to_vec());
                log::debug!("DA3 request: responding with device ID");
            }
            (b'=', b'u') => {
                // Kitty keyboard protocol - push flags to stack
                log::debug!("Kitty keyboard protocol push (ignored)");
            }
            _ => {
                log::debug!(
                    "Unknown prefixed CSI: {:?} {:?} {}",
                    csi.prefix as char,
                    csi.params,
                    csi.final_byte as char
                );
            }
        }
    }

    /// Handle DECRQM - DEC Private Mode Request
    /// tmux uses this to probe which modes the terminal supports
    /// Response: CSI ? Ps ; Pm $ y
    /// Pm: 0=not recognized, 1=set, 2=reset, 3=permanently set, 4=permanently reset
    fn handle_decrqm(&mut self, csi: &CsiAction) {
        let mode = csi.param(0, 0);
        let status = match mode {
            // All modes we actively track - use get_dec_mode for uniform access
            1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 25 | 1000 | 1002 | 1003 | 1004 | 1005 | 1006
            | 1015 | 1016 | 2004 | 2026 => {
                if self.screen.modes().get_dec_mode(mode) {
                    1
                } else {
                    2
                }
            }
            // Alternate screen variants
            47 | 1047 | 1049 => {
                if self.screen.modes().alternate_screen {
                    1
                } else {
                    2
                }
            }
            // Modes we don't support - report as not recognized
            _ => 0,
        };
        let response = format!("\x1b[?{};{}$y", mode, status);
        self.queue_response(response.into_bytes());
        log::debug!("DECRQM: mode {} status {}", mode, status);
    }

    /// Handle CSI sequences with intermediate bytes
    fn handle_csi_intermediate(&mut self, csi: &CsiAction) {
        match (csi.intermediates.as_slice(), csi.final_byte) {
            ([b' '], b'q') => {
                // DECSCUSR - Set Cursor Style
                let style = csi.param(0, 1);
                let cursor = self.screen.cursor_mut();
                match style {
                    0 | 1 => {
                        cursor.style = CursorStyle::Block;
                        cursor.blinking = true;
                    }
                    2 => {
                        cursor.style = CursorStyle::Block;
                        cursor.blinking = false;
                    }
                    3 => {
                        cursor.style = CursorStyle::Underline;
                        cursor.blinking = true;
                    }
                    4 => {
                        cursor.style = CursorStyle::Underline;
                        cursor.blinking = false;
                    }
                    5 => {
                        cursor.style = CursorStyle::Bar;
                        cursor.blinking = true;
                    }
                    6 => {
                        cursor.style = CursorStyle::Bar;
                        cursor.blinking = false;
                    }
                    _ => {}
                }
            }
            ([b'!'], b'p') => {
                // DECSTR - Soft Terminal Reset
                self.screen.modes_mut().reset();
                self.screen.cursor_mut().attrs.reset();
                self.screen.cursor_mut().style = CursorStyle::Block;
                self.screen.cursor_mut().blinking = true;
                self.screen.cursor_mut().visible = true;
                self.screen.clear_scroll_region();
                log::debug!("DECSTR: soft terminal reset");
            }
            ([b'"'], b'p') => {
                // DECSCL - Set Conformance Level
                // tmux may query this; just acknowledge
                log::debug!("DECSCL: set conformance level (ignored)");
            }
            _ => {
                log::debug!(
                    "Unknown CSI with intermediates: {:?} {:?} {}",
                    csi.intermediates,
                    csi.params,
                    csi.final_byte as char
                );
            }
        }
    }

    /// Handle CSI t / CSI ? t - Window manipulation operations
    fn handle_window_ops(&mut self, csi: &CsiAction) {
        let op = csi.param(0, 0);
        match op {
            14 => {
                // Report window size in pixels
                // Response: CSI 4 ; height ; width t
                // Use 16px per cell as a reasonable default
                let height = self.screen.rows() * 16;
                let width = self.screen.cols() * 16;
                let response = format!("\x1b[4;{};{}t", height, width);
                self.queue_response(response.into_bytes());
                log::debug!("Window ops 14: report pixel size {}x{}", width, height);
            }
            16 => {
                // Report cell size in pixels
                // Response: CSI 6 ; height ; width t
                let response = format!("\x1b[6;{};{}t", 16, 8);
                self.queue_response(response.into_bytes());
                log::debug!("Window ops 16: report cell size");
            }
            18 => {
                // Report terminal size in characters
                // Response: CSI 8 ; rows ; cols t
                let rows = self.screen.rows();
                let cols = self.screen.cols();
                let response = format!("\x1b[8;{};{}t", rows, cols);
                self.queue_response(response.into_bytes());
                log::debug!("Window ops 18: report text area size {}x{}", cols, rows);
            }
            22 => {
                // Push title to stack
                log::debug!("Window ops 22: push title (acknowledged)");
            }
            23 => {
                // Pop title from stack
                log::debug!("Window ops 23: pop title (acknowledged)");
            }
            _ => {
                log::debug!("Window ops {}: ignored", op);
            }
        }
    }

    /// Set DEC private mode
    fn set_dec_mode(&mut self, mode: u16, value: bool) {
        match mode {
            1 => {
                // DECCKM - Cursor Keys Mode
                self.screen.modes_mut().cursor_keys_application = value;
            }
            6 => {
                // DECOM - Origin Mode
                self.screen.modes_mut().origin_mode = value;
                self.screen.cursor_mut().origin_mode = value;
                if value {
                    let (top, _) = self.screen.scroll_region();
                    self.screen.move_cursor_to(top + 1, 1);
                } else {
                    self.screen.move_cursor_to(1, 1);
                }
            }
            7 => {
                // DECAWM - Auto-wrap Mode
                self.screen.modes_mut().auto_wrap = value;
            }
            25 => {
                // DECTCEM - Text Cursor Enable Mode
                self.screen.modes_mut().cursor_visible = value;
                self.screen.cursor_mut().visible = value;
            }
            1000 => {
                // Mouse tracking: VT200
                self.screen.modes_mut().mouse_vt200 = value;
            }
            1002 => {
                // Mouse tracking: Button event
                self.screen.modes_mut().mouse_button_event = value;
            }
            1003 => {
                // Mouse tracking: Any event
                self.screen.modes_mut().mouse_any_event = value;
            }
            1004 => {
                // Focus events
                self.screen.modes_mut().focus_events = value;
            }
            1006 => {
                // SGR mouse mode
                self.screen.modes_mut().mouse_sgr = value;
            }
            47 => {
                // Alternate screen buffer (without clearing)
                if value {
                    self.screen.enter_alternate_screen();
                } else {
                    self.screen.exit_alternate_screen();
                }
            }
            1047 => {
                // Alternate screen buffer (with clearing)
                if value {
                    self.screen.enter_alternate_screen();
                } else {
                    self.screen.exit_alternate_screen();
                }
            }
            1048 => {
                // Save/restore cursor (used with alternate screen)
                if value {
                    self.screen.save_cursor();
                } else {
                    self.screen.restore_cursor();
                }
            }
            1049 => {
                // Alternate screen buffer (combines 1047 + 1048)
                if value {
                    self.screen.save_cursor();
                    self.screen.enter_alternate_screen();
                } else {
                    self.screen.exit_alternate_screen();
                    self.screen.restore_cursor();
                }
            }
            2004 => {
                // Bracketed paste mode
                self.screen.modes_mut().bracketed_paste = value;
            }
            2026 => {
                // Synchronized output mode (used by TUI apps like Claude Code)
                // When enabled, the terminal should buffer output until disabled
                // This helps prevent flickering during rapid screen updates
                //
                // IMPORTANT: On the FIRST enable of this mode, we clear the screen.
                // This is because TUI apps like Claude Code use differential rendering
                // and expect a clean canvas. Without this, old terminal content would
                // show through in areas the TUI app doesn't explicitly overwrite.
                // The user reported "resizing fixes it" because resize triggers a full
                // redraw - this fix ensures the first render also gets a clean canvas.
                if value && !self.sync_output_first_enable {
                    self.sync_output_first_enable = true;
                    // Clear the entire screen to give TUI apps a clean canvas
                    self.screen.erase_display(2); // 2 = clear entire screen
                    self.screen.move_cursor_to(1, 1); // Move cursor to home
                    log::debug!(
                        "Synchronized output mode first enable: clearing screen for TUI app"
                    );
                }
                self.screen.modes_mut().synchronized_output = value;
                log::debug!("Synchronized output mode: {}", value);
            }
            _ => {
                self.screen.modes_mut().set_dec_mode(mode, value);
            }
        }
    }

    /// Handle SGR (Select Graphic Rendition)
    ///
    /// Supports both semicolon-separated (38;2;R;G;B) and colon-separated (38:2:R:G:B)
    /// extended color sequences, as well as mixed sequences like 4:3;38;5;196.
    /// Uses a unified index-based loop that checks subparams per-parameter.
    fn handle_sgr(&mut self, csi: &CsiAction) {
        let attrs = &mut self.screen.cursor_mut().attrs;

        if csi.params.is_empty() {
            attrs.reset();
            return;
        }

        // Collect params and subparams into vectors for index-based iteration.
        // This allows us to peek ahead for semicolon-separated extended colors
        // (38;5;N, 38;2;R;G;B) while also handling colon-separated subparams
        // (4:3, 38:2:R:G:B) on a per-parameter basis.
        let items: Vec<(u16, Vec<u16>)> = csi
            .params
            .iter_with_subparams()
            .map(|(v, s)| (v, s.to_vec()))
            .collect();

        let mut i = 0;
        while i < items.len() {
            let (param, ref subparams) = items[i];

            // If this parameter has colon-separated subparameters, handle them
            if !subparams.is_empty() {
                match param {
                    4 => {
                        // SGR 4:Ps - underline style
                        match subparams[0] {
                            0 => {
                                attrs.underline = false;
                                attrs.underline_style = UnderlineStyle::None;
                            }
                            1 => {
                                attrs.underline = true;
                                attrs.underline_style = UnderlineStyle::Single;
                            }
                            2 => {
                                attrs.underline = true;
                                attrs.underline_style = UnderlineStyle::Double;
                            }
                            3 => {
                                attrs.underline = true;
                                attrs.underline_style = UnderlineStyle::Curly;
                            }
                            4 => {
                                attrs.underline = true;
                                attrs.underline_style = UnderlineStyle::Dotted;
                            }
                            5 => {
                                attrs.underline = true;
                                attrs.underline_style = UnderlineStyle::Dashed;
                            }
                            _ => {
                                attrs.underline = true;
                                attrs.underline_style = UnderlineStyle::Single;
                            }
                        }
                    }
                    38 => {
                        // Colon-separated foreground color: 38:2:R:G:B or 38:5:N
                        match subparams[0] {
                            2 => {
                                if subparams.len() >= 4 {
                                    let (r, g, b) = if subparams.len() >= 5 {
                                        (subparams[2] as u8, subparams[3] as u8, subparams[4] as u8)
                                    } else {
                                        (subparams[1] as u8, subparams[2] as u8, subparams[3] as u8)
                                    };
                                    attrs.fg = Color::Rgb { r, g, b };
                                }
                            }
                            5 => {
                                if subparams.len() >= 2 {
                                    attrs.fg = Color::Indexed(subparams[1] as u8);
                                }
                            }
                            _ => {}
                        }
                    }
                    48 => {
                        // Colon-separated background color
                        match subparams[0] {
                            2 => {
                                if subparams.len() >= 4 {
                                    let (r, g, b) = if subparams.len() >= 5 {
                                        (subparams[2] as u8, subparams[3] as u8, subparams[4] as u8)
                                    } else {
                                        (subparams[1] as u8, subparams[2] as u8, subparams[3] as u8)
                                    };
                                    attrs.bg = Color::Rgb { r, g, b };
                                }
                            }
                            5 => {
                                if subparams.len() >= 2 {
                                    attrs.bg = Color::Indexed(subparams[1] as u8);
                                }
                            }
                            _ => {}
                        }
                    }
                    58 => {
                        // Colon-separated underline color
                        match subparams[0] {
                            2 => {
                                if subparams.len() >= 4 {
                                    let (r, g, b) = if subparams.len() >= 5 {
                                        (subparams[2] as u8, subparams[3] as u8, subparams[4] as u8)
                                    } else {
                                        (subparams[1] as u8, subparams[2] as u8, subparams[3] as u8)
                                    };
                                    attrs.underline_color = Color::Rgb { r, g, b };
                                }
                            }
                            5 => {
                                if subparams.len() >= 2 {
                                    attrs.underline_color = Color::Indexed(subparams[1] as u8);
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {
                        // Standard SGR param that happens to be in a sequence with subparams
                        Self::apply_simple_sgr(attrs, param);
                    }
                }
                i += 1;
                continue;
            }

            // No subparams: handle as standard SGR with semicolon-separated
            // extended colors that consume subsequent parameters
            match param {
                0 => attrs.reset(),
                1 => attrs.bold = true,
                2 => attrs.faint = true,
                3 => attrs.italic = true,
                4 => {
                    attrs.underline = true;
                    attrs.underline_style = UnderlineStyle::Single;
                }
                5 => attrs.blink = true,
                7 => attrs.inverse = true,
                8 => attrs.hidden = true,
                9 => attrs.strikethrough = true,
                21 => {
                    attrs.underline = true;
                    attrs.underline_style = UnderlineStyle::Double;
                }
                22 => {
                    attrs.bold = false;
                    attrs.faint = false;
                }
                23 => attrs.italic = false,
                24 => {
                    attrs.underline = false;
                    attrs.underline_style = UnderlineStyle::None;
                }
                25 => attrs.blink = false,
                27 => attrs.inverse = false,
                28 => attrs.hidden = false,
                29 => attrs.strikethrough = false,
                30..=37 => {
                    attrs.fg = Color::Indexed((param - 30) as u8);
                }
                38 => {
                    // Extended foreground color (semicolon-separated)
                    if i + 1 < items.len() {
                        match items[i + 1].0 {
                            5 => {
                                if i + 2 < items.len() {
                                    attrs.fg = Color::Indexed(items[i + 2].0 as u8);
                                    i += 2;
                                }
                            }
                            2 => {
                                if i + 4 < items.len() {
                                    attrs.fg = Color::Rgb {
                                        r: items[i + 2].0 as u8,
                                        g: items[i + 3].0 as u8,
                                        b: items[i + 4].0 as u8,
                                    };
                                    i += 4;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                39 => attrs.fg = Color::Default,
                40..=47 => {
                    attrs.bg = Color::Indexed((param - 40) as u8);
                }
                48 => {
                    // Extended background color (semicolon-separated)
                    if i + 1 < items.len() {
                        match items[i + 1].0 {
                            5 => {
                                if i + 2 < items.len() {
                                    attrs.bg = Color::Indexed(items[i + 2].0 as u8);
                                    i += 2;
                                }
                            }
                            2 => {
                                if i + 4 < items.len() {
                                    attrs.bg = Color::Rgb {
                                        r: items[i + 2].0 as u8,
                                        g: items[i + 3].0 as u8,
                                        b: items[i + 4].0 as u8,
                                    };
                                    i += 4;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                49 => attrs.bg = Color::Default,
                53 => attrs.overline = true,
                55 => attrs.overline = false,
                58 => {
                    // Extended underline color (semicolon-separated)
                    if i + 1 < items.len() {
                        match items[i + 1].0 {
                            5 => {
                                if i + 2 < items.len() {
                                    attrs.underline_color = Color::Indexed(items[i + 2].0 as u8);
                                    i += 2;
                                }
                            }
                            2 => {
                                if i + 4 < items.len() {
                                    attrs.underline_color = Color::Rgb {
                                        r: items[i + 2].0 as u8,
                                        g: items[i + 3].0 as u8,
                                        b: items[i + 4].0 as u8,
                                    };
                                    i += 4;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                59 => attrs.underline_color = Color::Default,
                90..=97 => {
                    attrs.fg = Color::Indexed((param - 90 + 8) as u8);
                }
                100..=107 => {
                    attrs.bg = Color::Indexed((param - 100 + 8) as u8);
                }
                _ => {
                    log::debug!("Unknown SGR parameter: {}", param);
                }
            }
            i += 1;
        }
    }

    /// Apply a simple (non-extended-color) SGR parameter to cell attributes.
    /// Used by the colon-separated subparams path for params that don't have subparams.
    fn apply_simple_sgr(attrs: &mut CellAttributes, param: u16) {
        match param {
            0 => attrs.reset(),
            1 => attrs.bold = true,
            2 => attrs.faint = true,
            3 => attrs.italic = true,
            4 => {
                attrs.underline = true;
                attrs.underline_style = UnderlineStyle::Single;
            }
            5 => attrs.blink = true,
            7 => attrs.inverse = true,
            8 => attrs.hidden = true,
            9 => attrs.strikethrough = true,
            21 => {
                attrs.underline = true;
                attrs.underline_style = UnderlineStyle::Double;
            }
            22 => {
                attrs.bold = false;
                attrs.faint = false;
            }
            23 => attrs.italic = false,
            24 => {
                attrs.underline = false;
                attrs.underline_style = UnderlineStyle::None;
            }
            25 => attrs.blink = false,
            27 => attrs.inverse = false,
            28 => attrs.hidden = false,
            29 => attrs.strikethrough = false,
            30..=37 => attrs.fg = Color::Indexed((param - 30) as u8),
            39 => attrs.fg = Color::Default,
            40..=47 => attrs.bg = Color::Indexed((param - 40) as u8),
            49 => attrs.bg = Color::Default,
            53 => attrs.overline = true,
            55 => attrs.overline = false,
            59 => attrs.underline_color = Color::Default,
            90..=97 => attrs.fg = Color::Indexed((param - 90 + 8) as u8),
            100..=107 => attrs.bg = Color::Indexed((param - 100 + 8) as u8),
            _ => {
                log::debug!("Unknown SGR parameter: {}", param);
            }
        }
    }

    /// Handle OSC sequences
    fn handle_osc(&mut self, osc: OscAction) {
        match osc {
            OscAction::SetIconAndTitle(title) | OscAction::SetTitle(title) => {
                self.title = title.clone();
                self.screen.set_title(&title);
                self.title_changed = true;
            }
            OscAction::SetIconName(_) => {
                // Icon name is typically ignored in modern terminals
            }
            OscAction::Hyperlink { params: _, uri } => {
                if uri.is_empty() {
                    // End hyperlink
                    self.screen.cursor_mut().hyperlink_id = 0;
                } else {
                    // Start hyperlink
                    let id = self.screen.register_hyperlink(&uri);
                    self.screen.cursor_mut().hyperlink_id = id;
                }
            }
            OscAction::Clipboard { clipboard: _, data } => {
                // OSC 52 clipboard - handled by the application layer
                log::debug!("OSC 52 clipboard: {} bytes", data.len());
            }
            OscAction::SetColor { index, color } => {
                if color == "?" {
                    // OSC 4 color query - respond with current palette color
                    // Response: OSC 4 ; index ; rgb:RR/GG/BB ST
                    let (r, g, b) = Self::default_palette_color(index);
                    let response =
                        format!("\x1b]4;{};rgb:{:02x}/{:02x}/{:02x}\x1b\\", index, r, g, b);
                    self.queue_response(response.into_bytes());
                    log::debug!(
                        "OSC 4 query: color {} = rgb:{:02x}/{:02x}/{:02x}",
                        index,
                        r,
                        g,
                        b
                    );
                } else {
                    log::debug!("Set color {}: {}", index, color);
                }
            }
            OscAction::SetForegroundColor(color) => {
                if color == "?" {
                    // OSC 10 color query - respond with foreground color
                    // Default foreground: light gray
                    self.queue_response(b"\x1b]10;rgb:dd/dd/dd\x1b\\".to_vec());
                    log::debug!("OSC 10 query: foreground color");
                } else {
                    log::debug!("Set foreground color: {}", color);
                }
            }
            OscAction::SetBackgroundColor(color) => {
                if color == "?" {
                    // OSC 11 color query - respond with background color
                    // Default background: black
                    self.queue_response(b"\x1b]11;rgb:00/00/00\x1b\\".to_vec());
                    log::debug!("OSC 11 query: background color");
                } else {
                    log::debug!("Set background color: {}", color);
                }
            }
            OscAction::SetCursorColor(color) => {
                if color == "?" {
                    // OSC 12 color query - respond with cursor color
                    self.queue_response(b"\x1b]12;rgb:dd/dd/dd\x1b\\".to_vec());
                    log::debug!("OSC 12 query: cursor color");
                } else {
                    log::debug!("Set cursor color: {}", color);
                }
            }
            OscAction::SetCurrentDirectory(dir) => {
                log::debug!("Set current directory: {}", dir);
            }
            OscAction::ResetColor(_)
            | OscAction::ResetForegroundColor
            | OscAction::ResetBackgroundColor
            | OscAction::ResetCursorColor => {
                log::debug!("Reset color");
            }
            OscAction::Unknown { command, data } => {
                log::debug!("Unknown OSC {}: {}", command, data);
            }
        }
    }

    /// Handle DCS (Device Control String) sequences
    /// Used by tmux for capability queries (XTGETTCAP) and passthrough
    fn handle_dcs(&mut self, intermediates: &[u8], final_byte: u8, data: &[u8]) {
        match (intermediates, final_byte) {
            ([b'+'], b'q') => {
                // XTGETTCAP - Request terminal capability
                // tmux sends DCS + q <hex-encoded-cap-name> ST
                // We respond with DCS 1 + r <hex-encoded-cap-name>=<hex-encoded-value> ST
                // or DCS 0 + r <hex-encoded-cap-name> ST if not found
                self.handle_xtgettcap(data);
            }
            ([b'$'], b'q') => {
                // DECRQSS - Request Selection or Setting
                // tmux uses this to query terminal state
                self.handle_decrqss(data);
            }
            _ => {
                // Check for tmux passthrough: DCS tmux; <escaped-data> ST
                // The parser consumes the first byte after intermediates as the DCS final_byte,
                // so for "tmux;..." the 't' is consumed as final_byte and data starts with "mux;..."
                // Reconstruct the full string by prepending the final_byte.
                let data_str = String::from_utf8_lossy(data);
                let full_str = format!("{}{}", final_byte as char, data_str);
                if let Some(inner) = full_str.strip_prefix("tmux;") {
                    // tmux passthrough - unwrap the escaped content and process it
                    // In tmux passthrough, ESC ESC becomes ESC
                    let unescaped = inner.replace("\x1b\x1b", "\x1b");
                    self.process(unescaped.as_bytes());
                    log::debug!("DCS tmux passthrough: processed {} bytes", unescaped.len());
                } else {
                    log::debug!(
                        "DCS sequence: intermediates={:?} final={} data_len={}",
                        intermediates,
                        final_byte as char,
                        data.len()
                    );
                }
            }
        }
    }

    /// Handle DECRQSS - Request Selection or Setting
    /// Responds with current terminal settings
    fn handle_decrqss(&mut self, data: &[u8]) {
        let request = String::from_utf8_lossy(data);
        match request.as_ref() {
            "m" => {
                // Query current SGR attributes
                // Response: DCS 1 $ r <SGR-params> m ST
                let attrs = self.screen.cursor().attrs;
                let mut sgr_parts: Vec<String> = vec!["0".to_string()];
                if attrs.bold {
                    sgr_parts.push("1".to_string());
                }
                if attrs.faint {
                    sgr_parts.push("2".to_string());
                }
                if attrs.italic {
                    sgr_parts.push("3".to_string());
                }
                if attrs.underline {
                    sgr_parts.push("4".to_string());
                }
                if attrs.blink {
                    sgr_parts.push("5".to_string());
                }
                if attrs.inverse {
                    sgr_parts.push("7".to_string());
                }
                if attrs.hidden {
                    sgr_parts.push("8".to_string());
                }
                if attrs.strikethrough {
                    sgr_parts.push("9".to_string());
                }
                if attrs.overline {
                    sgr_parts.push("53".to_string());
                }
                let sgr_str = sgr_parts.join(";");
                let response = format!("\x1bP1$r{}m\x1b\\", sgr_str);
                self.queue_response(response.into_bytes());
                log::debug!("DECRQSS SGR: {}", sgr_str);
            }
            "r" => {
                // Query scroll region (DECSTBM)
                let (top, bottom) = self.screen.scroll_region();
                let response = format!("\x1bP1$r{};{}r\x1b\\", top + 1, bottom + 1);
                self.queue_response(response.into_bytes());
                log::debug!("DECRQSS DECSTBM: {};{}", top + 1, bottom + 1);
            }
            "\"p" => {
                // Query conformance level (DECSCL)
                // Respond as VT220, 8-bit controls accepted
                let response = "\x1bP1$r62;1\"p\x1b\\".to_string();
                self.queue_response(response.into_bytes());
                log::debug!("DECRQSS DECSCL: 62;1");
            }
            " q" => {
                // Query cursor style (DECSCUSR)
                let style_num = match (self.screen.cursor().style, self.screen.cursor().blinking) {
                    (CursorStyle::Block, true) => 1,
                    (CursorStyle::Block, false) => 2,
                    (CursorStyle::Underline, true) => 3,
                    (CursorStyle::Underline, false) => 4,
                    (CursorStyle::Bar, true) => 5,
                    (CursorStyle::Bar, false) => 6,
                };
                let response = format!("\x1bP1$r{} q\x1b\\", style_num);
                self.queue_response(response.into_bytes());
                log::debug!("DECRQSS DECSCUSR: {}", style_num);
            }
            _ => {
                // Not recognized - respond with DCS 0 $ r ST
                self.queue_response(b"\x1bP0$r\x1b\\".to_vec());
                log::debug!("DECRQSS unknown: {:?}", request);
            }
        }
    }

    /// Handle XTGETTCAP - Terminal capability query
    /// tmux sends hex-encoded capability names and expects hex-encoded responses
    fn handle_xtgettcap(&mut self, data: &[u8]) {
        let hex_names = String::from_utf8_lossy(data);

        // Each capability name is hex-encoded, separated by ';'
        for hex_name in hex_names.split(';') {
            let name = Self::hex_decode(hex_name);
            let value = Self::get_capability(&name);

            if let Some(val) = value {
                // Found: DCS 1 + r <hex-name>=<hex-value> ST
                let hex_val = Self::hex_encode(&val);
                let response = format!("\x1bP1+r{}={}\x1b\\", hex_name, hex_val);
                self.queue_response(response.into_bytes());
                log::debug!("XTGETTCAP: {} = {:?}", name, val);
            } else {
                // Not found: DCS 0 + r <hex-name> ST
                let response = format!("\x1bP0+r{}\x1b\\", hex_name);
                self.queue_response(response.into_bytes());
                log::debug!("XTGETTCAP: {} not found", name);
            }
        }
    }

    /// Get a terminal capability value by name
    fn get_capability(name: &str) -> Option<String> {
        match name {
            // True color support - critical for tmux
            "Tc" | "RGB" => Some(String::new()), // empty value = flag is set
            "colors" => Some("256".to_string()),
            // Terminal name
            "TN" | "name" => Some("xterm-256color".to_string()),
            // Cursor style capabilities
            "Ss" => Some("\x1b[%p1%d q".to_string()), // Set cursor style
            "Se" => Some("\x1b[2 q".to_string()),     // Reset cursor style
            // Synchronized output
            "Sync" => Some(String::new()),
            // Overline support
            "Smol" => Some("\x1b[53m".to_string()),
            "Rmol" => Some("\x1b[55m".to_string()),
            // Extended underline (colored, styled)
            "Setulc" => {
                Some("\x1b[58:2:%p1%{65536}%/%d:%p1%{256}%/%{255}%&%d:%p1%{255}%&%dm".to_string())
            }
            // Underline style
            "Smulx" => Some("\x1b[4:%p1%dm".to_string()),
            // Strikethrough
            "smxx" => Some("\x1b[9m".to_string()),
            "rmxx" => Some("\x1b[29m".to_string()),
            // OSC 52 clipboard
            "Ms" => Some("\x1b]52;%p1%s;%p2%s\x07".to_string()),
            // Focus events
            "fd" => Some("\x1b[?1004l".to_string()),
            "fe" => Some("\x1b[?1004h".to_string()),
            // Bracketed paste
            "BE" => Some("\x1b[?2004h".to_string()),
            "BD" => Some("\x1b[?2004l".to_string()),
            "PS" => Some("\x1b[200~".to_string()),
            "PE" => Some("\x1b[201~".to_string()),
            // Extended mouse
            "XM" => Some(String::new()),
            // Cursor movement
            "cup" => Some("\x1b[%i%p1%d;%p2%dH".to_string()),
            // Clear screen
            "clear" => Some("\x1b[H\x1b[2J".to_string()),
            // Key capabilities
            "kbs" => Some("\x7f".to_string()),
            "kcuu1" => Some("\x1bOA".to_string()),
            "kcud1" => Some("\x1bOB".to_string()),
            "kcuf1" => Some("\x1bOC".to_string()),
            "kcub1" => Some("\x1bOD".to_string()),
            "khome" => Some("\x1bOH".to_string()),
            "kend" => Some("\x1bOF".to_string()),
            "knp" => Some("\x1b[6~".to_string()),
            "kpp" => Some("\x1b[5~".to_string()),
            "kdch1" => Some("\x1b[3~".to_string()),
            "kich1" => Some("\x1b[2~".to_string()),
            // Function keys
            "kf1" => Some("\x1bOP".to_string()),
            "kf2" => Some("\x1bOQ".to_string()),
            "kf3" => Some("\x1bOR".to_string()),
            "kf4" => Some("\x1bOS".to_string()),
            "kf5" => Some("\x1b[15~".to_string()),
            "kf6" => Some("\x1b[17~".to_string()),
            "kf7" => Some("\x1b[18~".to_string()),
            "kf8" => Some("\x1b[19~".to_string()),
            "kf9" => Some("\x1b[20~".to_string()),
            "kf10" => Some("\x1b[21~".to_string()),
            "kf11" => Some("\x1b[23~".to_string()),
            "kf12" => Some("\x1b[24~".to_string()),
            // Alternate charset
            "acsc" => Some("``aaffggiijjkkllmmnnooppqqrrssttuuvvwwxxyyzz{{||}}~~".to_string()),
            // Bold/dim/italics/etc
            "bold" => Some("\x1b[1m".to_string()),
            "dim" => Some("\x1b[2m".to_string()),
            "sitm" => Some("\x1b[3m".to_string()),
            "ritm" => Some("\x1b[23m".to_string()),
            "smul" => Some("\x1b[4m".to_string()),
            "rmul" => Some("\x1b[24m".to_string()),
            "rev" => Some("\x1b[7m".to_string()),
            "sgr0" => Some("\x1b[m".to_string()),
            // Columns/lines
            "cols" => Some("80".to_string()),
            "lines" => Some("24".to_string()),
            _ => None,
        }
    }

    /// Get default palette color for index (xterm-256color compatible)
    fn default_palette_color(index: u8) -> (u8, u8, u8) {
        match index {
            // Standard colors
            0 => (0, 0, 0),       // Black
            1 => (205, 0, 0),     // Red
            2 => (0, 205, 0),     // Green
            3 => (205, 205, 0),   // Yellow
            4 => (0, 0, 238),     // Blue
            5 => (205, 0, 205),   // Magenta
            6 => (0, 205, 205),   // Cyan
            7 => (229, 229, 229), // White
            // Bright colors
            8 => (127, 127, 127),  // Bright Black
            9 => (255, 0, 0),      // Bright Red
            10 => (0, 255, 0),     // Bright Green
            11 => (255, 255, 0),   // Bright Yellow
            12 => (92, 92, 255),   // Bright Blue
            13 => (255, 0, 255),   // Bright Magenta
            14 => (0, 255, 255),   // Bright Cyan
            15 => (255, 255, 255), // Bright White
            // 216 color cube (indices 16-231)
            16..=231 => {
                let idx = index - 16;
                let b_val = idx % 6;
                let g_val = (idx / 6) % 6;
                let r_val = idx / 36;
                let r = if r_val > 0 { r_val * 40 + 55 } else { 0 };
                let g = if g_val > 0 { g_val * 40 + 55 } else { 0 };
                let b = if b_val > 0 { b_val * 40 + 55 } else { 0 };
                (r, g, b)
            }
            // Grayscale ramp (indices 232-255)
            232..=255 => {
                let v = (index - 232) * 10 + 8;
                (v, v, v)
            }
        }
    }

    /// Hex decode a string (e.g., "546F" -> "To")
    fn hex_decode(hex: &str) -> String {
        let bytes: Vec<u8> = hex
            .as_bytes()
            .chunks(2)
            .filter_map(|chunk| {
                if chunk.len() == 2 {
                    let high = (chunk[0] as char).to_digit(16)?;
                    let low = (chunk[1] as char).to_digit(16)?;
                    Some((high * 16 + low) as u8)
                } else {
                    None
                }
            })
            .collect();
        String::from_utf8_lossy(&bytes).to_string()
    }

    /// Hex encode a string (e.g., "To" -> "546f")
    fn hex_encode(s: &str) -> String {
        s.bytes().map(|b| format!("{:02x}", b)).collect()
    }

    /// Resize the terminal
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.screen.resize(Dimensions::new(cols, rows));
    }

    /// Create a snapshot of the current state
    #[allow(dead_code)]
    pub fn snapshot(&self) -> Snapshot {
        self.screen.snapshot(false)
    }

    /// Check if synchronized output mode is enabled
    /// When enabled, the terminal should buffer output and not render until disabled
    /// This prevents flickering and interleaving issues with TUI apps like Claude Code
    pub fn is_synchronized_output(&self) -> bool {
        self.screen.modes().synchronized_output
    }

    /// Take pending responses that need to be sent back to the PTY
    /// Returns all pending responses and clears the queue
    pub fn take_pending_responses(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.pending_responses)
    }

    /// Send a focus event notification
    /// When focus_events mode is enabled (CSI ? 1004 h), the terminal
    /// should send CSI I (focus in) or CSI O (focus out) to the application
    #[allow(dead_code)]
    pub fn notify_focus(&mut self, focused: bool) {
        if self.screen.modes().focus_events {
            let response = if focused {
                b"\x1b[I".to_vec()
            } else {
                b"\x1b[O".to_vec()
            };
            self.pending_responses.push(response);
        }
        self.pending_focus_events.push(focused);
    }

    /// Take pending focus events
    #[allow(dead_code)]
    pub fn take_pending_focus_events(&mut self) -> Vec<bool> {
        std::mem::take(&mut self.pending_focus_events)
    }

    /// Queue a response to be sent back to the PTY
    fn queue_response(&mut self, response: Vec<u8>) {
        self.pending_responses.push(response);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_new() {
        let term = Terminal::new(80, 24);
        assert_eq!(term.screen().cols(), 80);
        assert_eq!(term.screen().rows(), 24);
    }

    #[test]
    fn test_terminal_print() {
        let mut term = Terminal::new(80, 24);
        term.process(b"Hello");

        assert_eq!(term.screen().cursor().col, 5);
        assert_eq!(term.screen().line(0).cell(0).display_char(), 'H');
    }

    #[test]
    fn test_terminal_cursor_movement() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x1b[10;20H"); // Move to row 10, col 20

        assert_eq!(term.screen().cursor().row, 9);
        assert_eq!(term.screen().cursor().col, 19);
    }

    #[test]
    fn test_terminal_sgr() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x1b[1;31mRed Bold\x1b[0m");

        // Check that text was printed
        assert_eq!(term.screen().line(0).cell(0).display_char(), 'R');

        // After reset, attributes should be default
        let attrs = &term.screen().cursor().attrs;
        assert!(!attrs.bold);
        assert_eq!(attrs.fg, Color::Default);
    }

    #[test]
    fn test_terminal_erase() {
        let mut term = Terminal::new(10, 3);
        term.process(b"AAAAAAAAAA");
        term.process(b"\x1b[1;5H"); // Move to row 1, col 5
        term.process(b"\x1b[0K"); // Erase to end of line

        assert_eq!(term.screen().line(0).cell(3).display_char(), 'A');
        assert!(term.screen().line(0).cell(4).is_empty());
    }

    #[test]
    fn test_terminal_scroll_region() {
        let mut term = Terminal::new(10, 5);
        term.process(b"A\nB\nC\nD\nE");
        term.process(b"\x1b[2;4r"); // Set scroll region to rows 2-4
        term.process(b"\x1b[4;1H"); // Move to row 4
        term.process(b"\n"); // Should scroll within region

        assert_eq!(term.screen().line(0).cell(0).display_char(), 'A');
        // Row 1 should now have C (B scrolled out of region)
    }

    #[test]
    fn test_terminal_alternate_screen() {
        let mut term = Terminal::new(80, 24);
        term.process(b"Primary");
        term.process(b"\x1b[?1049h"); // Enter alternate screen

        assert!(term.screen().modes().alternate_screen);
        assert!(term.screen().line(0).cell(0).is_empty());

        term.process(b"Alternate");
        term.process(b"\x1b[?1049l"); // Exit alternate screen

        assert!(!term.screen().modes().alternate_screen);
        assert_eq!(term.screen().line(0).cell(0).display_char(), 'P');
    }

    #[test]
    fn test_terminal_title() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x1b]0;My Title\x07");

        assert_eq!(term.title(), "My Title");
        assert!(term.take_title_changed());
        assert!(!term.take_title_changed()); // Should be cleared
    }

    #[test]
    fn test_da1_response() {
        let mut term = Terminal::new(80, 24);
        // Send DA1 request: CSI ? c (or CSI 0 c)
        term.process(b"\x1b[?c");

        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        // Should respond as VT220 with extended capabilities
        let response = String::from_utf8_lossy(&responses[0]);
        assert!(response.starts_with("\x1b[?62;"));
    }

    #[test]
    fn test_da2_response() {
        let mut term = Terminal::new(80, 24);
        // Send DA2 request: CSI > c
        term.process(b"\x1b[>c");

        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        assert_eq!(String::from_utf8_lossy(&responses[0]), "\x1b[>0;136;0c");
    }

    #[test]
    fn test_xtversion_response() {
        let mut term = Terminal::new(80, 24);
        // Send XTVERSION request: CSI > q
        term.process(b"\x1b[>q");

        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        assert_eq!(
            String::from_utf8_lossy(&responses[0]),
            "\x1bP>|mochi(0.1.0)\x1b\\"
        );
    }

    #[test]
    fn test_decrqm_mode_query() {
        let mut term = Terminal::new(80, 24);

        // Query auto-wrap mode (7) - should be set by default
        term.process(b"\x1b[?7$p");
        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        assert_eq!(
            String::from_utf8_lossy(&responses[0]),
            "\x1b[?7;1$y" // 1 = set
        );

        // Query alternate screen (1049) - should be reset by default
        term.process(b"\x1b[?1049$p");
        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        assert_eq!(
            String::from_utf8_lossy(&responses[0]),
            "\x1b[?1049;2$y" // 2 = reset
        );

        // Query unknown mode - should be not recognized
        term.process(b"\x1b[?9999$p");
        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        assert_eq!(
            String::from_utf8_lossy(&responses[0]),
            "\x1b[?9999;0$y" // 0 = not recognized
        );
    }

    #[test]
    fn test_decxcpr_extended_cursor_report() {
        let mut term = Terminal::new(80, 24);
        term.process(b"\x1b[5;10H"); // Move to row 5, col 10

        // Send DECXCPR request: CSI ? 6 n
        term.process(b"\x1b[?6n");
        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        assert_eq!(String::from_utf8_lossy(&responses[0]), "\x1b[?5;10R");
    }

    #[test]
    fn test_hex_encode_decode() {
        assert_eq!(Terminal::hex_encode("Tc"), "5463");
        assert_eq!(Terminal::hex_decode("5463"), "Tc");
        assert_eq!(Terminal::hex_encode("RGB"), "524742");
        assert_eq!(Terminal::hex_decode("524742"), "RGB");
    }

    #[test]
    fn test_xtgettcap_tc() {
        let mut term = Terminal::new(80, 24);
        // Query Tc (true color) capability: DCS + q 5463 ST
        // 5463 = hex("Tc")
        term.process(b"\x1bP+q5463\x1b\\");

        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        let response = String::from_utf8_lossy(&responses[0]);
        // Should start with DCS 1 + r (capability found)
        assert!(response.starts_with("\x1bP1+r5463="));
    }

    #[test]
    fn test_xtgettcap_unknown() {
        let mut term = Terminal::new(80, 24);
        // Query unknown capability
        // "unknown" in hex = "756e6b6e6f776e"
        term.process(b"\x1bP+q756e6b6e6f776e\x1b\\");

        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        let response = String::from_utf8_lossy(&responses[0]);
        // Should start with DCS 0 + r (capability not found)
        assert!(response.starts_with("\x1bP0+r"));
    }

    #[test]
    fn test_da3_response() {
        let mut term = Terminal::new(80, 24);
        // Send DA3 request: CSI = c
        term.process(b"\x1b[=c");

        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        let response = String::from_utf8_lossy(&responses[0]);
        assert!(response.starts_with("\x1bP!|"));
    }

    #[test]
    fn test_sgr_underline_styles() {
        let mut term = Terminal::new(80, 24);

        // SGR 4 - single underline
        term.process(b"\x1b[4mA");
        let attrs = &term.screen().line(0).cell(0).attrs;
        assert!(attrs.underline);
        assert_eq!(attrs.underline_style, UnderlineStyle::Single);

        // SGR 21 - double underline
        term.process(b"\x1b[0m\x1b[21mB");
        let attrs = &term.screen().line(0).cell(1).attrs;
        assert!(attrs.underline);
        assert_eq!(attrs.underline_style, UnderlineStyle::Double);

        // SGR 24 - underline off
        term.process(b"\x1b[24mC");
        let attrs = &term.screen().line(0).cell(2).attrs;
        assert!(!attrs.underline);
        assert_eq!(attrs.underline_style, UnderlineStyle::None);
    }

    #[test]
    fn test_sgr_overline() {
        let mut term = Terminal::new(80, 24);

        // SGR 53 - overline on
        term.process(b"\x1b[53mA");
        let attrs = &term.screen().line(0).cell(0).attrs;
        assert!(attrs.overline);

        // SGR 55 - overline off
        term.process(b"\x1b[55mB");
        let attrs = &term.screen().line(0).cell(1).attrs;
        assert!(!attrs.overline);
    }

    #[test]
    fn test_sgr_underline_color_semicolon() {
        let mut term = Terminal::new(80, 24);

        // SGR 58;2;R;G;B - underline color RGB
        term.process(b"\x1b[58;2;255;128;64mA");
        let attrs = &term.screen().line(0).cell(0).attrs;
        assert_eq!(
            attrs.underline_color,
            Color::Rgb {
                r: 255,
                g: 128,
                b: 64
            }
        );

        // SGR 59 - reset underline color
        term.process(b"\x1b[59mB");
        let attrs = &term.screen().line(0).cell(1).attrs;
        assert_eq!(attrs.underline_color, Color::Default);
    }

    #[test]
    fn test_rep_repeat_character() {
        let mut term = Terminal::new(80, 24);
        // Print 'X' then repeat it 4 times
        term.process(b"X\x1b[4b");

        assert_eq!(term.screen().line(0).cell(0).display_char(), 'X');
        assert_eq!(term.screen().line(0).cell(1).display_char(), 'X');
        assert_eq!(term.screen().line(0).cell(2).display_char(), 'X');
        assert_eq!(term.screen().line(0).cell(3).display_char(), 'X');
        assert_eq!(term.screen().line(0).cell(4).display_char(), 'X');
    }

    #[test]
    fn test_cbt_backward_tab() {
        let mut term = Terminal::new(80, 24);
        // Move forward with tabs then backward
        term.process(b"\tA"); // Tab to col 8, print A at col 8
        assert_eq!(term.screen().cursor().col, 9); // After printing A
        term.process(b"\x1b[Z"); // CBT - backward tab
        assert_eq!(term.screen().cursor().col, 8); // Should go back to tab stop at 8
    }

    #[test]
    fn test_decstr_soft_reset() {
        let mut term = Terminal::new(80, 24);
        // Set some modes
        term.process(b"\x1b[?25l"); // Hide cursor
        term.process(b"\x1b[1m"); // Bold
        assert!(!term.screen().modes().cursor_visible);

        // DECSTR - soft reset
        term.process(b"\x1b[!p");

        // Modes should be reset
        assert!(term.screen().modes().cursor_visible);
        assert!(term.screen().modes().auto_wrap);
    }

    #[test]
    fn test_xtsave_xtrestore() {
        let mut term = Terminal::new(80, 24);

        // Set bracketed paste mode
        term.process(b"\x1b[?2004h");
        assert!(term.screen().modes().bracketed_paste);

        // Save the mode
        term.process(b"\x1b[?2004s");

        // Reset it
        term.process(b"\x1b[?2004l");
        assert!(!term.screen().modes().bracketed_paste);

        // Restore it
        term.process(b"\x1b[?2004r");
        assert!(term.screen().modes().bracketed_paste);
    }

    #[test]
    fn test_window_ops_report_size() {
        let mut term = Terminal::new(80, 24);

        // CSI ? 18 t - report terminal size in characters
        term.process(b"\x1b[?18t");
        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        assert_eq!(String::from_utf8_lossy(&responses[0]), "\x1b[8;24;80t");
    }

    #[test]
    fn test_window_ops_report_cell_size() {
        let mut term = Terminal::new(80, 24);

        // CSI ? 16 t - report cell size in pixels
        term.process(b"\x1b[?16t");
        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        assert_eq!(String::from_utf8_lossy(&responses[0]), "\x1b[6;16;8t");
    }

    #[test]
    fn test_osc_10_foreground_query() {
        let mut term = Terminal::new(80, 24);
        // OSC 10 ; ? ST - query foreground color
        term.process(b"\x1b]10;?\x1b\\");

        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        let response = String::from_utf8_lossy(&responses[0]);
        assert!(response.starts_with("\x1b]10;rgb:"));
    }

    #[test]
    fn test_osc_11_background_query() {
        let mut term = Terminal::new(80, 24);
        // OSC 11 ; ? ST - query background color
        term.process(b"\x1b]11;?\x1b\\");

        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        let response = String::from_utf8_lossy(&responses[0]);
        assert!(response.starts_with("\x1b]11;rgb:"));
    }

    #[test]
    fn test_osc_12_cursor_color_query() {
        let mut term = Terminal::new(80, 24);
        // OSC 12 ; ? ST - query cursor color
        term.process(b"\x1b]12;?\x1b\\");

        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        let response = String::from_utf8_lossy(&responses[0]);
        assert!(response.starts_with("\x1b]12;rgb:"));
    }

    #[test]
    fn test_decrqss_sgr() {
        let mut term = Terminal::new(80, 24);
        // Set bold
        term.process(b"\x1b[1m");
        // Query SGR: DCS $ q m ST
        term.process(b"\x1bP$qm\x1b\\");

        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        let response = String::from_utf8_lossy(&responses[0]);
        assert!(response.starts_with("\x1bP1$r"));
        assert!(response.contains("1")); // bold
    }

    #[test]
    fn test_decrqss_decstbm() {
        let mut term = Terminal::new(80, 24);
        // Set scroll region
        term.process(b"\x1b[5;20r");
        // Query scroll region: DCS $ q r ST
        term.process(b"\x1bP$qr\x1b\\");

        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        let response = String::from_utf8_lossy(&responses[0]);
        assert!(response.starts_with("\x1bP1$r"));
    }

    #[test]
    fn test_decrqss_cursor_style() {
        let mut term = Terminal::new(80, 24);
        // Set cursor to bar blinking
        term.process(b"\x1b[5 q");
        // Query cursor style: DCS $ q SP q ST
        term.process(b"\x1bP$q q\x1b\\");

        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        let response = String::from_utf8_lossy(&responses[0]);
        assert!(response.starts_with("\x1bP1$r"));
        assert!(response.contains("5")); // bar blinking
    }

    #[test]
    fn test_decrqss_unknown() {
        let mut term = Terminal::new(80, 24);
        // Query unknown setting
        term.process(b"\x1bP$qXYZ\x1b\\");

        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        let response = String::from_utf8_lossy(&responses[0]);
        assert!(response.starts_with("\x1bP0$r")); // not recognized
    }

    #[test]
    fn test_focus_events() {
        let mut term = Terminal::new(80, 24);

        // Focus event without mode enabled - no response queued
        term.notify_focus(true);
        let responses = term.take_pending_responses();
        assert!(responses.is_empty());

        // Enable focus events
        term.process(b"\x1b[?1004h");
        assert!(term.screen().modes().focus_events);

        // Focus in
        term.notify_focus(true);
        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        assert_eq!(String::from_utf8_lossy(&responses[0]), "\x1b[I");

        // Focus out
        term.notify_focus(false);
        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        assert_eq!(String::from_utf8_lossy(&responses[0]), "\x1b[O");
    }

    #[test]
    fn test_kitty_keyboard_query() {
        let mut term = Terminal::new(80, 24);
        // CSI > u - query keyboard protocol flags
        term.process(b"\x1b[>u");

        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        assert_eq!(String::from_utf8_lossy(&responses[0]), "\x1b[?0u");
    }

    #[test]
    fn test_xtgettcap_expanded() {
        let mut term = Terminal::new(80, 24);

        // Query Smulx (underline style) capability
        let hex_name = Terminal::hex_encode("Smulx");
        let query = format!("\x1bP+q{}\x1b\\", hex_name);
        term.process(query.as_bytes());

        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        let response = String::from_utf8_lossy(&responses[0]);
        assert!(response.starts_with("\x1bP1+r")); // found

        // Query BE (bracketed paste enable) capability
        let hex_name = Terminal::hex_encode("BE");
        let query = format!("\x1bP+q{}\x1b\\", hex_name);
        term.process(query.as_bytes());

        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        let response = String::from_utf8_lossy(&responses[0]);
        assert!(response.starts_with("\x1bP1+r")); // found
    }

    #[test]
    fn test_decrqm_mouse_modes() {
        let mut term = Terminal::new(80, 24);

        // Query mouse UTF-8 mode (1005) - should be reset
        term.process(b"\x1b[?1005$p");
        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        assert_eq!(
            String::from_utf8_lossy(&responses[0]),
            "\x1b[?1005;2$y" // 2 = reset
        );

        // Enable and query
        term.process(b"\x1b[?1005h");
        term.process(b"\x1b[?1005$p");
        let responses = term.take_pending_responses();
        assert_eq!(responses.len(), 1);
        assert_eq!(
            String::from_utf8_lossy(&responses[0]),
            "\x1b[?1005;1$y" // 1 = set
        );
    }

    #[test]
    fn test_default_palette_colors() {
        // Test standard colors
        assert_eq!(Terminal::default_palette_color(0), (0, 0, 0)); // Black
        assert_eq!(Terminal::default_palette_color(1), (205, 0, 0)); // Red
        assert_eq!(Terminal::default_palette_color(7), (229, 229, 229)); // White
                                                                         // Test bright colors
        assert_eq!(Terminal::default_palette_color(9), (255, 0, 0)); // Bright Red
        assert_eq!(Terminal::default_palette_color(15), (255, 255, 255)); // Bright White
                                                                          // Test grayscale
        assert_eq!(Terminal::default_palette_color(232), (8, 8, 8)); // Darkest gray
    }

    #[test]
    fn test_decscl_intermediate() {
        let mut term = Terminal::new(80, 24);
        // DECSCL - set conformance level (should not crash)
        term.process(b"\x1b[62;1\"p");
        // Just verify it doesn't panic
    }

    #[test]
    fn test_sgr_mixed_colon_semicolon() {
        let mut term = Terminal::new(80, 24);

        // Mixed: curly underline (colon-separated) + 256-color foreground (semicolon-separated)
        // 4:3;38;5;196 = curly underline + color index 196
        term.process(b"\x1b[4:3;38;5;196mA");
        let attrs = &term.screen().line(0).cell(0).attrs;
        assert!(attrs.underline);
        assert_eq!(attrs.underline_style, UnderlineStyle::Curly);
        assert_eq!(attrs.fg, Color::Indexed(196));
        // blink should NOT be set (bug was: param 5 matched as blink)
        assert!(!attrs.blink);
    }

    #[test]
    fn test_sgr_colon_rgb_foreground() {
        let mut term = Terminal::new(80, 24);

        // Colon-separated RGB foreground: 38:2:255:128:64
        term.process(b"\x1b[38:2:255:128:64mA");
        let attrs = &term.screen().line(0).cell(0).attrs;
        assert_eq!(
            attrs.fg,
            Color::Rgb {
                r: 255,
                g: 128,
                b: 64
            }
        );
    }

    #[test]
    fn test_xtrestore_cursor_visibility() {
        let mut term = Terminal::new(80, 24);

        // Cursor starts visible
        assert!(term.screen().modes().cursor_visible);

        // Hide cursor
        term.process(b"\x1b[?25l");
        assert!(!term.screen().modes().cursor_visible);

        // Save the hidden state
        term.process(b"\x1b[?25s");

        // Show cursor again
        term.process(b"\x1b[?25h");
        assert!(term.screen().modes().cursor_visible);

        // Restore saved (hidden) state - should route through Terminal::set_dec_mode
        term.process(b"\x1b[?25r");
        assert!(!term.screen().modes().cursor_visible);
        // Cursor object should also be updated (not just modes flag)
        assert!(!term.screen().cursor().visible);
    }
}
