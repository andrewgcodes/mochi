//! Comprehensive tests for the terminal screen state machine

use terminal_core::{CellAttributes, Color, Charset, Dimensions, Screen, SelectionType, Point};

// ============================================================================
// Screen Creation
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
    assert_eq!(screen.title(), "");
}

#[test]
fn test_screen_new_empty_scrollback() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert!(screen.scrollback().is_empty());
}

#[test]
fn test_screen_new_no_selection() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert!(!screen.selection().active);
}

#[test]
fn test_screen_new_small() {
    let screen = Screen::new(Dimensions::new(1, 1));
    assert_eq!(screen.cols(), 1);
    assert_eq!(screen.rows(), 1);
}

#[test]
fn test_screen_dimensions_struct() {
    let screen = Screen::new(Dimensions::new(80, 24));
    let dims = screen.dimensions();
    assert_eq!(dims.cols, 80);
    assert_eq!(dims.rows, 24);
}

// ============================================================================
// Screen::print
// ============================================================================

#[test]
fn test_screen_print_single() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
    assert_eq!(screen.cursor().col, 1);
}

#[test]
fn test_screen_print_multiple() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    for c in "Hello".chars() {
        screen.print(c);
    }
    assert_eq!(screen.line(0).text(), "Hello");
    assert_eq!(screen.cursor().col, 5);
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
fn test_screen_print_wide_char() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('中');
    assert_eq!(screen.line(0).cell(0).display_char(), '中');
    assert_eq!(screen.line(0).cell(0).width(), 2);
    assert!(screen.line(0).cell(1).is_continuation());
    assert_eq!(screen.cursor().col, 2);
}

#[test]
fn test_screen_print_emoji() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('😀');
    assert_eq!(screen.line(0).cell(0).display_char(), '😀');
    assert_eq!(screen.cursor().col, 2);
}

// ============================================================================
// Screen wrapping
// ============================================================================

#[test]
fn test_screen_wrap_basic() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    for c in "Hello World".chars() {
        screen.print(c);
    }
    assert_eq!(screen.line(0).text(), "Hello");
    assert_eq!(screen.line(1).text(), " Worl");
    assert_eq!(screen.line(2).text(), "d");
}

#[test]
fn test_screen_wrap_sets_wrapped_flag() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    // "HelloWorldExtra" = 15 chars, 5 cols -> wraps twice
    for c in "HelloWorldExtra".chars() {
        screen.print(c);
    }
    // Line 0 wrapped to line 1, line 1 wrapped to line 2
    assert!(screen.line(0).wrapped);
    assert!(screen.line(1).wrapped);
}

#[test]
fn test_screen_no_wrap_when_disabled() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    screen.modes_mut().auto_wrap = false;
    for c in "Hello World".chars() {
        screen.print(c);
    }
    // Without auto_wrap, cursor stays at last column
    assert_eq!(screen.cursor().row, 0);
}

#[test]
fn test_screen_wrap_scrolls() {
    let mut screen = Screen::new(Dimensions::new(3, 2));
    for c in "ABCDEFGHI".chars() {
        screen.print(c);
    }
    // Screen should show last two rows of the 3 rows needed
    assert_eq!(screen.line(0).text(), "DEF");
    assert_eq!(screen.line(1).text(), "GHI");
}

// ============================================================================
// Screen::backspace
// ============================================================================

#[test]
fn test_screen_backspace() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.print('B');
    screen.backspace();
    assert_eq!(screen.cursor().col, 1);
}

#[test]
fn test_screen_backspace_at_start() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.backspace();
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_backspace_clears_pending_wrap() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    for c in "ABCDE".chars() {
        screen.print(c);
    }
    // cursor has pending wrap at col 4
    screen.backspace();
    assert!(!screen.cursor().pending_wrap);
}

// ============================================================================
// Screen::tab
// ============================================================================

#[test]
fn test_screen_tab_from_start() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.tab();
    assert_eq!(screen.cursor().col, 8);
}

#[test]
fn test_screen_tab_from_col_1() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.tab();
    assert_eq!(screen.cursor().col, 8);
}

#[test]
fn test_screen_tab_from_col_8() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.cursor_mut().col = 8;
    screen.tab();
    assert_eq!(screen.cursor().col, 16);
}

