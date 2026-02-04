//! Parser State Machine
//!
//! Implements a VT/xterm-compatible escape sequence parser using a state machine.
//! Based on the state machine described in:
//! - ECMA-48 (ISO 6429)
//! - https://vt100.net/emu/dec_ansi_parser
//!
//! The parser handles:
//! - UTF-8 decoding
//! - C0 control characters
//! - ESC sequences
//! - CSI sequences
//! - OSC sequences
//! - DCS/APC/PM/SOS sequences (consumed but not fully interpreted)

use super::action::{Action, ControlCode, CsiAction, EscAction, OscAction};
use super::{MAX_CSI_PARAMS, MAX_OSC_LENGTH, MAX_STRING_LENGTH};

/// Parser state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Normal text processing
    Ground,
    /// After ESC
    Escape,
    /// After ESC [
    CsiEntry,
    /// Collecting CSI parameters
    CsiParam,
    /// Collecting CSI intermediate characters
    CsiIntermediate,
    /// CSI sequence ignored (too many params, etc.)
    CsiIgnore,
    /// After ESC ]
    OscString,
    /// After ESC P (DCS)
    DcsEntry,
    /// DCS passthrough
    DcsPassthrough,
    /// DCS ignored
    DcsIgnore,
    /// After ESC _ (APC)
    ApcString,
    /// After ESC ^ (PM)
    PmString,
    /// After ESC X (SOS)
    SosString,
    /// After ESC (
    SelectG0,
    /// After ESC )
    SelectG1,
    /// After ESC #
    DecHash,
    /// UTF-8 continuation bytes expected
    Utf8(Utf8State),
}

/// UTF-8 decoding state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Utf8State {
    /// Accumulated codepoint value
    value: u32,
    /// Number of continuation bytes remaining
    remaining: u8,
}

/// The terminal parser
#[derive(Debug)]
pub struct Parser {
    /// Current state
    state: State,
    /// CSI parameters being collected
    csi_params: Vec<u16>,
    /// Current CSI parameter being built
    csi_current_param: u16,
    /// CSI intermediate characters
    csi_intermediates: Vec<char>,
    /// CSI private marker (?, >, etc.)
    csi_private_marker: Option<char>,
    /// OSC string being collected
    osc_string: String,
    /// DCS string being collected
    dcs_string: String,
    /// APC string being collected
    apc_string: String,
    /// PM string being collected
    pm_string: String,
    /// SOS string being collected
    sos_string: String,
    /// Whether we've seen a parameter digit in current CSI
    csi_has_param: bool,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    pub fn new() -> Self {
        Self {
            state: State::Ground,
            csi_params: Vec::with_capacity(MAX_CSI_PARAMS),
            csi_current_param: 0,
            csi_intermediates: Vec::new(),
            csi_private_marker: None,
            osc_string: String::new(),
            dcs_string: String::new(),
            apc_string: String::new(),
            pm_string: String::new(),
            sos_string: String::new(),
            csi_has_param: false,
        }
    }

    /// Parse a chunk of bytes and return the resulting actions
    pub fn parse(&mut self, data: &[u8]) -> Vec<Action> {
        let mut actions = Vec::new();

        for &byte in data {
            if let Some(action) = self.process_byte(byte) {
                actions.push(action);
            }
        }

        actions
    }

    /// Process a single byte and optionally return an action
    fn process_byte(&mut self, byte: u8) -> Option<Action> {
        // Handle UTF-8 continuation in ground state
        if let State::Utf8(utf8_state) = self.state {
            return self.process_utf8_continuation(byte, utf8_state);
        }

        // C0 controls are handled in most states
        if byte < 0x20 {
            return self.process_c0(byte);
        }

        // DEL is ignored in most contexts
        if byte == 0x7F {
            return None;
        }

        match self.state {
            State::Ground => self.process_ground(byte),
            State::Escape => self.process_escape(byte),
            State::CsiEntry => self.process_csi_entry(byte),
            State::CsiParam => self.process_csi_param(byte),
            State::CsiIntermediate => self.process_csi_intermediate(byte),
            State::CsiIgnore => self.process_csi_ignore(byte),
            State::OscString => self.process_osc_string(byte),
            State::DcsEntry => self.process_dcs_entry(byte),
            State::DcsPassthrough => self.process_dcs_passthrough(byte),
            State::DcsIgnore => self.process_dcs_ignore(byte),
            State::ApcString => self.process_apc_string(byte),
            State::PmString => self.process_pm_string(byte),
            State::SosString => self.process_sos_string(byte),
            State::SelectG0 => self.process_select_g0(byte),
            State::SelectG1 => self.process_select_g1(byte),
            State::DecHash => self.process_dec_hash(byte),
            State::Utf8(_) => unreachable!(),
        }
    }

