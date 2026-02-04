//! Mochi PTY (Pseudo-Terminal) Module
//!
//! This crate provides PTY handling for Linux, including:
//! - Opening PTY master/slave pairs
//! - Spawning child processes attached to PTYs
//! - Resizing PTYs (TIOCSWINSZ)
//! - Non-blocking I/O
//!
//! References:
//! - pty(7): https://man7.org/linux/man-pages/man7/pty.7.html
//! - posix_openpt(3): https://www.man7.org/linux/man-pages/man3/posix_openpt.3.html

pub mod error;
pub mod pty;

pub use error::PtyError;
pub use pty::{Pty, PtySize};
