//! Terminal Performer
//!
//! Applies parsed terminal actions to the screen model.
//! This is the bridge between the parser and the screen.

use log::{debug, trace};
use mochi_core::{Attributes, Color, Screen};
use mochi_parser::{action::c0, Action, Params};

pub struct Performer {
    current_hyperlink_id: Option<u32>,
    next_hyperlink_id: u32,
}

impl Performer {
    pub fn new() -> Self {
        Performer {
            current_hyperlink_id: None,
            next_hyperlink_id: 1,
        }
    }

    pub fn perform(&mut self, screen: &mut Screen, action: Action) {
        match action {
            Action::Print(c) => {
                screen.put_char(c);
            }

            Action::Execute(byte) => {
                self.execute(screen, byte);
            }

            Action::CsiDispatch {
                params,
                intermediates,
                final_byte,
                private_marker,
            } => {
                self.csi_dispatch(screen, &params, &intermediates, final_byte, private_marker);
            }

            Action::EscDispatch {
                intermediates,
                final_byte,
            } => {
                self.esc_dispatch(screen, &intermediates, final_byte);
            }

            Action::OscDispatch { command, payload } => {
                self.osc_dispatch(screen, command, &payload);
            }

            Action::DcsDispatch { .. } => {
                debug!("DCS sequence (not fully implemented)");
            }

            Action::ApcDispatch { .. } => {
                debug!("APC sequence (ignored)");
            }

            Action::PmDispatch { .. } => {
                debug!("PM sequence (ignored)");
            }

            Action::SosDispatch { .. } => {
                debug!("SOS sequence (ignored)");
            }
        }
    }

    fn execute(&self, screen: &mut Screen, byte: u8) {
        match byte {
            c0::BEL => {
                screen.bell();
            }
            c0::BS => {
                screen.backspace();
            }
            c0::HT => {
                screen.tab();
            }
            c0::LF | c0::VT | c0::FF => {
                screen.linefeed();
            }
            c0::CR => {
                screen.carriage_return();
            }
            c0::SO => {
                debug!("Shift Out (G1 charset) - not implemented");
            }
            c0::SI => {
                debug!("Shift In (G0 charset) - not implemented");
            }
            _ => {
                trace!("Unhandled control character: 0x{:02X}", byte);
            }
        }
    }

