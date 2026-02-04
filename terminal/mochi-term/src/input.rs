//! Keyboard input encoding for terminal emulation.
//!
//! Converts keyboard events into terminal escape sequences
//! that can be sent to the PTY.
//!
//! Note: This module is not yet used in the main application but will be
//! integrated when the GUI frontend is implemented.

#![allow(dead_code)]

use mochi_core::screen::MouseMode;

pub struct InputEncoder {
    application_cursor_keys: bool,
    application_keypad: bool,
}

impl InputEncoder {
    pub fn new() -> Self {
        InputEncoder {
            application_cursor_keys: false,
            application_keypad: false,
        }
    }

    pub fn set_application_cursor_keys(&mut self, enabled: bool) {
        self.application_cursor_keys = enabled;
    }

    pub fn set_application_keypad(&mut self, enabled: bool) {
        self.application_keypad = enabled;
    }

    pub fn encode_key(&self, key: Key, modifiers: Modifiers) -> Vec<u8> {
        match key {
            Key::Char(c) => self.encode_char(c, modifiers),
            Key::Enter => vec![b'\r'],
            Key::Tab => {
                if modifiers.shift {
                    b"\x1b[Z".to_vec()
                } else {
                    vec![b'\t']
                }
            }
            Key::Backspace => {
                if modifiers.alt {
                    b"\x1b\x7f".to_vec()
                } else {
                    vec![0x7f]
                }
            }
            Key::Escape => vec![0x1b],
            Key::Up => self.encode_cursor_key(b'A', modifiers),
            Key::Down => self.encode_cursor_key(b'B', modifiers),
            Key::Right => self.encode_cursor_key(b'C', modifiers),
            Key::Left => self.encode_cursor_key(b'D', modifiers),
            Key::Home => self.encode_special_key(1, modifiers),
            Key::End => self.encode_special_key(4, modifiers),
            Key::PageUp => self.encode_special_key(5, modifiers),
            Key::PageDown => self.encode_special_key(6, modifiers),
            Key::Insert => self.encode_special_key(2, modifiers),
            Key::Delete => self.encode_special_key(3, modifiers),
            Key::F(n) => self.encode_function_key(n, modifiers),
        }
    }

    fn encode_char(&self, c: char, modifiers: Modifiers) -> Vec<u8> {
        if modifiers.ctrl {
            if c.is_ascii_lowercase() {
                return vec![(c as u8) - b'a' + 1];
            }
            if c.is_ascii_uppercase() {
                return vec![(c as u8) - b'A' + 1];
            }
            match c {
                '@' => return vec![0],
                '[' => return vec![0x1b],
                '\\' => return vec![0x1c],
                ']' => return vec![0x1d],
                '^' => return vec![0x1e],
                '_' => return vec![0x1f],
                '?' => return vec![0x7f],
                _ => {}
            }
        }

        if modifiers.alt {
            let mut result = vec![0x1b];
            let mut buf = [0u8; 4];
            let encoded = c.encode_utf8(&mut buf);
            result.extend_from_slice(encoded.as_bytes());
            return result;
        }

        let mut buf = [0u8; 4];
        let encoded = c.encode_utf8(&mut buf);
        encoded.as_bytes().to_vec()
    }

    fn encode_cursor_key(&self, key: u8, modifiers: Modifiers) -> Vec<u8> {
        let modifier_code = self.modifier_code(modifiers);

        if modifier_code > 1 {
            format!("\x1b[1;{}{}", modifier_code, key as char).into_bytes()
        } else if self.application_cursor_keys {
            vec![0x1b, b'O', key]
        } else {
            vec![0x1b, b'[', key]
        }
    }

    fn encode_special_key(&self, code: u8, modifiers: Modifiers) -> Vec<u8> {
        let modifier_code = self.modifier_code(modifiers);

        if modifier_code > 1 {
            format!("\x1b[{};{}~", code, modifier_code).into_bytes()
        } else {
            format!("\x1b[{}~", code).into_bytes()
        }
    }

    fn encode_function_key(&self, n: u8, modifiers: Modifiers) -> Vec<u8> {
        let code = match n {
            1 => 11,
            2 => 12,
            3 => 13,
            4 => 14,
            5 => 15,
            6 => 17,
            7 => 18,
            8 => 19,
            9 => 20,
            10 => 21,
            11 => 23,
            12 => 24,
            _ => return vec![],
        };

        let modifier_code = self.modifier_code(modifiers);

        if modifier_code > 1 {
            format!("\x1b[{};{}~", code, modifier_code).into_bytes()
        } else {
            format!("\x1b[{}~", code).into_bytes()
        }
    }

    fn modifier_code(&self, modifiers: Modifiers) -> u8 {
        let mut code = 1u8;
        if modifiers.shift {
            code += 1;
        }
        if modifiers.alt {
            code += 2;
        }
        if modifiers.ctrl {
            code += 4;
        }
        code
    }

