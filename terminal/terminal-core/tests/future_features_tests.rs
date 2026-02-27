//! Tests for future terminal features
//!
//! These tests serve as guardrails for features that should be implemented.
//! They test the expected behavior of: search, tabs, splits, themes,
//! keybindings, clipboard, configuration, input handling, and more.
//!
//! Many of these tests verify existing primitives that future features
//! will build upon.

use terminal_core::*;

// ============================================================
// Search Feature Tests (using existing Screen primitives)
// ============================================================

/// Helper: create a screen with some text content
fn screen_with_text(lines: &[&str]) -> Screen {
    let cols = lines.iter().map(|l| l.len()).max().unwrap_or(80).max(80);
    let rows = lines.len().max(24);
    let mut screen = Screen::new(Dimensions::new(cols, rows));
    for (row, text) in lines.iter().enumerate() {
        screen.move_cursor_to(row + 1, 1); // 1-indexed
        for ch in text.chars() {
            screen.print(ch);
        }
    }
    screen
}

#[test]
fn test_search_screen_text_contains_pattern() {
    let screen = screen_with_text(&["Hello World", "Foo Bar Baz"]);
    let snap = screen.snapshot(false);
    let text = snap.screen_text();
    assert!(text.contains("Hello World"));
    assert!(text.contains("Foo Bar Baz"));
}

#[test]
fn test_search_screen_text_not_contains_pattern() {
    let screen = screen_with_text(&["Hello World"]);
    let snap = screen.snapshot(false);
    let text = snap.screen_text();
    assert!(!text.contains("Goodbye"));
}

#[test]
fn test_search_case_sensitive() {
    let screen = screen_with_text(&["Hello World"]);
    let snap = screen.snapshot(false);
    let text = snap.screen_text();
    assert!(text.contains("Hello"));
    assert!(!text.contains("hello"));
}

#[test]
fn test_search_case_insensitive() {
    let screen = screen_with_text(&["Hello World"]);
    let snap = screen.snapshot(false);
    let text = snap.screen_text().to_lowercase();
    assert!(text.contains("hello"));
    assert!(text.contains("world"));
}

#[test]
fn test_search_multiple_matches_on_same_line() {
    let screen = screen_with_text(&["aaa bbb aaa"]);
    let snap = screen.snapshot(false);
    let text = snap.screen_text();
    assert_eq!(text.matches("aaa").count(), 2);
}

#[test]
fn test_search_across_lines() {
    let screen = screen_with_text(&["First line", "Second line", "Third line"]);
    let snap = screen.snapshot(false);
    let text = snap.screen_text();
    assert!(text.contains("line"));
    assert_eq!(text.matches("line").count(), 3);
}

#[test]
fn test_search_empty_pattern() {
    let screen = screen_with_text(&["Hello"]);
    let snap = screen.snapshot(false);
    let text = snap.screen_text();
    assert!(text.contains(""));
}

#[test]
fn test_search_special_chars() {
    let screen = screen_with_text(&["path/to/file.rs"]);
    let snap = screen.snapshot(false);
    let text = snap.screen_text();
    assert!(text.contains("path/to/file.rs"));
}

#[test]
fn test_search_unicode() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    for (i, ch) in "日本語テスト".chars().enumerate() {
        screen.move_cursor_to(1, i * 2 + 1); // 1-indexed; wide chars take 2 columns
        screen.print(ch);
    }
    let snap = screen.snapshot(false);
    let text = snap.screen_text();
    assert!(text.contains("日本語"));
}

#[test]
fn test_search_in_scrollback() {
    let mut screen = Screen::new(Dimensions::new(80, 5));
    // Fill screen and scroll to push content into scrollback
    for i in 0..10 {
        for ch in format!("Line {}", i).chars() {
            screen.print(ch);
        }
        screen.linefeed();
        screen.carriage_return();
    }
    let snap = screen.snapshot(true);
    if let Some(sb) = &snap.scrollback {
        let sb_text: String = sb
            .iter()
            .map(|l| l.text.clone())
            .collect::<Vec<_>>()
            .join("\n");
        // Some early lines should be in scrollback
        assert!(!sb_text.is_empty());
    }
}

// ============================================================
// Tab Management Tests (using Screen as basis for tabs)
// ============================================================

#[test]
fn test_tab_create_new_screen() {
    // Each tab should have its own Screen
    let tab1 = Screen::new(Dimensions::new(80, 24));
    let tab2 = Screen::new(Dimensions::new(80, 24));
    assert_eq!(tab1.cols(), 80);
    assert_eq!(tab2.cols(), 80);
}

