//! VT/xterm escape sequence parser
//!
//! This parser implements a state machine based on the VT500-series parser
//! described in the DEC documentation and Paul Williams' state machine diagram.
//!
//! The parser is streaming and can handle arbitrary chunk boundaries.
//! It produces Actions that represent semantic terminal operations.

use crate::action::{c0, c1, Action, CsiAction, DcsAction, DcsHook, EscAction, OscAction};
use crate::params::Params;

/// Parser state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Ground state - normal character processing
    Ground,
    /// After ESC
    Escape,
    /// After ESC [
    CsiEntry,
    /// CSI parameter bytes
    CsiParam,
    /// CSI intermediate bytes
    CsiIntermediate,
    /// CSI ignore (invalid sequence)
    CsiIgnore,
    /// After ESC ]
    OscString,
    /// After ESC P
    DcsEntry,
    /// DCS parameter bytes
    DcsParam,
    /// DCS intermediate bytes
    DcsIntermediate,
    /// DCS passthrough (data)
    DcsPassthrough,
    /// DCS ignore
    DcsIgnore,
    /// After ESC _
    ApcString,
    /// After ESC ^
    PmString,
    /// After ESC X
    SosString,
    /// Escape intermediate
    EscapeIntermediate,
    /// UTF-8 continuation bytes
    Utf8,
}

/// The VT/xterm parser
#[derive(Debug)]
pub struct Parser {
    state: State,
    /// Intermediate bytes for ESC/CSI sequences
    intermediates: Vec<u8>,
    /// Parameter bytes for CSI sequences
    params_bytes: Vec<u8>,
    /// OSC string buffer
    osc_buffer: Vec<u8>,
    /// DCS data buffer
    dcs_buffer: Vec<u8>,
    /// APC/PM/SOS buffer
    string_buffer: Vec<u8>,
    /// UTF-8 buffer for multi-byte characters
    utf8_buffer: Vec<u8>,
    /// Expected UTF-8 continuation bytes
    utf8_remaining: u8,
    /// Current UTF-8 codepoint being assembled
    utf8_codepoint: u32,
    /// Whether we're in a private CSI sequence (?)
    csi_private: bool,
    /// DCS hook info
    dcs_hook: Option<DcsHook>,
    /// Maximum buffer sizes for security
    max_osc_len: usize,
    max_dcs_len: usize,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    /// Create a new parser
    pub fn new() -> Self {
        Parser {
            state: State::Ground,
            intermediates: Vec::with_capacity(4),
            params_bytes: Vec::with_capacity(64),
            osc_buffer: Vec::with_capacity(256),
            dcs_buffer: Vec::with_capacity(256),
            string_buffer: Vec::with_capacity(256),
            utf8_buffer: Vec::with_capacity(4),
            utf8_remaining: 0,
            utf8_codepoint: 0,
            csi_private: false,
            dcs_hook: None,
            max_osc_len: 65536,
            max_dcs_len: 65536,
        }
    }

    /// Set maximum OSC length (for security)
    pub fn set_max_osc_len(&mut self, len: usize) {
        self.max_osc_len = len;
    }

    /// Set maximum DCS length (for security)
    pub fn set_max_dcs_len(&mut self, len: usize) {
        self.max_dcs_len = len;
    }

    /// Reset parser state
    pub fn reset(&mut self) {
        self.state = State::Ground;
        self.intermediates.clear();
        self.params_bytes.clear();
        self.osc_buffer.clear();
        self.dcs_buffer.clear();
        self.string_buffer.clear();
        self.utf8_buffer.clear();
        self.utf8_remaining = 0;
        self.utf8_codepoint = 0;
        self.csi_private = false;
        self.dcs_hook = None;
    }

    /// Parse a chunk of bytes and return actions
    pub fn parse(&mut self, bytes: &[u8]) -> Vec<Action> {
        let mut actions = Vec::new();
        for &byte in bytes {
            self.advance(byte, &mut actions);
        }
        actions
    }

    /// Parse a single byte
    pub fn parse_byte(&mut self, byte: u8) -> Vec<Action> {
        let mut actions = Vec::new();
        self.advance(byte, &mut actions);
        actions
    }

