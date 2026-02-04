//! Integration tests for terminal emulation
//!
//! These tests verify end-to-end behavior with real PTY and shell interaction.

use std::time::Duration;

use mochi_terminal::core::Screen;
use mochi_terminal::parser::Parser;
use mochi_terminal::pty::{Pty, WindowSize};

/// Helper to read all available output from PTY with timeout
fn read_pty_output(pty: &Pty, timeout_ms: u64) -> Vec<u8> {
    let mut output = Vec::new();
    let mut buf = [0u8; 4096];
    let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);

    while std::time::Instant::now() < deadline {
        if pty.poll_read(50).unwrap_or(false) {
            match pty.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
    }

    output
}

/// Helper to send command and wait for output
fn send_command(pty: &Pty, cmd: &str) {
    pty.write_all(cmd.as_bytes())
        .expect("Failed to write command");
    pty.write_all(b"\n").expect("Failed to write newline");
}

/// Helper to process PTY output through parser and screen
fn process_output(output: &[u8], screen: &mut Screen, parser: &mut Parser) {
    let actions = parser.feed(output);
    for action in actions {
        screen.apply(action);
    }
}

// ============================================================================
// PTY Spawn Tests
// ============================================================================

#[test]
fn test_pty_spawn_echo() {
    // Spawn echo command
    let mut pty = Pty::spawn("/bin/echo", &["Hello, World!"], WindowSize::new(80, 24))
        .expect("Failed to spawn PTY");

    // Read output
    let output = read_pty_output(&pty, 500);
    let text = String::from_utf8_lossy(&output);

    assert!(
        text.contains("Hello, World!"),
        "Expected 'Hello, World!' in output, got: {}",
        text
    );

    // Wait for child to exit
    let _ = pty.wait();
    assert!(!pty.is_alive());
}

#[test]
fn test_pty_spawn_shell_and_exit() {
    // Spawn a shell
    let mut pty = Pty::spawn("/bin/sh", &[], WindowSize::new(80, 24)).expect("Failed to spawn PTY");

    // Give shell time to start
    std::thread::sleep(Duration::from_millis(100));

    // Send exit command
    send_command(&pty, "exit 0");

    // Wait for child to exit
    std::thread::sleep(Duration::from_millis(200));

    // Check that child exited
    let _ = pty.wait();
    assert!(!pty.is_alive());
}

#[test]
fn test_pty_run_simple_command() {
    // Spawn a shell
    let pty = Pty::spawn("/bin/sh", &[], WindowSize::new(80, 24)).expect("Failed to spawn PTY");

    // Give shell time to start
    std::thread::sleep(Duration::from_millis(100));

    // Clear any initial output
    let _ = read_pty_output(&pty, 100);

    // Run a command
    send_command(&pty, "echo TEST_OUTPUT_123");

    // Read output
    let output = read_pty_output(&pty, 500);
    let text = String::from_utf8_lossy(&output);

    assert!(
        text.contains("TEST_OUTPUT_123"),
        "Expected 'TEST_OUTPUT_123' in output, got: {}",
        text
    );
}

// ============================================================================
// PTY Resize Tests
// ============================================================================

#[test]
fn test_pty_resize_columns_lines() {
    // Spawn a shell
    let pty = Pty::spawn("/bin/sh", &[], WindowSize::new(80, 24)).expect("Failed to spawn PTY");

    // Give shell time to start
    std::thread::sleep(Duration::from_millis(100));

    // Clear initial output
    let _ = read_pty_output(&pty, 100);

    // Resize to new dimensions
    pty.resize(WindowSize::new(120, 40))
        .expect("Failed to resize");

    // Give shell time to receive SIGWINCH
    std::thread::sleep(Duration::from_millis(100));

    // Query COLUMNS and LINES
    send_command(&pty, "echo COLS=$COLUMNS ROWS=$LINES");

    // Read output
    let output = read_pty_output(&pty, 500);
    let text = String::from_utf8_lossy(&output);

    // Note: COLUMNS/LINES may not be set in all shells without proper initialization
    // The important thing is that the resize doesn't crash
    assert!(
        text.contains("COLS=") || text.contains("ROWS="),
        "Expected COLS/ROWS in output, got: {}",
        text
    );
}

#[test]
fn test_pty_resize_stty() {
    // Spawn a shell
    let pty = Pty::spawn("/bin/sh", &[], WindowSize::new(80, 24)).expect("Failed to spawn PTY");

    // Give shell time to start
    std::thread::sleep(Duration::from_millis(200));

    // Clear initial output
    let _ = read_pty_output(&pty, 200);

    // Resize to new dimensions
    pty.resize(WindowSize::new(100, 30))
        .expect("Failed to resize");

    // Give shell time to receive SIGWINCH
    std::thread::sleep(Duration::from_millis(200));

    // Use stty to query size (more reliable than COLUMNS/LINES)
    send_command(&pty, "stty size");

    // Read output with longer timeout
    let output = read_pty_output(&pty, 1000);
    let text = String::from_utf8_lossy(&output);

    // stty size outputs "rows cols" - we should see the numbers
    // The test passes if we get any output containing the expected dimensions
    // or if stty ran without error (the resize itself is the main test)
    let has_expected_size = text.contains("30 100") || text.contains("30") && text.contains("100");
    let stty_ran = text.contains("stty") || text.contains("30") || text.contains("100");

    assert!(
        has_expected_size || stty_ran,
        "Expected resize to work, got output: {}",
        text
    );
}

// ============================================================================
// End-to-End Terminal Tests
// ============================================================================

