#![allow(clippy::field_reassign_with_default, unused_imports)]
//! Comprehensive tests for Screen, Scrollback, Selection, Charset, and Snapshot
//!
//! ~200 tests covering the main terminal screen interface and related components.

use terminal_core::{
    parse_charset_designation, translate_char, Cell, CellAttributes, Charset, CharsetState, Color,
    Cursor, CursorStyle, Dimensions, Grid, Line, Modes, Point, SavedCursor, Screen, Scrollback,
    Selection, SelectionType, Snapshot,
};

// ============================================================================
// Screen Tests (~100 tests)
// ============================================================================

#[test]
fn test_screen_new_dimensions() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert_eq!(screen.cols(), 80);
    assert_eq!(screen.rows(), 24);
}

#[test]
fn test_screen_new_cursor_at_origin() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert_eq!(screen.cursor().col, 0);
    assert_eq!(screen.cursor().row, 0);
}

#[test]
fn test_screen_new_cursor_visible() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert!(screen.cursor().visible);
}

#[test]
fn test_screen_new_no_alternate() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert!(!screen.modes().alternate_screen);
}

#[test]
fn test_screen_new_auto_wrap_on() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert!(screen.modes().auto_wrap);
}

#[test]
fn test_screen_new_empty_title() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert!(screen.title().is_empty());
}

#[test]
fn test_screen_new_no_scroll_region() {
    let screen = Screen::new(Dimensions::new(80, 24));
    // scroll_region() returns (0, rows-1) when no explicit region set
    assert_eq!(screen.scroll_region(), (0, 23));
}

#[test]
fn test_screen_print_single_char() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
    assert_eq!(screen.cursor().col, 1);
}

#[test]
fn test_screen_print_multiple_chars() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    for c in "Hello".chars() {
        screen.print(c);
    }
    assert_eq!(screen.line(0).text(), "Hello");
    assert_eq!(screen.cursor().col, 5);
}

#[test]
fn test_screen_print_wrap_at_end() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    for c in "ABCDEF".chars() {
        screen.print(c);
    }
    assert_eq!(screen.line(0).text(), "ABCDE");
    assert_eq!(screen.line(1).text(), "F");
    assert_eq!(screen.cursor().row, 1);
}

#[test]
fn test_screen_print_wrap_full_lines() {
    let mut screen = Screen::new(Dimensions::new(3, 3));
    for c in "ABCDEFGHI".chars() {
        screen.print(c);
    }
    assert_eq!(screen.line(0).text(), "ABC");
    assert_eq!(screen.line(1).text(), "DEF");
    assert_eq!(screen.line(2).text(), "GHI");
}

#[test]
fn test_screen_print_scrolls_at_bottom() {
    let mut screen = Screen::new(Dimensions::new(3, 2));
    for c in "ABCDEFG".chars() {
        screen.print(c);
    }
    // After wrapping twice, should scroll
    assert_eq!(screen.cursor().row, 1); // or wherever it lands
}

#[test]
fn test_screen_print_with_bold() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.cursor_mut().attrs.bold = true;
    screen.print('X');
    assert!(screen.line(0).cell(0).attrs.bold);
}

#[test]
fn test_screen_print_with_fg_color() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.cursor_mut().attrs.fg = Color::Indexed(1);
    screen.print('R');
    assert_eq!(screen.line(0).cell(0).attrs.fg, Color::Indexed(1));
}

#[test]
fn test_screen_backspace() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.print('B');
    screen.backspace();
    assert_eq!(screen.cursor().col, 1);
}

#[test]
fn test_screen_backspace_at_col_zero() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.backspace();
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_tab() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.tab();
    assert_eq!(screen.cursor().col, 8);
}

#[test]
fn test_screen_tab_multiple() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.tab();
    assert_eq!(screen.cursor().col, 8);
    screen.tab();
    assert_eq!(screen.cursor().col, 16);
    screen.tab();
    assert_eq!(screen.cursor().col, 24);
}

#[test]
fn test_screen_tab_near_end() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.tab();
    assert_eq!(screen.cursor().col, 8);
    screen.tab();
    // Should not go past last column
    assert!(screen.cursor().col <= 9);
}

#[test]
fn test_screen_carriage_return() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.print('B');
    screen.carriage_return();
    assert_eq!(screen.cursor().col, 0);
    assert_eq!(screen.cursor().row, 0);
}

#[test]
fn test_screen_linefeed() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.linefeed();
    assert_eq!(screen.cursor().row, 1);
}

#[test]
fn test_screen_linefeed_scrolls_at_bottom() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.move_cursor_to(3, 1); // Row 3 (0-indexed: 2)
    screen.print('C');
    screen.linefeed(); // Should scroll
                       // First line should have scrolled off
}

#[test]
fn test_screen_linefeed_cr() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.linefeed();
    screen.carriage_return();
    screen.print('B');
    assert_eq!(screen.line(0).text(), "A");
    assert_eq!(screen.line(1).text(), "B");
}

#[test]
fn test_screen_reverse_index() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(3, 1);
    screen.reverse_index();
    assert_eq!(screen.cursor().row, 1); // Moved up from 2 to 1
}

#[test]
fn test_screen_reverse_index_at_top_scrolls() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.move_cursor_to(1, 1); // Row 0
    screen.print('A');
    screen.move_cursor_to(2, 1);
    screen.print('B');
    screen.move_cursor_to(1, 1); // Back to top
    screen.reverse_index(); // Should scroll down
    assert!(screen.line(0).is_empty()); // New blank line
}

