//! Keyboard and mouse input handling
//!
//! Converts GUI input events to terminal escape sequences.

use winit::event::MouseButton;
use winit::keyboard::{Key, ModifiersState, NamedKey};

/// Encode a key press to terminal escape sequence
pub fn encode_key(
    key: &Key,
    modifiers: ModifiersState,
    application_cursor_keys: bool,
    application_keypad: bool,
) -> Option<Vec<u8>> {
    let ctrl = modifiers.control_key();
    let alt = modifiers.alt_key();
    let _shift = modifiers.shift_key();

    match key {
        Key::Character(c) => {
            let c = c.chars().next()?;

            // On macOS, Ctrl+letter might produce the control character directly
            // (e.g., Ctrl+C produces '\x03' instead of 'c' with Ctrl modifier)
            // Handle control characters (0x01-0x1A) directly
            if c as u32 >= 1 && c as u32 <= 26 {
                return Some(vec![c as u8]);
            }

            if ctrl {
                // Ctrl+letter produces control characters
                if c.is_ascii_alphabetic() {
                    let code = (c.to_ascii_uppercase() as u8) - b'A' + 1;
                    return Some(vec![code]);
                }
                // Ctrl+special characters
                match c {
                    '@' => return Some(vec![0]),
                    '[' => return Some(vec![27]),
                    '\\' => return Some(vec![28]),
                    ']' => return Some(vec![29]),
                    '^' => return Some(vec![30]),
                    '_' => return Some(vec![31]),
                    '?' => return Some(vec![127]),
                    _ => {}
                }
            }

            if alt {
                // Alt+key sends ESC followed by the key
                let mut result = vec![0x1b];
                result.extend(c.to_string().as_bytes());
                return Some(result);
            }

            // Regular character
            Some(c.to_string().into_bytes())
        }
        Key::Named(named) => encode_named_key(
            named,
            modifiers,
            application_cursor_keys,
            application_keypad,
        ),
        Key::Unidentified(_) | Key::Dead(_) => None,
    }
}

/// Encode a named key to terminal escape sequence
fn encode_named_key(
    key: &NamedKey,
    modifiers: ModifiersState,
    application_cursor_keys: bool,
    _application_keypad: bool,
) -> Option<Vec<u8>> {
    let ctrl = modifiers.control_key();
    let alt = modifiers.alt_key();
    let shift = modifiers.shift_key();

    // Calculate modifier code for CSI sequences
    let modifier_code = if ctrl || alt || shift {
        let mut code = 1;
        if shift {
            code += 1;
        }
        if alt {
            code += 2;
        }
        if ctrl {
            code += 4;
        }
        Some(code)
    } else {
        None
    };

    match key {
        NamedKey::Enter => Some(vec![0x0d]),
        NamedKey::Tab => {
            if shift {
                Some(b"\x1b[Z".to_vec()) // Shift+Tab = backtab
            } else {
                Some(vec![0x09])
            }
        }
        NamedKey::Backspace => {
            if ctrl {
                Some(vec![0x08]) // Ctrl+Backspace
            } else {
                Some(vec![0x7f]) // DEL
            }
        }
        NamedKey::Escape => Some(vec![0x1b]),
        NamedKey::Space => {
            if ctrl {
                Some(vec![0x00]) // Ctrl+Space = NUL
            } else {
                Some(vec![0x20])
            }
        }

        // Arrow keys
        NamedKey::ArrowUp => Some(encode_cursor_key(
            b'A',
            modifier_code,
            application_cursor_keys,
        )),
        NamedKey::ArrowDown => Some(encode_cursor_key(
            b'B',
            modifier_code,
            application_cursor_keys,
        )),
        NamedKey::ArrowRight => Some(encode_cursor_key(
            b'C',
            modifier_code,
            application_cursor_keys,
        )),
        NamedKey::ArrowLeft => Some(encode_cursor_key(
            b'D',
            modifier_code,
            application_cursor_keys,
        )),

        // Navigation keys
        NamedKey::Home => Some(encode_special_key(b'H', modifier_code)),
        NamedKey::End => Some(encode_special_key(b'F', modifier_code)),
        NamedKey::PageUp => Some(encode_tilde_key(5, modifier_code)),
        NamedKey::PageDown => Some(encode_tilde_key(6, modifier_code)),
        NamedKey::Insert => Some(encode_tilde_key(2, modifier_code)),
        NamedKey::Delete => Some(encode_tilde_key(3, modifier_code)),

        // Function keys
        NamedKey::F1 => Some(encode_function_key(1, modifier_code)),
        NamedKey::F2 => Some(encode_function_key(2, modifier_code)),
        NamedKey::F3 => Some(encode_function_key(3, modifier_code)),
        NamedKey::F4 => Some(encode_function_key(4, modifier_code)),
        NamedKey::F5 => Some(encode_function_key(5, modifier_code)),
        NamedKey::F6 => Some(encode_function_key(6, modifier_code)),
        NamedKey::F7 => Some(encode_function_key(7, modifier_code)),
        NamedKey::F8 => Some(encode_function_key(8, modifier_code)),
        NamedKey::F9 => Some(encode_function_key(9, modifier_code)),
        NamedKey::F10 => Some(encode_function_key(10, modifier_code)),
        NamedKey::F11 => Some(encode_function_key(11, modifier_code)),
        NamedKey::F12 => Some(encode_function_key(12, modifier_code)),

        _ => None,
    }
}