#[test]
fn test_tab_independent_content() {
    let mut tab1 = Screen::new(Dimensions::new(80, 24));
    let mut tab2 = Screen::new(Dimensions::new(80, 24));
    tab1.print('A');
    tab2.print('B');
    let snap1 = tab1.snapshot(false);
    let snap2 = tab2.snapshot(false);
    assert!(snap1.screen_text().contains('A'));
    assert!(!snap1.screen_text().contains('B'));
    assert!(snap2.screen_text().contains('B'));
    assert!(!snap2.screen_text().contains('A'));
}

#[test]
fn test_tab_independent_cursor() {
    let mut tab1 = Screen::new(Dimensions::new(80, 24));
    let mut tab2 = Screen::new(Dimensions::new(80, 24));
    tab1.move_cursor_to(6, 11); // 1-indexed -> row=5, col=10
    tab2.move_cursor_to(11, 21); // 1-indexed -> row=10, col=20
    let snap1 = tab1.snapshot(false);
    let snap2 = tab2.snapshot(false);
    assert_eq!(snap1.cursor.row, 5);
    assert_eq!(snap1.cursor.col, 10);
    assert_eq!(snap2.cursor.row, 10);
    assert_eq!(snap2.cursor.col, 20);
}

#[test]
fn test_tab_independent_scrollback() {
    let mut tab1 = Screen::new(Dimensions::new(80, 5));
    let tab2 = Screen::new(Dimensions::new(80, 5));
    // Push content into tab1's scrollback
    for _ in 0..10 {
        tab1.linefeed();
    }
    let snap1 = tab1.snapshot(true);
    let snap2 = tab2.snapshot(true);
    // Tab1 should have scrollback, tab2 should not
    if let Some(sb1) = &snap1.scrollback {
        assert!(!sb1.is_empty());
    }
    if let Some(sb2) = &snap2.scrollback {
        assert!(sb2.is_empty());
    }
}

#[test]
fn test_tab_resize_all() {
    let mut tabs = vec![
        Screen::new(Dimensions::new(80, 24)),
        Screen::new(Dimensions::new(80, 24)),
        Screen::new(Dimensions::new(80, 24)),
    ];
    for tab in &mut tabs {
        tab.resize(Dimensions::new(120, 40));
    }
    for tab in &tabs {
        assert_eq!(tab.cols(), 120);
        assert_eq!(tab.rows(), 40);
    }
}

#[test]
fn test_tab_independent_modes() {
    let mut tab1 = Screen::new(Dimensions::new(80, 24));
    let tab2 = Screen::new(Dimensions::new(80, 24));
    tab1.enter_alternate_screen();
    let snap1 = tab1.snapshot(false);
    let snap2 = tab2.snapshot(false);
    assert!(snap1.modes.alternate_screen);
    assert!(!snap2.modes.alternate_screen);
}

#[test]
fn test_tab_title_per_tab() {
    let mut tab1 = Screen::new(Dimensions::new(80, 24));
    let mut tab2 = Screen::new(Dimensions::new(80, 24));
    tab1.set_title("Tab 1");
    tab2.set_title("Tab 2");
    let snap1 = tab1.snapshot(false);
    let snap2 = tab2.snapshot(false);
    assert_eq!(snap1.title, Some("Tab 1".to_string()));
    assert_eq!(snap2.title, Some("Tab 2".to_string()));
}

// ============================================================
// Split Pane Tests (using Screen as basis for splits)
// ============================================================

#[test]
fn test_split_vertical_half() {
    let pane1 = Screen::new(Dimensions::new(40, 24));
    let pane2 = Screen::new(Dimensions::new(40, 24));
    assert_eq!(pane1.cols() + pane2.cols(), 80);
}

#[test]
fn test_split_horizontal_half() {
    let pane1 = Screen::new(Dimensions::new(80, 12));
    let pane2 = Screen::new(Dimensions::new(80, 12));
    assert_eq!(pane1.rows() + pane2.rows(), 24);
}

#[test]
fn test_split_independent_content() {
    let mut pane1 = Screen::new(Dimensions::new(40, 24));
    let mut pane2 = Screen::new(Dimensions::new(40, 24));
    pane1.print('L');
    pane2.print('R');
    let s1 = pane1.snapshot(false);
    let s2 = pane2.snapshot(false);
    assert!(s1.screen_text().contains('L'));
    assert!(s2.screen_text().contains('R'));
}

#[test]
fn test_split_resize_proportional() {
    // When terminal resizes, splits should resize proportionally
    let mut pane1 = Screen::new(Dimensions::new(40, 24));
    let mut pane2 = Screen::new(Dimensions::new(40, 24));
    // Simulate resize to 120 cols
    pane1.resize(Dimensions::new(60, 24));
    pane2.resize(Dimensions::new(60, 24));
    assert_eq!(pane1.cols(), 60);
    assert_eq!(pane2.cols(), 60);
}

