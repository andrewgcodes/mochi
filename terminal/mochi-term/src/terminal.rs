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
    /// Window pixel dimensions (width, height) for CSI 14t responses
    window_pixel_size: (u32, u32),
    /// Cell pixel dimensions (width, height) for CSI 16t responses
    cell_pixel_size: (u32, u32),
    /// Foreground color RGB for OSC 10 query responses
    fg_color: (u8, u8, u8),
    /// Background color RGB for OSC 11 query responses
    bg_color: (u8, u8, u8),
    /// Cursor color RGB for OSC 12 query responses
    cursor_color: (u8, u8, u8),
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
            window_pixel_size: (0, 0),
            cell_pixel_size: (0, 0),
            fg_color: (212, 212, 212),
            bg_color: (30, 30, 30),
            cursor_color: (255, 255, 255),
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
            Action::Dcs { params, data } => {
                self.handle_dcs(params, &data);
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
        if csi.prefix == b'>' {
            self.handle_csi_gt(&csi);
            return;
        }

        if csi.private {
            self.handle_csi_private(&csi);
            return;
        }

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
                let mode = csi.param(0, 0);
                match mode {
                    5 => {
                        self.queue_response(b"\x1b[0n".to_vec());
                    }
                    6 => {
                        let row = self.screen.cursor().row + 1;
                        let col = self.screen.cursor().col + 1;
                        let response = format!("\x1b[{};{}R", row, col);
                        self.queue_response(response.into_bytes());
                    }
                    _ => {
                        log::debug!("DSR request with unknown mode: {}", mode);
                    }
                }
            }
            b't' => {
                self.handle_window_ops(&csi);
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

    /// Handle CSI sequences with `>` prefix (DA2, XTVERSION, etc.)
    fn handle_csi_gt(&mut self, csi: &CsiAction) {
        match csi.final_byte {
            b'c' => {
                self.queue_response(b"\x1b[>0;10;1c".to_vec());
            }
            b'q' => {
                let version = env!("CARGO_PKG_VERSION");
                let response = format!("\x1bP>|Mochi({})\x1b\\", version);
                self.queue_response(response.into_bytes());
            }
            _ => {
                log::debug!("Unknown CSI > {:?} {}", csi.params, csi.final_byte as char);
            }
        }
    }

    /// Handle CSI sequences with private marker (?)
    fn handle_csi_private(&mut self, csi: &CsiAction) {
        match csi.final_byte {
            b'h' => {
                for param in csi.params.iter() {
                    self.set_dec_mode(param, true);
                }
            }
            b'l' => {
                for param in csi.params.iter() {
                    self.set_dec_mode(param, false);
                }
            }
            b'c' => {
                self.queue_response(b"\x1b[?62;22c".to_vec());
            }
            b'n' => {
                let mode = csi.param(0, 0);
                match mode {
                    6 => {
                        let row = self.screen.cursor().row + 1;
                        let col = self.screen.cursor().col + 1;
                        let response = format!("\x1b[?{};{}R", row, col);
                        self.queue_response(response.into_bytes());
                    }
                    _ => {
                        log::debug!("Unknown private DSR mode: {}", mode);
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
        match (csi.intermediates.as_slice(), csi.final_byte, csi.prefix) {
            ([b' '], b'q', 0) => {
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
            ([b'$'], b'p', b'?') => {
                let mode = csi.param(0, 0);
                let known_modes: &[u16] = &[
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 25, 47, 1000, 1002, 1003,
                    1004, 1005, 1006, 1007, 1047, 1049, 2004, 2026,
                ];
                let ps = if known_modes.contains(&mode) {
                    if self.screen.modes().get_dec_mode(mode) { 1 } else { 2 }
                } else {
                    0
                };
                let response = format!("\x1b[?{};{}$y", mode, ps);
                self.queue_response(response.into_bytes());
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

    /// Handle CSI t window operations (used by tmux for size queries)
    fn handle_window_ops(&mut self, csi: &CsiAction) {
        let op = csi.param(0, 0);
        match op {
            14 => {
                let (w, h) = self.window_pixel_size;
                let response = format!("\x1b[4;{};{}t", h, w);
                self.queue_response(response.into_bytes());
            }
            16 => {
                let (cw, ch) = self.cell_pixel_size;
                let response = format!("\x1b[6;{};{}t", ch, cw);
                self.queue_response(response.into_bytes());
            }
            18 => {
                let rows = self.screen.rows();
                let cols = self.screen.cols();
                let response = format!("\x1b[8;{};{}t", rows, cols);
                self.queue_response(response.into_bytes());
            }
            22 => {
                log::debug!("CSI 22t: push title (ignored)");
            }
            23 => {
                log::debug!("CSI 23t: pop title (ignored)");
            }
            _ => {
                log::debug!("Unknown window op: {}", op);
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
                        "\x1b]10;rgb:{:04x}/{:04x}/{:04x}\x07",
                        (r as u16) << 8 | r as u16,
                        (g as u16) << 8 | g as u16,
                        (b as u16) << 8 | b as u16,
                    );
                    self.queue_response(response.into_bytes());
                } else {
                    log::debug!("Set foreground color: {}", color);
                }
            }
            OscAction::SetBackgroundColor(color) => {
                if color == "?" {
                    let (r, g, b) = self.bg_color;
                    let response = format!(
                        "\x1b]11;rgb:{:04x}/{:04x}/{:04x}\x07",
                        (r as u16) << 8 | r as u16,
                        (g as u16) << 8 | g as u16,
                        (b as u16) << 8 | b as u16,
                    );
                    self.queue_response(response.into_bytes());
                } else {
                    log::debug!("Set background color: {}", color);
                }
            }
            OscAction::SetCursorColor(color) => {
                if color == "?" {
                    let (r, g, b) = self.cursor_color;
                    let response = format!(
                        "\x1b]12;rgb:{:04x}/{:04x}/{:04x}\x07",
                        (r as u16) << 8 | r as u16,
                        (g as u16) << 8 | g as u16,
                        (b as u16) << 8 | b as u16,
                    );
                    self.queue_response(response.into_bytes());
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

    /// Handle DCS (Device Control String) sequences
    fn handle_dcs(&mut self, params: terminal_parser::Params, data: &[u8]) {
        let data_str = String::from_utf8_lossy(data);
        if let Some(query) = data_str.strip_prefix("$q") {
            match query {
                "m" => {
                    self.queue_response(b"\x1bP1$r0m\x1b\\".to_vec());
                }
                " q" => {
                    let style = match self.screen.cursor().style {
                        CursorStyle::Block => {
                            if self.screen.cursor().blinking { 1 } else { 2 }
                        }
                        CursorStyle::Underline => {
                            if self.screen.cursor().blinking { 3 } else { 4 }
                        }
                        CursorStyle::Bar => {
                            if self.screen.cursor().blinking { 5 } else { 6 }
                        }
                    };
                    let response = format!("\x1bP1$r{} q\x1b\\", style);
                    self.queue_response(response.into_bytes());
                }
                "r" => {
                    let (top, bottom) = self.screen.scroll_region();
                    let response = format!("\x1bP1$r{};{}r\x1b\\", top + 1, bottom);
                    self.queue_response(response.into_bytes());
                }
                _ => {
                    self.queue_response(b"\x1bP0$r\x1b\\".to_vec());
                    log::debug!("Unknown DECRQSS query: {}", query);
                }
            }
        } else {
            log::debug!("Unhandled DCS: params={:?} data={}", params, data_str);
        }
    }

    pub fn set_window_pixel_size(&mut self, width: u32, height: u32) {
        self.window_pixel_size = (width, height);
    }

    pub fn set_cell_pixel_size(&mut self, width: u32, height: u32) {
        self.cell_pixel_size = (width, height);
    }

    pub fn set_colors(&mut self, fg: (u8, u8, u8), bg: (u8, u8, u8), cursor: (u8, u8, u8)) {
        self.fg_color = fg;
        self.bg_color = bg;
        self.cursor_color = cursor;
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