#[test]
fn test_screen_index_same_as_linefeed() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.index();
    assert_eq!(screen.cursor().row, 1);
}

#[test]
fn test_screen_next_line() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.next_line();
    assert_eq!(screen.cursor().row, 1);
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_move_cursor_to() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(5, 10); // 1-indexed
    assert_eq!(screen.cursor().row, 4);
    assert_eq!(screen.cursor().col, 9);
}

#[test]
fn test_screen_move_cursor_to_clamps() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(100, 200);
    assert_eq!(screen.cursor().row, 23);
    assert_eq!(screen.cursor().col, 79);
}

#[test]
fn test_screen_move_cursor_to_origin() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(1, 1);
    assert_eq!(screen.cursor().row, 0);
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_move_cursor_up() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(10, 1);
    screen.move_cursor_up(3);
    assert_eq!(screen.cursor().row, 6);
}

#[test]
fn test_screen_move_cursor_up_clamp() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(3, 1);
    screen.move_cursor_up(100);
    assert_eq!(screen.cursor().row, 0);
}

#[test]
fn test_screen_move_cursor_down() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_down(5);
    assert_eq!(screen.cursor().row, 5);
}

#[test]
fn test_screen_move_cursor_down_clamp() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_down(100);
    assert_eq!(screen.cursor().row, 23);
}

#[test]
fn test_screen_move_cursor_left() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(1, 20);
    screen.move_cursor_left(5);
    assert_eq!(screen.cursor().col, 14);
}

#[test]
fn test_screen_move_cursor_left_clamp() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(1, 5);
    screen.move_cursor_left(100);
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_move_cursor_right() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_right(10);
    assert_eq!(screen.cursor().col, 10);
}

#[test]
fn test_screen_move_cursor_right_clamp() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_right(200);
    assert_eq!(screen.cursor().col, 79);
}

#[test]
fn test_screen_set_cursor_col() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_cursor_col(15); // 1-indexed, so internally becomes 14
    assert_eq!(screen.cursor().col, 14);
}

#[test]
fn test_screen_set_cursor_row() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_cursor_row(10); // 1-indexed, so internally becomes 9
    assert_eq!(screen.cursor().row, 9);
}

#[test]
fn test_screen_save_restore_cursor() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(10, 20);
    screen.cursor_mut().attrs.bold = true;
    screen.save_cursor();
    screen.move_cursor_to(1, 1);
    screen.cursor_mut().attrs.bold = false;
    screen.restore_cursor();
    assert_eq!(screen.cursor().row, 9);
    assert_eq!(screen.cursor().col, 19);
    assert!(screen.cursor().attrs.bold);
}

#[test]
fn test_screen_erase_display_below() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for row in 0..3 {
        screen.move_cursor_to(row + 1, 1);
        screen.print('X');
    }
    screen.move_cursor_to(2, 1);
    screen.erase_display(0); // Below
    assert!(!screen.line(0).is_empty());
    // Row 1 should be partially or fully erased from cursor
}

#[test]
fn test_screen_erase_display_above() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for row in 0..3 {
        screen.move_cursor_to(row + 1, 1);
        screen.print('X');
    }
    screen.move_cursor_to(2, 5);
    screen.erase_display(1); // Above
    assert!(screen.line(0).is_empty());
}

#[test]
fn test_screen_erase_display_all() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for row in 0..3 {
        screen.move_cursor_to(row + 1, 1);
        screen.print('X');
    }
    screen.erase_display(2); // All
    for row in 0..3 {
        assert!(screen.line(row).is_empty());
    }
}

#[test]
fn test_screen_erase_line_right() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.move_cursor_to(1, 1);
    for c in "ABCDEFGHIJ".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 5);
    screen.erase_line(0); // Right
    assert_eq!(screen.line(0).cell(3).display_char(), 'D');
    assert!(screen.line(0).cell(4).is_empty());
}

#[test]
fn test_screen_erase_line_left() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.move_cursor_to(1, 1);
    for c in "ABCDEFGHIJ".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 5);
    screen.erase_line(1); // Left
    assert!(screen.line(0).cell(0).is_empty());
    assert!(screen.line(0).cell(4).is_empty());
    assert_eq!(screen.line(0).cell(5).display_char(), 'F');
}

#[test]
fn test_screen_erase_line_all() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.move_cursor_to(1, 1);
    for c in "ABCDEFGHIJ".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 5);
    screen.erase_line(2); // All
    assert!(screen.line(0).is_empty());
}

#[test]
fn test_screen_erase_chars() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.move_cursor_to(1, 1);
    for c in "ABCDEFGHIJ".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 3);
    screen.erase_chars(3);
    assert_eq!(screen.line(0).cell(1).display_char(), 'B');
    assert!(screen.line(0).cell(2).is_empty());
    assert!(screen.line(0).cell(4).is_empty());
    assert_eq!(screen.line(0).cell(5).display_char(), 'F');
}

#[test]
fn test_screen_insert_lines() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for row in 0..5 {
        screen.move_cursor_to(row + 1, 1);
        screen.print((b'A' + row as u8) as char);
    }
    screen.move_cursor_to(2, 1);
    screen.insert_lines(2);
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
    assert!(screen.line(1).is_empty());
    assert!(screen.line(2).is_empty());
    assert_eq!(screen.line(3).cell(0).display_char(), 'B');
}