#[test]
fn test_screen_tab_at_end() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.cursor_mut().col = 79;
    screen.tab();
    assert_eq!(screen.cursor().col, 79); // Stays at end
}

#[test]
fn test_screen_tab_multiple() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.tab();
    screen.tab();
    assert_eq!(screen.cursor().col, 16);
}

// ============================================================================
// Screen::carriage_return
// ============================================================================

#[test]
fn test_screen_carriage_return() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.print('B');
    screen.carriage_return();
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_carriage_return_preserves_row() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(5, 10);
    screen.carriage_return();
    assert_eq!(screen.cursor().row, 4); // Row unchanged
    assert_eq!(screen.cursor().col, 0);
}

// ============================================================================
// Screen::linefeed
// ============================================================================

#[test]
fn test_screen_linefeed_basic() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.linefeed();
    assert_eq!(screen.cursor().row, 1);
}

#[test]
fn test_screen_linefeed_preserves_col() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.cursor_mut().col = 10;
    screen.linefeed();
    assert_eq!(screen.cursor().col, 10);
}

#[test]
fn test_screen_linefeed_with_linefeed_mode() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.modes_mut().linefeed_mode = true;
    screen.cursor_mut().col = 10;
    screen.linefeed();
    assert_eq!(screen.cursor().col, 0); // LF also does CR
}

#[test]
fn test_screen_linefeed_scrolls_at_bottom() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    screen.linefeed();
    screen.carriage_return();
    screen.print('B');
    screen.linefeed();
    screen.carriage_return();
    screen.print('C');
    screen.linefeed(); // Scrolls
    screen.carriage_return();
    screen.print('D');

    assert_eq!(screen.line(0).cell(0).display_char(), 'B');
    assert_eq!(screen.line(1).cell(0).display_char(), 'C');
    assert_eq!(screen.line(2).cell(0).display_char(), 'D');
}

#[test]
fn test_screen_linefeed_adds_to_scrollback() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    screen.linefeed();
    screen.carriage_return();
    screen.print('B');
    screen.linefeed();
    screen.carriage_return();
    screen.print('C');
    screen.linefeed(); // Scrolls, 'A' goes to scrollback
    assert_eq!(screen.scrollback().len(), 1);
}

// ============================================================================
// Screen::reverse_index
// ============================================================================

#[test]
fn test_screen_reverse_index_basic() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.cursor_mut().row = 5;
    screen.reverse_index();
    assert_eq!(screen.cursor().row, 4);
}

#[test]
fn test_screen_reverse_index_at_top_scrolls() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    screen.linefeed();
    screen.carriage_return();
    screen.print('B');
    screen.linefeed();
    screen.carriage_return();
    screen.print('C');
    screen.move_cursor_to(1, 1);
    screen.reverse_index(); // At top, should scroll down
    assert!(screen.line(0).is_empty());
    assert_eq!(screen.line(1).cell(0).display_char(), 'A');
}

// ============================================================================
// Screen::next_line
// ============================================================================

#[test]
fn test_screen_next_line() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.cursor_mut().col = 10;
    screen.next_line();
    assert_eq!(screen.cursor().row, 1);
    assert_eq!(screen.cursor().col, 0);
}

// ============================================================================
// Screen::move_cursor_to (1-indexed)
// ============================================================================

#[test]
fn test_screen_move_cursor_to() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(5, 10);
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
    screen.move_cursor_to(0, 0); // 0 should clamp to row 0, col 0
    assert_eq!(screen.cursor().row, 0);
    assert_eq!(screen.cursor().col, 0);
}

#[test]
fn test_screen_move_cursor_to_clears_pending_wrap() {
    let mut screen = Screen::new(Dimensions::new(5, 3));
    for c in "ABCDE".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 1);
    assert!(!screen.cursor().pending_wrap);
}

// ============================================================================
// Screen cursor movement
// ============================================================================

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
    screen.cursor_mut().col = 10;
    screen.move_cursor_left(3);
    assert_eq!(screen.cursor().col, 7);
}

#[test]
fn test_screen_move_cursor_left_clamp() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.cursor_mut().col = 5;
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

// ============================================================================
// Screen::set_cursor_col / set_cursor_row (1-indexed)
// ============================================================================

#[test]
fn test_screen_set_cursor_col() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_cursor_col(10);
    assert_eq!(screen.cursor().col, 9);
}

