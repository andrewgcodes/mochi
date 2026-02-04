//! Mochi Terminal - Main GUI Application
//!
//! A real Linux terminal emulator with GPU-accelerated rendering.

use std::process::ExitCode;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn main() -> ExitCode {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Mochi Terminal starting...");

    // TODO: Implement GUI terminal
    // For now, just print a message
    println!("Mochi Terminal v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("GUI terminal emulator is not yet implemented.");
    println!("Use mochi-headless for testing the terminal core.");
    println!("Use mochi-pty-test for testing PTY functionality.");

    ExitCode::SUCCESS
}
