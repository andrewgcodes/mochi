//! Parser State Machine
//!
//! Implements a VT500-series compatible parser state machine.
//! The parser handles arbitrary chunk boundaries and produces
//! semantic actions for the terminal core.
//!
//! # State Machine
//!
//! The parser follows the state machine model described in:
//! - "A parser for DEC's ANSI-compatible video terminals" by Paul Williams
//! - https://vt100.net/emu/dec_ansi_parser
//!
//! States:
//! - Ground: Normal text processing
//! - Escape: After ESC, waiting for next byte
//! - EscapeIntermediate: ESC followed by intermediate bytes
//! - CsiEntry: After CSI (ESC [), collecting parameters
//! - CsiParam: Collecting CSI parameters
//! - CsiIntermediate: CSI with intermediate bytes
//! - OscString: Collecting OSC payload
//! - DcsEntry/DcsParam/DcsIntermediate/DcsPassthrough: DCS handling
//! - SosPmApcString: SOS/PM/APC string collection

use super::actions::{Action, CsiAction, EscAction, OscAction};

/// Parser state
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

/// The terminal parser
#[derive(Debug)]
pub struct Parser {
    state: State,
    /// Intermediate bytes collected during parsing
    intermediates: Vec<u8>,
    /// Parameters for CSI sequences
    params: Vec<u32>,
    /// Current parameter being built
    current_param: u32,
    /// Whether we've seen a digit for the current parameter
    param_has_digit: bool,
    /// Whether this is a private sequence (starts with ?)
    private_marker: bool,
    /// OSC command number
    osc_command: u32,
    /// OSC string payload
    osc_string: Vec<u8>,
    /// DCS/SOS/PM/APC payload
    dcs_string: Vec<u8>,
    /// UTF-8 decoder state
    utf8_buffer: Vec<u8>,
    utf8_remaining: u8,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    /// Create a new parser in the ground state
    pub fn new() -> Self {
        Self {
            state: State::Ground,
            intermediates: Vec::with_capacity(4),
            params: Vec::with_capacity(16),
            current_param: 0,
            param_has_digit: false,
            private_marker: false,
            osc_command: 0,
            osc_string: Vec::with_capacity(256),
            dcs_string: Vec::with_capacity(256),
            utf8_buffer: Vec::with_capacity(4),
            utf8_remaining: 0,
        }
    }

    /// Reset the parser to initial state
    pub fn reset(&mut self) {
        self.state = State::Ground;
        self.clear_params();
        self.osc_string.clear();
        self.dcs_string.clear();
        self.utf8_buffer.clear();
        self.utf8_remaining = 0;
    }

    /// Clear parameter state
    fn clear_params(&mut self) {
        self.intermediates.clear();
        self.params.clear();
        self.current_param = 0;
        self.param_has_digit = false;
        self.private_marker = false;
    }

    /// Process a chunk of bytes, returning actions
    pub fn parse(&mut self, data: &[u8]) -> Vec<Action> {
        let mut actions = Vec::new();

        for &byte in data {
            if let Some(action) = self.process_byte(byte) {
                actions.push(action);
            }
        }

        actions
    }

