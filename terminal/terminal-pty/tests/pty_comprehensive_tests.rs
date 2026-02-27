//! Comprehensive tests for terminal-pty crate
//!
//! Tests covering WindowSize, Pty, Child, and Error types.

use nix::sys::signal::Signal;
use std::io;
use std::os::fd::AsRawFd;
use terminal_pty::{Child, Error, Pty, WindowSize};

// ============================================================================
// WindowSize Tests
// ============================================================================

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
    assert_eq!(size.pixel_width, 0);
    assert_eq!(size.pixel_height, 0);
}

#[test]
fn test_window_size_with_pixels() {
    let size = WindowSize::with_pixels(120, 40, 1920, 1080);
    assert_eq!(size.cols, 120);
    assert_eq!(size.rows, 40);
    assert_eq!(size.pixel_width, 1920);
    assert_eq!(size.pixel_height, 1080);
}

#[test]
fn test_window_size_with_pixels_zero() {
    let size = WindowSize::with_pixels(80, 24, 0, 0);
    assert_eq!(size.pixel_width, 0);
    assert_eq!(size.pixel_height, 0);
}

#[test]
fn test_window_size_to_winsize() {
    let size = WindowSize::new(80, 24);
    let ws = size.to_winsize();
    assert_eq!(ws.ws_col, 80);
    assert_eq!(ws.ws_row, 24);
    assert_eq!(ws.ws_xpixel, 0);
    assert_eq!(ws.ws_ypixel, 0);
}

#[test]
fn test_window_size_to_winsize_with_pixels() {
    let size = WindowSize::with_pixels(120, 40, 800, 600);
    let ws = size.to_winsize();
    assert_eq!(ws.ws_col, 120);
    assert_eq!(ws.ws_row, 40);
    assert_eq!(ws.ws_xpixel, 800);
    assert_eq!(ws.ws_ypixel, 600);
}

#[test]
fn test_window_size_from_winsize() {
    let ws = libc::winsize {
        ws_row: 50,
        ws_col: 132,
        ws_xpixel: 1024,
        ws_ypixel: 768,
    };
    let size = WindowSize::from(ws);
    assert_eq!(size.rows, 50);
    assert_eq!(size.cols, 132);
    assert_eq!(size.pixel_width, 1024);
    assert_eq!(size.pixel_height, 768);
}

#[test]
fn test_window_size_clone() {
    let size = WindowSize::new(80, 24);
    let cloned = size.clone();
    assert_eq!(size, cloned);
}

#[test]
fn test_window_size_copy() {
    let size = WindowSize::new(80, 24);
    let copied = size;
    assert_eq!(size.cols, copied.cols);
    assert_eq!(size.rows, copied.rows);
}

#[test]
fn test_window_size_equality() {
    let a = WindowSize::new(80, 24);
    let b = WindowSize::new(80, 24);
    assert_eq!(a, b);
}

#[test]
fn test_window_size_inequality() {
    let a = WindowSize::new(80, 24);
    let b = WindowSize::new(120, 40);
    assert_ne!(a, b);
}

#[test]
fn test_window_size_debug() {
    let size = WindowSize::new(80, 24);
    let debug = format!("{:?}", size);
    assert!(debug.contains("80"));
    assert!(debug.contains("24"));
}

#[test]
fn test_window_size_large_values() {
    let size = WindowSize::new(u16::MAX, u16::MAX);
    assert_eq!(size.cols, u16::MAX);
    assert_eq!(size.rows, u16::MAX);
}

#[test]
fn test_window_size_min_values() {
    let size = WindowSize::new(1, 1);
    assert_eq!(size.cols, 1);
    assert_eq!(size.rows, 1);
}

#[test]
fn test_window_size_zero_dimensions() {
    let size = WindowSize::new(0, 0);
    assert_eq!(size.cols, 0);
    assert_eq!(size.rows, 0);
}

#[test]
fn test_window_size_typical_terminal_sizes() {
    // VT100 standard
    let vt100 = WindowSize::new(80, 24);
    assert_eq!(vt100.cols, 80);
    assert_eq!(vt100.rows, 24);

    // Wide terminal
    let wide = WindowSize::new(132, 43);
    assert_eq!(wide.cols, 132);
    assert_eq!(wide.rows, 43);

    // Modern fullscreen
    let modern = WindowSize::new(200, 60);
    assert_eq!(modern.cols, 200);
    assert_eq!(modern.rows, 60);
}

#[test]
fn test_window_size_pixel_roundtrip() {
    let original = WindowSize::with_pixels(80, 24, 640, 480);
    let ws = original.to_winsize();
    let roundtrip = WindowSize::from(ws);
    assert_eq!(original, roundtrip);
}

// ============================================================================
// Pty Tests
// ============================================================================

#[test]
fn test_pty_creation() {
    let pty = Pty::new();
    assert!(pty.is_ok());
}

