//! Mochi Terminal Emulator
//!
//! A terminal emulator built from scratch.

use std::process::ExitCode;

use log::{error, info};

mod core;
mod gui;
mod parser;
mod pty;

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting Mochi Terminal");

    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("Fatal error: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = gui::Config::default();
    gui::run(config)
}