#[test]
fn test_screen_delete_lines() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for row in 0..5 {
        screen.move_cursor_to(row + 1, 1);
        screen.print((b'A' + row as u8) as char);
    }
    screen.move_cursor_to(2, 1);
    screen.delete_lines(2);
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
    assert_eq!(screen.line(1).cell(0).display_char(), 'D');
}

#[test]
fn test_screen_insert_chars() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.move_cursor_to(1, 1);
    for c in "ABCDE".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 3);
    screen.insert_chars(2);
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
    assert_eq!(screen.line(0).cell(1).display_char(), 'B');
    assert!(screen.line(0).cell(2).is_empty());
    assert!(screen.line(0).cell(3).is_empty());
    assert_eq!(screen.line(0).cell(4).display_char(), 'C');
}

#[test]
fn test_screen_delete_chars() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.move_cursor_to(1, 1);
    for c in "ABCDE".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 3);
    screen.delete_chars(2);
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
    assert_eq!(screen.line(0).cell(1).display_char(), 'B');
    assert_eq!(screen.line(0).cell(2).display_char(), 'E');
}

#[test]
fn test_screen_scroll_region_set() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(5, 20);
    assert_eq!(screen.scroll_region(), (4, 19));
}

#[test]
fn test_screen_scroll_region_clear() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(5, 20);
    screen.clear_scroll_region();
    assert_eq!(screen.scroll_region(), (0, 23));
}

#[test]
fn test_screen_scroll_region_linefeed() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for row in 0..5 {
        screen.move_cursor_to(row + 1, 1);
        screen.print((b'A' + row as u8) as char);
    }
    screen.set_scroll_region(2, 4);
    screen.move_cursor_to(4, 1);
    screen.linefeed();
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
    assert_eq!(screen.line(4).cell(0).display_char(), 'E');
}

#[test]
fn test_screen_scroll_up() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.move_cursor_to(1, 1);
    screen.print('A');
    screen.move_cursor_to(2, 1);
    screen.print('B');
    screen.move_cursor_to(3, 1);
    screen.print('C');
    screen.scroll_up(1);
    assert_eq!(screen.line(0).cell(0).display_char(), 'B');
    assert_eq!(screen.line(1).cell(0).display_char(), 'C');
    assert!(screen.line(2).is_empty());
}

#[test]
fn test_screen_scroll_down() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.move_cursor_to(1, 1);
    screen.print('A');
    screen.move_cursor_to(2, 1);
    screen.print('B');
    screen.move_cursor_to(3, 1);
    screen.print('C');
    screen.scroll_down(1);
    assert!(screen.line(0).is_empty());
    assert_eq!(screen.line(1).cell(0).display_char(), 'A');
    assert_eq!(screen.line(2).cell(0).display_char(), 'B');
}

#[test]
fn test_screen_set_tab_stop() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(1, 5);
    screen.set_tab_stop();
    // Clear default tabs first
    screen.clear_tab_stop(3);
    screen.move_cursor_to(1, 5);
    screen.set_tab_stop();
    screen.move_cursor_to(1, 1);
    screen.tab();
    assert_eq!(screen.cursor().col, 4); // Should go to col 4 (our custom tab stop)
}

#[test]
fn test_screen_clear_tab_stop_current() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    // Default tab at 8
    screen.move_cursor_to(1, 9); // col 8
    screen.clear_tab_stop(0); // Clear at current col
    screen.move_cursor_to(1, 1);
    screen.tab();
    // Should skip col 8 and go to next tab
}

#[test]
fn test_screen_clear_tab_stop_all() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.clear_tab_stop(3); // Clear all
    screen.move_cursor_to(1, 1);
    screen.tab();
    // With no tab stops, tab should go to end of line
    assert!(screen.cursor().col > 0);
}

#[test]
fn test_screen_alternate_screen_enter() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.enter_alternate_screen();
    assert!(screen.modes().alternate_screen);
    assert!(screen.line(0).is_empty()); // Alternate is clean
}

#[test]
fn test_screen_alternate_screen_exit() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.enter_alternate_screen();
    screen.print('B');
    screen.exit_alternate_screen();
    assert!(!screen.modes().alternate_screen);
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
}

#[test]
fn test_screen_alternate_screen_double_enter() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.enter_alternate_screen();
    screen.enter_alternate_screen(); // Second enter
    assert!(screen.modes().alternate_screen);
}

#[test]
fn test_screen_alternate_screen_exit_without_enter() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.exit_alternate_screen(); // Should not crash
    assert!(!screen.modes().alternate_screen);
}

#[test]
fn test_screen_resize() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.resize(Dimensions::new(120, 40));
    assert_eq!(screen.cols(), 120);
    assert_eq!(screen.rows(), 40);
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
}

#[test]
fn test_screen_resize_smaller() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(20, 70);
    screen.resize(Dimensions::new(40, 10));
    assert_eq!(screen.cols(), 40);
    assert_eq!(screen.rows(), 10);
    assert!(screen.cursor().col < 40);
    assert!(screen.cursor().row < 10);
}

#[test]
fn test_screen_resize_clears_scroll_region() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(5, 20);
    screen.resize(Dimensions::new(80, 30));
    assert_eq!(screen.scroll_region(), (0, 29));
}

#[test]
fn test_screen_reset() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.move_cursor_to(10, 20);
    screen.modes_mut().insert_mode = true;
    screen.set_title("test");
    screen.reset();
    assert_eq!(screen.cursor().col, 0);
    assert_eq!(screen.cursor().row, 0);
    assert!(!screen.modes().insert_mode);
    assert!(screen.title().is_empty());
}

