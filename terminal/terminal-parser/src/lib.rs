//! Terminal Parser - VT/xterm escape sequence parser
//!
//! This crate implements a streaming parser for terminal escape sequences.
//! It converts a byte stream into semantic terminal actions.
//!
//! The parser is designed to:
//! - Handle arbitrary chunk boundaries (streaming)
//! - Be deterministic
//! - Support UTF-8 text
//! - Parse CSI, OSC, ESC, and DCS sequences
//!
//! Reference: https://www.x.org/docs/xterm/ctlseqs.pdf

mod action;
pub mod kitty;
mod params;
mod parser;
pub mod sixel;
mod utf8;

pub use action::{Action, CsiAction, EscAction, OscAction};
pub use kitty::KittyAction;
pub use params::Params;
pub use parser::{Parser, ParserState};
pub use sixel::SixelImage;