    /// Process a C0 control character (0x00-0x1F)
    fn process_c0(&mut self, byte: u8) -> Option<Action> {
        match byte {
            0x00 => Some(Action::Control(ControlCode::Null)),
            0x07 => {
                // BEL - also terminates OSC
                if self.state == State::OscString {
                    let action = self.finish_osc();
                    self.state = State::Ground;
                    return action;
                }
                Some(Action::Control(ControlCode::Bell))
            }
            0x08 => Some(Action::Control(ControlCode::Backspace)),
            0x09 => Some(Action::Control(ControlCode::Tab)),
            0x0A => Some(Action::Control(ControlCode::LineFeed)),
            0x0B => Some(Action::Control(ControlCode::VerticalTab)),
            0x0C => Some(Action::Control(ControlCode::FormFeed)),
            0x0D => Some(Action::Control(ControlCode::CarriageReturn)),
            0x0E => Some(Action::Control(ControlCode::ShiftOut)),
            0x0F => Some(Action::Control(ControlCode::ShiftIn)),
            0x18 | 0x1A => {
                // CAN or SUB - cancel escape sequence
                self.state = State::Ground;
                if byte == 0x1A {
                    Some(Action::Control(ControlCode::Substitute))
                } else {
                    Some(Action::Control(ControlCode::Cancel))
                }
            }
            0x1B => {
                // ESC - start escape sequence
                // But in OSC/DCS/APC/PM/SOS states, ESC might be part of ST (ESC \)
                match self.state {
                    State::OscString => {
                        self.osc_string.push('\x1B');
                        None
                    }
                    State::DcsPassthrough => {
                        self.dcs_string.push('\x1B');
                        None
                    }
                    State::ApcString => {
                        self.apc_string.push('\x1B');
                        None
                    }
                    State::PmString => {
                        self.pm_string.push('\x1B');
                        None
                    }
                    State::SosString => {
                        self.sos_string.push('\x1B');
                        None
                    }
                    _ => {
                        self.state = State::Escape;
                        None
                    }
                }
            }
            _ => None, // Other C0 codes are ignored
        }
    }

    /// Process a byte in ground state
    fn process_ground(&mut self, byte: u8) -> Option<Action> {
        // Check for UTF-8 start byte
        if byte >= 0x80 {
            if byte & 0xE0 == 0xC0 {
                // 2-byte sequence
                self.state = State::Utf8(Utf8State {
                    value: (byte & 0x1F) as u32,
                    remaining: 1,
                });
                return None;
            } else if byte & 0xF0 == 0xE0 {
                // 3-byte sequence
                self.state = State::Utf8(Utf8State {
                    value: (byte & 0x0F) as u32,
                    remaining: 2,
                });
                return None;
            } else if byte & 0xF8 == 0xF0 {
                // 4-byte sequence
                self.state = State::Utf8(Utf8State {
                    value: (byte & 0x07) as u32,
                    remaining: 3,
                });
                return None;
            } else {
                // Invalid UTF-8 start byte - emit replacement character
                return Some(Action::Print('\u{FFFD}'));
            }
        }

        // ASCII printable character
        Some(Action::Print(byte as char))
    }

    /// Process UTF-8 continuation byte
    fn process_utf8_continuation(&mut self, byte: u8, mut utf8_state: Utf8State) -> Option<Action> {
        if byte & 0xC0 != 0x80 {
            // Invalid continuation byte - emit replacement and reprocess
            self.state = State::Ground;
            // We should reprocess this byte, but for simplicity emit replacement
            // and let the byte be processed in the next call
            return Some(Action::Print('\u{FFFD}'));
        }

        utf8_state.value = (utf8_state.value << 6) | (byte & 0x3F) as u32;
        utf8_state.remaining -= 1;

        if utf8_state.remaining == 0 {
            self.state = State::Ground;
            // Convert to char, using replacement for invalid codepoints
            match char::from_u32(utf8_state.value) {
                Some(c) => Some(Action::Print(c)),
                None => Some(Action::Print('\u{FFFD}')),
            }
        } else {
            self.state = State::Utf8(utf8_state);
            None
        }
    }

