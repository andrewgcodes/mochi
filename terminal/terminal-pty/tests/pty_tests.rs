//! Comprehensive tests for terminal-pty module

use std::io;
use std::thread;
use std::time::Duration;
use terminal_pty::{Child, Pty, WindowSize};

// ============================================================
// WindowSize Tests
// ============================================================

#[test]
fn test_window_size_new() {
    let size = WindowSize::new(80, 24);
    assert_eq!(size.cols, 80);
    assert_eq!(size.rows, 24);
    assert_eq!(size.pixel_width, 0);
    assert_eq!(size.pixel_height, 0);
}

#[test]
fn test_window_size_default() {
    let size = WindowSize::default();
    assert_eq!(size.cols, 80);
    assert_eq!(size.rows, 24);
}

#[test]
fn test_window_size_with_pixels() {
    let size = WindowSize::with_pixels(80, 24, 800, 600);
    assert_eq!(size.cols, 80);
    assert_eq!(size.rows, 24);
    assert_eq!(size.pixel_width, 800);
    assert_eq!(size.pixel_height, 600);
}

#[test]
fn test_window_size_to_winsize() {
    let size = WindowSize::new(120, 40);
    let ws = size.to_winsize();
    assert_eq!(ws.ws_col, 120);
    assert_eq!(ws.ws_row, 40);
    assert_eq!(ws.ws_xpixel, 0);
    assert_eq!(ws.ws_ypixel, 0);
}

#[test]
fn test_window_size_to_winsize_with_pixels() {
    let size = WindowSize::with_pixels(80, 24, 960, 480);
    let ws = size.to_winsize();
    assert_eq!(ws.ws_col, 80);
    assert_eq!(ws.ws_row, 24);
    assert_eq!(ws.ws_xpixel, 960);
    assert_eq!(ws.ws_ypixel, 480);
}

#[test]
fn test_window_size_from_winsize() {
    let ws = libc::winsize {
        ws_row: 40,
        ws_col: 120,
        ws_xpixel: 1200,
        ws_ypixel: 800,
    };
    let size = WindowSize::from(ws);
    assert_eq!(size.rows, 40);
    assert_eq!(size.cols, 120);
    assert_eq!(size.pixel_width, 1200);
    assert_eq!(size.pixel_height, 800);
}

#[test]
fn test_window_size_equality() {
    let s1 = WindowSize::new(80, 24);
    let s2 = WindowSize::new(80, 24);
    assert_eq!(s1, s2);
}

#[test]
fn test_window_size_inequality() {
    let s1 = WindowSize::new(80, 24);
    let s2 = WindowSize::new(120, 40);
    assert_ne!(s1, s2);
}

#[test]
fn test_window_size_clone() {
    let s1 = WindowSize::new(80, 24);
    let s2 = s1;
    assert_eq!(s1, s2);
}

#[test]
fn test_window_size_small() {
    let size = WindowSize::new(1, 1);
    assert_eq!(size.cols, 1);
    assert_eq!(size.rows, 1);
}

#[test]
fn test_window_size_large() {
    let size = WindowSize::new(500, 200);
    assert_eq!(size.cols, 500);
    assert_eq!(size.rows, 200);
}

#[test]
fn test_window_size_debug() {
    let size = WindowSize::new(80, 24);
    let debug = format!("{:?}", size);
    assert!(debug.contains("80"));
    assert!(debug.contains("24"));
}

// ============================================================
// PTY Creation Tests
// ============================================================

#[test]
fn test_pty_creation() {
    let pty = Pty::new();
    assert!(pty.is_ok());
}

#[test]
fn test_pty_slave_path_not_empty() {
    let pty = Pty::new().unwrap();
    assert!(!pty.slave_path().is_empty());
}

#[test]
fn test_pty_slave_path_starts_with_dev() {
    let pty = Pty::new().unwrap();
    assert!(pty.slave_path().starts_with("/dev/"));
}

#[test]
fn test_pty_master_fd_valid() {
    let pty = Pty::new().unwrap();
    assert!(pty.master_fd() >= 0);
}

// ============================================================
// PTY Window Size Tests
// ============================================================

#[test]
fn test_pty_set_window_size() {
    let pty = Pty::new().unwrap();
    let size = WindowSize::new(120, 40);
    assert!(pty.set_window_size(size).is_ok());
}