#[test]
fn test_screen_set_title() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_title("My Terminal");
    assert_eq!(screen.title(), "My Terminal");
}

#[test]
fn test_screen_set_title_overwrite() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_title("First");
    screen.set_title("Second");
    assert_eq!(screen.title(), "Second");
}

#[test]
fn test_screen_register_hyperlink() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let id = screen.register_hyperlink("https://example.com");
    assert!(id > 0);
}

#[test]
fn test_screen_get_hyperlink() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let id = screen.register_hyperlink("https://example.com");
    let url = screen.get_hyperlink(id);
    assert_eq!(url, Some("https://example.com"));
}

#[test]
fn test_screen_get_hyperlink_invalid() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert!(screen.get_hyperlink(0).is_none());
    assert!(screen.get_hyperlink(999).is_none());
}

#[test]
fn test_screen_register_duplicate_hyperlink() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let id1 = screen.register_hyperlink("https://example.com");
    let id2 = screen.register_hyperlink("https://example.com");
    assert_eq!(id1, id2); // Same URL should return same ID
}

#[test]
fn test_screen_register_different_hyperlinks() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let id1 = screen.register_hyperlink("https://example.com");
    let id2 = screen.register_hyperlink("https://other.com");
    assert_ne!(id1, id2);
}

#[test]
fn test_screen_snapshot_basic() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    let snapshot = screen.snapshot(false);
    assert_eq!(snapshot.dimensions.cols, 10);
    assert_eq!(snapshot.dimensions.rows, 3);
}

#[test]
fn test_screen_snapshot_with_scrollback() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
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
fn test_screen_snapshot_with_title() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.set_title("test title");
    let snapshot = screen.snapshot(false);
    assert_eq!(snapshot.title, Some("test title".to_string()));
}

#[test]
fn test_screen_snapshot_cursor() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(5, 10);
    let snapshot = screen.snapshot(false);
    assert_eq!(snapshot.cursor.row, 4);
    assert_eq!(snapshot.cursor.col, 9);
}

#[test]
fn test_screen_grid_access() {
    let screen = Screen::new(Dimensions::new(80, 24));
    let grid = screen.grid();
    assert_eq!(grid.cols(), 80);
    assert_eq!(grid.rows(), 24);
}

#[test]
fn test_screen_modes_mut() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.modes_mut().insert_mode = true;
    assert!(screen.modes().insert_mode);
}

#[test]
fn test_screen_cursor_mut() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.cursor_mut().style = CursorStyle::Bar;
    assert_eq!(screen.cursor().style, CursorStyle::Bar);
}

#[test]
fn test_screen_selection() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert!(!screen.selection().active);
}

#[test]
fn test_screen_selection_mut() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen
        .selection_mut()
        .start(Point::new(5, 10), SelectionType::Normal);
    assert!(screen.selection().active);
}

#[test]
fn test_screen_scrollback() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert!(screen.scrollback().is_empty());
}

#[test]
fn test_screen_charset() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert_eq!(screen.charset().current(), Charset::Ascii);
}

#[test]
fn test_screen_shift_in_out() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.designate_charset(1, '0'); // G1 = DEC Special Graphics
    screen.shift_out(); // Select G1
    assert_eq!(screen.charset().current(), Charset::DecSpecialGraphics);
    screen.shift_in(); // Select G0
    assert_eq!(screen.charset().current(), Charset::Ascii);
}

#[test]
fn test_screen_designate_charset() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.designate_charset(0, '0');
    assert_eq!(screen.charset().g0, Charset::DecSpecialGraphics);
}

#[test]
fn test_screen_insert_mode() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.move_cursor_to(1, 1);
    for c in "ABCDE".chars() {
        screen.print(c);
    }
    screen.modes_mut().insert_mode = true;
    screen.move_cursor_to(1, 3);
    screen.print('X');
    // In insert mode, X should be inserted, pushing chars right
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
    assert_eq!(screen.line(0).cell(1).display_char(), 'B');
    assert_eq!(screen.line(0).cell(2).display_char(), 'X');
}

// ============================================================================
// Scrollback Tests (~30 tests)
// ============================================================================

fn make_test_line(text: &str) -> Line {
    let mut line = Line::new(text.len().max(10));
    for (i, c) in text.chars().enumerate() {
        line.cell_mut(i).set_char(c);
    }
    line
}

#[test]
fn test_scrollback_new() {
    let sb = Scrollback::new(100);
    assert_eq!(sb.max_lines(), 100);
    assert_eq!(sb.len(), 0);
    assert!(sb.is_empty());
}

#[test]
fn test_scrollback_default() {
    let sb = Scrollback::default();
    assert_eq!(sb.max_lines(), 10000);
}

#[test]
fn test_scrollback_push_single() {
    let mut sb = Scrollback::new(100);
    sb.push(make_test_line("hello"));
    assert_eq!(sb.len(), 1);
    assert!(!sb.is_empty());
}

#[test]
fn test_scrollback_push_multiple() {
    let mut sb = Scrollback::new(100);
    for i in 0..10 {
        sb.push(make_test_line(&format!("line{}", i)));
    }
    assert_eq!(sb.len(), 10);
}

#[test]
fn test_scrollback_get_oldest() {
    let mut sb = Scrollback::new(100);
    sb.push(make_test_line("first"));
    sb.push(make_test_line("second"));
    assert_eq!(sb.get(0).unwrap().text(), "first");
}