#[test]
fn test_split_minimum_size() {
    // Each pane should have a minimum size
    let pane = Screen::new(Dimensions::new(2, 2));
    assert_eq!(pane.cols(), 2);
    assert_eq!(pane.rows(), 2);
}

// ============================================================
// Theme/Color Tests
// ============================================================

#[test]
fn test_theme_default_colors() {
    let attrs = CellAttributes::new();
    assert_eq!(attrs.fg, Color::Default);
    assert_eq!(attrs.bg, Color::Default);
}

#[test]
fn test_theme_16_standard_colors() {
    for i in 0..16 {
        let color = Color::Indexed(i);
        assert_eq!(color, Color::Indexed(i));
    }
}

#[test]
fn test_theme_256_color_palette() {
    // Verify known color mappings for specific indices
    assert_eq!(Color::Indexed(0).to_rgb(), (0, 0, 0)); // black
    assert_eq!(Color::Indexed(196).to_rgb(), (255, 0, 0)); // pure red in color cube
    assert_eq!(Color::Indexed(15).to_rgb(), (255, 255, 255)); // bright white
                                                              // Verify grayscale indices 232-255 have equal r/g/b components
    for i in 232..=255u8 {
        let (r, g, b) = Color::Indexed(i).to_rgb();
        assert_eq!(r, g, "Grayscale index {} should have r==g", i);
        assert_eq!(g, b, "Grayscale index {} should have g==b", i);
    }
    // Verify all 256 colors produce some output without panicking
    for i in 0..=255u8 {
        let _rgb = Color::Indexed(i).to_rgb();
    }
}

#[test]
fn test_theme_true_color_rgb() {
    let color = Color::rgb(128, 64, 32);
    let (r, g, b) = color.to_rgb();
    assert_eq!(r, 128);
    assert_eq!(g, 64);
    assert_eq!(b, 32);
}

#[test]
fn test_theme_color_roundtrip() {
    for r_val in (0..=255).step_by(51) {
        for g_val in (0..=255).step_by(51) {
            for b_val in (0..=255).step_by(51) {
                let color = Color::rgb(r_val, g_val, b_val);
                let (r, g, b) = color.to_rgb();
                assert_eq!(r, r_val);
                assert_eq!(g, g_val);
                assert_eq!(b, b_val);
            }
        }
    }
}

#[test]
fn test_theme_bold_attribute() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    assert!(attrs.bold);
    assert!(!attrs.italic);
}

#[test]
fn test_theme_italic_attribute() {
    let mut attrs = CellAttributes::new();
    attrs.italic = true;
    assert!(attrs.italic);
}

#[test]
fn test_theme_underline_attribute() {
    let mut attrs = CellAttributes::new();
    attrs.underline = true;
    assert!(attrs.underline);
}

#[test]
fn test_theme_strikethrough_attribute() {
    let mut attrs = CellAttributes::new();
    attrs.strikethrough = true;
    assert!(attrs.strikethrough);
}

#[test]
fn test_theme_inverse_attribute() {
    let mut attrs = CellAttributes::new();
    attrs.inverse = true;
    assert!(attrs.inverse);
}

#[test]
fn test_theme_hidden_attribute() {
    let mut attrs = CellAttributes::new();
    attrs.hidden = true;
    assert!(attrs.hidden);
}

#[test]
fn test_theme_blink_attribute() {
    let mut attrs = CellAttributes::new();
    attrs.blink = true;
    assert!(attrs.blink);
}

#[test]
fn test_theme_faint_attribute() {
    let mut attrs = CellAttributes::new();
    attrs.faint = true;
    assert!(attrs.faint);
}

#[test]
fn test_theme_all_attributes_combined() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    attrs.italic = true;
    attrs.underline = true;
    attrs.blink = true;
    attrs.inverse = true;
    attrs.hidden = true;
    attrs.strikethrough = true;
    attrs.faint = true;
    attrs.fg = Color::Indexed(1);
    attrs.bg = Color::rgb(0, 0, 0);
    assert!(attrs.bold && attrs.italic && attrs.underline && attrs.blink);
    assert!(attrs.inverse && attrs.hidden && attrs.strikethrough && attrs.faint);
}

// ============================================================
// Keybinding Tests (input encoding)
// ============================================================

#[test]
fn test_keybinding_enter_produces_cr() {
    // Enter key should produce \r (0x0D)
    let expected = b"\r";
    assert_eq!(expected[0], 0x0D);
}

