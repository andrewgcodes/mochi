//! Parser state machine
//!
//! Implements a VT500-series compatible parser based on:
//! <https://vt100.net/emu/dec_ansi_parser>
//!
//! The parser is designed to:
//! - Handle arbitrary chunk boundaries (streaming)
//! - Be deterministic (same input always produces same output)
//! - Never crash or hang on any input

use super::actions::TerminalAction;

/// Parser states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum State {
    /// Normal character processing
    #[default]
    Ground,
    /// After ESC, waiting for next byte
    Escape,
    /// ESC followed by intermediate byte(s)
    EscapeIntermediate,
    /// After CSI (ESC [), entry point
    CsiEntry,
    /// Collecting CSI parameters
    CsiParam,
    /// CSI with intermediate byte(s)
    CsiIntermediate,
    /// Invalid CSI, consuming until final byte
    CsiIgnore,
    /// Collecting OSC payload
    OscString,
    /// After DCS (ESC P), entry point
    DcsEntry,
    /// Collecting DCS parameters
    DcsParam,
    /// DCS with intermediate byte(s)
    DcsIntermediate,
    /// Receiving DCS data
    DcsPassthrough,
    /// Invalid DCS, consuming until ST
    DcsIgnore,
    /// Consuming SOS/PM/APC string (ignored)
    SosPmApcString,
}

/// Maximum number of parameters in a CSI/DCS sequence
const MAX_PARAMS: usize = 32;

/// Maximum length of OSC string
const MAX_OSC_LEN: usize = 65536;

/// Maximum length of DCS data
const MAX_DCS_LEN: usize = 65536;

/// Terminal escape sequence parser
#[derive(Debug, Clone)]
pub struct Parser {
    /// Current parser state
    state: State,
    /// Previous state (for ST handling)
    prev_state: State,
    /// Intermediate bytes collected
    intermediates: Vec<u8>,
    /// Parameters collected
    params: Vec<u16>,
    /// Current parameter being built
    current_param: u16,
    /// Whether we have a current parameter
    has_param: bool,
    /// OSC string being collected
    osc_string: Vec<u8>,
    /// DCS data being collected
    dcs_data: Vec<u8>,
    /// DCS final byte (stored when entering passthrough)
    dcs_final: u8,
    /// UTF-8 decoder state
    utf8_buffer: Vec<u8>,
    /// Expected UTF-8 continuation bytes
    utf8_remaining: u8,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    /// Create a new parser
    pub fn new() -> Self {
        Self {
            state: State::Ground,
            prev_state: State::Ground,
            intermediates: Vec::with_capacity(4),
            params: Vec::with_capacity(MAX_PARAMS),
            current_param: 0,
            has_param: false,
            osc_string: Vec::new(),
            dcs_data: Vec::new(),
            dcs_final: 0,
            utf8_buffer: Vec::with_capacity(4),
            utf8_remaining: 0,
        }
    }

    /// Reset the parser to initial state
    pub fn reset(&mut self) {
        self.state = State::Ground;
        self.prev_state = State::Ground;
        self.intermediates.clear();
        self.params.clear();
        self.current_param = 0;
        self.has_param = false;
        self.osc_string.clear();
        self.dcs_data.clear();
        self.dcs_final = 0;
        self.utf8_buffer.clear();
        self.utf8_remaining = 0;
    }

    /// Feed bytes to the parser and return actions
    pub fn feed(&mut self, input: &[u8]) -> Vec<TerminalAction> {
        let mut actions = Vec::new();
        for &byte in input {
            self.advance(byte, &mut actions);
        }
        actions
    }

    /// Feed a single byte and return any resulting action
    pub fn feed_byte(&mut self, byte: u8) -> Option<TerminalAction> {
        let mut actions = Vec::new();
        self.advance(byte, &mut actions);
        actions.into_iter().next()
    }

