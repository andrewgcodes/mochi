//! Terminal escape sequence parser
//!
//! A stateful parser that converts bytes into terminal actions.
//! Based on the VT500-series parser model from <https://vt100.net/emu/dec_ansi_parser>

mod actions;
mod state;

pub use actions::TerminalAction;
pub use state::Parser;
