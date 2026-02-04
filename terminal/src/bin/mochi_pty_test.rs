//! Mochi PTY Test - Interactive PTY relay
//!
//! A simple CLI program that spawns a shell via PTY and relays I/O.
//! Used for testing PTY functionality without the GUI.

use std::io::{self, Read, Write};
use std::process::ExitCode;
use std::thread;

use mochi_terminal::pty::{Pty, WindowSize};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn main() -> ExitCode {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer().with_writer(io::stderr))
        .init();

    let args: Vec<String> = std::env::args().collect();

    // Parse command line arguments
    let mut cols = 80u16;
    let mut rows = 24u16;
    let mut shell: Option<String> = None;
    let mut show_help = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-c" | "--cols" => {
                i += 1;
                if i < args.len() {
                    cols = args[i].parse().unwrap_or(80);
                }
            },
            "-r" | "--rows" => {
                i += 1;
                if i < args.len() {
                    rows = args[i].parse().unwrap_or(24);
                }
            },
            "-s" | "--shell" => {
                i += 1;
                if i < args.len() {
                    shell = Some(args[i].clone());
                }
            },
            "-h" | "--help" => {
                show_help = true;
            },
            _ => {},
        }
        i += 1;
    }

    if show_help {
        print_help();
        return ExitCode::SUCCESS;
    }

    tracing::info!("Starting PTY test with {}x{} terminal", cols, rows);

    // Spawn PTY
    let size = WindowSize::new(cols, rows);
    let mut pty = match &shell {
        Some(s) => {
            tracing::info!("Spawning shell: {}", s);
            match Pty::spawn(s, &[], size) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to spawn PTY: {}", e);
                    return ExitCode::FAILURE;
                },
            }
        },
        None => {
            tracing::info!("Spawning default shell");
            match Pty::spawn_shell(size) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Failed to spawn PTY: {}", e);
                    return ExitCode::FAILURE;
                },
            }
        },
    };

    tracing::info!("PTY spawned, child PID: {}", pty.child_pid());

    // Set stdin to raw mode for proper terminal interaction
    // Note: This is a simplified version; a full implementation would use termios

    // Spawn a thread to read from stdin and write to PTY
    let pty_fd = pty.master_fd();
    let stdin_thread = thread::spawn(move || {
        let mut stdin = io::stdin();
        let mut buf = [0u8; 1024];

        loop {
            match stdin.read(&mut buf) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    // Write to PTY
                    // SAFETY: We're using the raw fd directly here
                    let result = nix::unistd::write(pty_fd, &buf[..n]);
                    if result.is_err() {
                        break;
                    }
                },
                Err(_) => break,
            }
        }
    });

    // Main loop: read from PTY and write to stdout
    let mut stdout = io::stdout();
    let mut buf = [0u8; 4096];

    loop {
        // Check if child is still alive
        if !pty.is_alive() {
            tracing::info!("Child process exited");
            break;
        }

        // Poll for data
        match pty.poll_read(100) {
            Ok(true) => {
                // Data available
                match pty.read(&mut buf) {
                    Ok(0) => {
                        // EOF
                        break;
                    },
                    Ok(n) => {
                        // Write to stdout
                        if stdout.write_all(&buf[..n]).is_err() {
                            break;
                        }
                        let _ = stdout.flush();
                    },
                    Err(e) => {
                        tracing::error!("Read error: {}", e);
                        break;
                    },
                }
            },
            Ok(false) => {
                // Timeout, continue
            },
            Err(e) => {
                tracing::error!("Poll error: {}", e);
                break;
            },
        }
    }

    // Wait for stdin thread
    let _ = stdin_thread.join();

    // Wait for child
    match pty.wait() {
        Ok(code) => {
            tracing::info!("Child exited with code: {}", code);
            if code == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(code as u8)
            }
        },
        Err(e) => {
            tracing::error!("Wait error: {}", e);
            ExitCode::FAILURE
        },
    }
}

fn print_help() {
    println!("Mochi PTY Test - Interactive PTY relay");
    println!();
    println!("Usage: mochi-pty-test [OPTIONS]");
    println!();
    println!("Options:");
    println!("  -c, --cols <N>     Set terminal width (default: 80)");
    println!("  -r, --rows <N>     Set terminal height (default: 24)");
    println!("  -s, --shell <PATH> Shell to spawn (default: $SHELL or /bin/bash)");
    println!("  -h, --help         Show this help message");
    println!();
    println!("This program spawns a shell via PTY and relays I/O between");
    println!("stdin/stdout and the PTY. It's useful for testing PTY functionality.");
    println!();
    println!("Note: For proper terminal interaction, you may need to run this");
    println!("in a terminal that supports raw mode.");
}