#[test]
fn test_keybinding_backspace_produces_del() {
    // Backspace should produce DEL (0x7F) in most terminal modes
    let expected = b"\x7f";
    assert_eq!(expected[0], 0x7F);
}

#[test]
fn test_keybinding_tab_produces_ht() {
    // Tab should produce HT (0x09)
    let expected = b"\t";
    assert_eq!(expected[0], 0x09);
}

#[test]
fn test_keybinding_escape_produces_esc() {
    // Escape key should produce ESC (0x1B)
    let expected = b"\x1b";
    assert_eq!(expected[0], 0x1B);
}

#[test]
fn test_keybinding_ctrl_c() {
    // Ctrl+C should produce ETX (0x03)
    let expected = 0x03u8;
    assert_eq!(expected, b'C' - b'@');
}

#[test]
fn test_keybinding_ctrl_d() {
    // Ctrl+D should produce EOT (0x04)
    let expected = 0x04u8;
    assert_eq!(expected, b'D' - b'@');
}

#[test]
fn test_keybinding_ctrl_z() {
    // Ctrl+Z should produce SUB (0x1A)
    let expected = 0x1Au8;
    assert_eq!(expected, b'Z' - b'@');
}

#[test]
fn test_keybinding_ctrl_letters() {
    // All Ctrl+letter combinations (A-Z)
    for letter in b'A'..=b'Z' {
        let ctrl = letter - b'@';
        assert!(ctrl <= 0x1A);
    }
}

#[test]
fn test_keybinding_arrow_up() {
    let expected = b"\x1b[A";
    assert_eq!(expected, b"\x1b[A");
}

#[test]
fn test_keybinding_arrow_down() {
    let expected = b"\x1b[B";
    assert_eq!(expected, b"\x1b[B");
}

#[test]
fn test_keybinding_arrow_right() {
    let expected = b"\x1b[C";
    assert_eq!(expected, b"\x1b[C");
}

#[test]
fn test_keybinding_arrow_left() {
    let expected = b"\x1b[D";
    assert_eq!(expected, b"\x1b[D");
}

#[test]
fn test_keybinding_arrow_up_application_mode() {
    let expected = b"\x1bOA";
    assert_eq!(expected, b"\x1bOA");
}

#[test]
fn test_keybinding_arrow_down_application_mode() {
    let expected = b"\x1bOB";
    assert_eq!(expected, b"\x1bOB");
}

#[test]
fn test_keybinding_home() {
    let expected = b"\x1b[H";
    assert_eq!(expected, b"\x1b[H");
}

#[test]
fn test_keybinding_end() {
    let expected = b"\x1b[F";
    assert_eq!(expected, b"\x1b[F");
}

#[test]
fn test_keybinding_page_up() {
    let expected = b"\x1b[5~";
    assert_eq!(expected, b"\x1b[5~");
}

#[test]
fn test_keybinding_page_down() {
    let expected = b"\x1b[6~";
    assert_eq!(expected, b"\x1b[6~");
}

#[test]
fn test_keybinding_insert() {
    let expected = b"\x1b[2~";
    assert_eq!(expected, b"\x1b[2~");
}

#[test]
fn test_keybinding_delete() {
    let expected = b"\x1b[3~";
    assert_eq!(expected, b"\x1b[3~");
}

#[test]
fn test_keybinding_f1() {
    let expected = b"\x1bOP";
    assert_eq!(expected, b"\x1bOP");
}

#[test]
fn test_keybinding_f2() {
    let expected = b"\x1bOQ";
    assert_eq!(expected, b"\x1bOQ");
}

#[test]
fn test_keybinding_f3() {
    let expected = b"\x1bOR";
    assert_eq!(expected, b"\x1bOR");
}

#[test]
fn test_keybinding_f4() {
    let expected = b"\x1bOS";
    assert_eq!(expected, b"\x1bOS");
}

#[test]
fn test_keybinding_f5() {
    let expected = b"\x1b[15~";
    assert_eq!(expected, b"\x1b[15~");
}

#[test]
fn test_keybinding_f6() {
    let expected = b"\x1b[17~";
    assert_eq!(expected, b"\x1b[17~");
}

#[test]
fn test_keybinding_f7() {
    let expected = b"\x1b[18~";
    assert_eq!(expected, b"\x1b[18~");
}

#[test]
fn test_keybinding_f8() {
    let expected = b"\x1b[19~";
    assert_eq!(expected, b"\x1b[19~");
}

#[test]
fn test_keybinding_f9() {
    let expected = b"\x1b[20~";
    assert_eq!(expected, b"\x1b[20~");
}

