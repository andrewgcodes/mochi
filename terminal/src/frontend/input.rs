//! Input handling for the terminal
//!
//! Converts keyboard and mouse events to terminal escape sequences.

use std::fmt::Write;

/// Keyboard modifiers
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub logo: bool,
}

impl Modifiers {
    /// Create new modifiers
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any modifier is pressed
    pub fn any(&self) -> bool {
        self.shift || self.ctrl || self.alt || self.logo
    }

    /// Get the modifier parameter for CSI sequences (xterm style)
    /// Returns 1 + (shift ? 1 : 0) + (alt ? 2 : 0) + (ctrl ? 4 : 0)
    pub fn csi_param(&self) -> u8 {
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
}

/// Key codes for special keys
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
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

    // Navigation
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

    // Control
    Escape,
    Tab,
    Backspace,
    Enter,

    // Character input
    Char(char),
}

/// Encode a key press to terminal escape sequence
///
/// # Arguments
/// * `key` - The key code
/// * `mods` - Keyboard modifiers
/// * `application_cursor` - Whether application cursor mode is enabled
/// * `application_keypad` - Whether application keypad mode is enabled
///
/// # Returns
/// The escape sequence to send to the PTY, or None if the key should be ignored
pub fn encode_key(
    key: KeyCode,
    mods: Modifiers,
    application_cursor: bool,
    _application_keypad: bool,
) -> Option<String> {
    let mut output = String::new();

    match key {
        // Function keys
        KeyCode::F1 => encode_function_key(&mut output, 1, mods),
        KeyCode::F2 => encode_function_key(&mut output, 2, mods),
        KeyCode::F3 => encode_function_key(&mut output, 3, mods),
        KeyCode::F4 => encode_function_key(&mut output, 4, mods),
        KeyCode::F5 => encode_function_key(&mut output, 5, mods),
        KeyCode::F6 => encode_function_key(&mut output, 6, mods),
        KeyCode::F7 => encode_function_key(&mut output, 7, mods),
        KeyCode::F8 => encode_function_key(&mut output, 8, mods),
        KeyCode::F9 => encode_function_key(&mut output, 9, mods),
        KeyCode::F10 => encode_function_key(&mut output, 10, mods),
        KeyCode::F11 => encode_function_key(&mut output, 11, mods),
        KeyCode::F12 => encode_function_key(&mut output, 12, mods),

        // Arrow keys
        KeyCode::Up => encode_cursor_key(&mut output, 'A', mods, application_cursor),
        KeyCode::Down => encode_cursor_key(&mut output, 'B', mods, application_cursor),
        KeyCode::Right => encode_cursor_key(&mut output, 'C', mods, application_cursor),
        KeyCode::Left => encode_cursor_key(&mut output, 'D', mods, application_cursor),

        // Navigation keys
        KeyCode::Home => encode_special_key(&mut output, 1, 'H', mods),
        KeyCode::End => encode_special_key(&mut output, 1, 'F', mods),
        KeyCode::PageUp => encode_special_key(&mut output, 5, '~', mods),
        KeyCode::PageDown => encode_special_key(&mut output, 6, '~', mods),
        KeyCode::Insert => encode_special_key(&mut output, 2, '~', mods),
        KeyCode::Delete => encode_special_key(&mut output, 3, '~', mods),

        // Control keys
        KeyCode::Escape => {
            output.push('\x1b');
        },
        KeyCode::Tab => {
            if mods.shift {
                // Shift+Tab = backtab
                output.push_str("\x1b[Z");
            } else {
                output.push('\t');
            }
        },
        KeyCode::Backspace => {
            if mods.ctrl {
                // Ctrl+Backspace = delete word (send DEL or Ctrl+W)
                output.push('\x17'); // Ctrl+W
            } else if mods.alt {
                // Alt+Backspace
                output.push('\x1b');
                output.push('\x7f');
            } else {
                output.push('\x7f'); // DEL
            }
        },
        KeyCode::Enter => {
            if mods.alt {
                output.push('\x1b');
            }
            output.push('\r');
        },

        // Character input
        KeyCode::Char(c) => {
            encode_char(&mut output, c, mods);
        },
    }

    if output.is_empty() {
        None
    } else {
        Some(output)
    }
}

/// Encode a function key
fn encode_function_key(output: &mut String, num: u8, mods: Modifiers) {
    // F1-F4 use SS3 (ESC O) format, F5+ use CSI format
    // With modifiers, all use CSI format
    let code = match num {
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
        _ => return,
    };

    if mods.any() {
        let _ = write!(output, "\x1b[{};{}~", code, mods.csi_param());
    } else if num <= 4 {
        // F1-F4 without modifiers use SS3 format
        let final_char = match num {
            1 => 'P',
            2 => 'Q',
            3 => 'R',
            4 => 'S',
            _ => unreachable!(),
        };
        let _ = write!(output, "\x1bO{}", final_char);
    } else {
        let _ = write!(output, "\x1b[{}~", code);
    }
}

