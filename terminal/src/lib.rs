//! Mochi Terminal Emulator Library
//!
//! A terminal emulator built from scratch without using any terminal emulation libraries.
//! This crate provides the core functionality for terminal emulation including:
//!
//! - `core`: Screen model, cells, cursor, scrollback buffer
//! - `parser`: VT/xterm escape sequence parser
//! - `pty`: Linux PTY management
//! - `gui`: GUI rendering (optional feature)

pub mod core;
pub mod parser;
pub mod pty;