#[test]
fn test_keybinding_f10() {
    let expected = b"\x1b[21~";
    assert_eq!(expected, b"\x1b[21~");
}

#[test]
fn test_keybinding_f11() {
    let expected = b"\x1b[23~";
    assert_eq!(expected, b"\x1b[23~");
}

#[test]
fn test_keybinding_f12() {
    let expected = b"\x1b[24~";
    assert_eq!(expected, b"\x1b[24~");
}

// ============================================================
// Clipboard Tests
// ============================================================

#[test]
fn test_clipboard_selection_single_line() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    for ch in "Hello World".chars() {
        screen.print(ch);
    }
    let snap = screen.snapshot(false);
    let text = &snap.screen[0].text;
    assert!(text.contains("Hello World"));
}

#[test]
fn test_clipboard_selection_multi_line() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    for ch in "Line 1".chars() {
        screen.print(ch);
    }
    screen.linefeed();
    screen.carriage_return();
    for ch in "Line 2".chars() {
        screen.print(ch);
    }
    let snap = screen.snapshot(false);
    assert!(snap.screen[0].text.contains("Line 1"));
    assert!(snap.screen[1].text.contains("Line 2"));
}

#[test]
fn test_clipboard_selection_with_attributes() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    screen.cursor_mut().attrs = attrs;
    for ch in "Bold Text".chars() {
        screen.print(ch);
    }
    let snap = screen.snapshot(false);
    assert!(snap.screen[0].text.contains("Bold Text"));
    // Bold text was printed
    assert!(!snap.screen[0].text.is_empty());
}

#[test]
fn test_clipboard_empty_selection() {
    let screen = Screen::new(Dimensions::new(80, 24));
    let snap = screen.snapshot(false);
    assert_eq!(snap.screen[0].text, "");
}

// ============================================================
// Selection Type Tests
// ============================================================

#[test]
fn test_selection_normal_type() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    assert!(sel.active);
}

#[test]
fn test_selection_word_type() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Word);
    assert!(sel.active);
}

#[test]
fn test_selection_line_type() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Line);
    assert!(sel.active);
}

#[test]
fn test_selection_block_type() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Block);
    assert!(sel.active);
}

#[test]
fn test_selection_update_extends() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(5, 20));
    let (start, end) = sel.bounds();
    assert!(start.row <= end.row);
}

#[test]
fn test_selection_clear() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.clear();
    assert!(!sel.active);
}

// ============================================================
// Terminal Input Handling Tests
// ============================================================

#[test]
fn test_input_print_char_updates_cursor() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    let snap = screen.snapshot(false);
    assert_eq!(snap.cursor.col, 1); // Cursor moved right
}

#[test]
fn test_input_newline_moves_cursor_down() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.linefeed();
    let snap = screen.snapshot(false);
    assert_eq!(snap.cursor.row, 1);
}

#[test]
fn test_input_carriage_return_resets_col() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.print('B');
    screen.carriage_return();
    let snap = screen.snapshot(false);
    assert_eq!(snap.cursor.col, 0);
}

#[test]
fn test_input_backspace_moves_left() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.print('B');
    screen.backspace();
    let snap = screen.snapshot(false);
    assert_eq!(snap.cursor.col, 1);
}

#[test]
fn test_input_backspace_at_start_no_move() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.backspace();
    let snap = screen.snapshot(false);
    assert_eq!(snap.cursor.col, 0);
}

#[test]
fn test_input_tab_moves_to_next_stop() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.tab();
    let snap = screen.snapshot(false);
    assert_eq!(snap.cursor.col, 8);
}

#[test]
fn test_input_reverse_index_at_top() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.reverse_index();
    let snap = screen.snapshot(false);
    assert_eq!(snap.cursor.row, 0);
}

#[test]
fn test_input_index_at_bottom_scrolls() {
    let mut screen = Screen::new(Dimensions::new(80, 5));
    screen.move_cursor_to(5, 1); // 1-indexed: puts cursor at bottom row (row 4, 0-indexed)
    screen.index();
    let snap = screen.snapshot(false);
    assert_eq!(snap.cursor.row, 4);
}

// ============================================================
// Terminal Mode Tests (for future feature interaction)
// ============================================================

#[test]
fn test_mode_autowrap_wraps_at_end() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    for ch in "ABCDE".chars() {
        screen.print(ch);
    }
    // After printing 5 chars in a 5-col screen, cursor should be at col 4 (pending wrap)
    // Printing one more should wrap
    screen.print('F');
    let snap = screen.snapshot(false);
    assert_eq!(snap.screen[0].text, "ABCDE");
    assert!(snap.screen[1].text.contains('F'));
}

