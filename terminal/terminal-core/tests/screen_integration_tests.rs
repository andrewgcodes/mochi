#![allow(clippy::len_zero, clippy::clone_on_copy)]
//! Additional integration tests for Screen combining multiple operations
//!
//! ~150 tests covering complex screen interactions, edge cases, and
//! future terminal feature guardrails.

use terminal_core::{
    parse_charset_designation, translate_char, CellAttributes, Charset, Color, CursorStyle,
    Dimensions, Line, Point, Screen, Scrollback, Selection, SelectionType,
};

// ============================================================================
// Screen Print + Cursor Interaction Tests
// ============================================================================

#[test]
fn test_screen_print_then_backspace() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.print('B');
    screen.backspace();
    screen.print('C');
    assert_eq!(screen.cursor().col, 2);
}

#[test]
fn test_screen_print_then_carriage_return_then_print() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.print('B');
    screen.print('C');
    screen.carriage_return();
    screen.print('X');
    assert_eq!(screen.cursor().col, 1);
}

#[test]
fn test_screen_fill_entire_first_row() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for c in "ABCDEFGHIJ".chars() {
        screen.print(c);
    }
    assert_eq!(screen.cursor().row, 0);
}

#[test]
fn test_screen_fill_entire_screen() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    for i in 0..15 {
        screen.print((b'A' + (i % 26) as u8) as char);
    }
    assert_eq!(screen.cursor().row, 2);
}

#[test]
fn test_screen_overflow_screen_scrolls() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    for i in 0..20 {
        screen.print((b'A' + (i % 26) as u8) as char);
    }
    assert_eq!(screen.cursor().row, 2);
}

#[test]
fn test_screen_linefeed_then_print() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.linefeed();
    screen.print('B');
    assert_eq!(screen.cursor().row, 1);
}

#[test]
fn test_screen_next_line_then_print() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.print('B');
    screen.next_line();
    screen.print('C');
    assert_eq!(screen.cursor().row, 1);
    assert_eq!(screen.cursor().col, 1);
}

#[test]
fn test_screen_tab_then_print() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.tab();
    screen.print('A');
    assert_eq!(screen.cursor().col, 9);
}

#[test]
fn test_screen_multiple_tabs() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.tab();
    assert_eq!(screen.cursor().col, 8);
    screen.tab();
    assert_eq!(screen.cursor().col, 16);
    screen.tab();
    assert_eq!(screen.cursor().col, 24);
}

#[test]
fn test_screen_tab_custom_stops() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    // Clear all tab stops
    screen.clear_tab_stop(3);
    // Set custom stops at col 5 and col 10
    screen.move_cursor_to(1, 6); // col 5 (1-indexed col=6 -> 0-indexed col=5)
    screen.set_tab_stop();
    screen.move_cursor_to(1, 11); // col 10 (1-indexed col=11 -> 0-indexed col=10)
    screen.set_tab_stop();
    // Go back to start
    screen.move_cursor_to(1, 1);
    screen.tab();
    assert_eq!(screen.cursor().col, 5);
    screen.tab();
    assert_eq!(screen.cursor().col, 10);
}

// ============================================================================
// Cursor Movement Edge Cases
// ============================================================================

#[test]
fn test_screen_move_cursor_to_zero_zero() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.print('B');
    screen.move_cursor_to(1, 1);
    assert_eq!(screen.cursor().row, 0);
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_move_cursor_large_values() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(1000, 1000);
    assert_eq!(screen.cursor().row, 23);
    assert_eq!(screen.cursor().col, 79);
}

#[test]
fn test_screen_cursor_right_at_edge() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.move_cursor_right(100);
    assert_eq!(screen.cursor().col, 9);
}

#[test]
fn test_screen_cursor_down_at_bottom() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.move_cursor_down(100);
    assert_eq!(screen.cursor().row, 4);
}

#[test]
fn test_screen_cursor_left_at_zero() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.move_cursor_left(10);
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_cursor_up_at_zero() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.move_cursor_up(10);
    assert_eq!(screen.cursor().row, 0);
}

#[test]
fn test_screen_move_cursor_sequence() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(5, 10);
    assert_eq!(screen.cursor().row, 4);
    assert_eq!(screen.cursor().col, 9);
    screen.move_cursor_up(2);
    assert_eq!(screen.cursor().row, 2);
    screen.move_cursor_down(1);
    assert_eq!(screen.cursor().row, 3);
    screen.move_cursor_left(3);
    assert_eq!(screen.cursor().col, 6);
    screen.move_cursor_right(5);
    assert_eq!(screen.cursor().col, 11);
}