    /// Advance the parser state machine with a single byte
    fn advance(&mut self, byte: u8, actions: &mut Vec<Action>) {
        // Handle UTF-8 continuation in ground state
        if self.state == State::Ground && self.utf8_remaining > 0 {
            if byte & 0xC0 == 0x80 {
                // Valid continuation byte
                self.utf8_codepoint = (self.utf8_codepoint << 6) | (byte & 0x3F) as u32;
                self.utf8_remaining -= 1;
                if self.utf8_remaining == 0 {
                    // Complete UTF-8 sequence
                    if let Some(c) = char::from_u32(self.utf8_codepoint) {
                        actions.push(Action::Print(c));
                    } else {
                        // Invalid codepoint, emit replacement character
                        actions.push(Action::Print('\u{FFFD}'));
                    }
                }
                return;
            } else {
                // Invalid continuation, emit replacement and process this byte
                actions.push(Action::Print('\u{FFFD}'));
                self.utf8_remaining = 0;
            }
        }

        // Check for C1 controls (8-bit) that can interrupt any state
        if byte >= 0x80 && byte <= 0x9F {
            self.handle_c1(byte, actions);
            return;
        }

        // State-specific handling
        match self.state {
            State::Ground => self.ground(byte, actions),
            State::Escape => self.escape(byte, actions),
            State::EscapeIntermediate => self.escape_intermediate(byte, actions),
            State::CsiEntry => self.csi_entry(byte, actions),
            State::CsiParam => self.csi_param(byte, actions),
            State::CsiIntermediate => self.csi_intermediate(byte, actions),
            State::CsiIgnore => self.csi_ignore(byte, actions),
            State::OscString => self.osc_string(byte, actions),
            State::DcsEntry => self.dcs_entry(byte, actions),
            State::DcsParam => self.dcs_param(byte, actions),
            State::DcsIntermediate => self.dcs_intermediate(byte, actions),
            State::DcsPassthrough => self.dcs_passthrough(byte, actions),
            State::DcsIgnore => self.dcs_ignore(byte, actions),
            State::ApcString => self.apc_string(byte, actions),
            State::PmString => self.pm_string(byte, actions),
            State::SosString => self.sos_string(byte, actions),
            State::Utf8 => self.utf8(byte, actions),
        }
    }

