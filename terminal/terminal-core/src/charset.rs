//! Character set handling for terminal emulation
//!
//! Supports DEC Special Graphics (line drawing) and other character sets.

use serde::{Deserialize, Serialize};

/// Character set designations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Charset {
    /// ASCII (US) - default
    #[default]
    Ascii,
    /// DEC Special Graphics (line drawing characters)
    DecSpecialGraphics,
    /// UK character set
    Uk,
}

/// Character set state for G0-G3 slots
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharsetState {
    /// G0 character set
    pub g0: Charset,
    /// G1 character set
    pub g1: Charset,
    /// G2 character set
    pub g2: Charset,
    /// G3 character set
    pub g3: Charset,
    /// Currently active character set (GL - graphics left, 0=G0, 1=G1, etc.)
    pub active: u8,
    /// Single shift state (None, or Some(2) for SS2, Some(3) for SS3)
    pub single_shift: Option<u8>,
}

impl Default for CharsetState {
    fn default() -> Self {
        Self {
            g0: Charset::Ascii,
            g1: Charset::Ascii,
            g2: Charset::Ascii,
            g3: Charset::Ascii,
            active: 0, // G0 is active by default
            single_shift: None,
        }
    }
}

impl CharsetState {
    /// Create new charset state with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset to default state
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Get the currently active charset
    pub fn current(&self) -> Charset {
        // Single shift takes precedence
        if let Some(shift) = self.single_shift {
            return match shift {
                2 => self.g2,
                3 => self.g3,
                _ => self.get_slot(self.active),
            };
        }
        self.get_slot(self.active)
    }

    /// Get charset for a specific slot (0-3)
    fn get_slot(&self, slot: u8) -> Charset {
        match slot {
            0 => self.g0,
            1 => self.g1,
            2 => self.g2,
            3 => self.g3,
            _ => Charset::Ascii,
        }
    }

    /// Set charset for a slot
    pub fn set_slot(&mut self, slot: u8, charset: Charset) {
        match slot {
            0 => self.g0 = charset,
            1 => self.g1 = charset,
            2 => self.g2 = charset,
            3 => self.g3 = charset,
            _ => {}
        }
    }

    /// Shift In (SI) - select G0 into GL
    pub fn shift_in(&mut self) {
        self.active = 0;
        self.single_shift = None;
    }

    /// Shift Out (SO) - select G1 into GL
    pub fn shift_out(&mut self) {
        self.active = 1;
        self.single_shift = None;
    }

    /// Single Shift 2 (SS2) - use G2 for next character only
    pub fn single_shift_2(&mut self) {
        self.single_shift = Some(2);
    }

    /// Single Shift 3 (SS3) - use G3 for next character only
    pub fn single_shift_3(&mut self) {
        self.single_shift = Some(3);
    }

    /// Clear single shift after using it
    pub fn clear_single_shift(&mut self) {
        self.single_shift = None;
    }

    /// Translate a character through the current charset
    pub fn translate(&self, c: char) -> char {
        let charset = self.current();
        translate_char(c, charset)
    }
}

/// Translate a character through a specific charset
pub fn translate_char(c: char, charset: Charset) -> char {
    match charset {
        Charset::Ascii => c,
        Charset::DecSpecialGraphics => translate_dec_special_graphics(c),
        Charset::Uk => translate_uk(c),
    }
}

/// Translate DEC Special Graphics characters
/// Maps ASCII 0x5F-0x7E to line drawing and other special characters
fn translate_dec_special_graphics(c: char) -> char {
    match c {
        // Box drawing characters
        '`' => '◆', // Diamond
        'a' => '▒', // Checkerboard
        'b' => '␉', // HT symbol
        'c' => '␌', // FF symbol
        'd' => '␍', // CR symbol
        'e' => '␊', // LF symbol
        'f' => '°',  // Degree symbol
        'g' => '±',  // Plus/minus
        'h' => '␤', // NL symbol
        'i' => '␋', // VT symbol
        'j' => '┘', // Lower right corner
        'k' => '┐', // Upper right corner
        'l' => '┌', // Upper left corner
        'm' => '└', // Lower left corner
        'n' => '┼', // Crossing lines
        'o' => '⎺', // Scan line 1
        'p' => '⎻', // Scan line 3
        'q' => '─', // Horizontal line (scan line 5)
        'r' => '⎼', // Scan line 7
        's' => '⎽', // Scan line 9
        't' => '├', // Left tee
        'u' => '┤', // Right tee
        'v' => '┴', // Bottom tee
        'w' => '┬', // Top tee
        'x' => '│', // Vertical line
        'y' => '≤', // Less than or equal
        'z' => '≥', // Greater than or equal
        '{' => 'π',  // Pi
        '|' => '≠', // Not equal
        '}' => '£',  // Pound sterling
        '~' => '·',  // Centered dot / bullet
        _ => c,      // Pass through unchanged
    }
}

/// Translate UK character set (only # differs)
fn translate_uk(c: char) -> char {
    match c {
        '#' => '£', // Pound sterling
        _ => c,
    }
}

/// Parse a charset designation character
pub fn parse_charset_designation(c: char) -> Charset {
    match c {
        'B' | '@' => Charset::Ascii,       // ASCII
        '0' | '2' => Charset::DecSpecialGraphics, // DEC Special Graphics
        'A' => Charset::Uk,                // UK
        _ => Charset::Ascii,               // Default to ASCII for unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_charset_default() {
        let state = CharsetState::new();
        assert_eq!(state.g0, Charset::Ascii);
        assert_eq!(state.active, 0);
    }

    #[test]
    fn test_dec_special_graphics() {
        assert_eq!(translate_dec_special_graphics('j'), '┘');
        assert_eq!(translate_dec_special_graphics('k'), '┐');
        assert_eq!(translate_dec_special_graphics('l'), '┌');
        assert_eq!(translate_dec_special_graphics('m'), '└');
        assert_eq!(translate_dec_special_graphics('q'), '─');
        assert_eq!(translate_dec_special_graphics('x'), '│');
        assert_eq!(translate_dec_special_graphics('n'), '┼');
        assert_eq!(translate_dec_special_graphics('t'), '├');
        assert_eq!(translate_dec_special_graphics('u'), '┤');
        assert_eq!(translate_dec_special_graphics('v'), '┴');
        assert_eq!(translate_dec_special_graphics('w'), '┬');
    }

    #[test]
    fn test_shift_in_out() {
        let mut state = CharsetState::new();
        state.g1 = Charset::DecSpecialGraphics;

        assert_eq!(state.current(), Charset::Ascii);

        state.shift_out(); // Select G1
        assert_eq!(state.current(), Charset::DecSpecialGraphics);

        state.shift_in(); // Select G0
        assert_eq!(state.current(), Charset::Ascii);
    }

    #[test]
    fn test_translate() {
        let mut state = CharsetState::new();
        state.g0 = Charset::DecSpecialGraphics;

        assert_eq!(state.translate('q'), '─');
        assert_eq!(state.translate('A'), 'A'); // Non-special chars pass through
    }
}
