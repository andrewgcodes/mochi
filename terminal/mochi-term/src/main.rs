//! Mochi Terminal Emulator
//!
//! A real VT/xterm-style terminal emulator built from scratch.
//! This is the main entry point that ties together:
//! - mochi-core: Screen model and terminal state
//! - mochi-parser: Escape sequence parsing
//! - mochi-pty: PTY management and child process
//! - GUI rendering with winit and wgpu

mod config;
mod event_loop;
mod input;
mod performer;
mod renderer;

use std::io::{self, Read, Write};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use log::{debug, error, info, warn};
use mochi_core::Term;
use mochi_parser::Parser;
use mochi_pty::{Child, WindowSize};

use crate::config::Config;
use crate::event_loop::run_event_loop;
use crate::performer::Performer;

/// Terminal application state
pub struct App {
    /// Terminal state
    pub term: Term,
    /// Parser for escape sequences
    pub parser: Parser,
    /// Child process
    pub child: Option<Child>,
    /// Configuration
    pub config: Config,
    /// Window title (from OSC)
    pub title: String,
    /// Whether the terminal needs redrawing
    pub dirty: bool,
    /// Last time we received data
    pub last_activity: Instant,
}

impl App {
    /// Create a new terminal application
    pub fn new(config: Config) -> io::Result<Self> {
        let size = WindowSize::new(config.rows as u16, config.cols as u16);
        let child = Child::spawn_shell_with_size(size)?;

        Ok(App {
            term: Term::new(config.rows, config.cols),
            parser: Parser::new(),
            child: Some(child),
            config,
            title: String::from("Mochi Terminal"),
            dirty: true,
            last_activity: Instant::now(),
        })
    }

    /// Process input from the PTY
    pub fn process_pty_input(&mut self, data: &[u8]) {
        let actions = self.parser.parse(data);
        let mut performer = Performer::new(&mut self.term);

        for action in actions {
            performer.perform(action);
        }

        // Check if title changed
        if !self.term.title.is_empty() && self.term.title != self.title {
            self.title = self.term.title.clone();
        }

        self.dirty = true;
        self.last_activity = Instant::now();
    }

    /// Send input to the PTY
    pub fn send_input(&mut self, data: &[u8]) -> io::Result<()> {
        if let Some(ref mut child) = self.child {
            child.write_all(data)?;
        }
        Ok(())
    }

    /// Resize the terminal
    pub fn resize(&mut self, rows: usize, cols: usize) -> io::Result<()> {
        self.term.resize(rows, cols);
        if let Some(ref mut child) = self.child {
            child.resize(WindowSize::new(rows as u16, cols as u16))?;
        }
        self.dirty = true;
        Ok(())
    }

    /// Check if the child process has exited
    pub fn check_child(&mut self) -> Option<i32> {
        if let Some(ref child) = self.child {
            if let Ok(Some(status)) = child.try_wait() {
                return Some(status.code().unwrap_or(0));
            }
        }
        None
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    info!("Starting Mochi Terminal");

    // Load configuration
    let config = Config::default();
    info!("Configuration: {}x{}", config.cols, config.rows);

    // Create the application
    let app = App::new(config)?;

    // Run the event loop
    run_event_loop(app)?;

    info!("Mochi Terminal exited");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        // This test requires a PTY, so it may fail in some CI environments
        let config = Config::default();
        if let Ok(app) = App::new(config) {
            assert_eq!(app.term.rows(), 24);
            assert_eq!(app.term.cols(), 80);
        }
    }
}