/// Encode a cursor key (arrow keys)
fn encode_cursor_key(output: &mut String, final_char: char, mods: Modifiers, application: bool) {
    if mods.any() {
        let _ = write!(output, "\x1b[1;{}{}", mods.csi_param(), final_char);
    } else if application {
        let _ = write!(output, "\x1bO{}", final_char);
    } else {
        let _ = write!(output, "\x1b[{}", final_char);
    }
}

/// Encode a special key (Home, End, PageUp, PageDown, Insert, Delete)
fn encode_special_key(output: &mut String, code: u8, final_char: char, mods: Modifiers) {
    if mods.any() {
        let _ = write!(output, "\x1b[{};{}{}", code, mods.csi_param(), final_char);
    } else if final_char == '~' {
        let _ = write!(output, "\x1b[{}~", code);
    } else {
        let _ = write!(output, "\x1b[{}", final_char);
    }
}

/// Encode a character with modifiers
fn encode_char(output: &mut String, c: char, mods: Modifiers) {
    if mods.ctrl {
        // Ctrl+letter produces control character
        if c.is_ascii_lowercase() {
            let ctrl_char = (c as u8 - b'a' + 1) as char;
            if mods.alt {
                output.push('\x1b');
            }
            output.push(ctrl_char);
            return;
        } else if c.is_ascii_uppercase() {
            let ctrl_char = (c as u8 - b'A' + 1) as char;
            if mods.alt {
                output.push('\x1b');
            }
            output.push(ctrl_char);
            return;
        } else {
            // Special Ctrl combinations
            match c {
                '@' => {
                    if mods.alt {
                        output.push('\x1b');
                    }
                    output.push('\x00');
                    return;
                },
                '[' => {
                    if mods.alt {
                        output.push('\x1b');
                    }
                    output.push('\x1b');
                    return;
                },
                '\\' => {
                    if mods.alt {
                        output.push('\x1b');
                    }
                    output.push('\x1c');
                    return;
                },
                ']' => {
                    if mods.alt {
                        output.push('\x1b');
                    }
                    output.push('\x1d');
                    return;
                },
                '^' => {
                    if mods.alt {
                        output.push('\x1b');
                    }
                    output.push('\x1e');
                    return;
                },
                '_' => {
                    if mods.alt {
                        output.push('\x1b');
                    }
                    output.push('\x1f');
                    return;
                },
                '?' => {
                    if mods.alt {
                        output.push('\x1b');
                    }
                    output.push('\x7f');
                    return;
                },
                ' ' => {
                    if mods.alt {
                        output.push('\x1b');
                    }
                    output.push('\x00');
                    return;
                },
                _ => {},
            }
        }
    }

    // Alt+key sends ESC prefix
    if mods.alt {
        output.push('\x1b');
    }

    output.push(c);
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

/// Mouse event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventType {
    Press,
    Release,
    Motion,
}

/// Mouse encoding mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEncoding {
    X10,
    Utf8,
    Sgr,
    Urxvt,
}

