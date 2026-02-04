//! Input Encoding Module
//!
//! Handles encoding of keyboard and mouse input into terminal escape sequences.
//! This module translates GUI input events into the byte sequences expected by
//! terminal applications.
//!
//! # Keyboard Encoding
//!
//! Different keys produce different sequences depending on:
//! - Application cursor mode (DECCKM)
//! - Application keypad mode (DECKPAM/DECKPNM)
//! - Modifier keys (Shift, Ctrl, Alt)
//!
//! # Mouse Encoding
//!
//! Mouse events are encoded according to the active mouse mode:
//! - X10: Button press only
//! - Normal (VT200): Button press and release
//! - SGR: Extended encoding for large terminals

use crate::core::{MouseEncoding, MouseMode};

/// Keyboard modifiers
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl Modifiers {
    /// Get the modifier parameter for CSI sequences (1 + bitmask)
    /// Shift=1, Alt=2, Ctrl=4
    pub fn as_csi_param(&self) -> u8 {
        let mut param = 1;
        if self.shift {
            param += 1;
        }
        if self.alt {
            param += 2;
        }
        if self.ctrl {
            param += 4;
        }
        param
    }

    /// Check if any modifier is pressed
    pub fn any(&self) -> bool {
        self.shift || self.ctrl || self.alt
    }
}

/// Special keys that produce escape sequences
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    // Cursor keys
    Up,
    Down,
    Left,
    Right,

    // Navigation
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Editing
    Backspace,
    Tab,
    Enter,
    Escape,

    // Keypad (when not in application mode, these are same as regular keys)
    KeypadEnter,
    KeypadPlus,
    KeypadMinus,
    KeypadMultiply,
    KeypadDivide,
    KeypadDecimal,
    Keypad0,
    Keypad1,
    Keypad2,
    Keypad3,
    Keypad4,
    Keypad5,
    Keypad6,
    Keypad7,
    Keypad8,
    Keypad9,
}