#[test]
fn test_screen_set_cursor_col_zero() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_cursor_col(0);
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_set_cursor_row_zero() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_cursor_row(0);
    assert_eq!(screen.cursor().row, 0);
}

#[test]
fn test_screen_set_cursor_col_beyond_cols() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_cursor_col(1000);
    assert_eq!(screen.cursor().col, 79);
}

#[test]
fn test_screen_set_cursor_row_beyond_rows() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_cursor_row(1000);
    assert_eq!(screen.cursor().row, 23);
}

// ============================================================================
// Erase Operations
// ============================================================================

#[test]
fn test_screen_erase_display_below_from_middle() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for _ in 0..50 {
        screen.print('X');
    }
    screen.move_cursor_to(3, 5);
    screen.erase_display(0);
    assert_eq!(screen.cursor().row, 2);
    assert_eq!(screen.cursor().col, 4);
}

#[test]
fn test_screen_erase_display_above_from_middle() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for _ in 0..50 {
        screen.print('X');
    }
    screen.move_cursor_to(3, 5);
    screen.erase_display(1);
    assert_eq!(screen.cursor().row, 2);
    assert_eq!(screen.cursor().col, 4);
}

#[test]
fn test_screen_erase_display_all_preserves_cursor() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for _ in 0..50 {
        screen.print('X');
    }
    screen.move_cursor_to(3, 5);
    screen.erase_display(2);
    assert_eq!(screen.cursor().row, 2);
    assert_eq!(screen.cursor().col, 4);
}

#[test]
fn test_screen_erase_line_right_from_start() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for c in "ABCDEFGHIJ".chars() {
        screen.print(c);
    }
    screen.carriage_return();
    screen.erase_line(0);
}

#[test]
fn test_screen_erase_line_left_from_end() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for c in "ABCDEFGHIJ".chars() {
        screen.print(c);
    }
    screen.erase_line(1);
}

#[test]
fn test_screen_erase_chars_boundary() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for c in "ABCDEFGHIJ".chars() {
        screen.print(c);
    }
    screen.carriage_return();
    screen.erase_chars(5);
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_erase_chars_more_than_available() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for c in "ABCDEFGHIJ".chars() {
        screen.print(c);
    }
    screen.carriage_return();
    screen.erase_chars(100);
}

// ============================================================================
// Insert/Delete Line Operations
// ============================================================================

#[test]
fn test_screen_insert_lines_at_top() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for c in "HELLO".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 1);
    screen.insert_lines(1);
}

#[test]
fn test_screen_insert_lines_at_bottom() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.move_cursor_to(5, 1);
    screen.insert_lines(1);
}

#[test]
fn test_screen_insert_lines_more_than_available() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.insert_lines(100);
}

#[test]
fn test_screen_delete_lines_at_top() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for c in "HELLO".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 1);
    screen.delete_lines(1);
}

#[test]
fn test_screen_delete_lines_more_than_available() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.delete_lines(100);
}

#[test]
fn test_screen_insert_chars_at_start() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for c in "ABCDE".chars() {
        screen.print(c);
    }
    screen.carriage_return();
    screen.insert_chars(3);
}

#[test]
fn test_screen_delete_chars_at_start() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for c in "ABCDE".chars() {
        screen.print(c);
    }
    screen.carriage_return();
    screen.delete_chars(3);
}

// ============================================================================
// Scroll Region Tests
// ============================================================================

#[test]
fn test_screen_scroll_region_linefeed_inside() {
    let mut screen = Screen::new(Dimensions::new(10, 10));
    screen.set_scroll_region(3, 7);
    screen.move_cursor_to(7, 1);
    screen.linefeed();
}

#[test]
fn test_screen_scroll_region_reverse_index_inside() {
    let mut screen = Screen::new(Dimensions::new(10, 10));
    screen.set_scroll_region(3, 7);
    screen.move_cursor_to(3, 1);
    screen.reverse_index();
}

#[test]
fn test_screen_scroll_region_invalid_same_top_bottom() {
    let mut screen = Screen::new(Dimensions::new(10, 10));
    screen.set_scroll_region(5, 5);
    assert_eq!(screen.scroll_region(), (0, 9));
}

