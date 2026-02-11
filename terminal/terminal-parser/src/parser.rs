//! Terminal escape sequence parser
//!
//! Implements a state machine parser based on the VT500 series parser model.
//! Reference: https://vt100.net/emu/dec_ansi_parser
//!
//! The parser handles:
//! - C0 control characters
//! - ESC sequences
//! - CSI (Control Sequence Introducer) sequences
//! - OSC (Operating System Command) sequences
//! - DCS (Device Control String) sequences
//! - APC, PM, SOS sequences (consumed but ignored)

use crate::action::{Action, CsiAction, EscAction, OscAction};
use crate::params::Params;
use crate::utf8::{Utf8Decoder, Utf8Result};

/// Maximum length for OSC/DCS data to prevent DoS
const MAX_OSC_LEN: usize = 65536;
/// Maximum length for intermediate bytes
const MAX_INTERMEDIATES: usize = 4;

/// Parser state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserState {
    /// Normal text processing
    Ground,
    /// After ESC
    Escape,
    /// After ESC [
    CsiEntry,
    /// Collecting CSI parameters
    CsiParam,
    /// Collecting CSI intermediate bytes
    CsiIntermediate,
    /// CSI sequence is invalid, consume until final byte
    CsiIgnore,
    /// After ESC ]
    OscString,
    /// After ESC P
    DcsEntry,
    /// Collecting DCS parameters
    DcsParam,
    /// DCS passthrough mode
    DcsPassthrough,
    /// DCS sequence is invalid
    DcsIgnore,
    /// After ESC _ (APC)
    ApcString,
    /// After ESC ^ (PM)
    PmString,
    /// After ESC X (SOS)
    SosString,
    /// Escape intermediate (ESC followed by intermediate byte)
    EscapeIntermediate,
}

/// The terminal parser
#[derive(Debug, Clone)]
pub struct Parser {
    /// Current state
    state: ParserState,
    /// UTF-8 decoder
    utf8: Utf8Decoder,
    /// CSI parameters being collected
    params_buf: Vec<u8>,
    /// CSI intermediate bytes
    intermediates: Vec<u8>,
    /// Whether CSI sequence starts with ?
    private_marker: bool,
    /// The actual marker byte (b'?', b'>', b'<', b'=', or 0 for none)
    marker_byte: u8,
    /// OSC/DCS string data
    osc_data: Vec<u8>,
    /// DCS parameters
    dcs_params: Vec<u8>,
    /// Escape intermediate bytes
    esc_intermediates: Vec<u8>,
}

impl Parser {
    /// Create a new parser
    pub fn new() -> Self {
        Self {
            state: ParserState::Ground,
            utf8: Utf8Decoder::new(),
            params_buf: Vec::with_capacity(64),
            intermediates: Vec::with_capacity(MAX_INTERMEDIATES),
            private_marker: false,
            marker_byte: 0,
            osc_data: Vec::with_capacity(256),
            dcs_params: Vec::with_capacity(64),
            esc_intermediates: Vec::with_capacity(MAX_INTERMEDIATES),
        }
    }

    /// Get current parser state
    pub fn state(&self) -> ParserState {
        self.state
    }

    /// Reset parser to ground state
    pub fn reset(&mut self) {
        self.state = ParserState::Ground;
        self.utf8.reset();
        self.params_buf.clear();
        self.intermediates.clear();
        self.private_marker = false;
        self.marker_byte = 0;
        self.osc_data.clear();
        self.dcs_params.clear();
        self.esc_intermediates.clear();
    }

    /// Parse a chunk of bytes, calling the callback for each action
    pub fn parse<F>(&mut self, data: &[u8], mut callback: F)
    where
        F: FnMut(Action),
    {
        for &byte in data {
            self.advance(byte, &mut callback);
        }
    }

    /// Parse a chunk and collect actions into a vector
    pub fn parse_collect(&mut self, data: &[u8]) -> Vec<Action> {
        let mut actions = Vec::new();
        self.parse(data, |action| actions.push(action));
        actions
    }