    /// Advance the parser by one byte
    fn advance(&mut self, byte: u8, actions: &mut Vec<TerminalAction>) {
        // Handle UTF-8 continuation in ground state
        if self.state == State::Ground && self.utf8_remaining > 0 {
            if is_utf8_continuation(byte) {
                self.utf8_buffer.push(byte);
                self.utf8_remaining -= 1;
                if self.utf8_remaining == 0 {
                    // Complete UTF-8 sequence
                    if let Some(c) = decode_utf8(&self.utf8_buffer) {
                        actions.push(TerminalAction::Print(c));
                    } else {
                        // Invalid UTF-8, emit replacement character
                        actions.push(TerminalAction::Print('\u{FFFD}'));
                    }
                    self.utf8_buffer.clear();
                }
                return;
            } else {
                // Invalid continuation, emit replacement and process this byte
                actions.push(TerminalAction::Print('\u{FFFD}'));
                self.utf8_buffer.clear();
                self.utf8_remaining = 0;
            }
        }

        // Check for "anywhere" transitions (ESC and some C1 controls)
        // These can interrupt any state
        match byte {
            0x18 | 0x1A => {
                // CAN, SUB - cancel current sequence
                self.transition_to_ground();
                return;
            },
            0x1B => {
                // ESC - start escape sequence
                // Save previous state for ST handling (OSC/DCS terminated by ESC \)
                self.prev_state = self.state;
                self.clear_params();
                self.state = State::Escape;
                return;
            },
            _ => {},
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
            State::SosPmApcString => self.sos_pm_apc_string(byte, actions),
        }
    }

    /// Ground state - normal character processing
    fn ground(&mut self, byte: u8, actions: &mut Vec<TerminalAction>) {
        match byte {
            // C0 controls (except ESC which is handled above)
            0x00..=0x1A | 0x1C..=0x1F => {
                actions.push(TerminalAction::Execute(byte));
            },
            // DEL - ignore
            0x7F => {},
            // Printable ASCII
            0x20..=0x7E => {
                actions.push(TerminalAction::Print(byte as char));
            },
            // UTF-8 lead bytes
            0xC0..=0xDF => {
                // 2-byte sequence
                self.utf8_buffer.clear();
                self.utf8_buffer.push(byte);
                self.utf8_remaining = 1;
            },
            0xE0..=0xEF => {
                // 3-byte sequence
                self.utf8_buffer.clear();
                self.utf8_buffer.push(byte);
                self.utf8_remaining = 2;
            },
            0xF0..=0xF7 => {
                // 4-byte sequence
                self.utf8_buffer.clear();
                self.utf8_buffer.push(byte);
                self.utf8_remaining = 3;
            },
            // Invalid UTF-8 lead bytes or C1 controls
            // In UTF-8 mode, we don't recognize C1 controls (0x80-0x9F)
            // They are treated as invalid UTF-8
            _ => {
                actions.push(TerminalAction::Print('\u{FFFD}'));
            },
        }
    }