#[test]
fn test_pty_get_window_size() {
    let pty = Pty::new().unwrap();
    let size = WindowSize::new(120, 40);
    pty.set_window_size(size).unwrap();
    let retrieved = pty.get_window_size().unwrap();
    assert_eq!(retrieved.cols, 120);
    assert_eq!(retrieved.rows, 40);
}

#[test]
fn test_pty_set_window_size_multiple() {
    let pty = Pty::new().unwrap();
    pty.set_window_size(WindowSize::new(80, 24)).unwrap();
    pty.set_window_size(WindowSize::new(120, 40)).unwrap();
    let retrieved = pty.get_window_size().unwrap();
    assert_eq!(retrieved.cols, 120);
    assert_eq!(retrieved.rows, 40);
}

#[test]
fn test_pty_window_size_small() {
    let pty = Pty::new().unwrap();
    let size = WindowSize::new(1, 1);
    assert!(pty.set_window_size(size).is_ok());
    let retrieved = pty.get_window_size().unwrap();
    assert_eq!(retrieved.cols, 1);
    assert_eq!(retrieved.rows, 1);
}

// ============================================================
// PTY Non-blocking Tests
// ============================================================

#[test]
fn test_pty_set_nonblocking_on() {
    let pty = Pty::new().unwrap();
    assert!(pty.set_nonblocking(true).is_ok());
}

#[test]
fn test_pty_set_nonblocking_off() {
    let pty = Pty::new().unwrap();
    assert!(pty.set_nonblocking(false).is_ok());
}

#[test]
fn test_pty_set_nonblocking_toggle() {
    let pty = Pty::new().unwrap();
    assert!(pty.set_nonblocking(true).is_ok());
    assert!(pty.set_nonblocking(false).is_ok());
    assert!(pty.set_nonblocking(true).is_ok());
}

// ============================================================
// Child Process Tests
// ============================================================

#[test]
fn test_child_spawn_echo() {
    let child = Child::spawn(
        "/bin/echo",
        ["test_output"],
        None::<Vec<(String, String)>>,
        WindowSize::default(),
    );
    assert!(child.is_ok());
}

#[test]
fn test_child_spawn_true() {
    let child = Child::spawn(
        "/bin/true",
        std::iter::empty::<&str>(),
        None::<Vec<(String, String)>>,
        WindowSize::default(),
    );
    assert!(child.is_ok());
}

#[test]
fn test_child_spawn_false() {
    let child = Child::spawn(
        "/bin/false",
        std::iter::empty::<&str>(),
        None::<Vec<(String, String)>>,
        WindowSize::default(),
    );
    assert!(child.is_ok());
}

#[test]
fn test_child_spawn_shell() {
    let child = Child::spawn_shell(WindowSize::default());
    assert!(child.is_ok());
    let child = child.unwrap();
    let _ = child.signal(nix::sys::signal::Signal::SIGTERM);
}

#[test]
fn test_child_pid_valid() {
    let child = Child::spawn(
        "/bin/sleep",
        ["1"],
        None::<Vec<(String, String)>>,
        WindowSize::default(),
    )
    .unwrap();
    assert!(child.pid().as_raw() > 0);
    let _ = child.signal(nix::sys::signal::Signal::SIGTERM);
}

#[test]
fn test_child_pty_access() {
    let child = Child::spawn_shell(WindowSize::default()).unwrap();
    assert!(!child.pty().slave_path().is_empty());
    let _ = child.signal(nix::sys::signal::Signal::SIGTERM);
}

#[test]
fn test_child_resize() {
    let child = Child::spawn_shell(WindowSize::default()).unwrap();
    let new_size = WindowSize::new(120, 40);
    assert!(child.resize(new_size).is_ok());
    let retrieved = child.pty().get_window_size().unwrap();
    assert_eq!(retrieved.cols, 120);
    assert_eq!(retrieved.rows, 40);
    let _ = child.signal(nix::sys::signal::Signal::SIGTERM);
}

#[test]
fn test_child_set_nonblocking() {
    let child = Child::spawn_shell(WindowSize::default()).unwrap();
    assert!(child.set_nonblocking(true).is_ok());
    assert!(child.set_nonblocking(false).is_ok());
    let _ = child.signal(nix::sys::signal::Signal::SIGTERM);
}

#[test]
fn test_child_as_raw_fd() {
    let child = Child::spawn_shell(WindowSize::default()).unwrap();
    assert!(child.as_raw_fd() >= 0);
    let _ = child.signal(nix::sys::signal::Signal::SIGTERM);
}

