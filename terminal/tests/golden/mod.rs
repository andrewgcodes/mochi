//! Golden tests for terminal emulation
//!
//! These tests verify that the terminal produces correct output for known input sequences.
//! Each test feeds a byte sequence to the terminal and compares the resulting state
//! against an expected snapshot.

use mochi_terminal::core::{CompactSnapshot, Screen, Snapshot};
use mochi_terminal::parser::Parser;

/// Helper to run a golden test
fn run_golden_test(input: &[u8], cols: usize, rows: usize) -> (Screen, CompactSnapshot) {
    let mut screen = Screen::new(cols, rows);
    let mut parser = Parser::new();

    let actions = parser.feed(input);
    for action in actions {
        screen.apply(action);
    }

    let snapshot = CompactSnapshot::from_screen(&screen);
    (screen, snapshot)
}

/// Helper to run a golden test with chunked input (tests streaming)
fn run_golden_test_chunked(input: &[u8], cols: usize, rows: usize, chunk_size: usize) -> Screen {
    let mut screen = Screen::new(cols, rows);
    let mut parser = Parser::new();

    for chunk in input.chunks(chunk_size) {
        let actions = parser.feed(chunk);
        for action in actions {
            screen.apply(action);
        }
    }

    screen
}

// ============================================================================
// Basic printing tests
// ============================================================================

#[test]
fn test_simple_text() {
    let (_, snapshot) = run_golden_test(b"Hello, World!", 80, 24);

    assert_eq!(snapshot.cursor_row, 0);
    assert_eq!(snapshot.cursor_col, 13);
    assert_eq!(snapshot.text[0], "Hello, World!");
}

#[test]
fn test_multiline_text() {
    // Note: LF alone doesn't reset column (per VT100 spec)
    // Use CR+LF for proper line breaks
    let (_, snapshot) = run_golden_test(b"Line 1\r\nLine 2\r\nLine 3", 80, 24);

    assert_eq!(snapshot.text[0], "Line 1");
    assert_eq!(snapshot.text[1], "Line 2");
    assert_eq!(snapshot.text[2], "Line 3");
    assert_eq!(snapshot.cursor_row, 2);
}

#[test]
fn test_carriage_return() {
    let (_, snapshot) = run_golden_test(b"AAAA\rBB", 80, 24);

    assert_eq!(snapshot.text[0], "BBAA");
    assert_eq!(snapshot.cursor_col, 2);
}

#[test]
fn test_backspace() {
    let (_, snapshot) = run_golden_test(b"ABC\x08X", 80, 24);

    assert_eq!(snapshot.text[0], "ABX");
    assert_eq!(snapshot.cursor_col, 3);
}

#[test]
fn test_tab() {
    let (_, snapshot) = run_golden_test(b"A\tB\tC", 80, 24);

    // Tabs are at columns 0, 8, 16, 24, ...
    assert_eq!(snapshot.cursor_col, 17); // After 'C' at column 16
    let line = &snapshot.text[0];
    assert!(line.contains('A'));
    assert!(line.contains('B'));
    assert!(line.contains('C'));
}

#[test]
fn test_line_wrap() {
    // Fill a line and wrap
    let input: Vec<u8> = (0..85).map(|i| b'A' + (i % 26)).collect();
    let (_, snapshot) = run_golden_test(&input, 80, 24);

    assert_eq!(snapshot.cursor_row, 1);
    assert_eq!(snapshot.cursor_col, 5);
    assert_eq!(snapshot.text[0].len(), 80);
}

// ============================================================================
// CSI cursor movement tests
// ============================================================================

#[test]
fn test_csi_cursor_up() {
    // Move to row 5, then up 3
    let (_, snapshot) = run_golden_test(b"\x1b[6;1H\x1b[3A", 80, 24);

    assert_eq!(snapshot.cursor_row, 2); // 5 - 3 = 2 (0-indexed: row 5 is index 5, minus 3 = 2)
}

#[test]
fn test_csi_cursor_down() {
    let (_, snapshot) = run_golden_test(b"\x1b[5B", 80, 24);

    assert_eq!(snapshot.cursor_row, 5);
}

#[test]
fn test_csi_cursor_forward() {
    let (_, snapshot) = run_golden_test(b"\x1b[10C", 80, 24);

    assert_eq!(snapshot.cursor_col, 10);
}

