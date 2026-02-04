//! Terminal actions produced by the parser
//!
//! These actions represent the semantic meaning of parsed escape sequences.

use serde::{Deserialize, Serialize};

/// Actions produced by the parser
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalAction {
    /// Print a character to the screen at the current cursor position
    Print(char),

    /// Execute a C0 control character (0x00-0x1F except ESC)
    /// Common controls:
    /// - 0x07 BEL: Bell
    /// - 0x08 BS: Backspace
    /// - 0x09 HT: Horizontal Tab
    /// - 0x0A LF: Line Feed
    /// - 0x0B VT: Vertical Tab (treated as LF)
    /// - 0x0C FF: Form Feed (treated as LF)
    /// - 0x0D CR: Carriage Return
    /// - 0x0E SO: Shift Out
    /// - 0x0F SI: Shift In
    Execute(u8),

    /// CSI (Control Sequence Introducer) dispatch
    /// Format: ESC \[ \[params\] \[intermediates\] final
    CsiDispatch {
        /// Numeric parameters separated by semicolons
        /// Empty parameters are represented as 0
        params: Vec<u16>,
        /// Intermediate bytes (0x20-0x2F)
        /// First byte may be private marker (?, >, <, =)
        intermediates: Vec<u8>,
        /// Final byte (0x40-0x7E) determines the command
        final_byte: u8,
    },

    /// OSC (Operating System Command) dispatch
    /// Format: ESC ] Ps ; Pt BEL  or  ESC ] Ps ; Pt ST
    OscDispatch {
        /// Parameters split by semicolons
        /// First param is typically the command number
        params: Vec<Vec<u8>>,
    },

    /// ESC dispatch (non-CSI escape sequences)
    /// Format: ESC \[intermediates\] final
    EscDispatch {
        /// Intermediate bytes (0x20-0x2F)
        intermediates: Vec<u8>,
        /// Final byte
        final_byte: u8,
    },

    /// DCS (Device Control String) hook - start of DCS sequence
    DcsHook {
        /// Numeric parameters
        params: Vec<u16>,
        /// Intermediate bytes
        intermediates: Vec<u8>,
        /// Final byte
        final_byte: u8,
    },

    /// DCS data byte
    DcsPut(u8),

    /// DCS unhook - end of DCS sequence
    DcsUnhook,

    /// Full DCS dispatch (for simpler handling)
    DcsDispatch {
        /// Numeric parameters
        params: Vec<u16>,
        /// Intermediate bytes
        intermediates: Vec<u8>,
        /// Final byte
        final_byte: u8,
        /// Data bytes
        data: Vec<u8>,
    },
}

impl TerminalAction {
    /// Check if this is a print action
    pub fn is_print(&self) -> bool {
        matches!(self, TerminalAction::Print(_))
    }

    /// Check if this is an execute action
    pub fn is_execute(&self) -> bool {
        matches!(self, TerminalAction::Execute(_))
    }

    /// Check if this is a CSI action
    pub fn is_csi(&self) -> bool {
        matches!(self, TerminalAction::CsiDispatch { .. })
    }

    /// Check if this is an OSC action
    pub fn is_osc(&self) -> bool {
        matches!(self, TerminalAction::OscDispatch { .. })
    }

    /// Check if this is an ESC action
    pub fn is_esc(&self) -> bool {
        matches!(self, TerminalAction::EscDispatch { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_types() {
        assert!(TerminalAction::Print('A').is_print());
        assert!(TerminalAction::Execute(0x0A).is_execute());
        assert!(TerminalAction::CsiDispatch {
            params: vec![],
            intermediates: vec![],
            final_byte: b'H',
        }
        .is_csi());
        assert!(TerminalAction::OscDispatch { params: vec![] }.is_osc());
        assert!(TerminalAction::EscDispatch {
            intermediates: vec![],
            final_byte: b'7',
        }
        .is_esc());
    }

    #[test]
    fn test_action_serialization() {
        let action = TerminalAction::CsiDispatch {
            params: vec![1, 2, 3],
            intermediates: vec![b'?'],
            final_byte: b'h',
        };

        let json = serde_json::to_string(&action).unwrap();
        let restored: TerminalAction = serde_json::from_str(&json).unwrap();

        assert_eq!(action, restored);
    }
}
