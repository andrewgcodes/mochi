//! Comprehensive tests for terminal screen

use terminal_core::{Charset, Color, Dimensions, Screen};

// ============================================================
// Screen Creation Tests
// ============================================================

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
fn test_screen_new_not_alternate() {
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
    assert_eq!(screen.title(), "");
}

#[test]
fn test_screen_new_no_scroll_region() {
    let screen = Screen::new(Dimensions::new(80, 24));
    let (top, bottom) = screen.scroll_region();
    assert_eq!(top, 0);
    assert_eq!(bottom, 23);
}

#[test]
fn test_screen_new_scrollback_empty() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert!(screen.scrollback().is_empty());
}

#[test]
fn test_screen_new_selection_inactive() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert!(!screen.selection().active);
}

#[test]
fn test_screen_dimensions_method() {
    let screen = Screen::new(Dimensions::new(80, 24));
    let dims = screen.dimensions();
    assert_eq!(dims.cols, 80);
    assert_eq!(dims.rows, 24);
}

#[test]
fn test_screen_small_dimensions() {
    let screen = Screen::new(Dimensions::new(1, 1));
    assert_eq!(screen.cols(), 1);
    assert_eq!(screen.rows(), 1);
}

// ============================================================
// Print Tests
// ============================================================

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
    screen.print('H');
    screen.print('i');
    assert_eq!(screen.line(0).cell(0).display_char(), 'H');
    assert_eq!(screen.line(0).cell(1).display_char(), 'i');
    assert_eq!(screen.cursor().col, 2);
}

#[test]
fn test_screen_print_wide_char() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('中');
    assert_eq!(screen.line(0).cell(0).display_char(), '中');
    assert!(screen.line(0).cell(1).is_continuation());
    assert_eq!(screen.cursor().col, 2);
}

#[test]
fn test_screen_print_with_attrs() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.cursor_mut().attrs.bold = true;
    screen.print('B');
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
fn test_screen_print_with_hyperlink() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let id = screen.register_hyperlink("https://example.com");
    screen.cursor_mut().hyperlink_id = id;
    screen.print('L');
    assert_eq!(screen.line(0).cell(0).hyperlink_id, id);
}

// ============================================================
// Wrap Tests
// ============================================================

#[test]
fn test_screen_wrap_at_end_of_line() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    for c in "Hello World".chars() {
        screen.print(c);
    }
    assert_eq!(screen.line(0).text(), "Hello");
    assert_eq!(screen.line(1).text(), " Worl");
    assert_eq!(screen.line(2).text(), "d");
}

#[test]
fn test_screen_wrap_marks_line_wrapped() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    for c in "HelloW".chars() {
        screen.print(c);
    }
    assert!(screen.line(0).wrapped);
}

#[test]
fn test_screen_no_wrap_when_disabled() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    screen.modes_mut().auto_wrap = false;
    for c in "Hello World".chars() {
        screen.print(c);
    }
    // Cursor should stay at last column, chars overwrite
    assert_eq!(screen.cursor().col, 4);
    assert_eq!(screen.cursor().row, 0);
}

#[test]
fn test_screen_wrap_scrolls_at_bottom() {
    let mut screen = Screen::new(Dimensions::new(3, 2));
    // Fill first line
    for c in "ABC".chars() {
        screen.print(c);
    }
    // Wrap to second line
    for c in "DEF".chars() {
        screen.print(c);
    }
    // Wrap should scroll
    screen.print('G');
    assert_eq!(screen.line(0).text(), "DEF");
}

// ============================================================
// Backspace Tests
// ============================================================

#[test]
fn test_screen_backspace() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.print('B');
    screen.backspace();
    assert_eq!(screen.cursor().col, 1);
}

#[test]
fn test_screen_backspace_at_column_zero() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.backspace();
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_backspace_clears_pending_wrap() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    // Fill the line to trigger pending wrap
    for c in "ABCDE".chars() {
        screen.print(c);
    }
    // Cursor should be at col 4 with pending_wrap
    screen.backspace();
    assert_eq!(screen.cursor().col, 3);
    assert!(!screen.cursor().pending_wrap);
}

// ============================================================
// Tab Tests
// ============================================================

#[test]
fn test_screen_tab_from_start() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.tab();
    assert_eq!(screen.cursor().col, 8);
}

#[test]
fn test_screen_tab_from_middle() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.tab();
    assert_eq!(screen.cursor().col, 8);
}

