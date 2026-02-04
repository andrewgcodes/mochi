//! Input Encoder
//!
//! Encodes keyboard and mouse input into terminal escape sequences.

use crate::core::{Modes, MouseEncoding, MouseMode};
use winit::event::{ModifiersState, VirtualKeyCode};

/// Mouse button for encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
}

/// Mouse event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventType {
    Press,
    Release,
    Motion,
}

/// Encodes keyboard and mouse input into terminal escape sequences
pub struct InputEncoder {
    _private: (),
}

impl InputEncoder {
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Encode a mouse event into terminal escape sequence bytes
    /// Returns None if mouse tracking is disabled
    pub fn encode_mouse(
        &self,
        button: MouseButton,
        event_type: MouseEventType,
        col: usize,
        row: usize,
        modifiers: ModifiersState,
        modes: &Modes,
    ) -> Option<Vec<u8>> {
        // Check if mouse tracking is enabled
        if modes.mouse_tracking == MouseMode::None {
            return None;
        }

        // X10 mode only reports button presses
        if modes.mouse_tracking == MouseMode::X10 && event_type != MouseEventType::Press {
            return None;
        }

        // Normal mode reports press and release
        if modes.mouse_tracking == MouseMode::Normal && event_type == MouseEventType::Motion {
            return None;
        }

        // Calculate button code
        let button_code = match button {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
            MouseButton::WheelUp => 64,
            MouseButton::WheelDown => 65,
        };

        // Add modifier bits
        let mut code = button_code;
        if modifiers.shift() {
            code |= 4;
        }
        if modifiers.alt() {
            code |= 8;
        }
        if modifiers.ctrl() {
            code |= 16;
        }

        // For motion events, add motion flag
        if event_type == MouseEventType::Motion {
            code |= 32;
        }

        // For release events in non-SGR mode, use button 3
        let release_code = if event_type == MouseEventType::Release {
            3 // Release is encoded as button 3 in X10/UTF-8 modes
        } else {
            code
        };

        // Encode based on mouse encoding mode
        match modes.mouse_encoding {
            MouseEncoding::X10 => {
                // X10 encoding: ESC [ M Cb Cx Cy
                // Values are offset by 32 and limited to 223 (255 - 32)
                let cb = (release_code + 32) as u8;
                let cx = ((col + 1).min(223) + 32) as u8;
                let cy = ((row + 1).min(223) + 32) as u8;
                Some(vec![0x1b, b'[', b'M', cb, cx, cy])
            }
            MouseEncoding::Utf8 => {
                // UTF-8 encoding: same as X10 but allows larger coordinates
                let cb = (release_code + 32) as u8;
                let mut result = vec![0x1b, b'[', b'M', cb];
                // Encode col and row as UTF-8 if > 95
                encode_utf8_coord(&mut result, col + 1 + 32);
                encode_utf8_coord(&mut result, row + 1 + 32);
                Some(result)
            }
            MouseEncoding::Sgr => {
                // SGR encoding: ESC [ < Cb ; Cx ; Cy M/m
                // M for press, m for release
                let final_char = if event_type == MouseEventType::Release {
                    b'm'
                } else {
                    b'M'
                };
                Some(format!("\x1b[<{};{};{}{}", code, col + 1, row + 1, final_char as char).into_bytes())
            }
            MouseEncoding::Urxvt => {
                // URXVT encoding: ESC [ Cb ; Cx ; Cy M
                let cb = release_code + 32;
                Some(format!("\x1b[{};{};{}M", cb, col + 1, row + 1).into_bytes())
            }
        }
    }