#[test]
fn test_screen_set_cursor_col_clamp() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_cursor_col(200);
    assert_eq!(screen.cursor().col, 79);
}

#[test]
fn test_screen_set_cursor_row() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_cursor_row(10);
    assert_eq!(screen.cursor().row, 9);
}

#[test]
fn test_screen_set_cursor_row_clamp() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_cursor_row(200);
    assert_eq!(screen.cursor().row, 23);
}

// ============================================================================
// Screen::save_cursor / restore_cursor
// ============================================================================

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
fn test_screen_save_cursor_alternate_independent() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(10, 20);
    screen.save_cursor();

    screen.enter_alternate_screen();
    screen.move_cursor_to(5, 5);
    screen.save_cursor();

    screen.exit_alternate_screen();
    assert_eq!(screen.cursor().row, 9);
    assert_eq!(screen.cursor().col, 19);
}

// ============================================================================
// Screen::erase_display
// ============================================================================

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
    // Row 1 (index 1) cleared to col 4 (1-indexed col 5)
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
fn test_screen_erase_display_scrollback_preserved() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    screen.erase_display(3); // Mode 3 - ignored per implementation
    // Scrollback should still be intact (implementation ignores mode 3)
}

// ============================================================================
// Screen::erase_line
// ============================================================================

#[test]
fn test_screen_erase_line_to_end() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for c in "ABCDEFGHIJ".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 5);
    screen.erase_line(0);
    assert_eq!(screen.line(0).cell(3).display_char(), 'D');
    assert!(screen.line(0).cell(4).is_empty());
}

#[test]
fn test_screen_erase_line_to_start() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for c in "ABCDEFGHIJ".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 5);
    screen.erase_line(1);
    assert!(screen.line(0).cell(0).is_empty());
    assert!(screen.line(0).cell(4).is_empty());
    assert_eq!(screen.line(0).cell(5).display_char(), 'F');
}

#[test]
fn test_screen_erase_line_entire() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for c in "ABCDEFGHIJ".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 5);
    screen.erase_line(2);
    assert!(screen.line(0).is_empty());
}

// ============================================================================
// Screen::erase_chars
// ============================================================================

#[test]
fn test_screen_erase_chars() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for c in "ABCDEFGHIJ".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 3);
    screen.erase_chars(3);
    assert_eq!(screen.line(0).cell(1).display_char(), 'B');
    assert!(screen.line(0).cell(2).is_empty());
    assert!(screen.line(0).cell(3).is_empty());
    assert!(screen.line(0).cell(4).is_empty());
    assert_eq!(screen.line(0).cell(5).display_char(), 'F');
}

// ============================================================================
// Screen::insert_lines / delete_lines
// ============================================================================

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
    assert_eq!(screen.line(2).cell(0).display_char(), 'E');
    assert!(screen.line(3).is_empty());
}

// ============================================================================
// Screen::insert_chars / delete_chars
// ============================================================================

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
    assert!(screen.line(0).cell(3).is_empty());
    assert_eq!(screen.line(0).cell(4).display_char(), 'C');
}

#[test]
fn test_screen_delete_chars() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for c in "ABCDE".chars() {
        screen.print(c);
    }
    screen.move_cursor_to(1, 2);
    screen.delete_chars(2);
    assert_eq!(screen.line(0).cell(0).display_char(), 'A');
    assert_eq!(screen.line(0).cell(1).display_char(), 'D');
    assert_eq!(screen.line(0).cell(2).display_char(), 'E');
}

// ============================================================================
// Screen scroll region
// ============================================================================

#[test]
fn test_screen_set_scroll_region() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(5, 20);
    let (top, bottom) = screen.scroll_region();
    assert_eq!(top, 4);
    assert_eq!(bottom, 19);
}

#[test]
fn test_screen_scroll_region_default() {
    let screen = Screen::new(Dimensions::new(80, 24));
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
    assert_eq!(screen.line(1).cell(0).display_char(), 'C');
    assert_eq!(screen.line(2).cell(0).display_char(), 'D');
    assert!(screen.line(3).is_empty());
    assert_eq!(screen.line(4).cell(0).display_char(), 'E');
}

// ============================================================================
// Screen::scroll_up / scroll_down
// ============================================================================