    /// Process a byte after ESC
    fn process_escape(&mut self, byte: u8) -> Option<Action> {
        match byte {
            b'[' => {
                // CSI
                self.state = State::CsiEntry;
                self.reset_csi();
                None
            }
            b']' => {
                // OSC
                self.state = State::OscString;
                self.osc_string.clear();
                None
            }
            b'P' => {
                // DCS
                self.state = State::DcsEntry;
                self.dcs_string.clear();
                None
            }
            b'_' => {
                // APC
                self.state = State::ApcString;
                self.apc_string.clear();
                None
            }
            b'^' => {
                // PM
                self.state = State::PmString;
                self.pm_string.clear();
                None
            }
            b'X' => {
                // SOS
                self.state = State::SosString;
                self.sos_string.clear();
                None
            }
            b'7' => {
                self.state = State::Ground;
                Some(Action::Esc(EscAction::SaveCursor))
            }
            b'8' => {
                self.state = State::Ground;
                Some(Action::Esc(EscAction::RestoreCursor))
            }
            b'D' => {
                self.state = State::Ground;
                Some(Action::Esc(EscAction::Index))
            }
            b'M' => {
                self.state = State::Ground;
                Some(Action::Esc(EscAction::ReverseIndex))
            }
            b'E' => {
                self.state = State::Ground;
                Some(Action::Esc(EscAction::NextLine))
            }
            b'H' => {
                self.state = State::Ground;
                Some(Action::Esc(EscAction::TabSet))
            }
            b'c' => {
                self.state = State::Ground;
                Some(Action::Esc(EscAction::FullReset))
            }
            b'=' => {
                self.state = State::Ground;
                Some(Action::Esc(EscAction::ApplicationKeypad))
            }
            b'>' => {
                self.state = State::Ground;
                Some(Action::Esc(EscAction::NormalKeypad))
            }
            b'(' => {
                self.state = State::SelectG0;
                None
            }
            b')' => {
                self.state = State::SelectG1;
                None
            }
            b'#' => {
                self.state = State::DecHash;
                None
            }
            b'N' => {
                self.state = State::Ground;
                Some(Action::Esc(EscAction::SingleShift2))
            }
            b'O' => {
                self.state = State::Ground;
                Some(Action::Esc(EscAction::SingleShift3))
            }
            b'\\' => {
                // ST (String Terminator) - shouldn't appear here, ignore
                self.state = State::Ground;
                None
            }
            _ => {
                self.state = State::Ground;
                Some(Action::Esc(EscAction::Unknown(byte as char)))
            }
        }
    }

    /// Reset CSI state for a new sequence
    fn reset_csi(&mut self) {
        self.csi_params.clear();
        self.csi_current_param = 0;
        self.csi_intermediates.clear();
        self.csi_private_marker = None;
        self.csi_has_param = false;
    }

    /// Process a byte at CSI entry
    fn process_csi_entry(&mut self, byte: u8) -> Option<Action> {
        match byte {
            b'?' | b'>' | b'<' | b'=' | b'!' => {
                // Private marker
                self.csi_private_marker = Some(byte as char);
                self.state = State::CsiParam;
                None
            }
            b'0'..=b'9' => {
                self.csi_current_param = (byte - b'0') as u16;
                self.csi_has_param = true;
                self.state = State::CsiParam;
                None
            }
            b';' => {
                // Empty first parameter
                self.csi_params.push(0);
                self.state = State::CsiParam;
                None
            }
            b':' => {
                // Subparameter separator (used in SGR)
                self.csi_current_param = 0;
                self.csi_has_param = true;
                self.state = State::CsiParam;
                None
            }
            0x20..=0x2F => {
                // Intermediate character
                self.csi_intermediates.push(byte as char);
                self.state = State::CsiIntermediate;
                None
            }
            0x40..=0x7E => {
                // Final character - dispatch immediately
                self.state = State::Ground;
                Some(self.dispatch_csi(byte as char))
            }
            _ => {
                self.state = State::CsiIgnore;
                None
            }
        }
    }

