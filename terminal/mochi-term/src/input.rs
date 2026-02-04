//! Keyboard and mouse input handling
//!
//! This module handles encoding keyboard and mouse input into terminal escape sequences.
//! It supports:
//! - Normal key input (ASCII)
//! - Special keys (arrows, function keys, etc.)
//! - Modifier keys (Ctrl, Alt, Shift)
//! - Mouse reporting modes

use mochi_core::screen::{MouseEncoding, MouseMode};

/// Encode a key press into terminal input bytes
pub fn encode_key(
    key: Key,
    modifiers: Modifiers,
    app_cursor: bool,
    app_keypad: bool,
) -> Vec<u8> {
    match key {
        Key::Char(c) => encode_char(c, modifiers),
        Key::Enter => {
            if modifiers.ctrl {
                vec![0x0A] // Ctrl+Enter = LF
            } else {
                vec![0x0D] // CR
            }
        }
        Key::Tab => {
            if modifiers.shift {
                vec![0x1B, b'[', b'Z'] // Backtab
            } else {
                vec![0x09]
            }
        }
        Key::Backspace => {
            if modifiers.ctrl {
                vec![0x08] // Ctrl+Backspace
            } else if modifiers.alt {
                vec![0x1B, 0x7F] // Alt+Backspace
            } else {
                vec![0x7F] // DEL
            }
        }
        Key::Escape => vec![0x1B],
        Key::Up => encode_cursor_key(b'A', modifiers, app_cursor),
        Key::Down => encode_cursor_key(b'B', modifiers, app_cursor),
        Key::Right => encode_cursor_key(b'C', modifiers, app_cursor),
        Key::Left => encode_cursor_key(b'D', modifiers, app_cursor),
        Key::Home => encode_special_key(b'H', 1, modifiers, app_cursor),
        Key::End => encode_special_key(b'F', 4, modifiers, app_cursor),
        Key::PageUp => encode_special_key(b'~', 5, modifiers, false),
        Key::PageDown => encode_special_key(b'~', 6, modifiers, false),
        Key::Insert => encode_special_key(b'~', 2, modifiers, false),
        Key::Delete => encode_special_key(b'~', 3, modifiers, false),
        Key::F(n) => encode_function_key(n, modifiers),
    }
}

/// Encode a character with modifiers
fn encode_char(c: char, modifiers: Modifiers) -> Vec<u8> {
    if modifiers.ctrl {
        // Ctrl+letter produces control character
        if c.is_ascii_alphabetic() {
            let ctrl_char = (c.to_ascii_uppercase() as u8) - b'A' + 1;
            if modifiers.alt {
                return vec![0x1B, ctrl_char];
            }
            return vec![ctrl_char];
        }
        // Ctrl+special characters
        match c {
            '@' => return vec![0x00],
            '[' => return vec![0x1B],
            '\\' => return vec![0x1C],
            ']' => return vec![0x1D],
            '^' => return vec![0x1E],
            '_' => return vec![0x1F],
            '?' => return vec![0x7F],
            _ => {}
        }
    }

    if modifiers.alt {
        // Alt+char sends ESC followed by char
        let mut bytes = vec![0x1B];
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        bytes.extend_from_slice(s.as_bytes());
        return bytes;
    }

    // Normal character
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    s.as_bytes().to_vec()
}

/// Encode cursor key (arrow keys)
fn encode_cursor_key(key: u8, modifiers: Modifiers, app_cursor: bool) -> Vec<u8> {
    let modifier_code = modifiers.to_code();

    if modifier_code > 1 {
        // With modifiers: CSI 1 ; modifier key
        vec![0x1B, b'[', b'1', b';', b'0' + modifier_code, key]
    } else if app_cursor {
        // Application cursor mode: SS3 key
        vec![0x1B, b'O', key]
    } else {
        // Normal mode: CSI key
        vec![0x1B, b'[', key]
    }
}

