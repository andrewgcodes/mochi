//! Headless Terminal Runner
//!
//! A command-line tool for running the terminal emulator without a GUI.
//! Useful for testing and generating deterministic snapshots.
//!
//! # Usage
//!
//! ```bash
//! # Process input from stdin and output snapshot
//! echo -e "Hello\x1b[31mRed\x1b[0m" | mochi-headless --output snapshot.json
//!
//! # Process input from file
//! mochi-headless --input test.bin --output snapshot.json
//!
//! # Output as text instead of JSON
//! mochi-headless --input test.bin --text
//! ```

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;

use mochi_term::{Snapshot, Terminal};

/// Command-line arguments
struct Args {
    /// Input file (stdin if not specified)
    input: Option<PathBuf>,
    /// Output file (stdout if not specified)
    output: Option<PathBuf>,
    /// Output as text instead of JSON
    text: bool,
    /// Terminal columns
    cols: usize,
    /// Terminal rows
    rows: usize,
    /// Scrollback capacity
    scrollback: usize,
    /// Show help
    help: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            input: None,
            output: None,
            text: false,
            cols: 80,
            rows: 24,
            scrollback: 1000,
            help: false,
        }
    }
}

fn parse_args() -> Args {
    let mut args = Args::default();
    let mut argv: Vec<String> = std::env::args().skip(1).collect();

    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "-h" | "--help" => {
                args.help = true;
            }
            "-i" | "--input" => {
                i += 1;
                if i < argv.len() {
                    args.input = Some(PathBuf::from(&argv[i]));
                }
            }
            "-o" | "--output" => {
                i += 1;
                if i < argv.len() {
                    args.output = Some(PathBuf::from(&argv[i]));
                }
            }
            "-t" | "--text" => {
                args.text = true;
            }
            "-c" | "--cols" => {
                i += 1;
                if i < argv.len() {
                    args.cols = argv[i].parse().unwrap_or(80);
                }
            }
            "-r" | "--rows" => {
                i += 1;
                if i < argv.len() {
                    args.rows = argv[i].parse().unwrap_or(24);
                }
            }
            "-s" | "--scrollback" => {
                i += 1;
                if i < argv.len() {
                    args.scrollback = argv[i].parse().unwrap_or(1000);
                }
            }
            _ => {}
        }
        i += 1;
    }

    args
}

fn print_help() {
    eprintln!(
        r#"mochi-headless - Headless terminal emulator for testing

USAGE:
    mochi-headless [OPTIONS]

OPTIONS:
    -h, --help              Show this help message
    -i, --input <FILE>      Input file (stdin if not specified)
    -o, --output <FILE>     Output file (stdout if not specified)
    -t, --text              Output as plain text instead of JSON
    -c, --cols <N>          Terminal columns (default: 80)
    -r, --rows <N>          Terminal rows (default: 24)
    -s, --scrollback <N>    Scrollback capacity (default: 1000)

EXAMPLES:
    # Process escape sequences and output JSON snapshot
    echo -e "Hello\x1b[31mWorld\x1b[0m" | mochi-headless

    # Process from file and output text
    mochi-headless -i input.bin -t

    # Custom terminal size
    mochi-headless -c 120 -r 40 -i input.bin -o snapshot.json
"#
    );
}

fn main() -> io::Result<()> {
    let args = parse_args();

    if args.help {
        print_help();
        return Ok(());
    }

    // Read input
    let input_data = if let Some(path) = &args.input {
        std::fs::read(path)?
    } else {
        let mut data = Vec::new();
        io::stdin().read_to_end(&mut data)?;
        data
    };

    // Create terminal and process input
    let mut terminal = Terminal::new(args.cols, args.rows, args.scrollback);
    terminal.process(&input_data);

    // Generate snapshot
    let snapshot = Snapshot::from_screen(terminal.screen());

    // Output result
    let output_data = if args.text {
        snapshot.to_text()
    } else {
        snapshot.to_json().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    };

    if let Some(path) = &args.output {
        let mut file = File::create(path)?;
        file.write_all(output_data.as_bytes())?;
    } else {
        io::stdout().write_all(output_data.as_bytes())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headless_basic() {
        let mut terminal = Terminal::new(80, 24, 1000);
        terminal.process(b"Hello, World!");

        let snapshot = Snapshot::from_screen(terminal.screen());
        let text = snapshot.to_text();

        assert!(text.contains("Hello, World!"));
    }

    #[test]
    fn test_headless_colors() {
        let mut terminal = Terminal::new(80, 24, 1000);
        terminal.process(b"\x1b[31mRed\x1b[0m Normal");

        let snapshot = Snapshot::from_screen(terminal.screen());
        let json = snapshot.to_json().unwrap();
        let text = snapshot.to_text();

        // Verify text output contains the words
        assert!(text.contains("Red"));
        assert!(text.contains("Normal"));

        // Verify JSON is valid and contains the individual characters
        assert!(json.contains("\"content\": \"R\""));
        assert!(json.contains("\"content\": \"N\""));
        // Verify color information is present (Indexed color 1 = red)
        assert!(json.contains("\"index\": 1"));
    }

    #[test]
    fn test_headless_cursor_movement() {
        let mut terminal = Terminal::new(10, 5, 1000);
        terminal.process(b"\x1b[3;5HX");

        let snapshot = Snapshot::from_screen(terminal.screen());

        // Cursor should be at row 2 (0-indexed), col 5 (after printing X)
        assert_eq!(snapshot.cursor.row, 2);
        assert_eq!(snapshot.cursor.col, 5);
    }

    #[test]
    fn test_headless_json_roundtrip() {
        let mut terminal = Terminal::new(80, 24, 1000);
        terminal.process(b"Test\x1b[1;31mBold Red\x1b[0m");

        let snapshot1 = Snapshot::from_screen(terminal.screen());
        let json = snapshot1.to_json().unwrap();
        let snapshot2 = Snapshot::from_json(&json).unwrap();

        assert!(snapshot1.content_equals(&snapshot2));
    }
}
