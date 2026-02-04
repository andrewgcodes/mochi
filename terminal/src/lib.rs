//! Mochi Terminal Emulator
//!
//! A real Linux terminal emulator built from scratch, implementing VT/xterm-style
//! terminal emulation with proper PTY handling, escape sequence parsing, screen
//! state management, and GPU-accelerated rendering.
//!
//! # Architecture
//!
//! The terminal is structured as several independent modules:
//!
//! - `core`: Platform-independent screen model (cells, lines, screen, scrollback)
//! - `parser`: Escape sequence parser (CSI, OSC, DCS, etc.)
//! - `pty`: Linux PTY handling (posix_openpt, fork/exec, resize)
//! - `frontend`: GUI rendering (window, renderer, input encoding)
//! - `app`: Application glue (config, logging)
//!
//! # Example
//!
//! ```no_run
//! use mochi_terminal::core::Screen;
//! use mochi_terminal::parser::Parser;
//!
//! // Create a terminal screen
//! let mut screen = Screen::new(80, 24);
//!
//! // Create a parser
//! let mut parser = Parser::new();
//!
//! // Feed bytes and apply actions to screen
//! let input = b"Hello, \x1b[31mWorld\x1b[0m!";
//! for action in parser.feed(input) {
//!     screen.apply(action);
//! }
//! ```

pub mod app;
pub mod core;
pub mod frontend;
pub mod parser;
pub mod pty;

/// Re-export commonly used types
pub use core::{Cell, CellAttributes, Color, Cursor, CursorStyle, Line, Screen};
pub use parser::{Parser, TerminalAction};
#[cfg(unix)]
pub use pty::{Pty, PtyError, PtyResult, WindowSize};