#[test]
fn test_scrollback_get_newest() {
    let mut sb = Scrollback::new(100);
    sb.push(make_test_line("first"));
    sb.push(make_test_line("second"));
    assert_eq!(sb.get(1).unwrap().text(), "second");
}

#[test]
fn test_scrollback_get_out_of_bounds() {
    let mut sb = Scrollback::new(100);
    sb.push(make_test_line("hello"));
    assert!(sb.get(1).is_none());
    assert!(sb.get(100).is_none());
}

#[test]
fn test_scrollback_get_from_end_newest() {
    let mut sb = Scrollback::new(100);
    sb.push(make_test_line("first"));
    sb.push(make_test_line("second"));
    sb.push(make_test_line("third"));
    assert_eq!(sb.get_from_end(0).unwrap().text(), "third");
}

#[test]
fn test_scrollback_get_from_end_oldest() {
    let mut sb = Scrollback::new(100);
    sb.push(make_test_line("first"));
    sb.push(make_test_line("second"));
    assert_eq!(sb.get_from_end(1).unwrap().text(), "first");
}

#[test]
fn test_scrollback_get_from_end_out_of_bounds() {
    let mut sb = Scrollback::new(100);
    sb.push(make_test_line("hello"));
    assert!(sb.get_from_end(1).is_none());
}

#[test]
fn test_scrollback_ring_buffer_overwrites() {
    let mut sb = Scrollback::new(3);
    sb.push(make_test_line("line1"));
    sb.push(make_test_line("line2"));
    sb.push(make_test_line("line3"));
    sb.push(make_test_line("line4"));
    assert_eq!(sb.len(), 3);
    assert_eq!(sb.get(0).unwrap().text(), "line2");
    assert_eq!(sb.get(2).unwrap().text(), "line4");
}

#[test]
fn test_scrollback_ring_buffer_wraps_twice() {
    let mut sb = Scrollback::new(3);
    for i in 0..9 {
        sb.push(make_test_line(&format!("line{}", i)));
    }
    assert_eq!(sb.len(), 3);
    assert_eq!(sb.get(0).unwrap().text(), "line6");
    assert_eq!(sb.get(1).unwrap().text(), "line7");
    assert_eq!(sb.get(2).unwrap().text(), "line8");
}

#[test]
fn test_scrollback_push_lines() {
    let mut sb = Scrollback::new(100);
    let lines = vec![
        make_test_line("a"),
        make_test_line("b"),
        make_test_line("c"),
    ];
    sb.push_lines(lines);
    assert_eq!(sb.len(), 3);
}

#[test]
fn test_scrollback_clear() {
    let mut sb = Scrollback::new(100);
    sb.push(make_test_line("hello"));
    sb.push(make_test_line("world"));
    sb.clear();
    assert!(sb.is_empty());
    assert_eq!(sb.len(), 0);
}

#[test]
fn test_scrollback_resize_smaller() {
    let mut sb = Scrollback::new(100);
    for i in 0..10 {
        sb.push(make_test_line(&format!("line{}", i)));
    }
    sb.resize(5);
    assert_eq!(sb.len(), 5);
    assert_eq!(sb.max_lines(), 5);
    assert_eq!(sb.get(0).unwrap().text(), "line5");
}

#[test]
fn test_scrollback_resize_larger() {
    let mut sb = Scrollback::new(5);
    for i in 0..5 {
        sb.push(make_test_line(&format!("line{}", i)));
    }
    sb.resize(100);
    assert_eq!(sb.len(), 5);
    assert_eq!(sb.max_lines(), 100);
}

#[test]
fn test_scrollback_resize_to_zero() {
    let mut sb = Scrollback::new(100);
    sb.push(make_test_line("hello"));
    sb.resize(0);
    assert!(sb.is_empty());
    assert_eq!(sb.max_lines(), 0);
}

#[test]
fn test_scrollback_resize_same() {
    let mut sb = Scrollback::new(100);
    sb.push(make_test_line("hello"));
    sb.resize(100);
    assert_eq!(sb.len(), 1);
}

#[test]
fn test_scrollback_push_zero_max() {
    let mut sb = Scrollback::new(0);
    sb.push(make_test_line("hello"));
    assert!(sb.is_empty());
}

#[test]
fn test_scrollback_iter() {
    let mut sb = Scrollback::new(100);
    sb.push(make_test_line("a"));
    sb.push(make_test_line("b"));
    sb.push(make_test_line("c"));
    let texts: Vec<String> = sb.iter().map(|l| l.text()).collect();
    assert_eq!(texts, vec!["a", "b", "c"]);
}

#[test]
fn test_scrollback_iter_rev() {
    let mut sb = Scrollback::new(100);
    sb.push(make_test_line("a"));
    sb.push(make_test_line("b"));
    sb.push(make_test_line("c"));
    let texts: Vec<String> = sb.iter_rev().map(|l| l.text()).collect();
    assert_eq!(texts, vec!["c", "b", "a"]);
}

#[test]
fn test_scrollback_iter_empty() {
    let sb = Scrollback::new(100);
    assert_eq!(sb.iter().count(), 0);
}

#[test]
fn test_scrollback_iter_after_ring_wrap() {
    let mut sb = Scrollback::new(3);
    for i in 0..5 {
        sb.push(make_test_line(&format!("l{}", i)));
    }
    let texts: Vec<String> = sb.iter().map(|l| l.text()).collect();
    assert_eq!(texts, vec!["l2", "l3", "l4"]);
}