#[test]
fn test_screen_scroll_region_invalid_reversed() {
    let mut screen = Screen::new(Dimensions::new(10, 10));
    screen.set_scroll_region(7, 3);
    assert_eq!(screen.scroll_region(), (0, 9));
}

#[test]
fn test_screen_scroll_region_full_screen() {
    let mut screen = Screen::new(Dimensions::new(10, 10));
    screen.set_scroll_region(1, 10);
    assert_eq!(screen.scroll_region(), (0, 9));
}

#[test]
fn test_screen_scroll_up_with_region() {
    let mut screen = Screen::new(Dimensions::new(10, 10));
    screen.set_scroll_region(3, 7);
    screen.scroll_up(1);
}

#[test]
fn test_screen_scroll_down_with_region() {
    let mut screen = Screen::new(Dimensions::new(10, 10));
    screen.set_scroll_region(3, 7);
    screen.scroll_down(1);
}

#[test]
fn test_screen_scroll_up_multiple() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for _ in 0..25 {
        screen.print('X');
    }
    screen.scroll_up(3);
}

#[test]
fn test_screen_scroll_down_multiple() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for _ in 0..25 {
        screen.print('X');
    }
    screen.scroll_down(3);
}

// ============================================================================
// Alternate Screen Tests
// ============================================================================

#[test]
fn test_screen_alternate_preserves_primary_content() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for c in "HELLO".chars() {
        screen.print(c);
    }
    screen.enter_alternate_screen();
    for c in "WORLD".chars() {
        screen.print(c);
    }
    screen.exit_alternate_screen();
}

#[test]
fn test_screen_alternate_cursor_independent() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(5, 10);
    screen.enter_alternate_screen();
    assert_eq!(screen.cursor().row, 0);
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_alternate_screen_multiple_transitions() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    for _ in 0..5 {
        screen.enter_alternate_screen();
        screen.print('A');
        screen.exit_alternate_screen();
    }
}

// ============================================================================
// Resize Tests
// ============================================================================

#[test]
fn test_screen_resize_larger() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.resize(Dimensions::new(120, 40));
    assert_eq!(screen.cols(), 120);
    assert_eq!(screen.rows(), 40);
}

#[test]
fn test_screen_resize_smaller_clamps_cursor() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(20, 70);
    screen.resize(Dimensions::new(40, 10));
    assert!(screen.cursor().row < 10);
    assert!(screen.cursor().col < 40);
}

#[test]
fn test_screen_resize_same_size() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.resize(Dimensions::new(80, 24));
    assert_eq!(screen.cols(), 80);
    assert_eq!(screen.rows(), 24);
}

#[test]
fn test_screen_resize_minimum() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.resize(Dimensions::new(1, 1));
    assert_eq!(screen.cols(), 1);
    assert_eq!(screen.rows(), 1);
}

#[test]
fn test_screen_resize_preserves_content() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for c in "HELLO".chars() {
        screen.print(c);
    }
    screen.resize(Dimensions::new(20, 10));
}

// ============================================================================
// Title and Hyperlink Tests
// ============================================================================

#[test]
fn test_screen_set_title_long() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let long_title = "A".repeat(10000);
    screen.set_title(&long_title);
    assert!(screen.title().len() <= 10000);
}

#[test]
fn test_screen_set_title_unicode() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_title("日本語テスト");
    assert_eq!(screen.title(), "日本語テスト");
}

#[test]
fn test_screen_set_title_empty() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_title("hello");
    screen.set_title("");
    assert_eq!(screen.title(), "");
}

#[test]
fn test_screen_hyperlink_multiple() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let id1 = screen.register_hyperlink("https://example.com");
    let id2 = screen.register_hyperlink("https://rust-lang.org");
    assert_ne!(id1, id2);
    assert_eq!(screen.get_hyperlink(id1), Some("https://example.com"));
    assert_eq!(screen.get_hyperlink(id2), Some("https://rust-lang.org"));
}

#[test]
fn test_screen_hyperlink_same_url_same_id() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let id1 = screen.register_hyperlink("https://example.com");
    let id2 = screen.register_hyperlink("https://example.com");
    assert_eq!(id1, id2);
}

#[test]
fn test_screen_hyperlink_invalid_id() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert_eq!(screen.get_hyperlink(999), None);
}

// ============================================================================
// Save/Restore Cursor Tests
// ============================================================================

#[test]
fn test_screen_save_restore_cursor_position() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(10, 20);
    screen.save_cursor();
    screen.move_cursor_to(1, 1);
    screen.restore_cursor();
    assert_eq!(screen.cursor().row, 9);
    assert_eq!(screen.cursor().col, 19);
}

