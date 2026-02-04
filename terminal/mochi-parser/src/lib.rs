//! Mochi Terminal Parser
//!
//! This crate implements a VT/xterm escape sequence parser using a state machine.
//! It converts a stream of bytes into semantic terminal actions.
//!
//! The parser is:
//! - Streaming: can handle arbitrary chunk boundaries
//! - Stateful: maintains parser state between chunks
//! - Deterministic: same input always produces same output
//!
//! Supported sequences:
//! - C0 control characters (BEL, BS, HT, LF, VT, FF, CR, ESC)
//! - ESC sequences (DECSC, DECRC, IND, RI, NEL, HTS, charset selection)
//! - CSI sequences (cursor movement, erase, SGR, modes, scroll region)
//! - OSC sequences (window title, hyperlinks, clipboard)
//! - DCS sequences (consumed but not fully implemented)

pub mod action;
pub mod params;
pub mod parser;

pub use action::Action;
pub use params::Params;
pub use parser::Parser;