#[test]
fn test_csi_cursor_back() {
    let (_, snapshot) = run_golden_test(b"\x1b[20;20H\x1b[5D", 80, 24);

    assert_eq!(snapshot.cursor_col, 14); // 19 - 5 = 14 (0-indexed)
}

#[test]
fn test_csi_cursor_position() {
    let (_, snapshot) = run_golden_test(b"\x1b[10;20H", 80, 24);

    assert_eq!(snapshot.cursor_row, 9); // 10 - 1 = 9 (0-indexed)
    assert_eq!(snapshot.cursor_col, 19); // 20 - 1 = 19 (0-indexed)
}

#[test]
fn test_csi_cursor_home() {
    let (_, snapshot) = run_golden_test(b"\x1b[10;20H\x1b[H", 80, 24);

    assert_eq!(snapshot.cursor_row, 0);
    assert_eq!(snapshot.cursor_col, 0);
}

#[test]
fn test_csi_cursor_horizontal_absolute() {
    let (_, snapshot) = run_golden_test(b"\x1b[5;10H\x1b[25G", 80, 24);

    assert_eq!(snapshot.cursor_row, 4); // Unchanged
    assert_eq!(snapshot.cursor_col, 24); // 25 - 1 = 24
}

#[test]
fn test_csi_cursor_vertical_absolute() {
    let (_, snapshot) = run_golden_test(b"\x1b[5;10H\x1b[15d", 80, 24);

    assert_eq!(snapshot.cursor_row, 14); // 15 - 1 = 14
    assert_eq!(snapshot.cursor_col, 9); // Unchanged
}

// ============================================================================
// CSI erase tests
// ============================================================================

#[test]
fn test_erase_to_end_of_line() {
    let (_, snapshot) = run_golden_test(b"ABCDEFGH\x1b[1;4H\x1b[K", 80, 24);

    assert_eq!(snapshot.text[0], "ABC");
}

#[test]
fn test_erase_to_start_of_line() {
    let (_, snapshot) = run_golden_test(b"ABCDEFGH\x1b[1;4H\x1b[1K", 80, 24);

    // Erases from start to cursor (inclusive)
    assert_eq!(snapshot.text[0].trim_start(), "EFGH");
}

#[test]
fn test_erase_entire_line() {
    let (_, snapshot) = run_golden_test(b"ABCDEFGH\x1b[1;4H\x1b[2K", 80, 24);

    assert_eq!(snapshot.text[0].trim(), "");
}

#[test]
fn test_erase_to_end_of_display() {
    let (_, snapshot) = run_golden_test(b"Line1\nLine2\nLine3\x1b[2;1H\x1b[J", 80, 24);

    assert_eq!(snapshot.text[0], "Line1");
    assert_eq!(snapshot.text[1].trim(), "");
    assert_eq!(snapshot.text[2].trim(), "");
}

#[test]
fn test_erase_entire_display() {
    let (_, snapshot) = run_golden_test(b"Line1\nLine2\nLine3\x1b[2J", 80, 24);

    assert_eq!(snapshot.text[0].trim(), "");
    assert_eq!(snapshot.text[1].trim(), "");
    assert_eq!(snapshot.text[2].trim(), "");
}

// ============================================================================
// CSI insert/delete tests
// ============================================================================

#[test]
fn test_insert_characters() {
    let (_, snapshot) = run_golden_test(b"ABCDEF\x1b[1;3H\x1b[2@XX", 80, 24);

    // Insert 2 chars at position 2, then write XX
    assert_eq!(snapshot.text[0], "ABXXCDEF");
}

#[test]
fn test_delete_characters() {
    let (_, snapshot) = run_golden_test(b"ABCDEFGH\x1b[1;3H\x1b[2P", 80, 24);

    // Delete 2 chars at position 2
    assert_eq!(snapshot.text[0], "ABEFGH");
}

#[test]
fn test_erase_characters() {
    let (_, snapshot) = run_golden_test(b"ABCDEFGH\x1b[1;3H\x1b[3X", 80, 24);

    // Erase 3 chars at position 2 (replace with spaces)
    assert_eq!(snapshot.text[0], "AB   FGH");
}

#[test]
fn test_insert_lines() {
    // Use CR+LF for proper line breaks
    let (_, snapshot) = run_golden_test(b"Line1\r\nLine2\r\nLine3\x1b[2;1H\x1b[L", 80, 24);

    assert_eq!(snapshot.text[0], "Line1");
    assert_eq!(snapshot.text[1].trim(), ""); // Inserted blank line
    assert_eq!(snapshot.text[2], "Line2");
}