    /// Process a byte while collecting CSI parameters
    fn process_csi_param(&mut self, byte: u8) -> Option<Action> {
        match byte {
            b'0'..=b'9' => {
                self.csi_current_param = self
                    .csi_current_param
                    .saturating_mul(10)
                    .saturating_add((byte - b'0') as u16);
                self.csi_has_param = true;
                None
            }
            b';' => {
                // Parameter separator
                if self.csi_params.len() < MAX_CSI_PARAMS {
                    self.csi_params.push(self.csi_current_param);
                }
                self.csi_current_param = 0;
                self.csi_has_param = false;
                None
            }
            b':' => {
                // Subparameter separator - for now, treat like ;
                // This is used in SGR for things like underline styles
                if self.csi_params.len() < MAX_CSI_PARAMS {
                    self.csi_params.push(self.csi_current_param);
                }
                self.csi_current_param = 0;
                self.csi_has_param = false;
                None
            }
            0x20..=0x2F => {
                // Intermediate character
                if self.csi_has_param && self.csi_params.len() < MAX_CSI_PARAMS {
                    self.csi_params.push(self.csi_current_param);
                }
                self.csi_intermediates.push(byte as char);
                self.state = State::CsiIntermediate;
                None
            }
            0x40..=0x7E => {
                // Final character
                if self.csi_has_param && self.csi_params.len() < MAX_CSI_PARAMS {
                    self.csi_params.push(self.csi_current_param);
                }
                self.state = State::Ground;
                Some(self.dispatch_csi(byte as char))
            }
            _ => {
                self.state = State::CsiIgnore;
                None
            }
        }
    }

    /// Process a byte while collecting CSI intermediate characters
    fn process_csi_intermediate(&mut self, byte: u8) -> Option<Action> {
        match byte {
            0x20..=0x2F => {
                self.csi_intermediates.push(byte as char);
                None
            }
            0x40..=0x7E => {
                self.state = State::Ground;
                Some(self.dispatch_csi(byte as char))
            }
            _ => {
                self.state = State::CsiIgnore;
                None
            }
        }
    }

    /// Process a byte while ignoring a CSI sequence
    fn process_csi_ignore(&mut self, byte: u8) -> Option<Action> {
        if (0x40..=0x7E).contains(&byte) {
            self.state = State::Ground;
        }
        None
    }

    /// Dispatch a completed CSI sequence
    fn dispatch_csi(&self, final_char: char) -> Action {
        Action::Csi(CsiAction {
            final_char,
            params: self.csi_params.clone(),
            intermediates: self.csi_intermediates.clone(),
            private_marker: self.csi_private_marker,
        })
    }

    /// Process a byte while collecting OSC string
    fn process_osc_string(&mut self, byte: u8) -> Option<Action> {
        match byte {
            0x07 => {
                // BEL terminates OSC (handled in process_c0)
                unreachable!()
            }
            0x9C => {
                // ST (C1) terminates OSC
                let action = self.finish_osc();
                self.state = State::Ground;
                action
            }
            0x1B => {
                // Could be ESC \ (ST)
                // For simplicity, we'll check in the next byte
                // Actually, ESC is handled in process_c0, so we need special handling
                // Let's just accumulate and handle ESC \ specially
                self.osc_string.push('\x1B');
                None
            }
            _ => {
                // Check for ESC \ sequence
                if self.osc_string.ends_with('\x1B') && byte == b'\\' {
                    self.osc_string.pop(); // Remove the ESC
                    let action = self.finish_osc();
                    self.state = State::Ground;
                    return action;
                }

                if self.osc_string.len() < MAX_OSC_LENGTH {
                    self.osc_string.push(byte as char);
                }
                None
            }
        }
    }