#[test]
fn test_screen_tab_double() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.tab();
    screen.tab();
    assert_eq!(screen.cursor().col, 16);
}

#[test]
fn test_screen_tab_near_end() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.tab();
    assert_eq!(screen.cursor().col, 8);
    screen.tab();
    // Should clamp to last column
    assert_eq!(screen.cursor().col, 9);
}

#[test]
fn test_screen_set_tab_stop() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(1, 5); // col 4
    screen.set_tab_stop();
    screen.move_cursor_to(1, 1); // back to col 0
    screen.tab();
    assert_eq!(screen.cursor().col, 4);
}

#[test]
fn test_screen_clear_tab_stop_current() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    // Move to tab stop at col 8
    screen.move_cursor_to(1, 9); // col 8
    screen.clear_tab_stop(0); // Clear at current position
    screen.move_cursor_to(1, 1); // back to col 0
    screen.tab();
    // Should skip col 8 and go to next stop at 16
    assert_eq!(screen.cursor().col, 16);
}

#[test]
fn test_screen_clear_all_tab_stops() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.clear_tab_stop(3); // Clear all
    screen.tab();
    // With no tab stops, should go to end of line
    assert_eq!(screen.cursor().col, 79);
}

// ============================================================
// Carriage Return / Linefeed Tests
// ============================================================

#[test]
fn test_screen_carriage_return() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.print('B');
    screen.carriage_return();
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_linefeed_basic() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.linefeed();
    assert_eq!(screen.cursor().row, 1);
}

#[test]
fn test_screen_linefeed_at_bottom_scrolls() {
    let mut screen = Screen::new(Dimensions::new(80, 3));
    screen.print('A');
    screen.linefeed();
    screen.carriage_return();
    screen.print('B');
    screen.linefeed();
    screen.carriage_return();
    screen.print('C');
    screen.linefeed(); // Should scroll
    screen.carriage_return();
    screen.print('D');

    assert_eq!(screen.line(0).cell(0).display_char(), 'B');
    assert_eq!(screen.line(1).cell(0).display_char(), 'C');
    assert_eq!(screen.line(2).cell(0).display_char(), 'D');
}

#[test]
fn test_screen_linefeed_mode_also_does_cr() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.modes_mut().linefeed_mode = true;
    screen.print('A');
    screen.print('B');
    screen.linefeed();
    assert_eq!(screen.cursor().col, 0);
    assert_eq!(screen.cursor().row, 1);
}

#[test]
fn test_screen_linefeed_normal_preserves_col() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.print('B');
    screen.linefeed();
    assert_eq!(screen.cursor().col, 2);
    assert_eq!(screen.cursor().row, 1);
}

// ============================================================
// Reverse Index Tests
// ============================================================

#[test]
fn test_screen_reverse_index_basic() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(5, 1);
    screen.reverse_index();
    assert_eq!(screen.cursor().row, 3);
}

#[test]
fn test_screen_reverse_index_at_top_scrolls_down() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.move_cursor_to(1, 1);
    screen.print('A');
    screen.move_cursor_to(2, 1);
    screen.print('B');
    screen.move_cursor_to(1, 1);
    screen.reverse_index(); // Should scroll down
    assert!(screen.line(0).cell(0).is_empty());
    assert_eq!(screen.line(1).cell(0).display_char(), 'A');
}

// ============================================================
// Index / Next Line Tests
// ============================================================

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

// ============================================================
// Cursor Movement Tests
// ============================================================

#[test]
fn test_screen_move_cursor_to() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(5, 10); // 1-indexed
    assert_eq!(screen.cursor().row, 4);
    assert_eq!(screen.cursor().col, 9);
}

#[test]
fn test_screen_move_cursor_to_origin() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(1, 1);
    assert_eq!(screen.cursor().row, 0);
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_move_cursor_to_clamp() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(100, 200);
    assert_eq!(screen.cursor().row, 23);
    assert_eq!(screen.cursor().col, 79);
}

#[test]
fn test_screen_move_cursor_to_zero() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(0, 0);
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
    screen.move_cursor_to(5, 1);
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
    screen.move_cursor_to(1, 11);
    screen.move_cursor_left(3);
    assert_eq!(screen.cursor().col, 7);
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
    screen.move_cursor_right(100);
    assert_eq!(screen.cursor().col, 79);
}

#[test]
fn test_screen_set_cursor_col() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_cursor_col(10); // 1-indexed
    assert_eq!(screen.cursor().col, 9);
}

#[test]
fn test_screen_set_cursor_row() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_cursor_row(10); // 1-indexed
    assert_eq!(screen.cursor().row, 9);
}

