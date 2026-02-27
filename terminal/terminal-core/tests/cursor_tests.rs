//! Comprehensive tests for cursor state management

use terminal_core::{CellAttributes, Color, Cursor, CursorStyle};

// ============================================================
// Cursor Creation Tests
// ============================================================

#[test]
fn test_cursor_new_position() {
    let cursor = Cursor::new();
    assert_eq!(cursor.col, 0);
    assert_eq!(cursor.row, 0);
}

#[test]
fn test_cursor_new_visible() {
    let cursor = Cursor::new();
    assert!(cursor.visible);
}

#[test]
fn test_cursor_new_blinking() {
    let cursor = Cursor::new();
    assert!(cursor.blinking);
}

#[test]
fn test_cursor_new_style_block() {
    let cursor = Cursor::new();
    assert_eq!(cursor.style, CursorStyle::Block);
}

#[test]
fn test_cursor_new_no_origin_mode() {
    let cursor = Cursor::new();
    assert!(!cursor.origin_mode);
}

#[test]
fn test_cursor_new_no_pending_wrap() {
    let cursor = Cursor::new();
    assert!(!cursor.pending_wrap);
}

#[test]
fn test_cursor_new_no_hyperlink() {
    let cursor = Cursor::new();
    assert_eq!(cursor.hyperlink_id, 0);
}

#[test]
fn test_cursor_new_default_attrs() {
    let cursor = Cursor::new();
    assert_eq!(cursor.attrs, CellAttributes::default());
}

#[test]
fn test_cursor_default_equals_new() {
    let c1 = Cursor::new();
    let c2 = Cursor::default();
    assert_eq!(c1, c2);
}

// ============================================================
// CursorStyle Tests
// ============================================================

#[test]
fn test_cursor_style_block() {
    let style = CursorStyle::Block;
    assert_eq!(style, CursorStyle::Block);
}

#[test]
fn test_cursor_style_underline() {
    let style = CursorStyle::Underline;
    assert_eq!(style, CursorStyle::Underline);
}

#[test]
fn test_cursor_style_bar() {
    let style = CursorStyle::Bar;
    assert_eq!(style, CursorStyle::Bar);
}

#[test]
fn test_cursor_style_default_is_block() {
    assert_eq!(CursorStyle::default(), CursorStyle::Block);
}

#[test]
fn test_cursor_style_inequality() {
    assert_ne!(CursorStyle::Block, CursorStyle::Underline);
    assert_ne!(CursorStyle::Block, CursorStyle::Bar);
    assert_ne!(CursorStyle::Underline, CursorStyle::Bar);
}

#[test]
fn test_cursor_style_clone() {
    let style = CursorStyle::Bar;
    let clone = style;
    assert_eq!(style, clone);
}

// ============================================================
// move_to Tests
// ============================================================

#[test]
fn test_cursor_move_to_basic() {
    let mut cursor = Cursor::new();
    cursor.move_to(10, 5, 80, 24);
    assert_eq!(cursor.col, 10);
    assert_eq!(cursor.row, 5);
}

#[test]
fn test_cursor_move_to_origin() {
    let mut cursor = Cursor::new();
    cursor.move_to(0, 0, 80, 24);
    assert_eq!(cursor.col, 0);
    assert_eq!(cursor.row, 0);
}

#[test]
fn test_cursor_move_to_clamp_col() {
    let mut cursor = Cursor::new();
    cursor.move_to(100, 5, 80, 24);
    assert_eq!(cursor.col, 79);
}

#[test]
fn test_cursor_move_to_clamp_row() {
    let mut cursor = Cursor::new();
    cursor.move_to(10, 50, 80, 24);
    assert_eq!(cursor.row, 23);
}

#[test]
fn test_cursor_move_to_clamp_both() {
    let mut cursor = Cursor::new();
    cursor.move_to(200, 200, 80, 24);
    assert_eq!(cursor.col, 79);
    assert_eq!(cursor.row, 23);
}

#[test]
fn test_cursor_move_to_max_minus_one() {
    let mut cursor = Cursor::new();
    cursor.move_to(79, 23, 80, 24);
    assert_eq!(cursor.col, 79);
    assert_eq!(cursor.row, 23);
}

#[test]
fn test_cursor_move_to_clears_pending_wrap() {
    let mut cursor = Cursor::new();
    cursor.pending_wrap = true;
    cursor.move_to(5, 5, 80, 24);
    assert!(!cursor.pending_wrap);
}

// ============================================================
// move_up Tests
// ============================================================

#[test]
fn test_cursor_move_up_basic() {
    let mut cursor = Cursor::new();
    cursor.row = 10;
    cursor.move_up(3, 0);
    assert_eq!(cursor.row, 7);
}