/// Encode cursor key (arrow keys)
fn encode_cursor_key(key: u8, modifier: Option<u8>, application_mode: bool) -> Vec<u8> {
    if let Some(m) = modifier {
        format!("\x1b[1;{}{}", m, key as char).into_bytes()
    } else if application_mode {
        vec![0x1b, b'O', key]
    } else {
        vec![0x1b, b'[', key]
    }
}

/// Encode special key (Home, End)
fn encode_special_key(key: u8, modifier: Option<u8>) -> Vec<u8> {
    if let Some(m) = modifier {
        format!("\x1b[1;{}{}", m, key as char).into_bytes()
    } else {
        vec![0x1b, b'[', key]
    }
}

/// Encode tilde key (Insert, Delete, PageUp, PageDown)
fn encode_tilde_key(code: u8, modifier: Option<u8>) -> Vec<u8> {
    if let Some(m) = modifier {
        format!("\x1b[{};{}~", code, m).into_bytes()
    } else {
        format!("\x1b[{}~", code).into_bytes()
    }
}

/// Encode function key
fn encode_function_key(num: u8, modifier: Option<u8>) -> Vec<u8> {
    // F1-F4 use SS3 (ESC O) format, F5+ use CSI format
    let code = match num {
        1 => b'P',
        2 => b'Q',
        3 => b'R',
        4 => b'S',
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

    if num <= 4 {
        if let Some(m) = modifier {
            format!("\x1b[1;{}{}", m, code as char).into_bytes()
        } else {
            vec![0x1b, b'O', code]
        }
    } else if let Some(m) = modifier {
        format!("\x1b[{};{}~", code, m).into_bytes()
    } else {
        format!("\x1b[{}~", code).into_bytes()
    }
}

/// Mouse button encoding
#[derive(Debug, Clone, Copy)]
pub enum MouseEvent {
    Press(MouseButton, u16, u16),
    Release(MouseButton, u16, u16),
    Move(u16, u16),
    Scroll { x: u16, y: u16, delta: i8 },
}

/// Encode mouse event to terminal escape sequence
pub fn encode_mouse(
    event: MouseEvent,
    sgr_mode: bool,
    button_event_mode: bool,
    any_event_mode: bool,
) -> Option<Vec<u8>> {
    match event {
        MouseEvent::Press(button, x, y) => {
            let button_code = match button {
                MouseButton::Left => 0,
                MouseButton::Middle => 1,
                MouseButton::Right => 2,
                _ => return None,
            };
            Some(encode_mouse_event(button_code, x, y, true, sgr_mode))
        }
        MouseEvent::Release(button, x, y) => {
            let button_code = match button {
                MouseButton::Left => 0,
                MouseButton::Middle => 1,
                MouseButton::Right => 2,
                _ => return None,
            };
            Some(encode_mouse_event(button_code, x, y, false, sgr_mode))
        }
        MouseEvent::Move(x, y) => {
            if any_event_mode || button_event_mode {
                // Motion with no button = button code 35
                Some(encode_mouse_event(35, x, y, true, sgr_mode))
            } else {
                None
            }
        }
        MouseEvent::Scroll { x, y, delta } => {
            // Scroll up = 64, scroll down = 65
            let button_code = if delta > 0 { 64 } else { 65 };
            Some(encode_mouse_event(button_code, x, y, true, sgr_mode))
        }
    }
}

/// Encode a mouse event
fn encode_mouse_event(button: u8, x: u16, y: u16, pressed: bool, sgr_mode: bool) -> Vec<u8> {
    // Convert to 1-based coordinates
    let x = x.saturating_add(1);
    let y = y.saturating_add(1);

    if sgr_mode {
        // SGR mode: ESC [ < button ; x ; y M/m
        let suffix = if pressed { 'M' } else { 'm' };
        format!("\x1b[<{};{};{}{}", button, x, y, suffix).into_bytes()
    } else {
        // X10/VT200 mode: ESC [ M Cb Cx Cy
        // Coordinates are limited to 223 (+ 32 = 255)
        let x = (x.min(223) + 32) as u8;
        let y = (y.min(223) + 32) as u8;
        let button = if pressed { button + 32 } else { 35 }; // Release = button 3
        vec![0x1b, b'[', b'M', button, x, y]
    }
}

/// Encode focus event
pub fn encode_focus(focused: bool) -> Vec<u8> {
    if focused {
        b"\x1b[I".to_vec()
    } else {
        b"\x1b[O".to_vec()
    }
}

/// Wrap text for bracketed paste
pub fn encode_bracketed_paste(text: &str) -> Vec<u8> {
    let mut result = b"\x1b[200~".to_vec();
    result.extend(text.as_bytes());
    result.extend(b"\x1b[201~");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_character() {
        let key = Key::Character("a".into());
        let result = encode_key(&key, ModifiersState::empty(), false, false);
        assert_eq!(result, Some(b"a".to_vec()));
    }

    #[test]
    fn test_encode_ctrl_c() {
        let key = Key::Character("c".into());
        let result = encode_key(&key, ModifiersState::CONTROL, false, false);
        assert_eq!(result, Some(vec![3])); // ETX
    }

    #[test]
    fn test_encode_alt_a() {
        let key = Key::Character("a".into());
        let result = encode_key(&key, ModifiersState::ALT, false, false);
        assert_eq!(result, Some(vec![0x1b, b'a']));
    }

    #[test]
    fn test_encode_arrow_keys() {
        let key = Key::Named(NamedKey::ArrowUp);
        let result = encode_key(&key, ModifiersState::empty(), false, false);
        assert_eq!(result, Some(b"\x1b[A".to_vec()));

        let result = encode_key(&key, ModifiersState::empty(), true, false);
        assert_eq!(result, Some(b"\x1bOA".to_vec()));
    }

    #[test]
    fn test_encode_function_keys() {
        let key = Key::Named(NamedKey::F1);
        let result = encode_key(&key, ModifiersState::empty(), false, false);
        assert_eq!(result, Some(b"\x1bOP".to_vec()));

        let key = Key::Named(NamedKey::F5);
        let result = encode_key(&key, ModifiersState::empty(), false, false);
        assert_eq!(result, Some(b"\x1b[15~".to_vec()));
    }

    #[test]
    fn test_encode_mouse_sgr() {
        let result = encode_mouse_event(0, 10, 20, true, true);
        assert_eq!(result, b"\x1b[<0;11;21M".to_vec());

        let result = encode_mouse_event(0, 10, 20, false, true);
        assert_eq!(result, b"\x1b[<0;11;21m".to_vec());
    }

    #[test]
    fn test_bracketed_paste() {
        let result = encode_bracketed_paste("hello");
        assert_eq!(result, b"\x1b[200~hello\x1b[201~".to_vec());
    }

    #[test]
    fn test_focus_events() {
        assert_eq!(encode_focus(true), b"\x1b[I".to_vec());
        assert_eq!(encode_focus(false), b"\x1b[O".to_vec());
    }

    #[test]
    fn test_encode_direct_control_char() {
        // On macOS, Ctrl+C might produce '\x03' directly instead of 'c' with Ctrl modifier
        let key = Key::Character("\x03".into());
        let result = encode_key(&key, ModifiersState::empty(), false, false);
        assert_eq!(result, Some(vec![3])); // ETX (Ctrl+C)

        // Ctrl+A as direct control character
        let key = Key::Character("\x01".into());
        let result = encode_key(&key, ModifiersState::empty(), false, false);
        assert_eq!(result, Some(vec![1])); // SOH (Ctrl+A)
    }
}