/// Encode a mouse event
///
/// # Arguments
/// * `event_type` - Type of mouse event
/// * `button` - Mouse button (if applicable)
/// * `x` - Column (1-based)
/// * `y` - Row (1-based)
/// * `mods` - Keyboard modifiers
/// * `encoding` - Mouse encoding mode
///
/// # Returns
/// The escape sequence to send to the PTY
pub fn encode_mouse(
    event_type: MouseEventType,
    button: MouseButton,
    x: u16,
    y: u16,
    mods: Modifiers,
    encoding: MouseEncoding,
) -> String {
    let mut cb = match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        MouseButton::WheelUp => 64,
        MouseButton::WheelDown => 65,
    };

    // Add modifier bits
    if mods.shift {
        cb += 4;
    }
    if mods.alt {
        cb += 8;
    }
    if mods.ctrl {
        cb += 16;
    }

    // Add motion bit for motion events
    if event_type == MouseEventType::Motion {
        cb += 32;
    }

    match encoding {
        MouseEncoding::X10 | MouseEncoding::Utf8 => {
            // X10 encoding: ESC [ M Cb Cx Cy
            // Values are encoded as character + 32
            let cx = (x.min(223) + 32) as u8 as char;
            let cy = (y.min(223) + 32) as u8 as char;
            let cb_char = (cb + 32) as u8 as char;

            if event_type == MouseEventType::Release {
                // X10 doesn't distinguish release, use button 3
                let release_cb = (3 + 32) as u8 as char;
                format!("\x1b[M{}{}{}", release_cb, cx, cy)
            } else {
                format!("\x1b[M{}{}{}", cb_char, cx, cy)
            }
        },
        MouseEncoding::Sgr => {
            // SGR encoding: ESC [ < Cb ; Cx ; Cy M/m
            let final_char = if event_type == MouseEventType::Release {
                'm'
            } else {
                'M'
            };
            format!("\x1b[<{};{};{}{}", cb, x, y, final_char)
        },
        MouseEncoding::Urxvt => {
            // URXVT encoding: ESC [ Cb ; Cx ; Cy M
            format!("\x1b[{};{};{}M", cb + 32, x, y)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_char() {
        let mods = Modifiers::default();
        assert_eq!(
            encode_key(KeyCode::Char('a'), mods, false, false),
            Some("a".to_string())
        );
        assert_eq!(
            encode_key(KeyCode::Char('Z'), mods, false, false),
            Some("Z".to_string())
        );
    }

    #[test]
    fn test_encode_ctrl_char() {
        let mods = Modifiers {
            ctrl: true,
            ..Default::default()
        };
        assert_eq!(
            encode_key(KeyCode::Char('c'), mods, false, false),
            Some("\x03".to_string())
        );
        assert_eq!(
            encode_key(KeyCode::Char('a'), mods, false, false),
            Some("\x01".to_string())
        );
    }

    #[test]
    fn test_encode_alt_char() {
        let mods = Modifiers {
            alt: true,
            ..Default::default()
        };
        assert_eq!(
            encode_key(KeyCode::Char('x'), mods, false, false),
            Some("\x1bx".to_string())
        );
    }

    #[test]
    fn test_encode_arrow_keys() {
        let mods = Modifiers::default();
        assert_eq!(
            encode_key(KeyCode::Up, mods, false, false),
            Some("\x1b[A".to_string())
        );
        assert_eq!(
            encode_key(KeyCode::Down, mods, false, false),
            Some("\x1b[B".to_string())
        );
        assert_eq!(
            encode_key(KeyCode::Right, mods, false, false),
            Some("\x1b[C".to_string())
        );
        assert_eq!(
            encode_key(KeyCode::Left, mods, false, false),
            Some("\x1b[D".to_string())
        );
    }

    #[test]
    fn test_encode_arrow_keys_application() {
        let mods = Modifiers::default();
        assert_eq!(
            encode_key(KeyCode::Up, mods, true, false),
            Some("\x1bOA".to_string())
        );
        assert_eq!(
            encode_key(KeyCode::Down, mods, true, false),
            Some("\x1bOB".to_string())
        );
    }

    #[test]
    fn test_encode_arrow_keys_with_modifiers() {
        let mods = Modifiers {
            shift: true,
            ..Default::default()
        };
        assert_eq!(
            encode_key(KeyCode::Up, mods, false, false),
            Some("\x1b[1;2A".to_string())
        );

        let mods = Modifiers {
            ctrl: true,
            ..Default::default()
        };
        assert_eq!(
            encode_key(KeyCode::Up, mods, false, false),
            Some("\x1b[1;5A".to_string())
        );
    }

    #[test]
    fn test_encode_function_keys() {
        let mods = Modifiers::default();
        assert_eq!(
            encode_key(KeyCode::F1, mods, false, false),
            Some("\x1bOP".to_string())
        );
        assert_eq!(
            encode_key(KeyCode::F5, mods, false, false),
            Some("\x1b[15~".to_string())
        );
    }

    #[test]
    fn test_encode_special_keys() {
        let mods = Modifiers::default();
        assert_eq!(
            encode_key(KeyCode::Home, mods, false, false),
            Some("\x1b[H".to_string())
        );
        assert_eq!(
            encode_key(KeyCode::End, mods, false, false),
            Some("\x1b[F".to_string())
        );
        assert_eq!(
            encode_key(KeyCode::PageUp, mods, false, false),
            Some("\x1b[5~".to_string())
        );
        assert_eq!(
            encode_key(KeyCode::Delete, mods, false, false),
            Some("\x1b[3~".to_string())
        );
    }

    #[test]
    fn test_encode_mouse_sgr() {
        let mods = Modifiers::default();
        let seq = encode_mouse(
            MouseEventType::Press,
            MouseButton::Left,
            10,
            5,
            mods,
            MouseEncoding::Sgr,
        );
        assert_eq!(seq, "\x1b[<0;10;5M");

        let seq = encode_mouse(
            MouseEventType::Release,
            MouseButton::Left,
            10,
            5,
            mods,
            MouseEncoding::Sgr,
        );
        assert_eq!(seq, "\x1b[<0;10;5m");
    }

    #[test]
    fn test_encode_mouse_x10() {
        let mods = Modifiers::default();
        let seq = encode_mouse(
            MouseEventType::Press,
            MouseButton::Left,
            1,
            1,
            mods,
            MouseEncoding::X10,
        );
        // Button 0 + 32 = 32 = ' ', x=1+32=33='!', y=1+32=33='!'
        assert_eq!(seq, "\x1b[M !!");
    }

    #[test]
    fn test_modifiers_csi_param() {
        assert_eq!(Modifiers::default().csi_param(), 1);
        assert_eq!(
            Modifiers {
                shift: true,
                ..Default::default()
            }
            .csi_param(),
            2
        );
        assert_eq!(
            Modifiers {
                alt: true,
                ..Default::default()
            }
            .csi_param(),
            3
        );
        assert_eq!(
            Modifiers {
                ctrl: true,
                ..Default::default()
            }
            .csi_param(),
            5
        );
        assert_eq!(
            Modifiers {
                shift: true,
                ctrl: true,
                ..Default::default()
            }
            .csi_param(),
            6
        );
    }
}
