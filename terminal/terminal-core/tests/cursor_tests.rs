//! Comprehensive tests for cursor state management

use terminal_core::{CellAttributes, Color, Cursor, CursorStyle};

// ============================================================================
// Cursor Creation
// ============================================================================

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
fn test_cursor_default_trait() {
    let cursor = Cursor::default();
    assert_eq!(cursor.col, 0);
    assert_eq!(cursor.row, 0);
}

// ============================================================================
// CursorStyle
// ============================================================================

#[test]
fn test_cursor_style_block() {
    assert_eq!(CursorStyle::default(), CursorStyle::Block);
}

#[test]
fn test_cursor_style_underline() {
    let style = CursorStyle::Underline;
    assert_ne!(style, CursorStyle::Block);
}

#[test]
fn test_cursor_style_bar() {
    let style = CursorStyle::Bar;
    assert_ne!(style, CursorStyle::Block);
    assert_ne!(style, CursorStyle::Underline);
}

#[test]
fn test_cursor_style_equality() {
    assert_eq!(CursorStyle::Block, CursorStyle::Block);
    assert_eq!(CursorStyle::Underline, CursorStyle::Underline);
    assert_eq!(CursorStyle::Bar, CursorStyle::Bar);
}

// ============================================================================
// Cursor::move_to
// ============================================================================

#[test]
fn test_cursor_move_to_valid() {
    let mut cursor = Cursor::new();
    cursor.move_to(10, 5, 80, 24);
    assert_eq!(cursor.col, 10);
    assert_eq!(cursor.row, 5);
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
    cursor.move_to(100, 50, 80, 24);
    assert_eq!(cursor.col, 79);
    assert_eq!(cursor.row, 23);
}

#[test]
fn test_cursor_move_to_origin() {
    let mut cursor = Cursor::new();
    cursor.col = 50;
    cursor.row = 20;
    cursor.move_to(0, 0, 80, 24);
    assert_eq!(cursor.col, 0);
    assert_eq!(cursor.row, 0);
}

#[test]
fn test_cursor_move_to_clears_pending_wrap() {
    let mut cursor = Cursor::new();
    cursor.pending_wrap = true;
    cursor.move_to(10, 5, 80, 24);
    assert!(!cursor.pending_wrap);
}

// ============================================================================
// Cursor::move_up
// ============================================================================

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
fn test_cursor_move_up_zero() {
    let mut cursor = Cursor::new();
    cursor.row = 5;
    cursor.move_up(0, 0);
    assert_eq!(cursor.row, 5);
}

#[test]
fn test_cursor_move_up_clears_pending_wrap() {
    let mut cursor = Cursor::new();
    cursor.row = 5;
    cursor.pending_wrap = true;
    cursor.move_up(1, 0);
    assert!(!cursor.pending_wrap);
}

// ============================================================================
// Cursor::move_down
// ============================================================================

#[test]
fn test_cursor_move_down_basic() {
    let mut cursor = Cursor::new();
    cursor.row = 10;
    cursor.move_down(5, 23, 24);
    assert_eq!(cursor.row, 15);
}

#[test]
fn test_cursor_move_down_clamp_at_bottom() {
    let mut cursor = Cursor::new();
    cursor.row = 20;
    cursor.move_down(10, 23, 24);
    assert_eq!(cursor.row, 23);
}

#[test]
fn test_cursor_move_down_clamp_at_margin() {
    let mut cursor = Cursor::new();
    cursor.row = 5;
    cursor.origin_mode = true;
    cursor.move_down(100, 10, 24);
    assert_eq!(cursor.row, 10);
}

#[test]
fn test_cursor_move_down_zero() {
    let mut cursor = Cursor::new();
    cursor.row = 5;
    cursor.move_down(0, 23, 24);
    assert_eq!(cursor.row, 5);
}

#[test]
fn test_cursor_move_down_clears_pending_wrap() {
    let mut cursor = Cursor::new();
    cursor.pending_wrap = true;
    cursor.move_down(1, 23, 24);
    assert!(!cursor.pending_wrap);
}

