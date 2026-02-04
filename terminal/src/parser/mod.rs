//! Terminal Escape Sequence Parser
//!
//! A stateful parser that converts bytes into semantic terminal actions.
//! Implements a state machine based on the VT500-series parser model.
//!
//! # Design
//!
//! The parser is designed for incremental streaming: it can accept arbitrary
//! chunk boundaries and will correctly handle sequences split across chunks.
//!
//! # References
//!
//! - ECMA-48 (ISO 6429): Control Functions for Coded Character Sets
//! - Xterm Control Sequences: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
//! - VT220 Programmer Reference Manual

mod actions;
mod state;

pub use actions::{Action, CsiAction, EscAction, OscAction, SgrAttribute};
pub use state::Parser;
