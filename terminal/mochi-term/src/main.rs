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

use app::App;
use clap::Parser;
use config::{CliOverrides, Config, ThemeName};

/// Mochi Terminal - A modern, customizable terminal emulator
#[derive(Parser, Debug)]
#[command(name = "mochi")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file (default: ~/.config/mochi/config.toml)
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Theme to use (dark, light, solarized-dark, solarized-light, dracula, nord, gruvbox, onedark)
    #[arg(short, long)]
    theme: Option<String>,

    /// Font size in points
    #[arg(long)]
    font_size: Option<f32>,

    /// Font family name
    #[arg(long)]
    font_family: Option<String>,

    /// Shell command to run
    #[arg(short, long)]
    shell: Option<String>,

    /// Number of scrollback lines
    #[arg(long)]
    scrollback: Option<usize>,
}

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Mochi Terminal");

    // Parse command line arguments
    let args = Args::parse();

    // Build CLI overrides from arguments
    let cli_overrides = CliOverrides {
        theme: args
            .theme
            .as_ref()
            .and_then(|t| ThemeName::from_str(t).ok()),
        font_size: args.font_size,
        font_family: args.font_family,
        shell: args.shell,
        scrollback: args.scrollback,
    };

    // Load configuration with CLI overrides
    let config = match Config::load_with_overrides(args.config.clone(), cli_overrides) {
        Ok(cfg) => cfg,
        Err(e) => {
            log::error!("Failed to load configuration: {}", e);
            if args.config.is_some() {
                return Err(e.into());
            }
            log::warn!("Using default configuration");
            Config::default()
        }
    };

    log::info!("Theme: {}", config.theme.display_name());
    log::info!("Font: {} @ {}pt", config.font_family(), config.font_size());

    // Run the application
    let app = App::new(config)?;
    app.run()?;

    log::info!("Mochi Terminal exited");
    Ok(())
}