#[test]
fn test_child_is_running() {
    let child = Child::spawn(
        "/bin/sleep",
        ["5"],
        None::<Vec<(String, String)>>,
        WindowSize::default(),
    )
    .unwrap();
    thread::sleep(Duration::from_millis(50));
    assert!(child.is_running());
    let _ = child.signal(nix::sys::signal::Signal::SIGTERM);
}

#[test]
fn test_child_try_wait_running() {
    let child = Child::spawn(
        "/bin/sleep",
        ["5"],
        None::<Vec<(String, String)>>,
        WindowSize::default(),
    )
    .unwrap();
    thread::sleep(Duration::from_millis(50));
    let result = child.try_wait().unwrap();
    assert!(result.is_none());
    let _ = child.signal(nix::sys::signal::Signal::SIGTERM);
}

#[test]
fn test_child_read_output() {
    let mut child = Child::spawn(
        "/bin/echo",
        ["hello_pty_test"],
        None::<Vec<(String, String)>>,
        WindowSize::default(),
    )
    .unwrap();

    thread::sleep(Duration::from_millis(200));
    child.set_nonblocking(true).unwrap();

    let mut buf = [0u8; 1024];
    let mut output = String::new();
    loop {
        match child.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => output.push_str(&String::from_utf8_lossy(&buf[..n])),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }

    assert!(output.contains("hello_pty_test"));
}

#[test]
fn test_child_write_input() {
    let mut child = Child::spawn_shell(WindowSize::default()).unwrap();
    child.set_nonblocking(true).unwrap();
    thread::sleep(Duration::from_millis(300));

    // Drain initial output
    let mut buf = [0u8; 4096];
    loop {
        match child.read(&mut buf) {
            Ok(0) => break,
            Ok(_) => continue,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }

    // Write a command
    let result = child.write_all(b"echo PTY_WRITE_TEST_12345\n");
    assert!(result.is_ok());

    // Retry reading with increasing waits to handle slow shells
    let mut output = String::new();
    for _ in 0..5 {
        thread::sleep(Duration::from_millis(200));
        loop {
            match child.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output.push_str(&String::from_utf8_lossy(&buf[..n])),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
        if output.contains("PTY_WRITE_TEST_12345") {
            break;
        }
    }

    assert!(
        output.contains("PTY_WRITE_TEST_12345"),
        "Expected output to contain marker, got: {}",
        output
    );

    let _ = child.signal(nix::sys::signal::Signal::SIGTERM);
}

#[test]
fn test_child_spawn_with_env() {
    let env = vec![
        ("TERM".to_string(), "xterm-256color".to_string()),
        ("HOME".to_string(), "/tmp".to_string()),
        ("PATH".to_string(), "/usr/bin:/bin".to_string()),
        ("SHELL".to_string(), "/bin/sh".to_string()),
    ];
    let child = Child::spawn("/bin/echo", ["env_test"], Some(env), WindowSize::default());
    assert!(child.is_ok());
}

#[test]
fn test_child_signal_sigterm() {
    let child = Child::spawn(
        "/bin/sleep",
        ["10"],
        None::<Vec<(String, String)>>,
        WindowSize::default(),
    )
    .unwrap();
    thread::sleep(Duration::from_millis(50));
    assert!(child.signal(nix::sys::signal::Signal::SIGTERM).is_ok());
}

// ============================================================
// Error Type Tests
// ============================================================

#[test]
fn test_error_display_io() {
    let err = terminal_pty::Error::Io(io::Error::new(io::ErrorKind::Other, "test"));
    let msg = format!("{}", err);
    assert!(msg.contains("test"));
}

#[test]
fn test_error_display_pty_creation() {
    let err = terminal_pty::Error::PtyCreation("failed".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("failed"));
}

#[test]
fn test_error_display_spawn_failed() {
    let err = terminal_pty::Error::SpawnFailed("no such file".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("no such file"));
}

#[test]
fn test_error_display_window_size() {
    let err = terminal_pty::Error::WindowSize("invalid".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("invalid"));
}

#[test]
fn test_error_display_child_error() {
    let err = terminal_pty::Error::ChildError("exited".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("exited"));
}

#[test]
fn test_error_from_io() {
    let io_err = io::Error::new(io::ErrorKind::Other, "io_test");
    let err: terminal_pty::Error = io_err.into();
    assert!(matches!(err, terminal_pty::Error::Io(_)));
}
