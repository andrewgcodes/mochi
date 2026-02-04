//! Mochi Terminal Emulator
//!
//! A terminal emulator built from scratch, implementing VT/xterm-compatible
//! escape sequence handling with a modern GPU-accelerated GUI.

use env_logger::Env;
use log::{error, info};
use std::env;

mod config;
mod input;
mod performer;
mod renderer;

fn print_help() {
    println!("Mochi Terminal Emulator v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE:");
    println!("    mochi [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help           Print this help message");
    println!("    -v, --version        Print version information");
    println!("    -c, --config PATH    Use a custom config file");
    println!("    --print-config       Print the default configuration");
    println!();
    println!("CONFIGURATION:");
    println!("    Config file is loaded from:");
    println!("    1. $XDG_CONFIG_HOME/mochi/config.toml");
    println!("    2. ~/.config/mochi/config.toml");
    println!();
    println!("ENVIRONMENT:");
    println!("    TERM=xterm-256color is recommended");
    println!();
    println!("For more information, see: https://github.com/andrewgcodes/mochi");
}

fn print_version() {
    println!("mochi {}", env!("CARGO_PKG_VERSION"));
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args: Vec<String> = env::args().collect();
    let mut config_path: Option<std::path::PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-v" | "--version" => {
                print_version();
                return;
            }
            "--print-config" => {
                println!("{}", config::Config::default_config_toml());
                return;
            }
            "-c" | "--config" => {
                if i + 1 < args.len() {
                    config_path = Some(std::path::PathBuf::from(&args[i + 1]));
                    i += 1;
                } else {
                    eprintln!("Error: --config requires a path argument");
                    std::process::exit(1);
                }
            }
            arg => {
                eprintln!("Unknown argument: {}", arg);
                eprintln!("Use --help for usage information");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    info!("Mochi Terminal starting...");

    let config = if let Some(path) = config_path {
        match config::Config::load_from_file(&path) {
            Ok(c) => {
                info!("Loaded config from {:?}", path);
                c
            }
            Err(e) => {
                error!("Failed to load config: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        config::Config::load()
    };

    let cols = config.terminal.columns as usize;
    let rows = config.terminal.rows as usize;

    if let Err(e) = renderer::run_terminal(cols, rows) {
        error!("Fatal error: {}", e);
        std::process::exit(1);
    }

    info!("Mochi Terminal exiting");
}