#[test]
fn test_delete_lines() {
    // Use CR+LF for proper line breaks
    let (_, snapshot) = run_golden_test(b"Line1\r\nLine2\r\nLine3\x1b[2;1H\x1b[M", 80, 24);

    assert_eq!(snapshot.text[0], "Line1");
    assert_eq!(snapshot.text[1], "Line3");
}

// ============================================================================
// SGR (color/attribute) tests
// ============================================================================

#[test]
fn test_sgr_bold() {
    let (screen, _) = run_golden_test(b"\x1b[1mBold\x1b[0m", 80, 24);

    let snapshot = Snapshot::from_screen(&screen);
    assert!(snapshot.lines[0].cells[0].attrs.bold);
    assert!(!snapshot.lines[0].cells[4].attrs.bold); // After reset
}

#[test]
fn test_sgr_italic() {
    let (screen, _) = run_golden_test(b"\x1b[3mItalic\x1b[0m", 80, 24);

    let snapshot = Snapshot::from_screen(&screen);
    assert!(snapshot.lines[0].cells[0].attrs.italic);
}

#[test]
fn test_sgr_underline() {
    let (screen, _) = run_golden_test(b"\x1b[4mUnderline\x1b[0m", 80, 24);

    let snapshot = Snapshot::from_screen(&screen);
    assert!(snapshot.lines[0].cells[0].attrs.underline);
}

#[test]
fn test_sgr_inverse() {
    let (screen, _) = run_golden_test(b"\x1b[7mInverse\x1b[0m", 80, 24);

    let snapshot = Snapshot::from_screen(&screen);
    assert!(snapshot.lines[0].cells[0].attrs.inverse);
}

#[test]
fn test_sgr_16_colors() {
    use mochi_terminal::core::Color;

    let (screen, _) = run_golden_test(b"\x1b[31;42mRed on Green\x1b[0m", 80, 24);

    let snapshot = Snapshot::from_screen(&screen);
    assert_eq!(snapshot.lines[0].cells[0].fg, Color::Indexed(1)); // Red
    assert_eq!(snapshot.lines[0].cells[0].bg, Color::Indexed(2)); // Green
}

#[test]
fn test_sgr_256_colors() {
    use mochi_terminal::core::Color;

    let (screen, _) = run_golden_test(b"\x1b[38;5;196mRed256\x1b[0m", 80, 24);

    let snapshot = Snapshot::from_screen(&screen);
    assert_eq!(snapshot.lines[0].cells[0].fg, Color::Indexed(196));
}

#[test]
fn test_sgr_truecolor() {
    use mochi_terminal::core::Color;

    let (screen, _) = run_golden_test(b"\x1b[38;2;255;128;64mTruecolor\x1b[0m", 80, 24);

    let snapshot = Snapshot::from_screen(&screen);
    assert_eq!(snapshot.lines[0].cells[0].fg, Color::Rgb(255, 128, 64));
}

#[test]
fn test_sgr_reset_specific() {
    let (screen, _) = run_golden_test(b"\x1b[1;3;4mBIU\x1b[22mNoB\x1b[23mNoI\x1b[24mNoU", 80, 24);

    let snapshot = Snapshot::from_screen(&screen);
    // After "BIU" - all set
    assert!(snapshot.lines[0].cells[0].attrs.bold);
    assert!(snapshot.lines[0].cells[0].attrs.italic);
    assert!(snapshot.lines[0].cells[0].attrs.underline);
    // After "NoB" - bold reset
    assert!(!snapshot.lines[0].cells[3].attrs.bold);
    assert!(snapshot.lines[0].cells[3].attrs.italic);
    // After "NoI" - italic reset
    assert!(!snapshot.lines[0].cells[6].attrs.italic);
    assert!(snapshot.lines[0].cells[6].attrs.underline);
    // After "NoU" - underline reset
    assert!(!snapshot.lines[0].cells[9].attrs.underline);
}

// ============================================================================
// Scroll region tests
// ============================================================================