// ============================================================
// Origin Mode Cursor Tests
// ============================================================

#[test]
fn test_screen_origin_mode_cursor_to() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(5, 20); // rows 5-20
    screen.modes_mut().origin_mode = true;
    screen.move_cursor_to(1, 1); // Should be relative to scroll region
    assert_eq!(screen.cursor().row, 4); // scroll_top = 4 (0-indexed)
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_origin_mode_cursor_clamp() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(5, 20);
    screen.modes_mut().origin_mode = true;
    screen.move_cursor_to(100, 1);
    assert_eq!(screen.cursor().row, 19); // scroll_bottom
}

// ============================================================
// Save/Restore Cursor Tests
// ============================================================

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
fn test_screen_save_cursor_alternate_screen() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(10, 20);
    screen.save_cursor();

    screen.enter_alternate_screen();
    screen.move_cursor_to(5, 5);
    screen.save_cursor();

    screen.move_cursor_to(1, 1);
    screen.restore_cursor();

    // Should restore alternate screen's saved cursor
    assert_eq!(screen.cursor().row, 4);
    assert_eq!(screen.cursor().col, 4);
}

// ============================================================
// Erase Display Tests
// ============================================================

#[test]
fn test_screen_erase_display_below() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for row in 0..3 {
        screen.move_cursor_to(row + 1, 1);
        for c in "XXXXXXXXXX".chars() {
            screen.print(c);
        }
    }
    screen.move_cursor_to(2, 5);
    screen.erase_display(0);

    assert_eq!(screen.line(0).text(), "XXXXXXXXXX");
    assert_eq!(screen.line(1).text(), "XXXX");
    assert!(screen.line(2).is_empty());
}

#[test]
fn test_screen_erase_display_above() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for row in 0..3 {
        screen.move_cursor_to(row + 1, 1);
        for c in "XXXXXXXXXX".chars() {
            screen.print(c);
        }
    }
    screen.move_cursor_to(2, 5);
    screen.erase_display(1);

    assert!(screen.line(0).is_empty());
    // Row 1 should be cleared up to col 4 (inclusive)
    assert_eq!(screen.line(2).text(), "XXXXXXXXXX");
}

#[test]
fn test_screen_erase_display_all() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for row in 0..3 {
        screen.move_cursor_to(row + 1, 1);
        screen.print('X');
    }
    screen.erase_display(2);
    for row in 0..3 {
        assert!(screen.line(row).is_empty());
    }
}

#[test]
fn test_screen_erase_display_invalid_mode() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    screen.erase_display(99); // Invalid mode, should do nothing
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
}

// ============================================================
// Erase Line Tests
// ============================================================

#[test]
fn test_screen_erase_line_to_end() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for c in "XXXXXXXXXX".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 5);
    screen.erase_line(0);
    assert_eq!(screen.line(0).cell(3).display_char(), 'X');
    assert!(screen.line(0).cell(4).is_empty());
}

#[test]
fn test_screen_erase_line_to_start() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for c in "XXXXXXXXXX".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 5);
    screen.erase_line(1);
    assert!(screen.line(0).cell(0).is_empty());
    assert!(screen.line(0).cell(4).is_empty());
    assert_eq!(screen.line(0).cell(5).display_char(), 'X');
}

#[test]
fn test_screen_erase_line_entire() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for c in "XXXXXXXXXX".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 5);
    screen.erase_line(2);
    assert!(screen.line(0).is_empty());
}

// ============================================================
// Erase Chars Tests
// ============================================================

#[test]
fn test_screen_erase_chars() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for c in "ABCDEFGHIJ".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 4);
    screen.erase_chars(3);
    assert_eq!(screen.line(0).cell(2).display_char(), 'C');
    assert!(screen.line(0).cell(3).is_empty());
    assert!(screen.line(0).cell(5).is_empty());
    assert_eq!(screen.line(0).cell(6).display_char(), 'G');
}

// ============================================================
// Insert/Delete Lines Tests
// ============================================================

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
    assert!(screen.line(1).cell(0).is_empty());
    assert!(screen.line(2).cell(0).is_empty());
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
    assert_eq!(screen.line(2).cell(0).display_char(), 'E');
    assert!(screen.line(3).cell(0).is_empty());
}

// ============================================================
// Insert/Delete Chars Tests
// ============================================================