#[test]
fn test_pty_slave_path() {
    let pty = Pty::new().unwrap();
    let path = pty.slave_path();
    assert!(!path.is_empty());
    assert!(path.starts_with("/dev/"));
}

#[test]
fn test_pty_master_fd_valid() {
    let pty = Pty::new().unwrap();
    assert!(pty.master_fd() >= 0);
}

#[test]
fn test_pty_as_raw_fd() {
    let pty = Pty::new().unwrap();
    let fd = pty.as_raw_fd();
    assert!(fd >= 0);
}

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
fn test_pty_window_size_roundtrip() {
    let pty = Pty::new().unwrap();
    let sizes = [
        WindowSize::new(80, 24),
        WindowSize::new(120, 40),
        WindowSize::new(132, 43),
        WindowSize::new(200, 60),
    ];
    for size in &sizes {
        pty.set_window_size(*size).unwrap();
        let retrieved = pty.get_window_size().unwrap();
        assert_eq!(retrieved.cols, size.cols);
        assert_eq!(retrieved.rows, size.rows);
    }
}

#[test]
fn test_pty_set_nonblocking() {
    let pty = Pty::new().unwrap();
    assert!(pty.set_nonblocking(true).is_ok());
    assert!(pty.set_nonblocking(false).is_ok());
}

#[test]
fn test_pty_set_nonblocking_toggle() {
    let pty = Pty::new().unwrap();
    // Toggle multiple times
    for _ in 0..5 {
        assert!(pty.set_nonblocking(true).is_ok());
        assert!(pty.set_nonblocking(false).is_ok());
    }
}

#[test]
fn test_pty_nonblocking_read_no_data() {
    let mut pty = Pty::new().unwrap();
    pty.set_nonblocking(true).unwrap();
    let mut buf = [0u8; 1024];
    // Should return WouldBlock or 0 bytes since no slave is writing
    match pty.read(&mut buf) {
        Ok(0) => {}                                           // Fine
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {} // Expected
        other => panic!("Unexpected result: {:?}", other),
    }
}

