//! Mochi Terminal Emulator
//!
//! A terminal emulator built from scratch, implementing VT/xterm-compatible
//! escape sequence handling with a modern GUI.

use env_logger::Env;
use log::{error, info};
use mochi_core::Screen;
use mochi_parser::Parser;
use mochi_pty::{Pty, PtySize};
use std::io::{self, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

mod input;
mod performer;

use performer::Performer;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    info!("Mochi Terminal starting...");

    if let Err(e) = run() {
        error!("Fatal error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cols = 80;
    let rows = 24;

    let mut pty = Pty::new()?;
    pty.spawn(None, PtySize::new(cols, rows))?;
    pty.set_nonblocking(true)?;

    info!("Shell spawned with PTY");

    let mut screen = Screen::new(cols as usize, rows as usize);
    let mut parser = Parser::new();
    let mut performer = Performer::new();

    let (tx, rx) = mpsc::channel::<Vec<u8>>();

    let stdin_thread = thread::spawn(move || {
        let stdin = io::stdin();
        let mut buf = [0u8; 1024];
        loop {
            match stdin.lock().read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut read_buf = [0u8; 4096];

    loop {
        if let Some(result) = pty.check_child() {
            match result {
                Ok(()) => {
                    info!("Child process exited normally");
                    break;
                }
                Err(e) => {
                    info!("Child process exited: {}", e);
                    break;
                }
            }
        }

        while let Ok(input) = rx.try_recv() {
            pty.write(&input)?;
        }

        match pty.read(&mut read_buf) {
            Ok(n) if n > 0 => {
                parser.parse(&read_buf[..n], |action| {
                    performer.perform(&mut screen, action);
                });

                print_screen(&screen);
            }
            Ok(_) => {}
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => {
                error!("PTY read error: {}", e);
                break;
            }
        }

        thread::sleep(Duration::from_millis(10));
    }

    drop(stdin_thread);

    info!("Mochi Terminal exiting");
    Ok(())
}

fn print_screen(screen: &Screen) {
    print!("\x1b[2J\x1b[H");

    for row in 0..screen.rows() {
        if let Some(line) = screen.get_line(row) {
            print!("{}\r\n", line.text_content());
        }
    }

    print!(
        "\x1b[{};{}H",
        screen.cursor().row + 1,
        screen.cursor().col + 1
    );

    io::stdout().flush().ok();
}

use std::io::Read;
