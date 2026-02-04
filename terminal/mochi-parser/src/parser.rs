//! VT/xterm escape sequence parser.
//!
//! Implements a state machine that parses terminal escape sequences
//! according to the ECMA-48 standard and xterm extensions.
//!
//! The parser is designed to:
//! - Handle arbitrary chunk boundaries (streaming)
//! - Be deterministic
//! - Not crash on malformed input
//! - Support UTF-8 text
//!
//! References:
//! - ECMA-48: https://ecma-international.org/wp-content/uploads/ECMA-48_5th_edition_june_1991.pdf
//! - XTerm Control Sequences: https://invisible-island.net/xterm/ctlseqs/ctlseqs.pdf

use crate::action::{c0, c1, Action};
use crate::params::Params;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Ground,
    Escape,
    EscapeIntermediate,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    CsiIgnore,
    OscString,
    DcsEntry,
    DcsParam,
    DcsIntermediate,
    DcsPassthrough,
    DcsIgnore,
    SosPmApcString,
}

const MAX_INTERMEDIATES: usize = 4;
const MAX_OSC_PAYLOAD: usize = 65536;
const MAX_DCS_PAYLOAD: usize = 65536;

pub struct Parser {
    state: State,
    params: Params,
    intermediates: Vec<u8>,
    private_marker: Option<u8>,
    osc_payload: String,
    osc_command: u16,
    dcs_payload: Vec<u8>,
    sos_payload: Vec<u8>,
    utf8_buffer: Vec<u8>,
    utf8_remaining: usize,
    preceding_byte: Option<u8>,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            state: State::Ground,
            params: Params::new(),
            intermediates: Vec::with_capacity(MAX_INTERMEDIATES),
            private_marker: None,
            osc_payload: String::new(),
            osc_command: 0,
            dcs_payload: Vec::new(),
            sos_payload: Vec::new(),
            utf8_buffer: Vec::with_capacity(4),
            utf8_remaining: 0,
            preceding_byte: None,
        }
    }

    pub fn parse<F>(&mut self, input: &[u8], mut callback: F)
    where
        F: FnMut(Action),
    {
        for &byte in input {
            self.advance(byte, &mut callback);
        }
    }

    fn advance<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        if self.utf8_remaining > 0 {
            if byte & 0xC0 == 0x80 {
                self.utf8_buffer.push(byte);
                self.utf8_remaining -= 1;
                if self.utf8_remaining == 0 {
                    if let Ok(s) = std::str::from_utf8(&self.utf8_buffer) {
                        for c in s.chars() {
                            callback(Action::Print(c));
                        }
                    } else {
                        callback(Action::Print('\u{FFFD}'));
                    }
                    self.utf8_buffer.clear();
                }
                return;
            } else {
                callback(Action::Print('\u{FFFD}'));
                self.utf8_buffer.clear();
                self.utf8_remaining = 0;
            }
        }

        if self.state == State::Ground && byte >= 0x80 && byte < 0xA0 {
            self.handle_c1(byte, callback);
            return;
        }

        if self.state == State::Ground && byte >= 0xC0 {
            let (remaining, valid) = match byte {
                0xC0..=0xC1 => (0, false),
                0xC2..=0xDF => (1, true),
                0xE0..=0xEF => (2, true),
                0xF0..=0xF4 => (3, true),
                0xF5..=0xFF => (0, false),
                _ => (0, false),
            };
            
            if valid && remaining > 0 {
                self.utf8_buffer.clear();
                self.utf8_buffer.push(byte);
                self.utf8_remaining = remaining;
                return;
            } else if !valid {
                callback(Action::Print('\u{FFFD}'));
                return;
            }
        }

        match self.state {
            State::Ground => self.ground(byte, callback),
            State::Escape => self.escape(byte, callback),
            State::EscapeIntermediate => self.escape_intermediate(byte, callback),
            State::CsiEntry => self.csi_entry(byte, callback),
            State::CsiParam => self.csi_param(byte, callback),
            State::CsiIntermediate => self.csi_intermediate(byte, callback),
            State::CsiIgnore => self.csi_ignore(byte, callback),
            State::OscString => self.osc_string(byte, callback),
            State::DcsEntry => self.dcs_entry(byte, callback),
            State::DcsParam => self.dcs_param(byte, callback),
            State::DcsIntermediate => self.dcs_intermediate(byte, callback),
            State::DcsPassthrough => self.dcs_passthrough(byte, callback),
            State::DcsIgnore => self.dcs_ignore(byte, callback),
            State::SosPmApcString => self.sos_pm_apc_string(byte, callback),
        }
        
        self.preceding_byte = Some(byte);
    }

    fn handle_c1<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            c1::CSI => {
                self.clear();
                self.state = State::CsiEntry;
            }
            c1::OSC => {
                self.clear();
                self.osc_payload.clear();
                self.osc_command = 0;
                self.state = State::OscString;
            }
            c1::DCS => {
                self.clear();
                self.dcs_payload.clear();
                self.state = State::DcsEntry;
            }
            c1::SOS | c1::PM | c1::APC => {
                self.sos_payload.clear();
                self.state = State::SosPmApcString;
            }
            c1::ST => {
                self.state = State::Ground;
            }
            c1::IND => {
                callback(Action::EscDispatch {
                    intermediates: vec![],
                    final_byte: b'D',
                });
            }
            c1::NEL => {
                callback(Action::EscDispatch {
                    intermediates: vec![],
                    final_byte: b'E',
                });
            }
            c1::HTS => {
                callback(Action::EscDispatch {
                    intermediates: vec![],
                    final_byte: b'H',
                });
            }
            c1::RI => {
                callback(Action::EscDispatch {
                    intermediates: vec![],
                    final_byte: b'M',
                });
            }
            _ => {}
        }
    }

    fn ground<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            0x00..=0x1A | 0x1C..=0x1F => {
                callback(Action::Execute(byte));
            }
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            0x20..=0x7E => {
                callback(Action::Print(byte as char));
            }
            c0::DEL => {}
            0x80..=0x9F => {
                self.handle_c1(byte, callback);
            }
            0xA0..=0xFF => {
                callback(Action::Print(byte as char));
            }
        }
    }

    fn escape<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                callback(Action::Execute(byte));
            }
            c0::CAN | c0::SUB => {
                self.state = State::Ground;
            }
            c0::ESC => {
                self.clear();
            }
            0x20..=0x2F => {
                self.collect(byte);
                self.state = State::EscapeIntermediate;
            }
            b'[' => {
                self.clear();
                self.state = State::CsiEntry;
            }
            b']' => {
                self.osc_payload.clear();
                self.osc_command = 0;
                self.state = State::OscString;
            }
            b'P' => {
                self.clear();
                self.dcs_payload.clear();
                self.state = State::DcsEntry;
            }
            b'X' | b'^' | b'_' => {
                self.sos_payload.clear();
                self.state = State::SosPmApcString;
            }
            0x30..=0x4F | 0x51..=0x57 | 0x59 | 0x5A | 0x5C | 0x60..=0x7E => {
                callback(Action::EscDispatch {
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.state = State::Ground;
            }
            c0::DEL => {}
            _ => {
                self.state = State::Ground;
            }
        }
    }

    fn escape_intermediate<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                callback(Action::Execute(byte));
            }
            c0::CAN | c0::SUB => {
                self.state = State::Ground;
            }
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            0x20..=0x2F => {
                self.collect(byte);
            }
            0x30..=0x7E => {
                callback(Action::EscDispatch {
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.state = State::Ground;
            }
            c0::DEL => {}
            _ => {
                self.state = State::Ground;
            }
        }
    }

    fn csi_entry<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                callback(Action::Execute(byte));
            }
            c0::CAN | c0::SUB => {
                self.state = State::Ground;
            }
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            b'<' | b'=' | b'>' | b'?' => {
                self.private_marker = Some(byte);
                self.state = State::CsiParam;
            }
            0x30..=0x39 => {
                self.param(byte);
                self.state = State::CsiParam;
            }
            b';' => {
                self.params.push(0);
                self.state = State::CsiParam;
            }
            b':' => {
                self.params.push(0);
                self.state = State::CsiParam;
            }
            0x20..=0x2F => {
                self.collect(byte);
                self.state = State::CsiIntermediate;
            }
            0x40..=0x7E => {
                self.csi_dispatch(byte, callback);
                self.state = State::Ground;
            }
            c0::DEL => {}
            _ => {
                self.state = State::Ground;
            }
        }
    }

    fn csi_param<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                callback(Action::Execute(byte));
            }
            c0::CAN | c0::SUB => {
                self.state = State::Ground;
            }
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            0x30..=0x39 => {
                self.param(byte);
            }
            b';' => {
                self.params.push(0);
            }
            b':' => {
                self.params.push_subparam(0);
            }
            b'<' | b'=' | b'>' | b'?' => {
                self.state = State::CsiIgnore;
            }
            0x20..=0x2F => {
                self.collect(byte);
                self.state = State::CsiIntermediate;
            }
            0x40..=0x7E => {
                self.csi_dispatch(byte, callback);
                self.state = State::Ground;
            }
            c0::DEL => {}
            _ => {
                self.state = State::CsiIgnore;
            }
        }
    }

    fn csi_intermediate<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                callback(Action::Execute(byte));
            }
            c0::CAN | c0::SUB => {
                self.state = State::Ground;
            }
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            0x20..=0x2F => {
                self.collect(byte);
            }
            0x30..=0x3F => {
                self.state = State::CsiIgnore;
            }
            0x40..=0x7E => {
                self.csi_dispatch(byte, callback);
                self.state = State::Ground;
            }
            c0::DEL => {}
            _ => {
                self.state = State::CsiIgnore;
            }
        }
    }

    fn csi_ignore<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                callback(Action::Execute(byte));
            }
            c0::CAN | c0::SUB => {
                self.state = State::Ground;
            }
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            0x40..=0x7E => {
                self.state = State::Ground;
            }
            c0::DEL => {}
            _ => {}
        }
    }

    fn osc_string<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            c0::BEL => {
                self.osc_dispatch(callback);
                self.state = State::Ground;
            }
            c0::ESC => {
                self.preceding_byte = Some(byte);
            }
            c0::CAN | c0::SUB => {
                self.state = State::Ground;
            }
            0x20..=0x7E | 0xA0..=0xFF => {
                if self.preceding_byte == Some(c0::ESC) && byte == b'\\' {
                    self.osc_dispatch(callback);
                    self.state = State::Ground;
                } else {
                    if self.osc_payload.is_empty() && byte >= b'0' && byte <= b'9' {
                        self.osc_command = self.osc_command * 10 + (byte - b'0') as u16;
                    } else if self.osc_payload.is_empty() && byte == b';' {
                    } else if self.osc_payload.len() < MAX_OSC_PAYLOAD {
                        self.osc_payload.push(byte as char);
                    }
                }
            }
            c1::ST => {
                self.osc_dispatch(callback);
                self.state = State::Ground;
            }
            _ => {}
        }
    }

    fn dcs_entry<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {}
            c0::CAN | c0::SUB => {
                self.state = State::Ground;
            }
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            b'<' | b'=' | b'>' | b'?' => {
                self.private_marker = Some(byte);
                self.state = State::DcsParam;
            }
            0x30..=0x39 => {
                self.param(byte);
                self.state = State::DcsParam;
            }
            b';' => {
                self.params.push(0);
                self.state = State::DcsParam;
            }
            0x20..=0x2F => {
                self.collect(byte);
                self.state = State::DcsIntermediate;
            }
            0x40..=0x7E => {
                self.state = State::DcsPassthrough;
            }
            c0::DEL => {}
            _ => {
                self.state = State::DcsIgnore;
            }
        }
    }

    fn dcs_param<F>(&mut self, byte: u8, _callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {}
            c0::CAN | c0::SUB => {
                self.state = State::Ground;
            }
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            0x30..=0x39 => {
                self.param(byte);
            }
            b';' => {
                self.params.push(0);
            }
            b'<' | b'=' | b'>' | b'?' => {
                self.state = State::DcsIgnore;
            }
            0x20..=0x2F => {
                self.collect(byte);
                self.state = State::DcsIntermediate;
            }
            0x40..=0x7E => {
                self.state = State::DcsPassthrough;
            }
            c0::DEL => {}
            _ => {
                self.state = State::DcsIgnore;
            }
        }
    }

    fn dcs_intermediate<F>(&mut self, byte: u8, _callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {}
            c0::CAN | c0::SUB => {
                self.state = State::Ground;
            }
            c0::ESC => {
                self.clear();
                self.state = State::Escape;
            }
            0x20..=0x2F => {
                self.collect(byte);
            }
            0x30..=0x3F => {
                self.state = State::DcsIgnore;
            }
            0x40..=0x7E => {
                self.state = State::DcsPassthrough;
            }
            c0::DEL => {}
            _ => {
                self.state = State::DcsIgnore;
            }
        }
    }

    fn dcs_passthrough<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            c0::CAN | c0::SUB => {
                self.state = State::Ground;
            }
            c0::ESC => {
                self.preceding_byte = Some(byte);
            }
            c1::ST => {
                self.dcs_dispatch(callback);
                self.state = State::Ground;
            }
            0x00..=0x17 | 0x19 | 0x1C..=0x1F | 0x20..=0x7E => {
                if self.preceding_byte == Some(c0::ESC) && byte == b'\\' {
                    self.dcs_dispatch(callback);
                    self.state = State::Ground;
                } else if self.dcs_payload.len() < MAX_DCS_PAYLOAD {
                    self.dcs_payload.push(byte);
                }
            }
            c0::DEL => {}
            _ => {
                if self.dcs_payload.len() < MAX_DCS_PAYLOAD {
                    self.dcs_payload.push(byte);
                }
            }
        }
    }

    fn dcs_ignore<F>(&mut self, byte: u8, _callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            c0::CAN | c0::SUB => {
                self.state = State::Ground;
            }
            c0::ESC => {
                self.preceding_byte = Some(byte);
            }
            c1::ST => {
                self.state = State::Ground;
            }
            _ => {
                if self.preceding_byte == Some(c0::ESC) && byte == b'\\' {
                    self.state = State::Ground;
                }
            }
        }
    }

    fn sos_pm_apc_string<F>(&mut self, byte: u8, _callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            c0::CAN | c0::SUB => {
                self.state = State::Ground;
            }
            c0::ESC => {
                self.preceding_byte = Some(byte);
            }
            c1::ST => {
                self.state = State::Ground;
            }
            _ => {
                if self.preceding_byte == Some(c0::ESC) && byte == b'\\' {
                    self.state = State::Ground;
                } else {
                    self.sos_payload.push(byte);
                }
            }
        }
    }

    fn clear(&mut self) {
        self.params.clear();
        self.intermediates.clear();
        self.private_marker = None;
    }

    fn collect(&mut self, byte: u8) {
        if self.intermediates.len() < MAX_INTERMEDIATES {
            self.intermediates.push(byte);
        }
    }

    fn param(&mut self, byte: u8) {
        let digit = (byte - b'0') as u16;
        if self.params.is_empty() {
            self.params.push(digit);
        } else {
            let idx = self.params.len() - 1;
            if let Some(current) = self.params.get(idx) {
                let new_val = current.saturating_mul(10).saturating_add(digit);
                self.params.push(0);
                self.params = {
                    let mut p = Params::new();
                    for i in 0..idx {
                        if let Some(v) = self.params.get(i) {
                            p.push(v);
                        }
                    }
                    p.push(new_val);
                    p
                };
            }
        }
    }

    fn csi_dispatch<F>(&mut self, final_byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        callback(Action::CsiDispatch {
            params: self.params.clone(),
            intermediates: self.intermediates.clone(),
            final_byte,
            private_marker: self.private_marker,
        });
    }

    fn osc_dispatch<F>(&mut self, callback: &mut F)
    where
        F: FnMut(Action),
    {
        callback(Action::OscDispatch {
            command: self.osc_command,
            payload: self.osc_payload.clone(),
        });
    }

    fn dcs_dispatch<F>(&mut self, callback: &mut F)
    where
        F: FnMut(Action),
    {
        callback(Action::DcsDispatch {
            params: self.params.clone(),
            intermediates: self.intermediates.clone(),
            final_byte: 0,
            payload: self.dcs_payload.clone(),
        });
    }

    pub fn reset(&mut self) {
        self.state = State::Ground;
        self.clear();
        self.osc_payload.clear();
        self.osc_command = 0;
        self.dcs_payload.clear();
        self.sos_payload.clear();
        self.utf8_buffer.clear();
        self.utf8_remaining = 0;
        self.preceding_byte = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_all(input: &[u8]) -> Vec<Action> {
        let mut parser = Parser::new();
        let mut actions = Vec::new();
        parser.parse(input, |action| actions.push(action));
        actions
    }

    #[test]
    fn test_print_ascii() {
        let actions = parse_all(b"Hello");
        assert_eq!(actions.len(), 5);
        assert!(matches!(actions[0], Action::Print('H')));
        assert!(matches!(actions[4], Action::Print('o')));
    }

    #[test]
    fn test_control_characters() {
        let actions = parse_all(b"\x07\x08\x09\x0A\x0D");
        assert_eq!(actions.len(), 5);
        assert!(matches!(actions[0], Action::Execute(0x07)));
        assert!(matches!(actions[1], Action::Execute(0x08)));
        assert!(matches!(actions[2], Action::Execute(0x09)));
        assert!(matches!(actions[3], Action::Execute(0x0A)));
        assert!(matches!(actions[4], Action::Execute(0x0D)));
    }

    #[test]
    fn test_csi_cursor_up() {
        let actions = parse_all(b"\x1b[5A");
        assert_eq!(actions.len(), 1);
        if let Action::CsiDispatch { params, final_byte, .. } = &actions[0] {
            assert_eq!(*final_byte, b'A');
            assert_eq!(params.get(0), Some(5));
        } else {
            panic!("Expected CsiDispatch");
        }
    }

    #[test]
    fn test_csi_cursor_position() {
        let actions = parse_all(b"\x1b[10;20H");
        assert_eq!(actions.len(), 1);
        if let Action::CsiDispatch { params, final_byte, .. } = &actions[0] {
            assert_eq!(*final_byte, b'H');
            assert_eq!(params.get(0), Some(10));
            assert_eq!(params.get(1), Some(20));
        } else {
            panic!("Expected CsiDispatch");
        }
    }

    #[test]
    fn test_csi_private_mode() {
        let actions = parse_all(b"\x1b[?25h");
        assert_eq!(actions.len(), 1);
        if let Action::CsiDispatch { params, final_byte, private_marker, .. } = &actions[0] {
            assert_eq!(*final_byte, b'h');
            assert_eq!(*private_marker, Some(b'?'));
            assert_eq!(params.get(0), Some(25));
        } else {
            panic!("Expected CsiDispatch");
        }
    }

    #[test]
    fn test_csi_sgr() {
        let actions = parse_all(b"\x1b[1;31;42m");
        assert_eq!(actions.len(), 1);
        if let Action::CsiDispatch { params, final_byte, .. } = &actions[0] {
            assert_eq!(*final_byte, b'm');
            assert_eq!(params.get(0), Some(1));
            assert_eq!(params.get(1), Some(31));
            assert_eq!(params.get(2), Some(42));
        } else {
            panic!("Expected CsiDispatch");
        }
    }

    #[test]
    fn test_esc_sequence() {
        let actions = parse_all(b"\x1b7");
        assert_eq!(actions.len(), 1);
        if let Action::EscDispatch { final_byte, .. } = &actions[0] {
            assert_eq!(*final_byte, b'7');
        } else {
            panic!("Expected EscDispatch");
        }
    }

    #[test]
    fn test_osc_title() {
        let actions = parse_all(b"\x1b]0;My Title\x07");
        assert_eq!(actions.len(), 1);
        if let Action::OscDispatch { command, payload } = &actions[0] {
            assert_eq!(*command, 0);
            assert_eq!(payload, "My Title");
        } else {
            panic!("Expected OscDispatch");
        }
    }

    #[test]
    fn test_osc_with_st() {
        let actions = parse_all(b"\x1b]2;Window Title\x1b\\");
        assert_eq!(actions.len(), 1);
        if let Action::OscDispatch { command, payload } = &actions[0] {
            assert_eq!(*command, 2);
            assert_eq!(payload, "Window Title");
        } else {
            panic!("Expected OscDispatch");
        }
    }

    #[test]
    fn test_utf8_basic() {
        let actions = parse_all("Hello 世界".as_bytes());
        let chars: Vec<char> = actions.iter().filter_map(|a| {
            if let Action::Print(c) = a { Some(*c) } else { None }
        }).collect();
        assert_eq!(chars, vec!['H', 'e', 'l', 'l', 'o', ' ', '世', '界']);
    }

    #[test]
    fn test_chunk_boundary() {
        let mut parser = Parser::new();
        let mut actions = Vec::new();
        
        parser.parse(b"\x1b[", |a| actions.push(a));
        assert!(actions.is_empty());
        
        parser.parse(b"5", |a| actions.push(a));
        assert!(actions.is_empty());
        
        parser.parse(b"A", |a| actions.push(a));
        assert_eq!(actions.len(), 1);
        
        if let Action::CsiDispatch { params, final_byte, .. } = &actions[0] {
            assert_eq!(*final_byte, b'A');
            assert_eq!(params.get(0), Some(5));
        }
    }

    #[test]
    fn test_cancel_sequence() {
        let actions = parse_all(b"\x1b[\x18Hello");
        let prints: Vec<char> = actions.iter().filter_map(|a| {
            if let Action::Print(c) = a { Some(*c) } else { None }
        }).collect();
        assert_eq!(prints, vec!['H', 'e', 'l', 'l', 'o']);
    }

    #[test]
    fn test_empty_params() {
        let actions = parse_all(b"\x1b[;H");
        assert_eq!(actions.len(), 1);
        if let Action::CsiDispatch { params, final_byte, .. } = &actions[0] {
            assert_eq!(*final_byte, b'H');
            assert_eq!(params.get(0), Some(0));
        }
    }
}