#[test]
fn test_screen_save_restore_cursor_attrs() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.cursor_mut().attrs.bold = true;
    screen.cursor_mut().attrs.fg = Color::Indexed(1);
    screen.save_cursor();
    screen.cursor_mut().attrs = CellAttributes::default();
    screen.restore_cursor();
    assert!(screen.cursor().attrs.bold);
}

#[test]
fn test_screen_save_restore_cursor_alternate() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(5, 10);
    screen.save_cursor();
    screen.enter_alternate_screen();
    screen.move_cursor_to(15, 30);
    screen.save_cursor();
    screen.restore_cursor();
    assert_eq!(screen.cursor().row, 14);
    assert_eq!(screen.cursor().col, 29);
    screen.exit_alternate_screen();
    screen.restore_cursor();
    assert_eq!(screen.cursor().row, 4);
    assert_eq!(screen.cursor().col, 9);
}

#[test]
fn test_screen_restore_cursor_without_save() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(5, 10);
    screen.restore_cursor();
}

// ============================================================================
// Charset Tests
// ============================================================================

#[test]
fn test_screen_charset_dec_special() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.designate_charset(0, '0');
    screen.shift_in();
    screen.print('q');
}

#[test]
fn test_screen_charset_uk() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.designate_charset(0, 'A');
    screen.shift_in();
}

#[test]
fn test_screen_charset_switch_back_and_forth() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.designate_charset(0, '0');
    screen.designate_charset(1, 'B');
    screen.shift_in();
    screen.shift_out();
    screen.shift_in();
}

// ============================================================================
// Insert Mode Tests
// ============================================================================

#[test]
fn test_screen_insert_mode_on() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.modes_mut().insert_mode = true;
    for c in "ABCDE".chars() {
        screen.print(c);
    }
    screen.carriage_return();
    screen.print('X');
}

#[test]
fn test_screen_insert_mode_off() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.modes_mut().insert_mode = false;
    for c in "ABCDE".chars() {
        screen.print(c);
    }
    screen.carriage_return();
    screen.print('X');
}

// ============================================================================
// Modes Tests
// ============================================================================

#[test]
fn test_screen_origin_mode() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(5, 15);
    screen.modes_mut().origin_mode = true;
    screen.move_cursor_to(1, 1);
    assert_eq!(screen.cursor().row, 4);
}

#[test]
fn test_screen_auto_wrap_mode() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    screen.modes_mut().auto_wrap = true;
    for c in "ABCDEFGHIJ".chars() {
        screen.print(c);
    }
    assert_eq!(screen.cursor().row, 1);
}

#[test]
fn test_screen_auto_wrap_off_stays_at_edge() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    screen.modes_mut().auto_wrap = false;
    for c in "ABCDEFGHIJ".chars() {
        screen.print(c);
    }
    assert_eq!(screen.cursor().row, 0);
    assert_eq!(screen.cursor().col, 4);
}

// ============================================================================
// Snapshot Tests
// ============================================================================

#[test]
fn test_screen_snapshot_dimensions() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.print('A');
    let snapshot = screen.snapshot(false);
    assert_eq!(snapshot.dimensions.cols, 10);
    assert_eq!(snapshot.dimensions.rows, 5);
}

#[test]
fn test_screen_snapshot_cursor_position() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.move_cursor_to(3, 5);
    let snapshot = screen.snapshot(false);
    assert_eq!(snapshot.cursor.row, 2);
    assert_eq!(snapshot.cursor.col, 4);
}

#[test]
fn test_screen_snapshot_cursor_visible() {
    let screen = Screen::new(Dimensions::new(10, 5));
    let snapshot = screen.snapshot(false);
    assert!(snapshot.cursor.visible);
}

#[test]
fn test_screen_snapshot_cursor_hidden() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.cursor_mut().visible = false;
    let snapshot = screen.snapshot(false);
    assert!(!snapshot.cursor.visible);
}

#[test]
fn test_screen_snapshot_title() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.set_title("Test Title");
    let snapshot = screen.snapshot(false);
    assert_eq!(snapshot.title.as_deref(), Some("Test Title"));
}

#[test]
fn test_screen_snapshot_screen_lines_count() {
    let screen = Screen::new(Dimensions::new(10, 5));
    let snapshot = screen.snapshot(false);
    assert_eq!(snapshot.screen.len(), 5);
}

