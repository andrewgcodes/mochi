//! Mochi Terminal Emulator
//!
//! A VT/xterm-compatible terminal emulator built from scratch.

mod app;
mod config;
mod event;
mod input;
mod renderer;
mod terminal;

use std::process;

use app::App;
use config::{CliArgs, Config};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = CliArgs::parse();

    if cli.help {
        CliArgs::print_help();
        return;
    }

    if cli.version {
        CliArgs::print_version();
        return;
    }

    log::info!("Starting Mochi Terminal");

    let config = match Config::load_with_args(&cli) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error loading configuration: {}", e);
            log::error!("Configuration error: {}", e);
            process::exit(1);
        }
    };

    let app = match App::new(config) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Error initializing terminal: {}", e);
            log::error!("Initialization error: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = app.run() {
        eprintln!("Error running terminal: {}", e);
        log::error!("Runtime error: {}", e);
        process::exit(1);
    }

    log::info!("Mochi Terminal exited");
}