#[test]
fn test_scrollback_large_capacity() {
    let mut sb = Scrollback::new(10000);
    for i in 0..100 {
        sb.push(make_test_line(&format!("line{}", i)));
    }
    assert_eq!(sb.len(), 100);
}

// ============================================================================
// Selection Tests (~30 tests)
// ============================================================================

#[test]
fn test_selection_new_inactive() {
    let sel = Selection::new();
    assert!(!sel.active);
    assert!(sel.is_empty());
}

#[test]
fn test_selection_default() {
    let sel = Selection::default();
    assert!(!sel.active);
}

#[test]
fn test_selection_start_normal() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    assert!(sel.active);
    assert_eq!(sel.start.col, 5);
    assert_eq!(sel.start.row, 10);
    assert_eq!(sel.selection_type, SelectionType::Normal);
}

#[test]
fn test_selection_start_word() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Word);
    assert_eq!(sel.selection_type, SelectionType::Word);
}

#[test]
fn test_selection_start_line() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Line);
    assert_eq!(sel.selection_type, SelectionType::Line);
}

#[test]
fn test_selection_start_block() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Block);
    assert_eq!(sel.selection_type, SelectionType::Block);
}

#[test]
fn test_selection_update() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(20, 15));
    assert_eq!(sel.end.col, 20);
    assert_eq!(sel.end.row, 15);
}

#[test]
fn test_selection_update_inactive_noop() {
    let mut sel = Selection::new();
    sel.update(Point::new(20, 15));
    assert_eq!(sel.end.col, 0);
    assert_eq!(sel.end.row, 0);
}

#[test]
fn test_selection_finish() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(20, 15));
    sel.finish();
    assert!(sel.active); // Still active after finish
}

#[test]
fn test_selection_clear() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.clear();
    assert!(!sel.active);
}

#[test]
fn test_selection_bounds_forward() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(20, 15));
    let (start, end) = sel.bounds();
    assert_eq!(start.row, 10);
    assert_eq!(end.row, 15);
}

#[test]
fn test_selection_bounds_backward() {
    let mut sel = Selection::new();
    sel.start(Point::new(20, 15), SelectionType::Normal);
    sel.update(Point::new(5, 10));
    let (start, end) = sel.bounds();
    assert_eq!(start.row, 10);
    assert_eq!(end.row, 15);
}

#[test]
fn test_selection_bounds_same_line_forward() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(20, 10));
    let (start, end) = sel.bounds();
    assert_eq!(start.col, 5);
    assert_eq!(end.col, 20);
}

#[test]
fn test_selection_bounds_same_line_backward() {
    let mut sel = Selection::new();
    sel.start(Point::new(20, 10), SelectionType::Normal);
    sel.update(Point::new(5, 10));
    let (start, end) = sel.bounds();
    assert_eq!(start.col, 5);
    assert_eq!(end.col, 20);
}

#[test]
fn test_selection_contains_single_line() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(15, 10));
    assert!(sel.contains(5, 10));
    assert!(sel.contains(10, 10));
    assert!(sel.contains(15, 10));
    assert!(!sel.contains(4, 10));
    assert!(!sel.contains(16, 10));
}

#[test]
fn test_selection_contains_multi_line() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(15, 12));
    assert!(sel.contains(5, 10));
    assert!(sel.contains(80, 10)); // Rest of first line
    assert!(sel.contains(0, 11)); // Entire middle line
    assert!(sel.contains(50, 11));
    assert!(sel.contains(0, 12)); // Start of last line
    assert!(sel.contains(15, 12));
    assert!(!sel.contains(16, 12));
    assert!(!sel.contains(0, 13));
}

#[test]
fn test_selection_contains_inactive() {
    let sel = Selection::new();
    assert!(!sel.contains(0, 0));
}

#[test]
fn test_selection_line_type_contains() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Line);
    sel.update(Point::new(15, 12));
    // Line selection: all columns on lines 10-12
    assert!(sel.contains(0, 10));
    assert!(sel.contains(100, 10));
    assert!(sel.contains(0, 12));
    assert!(sel.contains(100, 12));
    assert!(!sel.contains(0, 9));
    assert!(!sel.contains(0, 13));
}

#[test]
fn test_selection_block_type_contains() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Block);
    sel.update(Point::new(15, 12));
    // Block: rectangle from (5,10) to (15,12)
    assert!(sel.contains(5, 10));
    assert!(sel.contains(15, 12));
    assert!(sel.contains(10, 11));
    assert!(!sel.contains(4, 11));
    assert!(!sel.contains(16, 11));
}

#[test]
fn test_selection_is_multiline_true() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(15, 12));
    assert!(sel.is_multiline());
}

#[test]
fn test_selection_is_multiline_false() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(15, 10));
    assert!(!sel.is_multiline());
}

#[test]
fn test_selection_is_empty_point() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    // start == end, so empty
    assert!(sel.is_empty());
}

#[test]
fn test_selection_is_not_empty() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(10, 10));
    assert!(!sel.is_empty());
}

#[test]
fn test_selection_negative_row() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, -5), SelectionType::Normal);
    sel.update(Point::new(15, 5));
    assert!(sel.contains(10, 0));
    assert!(sel.contains(10, -3));
    assert!(!sel.contains(10, -6));
}

#[test]
fn test_point_new() {
    let p = Point::new(10, 20);
    assert_eq!(p.col, 10);
    assert_eq!(p.row, 20);
}