    fn csi_dispatch(
        &mut self,
        screen: &mut Screen,
        params: &Params,
        intermediates: &[u8],
        final_byte: u8,
        private_marker: Option<u8>,
    ) {
        if let Some(b'?') = private_marker {
            self.csi_private_mode(screen, params, final_byte);
            return;
        }

        if !intermediates.is_empty() {
            match (intermediates.first(), final_byte) {
                (Some(b' '), b'q') => {
                    let style = params.get_or(0, 0);
                    self.set_cursor_style(screen, style);
                    return;
                }
                _ => {
                    debug!(
                        "CSI with intermediates {:?} {} - not implemented",
                        intermediates, final_byte as char
                    );
                    return;
                }
            }
        }

        match final_byte {
            b'A' => {
                let n = params.get_nonzero_or(0, 1) as usize;
                screen.move_cursor_up(n);
            }
            b'B' => {
                let n = params.get_nonzero_or(0, 1) as usize;
                screen.move_cursor_down(n);
            }
            b'C' => {
                let n = params.get_nonzero_or(0, 1) as usize;
                screen.move_cursor_forward(n);
            }
            b'D' => {
                let n = params.get_nonzero_or(0, 1) as usize;
                screen.move_cursor_backward(n);
            }
            b'E' => {
                let n = params.get_nonzero_or(0, 1) as usize;
                screen.move_cursor_down(n);
                screen.carriage_return();
            }
            b'F' => {
                let n = params.get_nonzero_or(0, 1) as usize;
                screen.move_cursor_up(n);
                screen.carriage_return();
            }
            b'G' => {
                let col = params.get_nonzero_or(0, 1) as usize;
                screen.move_cursor_to_col(col.saturating_sub(1));
            }
            b'H' | b'f' => {
                let row = params.get_nonzero_or(0, 1) as usize;
                let col = params.get_nonzero_or(1, 1) as usize;
                screen.move_cursor_to(row.saturating_sub(1), col.saturating_sub(1));
            }
            b'J' => {
                let mode = params.get_or(0, 0);
                screen.erase_in_display(mode);
            }
            b'K' => {
                let mode = params.get_or(0, 0);
                screen.erase_in_line(mode);
            }
            b'L' => {
                let n = params.get_nonzero_or(0, 1) as usize;
                screen.insert_lines(n);
            }
            b'M' => {
                let n = params.get_nonzero_or(0, 1) as usize;
                screen.delete_lines(n);
            }
            b'P' => {
                let n = params.get_nonzero_or(0, 1) as usize;
                screen.delete_chars(n);
            }
            b'S' => {
                let n = params.get_nonzero_or(0, 1) as usize;
                screen.scroll_up(n);
            }
            b'T' => {
                let n = params.get_nonzero_or(0, 1) as usize;
                screen.scroll_down(n);
            }
            b'X' => {
                let n = params.get_nonzero_or(0, 1) as usize;
                screen.erase_chars(n);
            }
            b'@' => {
                let n = params.get_nonzero_or(0, 1) as usize;
                screen.insert_chars(n);
            }
            b'd' => {
                let row = params.get_nonzero_or(0, 1) as usize;
                screen.move_cursor_to_row(row.saturating_sub(1));
            }
            b'g' => {
                let mode = params.get_or(0, 0);
                screen.clear_tab_stop(mode);
            }
            b'h' => {
                self.set_mode(screen, params, true);
            }
            b'l' => {
                self.set_mode(screen, params, false);
            }
            b'm' => {
                self.sgr(screen, params);
            }
            b'n' => {
                let mode = params.get_or(0, 0);
                self.device_status_report(screen, mode);
            }
            b'r' => {
                let top = params.get_nonzero_or(0, 1) as usize;
                let bottom = params.get_or(1, screen.rows() as u16) as usize;
                if top < bottom {
                    screen.set_scroll_region(top.saturating_sub(1), bottom.saturating_sub(1));
                }
            }
            b's' => {
                screen.save_cursor();
            }
            b'u' => {
                screen.restore_cursor();
            }
            b'`' => {
                let col = params.get_nonzero_or(0, 1) as usize;
                screen.move_cursor_to_col(col.saturating_sub(1));
            }
            _ => {
                debug!(
                    "Unhandled CSI sequence: {:?} {}",
                    params, final_byte as char
                );
            }
        }
    }