// ============================================================================
// Cursor::move_left
// ============================================================================

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
fn test_cursor_move_left_from_zero() {
    let mut cursor = Cursor::new();
    cursor.move_left(1);
    assert_eq!(cursor.col, 0);
}

#[test]
fn test_cursor_move_left_clears_pending_wrap() {
    let mut cursor = Cursor::new();
    cursor.col = 10;
    cursor.pending_wrap = true;
    cursor.move_left(1);
    assert!(!cursor.pending_wrap);
}

// ============================================================================
// Cursor::move_right
// ============================================================================

#[test]
fn test_cursor_move_right_basic() {
    let mut cursor = Cursor::new();
    cursor.col = 10;
    cursor.move_right(5, 80);
    assert_eq!(cursor.col, 15);
}

#[test]
fn test_cursor_move_right_clamp() {
    let mut cursor = Cursor::new();
    cursor.col = 75;
    cursor.move_right(10, 80);
    assert_eq!(cursor.col, 79);
}

#[test]
fn test_cursor_move_right_clears_pending_wrap() {
    let mut cursor = Cursor::new();
    cursor.pending_wrap = true;
    cursor.move_right(1, 80);
    assert!(!cursor.pending_wrap);
}

// ============================================================================
// Cursor::carriage_return
// ============================================================================

#[test]
fn test_cursor_carriage_return() {
    let mut cursor = Cursor::new();
    cursor.col = 50;
    cursor.carriage_return();
    assert_eq!(cursor.col, 0);
}

#[test]
fn test_cursor_carriage_return_from_zero() {
    let mut cursor = Cursor::new();
    cursor.carriage_return();
    assert_eq!(cursor.col, 0);
}

#[test]
fn test_cursor_carriage_return_clears_pending_wrap() {
    let mut cursor = Cursor::new();
    cursor.col = 79;
    cursor.pending_wrap = true;
    cursor.carriage_return();
    assert!(!cursor.pending_wrap);
}

#[test]
fn test_cursor_carriage_return_preserves_row() {
    let mut cursor = Cursor::new();
    cursor.col = 50;
    cursor.row = 10;
    cursor.carriage_return();
    assert_eq!(cursor.row, 10);
}

// ============================================================================
// Cursor::set_col / set_row
// ============================================================================

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
    cursor.set_col(10, 80);
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
    cursor.set_row(100, 24);
    assert_eq!(cursor.row, 23);
}

#[test]
fn test_cursor_set_row_clears_pending_wrap() {
    let mut cursor = Cursor::new();
    cursor.pending_wrap = true;
    cursor.set_row(10, 24);
    assert!(!cursor.pending_wrap);
}

// ============================================================================
// Cursor::reset
// ============================================================================

#[test]
fn test_cursor_reset_position() {
    let mut cursor = Cursor::new();
    cursor.col = 50;
    cursor.row = 20;
    cursor.reset();
    assert_eq!(cursor.col, 0);
    assert_eq!(cursor.row, 0);
}

#[test]
fn test_cursor_reset_style() {
    let mut cursor = Cursor::new();
    cursor.style = CursorStyle::Underline;
    cursor.reset();
    assert_eq!(cursor.style, CursorStyle::Block);
}

#[test]
fn test_cursor_reset_visibility() {
    let mut cursor = Cursor::new();
    cursor.visible = false;
    cursor.reset();
    assert!(cursor.visible);
}

#[test]
fn test_cursor_reset_attrs() {
    let mut cursor = Cursor::new();
    cursor.attrs.bold = true;
    cursor.attrs.fg = Color::Indexed(5);
    cursor.reset();
    assert_eq!(cursor.attrs, CellAttributes::default());
}

#[test]
fn test_cursor_reset_origin_mode() {
    let mut cursor = Cursor::new();
    cursor.origin_mode = true;
    cursor.reset();
    assert!(!cursor.origin_mode);
}

