//! PTY (Pseudoterminal) handling for Linux
//!
//! This module provides functionality for creating and managing pseudoterminals,
//! spawning child processes, and handling I/O.

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use unix::Pty;

/// Error type for PTY operations
#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    #[error("Failed to open PTY master: {0}")]
    OpenMaster(#[source] nix::Error),

    #[error("Failed to grant PTY access: {0}")]
    GrantPty(#[source] nix::Error),

    #[error("Failed to unlock PTY: {0}")]
    UnlockPty(#[source] nix::Error),

    #[error("Failed to get PTY slave name: {0}")]
    PtsName(#[source] nix::Error),

    #[error("Failed to open PTY slave: {0}")]
    OpenSlave(#[source] nix::Error),

    #[error("Failed to fork: {0}")]
    Fork(#[source] nix::Error),

    #[error("Failed to create session: {0}")]
    Setsid(#[source] nix::Error),

    #[error("Failed to set controlling terminal: {0}")]
    SetControllingTerminal(#[source] nix::Error),

    #[error("Failed to duplicate file descriptor: {0}")]
    Dup2(#[source] nix::Error),

    #[error("Failed to execute shell: {0}")]
    Exec(#[source] nix::Error),

    #[error("Failed to set window size: {0}")]
    SetWinsize(#[source] nix::Error),

    #[error("Failed to read from PTY: {0}")]
    Read(#[source] nix::Error),

    #[error("Failed to write to PTY: {0}")]
    Write(#[source] nix::Error),

    #[error("Failed to set non-blocking mode: {0}")]
    SetNonBlocking(#[source] nix::Error),

    #[error("Failed to poll: {0}")]
    Poll(#[source] nix::Error),

    #[error("Failed to wait for child: {0}")]
    Wait(#[source] nix::Error),

    #[error("Child process exited with status: {0}")]
    ChildExited(i32),

    #[error("Child process killed by signal: {0}")]
    ChildSignaled(i32),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for PTY operations
pub type PtyResult<T> = Result<T, PtyError>;

/// Window size for PTY
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl WindowSize {
    /// Create a new window size with just rows and columns
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }
    }

    /// Create a new window size with pixel dimensions
    pub fn with_pixels(cols: u16, rows: u16, pixel_width: u16, pixel_height: u16) -> Self {
        Self {
            rows,
            cols,
            pixel_width,
            pixel_height,
        }
    }
}

impl Default for WindowSize {
    fn default() -> Self {
        Self::new(80, 24)
    }
}