    /// Handle C1 control characters (8-bit)
    fn handle_c1(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            c1::CSI => {
                self.clear();
                self.state = State::CsiEntry;
            }
            c1::OSC => {
                self.clear();
                self.state = State::OscString;
            }
            c1::DCS => {
                self.clear();
                self.state = State::DcsEntry;
            }
            c1::ST => {
                // String terminator - handled in string states
                self.state = State::Ground;
            }
            c1::IND => {
                actions.push(Action::EscDispatch(EscAction {
                    intermediates: vec![],
                    final_byte: b'D',
                }));
                self.state = State::Ground;
            }
            c1::NEL => {
                actions.push(Action::EscDispatch(EscAction {
                    intermediates: vec![],
                    final_byte: b'E',
                }));
                self.state = State::Ground;
            }
            c1::HTS => {
                actions.push(Action::EscDispatch(EscAction {
                    intermediates: vec![],
                    final_byte: b'H',
                }));
                self.state = State::Ground;
            }
            c1::RI => {
                actions.push(Action::EscDispatch(EscAction {
                    intermediates: vec![],
                    final_byte: b'M',
                }));
                self.state = State::Ground;
            }
            c1::APC => {
                self.clear();
                self.state = State::ApcString;
            }
            c1::PM => {
                self.clear();
                self.state = State::PmString;
            }
            c1::SOS => {
                self.clear();
                self.state = State::SosString;
            }
            _ => {
                // Other C1 controls are ignored
                self.state = State::Ground;
            }
        }
    }

    /// Clear parser buffers
    fn clear(&mut self) {
        self.intermediates.clear();
        self.params_bytes.clear();
        self.osc_buffer.clear();
        self.dcs_buffer.clear();
        self.string_buffer.clear();
        self.csi_private = false;
        self.dcs_hook = None;
    }

    /// Ground state - normal character processing
    fn ground(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // C0 controls
            0x00..=0x1A | 0x1C..=0x1F => {
                actions.push(Action::Execute(byte));
            }
            // ESC
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            // DEL - ignore
            0x7F => {}
            // Printable ASCII
            0x20..=0x7E => {
                actions.push(Action::Print(byte as char));
            }
            // UTF-8 start bytes
            0xC0..=0xDF => {
                // 2-byte sequence
                self.utf8_codepoint = (byte & 0x1F) as u32;
                self.utf8_remaining = 1;
            }
            0xE0..=0xEF => {
                // 3-byte sequence
                self.utf8_codepoint = (byte & 0x0F) as u32;
                self.utf8_remaining = 2;
            }
            0xF0..=0xF7 => {
                // 4-byte sequence
                self.utf8_codepoint = (byte & 0x07) as u32;
                self.utf8_remaining = 3;
            }
            // Invalid UTF-8 start bytes
            0x80..=0xBF | 0xF8..=0xFF => {
                actions.push(Action::Print('\u{FFFD}'));
            }
            _ => {}
        }
    }

    /// Escape state - after ESC
    fn escape(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // C0 controls execute immediately
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                actions.push(Action::Execute(byte));
            }
            // CAN/SUB abort
            0x18 | 0x1A => {
                self.state = State::Ground;
            }
            // ESC restarts
            c0::ESC => {
                self.clear();
            }
            // DEL ignored
            0x7F => {}
            // CSI
            b'[' => {
                self.clear();
                self.state = State::CsiEntry;
            }
            // OSC
            b']' => {
                self.clear();
                self.state = State::OscString;
            }
            // DCS
            b'P' => {
                self.clear();
                self.state = State::DcsEntry;
            }
            // APC
            b'_' => {
                self.clear();
                self.state = State::ApcString;
            }
            // PM
            b'^' => {
                self.clear();
                self.state = State::PmString;
            }
            // SOS
            b'X' => {
                self.clear();
                self.state = State::SosString;
            }
            // Intermediate bytes
            0x20..=0x2F => {
                self.intermediates.push(byte);
                self.state = State::EscapeIntermediate;
            }
            // Final bytes
            0x30..=0x4F | 0x51..=0x57 | 0x59 | 0x5A | 0x5C | 0x60..=0x7E => {
                actions.push(Action::EscDispatch(EscAction {
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                }));
                self.state = State::Ground;
            }
            _ => {
                self.state = State::Ground;
            }
        }
    }

    /// Escape intermediate state
    fn escape_intermediate(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // C0 controls execute immediately
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                actions.push(Action::Execute(byte));
            }
            // CAN/SUB abort
            0x18 | 0x1A => {
                self.state = State::Ground;
            }
            // ESC restarts
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            // DEL ignored
            0x7F => {}
            // More intermediate bytes
            0x20..=0x2F => {
                if self.intermediates.len() < 4 {
                    self.intermediates.push(byte);
                }
            }
            // Final bytes
            0x30..=0x7E => {
                actions.push(Action::EscDispatch(EscAction {
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                }));
                self.state = State::Ground;
            }
            _ => {
                self.state = State::Ground;
            }
        }
    }

    /// CSI entry state
    fn csi_entry(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // C0 controls execute immediately
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                actions.push(Action::Execute(byte));
            }
            // CAN/SUB abort
            0x18 | 0x1A => {
                self.state = State::Ground;
            }
            // ESC restarts
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            // DEL ignored
            0x7F => {}
            // Private marker
            b'?' | b'>' | b'<' | b'=' => {
                self.csi_private = byte == b'?';
                self.params_bytes.push(byte);
                self.state = State::CsiParam;
            }
            // Parameter bytes
            0x30..=0x3B => {
                self.params_bytes.push(byte);
                self.state = State::CsiParam;
            }
            // Intermediate bytes
            0x20..=0x2F => {
                self.intermediates.push(byte);
                self.state = State::CsiIntermediate;
            }
            // Final bytes - dispatch
            0x40..=0x7E => {
                self.dispatch_csi(byte, actions);
                self.state = State::Ground;
            }
            // Invalid
            0x3C..=0x3F => {
                self.state = State::CsiIgnore;
            }
            _ => {
                self.state = State::Ground;
            }
        }
    }

    /// CSI parameter state
    fn csi_param(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // C0 controls execute immediately
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                actions.push(Action::Execute(byte));
            }
            // CAN/SUB abort
            0x18 | 0x1A => {
                self.state = State::Ground;
            }
            // ESC restarts
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            // DEL ignored
            0x7F => {}
            // Parameter bytes (including : for subparameters)
            0x30..=0x3B => {
                if self.params_bytes.len() < 256 {
                    self.params_bytes.push(byte);
                }
            }
            // Intermediate bytes
            0x20..=0x2F => {
                self.intermediates.push(byte);
                self.state = State::CsiIntermediate;
            }
            // Final bytes - dispatch
            0x40..=0x7E => {
                self.dispatch_csi(byte, actions);
                self.state = State::Ground;
            }
            // Invalid - go to ignore
            0x3C..=0x3F => {
                self.state = State::CsiIgnore;
            }
            _ => {
                self.state = State::Ground;
            }
        }
    }

    /// CSI intermediate state
    fn csi_intermediate(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // C0 controls execute immediately
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                actions.push(Action::Execute(byte));
            }
            // CAN/SUB abort
            0x18 | 0x1A => {
                self.state = State::Ground;
            }
            // ESC restarts
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            // DEL ignored
            0x7F => {}
            // More intermediate bytes
            0x20..=0x2F => {
                if self.intermediates.len() < 4 {
                    self.intermediates.push(byte);
                }
            }
            // Final bytes - dispatch
            0x40..=0x7E => {
                self.dispatch_csi(byte, actions);
                self.state = State::Ground;
            }
            // Invalid
            0x30..=0x3F => {
                self.state = State::CsiIgnore;
            }
            _ => {
                self.state = State::Ground;
            }
        }
    }

    /// CSI ignore state
    fn csi_ignore(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // C0 controls execute immediately
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                actions.push(Action::Execute(byte));
            }
            // CAN/SUB abort
            0x18 | 0x1A => {
                self.state = State::Ground;
            }
            // ESC restarts
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            // Final bytes end ignore
            0x40..=0x7E => {
                self.state = State::Ground;
            }
            _ => {}
        }
    }

    /// Dispatch a CSI sequence
    fn dispatch_csi(&mut self, final_byte: u8, actions: &mut Vec<Action>) {
        // Skip the private marker if present
        let params_start = if !self.params_bytes.is_empty()
            && matches!(self.params_bytes[0], b'?' | b'>' | b'<' | b'=')
        {
            1
        } else {
            0
        };

        let params = Params::parse(&self.params_bytes[params_start..]);

        actions.push(Action::CsiDispatch(CsiAction {
            params: params.to_vec(),
            intermediates: self.intermediates.clone(),
            final_byte,
            private: self.csi_private,
        }));
    }

    /// OSC string state
    fn osc_string(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // BEL terminates OSC (xterm extension)
            c0::BEL => {
                self.dispatch_osc(actions);
                self.state = State::Ground;
            }
            // ESC might start ST
            c0::ESC => {
                // Check for ST (ESC \) in next byte
                // For now, we'll handle this by checking in the next call
                self.osc_buffer.push(byte);
            }
            // ST (0x9C) terminates
            0x9C => {
                self.dispatch_osc(actions);
                self.state = State::Ground;
            }
            // CAN/SUB abort
            0x18 | 0x1A => {
                self.state = State::Ground;
            }
            // Collect string data
            _ => {
                // Check for ESC \ (ST)
                if !self.osc_buffer.is_empty() && self.osc_buffer.last() == Some(&c0::ESC) && byte == b'\\' {
                    self.osc_buffer.pop(); // Remove the ESC
                    self.dispatch_osc(actions);
                    self.state = State::Ground;
                } else if self.osc_buffer.len() < self.max_osc_len {
                    self.osc_buffer.push(byte);
                }
            }
        }
    }

    /// Dispatch an OSC sequence
    fn dispatch_osc(&mut self, actions: &mut Vec<Action>) {
        // Parse OSC: command;payload
        let mut command = 0u16;
        let mut payload_start = 0;

        for (i, &byte) in self.osc_buffer.iter().enumerate() {
            if byte == b';' {
                payload_start = i + 1;
                break;
            } else if byte.is_ascii_digit() {
                command = command.saturating_mul(10).saturating_add((byte - b'0') as u16);
            }
            payload_start = i + 1;
        }

        let payload = if payload_start < self.osc_buffer.len() {
            String::from_utf8_lossy(&self.osc_buffer[payload_start..]).to_string()
        } else {
            String::new()
        };

        actions.push(Action::OscDispatch(OscAction { command, payload }));
    }

    /// DCS entry state
    fn dcs_entry(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // CAN/SUB abort
            0x18 | 0x1A => {
                self.state = State::Ground;
            }
            // ESC restarts
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            // DEL ignored
            0x7F => {}
            // Parameter bytes
            0x30..=0x3B => {
                self.params_bytes.push(byte);
                self.state = State::DcsParam;
            }
            // Intermediate bytes
            0x20..=0x2F => {
                self.intermediates.push(byte);
                self.state = State::DcsIntermediate;
            }
            // Final bytes - start passthrough
            0x40..=0x7E => {
                self.dcs_hook(byte, actions);
                self.state = State::DcsPassthrough;
            }
            // Invalid
            0x3C..=0x3F => {
                self.state = State::DcsIgnore;
            }
            _ => {}
        }
    }

    /// DCS parameter state
    fn dcs_param(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // CAN/SUB abort
            0x18 | 0x1A => {
                self.state = State::Ground;
            }
            // ESC restarts
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            // DEL ignored
            0x7F => {}
            // Parameter bytes
            0x30..=0x3B => {
                if self.params_bytes.len() < 256 {
                    self.params_bytes.push(byte);
                }
            }
            // Intermediate bytes
            0x20..=0x2F => {
                self.intermediates.push(byte);
                self.state = State::DcsIntermediate;
            }
            // Final bytes - start passthrough
            0x40..=0x7E => {
                self.dcs_hook(byte, actions);
                self.state = State::DcsPassthrough;
            }
            // Invalid
            0x3C..=0x3F => {
                self.state = State::DcsIgnore;
            }
            _ => {}
        }
    }

    /// DCS intermediate state
    fn dcs_intermediate(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // CAN/SUB abort
            0x18 | 0x1A => {
                self.state = State::Ground;
            }
            // ESC restarts
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            // DEL ignored
            0x7F => {}
            // More intermediate bytes
            0x20..=0x2F => {
                if self.intermediates.len() < 4 {
                    self.intermediates.push(byte);
                }
            }
            // Final bytes - start passthrough
            0x40..=0x7E => {
                self.dcs_hook(byte, actions);
                self.state = State::DcsPassthrough;
            }
            // Invalid
            0x30..=0x3F => {
                self.state = State::DcsIgnore;
            }
            _ => {}
        }
    }

    /// DCS hook - start of DCS data
    fn dcs_hook(&mut self, final_byte: u8, actions: &mut Vec<Action>) {
        let params = Params::parse(&self.params_bytes);
        let hook = DcsHook {
            params: params.to_vec(),
            intermediates: self.intermediates.clone(),
            final_byte,
        };
        self.dcs_hook = Some(hook.clone());
        actions.push(Action::DcsHook(hook));
    }

    /// DCS passthrough state
    fn dcs_passthrough(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // ST terminates
            0x9C => {
                self.dcs_unhook(actions);
                self.state = State::Ground;
            }
            // ESC might start ST
            c0::ESC => {
                self.dcs_buffer.push(byte);
            }
            // CAN/SUB abort
            0x18 | 0x1A => {
                self.dcs_unhook(actions);
                self.state = State::Ground;
            }
            // Collect data
            _ => {
                // Check for ESC \ (ST)
                if !self.dcs_buffer.is_empty() && self.dcs_buffer.last() == Some(&c0::ESC) && byte == b'\\' {
                    self.dcs_buffer.pop(); // Remove the ESC
                    self.dcs_unhook(actions);
                    self.state = State::Ground;
                } else if self.dcs_buffer.len() < self.max_dcs_len {
                    self.dcs_buffer.push(byte);
                    actions.push(Action::DcsPut(byte));
                }
            }
        }
    }

    /// DCS unhook - end of DCS data
    fn dcs_unhook(&mut self, actions: &mut Vec<Action>) {
        actions.push(Action::DcsUnhook);
        self.dcs_hook = None;
    }

    /// DCS ignore state
    fn dcs_ignore(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // ST terminates
            0x9C => {
                self.state = State::Ground;
            }
            // ESC might start ST
            c0::ESC => {
                self.string_buffer.push(byte);
            }
            // CAN/SUB abort
            0x18 | 0x1A => {
                self.state = State::Ground;
            }
            _ => {
                // Check for ESC \ (ST)
                if !self.string_buffer.is_empty() && self.string_buffer.last() == Some(&c0::ESC) && byte == b'\\' {
                    self.state = State::Ground;
                }
            }
        }
    }

    /// APC string state
    fn apc_string(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // ST terminates
            0x9C => {
                actions.push(Action::ApcDispatch(self.string_buffer.clone()));
                self.state = State::Ground;
            }
            // ESC might start ST
            c0::ESC => {
                self.string_buffer.push(byte);
            }
            // CAN/SUB abort
            0x18 | 0x1A => {
                self.state = State::Ground;
            }
            _ => {
                // Check for ESC \ (ST)
                if !self.string_buffer.is_empty() && self.string_buffer.last() == Some(&c0::ESC) && byte == b'\\' {
                    self.string_buffer.pop();
                    actions.push(Action::ApcDispatch(self.string_buffer.clone()));
                    self.state = State::Ground;
                } else if self.string_buffer.len() < self.max_osc_len {
                    self.string_buffer.push(byte);
                }
            }
        }
    }

    /// PM string state
    fn pm_string(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // ST terminates
            0x9C => {
                actions.push(Action::PmDispatch(self.string_buffer.clone()));
                self.state = State::Ground;
            }
            // ESC might start ST
            c0::ESC => {
                self.string_buffer.push(byte);
            }
            // CAN/SUB abort
            0x18 | 0x1A => {
                self.state = State::Ground;
            }
            _ => {
                // Check for ESC \ (ST)
                if !self.string_buffer.is_empty() && self.string_buffer.last() == Some(&c0::ESC) && byte == b'\\' {
                    self.string_buffer.pop();
                    actions.push(Action::PmDispatch(self.string_buffer.clone()));
                    self.state = State::Ground;
                } else if self.string_buffer.len() < self.max_osc_len {
                    self.string_buffer.push(byte);
                }
            }
        }
    }

    /// SOS string state
    fn sos_string(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // ST terminates
            0x9C => {
                actions.push(Action::SosDispatch(self.string_buffer.clone()));
                self.state = State::Ground;
            }
            // ESC might start ST
            c0::ESC => {
                self.string_buffer.push(byte);
            }
            // CAN/SUB abort
            0x18 | 0x1A => {
                self.state = State::Ground;
            }
            _ => {
                // Check for ESC \ (ST)
                if !self.string_buffer.is_empty() && self.string_buffer.last() == Some(&c0::ESC) && byte == b'\\' {
                    self.string_buffer.pop();
                    actions.push(Action::SosDispatch(self.string_buffer.clone()));
                    self.state = State::Ground;
                } else if self.string_buffer.len() < self.max_osc_len {
                    self.string_buffer.push(byte);
                }
            }
        }
    }

    /// UTF-8 continuation state
    fn utf8(&mut self, byte: u8, actions: &mut Vec<Action>) {
        if byte & 0xC0 == 0x80 {
            // Valid continuation byte
            self.utf8_buffer.push(byte);
            self.utf8_remaining -= 1;
            if self.utf8_remaining == 0 {
                // Complete UTF-8 sequence
                if let Ok(s) = std::str::from_utf8(&self.utf8_buffer) {
                    for c in s.chars() {
                        actions.push(Action::Print(c));
                    }
                } else {
                    actions.push(Action::Print('\u{FFFD}'));
                }
                self.utf8_buffer.clear();
                self.state = State::Ground;
            }
        } else {
            // Invalid continuation, emit replacement and reprocess
            actions.push(Action::Print('\u{FFFD}'));
            self.utf8_buffer.clear();
            self.state = State::Ground;
            self.advance(byte, actions);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_ascii() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"Hello");
        assert_eq!(actions.len(), 5);
        assert_eq!(actions[0], Action::Print('H'));
        assert_eq!(actions[4], Action::Print('o'));
    }

    #[test]
    fn test_print_utf8() {
        let mut parser = Parser::new();
        let actions = parser.parse("Hello 世界".as_bytes());
        assert!(actions.contains(&Action::Print('世')));
        assert!(actions.contains(&Action::Print('界')));
    }

    #[test]
    fn test_c0_controls() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x07\x08\x09\x0A\x0D");
        assert_eq!(actions[0], Action::Execute(c0::BEL));
        assert_eq!(actions[1], Action::Execute(c0::BS));
        assert_eq!(actions[2], Action::Execute(c0::HT));
        assert_eq!(actions[3], Action::Execute(c0::LF));
        assert_eq!(actions[4], Action::Execute(c0::CR));
    }

    #[test]
    fn test_csi_cursor_up() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b[5A");
        assert_eq!(actions.len(), 1);
        if let Action::CsiDispatch(csi) = &actions[0] {
            assert_eq!(csi.params, vec![5]);
            assert_eq!(csi.final_byte, b'A');
            assert!(!csi.private);
        } else {
            panic!("Expected CsiDispatch");
        }
    }

    #[test]
    fn test_csi_private_mode() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b[?25h");
        assert_eq!(actions.len(), 1);
        if let Action::CsiDispatch(csi) = &actions[0] {
            assert_eq!(csi.params, vec![25]);
            assert_eq!(csi.final_byte, b'h');
            assert!(csi.private);
        } else {
            panic!("Expected CsiDispatch");
        }
    }

    #[test]
    fn test_csi_sgr() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b[1;31;42m");
        assert_eq!(actions.len(), 1);
        if let Action::CsiDispatch(csi) = &actions[0] {
            assert_eq!(csi.params, vec![1, 31, 42]);
            assert_eq!(csi.final_byte, b'm');
        } else {
            panic!("Expected CsiDispatch");
        }
    }

    #[test]
    fn test_csi_default_params() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b[H");
        if let Action::CsiDispatch(csi) = &actions[0] {
            assert!(csi.params.is_empty() || csi.params == vec![0]);
        } else {
            panic!("Expected CsiDispatch");
        }
    }

    #[test]
    fn test_osc_title() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b]0;My Title\x07");
        assert_eq!(actions.len(), 1);
        if let Action::OscDispatch(osc) = &actions[0] {
            assert_eq!(osc.command, 0);
            assert_eq!(osc.payload, "My Title");
        } else {
            panic!("Expected OscDispatch");
        }
    }

    #[test]
    fn test_osc_with_st() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b]2;Title\x1b\\");
        assert_eq!(actions.len(), 1);
        if let Action::OscDispatch(osc) = &actions[0] {
            assert_eq!(osc.command, 2);
            assert_eq!(osc.payload, "Title");
        } else {
            panic!("Expected OscDispatch");
        }
    }

    #[test]
    fn test_esc_save_cursor() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b7");
        assert_eq!(actions.len(), 1);
        if let Action::EscDispatch(esc) = &actions[0] {
            assert_eq!(esc.final_byte, b'7');
            assert!(esc.intermediates.is_empty());
        } else {
            panic!("Expected EscDispatch");
        }
    }

    #[test]
    fn test_esc_charset() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b(0");
        assert_eq!(actions.len(), 1);
        if let Action::EscDispatch(esc) = &actions[0] {
            assert_eq!(esc.intermediates, vec![b'(']);
            assert_eq!(esc.final_byte, b'0');
        } else {
            panic!("Expected EscDispatch");
        }
    }

    #[test]
    fn test_chunk_boundary() {
        let mut parser = Parser::new();

        // Split CSI sequence across chunks
        let actions1 = parser.parse(b"\x1b[");
        assert!(actions1.is_empty());

        let actions2 = parser.parse(b"5A");
        assert_eq!(actions2.len(), 1);
        if let Action::CsiDispatch(csi) = &actions2[0] {
            assert_eq!(csi.params, vec![5]);
            assert_eq!(csi.final_byte, b'A');
        } else {
            panic!("Expected CsiDispatch");
        }
    }

    #[test]
    fn test_utf8_chunk_boundary() {
        let mut parser = Parser::new();

        // UTF-8 for '世' is E4 B8 96
        let actions1 = parser.parse(&[0xE4]);
        assert!(actions1.is_empty());

        let actions2 = parser.parse(&[0xB8, 0x96]);
        assert_eq!(actions2.len(), 1);
        assert_eq!(actions2[0], Action::Print('世'));
    }

    #[test]
    fn test_invalid_utf8() {
        let mut parser = Parser::new();
        let actions = parser.parse(&[0xFF, 0xFE]);
        // Should produce replacement characters
        assert!(actions.iter().all(|a| matches!(a, Action::Print('\u{FFFD}'))));
    }

    #[test]
    fn test_can_aborts_sequence() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b[5\x18A");
        // CAN should abort the CSI, then 'A' is printed
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], Action::Print('A'));
    }
}