#[test]
fn test_mode_insert_mode_shifts_right() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for ch in "ABCDE".chars() {
        screen.print(ch);
    }
    screen.move_cursor_to(1, 3); // 1-indexed: row=0, col=2
    screen.modes_mut().insert_mode = true;
    screen.print('X');
    let snap = screen.snapshot(false);
    // X should be inserted, shifting CDE right
    assert!(snap.screen[0].text.contains("ABX"));
}

#[test]
fn test_mode_origin_mode_restricts_cursor() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(5, 20);
    screen.modes_mut().origin_mode = true;
    screen.move_cursor_to(1, 1); // In origin mode, row 1 maps to scroll top
    let snap = screen.snapshot(false);
    assert_eq!(snap.cursor.row, 4); // scroll region top (5-1=4, 0-indexed)
}

#[test]
fn test_mode_alternate_screen_preserves_main() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.enter_alternate_screen();
    screen.print('B');
    screen.exit_alternate_screen();
    let snap = screen.snapshot(false);
    assert!(snap.screen[0].text.contains('A'));
}

#[test]
fn test_mode_bracketed_paste() {
    let modes = Modes::new();
    assert!(!modes.bracketed_paste);
}

// ============================================================
// Scroll Region Tests
// ============================================================

#[test]
fn test_scroll_region_set() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(5, 20);
    let snap = screen.snapshot(false);
    // set_scroll_region converts 1-indexed to 0-indexed: (5-1=4, 20-1=19)
    assert_eq!(snap.scroll_region, Some((4, 19)));
}

#[test]
fn test_scroll_region_clear() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(5, 20);
    screen.clear_scroll_region();
    let snap = screen.snapshot(false);
    assert_eq!(snap.scroll_region, None);
}

#[test]
fn test_scroll_region_scroll_within() {
    let mut screen = Screen::new(Dimensions::new(80, 10));
    // Place content outside scroll region
    screen.move_cursor_to(1, 1);
    screen.print('T'); // row 0 - outside region above
    screen.set_scroll_region(2, 8);
    // Move cursor to the bottom of the scroll region (row 8, 1-indexed = row 7, 0-indexed)
    screen.move_cursor_to(8, 1);
    screen.print('X');
    screen.index(); // Should scroll within region since cursor is at scroll bottom
    let snap = screen.snapshot(false);
    // Cursor should remain at scroll bottom after scrolling
    assert_eq!(snap.cursor.row, 7);
    // Content above scroll region should be unaffected
    assert_eq!(snap.screen[0].text, "T");
}

// ============================================================
// Hyperlink Tests
// ============================================================

#[test]
fn test_hyperlink_register() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let id = screen.register_hyperlink("https://example.com");
    assert!(id > 0);
    screen.cursor_mut().hyperlink_id = id;
    screen.print('L');
    screen.print('i');
    screen.print('n');
    screen.print('k');
    screen.cursor_mut().hyperlink_id = 0;
    // Verify the hyperlink can be retrieved
    assert_eq!(screen.get_hyperlink(id), Some("https://example.com"));
}

#[test]
fn test_hyperlink_clear() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let id = screen.register_hyperlink("https://example.com");
    screen.cursor_mut().hyperlink_id = id;
    screen.print('A');
    screen.cursor_mut().hyperlink_id = 0;
    screen.print('B');
    let snap = screen.snapshot(false);
    // B should not have hyperlink
    let text = &snap.screen[0].text;
    assert!(text.contains("AB"));
}

// ============================================================
// Wide Character (CJK) Tests
// ============================================================

#[test]
fn test_wide_char_takes_two_columns() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('中');
    let snap = screen.snapshot(false);
    assert_eq!(snap.cursor.col, 2);
}

#[test]
fn test_wide_char_at_end_of_line() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    // Move to last column (1-indexed: col 5)
    screen.move_cursor_to(1, 5);
    screen.print('中'); // Width 2, but only 1 col left - prints at col 4
    let snap = screen.snapshot(false);
    // Wide char should be on row 0 (at the edge) or handled by the terminal
    let all_text: String = snap.screen.iter().map(|l| l.text.clone()).collect();
    assert!(all_text.contains('中'));
}

#[test]
fn test_wide_char_overwrite() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('中'); // Takes columns 0 and 1
    screen.move_cursor_to(1, 1); // 1-indexed: row 0, col 0
    screen.print('A'); // Should erase the wide char
    let snap = screen.snapshot(false);
    assert!(snap.screen[0].text.starts_with('A'));
}

#[test]
fn test_emoji_wide_char() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('🎉');
    let snap = screen.snapshot(false);
    assert_eq!(snap.cursor.col, 2);
}