#[test]
fn test_pty_try_read_no_data() {
    let mut pty = Pty::new().unwrap();
    pty.set_nonblocking(true).unwrap();
    let mut buf = [0u8; 1024];
    let result = pty.try_read(&mut buf);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_pty_multiple_creations() {
    // Create multiple PTYs concurrently
    let ptys: Vec<_> = (0..5).map(|_| Pty::new().unwrap()).collect();
    for pty in &ptys {
        assert!(pty.master_fd() >= 0);
        assert!(!pty.slave_path().is_empty());
    }
    // All should have unique slave paths
    let paths: Vec<_> = ptys.iter().map(|p| p.slave_path().to_string()).collect();
    for i in 0..paths.len() {
        for j in (i + 1)..paths.len() {
            assert_ne!(paths[i], paths[j], "PTY paths should be unique");
        }
    }
}

#[test]
fn test_pty_window_size_with_pixels() {
    let pty = Pty::new().unwrap();
    let size = WindowSize::with_pixels(80, 24, 800, 600);
    pty.set_window_size(size).unwrap();
    let retrieved = pty.get_window_size().unwrap();
    assert_eq!(retrieved.cols, 80);
    assert_eq!(retrieved.rows, 24);
    // Pixel dimensions may or may not be preserved depending on OS
}

// ============================================================================
// Child Process Tests
// ============================================================================

#[test]
fn test_child_spawn_echo() {
    let child = Child::spawn(
        "/bin/echo",
        ["hello"],
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
    let _ = child.signal(Signal::SIGTERM);
}

#[test]
fn test_child_pid() {
    let child = Child::spawn(
        "/bin/echo",
        ["test"],
        None::<Vec<(String, String)>>,
        WindowSize::default(),
    )
    .unwrap();
    let pid = child.pid();
    assert!(pid.as_raw() > 0);
}

#[test]
fn test_child_pty_access() {
    let child = Child::spawn_shell(WindowSize::default()).unwrap();
    let pty = child.pty();
    assert!(pty.master_fd() >= 0);
    let _ = child.signal(Signal::SIGTERM);
}

#[test]
fn test_child_as_raw_fd() {
    let child = Child::spawn_shell(WindowSize::default()).unwrap();
    let fd = child.as_raw_fd();
    assert!(fd >= 0);
    let _ = child.signal(Signal::SIGTERM);
}

#[test]
fn test_child_set_nonblocking() {
    let child = Child::spawn_shell(WindowSize::default()).unwrap();
    assert!(child.set_nonblocking(true).is_ok());
    assert!(child.set_nonblocking(false).is_ok());
    let _ = child.signal(Signal::SIGTERM);
}

#[test]
fn test_child_resize() {
    let child = Child::spawn_shell(WindowSize::default()).unwrap();
    let new_size = WindowSize::new(120, 40);
    assert!(child.resize(new_size).is_ok());
    let retrieved = child.pty().get_window_size().unwrap();
    assert_eq!(retrieved.cols, 120);
    assert_eq!(retrieved.rows, 40);
    let _ = child.signal(Signal::SIGTERM);
}

#[test]
fn test_child_signal() {
    let child = Child::spawn_shell(WindowSize::default()).unwrap();
    assert!(child.signal(Signal::SIGWINCH).is_ok());
    let _ = child.signal(Signal::SIGTERM);
}

#[test]
fn test_child_try_wait_running() {
    let child = Child::spawn_shell(WindowSize::default()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    let result = child.try_wait();
    assert!(result.is_ok());
    // Shell should still be alive
    assert!(result.unwrap().is_none());
    let _ = child.signal(Signal::SIGTERM);
}

#[test]
fn test_child_echo_output() {
    let mut child = Child::spawn(
        "/bin/echo",
        ["test_output_marker"],
        None::<Vec<(String, String)>>,
        WindowSize::default(),
    )
    .unwrap();
    child.set_nonblocking(true).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));

    let mut buf = [0u8; 4096];
    let mut output = String::new();
    loop {
        match child.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => output.push_str(&String::from_utf8_lossy(&buf[..n])),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }
    assert!(output.contains("test_output_marker"));
}

#[test]
fn test_child_spawn_with_env() {
    let env = vec![
        ("PATH".to_string(), "/usr/bin:/bin".to_string()),
        ("TERM".to_string(), "xterm-256color".to_string()),
        ("HOME".to_string(), "/tmp".to_string()),
        ("TEST_VAR".to_string(), "test_value".to_string()),
    ];
    let child = Child::spawn("/bin/echo", ["hello"], Some(env), WindowSize::default());
    assert!(child.is_ok());
}

#[test]
fn test_child_spawn_invalid_program() {
    let result = Child::spawn(
        "/nonexistent/program",
        Vec::<&str>::new(),
        None::<Vec<(String, String)>>,
        WindowSize::default(),
    );
    // This may succeed (fork succeeds) but child exits with 127
    // The spawn itself succeeds because fork happens first
    if let Ok(child) = result {
        std::thread::sleep(std::time::Duration::from_millis(100));
        // Child should have exited
        assert!(!child.is_running());
    }
}

#[test]
fn test_child_write_read_interaction() {
    let mut child = Child::spawn_shell(WindowSize::default()).unwrap();
    child.set_nonblocking(true).unwrap();

    // Wait for shell to start
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Drain initial output
    let mut buf = [0u8; 4096];
    loop {
        match child.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => continue,
        }
    }

    // Write command
    assert!(child.write_all(b"echo UNIQUE_MARKER_42\n").is_ok());
    std::thread::sleep(std::time::Duration::from_millis(300));

    let mut output = String::new();
    loop {
        match child.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => output.push_str(&String::from_utf8_lossy(&buf[..n])),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }
    assert!(output.contains("UNIQUE_MARKER_42"));

    let _ = child.signal(Signal::SIGTERM);
}

#[test]
fn test_child_multiple_resize() {
    let child = Child::spawn_shell(WindowSize::default()).unwrap();
    let sizes = [
        WindowSize::new(80, 24),
        WindowSize::new(120, 40),
        WindowSize::new(132, 43),
        WindowSize::new(200, 60),
        WindowSize::new(80, 24),
    ];
    for size in &sizes {
        assert!(child.resize(*size).is_ok());
        let retrieved = child.pty().get_window_size().unwrap();
        assert_eq!(retrieved.cols, size.cols);
        assert_eq!(retrieved.rows, size.rows);
    }
    let _ = child.signal(Signal::SIGTERM);
}

// ============================================================================
// Error Tests
// ============================================================================

#[test]
fn test_error_io() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "not found");
    let err: Error = Error::Io(io_err);
    let msg = format!("{}", err);
    assert!(msg.contains("not found"));
}

#[test]
fn test_error_pty_creation() {
    let err = Error::PtyCreation("test error".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("test error"));
    assert!(msg.contains("PTY"));
}

#[test]
fn test_error_spawn_failed() {
    let err = Error::SpawnFailed("spawn error".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("spawn error"));
}

#[test]
fn test_error_window_size() {
    let err = Error::WindowSize("size error".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("size error"));
}

#[test]
fn test_error_child_error() {
    let err = Error::ChildError("child error".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("child error"));
}

#[test]
fn test_error_debug() {
    let err = Error::PtyCreation("debug test".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("debug test"));
}

#[test]
fn test_error_from_io() {
    let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
    let err: Error = io_err.into();
    let msg = format!("{}", err);
    assert!(msg.contains("permission denied"));
}