#[test]
fn test_screen_insert_chars() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for c in "ABCDE".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 3);
    screen.insert_chars(2);
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
    assert_eq!(screen.line(0).cell(1).display_char(), 'B');
    assert!(screen.line(0).cell(2).is_empty());
}

#[test]
fn test_screen_delete_chars() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for c in "ABCDE".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 3);
    screen.delete_chars(2);
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
    assert_eq!(screen.line(0).cell(1).display_char(), 'B');
    assert_eq!(screen.line(0).cell(2).display_char(), 'E');
}

// ============================================================
// Scroll Region Tests
// ============================================================

#[test]
fn test_screen_set_scroll_region() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(5, 20);
    let (top, bottom) = screen.scroll_region();
    assert_eq!(top, 4);
    assert_eq!(bottom, 19);
}

#[test]
fn test_screen_set_scroll_region_moves_cursor_home() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(10, 20);
    screen.set_scroll_region(5, 20);
    assert_eq!(screen.cursor().col, 0);
    assert_eq!(screen.cursor().row, 0);
}

#[test]
fn test_screen_set_scroll_region_invalid_clears() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(20, 5); // top > bottom
    let (top, bottom) = screen.scroll_region();
    assert_eq!(top, 0);
    assert_eq!(bottom, 23);
}

#[test]
fn test_screen_clear_scroll_region() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(5, 20);
    screen.clear_scroll_region();
    let (top, bottom) = screen.scroll_region();
    assert_eq!(top, 0);
    assert_eq!(bottom, 23);
}

#[test]
fn test_screen_scroll_within_region() {
    let mut screen = Screen::new(Dimensions::new(10, 5));
    for row in 0..5 {
        screen.move_cursor_to(row + 1, 1);
        screen.print((b'A' + row as u8) as char);
    }
    screen.set_scroll_region(2, 4);
    screen.move_cursor_to(4, 1);
    screen.linefeed();
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
    assert_eq!(screen.line(1).cell(0).display_char(), 'C');
    assert_eq!(screen.line(2).cell(0).display_char(), 'D');
    assert!(screen.line(3).cell(0).is_empty());
    assert_eq!(screen.line(4).cell(0).display_char(), 'E');
}

// ============================================================
// Scroll Up/Down Tests
// ============================================================

#[test]
fn test_screen_scroll_up() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for row in 0..3 {
        screen.move_cursor_to(row + 1, 1);
        screen.print((b'A' + row as u8) as char);
    }
    screen.scroll_up(1);
    assert_eq!(screen.line(0).cell(0).display_char(), 'B');
    assert_eq!(screen.line(1).cell(0).display_char(), 'C');
    assert!(screen.line(2).cell(0).is_empty());
}

#[test]
fn test_screen_scroll_up_adds_to_scrollback() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    screen.scroll_up(1);
    assert!(!screen.scrollback().is_empty());
}

#[test]
fn test_screen_scroll_down() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for row in 0..3 {
        screen.move_cursor_to(row + 1, 1);
        screen.print((b'A' + row as u8) as char);
    }
    screen.scroll_down(1);
    assert!(screen.line(0).cell(0).is_empty());
    assert_eq!(screen.line(1).cell(0).display_char(), 'A');
    assert_eq!(screen.line(2).cell(0).display_char(), 'B');
}

// ============================================================
// Alternate Screen Tests
// ============================================================

#[test]
fn test_screen_enter_alternate() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.enter_alternate_screen();
    assert!(screen.modes().alternate_screen);
    assert!(screen.line(0).cell(0).is_empty());
}

#[test]
fn test_screen_exit_alternate() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.enter_alternate_screen();
    screen.print('B');
    screen.exit_alternate_screen();
    assert!(!screen.modes().alternate_screen);
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
}

#[test]
fn test_screen_alternate_no_scrollback() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.enter_alternate_screen();
    for row in 0..3 {
        screen.move_cursor_to(row + 1, 1);
        screen.print('X');
    }
    screen.scroll_up(1);
    // Alternate screen does NOT add to scrollback
    assert!(screen.scrollback().is_empty());
}

#[test]
fn test_screen_enter_alternate_resets_cursor() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(10, 20);
    screen.enter_alternate_screen();
    assert_eq!(screen.cursor().col, 0);
    assert_eq!(screen.cursor().row, 0);
}

// ============================================================
// Resize Tests
// ============================================================

#[test]
fn test_screen_resize_larger() {
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
    screen.print('A');
    screen.resize(Dimensions::new(40, 12));
    assert_eq!(screen.cols(), 40);
    assert_eq!(screen.rows(), 12);
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
}

