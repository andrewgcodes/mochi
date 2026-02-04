//! Terminal PTY - Linux pseudoterminal management
//!
//! This crate provides PTY (pseudoterminal) functionality for spawning
//! and managing child processes in a terminal emulator.
//!
//! Key features:
//! - PTY creation and management
//! - Child process spawning with proper session setup
//! - Non-blocking I/O
//! - Window size management (TIOCSWINSZ)
//!
//! Reference: https://www.man7.org/linux/man-pages/man3/posix_openpt.3.html

mod child;
mod error;
mod pty;
mod size;

pub use child::Child;
pub use error::{Error, Result};
pub use pty::Pty;
pub use size::WindowSize;