/// Encode special key (Home, End, Insert, Delete, PageUp, PageDown)
fn encode_special_key(final_byte: u8, code: u8, modifiers: Modifiers, app_mode: bool) -> Vec<u8> {
    let modifier_code = modifiers.to_code();

    if final_byte == b'~' {
        // Keys that use CSI code ~
        if modifier_code > 1 {
            vec![0x1B, b'[', b'0' + code, b';', b'0' + modifier_code, b'~']
        } else {
            vec![0x1B, b'[', b'0' + code, b'~']
        }
    } else {
        // Home/End use CSI H/F or SS3 H/F
        if modifier_code > 1 {
            vec![0x1B, b'[', b'1', b';', b'0' + modifier_code, final_byte]
        } else if app_mode {
            vec![0x1B, b'O', final_byte]
        } else {
            vec![0x1B, b'[', final_byte]
        }
    }
}

/// Encode function key (F1-F12)
fn encode_function_key(n: u8, modifiers: Modifiers) -> Vec<u8> {
    let modifier_code = modifiers.to_code();

    // Function key codes
    let (code, final_byte) = match n {
        1 => (11, b'~'),
        2 => (12, b'~'),
        3 => (13, b'~'),
        4 => (14, b'~'),
        5 => (15, b'~'),
        6 => (17, b'~'),
        7 => (18, b'~'),
        8 => (19, b'~'),
        9 => (20, b'~'),
        10 => (21, b'~'),
        11 => (23, b'~'),
        12 => (24, b'~'),
        _ => return vec![],
    };

    if modifier_code > 1 {
        format!("\x1b[{};{}~", code, modifier_code).into_bytes()
    } else {
        format!("\x1b[{}~", code).into_bytes()
    }
}

/// Keyboard key
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    Tab,
    Backspace,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    F(u8),
}

/// Keyboard modifiers
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl Modifiers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert to xterm modifier code (1 = none, 2 = shift, 3 = alt, etc.)
    pub fn to_code(&self) -> u8 {
        let mut code = 1u8;
        if self.shift {
            code += 1;
        }
        if self.alt {
            code += 2;
        }
        if self.ctrl {
            code += 4;
        }
        code
    }
}

/// Encode mouse event
pub fn encode_mouse(
    event: MouseEvent,
    x: u16,
    y: u16,
    mode: MouseMode,
    encoding: MouseEncoding,
) -> Vec<u8> {
    if mode == MouseMode::None {
        return vec![];
    }

    // Convert to 1-indexed coordinates
    let x = x.saturating_add(1);
    let y = y.saturating_add(1);

    // Calculate button code
    let button = match event {
        MouseEvent::Press(btn) => match btn {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
            MouseButton::WheelUp => 64,
            MouseButton::WheelDown => 65,
        },
        MouseEvent::Release(_) => 3,
        MouseEvent::Move => 32 + 3, // Motion with no button
    };

    match encoding {
        MouseEncoding::X10 => {
            // X10 encoding: CSI M Cb Cx Cy
            // Coordinates are limited to 223 (+ 32 = 255)
            let cb = (button + 32) as u8;
            let cx = (x.min(223) + 32) as u8;
            let cy = (y.min(223) + 32) as u8;
            vec![0x1B, b'[', b'M', cb, cx, cy]
        }
        MouseEncoding::Utf8 => {
            // UTF-8 encoding: like X10 but coordinates can be UTF-8 encoded
            let cb = (button + 32) as u8;
            let mut result = vec![0x1B, b'[', b'M', cb];
            encode_utf8_coord(x + 32, &mut result);
            encode_utf8_coord(y + 32, &mut result);
            result
        }
        MouseEncoding::Sgr => {
            // SGR encoding: CSI < Pb ; Px ; Py M/m
            let final_byte = match event {
                MouseEvent::Release(_) => b'm',
                _ => b'M',
            };
            format!("\x1b[<{};{};{}{}", button, x, y, final_byte as char).into_bytes()
        }
        MouseEncoding::Urxvt => {
            // URXVT encoding: CSI Pb ; Px ; Py M
            format!("\x1b[{};{};{}M", button + 32, x, y).into_bytes()
        }
    }
}