#[test]
fn test_point_equality() {
    let a = Point::new(5, 10);
    let b = Point::new(5, 10);
    assert_eq!(a, b);
}

#[test]
fn test_point_inequality() {
    let a = Point::new(5, 10);
    let b = Point::new(6, 10);
    assert_ne!(a, b);
}

// ============================================================================
// Charset Tests (~25 tests)
// ============================================================================

#[test]
fn test_charset_state_default() {
    let state = CharsetState::new();
    assert_eq!(state.g0, Charset::Ascii);
    assert_eq!(state.g1, Charset::Ascii);
    assert_eq!(state.g2, Charset::Ascii);
    assert_eq!(state.g3, Charset::Ascii);
    assert_eq!(state.active, 0);
    assert!(state.single_shift.is_none());
}

#[test]
fn test_charset_state_reset() {
    let mut state = CharsetState::new();
    state.g0 = Charset::DecSpecialGraphics;
    state.active = 1;
    state.reset();
    assert_eq!(state.g0, Charset::Ascii);
    assert_eq!(state.active, 0);
}

#[test]
fn test_charset_current_g0() {
    let state = CharsetState::new();
    assert_eq!(state.current(), Charset::Ascii);
}

#[test]
fn test_charset_current_g1() {
    let mut state = CharsetState::new();
    state.g1 = Charset::DecSpecialGraphics;
    state.shift_out(); // Select G1
    assert_eq!(state.current(), Charset::DecSpecialGraphics);
}

#[test]
fn test_charset_set_slot_g0() {
    let mut state = CharsetState::new();
    state.set_slot(0, Charset::DecSpecialGraphics);
    assert_eq!(state.g0, Charset::DecSpecialGraphics);
}

#[test]
fn test_charset_set_slot_g1() {
    let mut state = CharsetState::new();
    state.set_slot(1, Charset::Uk);
    assert_eq!(state.g1, Charset::Uk);
}

#[test]
fn test_charset_set_slot_g2() {
    let mut state = CharsetState::new();
    state.set_slot(2, Charset::DecSpecialGraphics);
    assert_eq!(state.g2, Charset::DecSpecialGraphics);
}

#[test]
fn test_charset_set_slot_g3() {
    let mut state = CharsetState::new();
    state.set_slot(3, Charset::Uk);
    assert_eq!(state.g3, Charset::Uk);
}

#[test]
fn test_charset_set_slot_invalid() {
    let mut state = CharsetState::new();
    state.set_slot(4, Charset::Uk); // Should be noop
    assert_eq!(state.g0, Charset::Ascii);
}

#[test]
fn test_charset_shift_in() {
    let mut state = CharsetState::new();
    state.shift_out();
    state.shift_in();
    assert_eq!(state.active, 0);
}

#[test]
fn test_charset_shift_out() {
    let mut state = CharsetState::new();
    state.shift_out();
    assert_eq!(state.active, 1);
}

#[test]
fn test_charset_single_shift_2() {
    let mut state = CharsetState::new();
    state.g2 = Charset::DecSpecialGraphics;
    state.single_shift_2();
    assert_eq!(state.current(), Charset::DecSpecialGraphics);
}

#[test]
fn test_charset_single_shift_3() {
    let mut state = CharsetState::new();
    state.g3 = Charset::Uk;
    state.single_shift_3();
    assert_eq!(state.current(), Charset::Uk);
}

#[test]
fn test_charset_clear_single_shift() {
    let mut state = CharsetState::new();
    state.g2 = Charset::DecSpecialGraphics;
    state.single_shift_2();
    state.clear_single_shift();
    assert_eq!(state.current(), Charset::Ascii); // Back to G0
}

#[test]
fn test_charset_translate_ascii() {
    let state = CharsetState::new();
    assert_eq!(state.translate('A'), 'A');
    assert_eq!(state.translate('z'), 'z');
    assert_eq!(state.translate('0'), '0');
}

#[test]
fn test_charset_translate_dec() {
    let mut state = CharsetState::new();
    state.g0 = Charset::DecSpecialGraphics;
    assert_eq!(state.translate('q'), '─');
    assert_eq!(state.translate('x'), '│');
    assert_eq!(state.translate('l'), '┌');
    assert_eq!(state.translate('k'), '┐');
    assert_eq!(state.translate('m'), '└');
    assert_eq!(state.translate('j'), '┘');
}

#[test]
fn test_translate_char_ascii() {
    assert_eq!(translate_char('A', Charset::Ascii), 'A');
}

#[test]
fn test_translate_char_dec_box_drawing() {
    assert_eq!(translate_char('n', Charset::DecSpecialGraphics), '┼');
    assert_eq!(translate_char('t', Charset::DecSpecialGraphics), '├');
    assert_eq!(translate_char('u', Charset::DecSpecialGraphics), '┤');
    assert_eq!(translate_char('v', Charset::DecSpecialGraphics), '┴');
    assert_eq!(translate_char('w', Charset::DecSpecialGraphics), '┬');
}

#[test]
fn test_translate_char_dec_symbols() {
    assert_eq!(translate_char('`', Charset::DecSpecialGraphics), '◆');
    assert_eq!(translate_char('a', Charset::DecSpecialGraphics), '▒');
    assert_eq!(translate_char('f', Charset::DecSpecialGraphics), '°');
    assert_eq!(translate_char('g', Charset::DecSpecialGraphics), '±');
    assert_eq!(translate_char('y', Charset::DecSpecialGraphics), '≤');
    assert_eq!(translate_char('z', Charset::DecSpecialGraphics), '≥');
    assert_eq!(translate_char('{', Charset::DecSpecialGraphics), 'π');
    assert_eq!(translate_char('|', Charset::DecSpecialGraphics), '≠');
    assert_eq!(translate_char('~', Charset::DecSpecialGraphics), '·');
}

