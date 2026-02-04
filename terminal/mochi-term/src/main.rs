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
use clap::Parser;
use config::{CliArgs, Config};

fn main() -> Result<(), Box<dyn Error>> {
    // Parse CLI arguments
    let args = CliArgs::parse();

    // Initialize logging with appropriate level
    let log_level = if args.debug { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    log::info!("Starting Mochi Terminal");

    // Load configuration with CLI overrides
    let config = match Config::load_with_args(&args) {
        Ok(config) => {
            log::info!("Configuration loaded successfully");
            log::debug!("Theme: {:?}", config.theme);
            log::debug!("Font: {} @ {}pt", config.font.family, config.font.size);
            config
        }
        Err(e) => {
            log::error!("Failed to load configuration: {}", e);
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    // Run the application
    let app = App::new(config, args)?;
    app.run()?;

    log::info!("Mochi Terminal exited");
    Ok(())
}
