//! Terminal Actions
//!
//! Actions represent the semantic operations that result from parsing
//! escape sequences. The terminal core applies these actions to update
//! its state.

use serde::{Deserialize, Serialize};

/// A terminal action produced by the parser
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Print a character at the current cursor position
    Print(char),

    /// Execute a C0 control character
    Execute(u8),

    /// CSI (Control Sequence Introducer) dispatch
    CsiDispatch(CsiAction),

    /// ESC sequence dispatch (non-CSI)
    EscDispatch(EscAction),

    /// OSC (Operating System Command) dispatch
    OscDispatch(OscAction),

    /// DCS (Device Control String) - currently just consumed, not fully implemented
    DcsDispatch(Vec<u8>),

    /// APC (Application Program Command) - consumed but ignored
    ApcDispatch(Vec<u8>),

    /// PM (Privacy Message) - consumed but ignored
    PmDispatch(Vec<u8>),

    /// SOS (Start of String) - consumed but ignored
    SosDispatch(Vec<u8>),

    /// Invalid/malformed sequence (for error tracking)
    Invalid(Vec<u8>),
}

/// CSI sequence action with parsed parameters
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CsiAction {
    /// Parameters (semicolon-separated numbers)
    pub params: Vec<u32>,
    /// Intermediate bytes (between CSI and final byte)
    pub intermediates: Vec<u8>,
    /// Final byte that identifies the command
    pub final_byte: u8,
    /// Whether this is a private sequence (starts with ?)
    pub private: bool,
}

impl CsiAction {
    /// Get parameter at index with default value
    pub fn param(&self, index: usize, default: u32) -> u32 {
        self.params.get(index).copied().unwrap_or(default)
    }

    /// Get parameter at index, treating 0 as default
    pub fn param_or_default(&self, index: usize, default: u32) -> u32 {
        match self.params.get(index) {
            Some(&0) | None => default,
            Some(&v) => v,
        }
    }
}

/// ESC sequence action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscAction {
    /// Save cursor (ESC 7 / DECSC)
    SaveCursor,
    /// Restore cursor (ESC 8 / DECRC)
    RestoreCursor,
    /// Index - move down one line, scroll if at bottom (ESC D)
    Index,
    /// Reverse index - move up one line, scroll if at top (ESC M)
    ReverseIndex,
    /// Next line - move to start of next line (ESC E)
    NextLine,
    /// Horizontal tab set (ESC H)
    HorizontalTabSet,
    /// Reset to initial state (ESC c)
    FullReset,
    /// Designate character set G0 (ESC ( X)
    DesignateG0(u8),
    /// Designate character set G1 (ESC ) X)
    DesignateG1(u8),
    /// Designate character set G2 (ESC * X)
    DesignateG2(u8),
    /// Designate character set G3 (ESC + X)
    DesignateG3(u8),
    /// Application keypad mode (ESC =)
    ApplicationKeypad,
    /// Normal keypad mode (ESC >)
    NormalKeypad,
    /// Set single shift 2 (ESC N)
    SingleShift2,
    /// Set single shift 3 (ESC O)
    SingleShift3,
    /// Unknown/unhandled ESC sequence
    Unknown(Vec<u8>),
}

/// OSC (Operating System Command) action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OscAction {
    /// Set window title (OSC 0 or OSC 2)
    SetTitle(String),
    /// Set icon name (OSC 1)
    SetIconName(String),
    /// Set/query color (OSC 4, 10, 11, etc.)
    SetColor {
        index: u32,
        color: String,
    },
    /// Hyperlink (OSC 8)
    Hyperlink {
        params: String,
        uri: String,
    },
    /// Clipboard operation (OSC 52)
    Clipboard {
        clipboard: String,
        data: String,
    },
    /// Reset color (OSC 104, 110, 111, etc.)
    ResetColor(u32),
    /// Unknown OSC command
    Unknown {
        command: u32,
        data: String,
    },
}

