//! PTY (Pseudo-Terminal) Module
//!
//! Handles creation and management of pseudo-terminals on Linux.
//! This module provides the interface between the terminal emulator
//! and child processes (shells, programs).
//!
//! # Overview
//!
//! A PTY consists of two parts:
//! - Master: The controlling side (our terminal emulator)
//! - Slave: The controlled side (the child process's terminal)
//!
//! Data written to the master appears as input to the slave, and
//! output from the slave can be read from the master.

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::{Pty, PtyError, WindowSize};

#[cfg(not(target_os = "linux"))]
compile_error!("This terminal emulator only supports Linux");