#[test]
fn test_cursor_move_up_clamp_at_zero() {
    let mut cursor = Cursor::new();
    cursor.row = 2;
    cursor.move_up(5, 0);
    assert_eq!(cursor.row, 0);
}

#[test]
fn test_cursor_move_up_clamp_at_margin() {
    let mut cursor = Cursor::new();
    cursor.row = 5;
    cursor.origin_mode = true;
    cursor.move_up(10, 3);
    assert_eq!(cursor.row, 3);
}

#[test]
fn test_cursor_move_up_no_origin_mode_ignores_margin() {
    let mut cursor = Cursor::new();
    cursor.row = 5;
    cursor.origin_mode = false;
    cursor.move_up(10, 3);
    assert_eq!(cursor.row, 0);
}

#[test]
fn test_cursor_move_up_clears_pending_wrap() {
    let mut cursor = Cursor::new();
    cursor.row = 10;
    cursor.pending_wrap = true;
    cursor.move_up(1, 0);
    assert!(!cursor.pending_wrap);
}

#[test]
fn test_cursor_move_up_zero() {
    let mut cursor = Cursor::new();
    cursor.row = 5;
    cursor.move_up(0, 0);
    assert_eq!(cursor.row, 5);
}

// ============================================================
// move_down Tests
// ============================================================

#[test]
fn test_cursor_move_down_basic() {
    let mut cursor = Cursor::new();
    cursor.row = 5;
    cursor.move_down(3, 23, 24);
    assert_eq!(cursor.row, 8);
}

#[test]
fn test_cursor_move_down_clamp_at_max() {
    let mut cursor = Cursor::new();
    cursor.row = 20;
    cursor.move_down(10, 23, 24);
    assert_eq!(cursor.row, 23);
}

#[test]
fn test_cursor_move_down_clamp_at_margin_origin() {
    let mut cursor = Cursor::new();
    cursor.row = 5;
    cursor.origin_mode = true;
    cursor.move_down(20, 10, 24);
    assert_eq!(cursor.row, 10);
}

#[test]
fn test_cursor_move_down_no_origin_mode() {
    let mut cursor = Cursor::new();
    cursor.row = 5;
    cursor.origin_mode = false;
    cursor.move_down(20, 10, 24);
    assert_eq!(cursor.row, 23);
}

#[test]
fn test_cursor_move_down_clears_pending_wrap() {
    let mut cursor = Cursor::new();
    cursor.pending_wrap = true;
    cursor.move_down(1, 23, 24);
    assert!(!cursor.pending_wrap);
}

#[test]
fn test_cursor_move_down_zero() {
    let mut cursor = Cursor::new();
    cursor.row = 5;
    cursor.move_down(0, 23, 24);
    assert_eq!(cursor.row, 5);
}

// ============================================================
// move_left / move_right Tests
// ============================================================

#[test]
fn test_cursor_move_left_basic() {
    let mut cursor = Cursor::new();
    cursor.col = 10;
    cursor.move_left(3);
    assert_eq!(cursor.col, 7);
}

#[test]
fn test_cursor_move_left_clamp_at_zero() {
    let mut cursor = Cursor::new();
    cursor.col = 2;
    cursor.move_left(5);
    assert_eq!(cursor.col, 0);
}

#[test]
fn test_cursor_move_left_zero() {
    let mut cursor = Cursor::new();
    cursor.col = 5;
    cursor.move_left(0);
    assert_eq!(cursor.col, 5);
}

#[test]
fn test_cursor_move_left_clears_pending_wrap() {
    let mut cursor = Cursor::new();
    cursor.col = 10;
    cursor.pending_wrap = true;
    cursor.move_left(1);
    assert!(!cursor.pending_wrap);
}

#[test]
fn test_cursor_move_right_basic() {
    let mut cursor = Cursor::new();
    cursor.col = 10;
    cursor.move_right(5, 80);
    assert_eq!(cursor.col, 15);
}

#[test]
fn test_cursor_move_right_clamp_at_max() {
    let mut cursor = Cursor::new();
    cursor.col = 70;
    cursor.move_right(20, 80);
    assert_eq!(cursor.col, 79);
}

#[test]
fn test_cursor_move_right_zero() {
    let mut cursor = Cursor::new();
    cursor.col = 5;
    cursor.move_right(0, 80);
    assert_eq!(cursor.col, 5);
}

#[test]
fn test_cursor_move_right_clears_pending_wrap() {
    let mut cursor = Cursor::new();
    cursor.col = 10;
    cursor.pending_wrap = true;
    cursor.move_right(1, 80);
    assert!(!cursor.pending_wrap);
}

// ============================================================
// carriage_return Tests
// ============================================================

#[test]
fn test_cursor_carriage_return() {
    let mut cursor = Cursor::new();
    cursor.col = 50;
    cursor.carriage_return();
    assert_eq!(cursor.col, 0);
}

