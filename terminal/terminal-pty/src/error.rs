//! Error types for PTY operations

use std::io;
use thiserror::Error;

/// PTY error type
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// PTY creation failed
    #[error("Failed to create PTY: {0}")]
    PtyCreation(String),

    /// Failed to spawn child process
    #[error("Failed to spawn child: {0}")]
    SpawnFailed(String),

    /// Failed to set window size
    #[error("Failed to set window size: {0}")]
    WindowSize(String),

    /// Child process error
    #[error("Child process error: {0}")]
    ChildError(String),

    /// Nix error
    #[error("System error: {0}")]
    Nix(#[from] nix::Error),
}

/// Result type for PTY operations
pub type Result<T> = std::result::Result<T, Error>;