#[test]
fn test_cursor_reset_pending_wrap() {
    let mut cursor = Cursor::new();
    cursor.pending_wrap = true;
    cursor.reset();
    assert!(!cursor.pending_wrap);
}

// ============================================================================
// Cursor attributes
// ============================================================================

#[test]
fn test_cursor_set_attrs_bold() {
    let mut cursor = Cursor::new();
    cursor.attrs.bold = true;
    assert!(cursor.attrs.bold);
}

#[test]
fn test_cursor_set_attrs_italic() {
    let mut cursor = Cursor::new();
    cursor.attrs.italic = true;
    assert!(cursor.attrs.italic);
}

#[test]
fn test_cursor_set_attrs_underline() {
    let mut cursor = Cursor::new();
    cursor.attrs.underline = true;
    assert!(cursor.attrs.underline);
}

#[test]
fn test_cursor_set_attrs_fg_color() {
    let mut cursor = Cursor::new();
    cursor.attrs.fg = Color::rgb(255, 0, 0);
    assert_eq!(cursor.attrs.fg, Color::rgb(255, 0, 0));
}

#[test]
fn test_cursor_set_attrs_bg_color() {
    let mut cursor = Cursor::new();
    cursor.attrs.bg = Color::Indexed(4);
    assert_eq!(cursor.attrs.bg, Color::Indexed(4));
}

#[test]
fn test_cursor_set_attrs_multiple() {
    let mut cursor = Cursor::new();
    cursor.attrs.bold = true;
    cursor.attrs.italic = true;
    cursor.attrs.underline = true;
    cursor.attrs.strikethrough = true;
    assert!(cursor.attrs.bold);
    assert!(cursor.attrs.italic);
    assert!(cursor.attrs.underline);
    assert!(cursor.attrs.strikethrough);
}

#[test]
fn test_cursor_hyperlink_id() {
    let mut cursor = Cursor::new();
    cursor.hyperlink_id = 42;
    assert_eq!(cursor.hyperlink_id, 42);
}

// ============================================================================
// Cursor clone/equality
// ============================================================================

#[test]
fn test_cursor_clone() {
    let mut cursor = Cursor::new();
    cursor.col = 10;
    cursor.row = 5;
    cursor.attrs.bold = true;
    let cloned = cursor.clone();
    assert_eq!(cursor, cloned);
}

#[test]
fn test_cursor_inequality() {
    let cursor1 = Cursor::new();
    let mut cursor2 = Cursor::new();
    cursor2.col = 1;
    assert_ne!(cursor1, cursor2);
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn test_cursor_move_to_1x1_grid() {
    let mut cursor = Cursor::new();
    cursor.move_to(0, 0, 1, 1);
    assert_eq!(cursor.col, 0);
    assert_eq!(cursor.row, 0);
}

#[test]
fn test_cursor_move_to_1x1_grid_clamp() {
    let mut cursor = Cursor::new();
    cursor.move_to(10, 10, 1, 1);
    assert_eq!(cursor.col, 0);
    assert_eq!(cursor.row, 0);
}

#[test]
fn test_cursor_move_right_zero() {
    let mut cursor = Cursor::new();
    cursor.col = 5;
    cursor.move_right(0, 80);
    assert_eq!(cursor.col, 5);
}

#[test]
fn test_cursor_move_left_zero() {
    let mut cursor = Cursor::new();
    cursor.col = 5;
    cursor.move_left(0);
    assert_eq!(cursor.col, 5);
}

#[test]
fn test_cursor_set_col_zero() {
    let mut cursor = Cursor::new();
    cursor.col = 10;
    cursor.set_col(0, 80);
    assert_eq!(cursor.col, 0);
}

#[test]
fn test_cursor_set_row_zero() {
    let mut cursor = Cursor::new();
    cursor.row = 10;
    cursor.set_row(0, 24);
    assert_eq!(cursor.row, 0);
}