#[test]
fn test_scroll_region() {
    // Set scroll region to lines 2-4, then scroll
    // Use CR+LF for proper line breaks
    let (_, snapshot) = run_golden_test(
        b"Line1\r\nLine2\r\nLine3\r\nLine4\r\nLine5\x1b[2;4r\x1b[4;1H\n",
        80,
        24,
    );

    assert_eq!(snapshot.text[0], "Line1");
    // Lines 2-4 should have scrolled
    assert_eq!(snapshot.text[1], "Line3");
    assert_eq!(snapshot.text[2], "Line4");
    assert_eq!(snapshot.text[4], "Line5");
}

// ============================================================================
// ESC sequence tests
// ============================================================================

#[test]
fn test_esc_save_restore_cursor() {
    let (_, snapshot) = run_golden_test(b"\x1b[5;10H\x1b7\x1b[1;1HA\x1b8B", 80, 24);

    // Cursor should be restored to (4, 9) then moved right by 'B'
    assert_eq!(snapshot.cursor_row, 4);
    assert_eq!(snapshot.cursor_col, 10);
}

#[test]
fn test_esc_index() {
    let (_, snapshot) = run_golden_test(b"\x1b[5;1H\x1bD", 80, 24);

    // IND moves cursor down
    assert_eq!(snapshot.cursor_row, 5);
}

#[test]
fn test_esc_reverse_index() {
    let (_, snapshot) = run_golden_test(b"\x1b[5;1H\x1bM", 80, 24);

    // RI moves cursor up
    assert_eq!(snapshot.cursor_row, 3);
}

#[test]
fn test_esc_next_line() {
    let (_, snapshot) = run_golden_test(b"\x1b[5;10H\x1bE", 80, 24);

    // NEL moves to start of next line
    assert_eq!(snapshot.cursor_row, 5);
    assert_eq!(snapshot.cursor_col, 0);
}

// ============================================================================
// OSC tests
// ============================================================================

#[test]
fn test_osc_set_title() {
    let (screen, _) = run_golden_test(b"\x1b]0;My Title\x07", 80, 24);

    assert_eq!(screen.title(), "My Title");
}

#[test]
fn test_osc_set_title_st() {
    // Using ST (ESC \) instead of BEL
    let (screen, _) = run_golden_test(b"\x1b]2;Another Title\x1b\\", 80, 24);

    assert_eq!(screen.title(), "Another Title");
}

// ============================================================================
// Mode tests
// ============================================================================

#[test]
fn test_cursor_visibility() {
    let (screen, _) = run_golden_test(b"\x1b[?25l", 80, 24);

    assert!(!screen.cursor().is_visible());

    let (screen, _) = run_golden_test(b"\x1b[?25l\x1b[?25h", 80, 24);

    assert!(screen.cursor().is_visible());
}

#[test]
fn test_alternate_screen() {
    // Write to primary, switch to alternate, write there, switch back
    let (screen, _) = run_golden_test(b"Primary\x1b[?1049hAlternate\x1b[?1049l", 80, 24);

    // Should be back on primary screen with "Primary" visible
    let snapshot = CompactSnapshot::from_screen(&screen);
    assert_eq!(snapshot.text[0], "Primary");
}

#[test]
fn test_bracketed_paste_mode() {
    let (screen, _) = run_golden_test(b"\x1b[?2004h", 80, 24);

    assert!(screen.modes().bracketed_paste);

    let (screen, _) = run_golden_test(b"\x1b[?2004h\x1b[?2004l", 80, 24);

    assert!(!screen.modes().bracketed_paste);
}

// ============================================================================
// Chunk boundary tests (streaming correctness)
// ============================================================================

#[test]
fn test_chunk_boundary_csi() {
    // Test that CSI sequences work correctly when split across chunks
    let input = b"\x1b[10;20H";

    // Test with various chunk sizes
    for chunk_size in 1..=input.len() {
        let screen = run_golden_test_chunked(input, 80, 24, chunk_size);
        let snapshot = CompactSnapshot::from_screen(&screen);
        assert_eq!(
            snapshot.cursor_row, 9,
            "Failed with chunk_size={}",
            chunk_size
        );
        assert_eq!(
            snapshot.cursor_col, 19,
            "Failed with chunk_size={}",
            chunk_size
        );
    }
}

#[test]
fn test_chunk_boundary_osc() {
    let input = b"\x1b]0;Test Title\x07";

    for chunk_size in 1..=input.len() {
        let screen = run_golden_test_chunked(input, 80, 24, chunk_size);
        assert_eq!(
            screen.title(),
            "Test Title",
            "Failed with chunk_size={}",
            chunk_size
        );
    }
}