/// Encode a special key press into terminal escape sequence
pub fn encode_key(
    key: Key,
    modifiers: Modifiers,
    application_cursor: bool,
    application_keypad: bool,
) -> Vec<u8> {
    match key {
        // Cursor keys
        Key::Up => encode_cursor_key(b'A', modifiers, application_cursor),
        Key::Down => encode_cursor_key(b'B', modifiers, application_cursor),
        Key::Right => encode_cursor_key(b'C', modifiers, application_cursor),
        Key::Left => encode_cursor_key(b'D', modifiers, application_cursor),

        // Navigation keys
        Key::Home => encode_special_key(1, b'~', modifiers),
        Key::Insert => encode_special_key(2, b'~', modifiers),
        Key::Delete => encode_special_key(3, b'~', modifiers),
        Key::End => encode_special_key(4, b'~', modifiers),
        Key::PageUp => encode_special_key(5, b'~', modifiers),
        Key::PageDown => encode_special_key(6, b'~', modifiers),

        // Function keys
        Key::F1 => encode_function_key(1, modifiers),
        Key::F2 => encode_function_key(2, modifiers),
        Key::F3 => encode_function_key(3, modifiers),
        Key::F4 => encode_function_key(4, modifiers),
        Key::F5 => encode_special_key(15, b'~', modifiers),
        Key::F6 => encode_special_key(17, b'~', modifiers),
        Key::F7 => encode_special_key(18, b'~', modifiers),
        Key::F8 => encode_special_key(19, b'~', modifiers),
        Key::F9 => encode_special_key(20, b'~', modifiers),
        Key::F10 => encode_special_key(21, b'~', modifiers),
        Key::F11 => encode_special_key(23, b'~', modifiers),
        Key::F12 => encode_special_key(24, b'~', modifiers),

        // Editing keys
        Key::Backspace => {
            if modifiers.ctrl {
                vec![0x08] // Ctrl+Backspace = BS
            } else if modifiers.alt {
                vec![0x1b, 0x7f] // Alt+Backspace = ESC DEL
            } else {
                vec![0x7f] // DEL
            }
        }
        Key::Tab => {
            if modifiers.shift {
                vec![0x1b, b'[', b'Z'] // Shift+Tab = CSI Z (backtab)
            } else {
                vec![0x09] // HT
            }
        }
        Key::Enter => {
            if modifiers.alt {
                vec![0x1b, 0x0d] // Alt+Enter
            } else {
                vec![0x0d] // CR
            }
        }
        Key::Escape => vec![0x1b],

        // Keypad keys
        Key::KeypadEnter => {
            if application_keypad {
                vec![0x1b, b'O', b'M']
            } else {
                vec![0x0d]
            }
        }
        Key::KeypadPlus => {
            if application_keypad {
                vec![0x1b, b'O', b'k']
            } else {
                vec![b'+']
            }
        }
        Key::KeypadMinus => {
            if application_keypad {
                vec![0x1b, b'O', b'm']
            } else {
                vec![b'-']
            }
        }
        Key::KeypadMultiply => {
            if application_keypad {
                vec![0x1b, b'O', b'j']
            } else {
                vec![b'*']
            }
        }
        Key::KeypadDivide => {
            if application_keypad {
                vec![0x1b, b'O', b'o']
            } else {
                vec![b'/']
            }
        }
        Key::KeypadDecimal => {
            if application_keypad {
                vec![0x1b, b'O', b'n']
            } else {
                vec![b'.']
            }
        }
        Key::Keypad0 => encode_keypad_digit(b'p', b'0', application_keypad),
        Key::Keypad1 => encode_keypad_digit(b'q', b'1', application_keypad),
        Key::Keypad2 => encode_keypad_digit(b'r', b'2', application_keypad),
        Key::Keypad3 => encode_keypad_digit(b's', b'3', application_keypad),
        Key::Keypad4 => encode_keypad_digit(b't', b'4', application_keypad),
        Key::Keypad5 => encode_keypad_digit(b'u', b'5', application_keypad),
        Key::Keypad6 => encode_keypad_digit(b'v', b'6', application_keypad),
        Key::Keypad7 => encode_keypad_digit(b'w', b'7', application_keypad),
        Key::Keypad8 => encode_keypad_digit(b'x', b'8', application_keypad),
        Key::Keypad9 => encode_keypad_digit(b'y', b'9', application_keypad),
    }
}

/// Encode a cursor key (arrow keys)
fn encode_cursor_key(code: u8, modifiers: Modifiers, application_mode: bool) -> Vec<u8> {
    if modifiers.any() {
        // With modifiers: CSI 1 ; modifier code
        let param = modifiers.as_csi_param();
        format!("\x1b[1;{}{}", param, code as char).into_bytes()
    } else if application_mode {
        // Application mode: SS3 code
        vec![0x1b, b'O', code]
    } else {
        // Normal mode: CSI code
        vec![0x1b, b'[', code]
    }
}

/// Encode a special key (Home, End, PgUp, PgDn, Insert, Delete, F5-F12)
fn encode_special_key(number: u8, final_byte: u8, modifiers: Modifiers) -> Vec<u8> {
    if modifiers.any() {
        let param = modifiers.as_csi_param();
        format!("\x1b[{};{}{}", number, param, final_byte as char).into_bytes()
    } else {
        format!("\x1b[{}{}", number, final_byte as char).into_bytes()
    }
}

/// Encode function keys F1-F4 (use SS3 in some modes)
fn encode_function_key(number: u8, modifiers: Modifiers) -> Vec<u8> {
    let code = match number {
        1 => b'P',
        2 => b'Q',
        3 => b'R',
        4 => b'S',
        _ => return vec![],
    };

    if modifiers.any() {
        let param = modifiers.as_csi_param();
        format!("\x1b[1;{}{}", param, code as char).into_bytes()
    } else {
        vec![0x1b, b'O', code]
    }
}

