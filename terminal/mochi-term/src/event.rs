//! Event handling for the terminal application

use std::time::Duration;

/// Events that can occur in the terminal application
#[derive(Debug)]
pub enum TerminalEvent {
    /// Data received from PTY
    PtyOutput(Vec<u8>),
    /// Child process exited
    ChildExited(i32),
    /// Window resize
    Resize { cols: u16, rows: u16 },
    /// Bell triggered
    Bell,
    /// Title changed
    TitleChanged(String),
    /// Redraw needed
    Redraw,
}

/// Event loop timing
pub struct EventTiming {
    /// Target frame rate
    pub target_fps: u32,
    /// PTY poll interval
    pub pty_poll_interval: Duration,
}

impl Default for EventTiming {
    fn default() -> Self {
        Self {
            target_fps: 60,
            pty_poll_interval: Duration::from_millis(1),
        }
    }
}

impl EventTiming {
    /// Get frame duration
    pub fn frame_duration(&self) -> Duration {
        Duration::from_secs_f64(1.0 / self.target_fps as f64)
    }
}