#[test]
fn test_screen_snapshot_json_roundtrip() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    screen.print('B');
    let snapshot = screen.snapshot(false);
    let json = serde_json::to_string(&snapshot).unwrap();
    let parsed: terminal_core::Snapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.dimensions.cols, snapshot.dimensions.cols);
    assert_eq!(parsed.dimensions.rows, snapshot.dimensions.rows);
    assert_eq!(parsed.cursor.row, snapshot.cursor.row);
    assert_eq!(parsed.cursor.col, snapshot.cursor.col);
}

#[test]
fn test_screen_snapshot_with_scrollback() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for i in 0..10 {
        for c in format!("Line {}", i).chars() {
            screen.print(c);
        }
        screen.linefeed();
        screen.carriage_return();
    }
    let snapshot = screen.snapshot(true);
    assert!(snapshot.scrollback.is_some());
}

#[test]
fn test_screen_snapshot_without_scrollback() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    let snapshot = screen.snapshot(false);
    assert!(snapshot.scrollback.is_none());
}

#[test]
fn test_screen_snapshot_modes() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.modes_mut().auto_wrap = true;
    screen.modes_mut().insert_mode = true;
    let snapshot = screen.snapshot(false);
    assert!(snapshot.modes.auto_wrap);
    assert!(snapshot.modes.insert_mode);
}

// ============================================================================
// Selection Tests
// ============================================================================

#[test]
fn test_selection_start_and_update() {
    let mut sel = Selection::default();
    sel.start(Point::new(5, 0), SelectionType::Normal);
    sel.update(Point::new(10, 2));
    assert!(sel.active);
}

#[test]
fn test_selection_clear_makes_inactive() {
    let mut sel = Selection::default();
    sel.start(Point::new(5, 0), SelectionType::Normal);
    sel.clear();
    assert!(!sel.active);
}

#[test]
fn test_selection_bounds_normalized() {
    let mut sel = Selection::default();
    sel.start(Point::new(10, 2), SelectionType::Normal);
    sel.update(Point::new(5, 0));
    let bounds = sel.bounds();
    assert!(
        bounds.0.row <= bounds.1.row
            || (bounds.0.row == bounds.1.row && bounds.0.col <= bounds.1.col)
    );
}

#[test]
fn test_selection_contains_point_inside() {
    let mut sel = Selection::default();
    sel.start(Point::new(0, 0), SelectionType::Normal);
    sel.update(Point::new(10, 2));
    sel.finish();
    // contains(col, row) - row 1 is between start row 0 and end row 2
    assert!(sel.contains(5, 1));
}

#[test]
fn test_selection_does_not_contain_outside() {
    let mut sel = Selection::default();
    sel.start(Point::new(0, 0), SelectionType::Normal);
    sel.update(Point::new(10, 2));
    sel.finish();
    assert!(!sel.contains(3, 5));
}

#[test]
fn test_selection_word_type() {
    let mut sel = Selection::default();
    sel.start(Point::new(5, 1), SelectionType::Word);
    assert_eq!(sel.selection_type, SelectionType::Word);
}

#[test]
fn test_selection_line_type() {
    let mut sel = Selection::default();
    sel.start(Point::new(5, 1), SelectionType::Line);
    assert_eq!(sel.selection_type, SelectionType::Line);
}

#[test]
fn test_selection_block_type() {
    let mut sel = Selection::default();
    sel.start(Point::new(5, 1), SelectionType::Block);
    assert_eq!(sel.selection_type, SelectionType::Block);
}

// ============================================================================
// Scrollback Integration Tests
// ============================================================================

#[test]
fn test_scrollback_default_capacity() {
    let scrollback = Scrollback::default();
    assert_eq!(scrollback.len(), 0);
}

#[test]
fn test_scrollback_push_and_get() {
    let mut scrollback = Scrollback::new(100);
    let line = Line::new(10);
    scrollback.push(line);
    assert_eq!(scrollback.len(), 1);
    assert!(scrollback.get(0).is_some());
}

#[test]
fn test_scrollback_ring_buffer_behavior() {
    let mut scrollback = Scrollback::new(3);
    for i in 0..5u8 {
        let mut line = Line::new(10);
        line.cell_mut(0).set_char((b'A' + i) as char);
        scrollback.push(line);
    }
    assert_eq!(scrollback.len(), 3);
}