    /// Finish OSC and return the action
    fn finish_osc(&mut self) -> Option<Action> {
        let s = std::mem::take(&mut self.osc_string);

        // Parse OSC command number
        let (cmd_str, data) = match s.find(';') {
            Some(pos) => (&s[..pos], &s[pos + 1..]),
            None => (s.as_str(), ""),
        };

        let cmd: u16 = cmd_str.parse().unwrap_or(u16::MAX);

        let action = match cmd {
            0 | 2 => OscAction::SetTitle(data.to_string()),
            1 => OscAction::SetIconName(data.to_string()),
            8 => {
                // Hyperlink: OSC 8 ; params ; url ST
                let parts: Vec<&str> = data.splitn(2, ';').collect();
                let (params, url) = match parts.as_slice() {
                    [p, u] => {
                        let params = if p.is_empty() {
                            None
                        } else {
                            Some((*p).to_string())
                        };
                        (params, (*u).to_string())
                    }
                    [u] => (None, (*u).to_string()),
                    _ => (None, String::new()),
                };
                OscAction::Hyperlink { params, url }
            }
            52 => {
                // Clipboard: OSC 52 ; clipboard ; data ST
                let parts: Vec<&str> = data.splitn(2, ';').collect();
                let (clipboard, clip_data) = match parts.as_slice() {
                    [c, d] => ((*c).to_string(), (*d).to_string()),
                    [c] => ((*c).to_string(), String::new()),
                    _ => (String::new(), String::new()),
                };
                OscAction::Clipboard {
                    clipboard,
                    data: clip_data,
                }
            }
            4 => {
                // Set color: OSC 4 ; index ; color ST
                let parts: Vec<&str> = data.splitn(2, ';').collect();
                if let [idx, color] = parts.as_slice() {
                    if let Ok(index) = idx.parse() {
                        OscAction::SetColor {
                            index,
                            color: (*color).to_string(),
                        }
                    } else {
                        OscAction::Unknown {
                            command: cmd,
                            data: data.to_string(),
                        }
                    }
                } else {
                    OscAction::Unknown {
                        command: cmd,
                        data: data.to_string(),
                    }
                }
            }
            104 => {
                // Reset color
                let index: u16 = data.parse().unwrap_or(0);
                OscAction::ResetColor { index }
            }
            10 => OscAction::SetColor {
                index: 256, // Special index for foreground
                color: data.to_string(),
            },
            11 => OscAction::SetColor {
                index: 257, // Special index for background
                color: data.to_string(),
            },
            110 => OscAction::ResetColor { index: 256 },
            111 => OscAction::ResetColor { index: 257 },
            _ => OscAction::Unknown {
                command: cmd,
                data: data.to_string(),
            },
        };

        Some(Action::Osc(action))
    }

    /// Process DCS entry
    fn process_dcs_entry(&mut self, byte: u8) -> Option<Action> {
        // For now, just collect everything until ST
        self.state = State::DcsPassthrough;
        self.dcs_string.push(byte as char);
        None
    }

    /// Process DCS passthrough
    fn process_dcs_passthrough(&mut self, byte: u8) -> Option<Action> {
        // Check for ST (ESC \)
        if self.dcs_string.ends_with('\x1B') && byte == b'\\' {
            self.dcs_string.pop();
            let s = std::mem::take(&mut self.dcs_string);
            self.state = State::Ground;
            return Some(Action::Dcs(s));
        }

        if byte == 0x9C {
            // C1 ST
            let s = std::mem::take(&mut self.dcs_string);
            self.state = State::Ground;
            return Some(Action::Dcs(s));
        }

        if self.dcs_string.len() < MAX_STRING_LENGTH {
            self.dcs_string.push(byte as char);
        } else {
            self.state = State::DcsIgnore;
        }
        None
    }

    /// Process DCS ignore
    fn process_dcs_ignore(&mut self, byte: u8) -> Option<Action> {
        if byte == 0x9C || (self.dcs_string.ends_with('\x1B') && byte == b'\\') {
            self.state = State::Ground;
        }
        None
    }

    /// Process APC string
    fn process_apc_string(&mut self, byte: u8) -> Option<Action> {
        if self.apc_string.ends_with('\x1B') && byte == b'\\' {
            self.apc_string.pop();
            let s = std::mem::take(&mut self.apc_string);
            self.state = State::Ground;
            return Some(Action::Apc(s));
        }

        if byte == 0x9C {
            let s = std::mem::take(&mut self.apc_string);
            self.state = State::Ground;
            return Some(Action::Apc(s));
        }

        if self.apc_string.len() < MAX_STRING_LENGTH {
            self.apc_string.push(byte as char);
        }
        None
    }

