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
use config::{CliArgs, Config};

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Mochi Terminal");

    // Parse CLI arguments
    let args = CliArgs::parse_args();

    // Load configuration with full precedence handling:
    // CLI flags > environment variables > config file > built-in defaults
    let config = match Config::load_with_args(&args) {
        Ok(config) => {
            log::info!("Configuration loaded successfully");
            if let Some(path) = Config::default_config_path() {
                log::debug!("Config path: {:?}", path);
            }
            config
        }
        Err(e) => {
            log::warn!("Failed to load configuration: {}. Using defaults.", e);
            Config::default()
        }
    };

    log::debug!(
        "Config: font_size={}, theme={:?}, dimensions={:?}",
        config.font_size,
        config.theme,
        config.dimensions
    );

    // Run the application
    let app = App::new(config)?;
    app.run()?;

    log::info!("Mochi Terminal exited");
    Ok(())
}