    pub fn encode_mouse(
        &self,
        event: MouseEvent,
        x: u16,
        y: u16,
        mode: MouseMode,
        sgr_mode: bool,
    ) -> Vec<u8> {
        if matches!(mode, MouseMode::None) {
            return vec![];
        }

        let button = match event {
            MouseEvent::Press(btn) => match btn {
                MouseButton::Left => 0,
                MouseButton::Middle => 1,
                MouseButton::Right => 2,
            },
            MouseEvent::Release(_) => 3,
            MouseEvent::ScrollUp => 64,
            MouseEvent::ScrollDown => 65,
            MouseEvent::Move => 32,
        };

        if sgr_mode {
            let suffix = match event {
                MouseEvent::Release(_) => 'm',
                _ => 'M',
            };
            format!("\x1b[<{};{};{}{}", button, x + 1, y + 1, suffix).into_bytes()
        } else {
            let cb = 32 + button;
            let cx = 32 + (x + 1).min(223) as u8;
            let cy = 32 + (y + 1).min(223) as u8;
            vec![0x1b, b'[', b'M', cb, cx, cy]
        }
    }

    pub fn encode_paste(&self, text: &str, bracketed: bool) -> Vec<u8> {
        if bracketed {
            let mut result = b"\x1b[200~".to_vec();
            result.extend_from_slice(text.as_bytes());
            result.extend_from_slice(b"\x1b[201~");
            result
        } else {
            text.as_bytes().to_vec()
        }
    }

    pub fn encode_focus(&self, focused: bool) -> Vec<u8> {
        if focused {
            b"\x1b[I".to_vec()
        } else {
            b"\x1b[O".to_vec()
        }
    }
}

impl Default for InputEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub enum MouseEvent {
    Press(MouseButton),
    Release(MouseButton),
    ScrollUp,
    ScrollDown,
    Move,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_char() {
        let encoder = InputEncoder::new();
        assert_eq!(
            encoder.encode_key(Key::Char('a'), Modifiers::default()),
            b"a"
        );
        assert_eq!(
            encoder.encode_key(Key::Char('A'), Modifiers::default()),
            b"A"
        );
    }

    #[test]
    fn test_encode_ctrl_char() {
        let encoder = InputEncoder::new();
        let mods = Modifiers {
            ctrl: true,
            ..Default::default()
        };
        assert_eq!(encoder.encode_key(Key::Char('c'), mods), vec![3]);
        assert_eq!(encoder.encode_key(Key::Char('a'), mods), vec![1]);
        assert_eq!(encoder.encode_key(Key::Char('z'), mods), vec![26]);
    }

    #[test]
    fn test_encode_alt_char() {
        let encoder = InputEncoder::new();
        let mods = Modifiers {
            alt: true,
            ..Default::default()
        };
        assert_eq!(encoder.encode_key(Key::Char('a'), mods), b"\x1ba");
    }

    #[test]
    fn test_encode_cursor_keys() {
        let encoder = InputEncoder::new();
        assert_eq!(encoder.encode_key(Key::Up, Modifiers::default()), b"\x1b[A");
        assert_eq!(
            encoder.encode_key(Key::Down, Modifiers::default()),
            b"\x1b[B"
        );
        assert_eq!(
            encoder.encode_key(Key::Right, Modifiers::default()),
            b"\x1b[C"
        );
        assert_eq!(
            encoder.encode_key(Key::Left, Modifiers::default()),
            b"\x1b[D"
        );
    }

    #[test]
    fn test_encode_cursor_keys_with_modifiers() {
        let encoder = InputEncoder::new();
        let mods = Modifiers {
            shift: true,
            ..Default::default()
        };
        assert_eq!(encoder.encode_key(Key::Up, mods), b"\x1b[1;2A");
    }

    #[test]
    fn test_encode_function_keys() {
        let encoder = InputEncoder::new();
        assert_eq!(
            encoder.encode_key(Key::F(1), Modifiers::default()),
            b"\x1b[11~"
        );
        assert_eq!(
            encoder.encode_key(Key::F(5), Modifiers::default()),
            b"\x1b[15~"
        );
        assert_eq!(
            encoder.encode_key(Key::F(12), Modifiers::default()),
            b"\x1b[24~"
        );
    }

    #[test]
    fn test_encode_special_keys() {
        let encoder = InputEncoder::new();
        assert_eq!(
            encoder.encode_key(Key::Home, Modifiers::default()),
            b"\x1b[1~"
        );
        assert_eq!(
            encoder.encode_key(Key::End, Modifiers::default()),
            b"\x1b[4~"
        );
        assert_eq!(
            encoder.encode_key(Key::PageUp, Modifiers::default()),
            b"\x1b[5~"
        );
        assert_eq!(
            encoder.encode_key(Key::PageDown, Modifiers::default()),
            b"\x1b[6~"
        );
    }

    #[test]
    fn test_encode_mouse_sgr() {
        let encoder = InputEncoder::new();
        let result = encoder.encode_mouse(
            MouseEvent::Press(MouseButton::Left),
            10,
            20,
            MouseMode::VT200,
            true,
        );
        assert_eq!(result, b"\x1b[<0;11;21M");
    }

    #[test]
    fn test_encode_bracketed_paste() {
        let encoder = InputEncoder::new();
        let result = encoder.encode_paste("hello", true);
        assert_eq!(result, b"\x1b[200~hello\x1b[201~");
    }

    #[test]
    fn test_encode_focus() {
        let encoder = InputEncoder::new();
        assert_eq!(encoder.encode_focus(true), b"\x1b[I");
        assert_eq!(encoder.encode_focus(false), b"\x1b[O");
    }
}
