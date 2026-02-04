//! PTY Relay
//!
//! A simple CLI tool that relays stdin/stdout to a PTY.
//! Used for testing PTY functionality without the GUI.

use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use std::time::Duration;

use mochi_term::pty::Pty;
use polling::{Event, Events, Poller};

fn main() -> io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Get terminal size
    let (cols, rows) = get_terminal_size().unwrap_or((80, 24));
    println!("Starting PTY relay ({}x{})", cols, rows);

    // Spawn PTY with shell
    let mut pty = Pty::spawn(None, cols, rows)?;
    println!("PTY spawned, child PID: {:?}", pty.child_pid());

    // Set stdin to raw mode
    let _raw_guard = RawModeGuard::new()?;

    // Create poller
    let poller = Poller::new()?;

    // Register stdin and PTY master for reading
    unsafe {
        poller.add(io::stdin().as_raw_fd(), Event::readable(0))?;
        poller.add(pty.as_raw_fd(), Event::readable(1))?;
    }

    let mut events = Events::new();
    let mut stdin_buf = [0u8; 4096];
    let mut pty_buf = [0u8; 65536];

    loop {
        events.clear();
        poller.wait(&mut events, Some(Duration::from_millis(100)))?;

        // Check if child is still running
        if !pty.is_running() {
            if let Ok(Some(code)) = pty.try_wait() {
                eprintln!("\r\nChild exited with code {}", code);
                break;
            }
        }

        for event in events.iter() {
            match event.key {
                0 => {
                    // stdin ready
                    let n = io::stdin().read(&mut stdin_buf)?;
                    if n == 0 {
                        // EOF on stdin
                        return Ok(());
                    }
                    pty.write_all(&stdin_buf[..n])?;
                }
                1 => {
                    // PTY ready
                    match pty.read(&mut pty_buf) {
                        Ok(0) => {}
                        Ok(n) => {
                            io::stdout().write_all(&pty_buf[..n])?;
                            io::stdout().flush()?;
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                        Err(e) => return Err(e),
                    }
                }
                _ => {}
            }
        }

        // Also try reading from PTY even without events (non-blocking)
        loop {
            match pty.read(&mut pty_buf) {
                Ok(0) => break,
                Ok(n) => {
                    io::stdout().write_all(&pty_buf[..n])?;
                    io::stdout().flush()?;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
    }

    Ok(())
}

/// Get terminal size using ioctl
fn get_terminal_size() -> Option<(u16, u16)> {
    use nix::libc;
    use nix::pty::Winsize;

    let mut ws = Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    let result = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) };

    if result == 0 && ws.ws_col > 0 && ws.ws_row > 0 {
        Some((ws.ws_col, ws.ws_row))
    } else {
        None
    }
}

/// RAII guard for raw terminal mode
struct RawModeGuard {
    original: nix::sys::termios::Termios,
}

impl RawModeGuard {
    fn new() -> io::Result<Self> {
        use nix::sys::termios::{self, LocalFlags, SetArg};

        let original =
            termios::tcgetattr(io::stdin()).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let mut raw = original.clone();

        // Disable canonical mode and echo
        raw.local_flags.remove(LocalFlags::ICANON);
        raw.local_flags.remove(LocalFlags::ECHO);
        raw.local_flags.remove(LocalFlags::ISIG);
        raw.local_flags.remove(LocalFlags::IEXTEN);

        // Set minimum characters and timeout
        raw.control_chars[nix::sys::termios::SpecialCharacterIndices::VMIN as usize] = 1;
        raw.control_chars[nix::sys::termios::SpecialCharacterIndices::VTIME as usize] = 0;

        termios::tcsetattr(io::stdin(), SetArg::TCSANOW, &raw)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(Self { original })
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        use nix::sys::termios::{self, SetArg};
        let _ = termios::tcsetattr(io::stdin(), SetArg::TCSANOW, &self.original);
    }
}
