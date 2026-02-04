//! Terminal Actions
//!
//! Semantic operations produced by the parser that should be applied to the screen.

use serde::{Deserialize, Serialize};

/// A terminal action produced by the parser
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Print a character to the screen
    Print(char),

    /// Execute a C0 control character
    Control(ControlCode),

    /// Execute a CSI (Control Sequence Introducer) command
    Csi(CsiAction),

    /// Execute an OSC (Operating System Command)
    Osc(OscAction),

    /// Execute an ESC sequence (non-CSI)
    Esc(EscAction),

    /// DCS (Device Control String) - currently just consumed
    Dcs(String),

    /// APC (Application Program Command) - currently just consumed
    Apc(String),

    /// PM (Privacy Message) - currently just consumed
    Pm(String),

    /// SOS (Start of String) - currently just consumed
    Sos(String),

    /// Invalid/unrecognized sequence (for debugging)
    Invalid(Vec<u8>),
}

/// C0 control codes (0x00-0x1F)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlCode {
    /// NUL - Null (ignored)
    Null,
    /// BEL - Bell
    Bell,
    /// BS - Backspace
    Backspace,
    /// HT - Horizontal Tab
    Tab,
    /// LF - Line Feed
    LineFeed,
    /// VT - Vertical Tab (treated as LF)
    VerticalTab,
    /// FF - Form Feed (treated as LF)
    FormFeed,
    /// CR - Carriage Return
    CarriageReturn,
    /// SO - Shift Out (switch to G1 charset)
    ShiftOut,
    /// SI - Shift In (switch to G0 charset)
    ShiftIn,
    /// CAN - Cancel (abort escape sequence)
    Cancel,
    /// SUB - Substitute (abort escape sequence, print replacement char)
    Substitute,
    /// ESC - Escape (handled separately)
    Escape,
}

/// CSI (Control Sequence Introducer) actions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CsiAction {
    /// The final character that identifies the command
    pub final_char: char,
    /// Parameters (semicolon-separated numbers)
    pub params: Vec<u16>,
    /// Intermediate characters (between CSI and params)
    pub intermediates: Vec<char>,
    /// Private marker (? or > or other)
    pub private_marker: Option<char>,
}

impl CsiAction {
    pub fn new(final_char: char) -> Self {
        Self {
            final_char,
            params: Vec::new(),
            intermediates: Vec::new(),
            private_marker: None,
        }
    }

    /// Get parameter at index, or default value if not present
    pub fn param(&self, index: usize, default: u16) -> u16 {
        self.params.get(index).copied().unwrap_or(default)
    }

    /// Get parameter at index, treating 0 as default
    pub fn param_or_default(&self, index: usize, default: u16) -> u16 {
        match self.params.get(index) {
            Some(&0) | None => default,
            Some(&v) => v,
        }
    }
}

/// OSC (Operating System Command) actions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OscAction {
    /// Set window title (OSC 0 or OSC 2)
    SetTitle(String),

    /// Set icon name (OSC 1)
    SetIconName(String),

    /// Set hyperlink (OSC 8)
    /// params: optional parameters (e.g., id=xxx)
    /// url: the URL (empty string to end hyperlink)
    Hyperlink { params: Option<String>, url: String },

    /// Clipboard operation (OSC 52)
    /// clipboard: which clipboard (c=clipboard, p=primary, etc.)
    /// data: base64-encoded data (or "?" to query)
    Clipboard { clipboard: String, data: String },

    /// Change color (OSC 4, 10, 11, etc.)
    SetColor { index: u16, color: String },

    /// Reset color (OSC 104, 110, 111, etc.)
    ResetColor { index: u16 },

    /// Unknown/unsupported OSC
    Unknown { command: u16, data: String },
}

/// ESC sequence actions (non-CSI)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscAction {
    /// ESC 7 - Save cursor (DECSC)
    SaveCursor,

    /// ESC 8 - Restore cursor (DECRC)
    RestoreCursor,

    /// ESC D - Index (IND) - move cursor down, scroll if at bottom
    Index,

    /// ESC M - Reverse Index (RI) - move cursor up, scroll if at top
    ReverseIndex,

    /// ESC E - Next Line (NEL) - move to beginning of next line
    NextLine,

    /// ESC H - Horizontal Tab Set (HTS)
    TabSet,

    /// ESC c - Full Reset (RIS)
    FullReset,

    /// ESC = - Application Keypad Mode (DECKPAM)
    ApplicationKeypad,

    /// ESC > - Normal Keypad Mode (DECKPNM)
    NormalKeypad,

    /// ESC ( B - Select ASCII charset for G0
    SelectG0Ascii,

    /// ESC ( 0 - Select DEC Special Graphics charset for G0
    SelectG0DecGraphics,

    /// ESC ) B - Select ASCII charset for G1
    SelectG1Ascii,

    /// ESC ) 0 - Select DEC Special Graphics charset for G1
    SelectG1DecGraphics,

    /// ESC # 8 - DEC Screen Alignment Test (DECALN)
    DecAlignmentTest,

    /// ESC N - Single Shift 2 (SS2)
    SingleShift2,

    /// ESC O - Single Shift 3 (SS3)
    SingleShift3,

    /// Unknown ESC sequence
    Unknown(char),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csi_action_param() {
        let mut csi = CsiAction::new('H');
        csi.params = vec![10, 20];

        assert_eq!(csi.param(0, 1), 10);
        assert_eq!(csi.param(1, 1), 20);
        assert_eq!(csi.param(2, 1), 1); // default
    }

    #[test]
    fn test_csi_action_param_or_default() {
        let mut csi = CsiAction::new('H');
        csi.params = vec![0, 5];

        assert_eq!(csi.param_or_default(0, 1), 1); // 0 treated as default
        assert_eq!(csi.param_or_default(1, 1), 5);
        assert_eq!(csi.param_or_default(2, 1), 1); // missing treated as default
    }
}
