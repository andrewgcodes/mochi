//! Mochi Headless Terminal Runner
//!
//! A headless terminal emulator for testing and automation.
//! Reads input from stdin or a file and outputs terminal state snapshots.

use std::io::{self, Read};
use std::process::ExitCode;

use mochi_terminal::core::{CompactSnapshot, Screen, Snapshot};
use mochi_terminal::parser::Parser;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn main() -> ExitCode {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")))
        .with(tracing_subscriber::fmt::layer().with_writer(io::stderr))
        .init();

    let args: Vec<String> = std::env::args().collect();

    // Parse command line arguments
    let mut cols = 80u16;
    let mut rows = 24u16;
    let mut input_file: Option<String> = None;
    let mut output_format = OutputFormat::Text;
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
            "-f" | "--file" => {
                i += 1;
                if i < args.len() {
                    input_file = Some(args[i].clone());
                }
            },
            "-j" | "--json" => {
                output_format = OutputFormat::Json;
            },
            "-t" | "--text" => {
                output_format = OutputFormat::Text;
            },
            "-h" | "--help" => {
                show_help = true;
            },
            _ => {
                // Treat as input file if no flag
                if input_file.is_none() && !args[i].starts_with('-') {
                    input_file = Some(args[i].clone());
                }
            },
        }
        i += 1;
    }

    if show_help {
        print_help();
        return ExitCode::SUCCESS;
    }

    // Create terminal
    let mut screen = Screen::new(cols as usize, rows as usize);
    let mut parser = Parser::new();

    // Read input
    let input_data = match &input_file {
        Some(path) => match std::fs::read(path) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Error reading file '{}': {}", path, e);
                return ExitCode::FAILURE;
            },
        },
        None => {
            // Read from stdin
            let mut data = Vec::new();
            if let Err(e) = io::stdin().read_to_end(&mut data) {
                eprintln!("Error reading stdin: {}", e);
                return ExitCode::FAILURE;
            }
            data
        },
    };

    // Process input
    let actions = parser.feed(&input_data);
    for action in actions {
        screen.apply(action);
    }

    // Output result
    match output_format {
        OutputFormat::Text => {
            let snapshot = CompactSnapshot::from_screen(&screen);
            println!("Terminal State ({}x{}):", cols, rows);
            println!("Cursor: ({}, {})", snapshot.cursor_row, snapshot.cursor_col);
            println!("---");
            for line in &snapshot.text {
                println!("{}", line);
            }
            println!("---");
        },
        OutputFormat::Json => {
            let snapshot = Snapshot::from_screen(&screen);
            match serde_json::to_string_pretty(&snapshot) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("Error serializing snapshot: {}", e);
                    return ExitCode::FAILURE;
                },
            }
        },
    }

    ExitCode::SUCCESS
}

#[derive(Clone, Copy)]
enum OutputFormat {
    Text,
    Json,
}

fn print_help() {
    println!("Mochi Headless Terminal Runner");
    println!();
    println!("Usage: mochi-headless [OPTIONS] [INPUT_FILE]");
    println!();
    println!("Options:");
    println!("  -c, --cols <N>     Set terminal width (default: 80)");
    println!("  -r, --rows <N>     Set terminal height (default: 24)");
    println!("  -f, --file <PATH>  Read input from file");
    println!("  -j, --json         Output snapshot as JSON");
    println!("  -t, --text         Output snapshot as text (default)");
    println!("  -h, --help         Show this help message");
    println!();
    println!("If no input file is specified, reads from stdin.");
    println!();
    println!("Examples:");
    println!("  echo -e 'Hello\\x1b[31mWorld\\x1b[0m' | mochi-headless");
    println!("  mochi-headless -c 120 -r 40 input.txt");
    println!("  mochi-headless --json < test.bin > snapshot.json");
}
