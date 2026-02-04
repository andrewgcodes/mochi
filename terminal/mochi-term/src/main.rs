//! Mochi Terminal Emulator
//!
//! A VT/xterm-compatible terminal emulator built from scratch.

mod app;
mod config;
mod event;
mod input;
mod renderer;
mod terminal;

use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::process;

use app::App;
use config::{CliArgs, Config, ThemeName};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const HELP_TEXT: &str = r#"Mochi Terminal - A VT/xterm-compatible terminal emulator

USAGE:
    mochi [OPTIONS]

OPTIONS:
    -c, --config <PATH>     Path to config file (default: ~/.config/mochi/config.toml)
    -f, --font-size <SIZE>  Font size in points (default: 14.0)
    -t, --theme <THEME>     Theme name: dark, light, solarized-dark, solarized-light, dracula, nord
    -s, --shell <SHELL>     Shell command to run (default: $SHELL)
    -h, --help              Print help information
    -V, --version           Print version information

ENVIRONMENT VARIABLES:
    MOCHI_FONT_SIZE         Override font size
    MOCHI_THEME             Override theme
    MOCHI_SHELL             Override shell command
    MOCHI_SCROLLBACK_LINES  Override scrollback buffer size
    MOCHI_OSC52_CLIPBOARD   Enable OSC 52 clipboard (set to "1" or "true")

CONFIG FILE:
    Default location: ~/.config/mochi/config.toml (follows XDG conventions)
    See docs/terminal/config.md for configuration options.

KEYBINDINGS:
    Ctrl+Shift+C            Copy selection to clipboard
    Ctrl+Shift+V            Paste from clipboard
    Ctrl+Shift+F            Open search bar
    Ctrl+Shift+R            Reload configuration
    Ctrl+Shift+T            Toggle theme (cycle through themes)
    Ctrl+=/-                Zoom in/out
    Ctrl+0                  Reset zoom
"#;

/// Parse command line arguments
fn parse_args() -> Result<CliArgs, String> {
    let args: Vec<String> = env::args().collect();
    let mut cli_args = CliArgs::default();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("{}", HELP_TEXT);
                process::exit(0);
            }
            "-V" | "--version" => {
                println!("mochi {}", VERSION);
                process::exit(0);
            }
            "-c" | "--config" => {
                i += 1;
                if i >= args.len() {
                    return Err("--config requires a path argument".to_string());
                }
                cli_args.config_path = Some(PathBuf::from(&args[i]));
            }
            "-f" | "--font-size" => {
                i += 1;
                if i >= args.len() {
                    return Err("--font-size requires a numeric argument".to_string());
                }
                cli_args.font_size = Some(
                    args[i]
                        .parse()
                        .map_err(|_| format!("Invalid font size: {}", args[i]))?,
                );
            }
            "-t" | "--theme" => {
                i += 1;
                if i >= args.len() {
                    return Err("--theme requires a theme name argument".to_string());
                }
                cli_args.theme = Some(ThemeName::from_str(&args[i])
                    .ok_or_else(|| format!("Unknown theme: {}. Valid themes: dark, light, solarized-dark, solarized-light, dracula, nord", args[i]))?);
            }
            "-s" | "--shell" => {
                i += 1;
                if i >= args.len() {
                    return Err("--shell requires a shell command argument".to_string());
                }
                cli_args.shell = Some(args[i].clone());
            }
            arg if arg.starts_with('-') => {
                return Err(format!("Unknown option: {}. Use --help for usage.", arg));
            }
            _ => {
                return Err(format!(
                    "Unexpected argument: {}. Use --help for usage.",
                    args[i]
                ));
            }
        }
        i += 1;
    }

    Ok(cli_args)
}

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Mochi Terminal v{}", VERSION);

    // Parse command line arguments
    let cli_args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    // Load configuration with precedence handling
    let config = match Config::load_with_args(&cli_args) {
        Ok(config) => {
            log::info!("Configuration loaded successfully");
            config
        }
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            log::warn!("Using default configuration due to error: {}", e);
            Config::default()
        }
    };

    log::debug!("Font size: {}, Theme: {:?}", config.font_size, config.theme);

    // Run the application
    let app = App::new(config)?;
    app.run()?;

    log::info!("Mochi Terminal exited");
    Ok(())
}