    fn csi_private_mode(&mut self, screen: &mut Screen, params: &Params, final_byte: u8) {
        let enable = final_byte == b'h';

        for i in 0..params.len() {
            if let Some(mode) = params.get(i) {
                match mode {
                    1 => {
                        debug!("DECCKM (cursor keys mode): {}", enable);
                    }
                    3 => {
                        debug!("DECCOLM (132 column mode): {}", enable);
                    }
                    5 => {
                        debug!("DECSCNM (reverse video): {}", enable);
                    }
                    6 => {
                        screen.modes.origin_mode = enable;
                        if enable {
                            screen.move_cursor_to(0, 0);
                        }
                    }
                    7 => {
                        screen.modes.autowrap = enable;
                    }
                    12 => {
                        screen.cursor_mut().blinking = enable;
                    }
                    25 => {
                        screen.modes.cursor_visible = enable;
                        screen.cursor_mut().visible = enable;
                    }
                    47 => {
                        if enable {
                            screen.enter_alternate_screen();
                        } else {
                            screen.exit_alternate_screen();
                        }
                    }
                    1000 => {
                        screen.modes.mouse_tracking = if enable {
                            mochi_core::screen::MouseMode::VT200
                        } else {
                            mochi_core::screen::MouseMode::None
                        };
                    }
                    1002 => {
                        screen.modes.mouse_tracking = if enable {
                            mochi_core::screen::MouseMode::ButtonEvent
                        } else {
                            mochi_core::screen::MouseMode::None
                        };
                    }
                    1003 => {
                        screen.modes.mouse_tracking = if enable {
                            mochi_core::screen::MouseMode::AnyEvent
                        } else {
                            mochi_core::screen::MouseMode::None
                        };
                    }
                    1004 => {
                        screen.modes.focus_events = enable;
                    }
                    1005 => {
                        screen.modes.mouse_encoding = if enable {
                            mochi_core::screen::MouseEncoding::Utf8
                        } else {
                            mochi_core::screen::MouseEncoding::Default
                        };
                    }
                    1006 => {
                        screen.modes.mouse_encoding = if enable {
                            mochi_core::screen::MouseEncoding::Sgr
                        } else {
                            mochi_core::screen::MouseEncoding::Default
                        };
                    }
                    1015 => {
                        screen.modes.mouse_encoding = if enable {
                            mochi_core::screen::MouseEncoding::Urxvt
                        } else {
                            mochi_core::screen::MouseEncoding::Default
                        };
                    }
                    1047 => {
                        if enable {
                            screen.enter_alternate_screen();
                        } else {
                            screen.exit_alternate_screen();
                        }
                    }
                    1048 => {
                        if enable {
                            screen.save_cursor();
                        } else {
                            screen.restore_cursor();
                        }
                    }
                    1049 => {
                        if enable {
                            screen.save_cursor();
                            screen.enter_alternate_screen();
                            screen.erase_in_display(2);
                        } else {
                            screen.exit_alternate_screen();
                            screen.restore_cursor();
                        }
                    }
                    2004 => {
                        screen.modes.bracketed_paste = enable;
                    }
                    _ => {
                        debug!("Unknown private mode {}: {}", mode, enable);
                    }
                }
            }
        }
    }

    fn set_mode(&self, screen: &mut Screen, params: &Params, enable: bool) {
        for i in 0..params.len() {
            if let Some(mode) = params.get(i) {
                match mode {
                    4 => {
                        screen.modes.insert_mode = enable;
                    }
                    20 => {
                        screen.modes.linefeed_mode = enable;
                    }
                    _ => {
                        debug!("Unknown mode {}: {}", mode, enable);
                    }
                }
            }
        }
    }

    fn sgr(&mut self, screen: &mut Screen, params: &Params) {
        if params.is_empty() {
            screen.attrs = Attributes::default();
            screen.fg = Color::Default;
            screen.bg = Color::Default;
            return;
        }

        let mut i = 0;
        while i < params.len() {
            let param = params.get(i).unwrap_or(0);

            match param {
                0 => {
                    screen.attrs = Attributes::default();
                    screen.fg = Color::Default;
                    screen.bg = Color::Default;
                }
                1 => screen.attrs.bold = true,
                2 => screen.attrs.faint = true,
                3 => screen.attrs.italic = true,
                4 => screen.attrs.underline = true,
                5 | 6 => screen.attrs.blink = true,
                7 => screen.attrs.inverse = true,
                8 => screen.attrs.hidden = true,
                9 => screen.attrs.strikethrough = true,

                21 => screen.attrs.bold = false,
                22 => {
                    screen.attrs.bold = false;
                    screen.attrs.faint = false;
                }
                23 => screen.attrs.italic = false,
                24 => screen.attrs.underline = false,
                25 => screen.attrs.blink = false,
                27 => screen.attrs.inverse = false,
                28 => screen.attrs.hidden = false,
                29 => screen.attrs.strikethrough = false,

                30..=37 => {
                    screen.fg = Color::Indexed((param - 30) as u8);
                }
                38 => {
                    if let Some(color) = self.parse_extended_color(params, &mut i) {
                        screen.fg = color;
                    }
                }
                39 => {
                    screen.fg = Color::Default;
                }

                40..=47 => {
                    screen.bg = Color::Indexed((param - 40) as u8);
                }
                48 => {
                    if let Some(color) = self.parse_extended_color(params, &mut i) {
                        screen.bg = color;
                    }
                }
                49 => {
                    screen.bg = Color::Default;
                }

                90..=97 => {
                    screen.fg = Color::Indexed((param - 90 + 8) as u8);
                }
                100..=107 => {
                    screen.bg = Color::Indexed((param - 100 + 8) as u8);
                }

                _ => {
                    trace!("Unknown SGR parameter: {}", param);
                }
            }

            i += 1;
        }
    }