    /// Encode a virtual key code into terminal escape sequence bytes
    pub fn encode_keycode(
        &self,
        keycode: VirtualKeyCode,
        modifiers: ModifiersState,
        modes: &Modes,
    ) -> Option<Vec<u8>> {
        let app_cursor = modes.application_cursor_keys;

        // Handle special keys
        match keycode {
            VirtualKeyCode::Return => Some(vec![b'\r']),
            VirtualKeyCode::Back => Some(vec![0x7f]),
            VirtualKeyCode::Tab => {
                if modifiers.shift() {
                    Some(b"\x1b[Z".to_vec())
                } else {
                    Some(vec![b'\t'])
                }
            }
            VirtualKeyCode::Escape => Some(vec![0x1b]),

            // Arrow keys
            VirtualKeyCode::Up => {
                if app_cursor {
                    Some(b"\x1bOA".to_vec())
                } else {
                    Some(b"\x1b[A".to_vec())
                }
            }
            VirtualKeyCode::Down => {
                if app_cursor {
                    Some(b"\x1bOB".to_vec())
                } else {
                    Some(b"\x1b[B".to_vec())
                }
            }
            VirtualKeyCode::Right => {
                if app_cursor {
                    Some(b"\x1bOC".to_vec())
                } else {
                    Some(b"\x1b[C".to_vec())
                }
            }
            VirtualKeyCode::Left => {
                if app_cursor {
                    Some(b"\x1bOD".to_vec())
                } else {
                    Some(b"\x1b[D".to_vec())
                }
            }

            // Navigation keys
            VirtualKeyCode::Home => Some(b"\x1b[H".to_vec()),
            VirtualKeyCode::End => Some(b"\x1b[F".to_vec()),
            VirtualKeyCode::PageUp => Some(b"\x1b[5~".to_vec()),
            VirtualKeyCode::PageDown => Some(b"\x1b[6~".to_vec()),
            VirtualKeyCode::Insert => Some(b"\x1b[2~".to_vec()),
            VirtualKeyCode::Delete => Some(b"\x1b[3~".to_vec()),

            // Function keys
            VirtualKeyCode::F1 => Some(b"\x1bOP".to_vec()),
            VirtualKeyCode::F2 => Some(b"\x1bOQ".to_vec()),
            VirtualKeyCode::F3 => Some(b"\x1bOR".to_vec()),
            VirtualKeyCode::F4 => Some(b"\x1bOS".to_vec()),
            VirtualKeyCode::F5 => Some(b"\x1b[15~".to_vec()),
            VirtualKeyCode::F6 => Some(b"\x1b[17~".to_vec()),
            VirtualKeyCode::F7 => Some(b"\x1b[18~".to_vec()),
            VirtualKeyCode::F8 => Some(b"\x1b[19~".to_vec()),
            VirtualKeyCode::F9 => Some(b"\x1b[20~".to_vec()),
            VirtualKeyCode::F10 => Some(b"\x1b[21~".to_vec()),
            VirtualKeyCode::F11 => Some(b"\x1b[23~".to_vec()),
            VirtualKeyCode::F12 => Some(b"\x1b[24~".to_vec()),

            _ => None,
        }
    }
}

impl Default for InputEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Encode a coordinate value for UTF-8 mouse encoding
fn encode_utf8_coord(buf: &mut Vec<u8>, value: usize) {
    if value < 128 {
        buf.push(value as u8);
    } else if value < 2048 {
        buf.push((0xC0 | (value >> 6)) as u8);
        buf.push((0x80 | (value & 0x3F)) as u8);
    } else {
        buf.push((0xE0 | (value >> 12)) as u8);
        buf.push((0x80 | ((value >> 6) & 0x3F)) as u8);
        buf.push((0x80 | (value & 0x3F)) as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{MouseEncoding, MouseMode};

    fn make_modes(mouse_tracking: MouseMode, mouse_encoding: MouseEncoding) -> Modes {
        let mut modes = Modes::new();
        modes.mouse_tracking = mouse_tracking;
        modes.mouse_encoding = mouse_encoding;
        modes
    }

    #[test]
    fn test_mouse_disabled() {
        let encoder = InputEncoder::new();
        let modes = make_modes(MouseMode::None, MouseEncoding::X10);
        let modifiers = ModifiersState::empty();

        let result = encoder.encode_mouse(
            MouseButton::Left,
            MouseEventType::Press,
            10,
            5,
            modifiers,
            &modes,
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_mouse_x10_encoding() {
        let encoder = InputEncoder::new();
        let modes = make_modes(MouseMode::Normal, MouseEncoding::X10);
        let modifiers = ModifiersState::empty();

        let result = encoder.encode_mouse(
            MouseButton::Left,
            MouseEventType::Press,
            10,
            5,
            modifiers,
            &modes,
        );
        assert!(result.is_some());
        let bytes = result.unwrap();
        assert_eq!(bytes[0], 0x1b);
        assert_eq!(bytes[1], b'[');
        assert_eq!(bytes[2], b'M');
        // Button 0 + 32 = 32
        assert_eq!(bytes[3], 32);
        // Col 11 + 32 = 43
        assert_eq!(bytes[4], 43);
        // Row 6 + 32 = 38
        assert_eq!(bytes[5], 38);
    }

    #[test]
    fn test_mouse_sgr_encoding() {
        let encoder = InputEncoder::new();
        let modes = make_modes(MouseMode::Normal, MouseEncoding::Sgr);
        let modifiers = ModifiersState::empty();

        let result = encoder.encode_mouse(
            MouseButton::Left,
            MouseEventType::Press,
            10,
            5,
            modifiers,
            &modes,
        );
        assert!(result.is_some());
        let bytes = result.unwrap();
        assert_eq!(bytes, b"\x1b[<0;11;6M");

        // Test release
        let result = encoder.encode_mouse(
            MouseButton::Left,
            MouseEventType::Release,
            10,
            5,
            modifiers,
            &modes,
        );
        assert!(result.is_some());
        let bytes = result.unwrap();
        assert_eq!(bytes, b"\x1b[<0;11;6m");
    }
}