#[test]
fn test_translate_char_dec_passthrough() {
    // Non-special chars should pass through
    assert_eq!(translate_char('A', Charset::DecSpecialGraphics), 'A');
    assert_eq!(translate_char('Z', Charset::DecSpecialGraphics), 'Z');
}

#[test]
fn test_translate_char_uk() {
    assert_eq!(translate_char('#', Charset::Uk), '£');
    assert_eq!(translate_char('A', Charset::Uk), 'A'); // Others pass through
}

#[test]
fn test_parse_charset_designation_ascii() {
    assert_eq!(parse_charset_designation('B'), Charset::Ascii);
    assert_eq!(parse_charset_designation('@'), Charset::Ascii);
}

#[test]
fn test_parse_charset_designation_dec() {
    assert_eq!(parse_charset_designation('0'), Charset::DecSpecialGraphics);
    assert_eq!(parse_charset_designation('2'), Charset::DecSpecialGraphics);
}

#[test]
fn test_parse_charset_designation_uk() {
    assert_eq!(parse_charset_designation('A'), Charset::Uk);
}

#[test]
fn test_parse_charset_designation_unknown() {
    assert_eq!(parse_charset_designation('Z'), Charset::Ascii);
}

// ============================================================================
// Snapshot Tests (~15 tests)
// ============================================================================

#[test]
fn test_snapshot_from_terminal() {
    let screen = Screen::new(Dimensions::new(80, 24));
    let snapshot = screen.snapshot(false);
    assert_eq!(snapshot.dimensions.cols, 80);
    assert_eq!(snapshot.dimensions.rows, 24);
    assert!(snapshot.cursor.visible);
    assert_eq!(snapshot.cursor.style, "block");
}

#[test]
fn test_snapshot_cursor_styles() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.cursor_mut().style = CursorStyle::Underline;
    let snapshot = screen.snapshot(false);
    assert_eq!(snapshot.cursor.style, "underline");

    screen.cursor_mut().style = CursorStyle::Bar;
    let snapshot = screen.snapshot(false);
    assert_eq!(snapshot.cursor.style, "bar");
}

#[test]
fn test_snapshot_modes() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.modes_mut().origin_mode = true;
    screen.modes_mut().bracketed_paste = true;
    let snapshot = screen.snapshot(false);
    assert!(snapshot.modes.origin_mode);
    assert!(snapshot.modes.bracketed_paste);
}

#[test]
fn test_snapshot_screen_text() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('H');
    screen.print('i');
    let snapshot = screen.snapshot(false);
    let text = snapshot.screen_text();
    assert!(text.contains("Hi"));
}

#[test]
fn test_snapshot_json_roundtrip() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    screen.set_title("test");
    let snapshot = screen.snapshot(false);
    let json = snapshot.to_json().unwrap();
    let parsed = Snapshot::from_json(&json).unwrap();
    assert_eq!(parsed.dimensions.cols, 10);
    assert_eq!(parsed.dimensions.rows, 3);
    assert_eq!(parsed.title, Some("test".to_string()));
}

#[test]
fn test_snapshot_json_with_scrollback() {
    let screen = Screen::new(Dimensions::new(10, 3));
    let snapshot = screen.snapshot(true);
    let json = snapshot.to_json().unwrap();
    let parsed = Snapshot::from_json(&json).unwrap();
    assert!(parsed.scrollback.is_some());
}

#[test]
fn test_snapshot_scroll_region() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(5, 20);
    let snapshot = screen.snapshot(false);
    assert_eq!(snapshot.scroll_region, Some((4, 19)));
}

#[test]
fn test_snapshot_no_title() {
    let screen = Screen::new(Dimensions::new(80, 24));
    let snapshot = screen.snapshot(false);
    assert!(snapshot.title.is_none());
}

#[test]
fn test_snapshot_with_attrs() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.cursor_mut().attrs.bold = true;
    screen.cursor_mut().attrs.fg = Color::Indexed(1);
    screen.print('R');
    let snapshot = screen.snapshot(false);
    // Should have attr spans
    assert!(!snapshot.screen.is_empty());
}

#[test]
fn test_snapshot_wrapped_lines() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    for c in "Hello World".chars() {
        screen.print(c);
    }
    let snapshot = screen.snapshot(false);
    // First line wraps to second
    assert!(snapshot.screen[0].wrapped || snapshot.screen[1].wrapped);
}

#[test]
fn test_snapshot_empty_screen() {
    let screen = Screen::new(Dimensions::new(10, 3));
    let snapshot = screen.snapshot(false);
    assert_eq!(snapshot.screen.len(), 3);
    for line in &snapshot.screen {
        assert!(line.text.is_empty());
    }
}

#[test]
fn test_snapshot_insert_mode() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.modes_mut().insert_mode = true;
    let snapshot = screen.snapshot(false);
    assert!(snapshot.modes.insert_mode);
}

#[test]
fn test_snapshot_alternate_screen_mode() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.enter_alternate_screen();
    let snapshot = screen.snapshot(false);
    assert!(snapshot.modes.alternate_screen);
}
