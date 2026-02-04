//! Mochi Terminal Emulator Library
//!
//! A real Linux terminal emulator built from scratch, implementing VT/xterm
//! escape sequence parsing, screen model management, and PTY handling.
//!
//! # Architecture
//!
//! The library is organized into several modules:
//!
//! - `core`: Platform-independent terminal core (screen model, cells, cursor, scrollback)
//! - `parser`: Stateful escape sequence parser (ESC, CSI, OSC, DCS)
//! - `pty`: Linux PTY and child process management
//! - `input`: Keyboard and mouse input encoding
//! - `renderer`: GUI rendering (window, fonts, grid display)
//!
//! # Design Principles
//!
//! 1. **Deterministic Core**: Given the same byte stream, the terminal core
//!    produces identical screen snapshots. The GUI only renders snapshots.
//!
//! 2. **No Terminal Libraries**: This implementation does not use any terminal
//!    emulation libraries (libvte, termwiz, etc.). All parsing and screen
//!    management is implemented from scratch.
//!
//! 3. **Streaming Parser**: The parser handles arbitrary chunk boundaries,
//!    allowing incremental processing of PTY output.
//!
//! # Supported Features
//!
//! See `docs/escape-sequences.md` for the full coverage matrix.

pub mod core;
pub mod input;
pub mod parser;
pub mod pty;
pub mod renderer;
mod terminal;

pub use core::{Cell, Color, Cursor, Screen, Scrollback, Snapshot, Style};
pub use parser::{Action, Parser, SgrAttribute};
pub use pty::Pty;
pub use terminal::Terminal;