#[test]
fn test_scrollback_clear() {
    let mut scrollback = Scrollback::new(100);
    for _ in 0..10 {
        scrollback.push(Line::new(5));
    }
    scrollback.clear();
    assert_eq!(scrollback.len(), 0);
}

// ============================================================================
// Grid Access Tests
// ============================================================================

#[test]
fn test_screen_grid_line_access() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for c in "HELLO".chars() {
        screen.print(c);
    }
    let line = screen.line(0);
    assert_eq!(line.cell(0).display_char(), 'H');
    assert_eq!(line.cell(1).display_char(), 'E');
    assert_eq!(line.cell(2).display_char(), 'L');
    assert_eq!(line.cell(3).display_char(), 'L');
    assert_eq!(line.cell(4).display_char(), 'O');
}

#[test]
fn test_screen_grid_dimensions() {
    let screen = Screen::new(Dimensions::new(80, 24));
    let dims = screen.dimensions();
    assert_eq!(dims.cols, 80);
    assert_eq!(dims.rows, 24);
}

// ============================================================================
// Reset Tests
// ============================================================================

#[test]
fn test_screen_reset_clears_content() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for _ in 0..50 {
        screen.print('X');
    }
    screen.reset();
    assert_eq!(screen.cursor().row, 0);
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_reset_clears_title() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.set_title("test");
    screen.reset();
    assert_eq!(screen.title(), "");
}

#[test]
fn test_screen_reset_clears_modes() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.modes_mut().insert_mode = true;
    screen.modes_mut().origin_mode = true;
    screen.reset();
}

// ============================================================================
// Complex Interaction Tests
// ============================================================================

#[test]
fn test_screen_print_all_printable_ascii() {
    let mut screen = Screen::new(Dimensions::new(100, 5));
    for c in (0x20u8..=0x7Eu8).map(|b| b as char) {
        screen.print(c);
    }
    assert_eq!(screen.cursor().col, 95);
}

#[test]
fn test_screen_print_unicode_cjk() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('日');
    screen.print('本');
}

#[test]
fn test_screen_print_emoji() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('😀');
}

#[test]
fn test_screen_rapid_resize() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    for c in "HELLO WORLD".chars() {
        screen.print(c);
    }
    for i in 0..20 {
        let cols = 40 + (i % 80) as usize;
        let rows = 10 + (i % 30) as usize;
        screen.resize(Dimensions::new(cols, rows));
    }
}

#[test]
fn test_screen_print_after_erase_all() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for _ in 0..50 {
        screen.print('X');
    }
    screen.erase_display(2);
    screen.move_cursor_to(1, 1);
    screen.print('A');
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
}

#[test]
fn test_screen_scrollback_after_scroll() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for i in 0..10 {
        for c in format!("Line {}", i).chars() {
            screen.print(c);
        }
        screen.linefeed();
        screen.carriage_return();
    }
    assert!(screen.scrollback().len() > 0);
}

#[test]
fn test_screen_print_with_all_attrs() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.cursor_mut().attrs.bold = true;
    screen.cursor_mut().attrs.italic = true;
    screen.cursor_mut().attrs.underline = true;
    screen.cursor_mut().attrs.blink = true;
    screen.cursor_mut().attrs.inverse = true;
    screen.cursor_mut().attrs.hidden = true;
    screen.cursor_mut().attrs.strikethrough = true;
    screen.cursor_mut().attrs.fg = Color::Indexed(1);
    screen.cursor_mut().attrs.bg = Color::Indexed(4);
    screen.print('X');
    let cell = screen.line(0).cell(0);
    assert!(cell.attrs.bold);
    assert!(cell.attrs.italic);
    assert!(cell.attrs.underline);
}

#[test]
fn test_screen_cursor_styles() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.cursor_mut().style = CursorStyle::Block;
    assert_eq!(screen.cursor().style, CursorStyle::Block);
    screen.cursor_mut().style = CursorStyle::Underline;
    assert_eq!(screen.cursor().style, CursorStyle::Underline);
    screen.cursor_mut().style = CursorStyle::Bar;
    assert_eq!(screen.cursor().style, CursorStyle::Bar);
}

#[test]
fn test_screen_cursor_visibility_toggle() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    assert!(screen.cursor().visible);
    screen.cursor_mut().visible = false;
    assert!(!screen.cursor().visible);
    screen.cursor_mut().visible = true;
    assert!(screen.cursor().visible);
}

// ============================================================================
// Translate Char Tests
// ============================================================================