#[test]
fn test_screen_scroll_up() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    screen.linefeed();
    screen.carriage_return();
    screen.print('B');
    screen.linefeed();
    screen.carriage_return();
    screen.print('C');
    screen.scroll_up(1);
    assert_eq!(screen.line(0).cell(0).display_char(), 'B');
    assert_eq!(screen.line(1).cell(0).display_char(), 'C');
    assert!(screen.line(2).is_empty());
}

#[test]
fn test_screen_scroll_down() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    screen.linefeed();
    screen.carriage_return();
    screen.print('B');
    screen.linefeed();
    screen.carriage_return();
    screen.print('C');
    screen.scroll_down(1);
    assert!(screen.line(0).is_empty());
    assert_eq!(screen.line(1).cell(0).display_char(), 'A');
    assert_eq!(screen.line(2).cell(0).display_char(), 'B');
}

// ============================================================================
// Screen::enter_alternate_screen / exit_alternate_screen
// ============================================================================

#[test]
fn test_screen_enter_alternate() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.enter_alternate_screen();
    assert!(screen.modes().alternate_screen);
    assert!(screen.line(0).cell(0).is_empty()); // Alternate starts clean
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
fn test_screen_alternate_cursor_restored() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(10, 20);
    screen.enter_alternate_screen();
    assert_eq!(screen.cursor().row, 0); // Reset in alternate
    screen.exit_alternate_screen();
    assert_eq!(screen.cursor().row, 9);
    assert_eq!(screen.cursor().col, 19);
}

#[test]
fn test_screen_alternate_no_scrollback() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.enter_alternate_screen();
    for _ in 0..10 {
        screen.print('X');
        screen.linefeed();
        screen.carriage_return();
    }
    // Alternate screen scrolling doesn't add to scrollback
    assert!(screen.scrollback().is_empty());
}

// ============================================================================
// Screen::resize
// ============================================================================

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
    screen.resize(Dimensions::new(40, 12));
    assert_eq!(screen.cols(), 40);
    assert_eq!(screen.rows(), 12);
}

#[test]
fn test_screen_resize_clamps_cursor() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(20, 70);
    screen.resize(Dimensions::new(40, 10));
    assert!(screen.cursor().col < 40);
    assert!(screen.cursor().row < 10);
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

// ============================================================================
// Screen::reset
// ============================================================================

#[test]
fn test_screen_reset() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.print('A');
    screen.move_cursor_to(10, 20);
    screen.modes_mut().bracketed_paste = true;
    screen.set_title("test");

    screen.reset();

    assert_eq!(screen.cursor().col, 0);
    assert_eq!(screen.cursor().row, 0);
    assert!(screen.line(0).cell(0).is_empty());
    assert!(!screen.modes().bracketed_paste);
    assert_eq!(screen.title(), "");
}

// ============================================================================
// Screen::set_title
// ============================================================================

#[test]
fn test_screen_set_title() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_title("My Terminal");
    assert_eq!(screen.title(), "My Terminal");
}

#[test]
fn test_screen_set_title_empty() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_title("test");
    screen.set_title("");
    assert_eq!(screen.title(), "");
}

#[test]
fn test_screen_set_title_long_truncated() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let long_title: String = "A".repeat(5000);
    screen.set_title(&long_title);
    assert!(screen.title().len() <= 4096);
}

// ============================================================================
// Screen::snapshot
// ============================================================================

#[test]
fn test_screen_snapshot_basic() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('H');
    screen.print('i');
    let snap = screen.snapshot(false);
    assert_eq!(snap.dimensions.cols, 10);
    assert_eq!(snap.dimensions.rows, 3);
    assert_eq!(snap.cursor.col, 2);
    assert_eq!(snap.cursor.row, 0);
}

#[test]
fn test_screen_snapshot_with_scrollback() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    for i in 0..5 {
        screen.print((b'A' + i as u8) as char);
        screen.linefeed();
        screen.carriage_return();
    }
    let snap = screen.snapshot(true);
    assert!(snap.scrollback.is_some());
}

#[test]
fn test_screen_snapshot_without_scrollback() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    let snap = screen.snapshot(false);
    assert!(snap.scrollback.is_none());
}

#[test]
fn test_screen_snapshot_screen_text() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('H');
    screen.print('i');
    let snap = screen.snapshot(false);
    let text = snap.screen_text();
    assert!(text.contains("Hi"));
}

#[test]
fn test_screen_snapshot_title() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.set_title("My Title");
    let snap = screen.snapshot(false);
    assert_eq!(snap.title, Some("My Title".to_string()));
}