    /// Advance the parser by one byte
    fn advance<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        // Handle C0 controls that are always processed (except in string states)
        match self.state {
            ParserState::OscString
            | ParserState::DcsPassthrough
            | ParserState::ApcString
            | ParserState::PmString
            | ParserState::SosString => {
                // In string states, only ESC and certain controls terminate
                match byte {
                    0x1B => {
                        // ESC - might be ST (ESC \)
                        // We need to peek at next byte, but for streaming we handle it differently
                        // Store that we saw ESC and handle in next byte
                        self.handle_string_escape(callback);
                    }
                    0x07 => {
                        // BEL terminates OSC (xterm extension)
                        if self.state == ParserState::OscString {
                            self.finish_osc(callback);
                        } else {
                            self.collect_string_byte(byte);
                        }
                    }
                    0x9C => {
                        // ST (String Terminator) - 8-bit
                        self.finish_string(callback);
                    }
                    0x18 | 0x1A => {
                        // CAN, SUB - abort sequence
                        self.state = ParserState::Ground;
                        self.osc_data.clear();
                    }
                    _ => {
                        self.collect_string_byte(byte);
                    }
                }
                return;
            }
            _ => {}
        }

        // C0 controls (0x00-0x1F) - always execute except in string states
        if byte < 0x20 {
            match byte {
                0x1B => {
                    // ESC
                    self.enter_escape();
                }
                0x18 | 0x1A => {
                    // CAN, SUB - cancel current sequence
                    self.state = ParserState::Ground;
                }
                0x07..=0x0D => {
                    // BEL, BS, HT, LF, VT, FF, CR
                    callback(Action::Control(byte));
                }
                _ => {
                    // Other C0 controls - ignore or pass through
                }
            }
            return;
        }

        // C1 controls (0x80-0x9F) - 8-bit equivalents
        // But only if we're not in the middle of a UTF-8 sequence
        if (0x80..=0x9F).contains(&byte) && !self.utf8.is_pending() {
            match byte {
                0x90 => {
                    // DCS
                    self.enter_dcs();
                }
                0x9B => {
                    // CSI
                    self.enter_csi();
                }
                0x9D => {
                    // OSC
                    self.enter_osc();
                }
                0x9E => {
                    // PM
                    self.enter_pm();
                }
                0x9F => {
                    // APC
                    self.enter_apc();
                }
                0x9C => {
                    // ST - ignore if not in string state
                }
                _ => {
                    // Other C1 controls - ignore
                }
            }
            return;
        }