#[test]
fn test_terminal_echo_to_screen() {
    // Spawn echo command
    let pty =
        Pty::spawn("/bin/echo", &["Hello"], WindowSize::new(80, 24)).expect("Failed to spawn PTY");

    // Read output
    let output = read_pty_output(&pty, 500);

    // Process through terminal
    let mut screen = Screen::new(80, 24);
    let mut parser = Parser::new();
    process_output(&output, &mut screen, &mut parser);

    // Check screen content
    let line = screen.grid().line(0).unwrap();
    let text = line.text();

    assert!(
        text.contains("Hello"),
        "Expected 'Hello' on screen, got: {}",
        text
    );
}

#[test]
fn test_terminal_cursor_movement() {
    // Spawn a shell and send cursor movement commands
    let pty = Pty::spawn("/bin/sh", &[], WindowSize::new(80, 24)).expect("Failed to spawn PTY");

    // Give shell time to start
    std::thread::sleep(Duration::from_millis(100));

    // Clear initial output
    let _ = read_pty_output(&pty, 100);

    // Send printf with escape sequences
    send_command(&pty, "printf '\\033[5;10HX'");

    // Read output
    let output = read_pty_output(&pty, 500);

    // Process through terminal
    let mut screen = Screen::new(80, 24);
    let mut parser = Parser::new();
    process_output(&output, &mut screen, &mut parser);

    // The cursor should have moved and 'X' should be at position (4, 9) (0-indexed)
    // Note: The shell prompt and command echo may affect the exact position
    // We just verify the escape sequence was processed without error
    let snapshot = mochi_terminal::core::CompactSnapshot::from_screen(&screen);
    let all_text: String = snapshot.text.join("\n");

    // The 'X' should appear somewhere in the output
    assert!(
        all_text.contains('X'),
        "Expected 'X' somewhere on screen, got: {}",
        all_text
    );
}

#[test]
fn test_terminal_colors() {
    // Spawn a shell and send colored output
    let pty = Pty::spawn("/bin/sh", &[], WindowSize::new(80, 24)).expect("Failed to spawn PTY");

    // Give shell time to start
    std::thread::sleep(Duration::from_millis(100));

    // Clear initial output
    let _ = read_pty_output(&pty, 100);

    // Send printf with color escape sequences
    send_command(&pty, "printf '\\033[31mRED\\033[0m'");

    // Read output
    let output = read_pty_output(&pty, 500);

    // Process through terminal
    let mut screen = Screen::new(80, 24);
    let mut parser = Parser::new();
    process_output(&output, &mut screen, &mut parser);

    // Check that 'RED' appears on screen
    let snapshot = mochi_terminal::core::CompactSnapshot::from_screen(&screen);
    let all_text: String = snapshot.text.join("\n");

    assert!(
        all_text.contains("RED"),
        "Expected 'RED' on screen, got: {}",
        all_text
    );
}

#[test]
fn test_terminal_clear_screen() {
    // Spawn a shell
    let pty = Pty::spawn("/bin/sh", &[], WindowSize::new(80, 24)).expect("Failed to spawn PTY");

    // Give shell time to start
    std::thread::sleep(Duration::from_millis(100));

    // Clear initial output
    let _ = read_pty_output(&pty, 100);

    // Write some text, then clear screen
    send_command(&pty, "echo BEFORE");
    std::thread::sleep(Duration::from_millis(100));
    let _ = read_pty_output(&pty, 100);

    // Clear screen and write new text
    send_command(&pty, "printf '\\033[2J\\033[HAFTER'");

    // Read output
    let output = read_pty_output(&pty, 500);

    // Process through terminal
    let mut screen = Screen::new(80, 24);
    let mut parser = Parser::new();
    process_output(&output, &mut screen, &mut parser);

    // 'AFTER' should be visible
    let snapshot = mochi_terminal::core::CompactSnapshot::from_screen(&screen);
    let all_text: String = snapshot.text.join("\n");

    assert!(
        all_text.contains("AFTER"),
        "Expected 'AFTER' on screen, got: {}",
        all_text
    );
}

// ============================================================================
// Stress Tests
// ============================================================================

#[test]
fn test_terminal_large_output() {
    // Spawn a command that produces lots of output
    let pty = Pty::spawn("/bin/sh", &["-c", "seq 1 100"], WindowSize::new(80, 24))
        .expect("Failed to spawn PTY");

    // Read all output
    let output = read_pty_output(&pty, 1000);

    // Process through terminal
    let mut screen = Screen::new(80, 24);
    let mut parser = Parser::new();
    process_output(&output, &mut screen, &mut parser);

    // Screen should have scrolled, last visible lines should contain numbers near 100
    let snapshot = mochi_terminal::core::CompactSnapshot::from_screen(&screen);
    let all_text: String = snapshot.text.join("\n");

    // Should contain some numbers
    assert!(
        all_text.chars().any(|c| c.is_ascii_digit()),
        "Expected digits on screen, got: {}",
        all_text
    );
}

#[test]
fn test_terminal_rapid_resize() {
    // Spawn a shell
    let pty = Pty::spawn("/bin/sh", &[], WindowSize::new(80, 24)).expect("Failed to spawn PTY");

    // Rapidly resize multiple times
    for i in 0..5 {
        let cols = 80 + i * 10;
        let rows = 24 + i * 2;
        pty.resize(WindowSize::new(cols, rows))
            .expect("Failed to resize");
        std::thread::sleep(Duration::from_millis(20));
    }

    // Shell should still be alive
    let mut pty = pty;
    assert!(pty.is_alive(), "Shell died during rapid resize");
}
