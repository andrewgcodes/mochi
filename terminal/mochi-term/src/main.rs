//! Mochi Terminal Emulator
//!
//! A VT/xterm-compatible terminal emulator built from scratch.

mod app;
mod config;
mod event;
mod gpu_renderer;
mod input;
mod renderer;
mod terminal;

use std::error::Error;

use app::App;
use clap::Parser;
use config::{CliArgs, Config};

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging with "warn" level by default for faster startup
    // Users can set RUST_LOG=info or RUST_LOG=debug for more verbose output
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    log::debug!("Starting Mochi Terminal");

    // Parse CLI arguments
    let args = CliArgs::parse();

    // Load configuration with precedence: CLI > env > file > defaults
    let config = match Config::load_with_args(&args) {
        Ok(config) => config,
        Err(e) => {
            log::error!("Configuration error: {}", e);
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    log::debug!("Theme: {:?}", config.theme);
    log::debug!("Font: {} @ {}pt", config.font.family, config.font.size);

    // Run the application
    let app = App::new(config)?;
    app.run()?;

    log::debug!("Mochi Terminal exited");
    Ok(())
}