/// Encode keypad digit
fn encode_keypad_digit(app_code: u8, normal_code: u8, application_mode: bool) -> Vec<u8> {
    if application_mode {
        vec![0x1b, b'O', app_code]
    } else {
        vec![normal_code]
    }
}

/// Encode a character with modifiers
pub fn encode_char(c: char, modifiers: Modifiers) -> Vec<u8> {
    if modifiers.ctrl && c.is_ascii_alphabetic() {
        // Ctrl+letter produces control character
        let ctrl_char = (c.to_ascii_uppercase() as u8) - b'@';
        if modifiers.alt {
            vec![0x1b, ctrl_char]
        } else {
            vec![ctrl_char]
        }
    } else if modifiers.alt {
        // Alt+char sends ESC prefix
        let mut bytes = vec![0x1b];
        let mut buf = [0u8; 4];
        bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        bytes
    } else {
        // Normal character
        let mut buf = [0u8; 4];
        c.encode_utf8(&mut buf).as_bytes().to_vec()
    }
}

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,
    Button4,
    Button5,
}

/// Mouse event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventType {
    Press,
    Release,
    Move,
}

/// Encode a mouse event
pub fn encode_mouse(
    button: MouseButton,
    event_type: MouseEventType,
    col: u16,
    row: u16,
    modifiers: Modifiers,
    mode: MouseMode,
    encoding: MouseEncoding,
) -> Option<Vec<u8>> {
    // Check if this event should be reported based on mode
    match mode {
        MouseMode::None => return None,
        MouseMode::X10 => {
            // X10 only reports button press
            if event_type != MouseEventType::Press {
                return None;
            }
            // X10 doesn't report wheel events
            if matches!(
                button,
                MouseButton::WheelUp
                    | MouseButton::WheelDown
                    | MouseButton::WheelLeft
                    | MouseButton::WheelRight
            ) {
                return None;
            }
        }
        MouseMode::Normal | MouseMode::Highlight => {
            // Normal reports press and release
            if event_type == MouseEventType::Move {
                return None;
            }
        }
        MouseMode::ButtonEvent => {
            // Button event reports motion only while button pressed
            // For simplicity, we report all motion here
        }
        MouseMode::AnyEvent => {
            // Any event reports all motion
        }
    }

    // Calculate button code
    let mut button_code: u8 = match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        MouseButton::WheelUp => 64,
        MouseButton::WheelDown => 65,
        MouseButton::WheelLeft => 66,
        MouseButton::WheelRight => 67,
        MouseButton::Button4 => 128,
        MouseButton::Button5 => 129,
    };

    // Add modifier bits
    if modifiers.shift {
        button_code |= 4;
    }
    if modifiers.alt {
        button_code |= 8;
    }
    if modifiers.ctrl {
        button_code |= 16;
    }

    // Add motion bit for move events
    if event_type == MouseEventType::Move {
        button_code |= 32;
    }

    // Encode based on encoding mode
    match encoding {
        MouseEncoding::X10 => {
            // X10 encoding: CSI M Cb Cx Cy
            // Coordinates are 1-based and offset by 32
            let cx = (col + 1).min(223) as u8 + 32;
            let cy = (row + 1).min(223) as u8 + 32;
            let cb = if event_type == MouseEventType::Release {
                3 + 32 // Release is button 3 in X10
            } else {
                button_code + 32
            };
            Some(vec![0x1b, b'[', b'M', cb, cx, cy])
        }
        MouseEncoding::Utf8 => {
            // UTF-8 encoding: like X10 but coordinates can be > 223
            let cx = col + 1 + 32;
            let cy = row + 1 + 32;
            let cb = if event_type == MouseEventType::Release {
                3 + 32
            } else {
                button_code as u16 + 32
            };

            let mut result = vec![0x1b, b'[', b'M'];
            encode_utf8_coord(&mut result, cb);
            encode_utf8_coord(&mut result, cx);
            encode_utf8_coord(&mut result, cy);
            Some(result)
        }
        MouseEncoding::Sgr => {
            // SGR encoding: CSI < Pb ; Px ; Py M/m
            let final_char = if event_type == MouseEventType::Release {
                b'm'
            } else {
                b'M'
            };
            Some(
                format!(
                    "\x1b[<{};{};{}{}",
                    button_code,
                    col + 1,
                    row + 1,
                    final_char as char
                )
                .into_bytes(),
            )
        }
        MouseEncoding::Urxvt => {
            // URXVT encoding: CSI Pb ; Px ; Py M
            let cb = if event_type == MouseEventType::Release {
                3
            } else {
                button_code
            };
            Some(format!("\x1b[{};{};{}M", cb + 32, col + 1, row + 1).into_bytes())
        }
    }
}

