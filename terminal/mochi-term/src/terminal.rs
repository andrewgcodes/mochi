//! Terminal state management
//!
//! Integrates the parser and screen model to handle terminal emulation.

use terminal_core::{Color, CursorStyle, Dimensions, Screen, Snapshot};
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
    /// Foreground color as (r, g, b) for OSC 10 query responses
    fg_color: (u8, u8, u8),
    /// Background color as (r, g, b) for OSC 11 query responses
    bg_color: (u8, u8, u8),
    /// Terminal pixel dimensions (width, height) for CSI 14 t responses
    pixel_size: (u32, u32),
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
            fg_color: (212, 212, 212),
            bg_color: (30, 30, 30),
            pixel_size: (0, 0),
        }
    }

    pub fn set_colors(&mut self, fg: (u8, u8, u8), bg: (u8, u8, u8)) {
        self.fg_color = fg;
        self.bg_color = bg;
    }

    pub fn set_pixel_size(&mut self, width: u32, height: u32) {
        self.pixel_size = (width, height);
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
            Action::Dcs { .. } => {
                // DCS sequences are currently not implemented
                log::debug!("DCS sequence ignored");
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
        // Handle CSI > sequences (DA2, XTVERSION, etc.)
        if csi.marker == b'>' {
            self.handle_csi_gt(&csi);
            return;
        }

        // Handle private sequences
        if csi.private {
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
            b't' => {
                // Window manipulation (XTWINOPS)
                self.handle_window_ops(&csi);
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

    /// Handle CSI > sequences (DA2, XTVERSION, etc.)
    fn handle_csi_gt(&mut self, csi: &CsiAction) {
        match csi.final_byte {
            b'c' => {
                // DA2 - Secondary Device Attributes
                // tmux uses this to detect terminal capabilities
                // Response format: CSI > Pp ; Pv ; Pc c
                // Pp=65 (xterm), Pv=version, Pc=0
                // We identify as xterm patch level 377 for broad compatibility
                self.queue_response(b"\x1b[>65;377;0c".to_vec());
                log::debug!("DA2 request: responding as xterm 377");
            }
            b'q' => {
                // XTVERSION - Report terminal name and version
                // Response: DCS > | terminal-name(version) ST
                self.queue_response(b"\x1bP>|mochi(0.1.0)\x1b\\".to_vec());
                log::debug!("XTVERSION request: responding with mochi version");
            }
            b'm' | b'n' => {
                // xterm modifyOtherKeys / modifyKeys - acknowledge but ignore
                log::debug!("CSI > {} ignored", csi.final_byte as char);
            }
            _ => {
                log::debug!(
                    "Unknown CSI > sequence: {:?}{}",
                    csi.params,
                    csi.final_byte as char
                );
            }
        }
    }

    /// Handle CSI sequences with private marker (?)
    fn handle_csi_private(&mut self, csi: &CsiAction) {
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
                // Respond as VT220 with features tmux expects:
                // 62 = VT220, 4 = sixel, 22 = ANSI color
                // This tells tmux we support 256 colors and standard features
                self.queue_response(b"\x1b[?62;22c".to_vec());
                log::debug!("DA1 request: responding as VT220 with ANSI color");
            }
            b'n' => {
                // DECDSR - DEC-specific Device Status Report
                let mode = csi.param(0, 0);
                match mode {
                    6 => {
                        // Extended cursor position report (DECXCPR)
                        let row = self.screen.cursor().row + 1;
                        let col = self.screen.cursor().col + 1;
                        let response = format!("\x1b[?{};{}R", row, col);
                        self.queue_response(response.into_bytes());
                        log::debug!("DECXCPR: responding row={} col={}", row, col);
                    }
                    _ => {
                        log::debug!("Unknown DECDSR mode: {}", mode);
                    }
                }
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

    /// Handle CSI sequences with intermediate bytes
    fn handle_csi_intermediate(&mut self, csi: &CsiAction) {
        match (csi.intermediates.as_slice(), csi.final_byte, csi.private) {
            ([b' '], b'q', false) => {
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
            ([b'!'], b'p', false) => {
                // DECSTR - Soft Terminal Reset
                self.screen.modes_mut().reset();
                self.screen.clear_scroll_region();
                self.screen.cursor_mut().attrs = Default::default();
                log::debug!("DECSTR: soft terminal reset");
            }
            ([b'$'], b'p', true) => {
                // DECRQM - Request Mode (DEC private)
                let mode = csi.param(0, 0);
                let state = if self.screen.modes().get_dec_mode(mode) {
                    1 // set
                } else {
                    2 // reset
                };
                let response = format!("\x1b[?{};{}$y", mode, state);
                self.queue_response(response.into_bytes());
                log::debug!("DECRQM: mode {} is {}", mode, state);
            }
            ([b'$'], b'p', false) => {
                // ANSI DECRQM - Request Mode (standard)
                let mode = csi.param(0, 0);
                let state = match mode {
                    4 => {
                        if self.screen.modes().insert_mode {
                            1
                        } else {
                            2
                        }
                    }
                    20 => {
                        if self.screen.modes().linefeed_mode {
                            1
                        } else {
                            2
                        }
                    }
                    _ => 0, // not recognized
                };
                let response = format!("\x1b[{};{}$y", mode, state);
                self.queue_response(response.into_bytes());
                log::debug!("ANSI DECRQM: mode {} is {}", mode, state);
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

    /// Handle CSI t - Window manipulation (XTWINOPS)
    fn handle_window_ops(&mut self, csi: &CsiAction) {
        let op = csi.param(0, 0);
        match op {
            14 => {
                // Report window size in pixels
                let (pw, ph) = self.pixel_size;
                let response = format!("\x1b[4;{};{}t", ph, pw);
                self.queue_response(response.into_bytes());
                log::debug!("XTWINOPS 14: report pixel size {}x{}", pw, ph);
            }
            18 => {
                // Report terminal size in characters
                let rows = self.screen.rows();
                let cols = self.screen.cols();
                let response = format!("\x1b[8;{};{}t", rows, cols);
                self.queue_response(response.into_bytes());
                log::debug!("XTWINOPS 18: report char size {}x{}", cols, rows);
            }
            22 => {
                // Push title to stack (tmux uses this)
                log::debug!("XTWINOPS 22: push title (acknowledged)");
            }
            23 => {
                // Pop title from stack (tmux uses this)
                log::debug!("XTWINOPS 23: pop title (acknowledged)");
            }
            _ => {
                log::debug!("XTWINOPS {}: ignored", op);
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
    fn handle_sgr(&mut self, csi: &CsiAction) {
        let attrs = &mut self.screen.cursor_mut().attrs;

        if csi.params.is_empty() {
            attrs.reset();
            return;
        }

        let mut i = 0;
        let params: Vec<u16> = csi.params.iter().collect();

        while i < params.len() {
            let param = params[i];
            match param {
                0 => attrs.reset(),
                1 => attrs.bold = true,
                2 => attrs.faint = true,
                3 => attrs.italic = true,
                4 => attrs.underline = true,
                5 => attrs.blink = true,
                7 => attrs.inverse = true,
                8 => attrs.hidden = true,
                9 => attrs.strikethrough = true,
                21 => attrs.bold = false, // Double underline or bold off
                22 => {
                    attrs.bold = false;
                    attrs.faint = false;
                }
                23 => attrs.italic = false,
                24 => attrs.underline = false,
                25 => attrs.blink = false,
                27 => attrs.inverse = false,
                28 => attrs.hidden = false,
                29 => attrs.strikethrough = false,
                30..=37 => {
                    attrs.fg = Color::Indexed((param - 30) as u8);
                }
                38 => {
                    // Extended foreground color
                    if i + 1 < params.len() {
                        match params[i + 1] {
                            5 => {
                                // 256 color: 38;5;N
                                if i + 2 < params.len() {
                                    attrs.fg = Color::Indexed(params[i + 2] as u8);
                                    i += 2;
                                }
                            }
                            2 => {
                                // True color: 38;2;R;G;B
                                if i + 4 < params.len() {
                                    attrs.fg = Color::Rgb {
                                        r: params[i + 2] as u8,
                                        g: params[i + 3] as u8,
                                        b: params[i + 4] as u8,
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
                    // Extended background color
                    if i + 1 < params.len() {
                        match params[i + 1] {
                            5 => {
                                // 256 color: 48;5;N
                                if i + 2 < params.len() {
                                    attrs.bg = Color::Indexed(params[i + 2] as u8);
                                    i += 2;
                                }
                            }
                            2 => {
                                // True color: 48;2;R;G;B
                                if i + 4 < params.len() {
                                    attrs.bg = Color::Rgb {
                                        r: params[i + 2] as u8,
                                        g: params[i + 3] as u8,
                                        b: params[i + 4] as u8,
                                    };
                                    i += 4;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                49 => attrs.bg = Color::Default,
                90..=97 => {
                    // Bright foreground colors
                    attrs.fg = Color::Indexed((param - 90 + 8) as u8);
                }
                100..=107 => {
                    // Bright background colors
                    attrs.bg = Color::Indexed((param - 100 + 8) as u8);
                }
                _ => {
                    log::debug!("Unknown SGR parameter: {}", param);
                }
            }
            i += 1;
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
                log::debug!("Set color {}: {}", index, color);
            }
            OscAction::SetForegroundColor(color) => {
                if color == "?" {
                    let (r, g, b) = self.fg_color;
                    let response = format!(
                        "\x1b]10;rgb:{:02x}{:02x}/{:02x}{:02x}/{:02x}{:02x}\x1b\\",
                        r, r, g, g, b, b
                    );
                    self.queue_response(response.into_bytes());
                    log::debug!("OSC 10 query: responding with fg color");
                } else {
                    log::debug!("Set foreground color: {}", color);
                }
            }
            OscAction::SetBackgroundColor(color) => {
                if color == "?" {
                    let (r, g, b) = self.bg_color;
                    let response = format!(
                        "\x1b]11;rgb:{:02x}{:02x}/{:02x}{:02x}/{:02x}{:02x}\x1b\\",
                        r, r, g, g, b, b
                    );
                    self.queue_response(response.into_bytes());
                    log::debug!("OSC 11 query: responding with bg color");
                } else {
                    log::debug!("Set background color: {}", color);
                }
            }
            OscAction::SetCursorColor(color) => {
                log::debug!("Set cursor color: {}", color);
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
}