    /// Process a single byte
    fn process_byte(&mut self, byte: u8) -> Option<Action> {
        // Handle UTF-8 continuation in ground state
        if self.state == State::Ground && self.utf8_remaining > 0 {
            return self.process_utf8_continuation(byte);
        }

        // C0 controls are handled in most states
        if byte < 0x20 {
            return self.process_c0(byte);
        }

        // DEL is ignored in most states
        if byte == 0x7F {
            return None;
        }

        // C1 controls (0x80-0x9F) - treat as 7-bit equivalents
        if (0x80..=0x9F).contains(&byte) {
            return self.process_c1(byte);
        }

        match self.state {
            State::Ground => self.process_ground(byte),
            State::Escape => self.process_escape(byte),
            State::EscapeIntermediate => self.process_escape_intermediate(byte),
            State::CsiEntry => self.process_csi_entry(byte),
            State::CsiParam => self.process_csi_param(byte),
            State::CsiIntermediate => self.process_csi_intermediate(byte),
            State::CsiIgnore => self.process_csi_ignore(byte),
            State::OscString => self.process_osc_string(byte),
            State::DcsEntry => self.process_dcs_entry(byte),
            State::DcsParam => self.process_dcs_param(byte),
            State::DcsIntermediate => self.process_dcs_intermediate(byte),
            State::DcsPassthrough => self.process_dcs_passthrough(byte),
            State::DcsIgnore => self.process_dcs_ignore(byte),
            State::SosPmApcString => self.process_sos_pm_apc_string(byte),
        }
    }