#[test]
fn test_chunk_boundary_utf8() {
    // UTF-8 multi-byte character split across chunks
    let input = "Hello, 世界!".as_bytes();

    for chunk_size in 1..=input.len() {
        let screen = run_golden_test_chunked(input, 80, 24, chunk_size);
        let snapshot = CompactSnapshot::from_screen(&screen);
        assert!(
            snapshot.text[0].contains("世界"),
            "Failed with chunk_size={}, got: '{}'",
            chunk_size,
            snapshot.text[0]
        );
    }
}

#[test]
fn test_chunk_boundary_sgr() {
    let input = b"\x1b[38;2;255;128;64mColor\x1b[0m";

    for chunk_size in 1..=input.len() {
        let screen = run_golden_test_chunked(input, 80, 24, chunk_size);
        let snapshot = Snapshot::from_screen(&screen);
        assert_eq!(
            snapshot.lines[0].cells[0].fg,
            mochi_terminal::core::Color::Rgb(255, 128, 64),
            "Failed with chunk_size={}",
            chunk_size
        );
    }
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_cursor_bounds() {
    // Try to move cursor beyond screen bounds
    let (_, snapshot) = run_golden_test(b"\x1b[100;100H", 80, 24);

    assert_eq!(snapshot.cursor_row, 23); // Clamped to last row
    assert_eq!(snapshot.cursor_col, 79); // Clamped to last column
}

#[test]
fn test_empty_params() {
    // CSI with empty params should use defaults
    let (_, snapshot) = run_golden_test(b"\x1b[;H", 80, 24);

    assert_eq!(snapshot.cursor_row, 0);
    assert_eq!(snapshot.cursor_col, 0);
}

#[test]
fn test_cancel_sequence() {
    // CAN (0x18) should cancel an escape sequence
    let (_, snapshot) = run_golden_test(b"\x1b[5\x18A", 80, 24);

    // The CSI should be cancelled, 'A' should be printed
    assert_eq!(snapshot.text[0], "A");
}

#[test]
fn test_c0_in_escape() {
    // C0 controls should be executed even in the middle of escape sequences
    let (_, snapshot) = run_golden_test(b"\x1b[5\nH", 80, 24);

    // LF should be executed, then CSI 5 H should position cursor
    assert_eq!(snapshot.cursor_row, 4); // CSI 5 H = row 4 (0-indexed)
}

// ============================================================================
// Complex sequence tests (real-world scenarios)
// ============================================================================

#[test]
fn test_vim_like_clear_screen() {
    // Typical vim startup sequence
    let (_, snapshot) = run_golden_test(b"\x1b[?1049h\x1b[H\x1b[2J", 80, 24);

    // Should be on alternate screen, cursor at home, screen cleared
    assert_eq!(snapshot.cursor_row, 0);
    assert_eq!(snapshot.cursor_col, 0);
}

#[test]
fn test_prompt_with_colors() {
    // Typical bash prompt with colors
    let input = b"\x1b[32muser\x1b[0m@\x1b[34mhost\x1b[0m:\x1b[36m~/dir\x1b[0m$ ";
    let (screen, snapshot) = run_golden_test(input, 80, 24);

    assert!(snapshot.text[0].contains("user@host:~/dir$"));

    // Check colors
    let full_snapshot = Snapshot::from_screen(&screen);
    assert_eq!(
        full_snapshot.lines[0].cells[0].fg,
        mochi_terminal::core::Color::Indexed(2)
    ); // Green
}

#[test]
fn test_progress_bar() {
    // Simulate a progress bar update
    let (_, snapshot) = run_golden_test(b"[          ]\r[####      ]", 80, 24);

    assert_eq!(snapshot.text[0], "[####      ]");
}

#[test]
fn test_status_line() {
    // Write content, then update status line at bottom
    let input = b"Content\n\x1b[24;1H\x1b[7mStatus Line\x1b[0m\x1b[1;1H";
    let (screen, snapshot) = run_golden_test(input, 80, 24);

    assert_eq!(snapshot.text[0], "Content");
    assert!(snapshot.text[23].contains("Status Line"));

    // Check inverse attribute on status line
    let full_snapshot = Snapshot::from_screen(&screen);
    assert!(full_snapshot.lines[23].cells[0].attrs.inverse);
}
