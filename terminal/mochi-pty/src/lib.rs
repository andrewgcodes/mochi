//! Mochi PTY Management
//!
//! This crate provides PTY (pseudo-terminal) management for Linux.
//! It handles:
//! - Creating PTY master/slave pairs
//! - Spawning child processes attached to the PTY
//! - Non-blocking I/O with the PTY
//! - Window size management (SIGWINCH)
//!
//! This is Linux-specific and uses the POSIX PTY APIs.

pub mod child;
pub mod pty;
pub mod size;

pub use child::Child;
pub use pty::Pty;
pub use size::WindowSize;