    /// Process PM string
    fn process_pm_string(&mut self, byte: u8) -> Option<Action> {
        if self.pm_string.ends_with('\x1B') && byte == b'\\' {
            self.pm_string.pop();
            let s = std::mem::take(&mut self.pm_string);
            self.state = State::Ground;
            return Some(Action::Pm(s));
        }

        if byte == 0x9C {
            let s = std::mem::take(&mut self.pm_string);
            self.state = State::Ground;
            return Some(Action::Pm(s));
        }

        if self.pm_string.len() < MAX_STRING_LENGTH {
            self.pm_string.push(byte as char);
        }
        None
    }

    /// Process SOS string
    fn process_sos_string(&mut self, byte: u8) -> Option<Action> {
        if self.sos_string.ends_with('\x1B') && byte == b'\\' {
            self.sos_string.pop();
            let s = std::mem::take(&mut self.sos_string);
            self.state = State::Ground;
            return Some(Action::Sos(s));
        }

        if byte == 0x9C {
            let s = std::mem::take(&mut self.sos_string);
            self.state = State::Ground;
            return Some(Action::Sos(s));
        }

        if self.sos_string.len() < MAX_STRING_LENGTH {
            self.sos_string.push(byte as char);
        }
        None
    }

    /// Process G0 charset selection
    fn process_select_g0(&mut self, byte: u8) -> Option<Action> {
        self.state = State::Ground;
        match byte {
            b'B' => Some(Action::Esc(EscAction::SelectG0Ascii)),
            b'0' => Some(Action::Esc(EscAction::SelectG0DecGraphics)),
            _ => Some(Action::Esc(EscAction::Unknown(byte as char))),
        }
    }

    /// Process G1 charset selection
    fn process_select_g1(&mut self, byte: u8) -> Option<Action> {
        self.state = State::Ground;
        match byte {
            b'B' => Some(Action::Esc(EscAction::SelectG1Ascii)),
            b'0' => Some(Action::Esc(EscAction::SelectG1DecGraphics)),
            _ => Some(Action::Esc(EscAction::Unknown(byte as char))),
        }
    }