#[test]
fn test_translate_char_dec_all_box_drawing() {
    let mappings = vec![
        ('j', '┘'),
        ('k', '┐'),
        ('l', '┌'),
        ('m', '└'),
        ('n', '┼'),
        ('q', '─'),
        ('t', '├'),
        ('u', '┤'),
        ('v', '┴'),
        ('w', '┬'),
        ('x', '│'),
    ];
    for (input, expected) in mappings {
        assert_eq!(
            translate_char(input, Charset::DecSpecialGraphics),
            expected,
            "DEC mapping for '{}' should be '{}'",
            input,
            expected
        );
    }
}

#[test]
fn test_translate_char_dec_misc() {
    assert_eq!(translate_char('`', Charset::DecSpecialGraphics), '◆');
    assert_eq!(translate_char('a', Charset::DecSpecialGraphics), '▒');
    assert_eq!(translate_char('f', Charset::DecSpecialGraphics), '°');
    assert_eq!(translate_char('g', Charset::DecSpecialGraphics), '±');
}

#[test]
fn test_translate_char_ascii_passthrough() {
    for c in (0x20u8..=0x7E).map(|b| b as char) {
        assert_eq!(
            translate_char(c, Charset::Ascii),
            c,
            "ASCII charset should pass through '{}'",
            c
        );
    }
}

#[test]
fn test_translate_char_uk_pound() {
    assert_eq!(translate_char('#', Charset::Uk), '£');
}

#[test]
fn test_translate_char_uk_other_passthrough() {
    assert_eq!(translate_char('A', Charset::Uk), 'A');
    assert_eq!(translate_char('z', Charset::Uk), 'z');
}

// ============================================================================
// Parse Charset Designation Tests
// ============================================================================

#[test]
fn test_parse_charset_designation_ascii_b() {
    assert_eq!(parse_charset_designation('B'), Charset::Ascii);
}

#[test]
fn test_parse_charset_designation_ascii_at() {
    assert_eq!(parse_charset_designation('@'), Charset::Ascii);
}

#[test]
fn test_parse_charset_designation_dec_0() {
    assert_eq!(parse_charset_designation('0'), Charset::DecSpecialGraphics);
}

#[test]
fn test_parse_charset_designation_dec_2() {
    assert_eq!(parse_charset_designation('2'), Charset::DecSpecialGraphics);
}

#[test]
fn test_parse_charset_designation_uk() {
    assert_eq!(parse_charset_designation('A'), Charset::Uk);
}

#[test]
fn test_parse_charset_designation_unknown_defaults_ascii() {
    assert_eq!(parse_charset_designation('Z'), Charset::Ascii);
    assert_eq!(parse_charset_designation('X'), Charset::Ascii);
}

// ============================================================================
// Dimensions Tests
// ============================================================================

#[test]
fn test_dimensions_new() {
    let dims = Dimensions::new(80, 24);
    assert_eq!(dims.cols, 80);
    assert_eq!(dims.rows, 24);
}

#[test]
fn test_dimensions_equality() {
    assert_eq!(Dimensions::new(80, 24), Dimensions::new(80, 24));
    assert_ne!(Dimensions::new(80, 24), Dimensions::new(120, 40));
}

#[test]
fn test_dimensions_clone() {
    let dims = Dimensions::new(80, 24);
    let cloned = dims.clone();
    assert_eq!(dims, cloned);
}

#[test]
fn test_dimensions_copy() {
    let dims = Dimensions::new(80, 24);
    let copied = dims;
    assert_eq!(dims, copied);
}

#[test]
fn test_dimensions_debug() {
    let dims = Dimensions::new(80, 24);
    let debug = format!("{:?}", dims);
    assert!(debug.contains("80"));
    assert!(debug.contains("24"));
}

#[test]
fn test_dimensions_default() {
    let dims = Dimensions::default();
    assert_eq!(dims.cols, 80);
    assert_eq!(dims.rows, 24);
}

// ============================================================================
// Screen Stress / Edge Case Tests
// ============================================================================

#[test]
fn test_screen_many_linefeeds() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    for _ in 0..1000 {
        screen.linefeed();
    }
    assert_eq!(screen.cursor().row, 23);
}

#[test]
fn test_screen_many_reverse_indexes() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(12, 1);
    for _ in 0..1000 {
        screen.reverse_index();
    }
    assert_eq!(screen.cursor().row, 0);
}

