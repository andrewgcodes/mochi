//! PTY (Pseudo-Terminal) Module
//!
//! Handles creation and management of pseudo-terminals on Unix systems
//! (Linux and macOS). This module provides the interface between the
//! terminal emulator and child processes (shells, programs).
//!
//! # Overview
//!
//! A PTY consists of two parts:
//! - Master: The controlling side (our terminal emulator)
//! - Slave: The controlled side (the child process's terminal)
//!
//! Data written to the master appears as input to the slave, and
//! output from the slave can be read from the master.

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix;

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use unix::{Pty, PtyError, WindowSize};

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
compile_error!("This terminal emulator only supports Linux and macOS");