/// SGR (Select Graphic Rendition) attribute
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SgrAttribute {
    /// Reset all attributes (0)
    Reset,
    /// Bold (1)
    Bold,
    /// Faint/dim (2)
    Faint,
    /// Italic (3)
    Italic,
    /// Underline (4)
    Underline,
    /// Slow blink (5)
    SlowBlink,
    /// Rapid blink (6)
    RapidBlink,
    /// Inverse/reverse video (7)
    Inverse,
    /// Hidden/invisible (8)
    Hidden,
    /// Strikethrough (9)
    Strikethrough,
    /// Normal intensity - not bold, not faint (22)
    NormalIntensity,
    /// Not italic (23)
    NotItalic,
    /// Not underlined (24)
    NotUnderlined,
    /// Not blinking (25)
    NotBlinking,
    /// Not inverse (27)
    NotInverse,
    /// Not hidden (28)
    NotHidden,
    /// Not strikethrough (29)
    NotStrikethrough,
    /// Foreground color (30-37, 90-97)
    ForegroundIndexed(u8),
    /// Background color (40-47, 100-107)
    BackgroundIndexed(u8),
    /// Default foreground (39)
    DefaultForeground,
    /// Default background (49)
    DefaultBackground,
    /// 256-color foreground (38;5;N)
    Foreground256(u8),
    /// 256-color background (48;5;N)
    Background256(u8),
    /// True color foreground (38;2;R;G;B)
    ForegroundRgb(u8, u8, u8),
    /// True color background (48;2;R;G;B)
    BackgroundRgb(u8, u8, u8),
}

