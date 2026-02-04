//! Error types for PTY operations.

use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PtyError {
    #[error("Failed to open PTY master: {0}")]
    OpenMaster(#[source] nix::Error),

    #[error("Failed to grant PTY access: {0}")]
    GrantPty(#[source] nix::Error),

    #[error("Failed to unlock PTY: {0}")]
    UnlockPty(#[source] nix::Error),

    #[error("Failed to get slave name: {0}")]
    GetSlaveName(#[source] nix::Error),

    #[error("Failed to open slave PTY: {0}")]
    OpenSlave(#[source] nix::Error),

    #[error("Failed to fork process: {0}")]
    Fork(#[source] nix::Error),

    #[error("Failed to create session: {0}")]
    Setsid(#[source] nix::Error),

    #[error("Failed to set controlling terminal: {0}")]
    SetControllingTerminal(#[source] nix::Error),

    #[error("Failed to execute shell: {0}")]
    Exec(#[source] nix::Error),

    #[error("Failed to set window size: {0}")]
    SetWindowSize(#[source] nix::Error),

    #[error("Failed to set non-blocking mode: {0}")]
    SetNonBlocking(#[source] nix::Error),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Child process exited with status: {0}")]
    ChildExited(i32),

    #[error("Child process was killed by signal: {0}")]
    ChildKilled(i32),

    #[error("Environment variable error: {0}")]
    EnvVar(String),
}
