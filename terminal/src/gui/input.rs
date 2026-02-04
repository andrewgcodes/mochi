//! Input Encoder
//!
//! Encodes keyboard and mouse input into terminal escape sequences.

use crate::core::Modes;
use winit::event::{ModifiersState, VirtualKeyCode};

/// Encodes keyboard input into terminal escape sequences
pub struct InputEncoder {
    _private: (),
}

impl InputEncoder {
    pub fn new() -> Self {
        Self { _private: () }
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
