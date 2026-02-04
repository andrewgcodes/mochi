//! Mochi Terminal Emulator
//!
//! A terminal emulator built from scratch, implementing VT/xterm-compatible
//! escape sequence handling with a modern GPU-accelerated GUI.

use env_logger::Env;
use log::{error, info};

mod input;
mod performer;
mod renderer;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    info!("Mochi Terminal starting...");

    let cols = 80;
    let rows = 24;

    if let Err(e) = renderer::run_terminal(cols, rows) {
        error!("Fatal error: {}", e);
        std::process::exit(1);
    }

    info!("Mochi Terminal exiting");
}
