//! Terminal actions produced by the parser
//!
//! These represent the semantic meaning of parsed escape sequences.

use crate::params::Params;

/// Actions produced by the parser
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Print a character to the screen
    Print(char),

    /// Execute a C0 control character
    /// BEL (0x07), BS (0x08), HT (0x09), LF (0x0A), VT (0x0B), FF (0x0C), CR (0x0D)
    Control(u8),

    /// ESC sequence (non-CSI)
    Esc(EscAction),

    /// CSI (Control Sequence Introducer) sequence
    Csi(CsiAction),

    /// OSC (Operating System Command) sequence
    Osc(OscAction),

    /// DCS (Device Control String) - currently just consumed
    Dcs { params: Params, data: Vec<u8> },

    /// APC (Application Program Command) - consumed and ignored
    Apc(Vec<u8>),

    /// PM (Privacy Message) - consumed and ignored
    Pm(Vec<u8>),

    /// SOS (Start of String) - consumed and ignored
    Sos(Vec<u8>),

    /// Invalid/unrecognized sequence (for debugging)
    Invalid(Vec<u8>),
}

/// ESC sequence actions (non-CSI)
#[derive(Debug, Clone, PartialEq)]
pub enum EscAction {
    /// ESC 7 - Save cursor (DECSC)
    SaveCursor,
    /// ESC 8 - Restore cursor (DECRC)
    RestoreCursor,
    /// ESC D - Index (IND) - move cursor down, scroll if at bottom
    Index,
    /// ESC M - Reverse Index (RI) - move cursor up, scroll if at top
    ReverseIndex,
    /// ESC E - Next Line (NEL) - move to start of next line
    NextLine,
    /// ESC H - Horizontal Tab Set (HTS)
    HorizontalTabSet,
    /// ESC c - Full Reset (RIS)
    FullReset,
    /// ESC = - Application Keypad Mode (DECKPAM)
    ApplicationKeypad,
    /// ESC > - Normal Keypad Mode (DECKPNM)
    NormalKeypad,
    /// ESC ( C - Designate G0 Character Set
    DesignateG0(char),
    /// ESC ) C - Designate G1 Character Set
    DesignateG1(char),
    /// ESC * C - Designate G2 Character Set
    DesignateG2(char),
    /// ESC + C - Designate G3 Character Set
    DesignateG3(char),
    /// ESC # 8 - DEC Screen Alignment Test (DECALN)
    DecAlignmentTest,
    /// Unknown ESC sequence
    Unknown(Vec<u8>),
}

/// CSI sequence actions
#[derive(Debug, Clone, PartialEq)]
pub struct CsiAction {
    /// Parameters (semicolon-separated numbers)
    pub params: Params,
    /// Intermediate bytes (0x20-0x2F)
    pub intermediates: Vec<u8>,
    /// Final byte (0x40-0x7E)
    pub final_byte: u8,
    /// Whether this is a private sequence (starts with ?)
    pub private: bool,
    /// Raw marker byte: 0=none, b'?'=private, b'>'=DA2/XTVERSION, b'<', b'='
    pub marker: u8,
}

impl CsiAction {
    /// Get the first parameter with a default value
    pub fn param(&self, index: usize, default: u16) -> u16 {
        self.params.get(index).unwrap_or(default)
    }

    /// Check if this is a specific CSI sequence
    pub fn is(&self, final_byte: u8) -> bool {
        self.final_byte == final_byte && self.intermediates.is_empty() && !self.private
    }

    /// Check if this is a specific private CSI sequence
    pub fn is_private(&self, final_byte: u8) -> bool {
        self.final_byte == final_byte && self.intermediates.is_empty() && self.private
    }
}

/// OSC sequence actions
#[derive(Debug, Clone, PartialEq)]
pub enum OscAction {
    /// OSC 0 - Set icon name and window title
    SetIconAndTitle(String),
    /// OSC 1 - Set icon name
    SetIconName(String),
    /// OSC 2 - Set window title
    SetTitle(String),
    /// OSC 4 - Set/query color palette
    SetColor { index: u8, color: String },
    /// OSC 7 - Set current directory
    SetCurrentDirectory(String),
    /// OSC 8 - Hyperlink
    Hyperlink { params: String, uri: String },
    /// OSC 10 - Set foreground color
    SetForegroundColor(String),
    /// OSC 11 - Set background color
    SetBackgroundColor(String),
    /// OSC 12 - Set cursor color
    SetCursorColor(String),
    /// OSC 52 - Clipboard operation
    Clipboard { clipboard: String, data: String },
    /// OSC 104 - Reset color
    ResetColor(Option<u8>),
    /// OSC 110 - Reset foreground color
    ResetForegroundColor,
    /// OSC 111 - Reset background color
    ResetBackgroundColor,
    /// OSC 112 - Reset cursor color
    ResetCursorColor,
    /// Unknown OSC sequence
    Unknown { command: u16, data: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csi_action_param() {
        let csi = CsiAction {
            params: Params::from_slice(&[10, 20, 30]),
            intermediates: vec![],
            final_byte: b'H',
            private: false,
            marker: 0,
        };

        assert_eq!(csi.param(0, 1), 10);
        assert_eq!(csi.param(1, 1), 20);
        assert_eq!(csi.param(5, 99), 99);
    }

    #[test]
    fn test_csi_action_is() {
        let csi = CsiAction {
            params: Params::new(),
            intermediates: vec![],
            final_byte: b'H',
            private: false,
            marker: 0,
        };

        assert!(csi.is(b'H'));
        assert!(!csi.is(b'J'));
        assert!(!csi.is_private(b'H'));
    }

    #[test]
    fn test_csi_action_is_private() {
        let csi = CsiAction {
            params: Params::new(),
            intermediates: vec![],
            final_byte: b'h',
            private: true,
            marker: b'?',
        };

        assert!(csi.is_private(b'h'));
        assert!(!csi.is(b'h'));
    }
}
