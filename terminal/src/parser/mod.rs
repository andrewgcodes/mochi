//! Terminal Parser Module
//!
//! A stateful parser that converts bytes into semantic terminal operations.
//! Implements VT/xterm escape sequence parsing according to ECMA-48 and
//! xterm control sequences documentation.
//!
//! The parser is designed to:
//! - Handle incremental streaming (arbitrary chunk boundaries)
//! - Be deterministic (same input always produces same output)
//! - Never panic on invalid input
//! - Correctly handle UTF-8 decoding

mod action;
mod state;

pub use action::{Action, ControlCode, CsiAction, EscAction, OscAction};
pub use state::Parser;

/// Maximum length for OSC string payloads (security limit)
pub const MAX_OSC_LENGTH: usize = 65536;

/// Maximum number of CSI parameters
pub const MAX_CSI_PARAMS: usize = 32;

/// Maximum length for DCS/APC/PM/SOS strings
pub const MAX_STRING_LENGTH: usize = 65536;