#[test]
fn test_cursor_carriage_return_clears_pending_wrap() {
    let mut cursor = Cursor::new();
    cursor.col = 50;
    cursor.pending_wrap = true;
    cursor.carriage_return();
    assert!(!cursor.pending_wrap);
}

#[test]
fn test_cursor_carriage_return_at_zero() {
    let mut cursor = Cursor::new();
    cursor.col = 0;
    cursor.carriage_return();
    assert_eq!(cursor.col, 0);
}

// ============================================================
// set_col / set_row Tests
// ============================================================

#[test]
fn test_cursor_set_col() {
    let mut cursor = Cursor::new();
    cursor.set_col(10, 80);
    assert_eq!(cursor.col, 10);
}

#[test]
fn test_cursor_set_col_clamp() {
    let mut cursor = Cursor::new();
    cursor.set_col(100, 80);
    assert_eq!(cursor.col, 79);
}

#[test]
fn test_cursor_set_col_clears_pending_wrap() {
    let mut cursor = Cursor::new();
    cursor.pending_wrap = true;
    cursor.set_col(5, 80);
    assert!(!cursor.pending_wrap);
}

#[test]
fn test_cursor_set_row() {
    let mut cursor = Cursor::new();
    cursor.set_row(10, 24);
    assert_eq!(cursor.row, 10);
}

#[test]
fn test_cursor_set_row_clamp() {
    let mut cursor = Cursor::new();
    cursor.set_row(50, 24);
    assert_eq!(cursor.row, 23);
}

#[test]
fn test_cursor_set_row_clears_pending_wrap() {
    let mut cursor = Cursor::new();
    cursor.pending_wrap = true;
    cursor.set_row(5, 24);
    assert!(!cursor.pending_wrap);
}

// ============================================================
// reset Tests
// ============================================================

#[test]
fn test_cursor_reset() {
    let mut cursor = Cursor::new();
    cursor.col = 50;
    cursor.row = 20;
    cursor.visible = false;
    cursor.blinking = false;
    cursor.style = CursorStyle::Bar;
    cursor.origin_mode = true;
    cursor.pending_wrap = true;
    cursor.hyperlink_id = 42;
    cursor.attrs.bold = true;
    cursor.attrs.fg = Color::Indexed(5);

    cursor.reset();

    assert_eq!(cursor, Cursor::new());
}

// ============================================================
// SavedCursor Tests
// ============================================================

#[test]
fn test_saved_cursor_via_screen_save_restore() {
    use terminal_core::{Dimensions, Screen};
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.move_cursor_to(6, 11); // 1-indexed -> row=5, col=10
    screen.cursor_mut().attrs.bold = true;
    screen.cursor_mut().attrs.fg = Color::Indexed(3);
    screen.cursor_mut().origin_mode = true;
    screen.cursor_mut().hyperlink_id = 7;

    screen.save_cursor();

    // Modify cursor
    screen.move_cursor_to(1, 1);
    screen.cursor_mut().attrs.bold = false;
    screen.cursor_mut().attrs.fg = Color::Default;
    screen.cursor_mut().origin_mode = false;
    screen.cursor_mut().hyperlink_id = 0;

    screen.restore_cursor();

    assert_eq!(screen.cursor().col, 10);
    assert_eq!(screen.cursor().row, 5);
    assert!(screen.cursor().attrs.bold);
    assert_eq!(screen.cursor().attrs.fg, Color::Indexed(3));
    assert!(screen.cursor().origin_mode);
    assert_eq!(screen.cursor().hyperlink_id, 7);
}

#[test]
fn test_saved_cursor_default_via_screen() {
    use terminal_core::{Dimensions, Screen};
    // A fresh screen has default saved cursor state
    let mut screen = Screen::new(Dimensions::new(80, 24));
    // Restore without prior save should give defaults
    screen.restore_cursor();
    assert_eq!(screen.cursor().col, 0);
    assert_eq!(screen.cursor().row, 0);
    assert!(!screen.cursor().origin_mode);
    assert!(!screen.cursor().pending_wrap);
    assert_eq!(screen.cursor().hyperlink_id, 0);
}

#[test]
fn test_saved_cursor_restore_does_not_affect_style() {
    use terminal_core::{Dimensions, Screen};
    let mut screen = Screen::new(Dimensions::new(80, 24));
    screen.cursor_mut().style = CursorStyle::Bar;
    screen.cursor_mut().visible = false;

    screen.save_cursor();

    screen.cursor_mut().style = CursorStyle::Underline;
    screen.cursor_mut().visible = true;

    screen.restore_cursor();

    // Style and visible are NOT saved/restored by SavedCursor
    assert_eq!(screen.cursor().style, CursorStyle::Underline);
    assert!(screen.cursor().visible);
}