#[test]
fn test_screen_resize_clamps_cursor() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(24, 80);
    screen.resize(Dimensions::new(40, 12));
    assert!(screen.cursor().col < 40);
    assert!(screen.cursor().row < 12);
}

#[test]
fn test_screen_resize_clears_scroll_region() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(5, 20);
    screen.resize(Dimensions::new(80, 24));
    let (top, bottom) = screen.scroll_region();
    assert_eq!(top, 0);
    assert_eq!(bottom, 23);
}

// ============================================================
// Reset Tests
// ============================================================

#[test]
fn test_screen_reset() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.move_cursor_to(10, 20);
    screen.modes_mut().alternate_screen = true;
    screen.set_title("Test");
    screen.reset();

    assert_eq!(screen.cursor().col, 0);
    assert_eq!(screen.cursor().row, 0);
    assert!(screen.line(0).cell(0).is_empty());
    assert_eq!(screen.title(), "");
}

// ============================================================
// Title Tests
// ============================================================

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
fn test_screen_set_title_truncates_long() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let long_title: String = "X".repeat(5000);
    screen.set_title(&long_title);
    assert!(screen.title().len() <= 4096);
}

// ============================================================
// Hyperlink Tests
// ============================================================

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
    assert_eq!(screen.get_hyperlink(id), Some("https://example.com"));
}

#[test]
fn test_screen_get_hyperlink_zero() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert_eq!(screen.get_hyperlink(0), None);
}

#[test]
fn test_screen_register_duplicate_hyperlink() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let id1 = screen.register_hyperlink("https://example.com");
    let id2 = screen.register_hyperlink("https://example.com");
    assert_eq!(id1, id2);
}

#[test]
fn test_screen_register_different_hyperlinks() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let id1 = screen.register_hyperlink("https://example.com");
    let id2 = screen.register_hyperlink("https://other.com");
    assert_ne!(id1, id2);
}

// ============================================================
// Snapshot Tests
// ============================================================

#[test]
fn test_screen_snapshot_basic() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('H');
    screen.print('i');
    let snap = screen.snapshot(false);
    assert_eq!(snap.dimensions.cols, 10);
    assert_eq!(snap.dimensions.rows, 3);
    assert!(snap.screen[0].text.contains("Hi"));
}

#[test]
fn test_screen_snapshot_with_scrollback() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    screen.scroll_up(1);
    let snap = screen.snapshot(true);
    assert!(snap.scrollback.is_some());
}

#[test]
fn test_screen_snapshot_without_scrollback() {
    let screen = Screen::new(Dimensions::new(10, 3));
    let snap = screen.snapshot(false);
    assert!(snap.scrollback.is_none());
}

#[test]
fn test_screen_snapshot_json_roundtrip() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('X');
    let snap = screen.snapshot(false);
    let json = snap.to_json().unwrap();
    let parsed = terminal_core::Snapshot::from_json(&json).unwrap();
    assert_eq!(parsed.dimensions.cols, 10);
}

// ============================================================
// Charset Tests
// ============================================================

#[test]
fn test_screen_charset_default_ascii() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert_eq!(screen.charset().current(), Charset::Ascii);
}

#[test]
fn test_screen_designate_charset_dec() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.designate_charset(0, '0');
    assert_eq!(screen.charset().g0, Charset::DecSpecialGraphics);
}

#[test]
fn test_screen_shift_out_shift_in() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.designate_charset(1, '0');
    screen.shift_out();
    assert_eq!(screen.charset().active, 1);
    screen.shift_in();
    assert_eq!(screen.charset().active, 0);
}

#[test]
fn test_screen_print_through_charset() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.designate_charset(0, '0'); // DEC Special Graphics
    screen.print('q'); // Should translate to '─'
    assert_eq!(screen.line(0).cell(0).display_char(), '─');
}

// ============================================================
// Insert Mode Tests
// ============================================================

#[test]
fn test_screen_insert_mode() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for c in "ABCDE".chars() {
        screen.print(c);
    }
    screen.modes_mut().insert_mode = true;
    screen.move_cursor_to(1, 3);
    screen.print('X');
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
    assert_eq!(screen.line(0).cell(1).display_char(), 'B');
    assert_eq!(screen.line(0).cell(2).display_char(), 'X');
}

// ============================================================
// Line Access Tests
// ============================================================

#[test]
fn test_screen_line_access() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    let line = screen.line(0);
    assert_eq!(line.cell(0).display_char(), 'A');
}