        // State-specific handling
        match self.state {
            ParserState::Ground => {
                self.handle_ground(byte, callback);
            }
            ParserState::Escape => {
                self.handle_escape(byte, callback);
            }
            ParserState::EscapeIntermediate => {
                self.handle_escape_intermediate(byte, callback);
            }
            ParserState::CsiEntry => {
                self.handle_csi_entry(byte, callback);
            }
            ParserState::CsiParam => {
                self.handle_csi_param(byte, callback);
            }
            ParserState::CsiIntermediate => {
                self.handle_csi_intermediate(byte, callback);
            }
            ParserState::CsiIgnore => {
                self.handle_csi_ignore(byte);
            }
            ParserState::DcsEntry => {
                self.handle_dcs_entry(byte);
            }
            ParserState::DcsParam => {
                self.handle_dcs_param(byte);
            }
            ParserState::DcsPassthrough => {
                // Handled above in string states
            }
            ParserState::DcsIgnore => {
                // Wait for ST
            }
            ParserState::OscString
            | ParserState::ApcString
            | ParserState::PmString
            | ParserState::SosString => {
                // Handled above in string states
            }
        }
    }

    fn handle_ground<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        // Printable characters
        if (0x20..0x7F).contains(&byte) {
            callback(Action::Print(byte as char));
        } else if byte >= 0x80 {
            // UTF-8 handling
            match self.utf8.feed(byte) {
                Utf8Result::Char(c) => callback(Action::Print(c)),
                Utf8Result::Invalid => callback(Action::Print(Utf8Decoder::replacement_char())),
                Utf8Result::Pending => {}
            }
        }
    }

    fn enter_escape(&mut self) {
        self.state = ParserState::Escape;
        self.esc_intermediates.clear();
    }

    fn handle_escape<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            b'[' => {
                self.enter_csi();
            }
            b']' => {
                self.enter_osc();
            }
            b'P' => {
                self.enter_dcs();
            }
            b'_' => {
                self.enter_apc();
            }
            b'^' => {
                self.enter_pm();
            }
            b'X' => {
                self.enter_sos();
            }
            b'\\' => {
                // ST (String Terminator) - ignore if not in string
                self.state = ParserState::Ground;
            }
            b'7' => {
                callback(Action::Esc(EscAction::SaveCursor));
                self.state = ParserState::Ground;
            }
            b'8' => {
                callback(Action::Esc(EscAction::RestoreCursor));
                self.state = ParserState::Ground;
            }
            b'D' => {
                callback(Action::Esc(EscAction::Index));
                self.state = ParserState::Ground;
            }
            b'M' => {
                callback(Action::Esc(EscAction::ReverseIndex));
                self.state = ParserState::Ground;
            }
            b'E' => {
                callback(Action::Esc(EscAction::NextLine));
                self.state = ParserState::Ground;
            }
            b'H' => {
                callback(Action::Esc(EscAction::HorizontalTabSet));
                self.state = ParserState::Ground;
            }
            b'c' => {
                callback(Action::Esc(EscAction::FullReset));
                self.state = ParserState::Ground;
            }
            b'=' => {
                callback(Action::Esc(EscAction::ApplicationKeypad));
                self.state = ParserState::Ground;
            }
            b'>' => {
                callback(Action::Esc(EscAction::NormalKeypad));
                self.state = ParserState::Ground;
            }
            b'(' | b')' | b'*' | b'+' => {
                // Character set designation - need next byte
                self.esc_intermediates.push(byte);
                self.state = ParserState::EscapeIntermediate;
            }
            b'#' => {
                // DEC private sequences
                self.esc_intermediates.push(byte);
                self.state = ParserState::EscapeIntermediate;
            }
            0x20..=0x2F => {
                // Intermediate bytes
                self.esc_intermediates.push(byte);
                self.state = ParserState::EscapeIntermediate;
            }
            0x30..=0x7E => {
                // Final byte - unknown sequence
                callback(Action::Esc(EscAction::Unknown(vec![byte])));
                self.state = ParserState::Ground;
            }
            _ => {
                // Invalid - return to ground
                self.state = ParserState::Ground;
            }
        }
    }

    fn handle_escape_intermediate<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            0x20..=0x2F => {
                // More intermediate bytes
                if self.esc_intermediates.len() < MAX_INTERMEDIATES {
                    self.esc_intermediates.push(byte);
                }
            }
            0x30..=0x7E => {
                // Final byte
                self.dispatch_esc(byte, callback);
                self.state = ParserState::Ground;
            }
            _ => {
                // Invalid
                self.state = ParserState::Ground;
            }
        }
    }

    fn dispatch_esc<F>(&mut self, final_byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        let action = match (self.esc_intermediates.as_slice(), final_byte) {
            ([b'('], c) => EscAction::DesignateG0(c as char),
            ([b')'], c) => EscAction::DesignateG1(c as char),
            ([b'*'], c) => EscAction::DesignateG2(c as char),
            ([b'+'], c) => EscAction::DesignateG3(c as char),
            ([b'#'], b'8') => EscAction::DecAlignmentTest,
            _ => {
                let mut data = self.esc_intermediates.clone();
                data.push(final_byte);
                EscAction::Unknown(data)
            }
        };
        callback(Action::Esc(action));
    }

    fn enter_csi(&mut self) {
        self.state = ParserState::CsiEntry;
        self.params_buf.clear();
        self.intermediates.clear();
        self.private_marker = false;
        self.marker_byte = 0;
    }

    fn handle_csi_entry<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            b'?' | b'>' | b'<' | b'=' => {
                self.private_marker = byte == b'?';
                self.marker_byte = byte;
                self.state = ParserState::CsiParam;
            }
            b'0'..=b'9' | b';' | b':' => {
                self.params_buf.push(byte);
                self.state = ParserState::CsiParam;
            }
            0x20..=0x2F => {
                // Intermediate byte
                self.intermediates.push(byte);
                self.state = ParserState::CsiIntermediate;
            }
            0x40..=0x7E => {
                // Final byte - dispatch
                self.dispatch_csi(byte, callback);
                self.state = ParserState::Ground;
            }
            _ => {
                self.state = ParserState::CsiIgnore;
            }
        }
    }

    fn handle_csi_param<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            b'0'..=b'9' | b';' | b':' => {
                self.params_buf.push(byte);
            }
            0x20..=0x2F => {
                // Intermediate byte
                self.intermediates.push(byte);
                self.state = ParserState::CsiIntermediate;
            }
            0x40..=0x7E => {
                // Final byte - dispatch
                self.dispatch_csi(byte, callback);
                self.state = ParserState::Ground;
            }
            b'?' | b'>' | b'<' | b'=' => {
                // Private marker in wrong position - ignore sequence
                self.state = ParserState::CsiIgnore;
            }
            _ => {
                self.state = ParserState::CsiIgnore;
            }
        }
    }

    fn handle_csi_intermediate<F>(&mut self, byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match byte {
            0x20..=0x2F => {
                if self.intermediates.len() < MAX_INTERMEDIATES {
                    self.intermediates.push(byte);
                } else {
                    self.state = ParserState::CsiIgnore;
                }
            }
            0x40..=0x7E => {
                // Final byte - dispatch
                self.dispatch_csi(byte, callback);
                self.state = ParserState::Ground;
            }
            _ => {
                self.state = ParserState::CsiIgnore;
            }
        }
    }

    fn handle_csi_ignore(&mut self, byte: u8) {
        if (0x40..=0x7E).contains(&byte) {
            self.state = ParserState::Ground;
        }
    }

    fn dispatch_csi<F>(&mut self, final_byte: u8, callback: &mut F)
    where
        F: FnMut(Action),
    {
        let params = Params::parse(&self.params_buf);
        let action = CsiAction {
            params,
            intermediates: self.intermediates.clone(),
            final_byte,
            private: self.private_marker,
            marker: self.marker_byte,
        };
        callback(Action::Csi(action));
    }

    fn enter_osc(&mut self) {
        self.state = ParserState::OscString;
        self.osc_data.clear();
    }

    fn enter_dcs(&mut self) {
        self.state = ParserState::DcsEntry;
        self.dcs_params.clear();
        self.osc_data.clear();
    }

    fn handle_dcs_entry(&mut self, byte: u8) {
        match byte {
            b'0'..=b'9' | b';' => {
                self.dcs_params.push(byte);
                self.state = ParserState::DcsParam;
            }
            0x40..=0x7E => {
                // Final byte - enter passthrough
                self.state = ParserState::DcsPassthrough;
            }
            _ => {
                self.state = ParserState::DcsIgnore;
            }
        }
    }

    fn handle_dcs_param(&mut self, byte: u8) {
        match byte {
            b'0'..=b'9' | b';' => {
                self.dcs_params.push(byte);
            }
            0x40..=0x7E => {
                // Final byte - enter passthrough
                self.state = ParserState::DcsPassthrough;
            }
            _ => {
                self.state = ParserState::DcsIgnore;
            }
        }
    }

    fn enter_apc(&mut self) {
        self.state = ParserState::ApcString;
        self.osc_data.clear();
    }

    fn enter_pm(&mut self) {
        self.state = ParserState::PmString;
        self.osc_data.clear();
    }

    fn enter_sos(&mut self) {
        self.state = ParserState::SosString;
        self.osc_data.clear();
    }

    fn collect_string_byte(&mut self, byte: u8) {
        if self.osc_data.len() < MAX_OSC_LEN {
            self.osc_data.push(byte);
        }
    }

    fn handle_string_escape<F>(&mut self, callback: &mut F)
    where
        F: FnMut(Action),
    {
        // We saw ESC in a string state - this could be ST (ESC \)
        // Finish the string and transition to Escape state so the next byte
        // (likely '\') is properly handled as part of the ESC sequence.
        // This fixes the bug where ESC \ would print a literal backslash.
        self.finish_string_to_escape(callback);
    }

    fn finish_string_to_escape<F>(&mut self, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match self.state {
            ParserState::OscString => {
                self.finish_osc(callback);
            }
            ParserState::DcsPassthrough => {
                let params = Params::parse(&self.dcs_params);
                callback(Action::Dcs {
                    params,
                    data: self.osc_data.clone(),
                });
            }
            ParserState::ApcString => {
                callback(Action::Apc(self.osc_data.clone()));
            }
            ParserState::PmString => {
                callback(Action::Pm(self.osc_data.clone()));
            }
            ParserState::SosString => {
                callback(Action::Sos(self.osc_data.clone()));
            }
            _ => {}
        }
        // Transition to Escape state instead of Ground so that the next byte
        // (the '\' in ESC \) is handled as part of the escape sequence
        self.state = ParserState::Escape;
        self.osc_data.clear();
    }

    fn finish_string<F>(&mut self, callback: &mut F)
    where
        F: FnMut(Action),
    {
        match self.state {
            ParserState::OscString => {
                self.finish_osc(callback);
            }
            ParserState::DcsPassthrough => {
                let params = Params::parse(&self.dcs_params);
                callback(Action::Dcs {
                    params,
                    data: self.osc_data.clone(),
                });
            }
            ParserState::ApcString => {
                callback(Action::Apc(self.osc_data.clone()));
            }
            ParserState::PmString => {
                callback(Action::Pm(self.osc_data.clone()));
            }
            ParserState::SosString => {
                callback(Action::Sos(self.osc_data.clone()));
            }
            _ => {}
        }
        self.state = ParserState::Ground;
        self.osc_data.clear();
    }

    fn finish_osc<F>(&mut self, callback: &mut F)
    where
        F: FnMut(Action),
    {
        let data = String::from_utf8_lossy(&self.osc_data).to_string();

        // Parse OSC command number
        let (cmd, payload) = if let Some(sep_pos) = data.find(';') {
            let cmd_str = &data[..sep_pos];
            let payload = &data[sep_pos + 1..];
            (cmd_str.parse::<u16>().unwrap_or(0), payload.to_string())
        } else {
            (data.parse::<u16>().unwrap_or(0), String::new())
        };

        let action = match cmd {
            0 => OscAction::SetIconAndTitle(payload),
            1 => OscAction::SetIconName(payload),
            2 => OscAction::SetTitle(payload),
            4 => {
                // Set color: OSC 4 ; index ; color ST
                if let Some(sep) = payload.find(';') {
                    let index = payload[..sep].parse::<u8>().unwrap_or(0);
                    let color = payload[sep + 1..].to_string();
                    OscAction::SetColor { index, color }
                } else {
                    OscAction::Unknown {
                        command: cmd,
                        data: payload,
                    }
                }
            }
            7 => OscAction::SetCurrentDirectory(payload),
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
            10 => OscAction::SetForegroundColor(payload),
            11 => OscAction::SetBackgroundColor(payload),
            12 => OscAction::SetCursorColor(payload),
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
            104 => {
                // Reset color
                let index = payload.parse::<u8>().ok();
                OscAction::ResetColor(index)
            }
            110 => OscAction::ResetForegroundColor,
            111 => OscAction::ResetBackgroundColor,
            112 => OscAction::ResetCursorColor,
            _ => OscAction::Unknown {
                command: cmd,
                data: payload,
            },
        };

        callback(Action::Osc(action));
        self.state = ParserState::Ground;
        self.osc_data.clear();
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_print() {
        let mut parser = Parser::new();
        let actions = parser.parse_collect(b"Hello");

        assert_eq!(actions.len(), 5);
        assert_eq!(actions[0], Action::Print('H'));
        assert_eq!(actions[4], Action::Print('o'));
    }

    #[test]
    fn test_parser_control() {
        let mut parser = Parser::new();
        let actions = parser.parse_collect(b"\x07\x08\x09\x0A\x0D");

        assert_eq!(actions.len(), 5);
        assert_eq!(actions[0], Action::Control(0x07)); // BEL
        assert_eq!(actions[1], Action::Control(0x08)); // BS
        assert_eq!(actions[2], Action::Control(0x09)); // HT
        assert_eq!(actions[3], Action::Control(0x0A)); // LF
        assert_eq!(actions[4], Action::Control(0x0D)); // CR
    }

    #[test]
    fn test_parser_csi_cursor() {
        let mut parser = Parser::new();
        let actions = parser.parse_collect(b"\x1b[10;20H");

        assert_eq!(actions.len(), 1);
        if let Action::Csi(csi) = &actions[0] {
            assert_eq!(csi.final_byte, b'H');
            assert_eq!(csi.param(0, 1), 10);
            assert_eq!(csi.param(1, 1), 20);
            assert!(!csi.private);
        } else {
            panic!("Expected CSI action");
        }
    }

    #[test]
    fn test_parser_csi_private() {
        let mut parser = Parser::new();
        let actions = parser.parse_collect(b"\x1b[?25h");

        assert_eq!(actions.len(), 1);
        if let Action::Csi(csi) = &actions[0] {
            assert_eq!(csi.final_byte, b'h');
            assert_eq!(csi.param(0, 0), 25);
            assert!(csi.private);
        } else {
            panic!("Expected CSI action");
        }
    }

    #[test]
    fn test_parser_csi_sgr() {
        let mut parser = Parser::new();
        let actions = parser.parse_collect(b"\x1b[1;31;42m");

        assert_eq!(actions.len(), 1);
        if let Action::Csi(csi) = &actions[0] {
            assert_eq!(csi.final_byte, b'm');
            assert_eq!(csi.params.len(), 3);
            assert_eq!(csi.param(0, 0), 1);
            assert_eq!(csi.param(1, 0), 31);
            assert_eq!(csi.param(2, 0), 42);
        } else {
            panic!("Expected CSI action");
        }
    }

    #[test]
    fn test_parser_esc_save_restore() {
        let mut parser = Parser::new();
        let actions = parser.parse_collect(b"\x1b7\x1b8");

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0], Action::Esc(EscAction::SaveCursor));
        assert_eq!(actions[1], Action::Esc(EscAction::RestoreCursor));
    }

    #[test]
    fn test_parser_esc_index() {
        let mut parser = Parser::new();
        let actions = parser.parse_collect(b"\x1bD\x1bM\x1bE");

        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0], Action::Esc(EscAction::Index));
        assert_eq!(actions[1], Action::Esc(EscAction::ReverseIndex));
        assert_eq!(actions[2], Action::Esc(EscAction::NextLine));
    }

    #[test]
    fn test_parser_osc_title() {
        let mut parser = Parser::new();
        let actions = parser.parse_collect(b"\x1b]0;My Title\x07");

        assert_eq!(actions.len(), 1);
        if let Action::Osc(OscAction::SetIconAndTitle(title)) = &actions[0] {
            assert_eq!(title, "My Title");
        } else {
            panic!("Expected OSC SetIconAndTitle action");
        }
    }

    #[test]
    fn test_parser_osc_hyperlink() {
        let mut parser = Parser::new();
        let actions = parser.parse_collect(b"\x1b]8;;https://example.com\x07");

        assert_eq!(actions.len(), 1);
        if let Action::Osc(OscAction::Hyperlink { params, uri }) = &actions[0] {
            assert_eq!(params, "");
            assert_eq!(uri, "https://example.com");
        } else {
            panic!("Expected OSC Hyperlink action");
        }
    }

    #[test]
    fn test_parser_utf8() {
        let mut parser = Parser::new();
        let actions = parser.parse_collect("Hello 世界 🎉".as_bytes());

        let chars: Vec<char> = actions
            .iter()
            .filter_map(|a| match a {
                Action::Print(c) => Some(*c),
                _ => None,
            })
            .collect();

        assert_eq!(
            chars,
            vec!['H', 'e', 'l', 'l', 'o', ' ', '世', '界', ' ', '🎉']
        );
    }

    #[test]
    fn test_parser_streaming() {
        // Test that parsing works correctly across chunk boundaries
        let mut parser = Parser::new();

        // Split CSI sequence across chunks
        let actions1 = parser.parse_collect(b"\x1b[10");
        assert!(actions1.is_empty()); // Sequence not complete

        let actions2 = parser.parse_collect(b";20H");
        assert_eq!(actions2.len(), 1);
        if let Action::Csi(csi) = &actions2[0] {
            assert_eq!(csi.param(0, 1), 10);
            assert_eq!(csi.param(1, 1), 20);
        }
    }

    #[test]
    fn test_parser_streaming_utf8() {
        let mut parser = Parser::new();

        // Split UTF-8 character across chunks
        // '中' = 0xE4 0xB8 0xAD
        let actions1 = parser.parse_collect(&[0xE4]);
        assert!(actions1.is_empty());

        let actions2 = parser.parse_collect(&[0xB8]);
        assert!(actions2.is_empty());

        let actions3 = parser.parse_collect(&[0xAD]);
        assert_eq!(actions3.len(), 1);
        assert_eq!(actions3[0], Action::Print('中'));
    }

    #[test]
    fn test_parser_designate_charset() {
        let mut parser = Parser::new();
        let actions = parser.parse_collect(b"\x1b(B\x1b(0");

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0], Action::Esc(EscAction::DesignateG0('B')));
        assert_eq!(actions[1], Action::Esc(EscAction::DesignateG0('0')));
    }

    #[test]
    fn test_parser_reset() {
        let mut parser = Parser::new();

        // Start a sequence
        parser.parse_collect(b"\x1b[10");
        assert_eq!(parser.state(), ParserState::CsiParam);

        // Reset
        parser.reset();
        assert_eq!(parser.state(), ParserState::Ground);

        // Should work normally now
        let actions = parser.parse_collect(b"A");
        assert_eq!(actions[0], Action::Print('A'));
    }
}