/// Encode a coordinate as UTF-8 for mouse reporting
fn encode_utf8_coord(coord: u16, output: &mut Vec<u8>) {
    if coord < 128 {
        output.push(coord as u8);
    } else if coord < 2048 {
        output.push(0xC0 | ((coord >> 6) as u8));
        output.push(0x80 | ((coord & 0x3F) as u8));
    } else {
        output.push(0xE0 | ((coord >> 12) as u8));
        output.push(0x80 | (((coord >> 6) & 0x3F) as u8));
        output.push(0x80 | ((coord & 0x3F) as u8));
    }
}

/// Mouse event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEvent {
    Press(MouseButton),
    Release(MouseButton),
    Move,
}

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
}

/// Encode focus event
pub fn encode_focus(focused: bool) -> Vec<u8> {
    if focused {
        vec![0x1B, b'[', b'I']
    } else {
        vec![0x1B, b'[', b'O']
    }
}

/// Encode bracketed paste start/end
pub fn bracketed_paste_start() -> Vec<u8> {
    vec![0x1B, b'[', b'2', b'0', b'0', b'~']
}

pub fn bracketed_paste_end() -> Vec<u8> {
    vec![0x1B, b'[', b'2', b'0', b'1', b'~']
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_char() {
        assert_eq!(encode_char('a', Modifiers::default()), vec![b'a']);
        assert_eq!(encode_char('A', Modifiers::default()), vec![b'A']);
    }

    #[test]
    fn test_encode_ctrl_char() {
        let mods = Modifiers { ctrl: true, ..Default::default() };
        assert_eq!(encode_char('c', mods), vec![0x03]); // Ctrl+C
        assert_eq!(encode_char('a', mods), vec![0x01]); // Ctrl+A
        assert_eq!(encode_char('z', mods), vec![0x1A]); // Ctrl+Z
    }

    #[test]
    fn test_encode_alt_char() {
        let mods = Modifiers { alt: true, ..Default::default() };
        assert_eq!(encode_char('a', mods), vec![0x1B, b'a']);
    }

    #[test]
    fn test_encode_arrow_keys() {
        let mods = Modifiers::default();
        assert_eq!(encode_cursor_key(b'A', mods, false), vec![0x1B, b'[', b'A']);
        assert_eq!(encode_cursor_key(b'A', mods, true), vec![0x1B, b'O', b'A']);
    }

    #[test]
    fn test_encode_arrow_with_modifiers() {
        let mods = Modifiers { shift: true, ..Default::default() };
        assert_eq!(
            encode_cursor_key(b'A', mods, false),
            vec![0x1B, b'[', b'1', b';', b'2', b'A']
        );
    }

    #[test]
    fn test_encode_function_keys() {
        let mods = Modifiers::default();
        assert_eq!(encode_function_key(1, mods), b"\x1b[11~".to_vec());
        assert_eq!(encode_function_key(5, mods), b"\x1b[15~".to_vec());
    }

    #[test]
    fn test_encode_mouse_x10() {
        let bytes = encode_mouse(
            MouseEvent::Press(MouseButton::Left),
            0,
            0,
            MouseMode::X10,
            MouseEncoding::X10,
        );
        assert_eq!(bytes, vec![0x1B, b'[', b'M', 32, 33, 33]);
    }

    #[test]
    fn test_encode_mouse_sgr() {
        let bytes = encode_mouse(
            MouseEvent::Press(MouseButton::Left),
            10,
            20,
            MouseMode::VT200,
            MouseEncoding::Sgr,
        );
        assert_eq!(bytes, b"\x1b[<0;11;21M".to_vec());

        let bytes = encode_mouse(
            MouseEvent::Release(MouseButton::Left),
            10,
            20,
            MouseMode::VT200,
            MouseEncoding::Sgr,
        );
        assert_eq!(bytes, b"\x1b[<3;11;21m".to_vec());
    }

    #[test]
    fn test_bracketed_paste() {
        assert_eq!(bracketed_paste_start(), b"\x1b[200~".to_vec());
        assert_eq!(bracketed_paste_end(), b"\x1b[201~".to_vec());
    }

    #[test]
    fn test_focus_events() {
        assert_eq!(encode_focus(true), b"\x1b[I".to_vec());
        assert_eq!(encode_focus(false), b"\x1b[O".to_vec());
    }
}
