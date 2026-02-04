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
use std::path::PathBuf;
use std::process;

use app::App;
use clap::Parser;
use config::Config;

/// Mochi Terminal Emulator - A VT/xterm-compatible terminal built from scratch
#[derive(Parser, Debug)]
#[command(name = "mochi")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to configuration file (overrides default XDG location)
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Font family name
    #[arg(long, value_name = "FONT")]
    font_family: Option<String>,

    /// Font size in points
    #[arg(long, value_name = "SIZE")]
    font_size: Option<f32>,

    /// Theme name (dark, light, solarized-dark, solarized-light, dracula, nord)
    #[arg(short, long, value_name = "THEME")]
    theme: Option<String>,

    /// Shell command to run
    #[arg(short, long, value_name = "SHELL")]
    shell: Option<String>,

    /// Initial columns
    #[arg(long, value_name = "COLS")]
    columns: Option<u16>,

    /// Initial rows
    #[arg(long, value_name = "ROWS")]
    rows: Option<u16>,

    /// Number of scrollback lines
    #[arg(long, value_name = "LINES")]
    scrollback: Option<usize>,

    /// Enable OSC 52 clipboard support (disabled by default for security)
    #[arg(long)]
    osc52_clipboard: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Mochi Terminal");

    // Parse CLI arguments
    let args = Args::parse();

    // Load configuration with precedence: CLI > env > file > defaults
    let config = match Config::load_with_args(&args) {
        Ok(config) => config,
        Err(e) => {
            log::error!("Configuration error: {}", e);
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    log::debug!("Effective configuration: {:?}", config);

    // Run the application
    let app = App::new(config)?;
    app.run()?;

    log::info!("Mochi Terminal exited");
    Ok(())
}