    fn parse_extended_color(&self, params: &Params, i: &mut usize) -> Option<Color> {
        let next = params.get(*i + 1)?;

        match next {
            2 => {
                let r = params.get(*i + 2)? as u8;
                let g = params.get(*i + 3)? as u8;
                let b = params.get(*i + 4)? as u8;
                *i += 4;
                Some(Color::Rgb(r, g, b))
            }
            5 => {
                let idx = params.get(*i + 2)? as u8;
                *i += 2;
                Some(Color::Indexed(idx))
            }
            _ => None,
        }
    }

    fn set_cursor_style(&self, screen: &mut Screen, style: u16) {
        use mochi_core::CursorStyle;

        let (cursor_style, blinking) = match style {
            0 | 1 => (CursorStyle::Block, true),
            2 => (CursorStyle::Block, false),
            3 => (CursorStyle::Underline, true),
            4 => (CursorStyle::Underline, false),
            5 => (CursorStyle::Bar, true),
            6 => (CursorStyle::Bar, false),
            _ => return,
        };

        screen.cursor_mut().style = cursor_style;
        screen.cursor_mut().blinking = blinking;
    }

    fn device_status_report(&self, screen: &Screen, mode: u16) {
        match mode {
            5 => {
                debug!("DSR: Device status requested (would respond ESC[0n)");
            }
            6 => {
                let row = screen.cursor().row + 1;
                let col = screen.cursor().col + 1;
                debug!(
                    "DSR: Cursor position requested (would respond ESC[{};{}R)",
                    row, col
                );
            }
            _ => {
                debug!("Unknown DSR mode: {}", mode);
            }
        }
    }

    fn esc_dispatch(&mut self, screen: &mut Screen, intermediates: &[u8], final_byte: u8) {
        match (intermediates.first(), final_byte) {
            (None, b'7') => {
                screen.save_cursor();
            }
            (None, b'8') => {
                screen.restore_cursor();
            }
            (None, b'D') => {
                screen.linefeed();
            }
            (None, b'E') => {
                screen.carriage_return();
                screen.linefeed();
            }
            (None, b'H') => {
                screen.set_tab_stop();
            }
            (None, b'M') => {
                screen.reverse_index();
            }
            (None, b'c') => {
                screen.reset();
            }
            (Some(b'#'), b'8') => {
                for row in 0..screen.rows() {
                    for col in 0..screen.cols() {
                        if let Some(cell) = screen.get_cell_mut(row, col) {
                            cell.character = 'E';
                        }
                    }
                }
            }
            (Some(b'('), _) => {
                debug!("G0 charset selection: {}", final_byte as char);
            }
            (Some(b')'), _) => {
                debug!("G1 charset selection: {}", final_byte as char);
            }
            _ => {
                debug!(
                    "Unhandled ESC sequence: {:?} {}",
                    intermediates, final_byte as char
                );
            }
        }
    }

    fn osc_dispatch(&mut self, screen: &mut Screen, command: u16, payload: &str) {
        match command {
            0 => {
                screen.title = payload.to_string();
                screen.icon_name = payload.to_string();
                debug!("Set title and icon: {}", payload);
            }
            1 => {
                screen.icon_name = payload.to_string();
                debug!("Set icon name: {}", payload);
            }
            2 => {
                screen.title = payload.to_string();
                debug!("Set title: {}", payload);
            }
            8 => {
                if payload.is_empty() {
                    self.current_hyperlink_id = None;
                } else {
                    self.current_hyperlink_id = Some(self.next_hyperlink_id);
                    self.next_hyperlink_id += 1;
                    debug!("Hyperlink: {}", payload);
                }
            }
            52 => {
                debug!(
                    "OSC 52 clipboard operation (security-sensitive, not implemented by default)"
                );
            }
            _ => {
                debug!("Unknown OSC command {}: {}", command, payload);
            }
        }
    }
}