    /// Escape state - after ESC
    fn escape(&mut self, byte: u8, actions: &mut Vec<TerminalAction>) {
        match byte {
            // C0 controls - execute immediately
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                actions.push(TerminalAction::Execute(byte));
            },
            // Intermediate bytes - collect
            0x20..=0x2F => {
                self.collect_intermediate(byte);
                self.state = State::EscapeIntermediate;
            },
            // CSI - ESC [
            0x5B => {
                self.clear_params();
                self.state = State::CsiEntry;
            },
            // OSC - ESC ]
            0x5D => {
                self.osc_string.clear();
                self.state = State::OscString;
            },
            // DCS - ESC P
            0x50 => {
                self.clear_params();
                self.state = State::DcsEntry;
            },
            // SOS - ESC X
            0x58 => {
                self.state = State::SosPmApcString;
            },
            // PM - ESC ^
            0x5E => {
                self.state = State::SosPmApcString;
            },
            // APC - ESC _
            0x5F => {
                self.state = State::SosPmApcString;
            },
            // ST - ESC \ (string terminator)
            // If we came from OSC state, dispatch the OSC before going to ground
            0x5C => {
                match self.prev_state {
                    State::OscString => {
                        self.dispatch_osc(actions);
                    },
                    State::DcsPassthrough => {
                        // DCS unhook would go here
                        actions.push(TerminalAction::DcsUnhook);
                    },
                    _ => {},
                }
                self.prev_state = State::Ground;
                self.transition_to_ground();
            },
            // Final bytes - dispatch ESC sequence
            0x30..=0x4F | 0x51..=0x57 | 0x59 | 0x5A | 0x60..=0x7E => {
                actions.push(TerminalAction::EscDispatch {
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.transition_to_ground();
            },
            // DEL - ignore
            0x7F => {},
            // Anything else - ignore and return to ground
            _ => {
                self.transition_to_ground();
            },
        }
    }

    /// Escape intermediate state - ESC followed by intermediate byte(s)
    fn escape_intermediate(&mut self, byte: u8, actions: &mut Vec<TerminalAction>) {
        match byte {
            // C0 controls - execute immediately
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                actions.push(TerminalAction::Execute(byte));
            },
            // More intermediate bytes - collect
            0x20..=0x2F => {
                self.collect_intermediate(byte);
            },
            // Final bytes - dispatch ESC sequence
            0x30..=0x7E => {
                actions.push(TerminalAction::EscDispatch {
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.transition_to_ground();
            },
            // DEL - ignore
            0x7F => {},
            // Anything else - ignore and return to ground
            _ => {
                self.transition_to_ground();
            },
        }
    }

    /// CSI entry state - after CSI (ESC [)
    fn csi_entry(&mut self, byte: u8, actions: &mut Vec<TerminalAction>) {
        match byte {
            // C0 controls - execute immediately
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                actions.push(TerminalAction::Execute(byte));
            },
            // Parameter bytes (digits and semicolon)
            0x30..=0x39 | 0x3B => {
                self.param_byte(byte);
                self.state = State::CsiParam;
            },
            // Colon - subparameter separator (treat as param for now)
            0x3A => {
                self.state = State::CsiIgnore;
            },
            // Private marker (?, >, <, =)
            0x3C..=0x3F => {
                self.collect_intermediate(byte);
                self.state = State::CsiParam;
            },
            // Intermediate bytes
            0x20..=0x2F => {
                self.collect_intermediate(byte);
                self.state = State::CsiIntermediate;
            },
            // Final bytes - dispatch
            0x40..=0x7E => {
                self.finish_param();
                actions.push(TerminalAction::CsiDispatch {
                    params: self.params.clone(),
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.transition_to_ground();
            },
            // DEL - ignore
            0x7F => {},
            // Anything else - ignore
            _ => {},
        }
    }

    /// CSI param state - collecting parameters
    fn csi_param(&mut self, byte: u8, actions: &mut Vec<TerminalAction>) {
        match byte {
            // C0 controls - execute immediately
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                actions.push(TerminalAction::Execute(byte));
            },
            // Parameter bytes (digits and semicolon)
            0x30..=0x39 | 0x3B => {
                self.param_byte(byte);
            },
            // Colon or additional private markers - invalid
            0x3A | 0x3C..=0x3F => {
                self.state = State::CsiIgnore;
            },
            // Intermediate bytes
            0x20..=0x2F => {
                self.finish_param();
                self.collect_intermediate(byte);
                self.state = State::CsiIntermediate;
            },
            // Final bytes - dispatch
            0x40..=0x7E => {
                self.finish_param();
                actions.push(TerminalAction::CsiDispatch {
                    params: self.params.clone(),
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.transition_to_ground();
            },
            // DEL - ignore
            0x7F => {},
            // Anything else - ignore
            _ => {},
        }
    }

    /// CSI intermediate state - after intermediate byte(s)
    fn csi_intermediate(&mut self, byte: u8, actions: &mut Vec<TerminalAction>) {
        match byte {
            // C0 controls - execute immediately
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                actions.push(TerminalAction::Execute(byte));
            },
            // More intermediate bytes
            0x20..=0x2F => {
                self.collect_intermediate(byte);
            },
            // Parameter bytes after intermediate - invalid
            0x30..=0x3F => {
                self.state = State::CsiIgnore;
            },
            // Final bytes - dispatch
            0x40..=0x7E => {
                actions.push(TerminalAction::CsiDispatch {
                    params: self.params.clone(),
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.transition_to_ground();
            },
            // DEL - ignore
            0x7F => {},
            // Anything else - ignore
            _ => {},
        }
    }

    /// CSI ignore state - invalid CSI, consuming until final byte
    fn csi_ignore(&mut self, byte: u8, actions: &mut Vec<TerminalAction>) {
        match byte {
            // C0 controls - execute immediately
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                actions.push(TerminalAction::Execute(byte));
            },
            // Final bytes - return to ground
            0x40..=0x7E => {
                self.transition_to_ground();
            },
            // Everything else - ignore
            _ => {},
        }
    }

    /// OSC string state - collecting OSC payload
    fn osc_string(&mut self, byte: u8, actions: &mut Vec<TerminalAction>) {
        match byte {
            // BEL - terminate OSC
            0x07 => {
                self.dispatch_osc(actions);
                self.transition_to_ground();
            },
            // ESC - handled by advance() before calling this function
            // (ESC anywhere transition), but include for exhaustiveness
            0x1B => {},
            // C0 controls (except BEL and ESC) - ignore in OSC
            0x00..=0x06 | 0x08..=0x1A | 0x1C..=0x1F => {},
            // Printable characters - collect
            0x20..=0x7F => {
                if self.osc_string.len() < MAX_OSC_LEN {
                    self.osc_string.push(byte);
                }
            },
            // High bytes - collect (for UTF-8 in OSC)
            0x80..=0xFF => {
                if self.osc_string.len() < MAX_OSC_LEN {
                    self.osc_string.push(byte);
                }
            },
        }
    }

    /// DCS entry state - after DCS (ESC P)
    fn dcs_entry(&mut self, byte: u8, actions: &mut Vec<TerminalAction>) {
        match byte {
            // C0 controls - ignore in DCS
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {},
            // Parameter bytes
            0x30..=0x39 | 0x3B => {
                self.param_byte(byte);
                self.state = State::DcsParam;
            },
            // Colon - invalid
            0x3A => {
                self.state = State::DcsIgnore;
            },
            // Private marker
            0x3C..=0x3F => {
                self.collect_intermediate(byte);
                self.state = State::DcsParam;
            },
            // Intermediate bytes
            0x20..=0x2F => {
                self.collect_intermediate(byte);
                self.state = State::DcsIntermediate;
            },
            // Final bytes - enter passthrough
            0x40..=0x7E => {
                self.finish_param();
                self.dcs_final = byte;
                self.dcs_data.clear();
                actions.push(TerminalAction::DcsHook {
                    params: self.params.clone(),
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.state = State::DcsPassthrough;
            },
            // DEL - ignore
            0x7F => {},
            // Anything else - ignore
            _ => {},
        }
    }

    /// DCS param state - collecting parameters
    fn dcs_param(&mut self, byte: u8, actions: &mut Vec<TerminalAction>) {
        match byte {
            // C0 controls - ignore
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {},
            // Parameter bytes
            0x30..=0x39 | 0x3B => {
                self.param_byte(byte);
            },
            // Colon or additional private markers - invalid
            0x3A | 0x3C..=0x3F => {
                self.state = State::DcsIgnore;
            },
            // Intermediate bytes
            0x20..=0x2F => {
                self.finish_param();
                self.collect_intermediate(byte);
                self.state = State::DcsIntermediate;
            },
            // Final bytes - enter passthrough
            0x40..=0x7E => {
                self.finish_param();
                self.dcs_final = byte;
                self.dcs_data.clear();
                actions.push(TerminalAction::DcsHook {
                    params: self.params.clone(),
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.state = State::DcsPassthrough;
            },
            // DEL - ignore
            0x7F => {},
            // Anything else - ignore
            _ => {},
        }
    }

    /// DCS intermediate state
    fn dcs_intermediate(&mut self, byte: u8, actions: &mut Vec<TerminalAction>) {
        match byte {
            // C0 controls - ignore
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {},
            // More intermediate bytes
            0x20..=0x2F => {
                self.collect_intermediate(byte);
            },
            // Parameter bytes after intermediate - invalid
            0x30..=0x3F => {
                self.state = State::DcsIgnore;
            },
            // Final bytes - enter passthrough
            0x40..=0x7E => {
                self.dcs_final = byte;
                self.dcs_data.clear();
                actions.push(TerminalAction::DcsHook {
                    params: self.params.clone(),
                    intermediates: self.intermediates.clone(),
                    final_byte: byte,
                });
                self.state = State::DcsPassthrough;
            },
            // DEL - ignore
            0x7F => {},
            // Anything else - ignore
            _ => {},
        }
    }

    /// DCS passthrough state - receiving data
    fn dcs_passthrough(&mut self, byte: u8, actions: &mut Vec<TerminalAction>) {
        match byte {
            // C0 controls (except CAN, SUB, ESC) - put
            0x00..=0x17 | 0x19 | 0x1C..=0x1F => {
                if self.dcs_data.len() < MAX_DCS_LEN {
                    self.dcs_data.push(byte);
                    actions.push(TerminalAction::DcsPut(byte));
                }
            },
            // CAN (0x18) and SUB (0x1A) - cancel DCS, return to ground
            // These are handled by advance() before calling this function
            0x18 | 0x1A => {},
            // ESC (0x1B) - handled by advance() for ESC anywhere transition
            0x1B => {},
            // Printable and high bytes - put
            0x20..=0x7E | 0x80..=0xFF => {
                if self.dcs_data.len() < MAX_DCS_LEN {
                    self.dcs_data.push(byte);
                    actions.push(TerminalAction::DcsPut(byte));
                }
            },
            // DEL - ignore
            0x7F => {},
        }
        // Note: ST (ESC \) is handled by the ESC anywhere transition
        // which will cause a transition to Escape state, then the \ will
        // be processed and we'll dispatch the DCS
    }

    /// DCS ignore state - invalid DCS, consuming until ST
    fn dcs_ignore(&mut self, byte: u8, _actions: &mut [TerminalAction]) {
        // Just consume everything until ST (handled by ESC anywhere)
        let _ = byte;
    }

    /// SOS/PM/APC string state - consuming and ignoring
    fn sos_pm_apc_string(&mut self, byte: u8, _actions: &mut [TerminalAction]) {
        // Just consume everything until ST (handled by ESC anywhere)
        let _ = byte;
    }

    /// Transition to ground state
    fn transition_to_ground(&mut self) {
        // Check if we were in DCS passthrough and need to unhook
        if self.state == State::DcsPassthrough {
            // DcsUnhook would be emitted here if needed
        }
        self.state = State::Ground;
        self.intermediates.clear();
        self.utf8_buffer.clear();
        self.utf8_remaining = 0;
    }

    /// Clear parameters for a new sequence
    fn clear_params(&mut self) {
        self.params.clear();
        self.intermediates.clear();
        self.current_param = 0;
        self.has_param = false;
    }

    /// Collect an intermediate byte
    fn collect_intermediate(&mut self, byte: u8) {
        if self.intermediates.len() < 4 {
            self.intermediates.push(byte);
        }
    }

    /// Process a parameter byte (digit or semicolon)
    fn param_byte(&mut self, byte: u8) {
        match byte {
            0x30..=0x39 => {
                // Digit
                self.current_param = self
                    .current_param
                    .saturating_mul(10)
                    .saturating_add((byte - 0x30) as u16);
                self.has_param = true;
            },
            0x3B => {
                // Semicolon - finish current param and start new one
                self.finish_param();
            },
            _ => {},
        }
    }

    /// Finish the current parameter
    fn finish_param(&mut self) {
        if self.params.len() < MAX_PARAMS {
            // If we have a param, push it; otherwise push 0 for empty param
            self.params.push(if self.has_param {
                self.current_param
            } else {
                0
            });
        }
        self.current_param = 0;
        self.has_param = false;
    }

    /// Dispatch OSC sequence
    fn dispatch_osc(&mut self, actions: &mut Vec<TerminalAction>) {
        // Split OSC string on semicolons
        let params: Vec<Vec<u8>> = self
            .osc_string
            .split(|&b| b == b';')
            .map(|s| s.to_vec())
            .collect();

        actions.push(TerminalAction::OscDispatch { params });
        self.osc_string.clear();
    }
}

/// Check if a byte is a UTF-8 continuation byte
fn is_utf8_continuation(byte: u8) -> bool {
    (byte & 0xC0) == 0x80
}

/// Decode a UTF-8 sequence
fn decode_utf8(bytes: &[u8]) -> Option<char> {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|s| s.chars().next())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_new() {
        let parser = Parser::new();
        assert_eq!(parser.state, State::Ground);
    }

    #[test]
    fn test_parser_print() {
        let mut parser = Parser::new();
        let actions = parser.feed(b"Hello");

        assert_eq!(actions.len(), 5);
        assert_eq!(actions[0], TerminalAction::Print('H'));
        assert_eq!(actions[1], TerminalAction::Print('e'));
        assert_eq!(actions[2], TerminalAction::Print('l'));
        assert_eq!(actions[3], TerminalAction::Print('l'));
        assert_eq!(actions[4], TerminalAction::Print('o'));
    }

    #[test]
    fn test_parser_control() {
        let mut parser = Parser::new();
        let actions = parser.feed(b"\n\r\t");

        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0], TerminalAction::Execute(0x0A));
        assert_eq!(actions[1], TerminalAction::Execute(0x0D));
        assert_eq!(actions[2], TerminalAction::Execute(0x09));
    }

    #[test]
    fn test_parser_csi_simple() {
        let mut parser = Parser::new();
        let actions = parser.feed(b"\x1b[H");

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            TerminalAction::CsiDispatch {
                params,
                intermediates,
                final_byte,
            } => {
                assert!(params.is_empty() || params == &[0]);
                assert!(intermediates.is_empty());
                assert_eq!(*final_byte, b'H');
            },
            _ => panic!("Expected CsiDispatch"),
        }
    }

    #[test]
    fn test_parser_csi_params() {
        let mut parser = Parser::new();
        let actions = parser.feed(b"\x1b[5;10H");

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            TerminalAction::CsiDispatch {
                params,
                intermediates,
                final_byte,
            } => {
                assert_eq!(params, &[5, 10]);
                assert!(intermediates.is_empty());
                assert_eq!(*final_byte, b'H');
            },
            _ => panic!("Expected CsiDispatch"),
        }
    }

    #[test]
    fn test_parser_csi_private() {
        let mut parser = Parser::new();
        let actions = parser.feed(b"\x1b[?25h");

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            TerminalAction::CsiDispatch {
                params,
                intermediates,
                final_byte,
            } => {
                assert_eq!(params, &[25]);
                assert_eq!(intermediates, b"?");
                assert_eq!(*final_byte, b'h');
            },
            _ => panic!("Expected CsiDispatch"),
        }
    }

    #[test]
    fn test_parser_sgr() {
        let mut parser = Parser::new();
        let actions = parser.feed(b"\x1b[1;31m");

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            TerminalAction::CsiDispatch {
                params,
                intermediates,
                final_byte,
            } => {
                assert_eq!(params, &[1, 31]);
                assert!(intermediates.is_empty());
                assert_eq!(*final_byte, b'm');
            },
            _ => panic!("Expected CsiDispatch"),
        }
    }

    #[test]
    fn test_parser_osc() {
        let mut parser = Parser::new();
        let actions = parser.feed(b"\x1b]0;Title\x07");

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            TerminalAction::OscDispatch { params } => {
                assert_eq!(params.len(), 2);
                assert_eq!(params[0], b"0");
                assert_eq!(params[1], b"Title");
            },
            _ => panic!("Expected OscDispatch"),
        }
    }