impl CsiAction {
    /// Parse SGR parameters into a list of attributes
    pub fn parse_sgr(&self) -> Vec<SgrAttribute> {
        let mut attrs = Vec::new();
        let mut i = 0;

        while i < self.params.len() {
            let code = self.params[i];
            match code {
                0 => attrs.push(SgrAttribute::Reset),
                1 => attrs.push(SgrAttribute::Bold),
                2 => attrs.push(SgrAttribute::Faint),
                3 => attrs.push(SgrAttribute::Italic),
                4 => attrs.push(SgrAttribute::Underline),
                5 => attrs.push(SgrAttribute::SlowBlink),
                6 => attrs.push(SgrAttribute::RapidBlink),
                7 => attrs.push(SgrAttribute::Inverse),
                8 => attrs.push(SgrAttribute::Hidden),
                9 => attrs.push(SgrAttribute::Strikethrough),
                22 => attrs.push(SgrAttribute::NormalIntensity),
                23 => attrs.push(SgrAttribute::NotItalic),
                24 => attrs.push(SgrAttribute::NotUnderlined),
                25 => attrs.push(SgrAttribute::NotBlinking),
                27 => attrs.push(SgrAttribute::NotInverse),
                28 => attrs.push(SgrAttribute::NotHidden),
                29 => attrs.push(SgrAttribute::NotStrikethrough),
                30..=37 => attrs.push(SgrAttribute::ForegroundIndexed((code - 30) as u8)),
                38 => {
                    // Extended foreground color
                    if i + 1 < self.params.len() {
                        match self.params[i + 1] {
                            5 if i + 2 < self.params.len() => {
                                // 256-color: 38;5;N
                                attrs.push(SgrAttribute::Foreground256(
                                    self.params[i + 2] as u8,
                                ));
                                i += 2;
                            }
                            2 if i + 4 < self.params.len() => {
                                // True color: 38;2;R;G;B
                                attrs.push(SgrAttribute::ForegroundRgb(
                                    self.params[i + 2] as u8,
                                    self.params[i + 3] as u8,
                                    self.params[i + 4] as u8,
                                ));
                                i += 4;
                            }
                            _ => {}
                        }
                    }
                }
                39 => attrs.push(SgrAttribute::DefaultForeground),
                40..=47 => attrs.push(SgrAttribute::BackgroundIndexed((code - 40) as u8)),
                48 => {
                    // Extended background color
                    if i + 1 < self.params.len() {
                        match self.params[i + 1] {
                            5 if i + 2 < self.params.len() => {
                                // 256-color: 48;5;N
                                attrs.push(SgrAttribute::Background256(
                                    self.params[i + 2] as u8,
                                ));
                                i += 2;
                            }
                            2 if i + 4 < self.params.len() => {
                                // True color: 48;2;R;G;B
                                attrs.push(SgrAttribute::BackgroundRgb(
                                    self.params[i + 2] as u8,
                                    self.params[i + 3] as u8,
                                    self.params[i + 4] as u8,
                                ));
                                i += 4;
                            }
                            _ => {}
                        }
                    }
                }
                49 => attrs.push(SgrAttribute::DefaultBackground),
                90..=97 => attrs.push(SgrAttribute::ForegroundIndexed((code - 90 + 8) as u8)),
                100..=107 => attrs.push(SgrAttribute::BackgroundIndexed((code - 100 + 8) as u8)),
                _ => {} // Unknown SGR code, ignore
            }
            i += 1;
        }

        // Empty params means reset
        if attrs.is_empty() {
            attrs.push(SgrAttribute::Reset);
        }

        attrs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csi_param() {
        let csi = CsiAction {
            params: vec![5, 10],
            intermediates: vec![],
            final_byte: b'H',
            private: false,
        };

        assert_eq!(csi.param(0, 1), 5);
        assert_eq!(csi.param(1, 1), 10);
        assert_eq!(csi.param(2, 99), 99); // Default for missing
    }

    #[test]
    fn test_csi_param_or_default() {
        let csi = CsiAction {
            params: vec![0, 5],
            intermediates: vec![],
            final_byte: b'H',
            private: false,
        };

        assert_eq!(csi.param_or_default(0, 1), 1); // 0 treated as default
        assert_eq!(csi.param_or_default(1, 1), 5);
        assert_eq!(csi.param_or_default(2, 1), 1); // Missing treated as default
    }

    #[test]
    fn test_sgr_parse_basic() {
        let csi = CsiAction {
            params: vec![1, 4, 31],
            intermediates: vec![],
            final_byte: b'm',
            private: false,
        };

        let attrs = csi.parse_sgr();
        assert_eq!(attrs.len(), 3);
        assert_eq!(attrs[0], SgrAttribute::Bold);
        assert_eq!(attrs[1], SgrAttribute::Underline);
        assert_eq!(attrs[2], SgrAttribute::ForegroundIndexed(1)); // Red
    }

    #[test]
    fn test_sgr_parse_256_color() {
        let csi = CsiAction {
            params: vec![38, 5, 196],
            intermediates: vec![],
            final_byte: b'm',
            private: false,
        };

        let attrs = csi.parse_sgr();
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0], SgrAttribute::Foreground256(196));
    }

    #[test]
    fn test_sgr_parse_truecolor() {
        let csi = CsiAction {
            params: vec![48, 2, 255, 128, 0],
            intermediates: vec![],
            final_byte: b'm',
            private: false,
        };

        let attrs = csi.parse_sgr();
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0], SgrAttribute::BackgroundRgb(255, 128, 0));
    }

    #[test]
    fn test_sgr_parse_empty() {
        let csi = CsiAction {
            params: vec![],
            intermediates: vec![],
            final_byte: b'm',
            private: false,
        };

        let attrs = csi.parse_sgr();
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0], SgrAttribute::Reset);
    }

    #[test]
    fn test_sgr_parse_bright_colors() {
        let csi = CsiAction {
            params: vec![91, 101],
            intermediates: vec![],
            final_byte: b'm',
            private: false,
        };

        let attrs = csi.parse_sgr();
        assert_eq!(attrs.len(), 2);
        assert_eq!(attrs[0], SgrAttribute::ForegroundIndexed(9)); // Bright red
        assert_eq!(attrs[1], SgrAttribute::BackgroundIndexed(9)); // Bright red bg
    }
}