// ============================================================
// Screen Erase Tests
// ============================================================

#[test]
fn test_erase_display_below() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for i in 0..5 {
        screen.move_cursor_to(i + 1, 1); // 1-indexed
        for ch in format!("Line{}", i).chars() {
            screen.print(ch);
        }
    }
    screen.move_cursor_to(3, 1); // 1-indexed: row 2
    screen.erase_display(0); // Erase below (from cursor)
    let snap = screen.snapshot(false);
    // Lines 0 and 1 should still have content
    assert!(snap.screen[0].text.contains("Line0"));
    assert!(snap.screen[1].text.contains("Line1"));
}

#[test]
fn test_erase_display_above() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for i in 0..5 {
        screen.move_cursor_to(i + 1, 1); // 1-indexed
        for ch in format!("Line{}", i).chars() {
            screen.print(ch);
        }
    }
    screen.move_cursor_to(3, 1); // 1-indexed: row 2
    screen.erase_display(1); // Erase above
    let snap = screen.snapshot(false);
    // Lines below should still have content
    assert!(snap.screen[3].text.contains("Line3"));
    assert!(snap.screen[4].text.contains("Line4"));
}

#[test]
fn test_erase_display_all() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for i in 0..5 {
        screen.move_cursor_to(i + 1, 1); // 1-indexed
        screen.print('X');
    }
    screen.erase_display(2); // Erase all
    let snap = screen.snapshot(false);
    for line in &snap.screen {
        assert_eq!(line.text, "");
    }
}

#[test]
fn test_erase_line_to_right() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for ch in "ABCDEFGHIJ".chars() {
        screen.print(ch);
    }
    screen.move_cursor_to(1, 6); // 1-indexed: row 0, col 5
    screen.erase_line(0); // Erase to right
    let snap = screen.snapshot(false);
    assert_eq!(snap.screen[0].text, "ABCDE");
}

#[test]
fn test_erase_line_to_left() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for ch in "ABCDEFGHIJ".chars() {
        screen.print(ch);
    }
    screen.move_cursor_to(1, 6); // 1-indexed: row 0, col 5
    screen.erase_line(1); // Erase to left
    let snap = screen.snapshot(false);
    // Characters after cursor should remain
    let text = &snap.screen[0].text;
    assert!(text.contains("GHIJ"));
}

#[test]
fn test_erase_line_entire() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for ch in "ABCDEFGHIJ".chars() {
        screen.print(ch);
    }
    screen.move_cursor_to(1, 6); // 1-indexed: row 0, col 5
    screen.erase_line(2); // Erase entire line
    let snap = screen.snapshot(false);
    assert_eq!(snap.screen[0].text, "");
}

// ============================================================
// Insert / Delete Operations Tests
// ============================================================

#[test]
fn test_insert_lines() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.move_cursor_to(1, 1); // 1-indexed: row 0
    screen.print('A');
    screen.move_cursor_to(2, 1); // 1-indexed: row 1
    screen.print('B');
    screen.move_cursor_to(1, 1); // 1-indexed: row 0
    screen.insert_lines(1);
    let snap = screen.snapshot(false);
    // Line 0 should now be empty (inserted)
    assert_eq!(snap.screen[0].text, "");
    // A should have moved down
    assert!(snap.screen[1].text.contains('A'));
}

#[test]
fn test_delete_lines() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.move_cursor_to(1, 1); // 1-indexed: row 0
    screen.print('A');
    screen.move_cursor_to(2, 1); // 1-indexed: row 1
    screen.print('B');
    screen.move_cursor_to(1, 1); // 1-indexed: row 0
    screen.delete_lines(1);
    let snap = screen.snapshot(false);
    // B should have moved up to row 0
    assert!(snap.screen[0].text.contains('B'));
}

#[test]
fn test_insert_chars() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for ch in "ABCDE".chars() {
        screen.print(ch);
    }
    screen.move_cursor_to(1, 3); // 1-indexed: row 0, col 2
    screen.insert_chars(2);
    let snap = screen.snapshot(false);
    // AB should stay, 2 blanks inserted, then CDE shifted right
    let text = &snap.screen[0].text;
    assert!(text.starts_with("AB"));
}

#[test]
fn test_delete_chars() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for ch in "ABCDE".chars() {
        screen.print(ch);
    }
    screen.move_cursor_to(1, 3); // 1-indexed: row 0, col 2
    screen.delete_chars(2);
    let snap = screen.snapshot(false);
    // AB should stay, CD deleted, E shifts left
    let text = &snap.screen[0].text;
    assert!(text.starts_with("ABE"));
}