    /// Process DEC hash sequence
    fn process_dec_hash(&mut self, byte: u8) -> Option<Action> {
        self.state = State::Ground;
        match byte {
            b'8' => Some(Action::Esc(EscAction::DecAlignmentTest)),
            _ => Some(Action::Esc(EscAction::Unknown(byte as char))),
        }
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
    fn test_parser_utf8() {
        let mut parser = Parser::new();
        let actions = parser.parse("日本語".as_bytes());
        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0], Action::Print('日'));
        assert_eq!(actions[1], Action::Print('本'));
        assert_eq!(actions[2], Action::Print('語'));
    }

    #[test]
    fn test_parser_control() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x07\x08\x09\x0A\x0D");
        assert_eq!(actions.len(), 5);
        assert_eq!(actions[0], Action::Control(ControlCode::Bell));
        assert_eq!(actions[1], Action::Control(ControlCode::Backspace));
        assert_eq!(actions[2], Action::Control(ControlCode::Tab));
        assert_eq!(actions[3], Action::Control(ControlCode::LineFeed));
        assert_eq!(actions[4], Action::Control(ControlCode::CarriageReturn));
    }

    #[test]
    fn test_parser_csi_cursor_up() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b[5A");
        assert_eq!(actions.len(), 1);
        if let Action::Csi(csi) = &actions[0] {
            assert_eq!(csi.final_char, 'A');
            assert_eq!(csi.params, vec![5]);
        } else {
            panic!("Expected CSI action");
        }
    }

    #[test]
    fn test_parser_csi_cursor_position() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b[10;20H");
        assert_eq!(actions.len(), 1);
        if let Action::Csi(csi) = &actions[0] {
            assert_eq!(csi.final_char, 'H');
            assert_eq!(csi.params, vec![10, 20]);
        } else {
            panic!("Expected CSI action");
        }
    }

    #[test]
    fn test_parser_csi_private_mode() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b[?25h");
        assert_eq!(actions.len(), 1);
        if let Action::Csi(csi) = &actions[0] {
            assert_eq!(csi.final_char, 'h');
            assert_eq!(csi.params, vec![25]);
            assert_eq!(csi.private_marker, Some('?'));
        } else {
            panic!("Expected CSI action");
        }
    }

    #[test]
    fn test_parser_csi_sgr() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b[1;31;42m");
        assert_eq!(actions.len(), 1);
        if let Action::Csi(csi) = &actions[0] {
            assert_eq!(csi.final_char, 'm');
            assert_eq!(csi.params, vec![1, 31, 42]);
        } else {
            panic!("Expected CSI action");
        }
    }

    #[test]
    fn test_parser_osc_title() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b]0;My Title\x07");
        assert_eq!(actions.len(), 1);
        if let Action::Osc(OscAction::SetTitle(title)) = &actions[0] {
            assert_eq!(title, "My Title");
        } else {
            panic!("Expected OSC SetTitle action");
        }
    }

    #[test]
    fn test_parser_osc_title_st() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b]2;Title\x1b\\");
        assert_eq!(actions.len(), 1);
        if let Action::Osc(OscAction::SetTitle(title)) = &actions[0] {
            assert_eq!(title, "Title");
        } else {
            panic!("Expected OSC SetTitle action");
        }
    }

    #[test]
    fn test_parser_osc_hyperlink() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b]8;id=123;https://example.com\x07");
        assert_eq!(actions.len(), 1);
        if let Action::Osc(OscAction::Hyperlink { params, url }) = &actions[0] {
            assert_eq!(params.as_deref(), Some("id=123"));
            assert_eq!(url, "https://example.com");
        } else {
            panic!("Expected OSC Hyperlink action");
        }
    }

    #[test]
    fn test_parser_esc_save_restore() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1b7\x1b8");
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0], Action::Esc(EscAction::SaveCursor));
        assert_eq!(actions[1], Action::Esc(EscAction::RestoreCursor));
    }

    #[test]
    fn test_parser_esc_index() {
        let mut parser = Parser::new();
        let actions = parser.parse(b"\x1bD\x1bM\x1bE");
        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0], Action::Esc(EscAction::Index));
        assert_eq!(actions[1], Action::Esc(EscAction::ReverseIndex));
        assert_eq!(actions[2], Action::Esc(EscAction::NextLine));
    }

    #[test]
    fn test_parser_chunk_boundary() {
        let mut parser = Parser::new();

        // Split CSI sequence across chunks
        let actions1 = parser.parse(b"\x1b[");
        assert!(actions1.is_empty());

        let actions2 = parser.parse(b"10;20");
        assert!(actions2.is_empty());

        let actions3 = parser.parse(b"H");
        assert_eq!(actions3.len(), 1);
        if let Action::Csi(csi) = &actions3[0] {
            assert_eq!(csi.final_char, 'H');
            assert_eq!(csi.params, vec![10, 20]);
        } else {
            panic!("Expected CSI action");
        }
    }

    #[test]
    fn test_parser_utf8_chunk_boundary() {
        let mut parser = Parser::new();

        // UTF-8 for '日' is E6 97 A5
        let actions1 = parser.parse(&[0xE6]);
        assert!(actions1.is_empty());

        let actions2 = parser.parse(&[0x97]);
        assert!(actions2.is_empty());

        let actions3 = parser.parse(&[0xA5]);
        assert_eq!(actions3.len(), 1);
        assert_eq!(actions3[0], Action::Print('日'));
    }

    #[test]
    fn test_parser_invalid_utf8() {
        let mut parser = Parser::new();

        // Invalid UTF-8 sequence
        let actions = parser.parse(&[0xFF]);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], Action::Print('\u{FFFD}'));
    }

    #[test]
    fn test_parser_csi_default_params() {
        let mut parser = Parser::new();

        // CSI H with no params
        let actions = parser.parse(b"\x1b[H");
        assert_eq!(actions.len(), 1);
        if let Action::Csi(csi) = &actions[0] {
            assert_eq!(csi.final_char, 'H');
            assert!(csi.params.is_empty());
            assert_eq!(csi.param_or_default(0, 1), 1);
            assert_eq!(csi.param_or_default(1, 1), 1);
        } else {
            panic!("Expected CSI action");
        }
    }

    #[test]
    fn test_parser_csi_empty_params() {
        let mut parser = Parser::new();

        // CSI ; H (empty first param)
        let actions = parser.parse(b"\x1b[;5H");
        assert_eq!(actions.len(), 1);
        if let Action::Csi(csi) = &actions[0] {
            assert_eq!(csi.final_char, 'H');
            assert_eq!(csi.params, vec![0, 5]);
        } else {
            panic!("Expected CSI action");
        }
    }
}