/// Encode a coordinate as UTF-8 for mouse reporting
fn encode_utf8_coord(result: &mut Vec<u8>, coord: u16) {
    if coord < 128 {
        result.push(coord as u8);
    } else if coord < 2048 {
        result.push(0xC0 | ((coord >> 6) as u8));
        result.push(0x80 | ((coord & 0x3F) as u8));
    } else {
        result.push(0xE0 | ((coord >> 12) as u8));
        result.push(0x80 | (((coord >> 6) & 0x3F) as u8));
        result.push(0x80 | ((coord & 0x3F) as u8));
    }
}

/// Encode focus in/out events
pub fn encode_focus(focused: bool) -> Vec<u8> {
    if focused {
        vec![0x1b, b'[', b'I'] // CSI I = focus in
    } else {
        vec![0x1b, b'[', b'O'] // CSI O = focus out
    }
}

/// Encode bracketed paste start/end
pub fn encode_bracketed_paste(start: bool) -> Vec<u8> {
    if start {
        vec![0x1b, b'[', b'2', b'0', b'0', b'~'] // CSI 200 ~
    } else {
        vec![0x1b, b'[', b'2', b'0', b'1', b'~'] // CSI 201 ~
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_keys_normal() {
        let mods = Modifiers::default();
        assert_eq!(encode_key(Key::Up, mods, false, false), b"\x1b[A");
        assert_eq!(encode_key(Key::Down, mods, false, false), b"\x1b[B");
        assert_eq!(encode_key(Key::Right, mods, false, false), b"\x1b[C");
        assert_eq!(encode_key(Key::Left, mods, false, false), b"\x1b[D");
    }

    #[test]
    fn test_cursor_keys_application() {
        let mods = Modifiers::default();
        assert_eq!(encode_key(Key::Up, mods, true, false), b"\x1bOA");
        assert_eq!(encode_key(Key::Down, mods, true, false), b"\x1bOB");
    }

    #[test]
    fn test_cursor_keys_with_modifiers() {
        let mods = Modifiers {
            shift: true,
            ctrl: false,
            alt: false,
        };
        assert_eq!(encode_key(Key::Up, mods, false, false), b"\x1b[1;2A");

        let mods = Modifiers {
            shift: false,
            ctrl: true,
            alt: false,
        };
        assert_eq!(encode_key(Key::Up, mods, false, false), b"\x1b[1;5A");

        let mods = Modifiers {
            shift: true,
            ctrl: true,
            alt: false,
        };
        assert_eq!(encode_key(Key::Up, mods, false, false), b"\x1b[1;6A");
    }

    #[test]
    fn test_function_keys() {
        let mods = Modifiers::default();
        assert_eq!(encode_key(Key::F1, mods, false, false), b"\x1bOP");
        assert_eq!(encode_key(Key::F2, mods, false, false), b"\x1bOQ");
        assert_eq!(encode_key(Key::F5, mods, false, false), b"\x1b[15~");
        assert_eq!(encode_key(Key::F12, mods, false, false), b"\x1b[24~");
    }

    #[test]
    fn test_navigation_keys() {
        let mods = Modifiers::default();
        assert_eq!(encode_key(Key::Home, mods, false, false), b"\x1b[1~");
        assert_eq!(encode_key(Key::End, mods, false, false), b"\x1b[4~");
        assert_eq!(encode_key(Key::PageUp, mods, false, false), b"\x1b[5~");
        assert_eq!(encode_key(Key::PageDown, mods, false, false), b"\x1b[6~");
        assert_eq!(encode_key(Key::Insert, mods, false, false), b"\x1b[2~");
        assert_eq!(encode_key(Key::Delete, mods, false, false), b"\x1b[3~");
    }

    #[test]
    fn test_editing_keys() {
        let mods = Modifiers::default();
        assert_eq!(encode_key(Key::Backspace, mods, false, false), b"\x7f");
        assert_eq!(encode_key(Key::Tab, mods, false, false), b"\x09");
        assert_eq!(encode_key(Key::Enter, mods, false, false), b"\x0d");
        assert_eq!(encode_key(Key::Escape, mods, false, false), b"\x1b");

        let shift = Modifiers {
            shift: true,
            ctrl: false,
            alt: false,
        };
        assert_eq!(encode_key(Key::Tab, shift, false, false), b"\x1b[Z");
    }

    #[test]
    fn test_encode_char() {
        let mods = Modifiers::default();
        assert_eq!(encode_char('a', mods), b"a");
        assert_eq!(encode_char('Z', mods), b"Z");

        let ctrl = Modifiers {
            shift: false,
            ctrl: true,
            alt: false,
        };
        assert_eq!(encode_char('c', ctrl), vec![0x03]); // Ctrl+C
        assert_eq!(encode_char('a', ctrl), vec![0x01]); // Ctrl+A

        let alt = Modifiers {
            shift: false,
            ctrl: false,
            alt: true,
        };
        assert_eq!(encode_char('x', alt), b"\x1bx");
    }

    #[test]
    fn test_mouse_sgr_encoding() {
        let mods = Modifiers::default();
        let result = encode_mouse(
            MouseButton::Left,
            MouseEventType::Press,
            10,
            5,
            mods,
            MouseMode::Normal,
            MouseEncoding::Sgr,
        );
        assert_eq!(result, Some(b"\x1b[<0;11;6M".to_vec()));

        let result = encode_mouse(
            MouseButton::Left,
            MouseEventType::Release,
            10,
            5,
            mods,
            MouseMode::Normal,
            MouseEncoding::Sgr,
        );
        assert_eq!(result, Some(b"\x1b[<0;11;6m".to_vec()));
    }

    #[test]
    fn test_mouse_x10_encoding() {
        let mods = Modifiers::default();
        let result = encode_mouse(
            MouseButton::Left,
            MouseEventType::Press,
            0,
            0,
            mods,
            MouseMode::X10,
            MouseEncoding::X10,
        );
        // Button 0 + 32 = 32, col 1 + 32 = 33, row 1 + 32 = 33
        assert_eq!(result, Some(vec![0x1b, b'[', b'M', 32, 33, 33]));
    }

    #[test]
    fn test_mouse_mode_filtering() {
        let mods = Modifiers::default();

        // X10 mode doesn't report release
        let result = encode_mouse(
            MouseButton::Left,
            MouseEventType::Release,
            0,
            0,
            mods,
            MouseMode::X10,
            MouseEncoding::X10,
        );
        assert_eq!(result, None);

        // None mode doesn't report anything
        let result = encode_mouse(
            MouseButton::Left,
            MouseEventType::Press,
            0,
            0,
            mods,
            MouseMode::None,
            MouseEncoding::X10,
        );
        assert_eq!(result, None);
    }

    #[test]
    fn test_bracketed_paste() {
        assert_eq!(encode_bracketed_paste(true), b"\x1b[200~");
        assert_eq!(encode_bracketed_paste(false), b"\x1b[201~");
    }

    #[test]
    fn test_focus_events() {
        assert_eq!(encode_focus(true), b"\x1b[I");
        assert_eq!(encode_focus(false), b"\x1b[O");
    }
}