impl Default for Performer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mochi_parser::Parser;

    fn apply_sequence(screen: &mut Screen, input: &[u8]) {
        let mut parser = Parser::new();
        let mut performer = Performer::new();
        parser.parse(input, |action| {
            performer.perform(screen, action);
        });
    }

    #[test]
    fn test_cursor_movement() {
        let mut screen = Screen::new(80, 24);
        apply_sequence(&mut screen, b"\x1b[10;20H");
        assert_eq!(screen.cursor().row, 9);
        assert_eq!(screen.cursor().col, 19);
    }

    #[test]
    fn test_cursor_up() {
        let mut screen = Screen::new(80, 24);
        screen.move_cursor_to(10, 10);
        apply_sequence(&mut screen, b"\x1b[5A");
        assert_eq!(screen.cursor().row, 5);
    }

    #[test]
    fn test_erase_display() {
        let mut screen = Screen::new(80, 24);
        apply_sequence(&mut screen, b"Hello");
        apply_sequence(&mut screen, b"\x1b[2J");
        assert_eq!(screen.get_cell(0, 0).unwrap().character, ' ');
    }

    #[test]
    fn test_sgr_bold() {
        let mut screen = Screen::new(80, 24);
        apply_sequence(&mut screen, b"\x1b[1m");
        assert!(screen.attrs.bold);
    }

    #[test]
    fn test_sgr_colors() {
        let mut screen = Screen::new(80, 24);
        apply_sequence(&mut screen, b"\x1b[31m");
        assert_eq!(screen.fg, Color::Indexed(1));

        apply_sequence(&mut screen, b"\x1b[42m");
        assert_eq!(screen.bg, Color::Indexed(2));
    }

    #[test]
    fn test_sgr_256_color() {
        let mut screen = Screen::new(80, 24);
        apply_sequence(&mut screen, b"\x1b[38;5;196m");
        assert_eq!(screen.fg, Color::Indexed(196));
    }

    #[test]
    fn test_sgr_truecolor() {
        let mut screen = Screen::new(80, 24);
        apply_sequence(&mut screen, b"\x1b[38;2;255;128;64m");
        assert_eq!(screen.fg, Color::Rgb(255, 128, 64));
    }

    #[test]
    fn test_alternate_screen() {
        let mut screen = Screen::new(80, 24);
        apply_sequence(&mut screen, b"Hello");
        apply_sequence(&mut screen, b"\x1b[?1049h");
        assert!(screen.is_using_alternate());
        assert_eq!(screen.get_cell(0, 0).unwrap().character, ' ');

        apply_sequence(&mut screen, b"\x1b[?1049l");
        assert!(!screen.is_using_alternate());
        assert_eq!(screen.get_cell(0, 0).unwrap().character, 'H');
    }

    #[test]
    fn test_scroll_region() {
        let mut screen = Screen::new(80, 24);
        apply_sequence(&mut screen, b"\x1b[5;15r");
        assert_eq!(screen.scroll_region().top, 4);
        assert_eq!(screen.scroll_region().bottom, 14);
    }

    #[test]
    fn test_save_restore_cursor() {
        let mut screen = Screen::new(80, 24);
        apply_sequence(&mut screen, b"\x1b[10;20H\x1b7");
        apply_sequence(&mut screen, b"\x1b[1;1H");
        assert_eq!(screen.cursor().row, 0);

        apply_sequence(&mut screen, b"\x1b8");
        assert_eq!(screen.cursor().row, 9);
        assert_eq!(screen.cursor().col, 19);
    }

    #[test]
    fn test_osc_title() {
        let mut screen = Screen::new(80, 24);
        apply_sequence(&mut screen, b"\x1b]0;My Terminal\x07");
        assert_eq!(screen.title, "My Terminal");
    }
}