#[test]
fn test_screen_alternating_print_and_linefeed() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for i in 0..100 {
        screen.print((b'A' + (i % 26) as u8) as char);
        screen.linefeed();
        screen.carriage_return();
    }
}

#[test]
fn test_screen_erase_display_all_then_fill() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for _ in 0..50 {
        screen.print('X');
    }
    screen.erase_display(2);
    screen.move_cursor_to(1, 1);
    for _ in 0..50 {
        screen.print('Y');
    }
}

#[test]
fn test_screen_concurrent_scroll_region_operations() {
    let mut screen = Screen::new(Dimensions::new(20, 20));
    screen.set_scroll_region(5, 15);
    screen.move_cursor_to(15, 1);
    for _ in 0..20 {
        screen.print('A');
        screen.linefeed();
        screen.carriage_return();
    }
    screen.scroll_up(5);
    screen.scroll_down(3);
}

#[test]
fn test_screen_alternate_screen_with_scrollback() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for i in 0..20 {
        for c in format!("L{}", i).chars() {
            screen.print(c);
        }
        screen.linefeed();
        screen.carriage_return();
    }
    let scrollback_before = screen.scrollback().len();
    assert!(scrollback_before > 0);
    screen.enter_alternate_screen();
    screen.print('X');
    screen.exit_alternate_screen();
    assert_eq!(screen.scrollback().len(), scrollback_before);
}

#[test]
fn test_screen_backspace_at_line_start() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.backspace();
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_many_backspaces() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.print('B');
    screen.print('C');
    for _ in 0..10 {
        screen.backspace();
    }
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_tab_at_end_of_line() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    screen.move_cursor_to(1, 10);
    screen.tab();
    assert!(screen.cursor().col <= 9);
}

#[test]
fn test_screen_cursor_col_at_every_position() {
    let mut screen = Screen::new(Dimensions::new(20, 5));
    for i in 1..=20 {
        screen.set_cursor_col(i);
        assert_eq!(screen.cursor().col, (i - 1).min(19));
    }
}

#[test]
fn test_screen_cursor_row_at_every_position() {
    let mut screen = Screen::new(Dimensions::new(20, 10));
    for i in 1..=10 {
        screen.set_cursor_row(i);
        assert_eq!(screen.cursor().row, (i - 1).min(9));
    }
}

#[test]
fn test_screen_mixed_operations_stress() {
    let mut screen = Screen::new(Dimensions::new(40, 20));
    for i in 0..200u32 {
        match i % 7 {
            0 => screen.print((b'A' + (i as u8 % 26)) as char),
            1 => screen.linefeed(),
            2 => screen.carriage_return(),
            3 => screen.move_cursor_right(1),
            4 => screen.move_cursor_down(1),
            5 => screen.backspace(),
            6 => screen.tab(),
            _ => unreachable!(),
        }
    }
}

#[test]
fn test_screen_print_newline_preserves_col() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.print('B');
    let col_before = screen.cursor().col;
    screen.linefeed();
    assert_eq!(screen.cursor().col, col_before);
}

#[test]
fn test_screen_multiple_scroll_regions_sequential() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(1, 10);
    assert_eq!(screen.scroll_region(), (0, 9));
    screen.set_scroll_region(5, 20);
    assert_eq!(screen.scroll_region(), (4, 19));
    screen.set_scroll_region(1, 24);
    assert_eq!(screen.scroll_region(), (0, 23));
}

// ============================================================================
// Additional Line Tests
// ============================================================================

#[test]
fn test_line_new_has_correct_length() {
    let line = Line::new(80);
    assert_eq!(line.cols(), 80);
}

#[test]
fn test_line_cell_access() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('A');
    assert_eq!(line.cell(0).display_char(), 'A');
}

#[test]
fn test_line_cell_mut_set_char() {
    let mut line = Line::new(10);
    for i in 0..10 {
        line.cell_mut(i).set_char((b'A' + i as u8) as char);
    }
    for i in 0..10 {
        assert_eq!(line.cell(i).display_char(), (b'A' + i as u8) as char);
    }
}

#[test]
fn test_line_wrapped_flag() {
    let mut line = Line::new(10);
    assert!(!line.wrapped);
    line.wrapped = true;
    assert!(line.wrapped);
    line.wrapped = false;
    assert!(!line.wrapped);
}

#[test]
fn test_line_clear() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('A');
    line.clear(CellAttributes::default());
    assert_eq!(line.cell(0).display_char(), ' ');
}
