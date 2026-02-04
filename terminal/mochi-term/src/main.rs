//! Mochi Terminal Emulator
//!
//! A VT/xterm-compatible terminal emulator built from scratch.

mod app;
mod config;
mod event;
mod input;
mod renderer;
mod terminal;

use std::error::Error;

use app::App;
use config::Config;

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Mochi Terminal");

    // Load configuration
    let config = Config::load().unwrap_or_default();

    // Run the application
    let mut app = App::new(config)?;
    app.run()?;

    log::info!("Mochi Terminal exited");
    Ok(())
}
