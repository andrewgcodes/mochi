//! Terminal actions produced by the parser.
//!
//! These represent the semantic meaning of parsed escape sequences
//! and control characters.

use crate::params::Params;

#[derive(Debug, Clone)]
pub enum Action {
    Print(char),

    Execute(u8),

    CsiDispatch {
        params: Params,
        intermediates: Vec<u8>,
        final_byte: u8,
        private_marker: Option<u8>,
    },

    EscDispatch {
        intermediates: Vec<u8>,
        final_byte: u8,
    },

    OscDispatch {
        command: u16,
        payload: String,
    },

    DcsDispatch {
        params: Params,
        intermediates: Vec<u8>,
        final_byte: u8,
        payload: Vec<u8>,
    },

    ApcDispatch {
        payload: Vec<u8>,
    },

    PmDispatch {
        payload: Vec<u8>,
    },

    SosDispatch {
        payload: Vec<u8>,
    },
}

impl Action {
    pub fn is_print(&self) -> bool {
        matches!(self, Action::Print(_))
    }

    pub fn is_csi(&self) -> bool {
        matches!(self, Action::CsiDispatch { .. })
    }

    pub fn is_osc(&self) -> bool {
        matches!(self, Action::OscDispatch { .. })
    }
}

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
    pub const DC1: u8 = 0x11;
    pub const DC2: u8 = 0x12;
    pub const DC3: u8 = 0x13;
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
    fn test_action_variants() {
        let print = Action::Print('A');
        assert!(print.is_print());
        assert!(!print.is_csi());

        let csi = Action::CsiDispatch {
            params: Params::new(),
            intermediates: vec![],
            final_byte: b'H',
            private_marker: None,
        };
        assert!(csi.is_csi());
        assert!(!csi.is_print());
    }
}
