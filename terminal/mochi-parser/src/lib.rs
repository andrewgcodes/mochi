//! Mochi Terminal Parser
//!
//! This crate implements a VT/xterm-compatible escape sequence parser.
//! It converts a stream of bytes into semantic terminal actions.
//!
//! The parser is:
//! - Stateful: maintains parsing state across chunk boundaries
//! - Streaming: can accept arbitrary chunk sizes
//! - Deterministic: same input always produces same output
//!
//! Supported sequences:
//! - C0 control characters (BEL, BS, HT, LF, CR, ESC, etc.)
//! - ESC sequences (cursor save/restore, charset selection, etc.)
//! - CSI sequences (cursor movement, erase, SGR, modes, etc.)
//! - OSC sequences (window title, hyperlinks, clipboard)
//! - DCS sequences (consumed but not fully implemented)

pub mod action;
pub mod params;
pub mod parser;

pub use action::Action;
pub use params::Params;
pub use parser::Parser;