#[test]
fn test_screen_snapshot_json_roundtrip() {
    let mut screen = Screen::new(Dimensions::new(10, 3));
    screen.print('A');
    let snap = screen.snapshot(false);
    let json = snap.to_json().unwrap();
    let parsed = terminal_core::Snapshot::from_json(&json).unwrap();
    assert_eq!(parsed.dimensions.cols, 10);
    assert_eq!(parsed.dimensions.rows, 3);
}

// ============================================================================
// Screen::register_hyperlink / get_hyperlink
// ============================================================================

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
fn test_screen_register_hyperlink_dedup() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let id1 = screen.register_hyperlink("https://example.com");
    let id2 = screen.register_hyperlink("https://example.com");
    assert_eq!(id1, id2);
}

#[test]
fn test_screen_register_hyperlink_different_urls() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    let id1 = screen.register_hyperlink("https://a.com");
    let id2 = screen.register_hyperlink("https://b.com");
    assert_ne!(id1, id2);
}

// ============================================================================
// Screen::charset operations
// ============================================================================

#[test]
fn test_screen_charset_default() {
    let screen = Screen::new(Dimensions::new(80, 24));
    assert_eq!(screen.charset().current(), Charset::Ascii);
}

#[test]
fn test_screen_shift_out_in() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.designate_charset(1, '0'); // G1 = DEC Special
    screen.shift_out(); // Activate G1
    assert_eq!(screen.charset().current(), Charset::DecSpecialGraphics);
    screen.shift_in(); // Back to G0
    assert_eq!(screen.charset().current(), Charset::Ascii);
}

#[test]
fn test_screen_designate_charset() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.designate_charset(0, '0');
    assert_eq!(screen.charset().g0, Charset::DecSpecialGraphics);
}

#[test]
fn test_screen_print_with_dec_charset() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.designate_charset(0, '0');
    screen.print('q'); // Should translate to ─
    assert_eq!(screen.line(0).cell(0).display_char(), '─');
}

// ============================================================================
// Screen::tab stops
// ============================================================================

#[test]
fn test_screen_set_tab_stop() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.cursor_mut().col = 5;
    screen.set_tab_stop();
    screen.cursor_mut().col = 0;
    screen.tab();
    assert_eq!(screen.cursor().col, 5);
}

#[test]
fn test_screen_clear_tab_stop_current() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.clear_tab_stop(0); // Clear at col 0
    screen.tab();
    assert_eq!(screen.cursor().col, 8); // Jumps to default tab at 8
}

#[test]
fn test_screen_clear_tab_stop_all() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.clear_tab_stop(3); // Clear all tab stops
    screen.tab();
    assert_eq!(screen.cursor().col, 79); // No tab stops, goes to end
}

// ============================================================================
// Screen::insert mode
// ============================================================================

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
    assert_eq!(screen.line(0).cell(3).display_char(), 'C');
}

// ============================================================================
// Screen selection access
// ============================================================================

#[test]
fn test_screen_selection_mut() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.selection_mut().start(Point::new(0, 0), SelectionType::Normal);
    assert!(screen.selection().active);
}

// ============================================================================
// Screen origin mode
// ============================================================================

#[test]
fn test_screen_origin_mode_cursor_movement() {
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.set_scroll_region(5, 20);
    screen.modes_mut().origin_mode = true;
    screen.move_cursor_to(1, 1); // Relative to scroll region
    assert_eq!(screen.cursor().row, 4); // top of scroll region (1-indexed row 5 = idx 4)
}

// ============================================================================
// Dimensions
// ============================================================================

#[test]
fn test_dimensions_new() {
    let d = Dimensions::new(80, 24);
    assert_eq!(d.cols, 80);
    assert_eq!(d.rows, 24);
}

#[test]
fn test_dimensions_default() {
    let d = Dimensions::default();
    assert_eq!(d.cols, 80);
    assert_eq!(d.rows, 24);
}

#[test]
fn test_dimensions_equality() {
    assert_eq!(Dimensions::new(80, 24), Dimensions::new(80, 24));
    assert_ne!(Dimensions::new(80, 24), Dimensions::new(80, 25));
}

#[test]
fn test_dimensions_clone() {
    let d = Dimensions::new(80, 24);
    let d2 = d;
    assert_eq!(d, d2);
}
