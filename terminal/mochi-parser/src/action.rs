//! Terminal actions produced by the parser
//!
//! Actions represent semantic operations that the terminal should perform.
//! They are produced by the parser and consumed by the terminal core.

use serde::{Deserialize, Serialize};

/// A terminal action produced by the parser
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Print a character (may be a grapheme cluster)
    Print(char),

    /// Execute a C0 control character
    Execute(u8),

    /// CSI sequence dispatch
    CsiDispatch(CsiAction),

    /// ESC sequence dispatch
    EscDispatch(EscAction),

    /// OSC sequence dispatch
    OscDispatch(OscAction),

    /// DCS sequence (device control string)
    DcsDispatch(DcsAction),

    /// Hook for DCS start
    DcsHook(DcsHook),

    /// DCS data (put)
    DcsPut(u8),

    /// DCS unhook (end)
    DcsUnhook,

    /// APC sequence (application program command) - consumed but ignored
    ApcDispatch(Vec<u8>),

    /// PM sequence (privacy message) - consumed but ignored
    PmDispatch(Vec<u8>),

    /// SOS sequence (start of string) - consumed but ignored
    SosDispatch(Vec<u8>),
}

/// CSI (Control Sequence Introducer) action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CsiAction {
    /// Parameters (semicolon-separated numbers)
    pub params: Vec<u16>,
    /// Intermediate bytes (between CSI and final byte)
    pub intermediates: Vec<u8>,
    /// Final byte (determines the action)
    pub final_byte: u8,
    /// Whether this is a private sequence (starts with ?)
    pub private: bool,
}

impl CsiAction {
    /// Get parameter at index, or default value
    pub fn param(&self, index: usize, default: u16) -> u16 {
        self.params.get(index).copied().unwrap_or(default)
    }

    /// Get parameter at index, treating 0 as 1 (common for cursor movement)
    pub fn param_or_one(&self, index: usize) -> u16 {
        let p = self.param(index, 0);
        if p == 0 { 1 } else { p }
    }
}

/// ESC sequence action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscAction {
    /// Intermediate bytes
    pub intermediates: Vec<u8>,
    /// Final byte
    pub final_byte: u8,
}

/// OSC (Operating System Command) action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OscAction {
    /// OSC command number
    pub command: u16,
    /// Payload (after the semicolon)
    pub payload: String,
}

/// DCS (Device Control String) action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DcsAction {
    /// Parameters
    pub params: Vec<u16>,
    /// Intermediate bytes
    pub intermediates: Vec<u8>,
    /// Final byte
    pub final_byte: u8,
    /// Data payload
    pub data: Vec<u8>,
}

/// DCS hook (start of DCS sequence)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DcsHook {
    /// Parameters
    pub params: Vec<u16>,
    /// Intermediate bytes
    pub intermediates: Vec<u8>,
    /// Final byte
    pub final_byte: u8,
}

/// C0 control characters
pub mod c0 {
    pub const NUL: u8 = 0x00;
    pub const SOH: u8 = 0x01;
    pub const STX: u8 = 0x02;
    pub const ETX: u8 = 0x03;
    pub const EOT: u8 = 0x04;
    pub const ENQ: u8 = 0x05;
    pub const ACK: u8 = 0x06;
    pub const BEL: u8 = 0x07;
    pub const BS: u8 = 0x08;
    pub const HT: u8 = 0x09;
    pub const LF: u8 = 0x0A;
    pub const VT: u8 = 0x0B;
    pub const FF: u8 = 0x0C;
    pub const CR: u8 = 0x0D;
    pub const SO: u8 = 0x0E;
    pub const SI: u8 = 0x0F;
    pub const DLE: u8 = 0x10;
    pub const DC1: u8 = 0x11; // XON
    pub const DC2: u8 = 0x12;
    pub const DC3: u8 = 0x13; // XOFF
    pub const DC4: u8 = 0x14;
    pub const NAK: u8 = 0x15;
    pub const SYN: u8 = 0x16;
    pub const ETB: u8 = 0x17;
    pub const CAN: u8 = 0x18;
    pub const EM: u8 = 0x19;
    pub const SUB: u8 = 0x1A;
    pub const ESC: u8 = 0x1B;
    pub const FS: u8 = 0x1C;
    pub const GS: u8 = 0x1D;
    pub const RS: u8 = 0x1E;
    pub const US: u8 = 0x1F;
    pub const DEL: u8 = 0x7F;
}

/// C1 control characters (8-bit)
pub mod c1 {
    pub const PAD: u8 = 0x80;
    pub const HOP: u8 = 0x81;
    pub const BPH: u8 = 0x82;
    pub const NBH: u8 = 0x83;
    pub const IND: u8 = 0x84;
    pub const NEL: u8 = 0x85;
    pub const SSA: u8 = 0x86;
    pub const ESA: u8 = 0x87;
    pub const HTS: u8 = 0x88;
    pub const HTJ: u8 = 0x89;
    pub const VTS: u8 = 0x8A;
    pub const PLD: u8 = 0x8B;
    pub const PLU: u8 = 0x8C;
    pub const RI: u8 = 0x8D;
    pub const SS2: u8 = 0x8E;
    pub const SS3: u8 = 0x8F;
    pub const DCS: u8 = 0x90;
    pub const PU1: u8 = 0x91;
    pub const PU2: u8 = 0x92;
    pub const STS: u8 = 0x93;
    pub const CCH: u8 = 0x94;
    pub const MW: u8 = 0x95;
    pub const SPA: u8 = 0x96;
    pub const EPA: u8 = 0x97;
    pub const SOS: u8 = 0x98;
    pub const SGCI: u8 = 0x99;
    pub const SCI: u8 = 0x9A;
    pub const CSI: u8 = 0x9B;
    pub const ST: u8 = 0x9C;
    pub const OSC: u8 = 0x9D;
    pub const PM: u8 = 0x9E;
    pub const APC: u8 = 0x9F;
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
        assert_eq!(csi.param(2, 99), 99);
    }

    #[test]
    fn test_csi_param_or_one() {
        let csi = CsiAction {
            params: vec![0, 5],
            intermediates: vec![],
            final_byte: b'A',
            private: false,
        };

        assert_eq!(csi.param_or_one(0), 1); // 0 becomes 1
        assert_eq!(csi.param_or_one(1), 5);
        assert_eq!(csi.param_or_one(2), 1); // missing becomes 1
    }
}
