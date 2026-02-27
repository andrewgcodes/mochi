//! Comprehensive tests for terminal-core lib module

use terminal_core::Dimensions;

// ============================================================
// Dimensions Tests
// ============================================================

#[test]
fn test_dimensions_new() {
    let dims = Dimensions::new(80, 24);
    assert_eq!(dims.cols, 80);
    assert_eq!(dims.rows, 24);
}

#[test]
fn test_dimensions_default() {
    let dims = Dimensions::default();
    assert_eq!(dims.cols, 80);
    assert_eq!(dims.rows, 24);
}

#[test]
fn test_dimensions_custom() {
    let dims = Dimensions::new(120, 40);
    assert_eq!(dims.cols, 120);
    assert_eq!(dims.rows, 40);
}

#[test]
fn test_dimensions_small() {
    let dims = Dimensions::new(1, 1);
    assert_eq!(dims.cols, 1);
    assert_eq!(dims.rows, 1);
}

#[test]
fn test_dimensions_equality() {
    let d1 = Dimensions::new(80, 24);
    let d2 = Dimensions::new(80, 24);
    assert_eq!(d1, d2);
}

#[test]
fn test_dimensions_inequality_cols() {
    let d1 = Dimensions::new(80, 24);
    let d2 = Dimensions::new(120, 24);
    assert_ne!(d1, d2);
}

#[test]
fn test_dimensions_inequality_rows() {
    let d1 = Dimensions::new(80, 24);
    let d2 = Dimensions::new(80, 40);
    assert_ne!(d1, d2);
}

#[test]
fn test_dimensions_clone() {
    let dims = Dimensions::new(80, 24);
    let clone = dims;
    assert_eq!(dims, clone);
}

#[test]
fn test_dimensions_copy() {
    let dims = Dimensions::new(80, 24);
    let copy = dims;
    assert_eq!(dims.cols, copy.cols);
    assert_eq!(dims.rows, copy.rows);
}

#[test]
fn test_dimensions_large() {
    let dims = Dimensions::new(10000, 5000);
    assert_eq!(dims.cols, 10000);
    assert_eq!(dims.rows, 5000);
}

// ============================================================
// Public Exports Tests (verify types are accessible)
// ============================================================

#[test]
fn test_cell_accessible() {
    let cell = terminal_core::Cell::new();
    assert!(cell.is_empty());
}

#[test]
fn test_cell_attributes_accessible() {
    let attrs = terminal_core::CellAttributes::new();
    assert!(!attrs.bold);
}

#[test]
fn test_color_accessible() {
    let color = terminal_core::Color::Default;
    assert_eq!(color, terminal_core::Color::Default);
}

#[test]
fn test_cursor_accessible() {
    let cursor = terminal_core::Cursor::new();
    assert_eq!(cursor.col, 0);
}

#[test]
fn test_cursor_style_accessible() {
    let style = terminal_core::CursorStyle::Block;
    assert_eq!(style, terminal_core::CursorStyle::Block);
}

#[test]
fn test_grid_accessible() {
    let grid = terminal_core::Grid::new(Dimensions::new(10, 5));
    assert_eq!(grid.cols(), 10);
}

#[test]
fn test_line_accessible() {
    let line = terminal_core::Line::new(10);
    assert_eq!(line.cols(), 10);
}

#[test]
fn test_modes_accessible() {
    let modes = terminal_core::Modes::new();
    assert!(modes.auto_wrap);
}

#[test]
fn test_screen_accessible() {
    let screen = terminal_core::Screen::new(Dimensions::new(80, 24));
    assert_eq!(screen.cols(), 80);
}

#[test]
fn test_scrollback_accessible() {
    let sb = terminal_core::Scrollback::new(100);
    assert!(sb.is_empty());
}

#[test]
fn test_selection_accessible() {
    let sel = terminal_core::Selection::new();
    assert!(!sel.active);
}

#[test]
fn test_point_accessible() {
    let p = terminal_core::Point::new(5, 10);
    assert_eq!(p.col, 5);
}

#[test]
fn test_selection_type_accessible() {
    let st = terminal_core::SelectionType::Normal;
    assert_eq!(st, terminal_core::SelectionType::Normal);
}

#[test]
fn test_charset_accessible() {
    let cs = terminal_core::Charset::Ascii;
    assert_eq!(cs, terminal_core::Charset::Ascii);
}

#[test]
fn test_charset_state_accessible() {
    let cs = terminal_core::CharsetState::new();
    assert_eq!(cs.current(), terminal_core::Charset::Ascii);
}