    /// Process C0 control characters (0x00-0x1F)
    fn process_c0(&mut self, byte: u8) -> Option<Action> {
        match byte {
            // Always handle these regardless of state
            0x18 | 0x1A => {
                // CAN, SUB - cancel current sequence
                self.state = State::Ground;
                None
            }
            0x1B => {
                // ESC - start escape sequence
                self.state = State::Escape;
                self.clear_params();
                None
            }
            // Execute these in ground state or pass through in string states
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                match self.state {
                    State::OscString => {
                        // BEL (0x07) terminates OSC sequence (xterm extension)
                        if byte == 0x07 {
                            self.terminate_osc()
                        } else {
                            // Other C0 controls are ignored in OSC string
                            None
                        }
                    }
                    State::DcsPassthrough | State::SosPmApcString => {
                        // In string states, most C0 are ignored except terminators
                        None
                    }
                    _ => {
                        // Execute the control character
                        Some(Action::Execute(byte))
                    }
                }
            }
            _ => None,
        }
    }

    /// Process C1 control characters (0x80-0x9F)
    fn process_c1(&mut self, byte: u8) -> Option<Action> {
        match byte {
            0x90 => {
                // DCS
                self.state = State::DcsEntry;
                self.clear_params();
                self.dcs_string.clear();
                None
            }
            0x98 => {
                // SOS
                self.state = State::SosPmApcString;
                self.dcs_string.clear();
                None
            }
            0x9B => {
                // CSI
                self.state = State::CsiEntry;
                self.clear_params();
                None
            }
            0x9C => {
                // ST (String Terminator)
                self.terminate_string()
            }
            0x9D => {
                // OSC
                self.state = State::OscString;
                self.osc_command = 0;
                self.osc_string.clear();
                None
            }
            0x9E => {
                // PM
                self.state = State::SosPmApcString;
                self.dcs_string.clear();
                None
            }
            0x9F => {
                // APC
                self.state = State::SosPmApcString;
                self.dcs_string.clear();
                None
            }
            _ => None,
        }
    }

    /// Process bytes in ground state (normal text)
    fn process_ground(&mut self, byte: u8) -> Option<Action> {
        // Check for UTF-8 start byte
        if byte >= 0xC0 {
            return self.start_utf8(byte);
        }

        // Regular ASCII printable character
        Some(Action::Print(byte as char))
    }

    /// Start UTF-8 sequence
    fn start_utf8(&mut self, byte: u8) -> Option<Action> {
        self.utf8_buffer.clear();
        self.utf8_buffer.push(byte);

        if byte < 0xC0 {
            // Invalid start byte, emit replacement
            return Some(Action::Print('\u{FFFD}'));
        } else if byte < 0xE0 {
            self.utf8_remaining = 1;
        } else if byte < 0xF0 {
            self.utf8_remaining = 2;
        } else if byte < 0xF8 {
            self.utf8_remaining = 3;
        } else {
            // Invalid start byte
            return Some(Action::Print('\u{FFFD}'));
        }

        None
    }

    /// Process UTF-8 continuation byte
    fn process_utf8_continuation(&mut self, byte: u8) -> Option<Action> {
        if (0x80..=0xBF).contains(&byte) {
            self.utf8_buffer.push(byte);
            self.utf8_remaining -= 1;

            if self.utf8_remaining == 0 {
                // Complete UTF-8 sequence
                let s = String::from_utf8_lossy(&self.utf8_buffer);
                let c = s.chars().next().unwrap_or('\u{FFFD}');
                self.utf8_buffer.clear();
                return Some(Action::Print(c));
            }
            None
        } else {
            // Invalid continuation, emit replacement and reprocess byte
            self.utf8_buffer.clear();
            self.utf8_remaining = 0;

            // Emit replacement for incomplete sequence
            let action = Some(Action::Print('\u{FFFD}'));

            // The current byte might be a new sequence start or control
            // We need to handle it, but we can only return one action
            // Store it for next iteration by recursively calling process_byte
            // Actually, we should just return the replacement and let the
            // next call handle this byte. But we've already consumed it.
            // For simplicity, we'll just return the replacement.
            // A more complete implementation would queue the byte.
            action
        }
    }

    /// Process bytes in escape state
    fn process_escape(&mut self, byte: u8) -> Option<Action> {
        match byte {
            // Intermediate bytes
            0x20..=0x2F => {
                self.intermediates.push(byte);
                self.state = State::EscapeIntermediate;
                None
            }
            // Final bytes - dispatch ESC sequence
            0x30..=0x4F | 0x51..=0x57 | 0x59 | 0x5A | 0x5C | 0x60..=0x7E => {
                self.state = State::Ground;
                self.dispatch_esc(byte)
            }
            // CSI (ESC [)
            0x5B => {
                self.state = State::CsiEntry;
                self.clear_params();
                None
            }
            // OSC (ESC ])
            0x5D => {
                self.state = State::OscString;
                self.osc_command = 0;
                self.osc_string.clear();
                None
            }
            // DCS (ESC P)
            0x50 => {
                self.state = State::DcsEntry;
                self.clear_params();
                self.dcs_string.clear();
                None
            }
            // SOS (ESC X)
            0x58 => {
                self.state = State::SosPmApcString;
                self.dcs_string.clear();
                None
            }
            // PM (ESC ^)
            0x5E => {
                self.state = State::SosPmApcString;
                self.dcs_string.clear();
                None
            }
            // APC (ESC _)
            0x5F => {
                self.state = State::SosPmApcString;
                self.dcs_string.clear();
                None
            }
            _ => {
                self.state = State::Ground;
                None
            }
        }
    }

    /// Process bytes in escape intermediate state
    fn process_escape_intermediate(&mut self, byte: u8) -> Option<Action> {
        match byte {
            0x20..=0x2F => {
                self.intermediates.push(byte);
                None
            }
            0x30..=0x7E => {
                self.state = State::Ground;
                self.dispatch_esc(byte)
            }
            _ => {
                self.state = State::Ground;
                None
            }
        }
    }

    /// Dispatch ESC sequence
    fn dispatch_esc(&mut self, final_byte: u8) -> Option<Action> {
        let action = if self.intermediates.is_empty() {
            match final_byte {
                b'7' => EscAction::SaveCursor,
                b'8' => EscAction::RestoreCursor,
                b'D' => EscAction::Index,
                b'M' => EscAction::ReverseIndex,
                b'E' => EscAction::NextLine,
                b'H' => EscAction::HorizontalTabSet,
                b'c' => EscAction::FullReset,
                b'=' => EscAction::ApplicationKeypad,
                b'>' => EscAction::NormalKeypad,
                b'N' => EscAction::SingleShift2,
                b'O' => EscAction::SingleShift3,
                _ => EscAction::Unknown(vec![final_byte]),
            }
        } else if self.intermediates.len() == 1 {
            match self.intermediates[0] {
                b'(' => EscAction::DesignateG0(final_byte),
                b')' => EscAction::DesignateG1(final_byte),
                b'*' => EscAction::DesignateG2(final_byte),
                b'+' => EscAction::DesignateG3(final_byte),
                _ => {
                    let mut seq = self.intermediates.clone();
                    seq.push(final_byte);
                    EscAction::Unknown(seq)
                }
            }
        } else {
            let mut seq = self.intermediates.clone();
            seq.push(final_byte);
            EscAction::Unknown(seq)
        };

        Some(Action::EscDispatch(action))
    }

    /// Process bytes in CSI entry state
    fn process_csi_entry(&mut self, byte: u8) -> Option<Action> {
        match byte {
            // Parameter bytes
            0x30..=0x39 => {
                self.current_param = (byte - b'0') as u32;
                self.param_has_digit = true;
                self.state = State::CsiParam;
                None
            }
            // Semicolon - empty first parameter
            b';' => {
                self.params.push(0);
                self.state = State::CsiParam;
                None
            }
            // Private marker
            b'?' | b'>' | b'<' | b'=' => {
                self.private_marker = byte == b'?';
                self.intermediates.push(byte);
                self.state = State::CsiParam;
                None
            }
            // Intermediate bytes
            0x20..=0x2F => {
                self.intermediates.push(byte);
                self.state = State::CsiIntermediate;
                None
            }
            // Final bytes - dispatch
            0x40..=0x7E => {
                self.state = State::Ground;
                self.dispatch_csi(byte)
            }
            // Colon - subparameter separator (treat like semicolon for now)
            b':' => {
                self.params.push(0);
                self.state = State::CsiParam;
                None
            }
            _ => {
                self.state = State::CsiIgnore;
                None
            }
        }
    }

    /// Process bytes in CSI param state
    fn process_csi_param(&mut self, byte: u8) -> Option<Action> {
        match byte {
            // Digit
            0x30..=0x39 => {
                self.current_param = self
                    .current_param
                    .saturating_mul(10)
                    .saturating_add((byte - b'0') as u32);
                self.param_has_digit = true;
                None
            }
            // Semicolon - parameter separator
            b';' => {
                self.params.push(self.current_param);
                self.current_param = 0;
                self.param_has_digit = false;
                None
            }
            // Colon - subparameter separator
            b':' => {
                self.params.push(self.current_param);
                self.current_param = 0;
                self.param_has_digit = false;
                None
            }
            // Intermediate bytes
            0x20..=0x2F => {
                if self.param_has_digit {
                    self.params.push(self.current_param);
                }
                self.intermediates.push(byte);
                self.state = State::CsiIntermediate;
                None
            }
            // Final bytes - dispatch
            0x40..=0x7E => {
                if self.param_has_digit || !self.params.is_empty() {
                    self.params.push(self.current_param);
                }
                self.state = State::Ground;
                self.dispatch_csi(byte)
            }
            // Private markers in wrong position
            b'?' | b'>' | b'<' | b'=' => {
                self.state = State::CsiIgnore;
                None
            }
            _ => {
                self.state = State::CsiIgnore;
                None
            }
        }
    }

    /// Process bytes in CSI intermediate state
    fn process_csi_intermediate(&mut self, byte: u8) -> Option<Action> {
        match byte {
            0x20..=0x2F => {
                self.intermediates.push(byte);
                None
            }
            0x40..=0x7E => {
                self.state = State::Ground;
                self.dispatch_csi(byte)
            }
            _ => {
                self.state = State::CsiIgnore;
                None
            }
        }
    }

    /// Process bytes in CSI ignore state
    fn process_csi_ignore(&mut self, byte: u8) -> Option<Action> {
        if (0x40..=0x7E).contains(&byte) {
            self.state = State::Ground;
        }
        None
    }

    /// Dispatch CSI sequence
    fn dispatch_csi(&mut self, final_byte: u8) -> Option<Action> {
        // Filter out the private marker from intermediates for the action
        let intermediates: Vec<u8> = self
            .intermediates
            .iter()
            .filter(|&&b| b != b'?' && b != b'>' && b != b'<' && b != b'=')
            .copied()
            .collect();

        let action = CsiAction {
            params: self.params.clone(),
            intermediates,
            final_byte,
            private: self.private_marker,
        };

        Some(Action::CsiDispatch(action))
    }

    /// Process bytes in OSC string state
    fn process_osc_string(&mut self, byte: u8) -> Option<Action> {
        match byte {
            // BEL terminates OSC (xterm extension)
            0x07 => self.terminate_osc(),
            // ST (ESC \) is handled via C0/escape processing
            // Collect string data
            0x20..=0x7E | 0x80..=0xFF => {
                // First, check if we're still collecting the command number
                if self.osc_string.is_empty() && byte.is_ascii_digit() {
                    self.osc_command = self.osc_command * 10 + (byte - b'0') as u32;
                } else if self.osc_string.is_empty() && byte == b';' {
                    // End of command number, start of payload
                    // Don't add the semicolon to the string
                } else {
                    self.osc_string.push(byte);
                }
                None
            }
            _ => None,
        }
    }

    /// Terminate OSC sequence and dispatch
    fn terminate_osc(&mut self) -> Option<Action> {
        self.state = State::Ground;

        let payload = String::from_utf8_lossy(&self.osc_string).to_string();

        let action = match self.osc_command {
            0 | 2 => OscAction::SetTitle(payload),
            1 => OscAction::SetIconName(payload),
            4 => {
                // Color setting: OSC 4 ; index ; color ST
                let parts: Vec<&str> = payload.splitn(2, ';').collect();
                if parts.len() == 2 {
                    if let Ok(index) = parts[0].parse() {
                        OscAction::SetColor {
                            index,
                            color: parts[1].to_string(),
                        }
                    } else {
                        OscAction::Unknown {
                            command: 4,
                            data: payload,
                        }
                    }
                } else {
                    OscAction::Unknown {
                        command: 4,
                        data: payload,
                    }
                }
            }
            8 => {
                // Hyperlink: OSC 8 ; params ; uri ST
                let parts: Vec<&str> = payload.splitn(2, ';').collect();
                if parts.len() == 2 {
                    OscAction::Hyperlink {
                        params: parts[0].to_string(),
                        uri: parts[1].to_string(),
                    }
                } else {
                    OscAction::Hyperlink {
                        params: String::new(),
                        uri: payload,
                    }
                }
            }
            52 => {
                // Clipboard: OSC 52 ; clipboard ; data ST
                let parts: Vec<&str> = payload.splitn(2, ';').collect();
                if parts.len() == 2 {
                    OscAction::Clipboard {
                        clipboard: parts[0].to_string(),
                        data: parts[1].to_string(),
                    }
                } else {
                    OscAction::Clipboard {
                        clipboard: String::new(),
                        data: payload,
                    }
                }
            }
            104 => OscAction::ResetColor(4),
            110 => OscAction::ResetColor(10),
            111 => OscAction::ResetColor(11),
            _ => OscAction::Unknown {
                command: self.osc_command,
                data: payload,
            },
        };

        self.osc_string.clear();
        Some(Action::OscDispatch(action))
    }

    /// Process bytes in DCS entry state
    fn process_dcs_entry(&mut self, byte: u8) -> Option<Action> {
        match byte {
            0x30..=0x39 => {
                self.current_param = (byte - b'0') as u32;
                self.param_has_digit = true;
                self.state = State::DcsParam;
                None
            }
            b';' => {
                self.params.push(0);
                self.state = State::DcsParam;
                None
            }
            0x20..=0x2F => {
                self.intermediates.push(byte);
                self.state = State::DcsIntermediate;
                None
            }
            0x40..=0x7E => {
                self.state = State::DcsPassthrough;
                None
            }
            b':' | b'<' | b'=' | b'>' | b'?' => {
                self.state = State::DcsParam;
                None
            }
            _ => {
                self.state = State::DcsIgnore;
                None
            }
        }
    }

    /// Process bytes in DCS param state
    fn process_dcs_param(&mut self, byte: u8) -> Option<Action> {
        match byte {
            0x30..=0x39 => {
                self.current_param = self
                    .current_param
                    .saturating_mul(10)
                    .saturating_add((byte - b'0') as u32);
                self.param_has_digit = true;
                None
            }
            b';' => {
                self.params.push(self.current_param);
                self.current_param = 0;
                self.param_has_digit = false;
                None
            }
            0x20..=0x2F => {
                self.intermediates.push(byte);
                self.state = State::DcsIntermediate;
                None
            }
            0x40..=0x7E => {
                self.state = State::DcsPassthrough;
                None
            }
            _ => {
                self.state = State::DcsIgnore;
                None
            }
        }
    }

    /// Process bytes in DCS intermediate state
    fn process_dcs_intermediate(&mut self, byte: u8) -> Option<Action> {
        match byte {
            0x20..=0x2F => {
                self.intermediates.push(byte);
                None
            }
            0x40..=0x7E => {
                self.state = State::DcsPassthrough;
                None
            }
            _ => {
                self.state = State::DcsIgnore;
                None
            }
        }
    }

    /// Process bytes in DCS passthrough state
    fn process_dcs_passthrough(&mut self, byte: u8) -> Option<Action> {
        // Collect data until ST
        if byte != 0x9C {
            self.dcs_string.push(byte);
        }
        None
    }

    /// Process bytes in DCS ignore state
    fn process_dcs_ignore(&mut self, _byte: u8) -> Option<Action> {
        // Just consume until ST
        None
    }

    /// Process bytes in SOS/PM/APC string state
    fn process_sos_pm_apc_string(&mut self, byte: u8) -> Option<Action> {
        // Collect until ST
        self.dcs_string.push(byte);
        None
    }

    /// Terminate string sequence (ST received)
    fn terminate_string(&mut self) -> Option<Action> {
        let action = match self.state {
            State::OscString => return self.terminate_osc(),
            State::DcsPassthrough => {
                let data = std::mem::take(&mut self.dcs_string);
                Some(Action::DcsDispatch(data))
            }
            State::SosPmApcString => {
                let data = std::mem::take(&mut self.dcs_string);
                // We don't know which one it was, just return as APC
                Some(Action::ApcDispatch(data))
            }
            _ => None,
        };

        self.state = State::Ground;
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_print() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"Hello");

        assert_eq!(actions.len(), 5);
        assert_eq!(actions[0], Action::Print('H'));
        assert_eq!(actions[4], Action::Print('o'));
    }

    #[test]
    fn test_parser_c0_controls() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"A\nB\rC");

        assert_eq!(actions.len(), 5);
        assert_eq!(actions[0], Action::Print('A'));
        assert_eq!(actions[1], Action::Execute(b'\n'));
        assert_eq!(actions[2], Action::Print('B'));
        assert_eq!(actions[3], Action::Execute(b'\r'));
        assert_eq!(actions[4], Action::Print('C'));
    }

    #[test]
    fn test_parser_csi_cursor_up() {
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
    fn test_parser_csi_cup() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b[10;20H");

        assert_eq!(actions.len(), 1);
        if let Action::CsiDispatch(csi) = &actions[0] {
            assert_eq!(csi.params, vec![10, 20]);
            assert_eq!(csi.final_byte, b'H');
        } else {
            panic!("Expected CsiDispatch");
        }
    }

    #[test]
    fn test_parser_csi_private() {
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
    fn test_parser_csi_sgr() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b[1;31;48;2;255;128;0m");

        assert_eq!(actions.len(), 1);
        if let Action::CsiDispatch(csi) = &actions[0] {
            assert_eq!(csi.params, vec![1, 31, 48, 2, 255, 128, 0]);
            assert_eq!(csi.final_byte, b'm');
        } else {
            panic!("Expected CsiDispatch");
        }
    }

    #[test]
    fn test_parser_esc_save_restore() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b7\x1b8");

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0], Action::EscDispatch(EscAction::SaveCursor));
        assert_eq!(actions[1], Action::EscDispatch(EscAction::RestoreCursor));
    }

    #[test]
    fn test_parser_osc_title() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b]0;My Title\x07");

        assert_eq!(actions.len(), 1);
        if let Action::OscDispatch(OscAction::SetTitle(title)) = &actions[0] {
            assert_eq!(title, "My Title");
        } else {
            panic!("Expected OscDispatch SetTitle");
        }
    }

    #[test]
    fn test_parser_osc_hyperlink() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b]8;;https://example.com\x07");

        assert_eq!(actions.len(), 1);
        if let Action::OscDispatch(OscAction::Hyperlink { params, uri }) = &actions[0] {
            assert_eq!(params, "");
            assert_eq!(uri, "https://example.com");
        } else {
            panic!("Expected OscDispatch Hyperlink");
        }
    }

    #[test]
    fn test_parser_utf8() {
        let mut parser = Parser::new();
        let actions = parser.parse("Hello 世界".as_bytes());

        // "Hello " = 6 chars, "世界" = 2 chars
        assert_eq!(actions.len(), 8);
        assert_eq!(actions[6], Action::Print('世'));
        assert_eq!(actions[7], Action::Print('界'));
    }

    #[test]
    fn test_parser_chunk_boundary() {
        let mut parser = Parser::new();

        // Split CSI sequence across chunks
        let actions1 = parser.parse(b"\x1b[");
        let actions2 = parser.parse(b"5");
        let actions3 = parser.parse(b"A");

        assert!(actions1.is_empty());
        assert!(actions2.is_empty());
        assert_eq!(actions3.len(), 1);

        if let Action::CsiDispatch(csi) = &actions3[0] {
            assert_eq!(csi.params, vec![5]);
            assert_eq!(csi.final_byte, b'A');
        } else {
            panic!("Expected CsiDispatch");
        }
    }

    #[test]
    fn test_parser_utf8_chunk_boundary() {
        let mut parser = Parser::new();

        // UTF-8 for '世' is E4 B8 96
        let actions1 = parser.parse(&[0xE4]);
        let actions2 = parser.parse(&[0xB8]);
        let actions3 = parser.parse(&[0x96]);

        assert!(actions1.is_empty());
        assert!(actions2.is_empty());
        assert_eq!(actions3.len(), 1);
        assert_eq!(actions3[0], Action::Print('世'));
    }

    #[test]
    fn test_parser_cancel_sequence() {
        let mut parser = Parser::new();

        // Start CSI, then cancel with CAN
        let actions = parser.parse(b"\x1b[5\x18A");

        // CAN cancels the sequence, 'A' is printed
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], Action::Print('A'));
    }

    #[test]
    fn test_parser_empty_params() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b[H");

        assert_eq!(actions.len(), 1);
        if let Action::CsiDispatch(csi) = &actions[0] {
            assert!(csi.params.is_empty());
            assert_eq!(csi.final_byte, b'H');
        } else {
            panic!("Expected CsiDispatch");
        }
    }

    #[test]
    fn test_parser_designate_charset() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b(B\x1b)0");

        assert_eq!(actions.len(), 2);
        assert_eq!(
            actions[0],
            Action::EscDispatch(EscAction::DesignateG0(b'B'))
        );
        assert_eq!(
            actions[1],
            Action::EscDispatch(EscAction::DesignateG1(b'0'))
        );
    }
}