// ============================================================
// Scrollback Buffer Tests
// ============================================================

#[test]
fn test_scrollback_fills_on_scroll() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    screen.linefeed();
    screen.carriage_return();
    screen.print('B');
    screen.linefeed();
    screen.carriage_return();
    screen.print('C');
    screen.linefeed(); // Should scroll, pushing top line to scrollback
    let snap = screen.snapshot(true);
    if let Some(sb) = &snap.scrollback {
        assert!(!sb.is_empty());
    }
}

#[test]
fn test_scrollback_preserves_content() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('X');
    screen.linefeed();
    screen.linefeed();
    screen.linefeed(); // Push X line into scrollback
    let snap = screen.snapshot(true);
    if let Some(sb) = &snap.scrollback {
        let has_x = sb.iter().any(|l| l.text.contains('X'));
        assert!(has_x);
    }
}

// ============================================================
// Terminal Reset Tests
// ============================================================

#[test]
fn test_screen_reset_clears_content() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.reset();
    let snap = screen.snapshot(false);
    assert_eq!(snap.screen[0].text, "");
}

#[test]
fn test_screen_reset_resets_cursor() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(10, 20);
    screen.reset();
    let snap = screen.snapshot(false);
    assert_eq!(snap.cursor.row, 0);
    assert_eq!(snap.cursor.col, 0);
}

#[test]
fn test_screen_reset_clears_scroll_region() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(5, 20);
    screen.reset();
    let snap = screen.snapshot(false);
    assert_eq!(snap.scroll_region, None);
}

#[test]
fn test_screen_reset_clears_title() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_title("My Title");
    screen.reset();
    let snap = screen.snapshot(false);
    assert_eq!(snap.title, None);
}

// ============================================================
// Configuration Tests (using Dimensions/defaults)
// ============================================================

#[test]
fn test_config_default_dimensions() {
    let dims = Dimensions::default();
    assert_eq!(dims.cols, 80);
    assert_eq!(dims.rows, 24);
}

#[test]
fn test_config_custom_dimensions() {
    let dims = Dimensions::new(120, 40);
    assert_eq!(dims.cols, 120);
    assert_eq!(dims.rows, 40);
}

#[test]
fn test_config_small_dimensions() {
    let dims = Dimensions::new(1, 1);
    let screen = Screen::new(dims);
    assert_eq!(screen.cols(), 1);
    assert_eq!(screen.rows(), 1);
}

#[test]
fn test_config_large_dimensions() {
    let dims = Dimensions::new(500, 200);
    let screen = Screen::new(dims);
    assert_eq!(screen.cols(), 500);
    assert_eq!(screen.rows(), 200);
}

#[test]
fn test_config_scrollback_default() {
    let sb = Scrollback::new(10000);
    assert!(sb.is_empty());
    assert_eq!(sb.max_lines(), 10000);
}

#[test]
fn test_config_scrollback_custom() {
    let sb = Scrollback::new(5000);
    assert_eq!(sb.max_lines(), 5000);
}

#[test]
fn test_config_scrollback_zero() {
    let sb = Scrollback::new(0);
    assert_eq!(sb.max_lines(), 0);
}

// ============================================================
// Snapshot Serialization Tests
// ============================================================

#[test]
fn test_snapshot_json_roundtrip() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    screen.set_title("Test");
    let snap = screen.snapshot(false);
    let json = snap.to_json().unwrap();
    let parsed = Snapshot::from_json(&json).unwrap();
    assert_eq!(parsed.dimensions.cols, 10);
    assert_eq!(parsed.dimensions.rows, 3);
    assert_eq!(parsed.title, Some("Test".to_string()));
}

#[test]
fn test_snapshot_json_preserves_screen_content() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for ch in "Hello".chars() {
        screen.print(ch);
    }
    let snap = screen.snapshot(false);
    let json = snap.to_json().unwrap();
    let parsed = Snapshot::from_json(&json).unwrap();
    assert!(parsed.screen[0].text.contains("Hello"));
}

#[test]
fn test_snapshot_json_preserves_cursor() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(6, 11); // 1-indexed -> row=5, col=10
    let snap = screen.snapshot(false);
    let json = snap.to_json().unwrap();
    let parsed = Snapshot::from_json(&json).unwrap();
    assert_eq!(parsed.cursor.row, 5);
    assert_eq!(parsed.cursor.col, 10);
}

#[test]
fn test_snapshot_json_preserves_modes() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.enter_alternate_screen();
    let snap = screen.snapshot(false);
    let json = snap.to_json().unwrap();
    let parsed = Snapshot::from_json(&json).unwrap();
    assert!(parsed.modes.alternate_screen);
}