    #[test]
    fn test_parser_esc() {
        let mut parser = Parser::new();
        let actions = parser.feed(b"\x1b7");

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            TerminalAction::EscDispatch {
                intermediates,
                final_byte,
            } => {
                assert!(intermediates.is_empty());
                assert_eq!(*final_byte, b'7');
            },
            _ => panic!("Expected EscDispatch"),
        }
    }

    #[test]
    fn test_parser_utf8() {
        let mut parser = Parser::new();
        let actions = parser.feed("Hello, 世界!".as_bytes());

        // H e l l o ,   世 界 !
        assert_eq!(actions.len(), 10);
        assert_eq!(actions[7], TerminalAction::Print('世'));
        assert_eq!(actions[8], TerminalAction::Print('界'));
    }

    #[test]
    fn test_parser_chunk_boundary() {
        let mut parser = Parser::new();

        // Split CSI sequence across chunks
        let actions1 = parser.feed(b"\x1b[");
        let actions2 = parser.feed(b"5;");
        let actions3 = parser.feed(b"10H");

        assert!(actions1.is_empty());
        assert!(actions2.is_empty());
        assert_eq!(actions3.len(), 1);
        match &actions3[0] {
            TerminalAction::CsiDispatch { params, .. } => {
                assert_eq!(params, &[5, 10]);
            },
            _ => panic!("Expected CsiDispatch"),
        }
    }

    #[test]
    fn test_parser_cancel() {
        let mut parser = Parser::new();

        // Start CSI then cancel with CAN
        let actions = parser.feed(b"\x1b[5\x18A");

        // CAN cancels the sequence, then 'A' is printed
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], TerminalAction::Print('A'));
    }

    #[test]
    fn test_parser_empty_params() {
        let mut parser = Parser::new();
        let actions = parser.feed(b"\x1b[;5H");

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            TerminalAction::CsiDispatch { params, .. } => {
                // First param is empty (0), second is 5
                assert_eq!(params, &[0, 5]);
            },
            _ => panic!("Expected CsiDispatch"),
        }
    }

    #[test]
    fn test_parser_256_color() {
        let mut parser = Parser::new();
        let actions = parser.feed(b"\x1b[38;5;196m");

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            TerminalAction::CsiDispatch { params, .. } => {
                assert_eq!(params, &[38, 5, 196]);
            },
            _ => panic!("Expected CsiDispatch"),
        }
    }

    #[test]
    fn test_parser_truecolor() {
        let mut parser = Parser::new();
        let actions = parser.feed(b"\x1b[38;2;255;128;64m");

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            TerminalAction::CsiDispatch { params, .. } => {
                assert_eq!(params, &[38, 2, 255, 128, 64]);
            },
            _ => panic!("Expected CsiDispatch"),
        }
    }

    #[test]
    fn test_parser_invalid_utf8() {
        let mut parser = Parser::new();
        // Invalid UTF-8 sequence
        let actions = parser.feed(&[0xFF, 0xFE]);

        // Should produce replacement characters
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0], TerminalAction::Print('\u{FFFD}'));
        assert_eq!(actions[1], TerminalAction::Print('\u{FFFD}'));
    }

    #[test]
    fn test_parser_c0_in_csi() {
        let mut parser = Parser::new();
        // C0 control in the middle of CSI should be executed
        let actions = parser.feed(b"\x1b[5\nH");

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0], TerminalAction::Execute(0x0A));
        match &actions[1] {
            TerminalAction::CsiDispatch { params, .. } => {
                assert_eq!(params, &[5]);
            },
            _ => panic!("Expected CsiDispatch"),
        }
    }

    #[test]
    fn test_parser_utf8_chunked() {
        let mut parser = Parser::new();

        // "世" is \xe4\xb8\x96 in UTF-8 (3-byte sequence)
        // Feed one byte at a time
        let actions1 = parser.feed(&[0xe4]);
        assert!(actions1.is_empty(), "Lead byte should not produce action");

        let actions2 = parser.feed(&[0xb8]);
        assert!(actions2.is_empty(), "Second byte should not produce action");

        let actions3 = parser.feed(&[0x96]);
        assert_eq!(
            actions3.len(),
            1,
            "Third byte should complete the character"
        );
        assert_eq!(actions3[0], TerminalAction::Print('世'));
    }
}
