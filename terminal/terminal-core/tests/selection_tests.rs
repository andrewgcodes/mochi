//! Comprehensive tests for text selection

use terminal_core::{Point, Selection, SelectionType};

// ============================================================
// Point Tests
// ============================================================

#[test]
fn test_point_new() {
    let p = Point::new(5, 10);
    assert_eq!(p.col, 5);
    assert_eq!(p.row, 10);
}

#[test]
fn test_point_zero() {
    let p = Point::new(0, 0);
    assert_eq!(p.col, 0);
    assert_eq!(p.row, 0);
}

#[test]
fn test_point_negative_row() {
    let p = Point::new(5, -10);
    assert_eq!(p.row, -10);
}

#[test]
fn test_point_equality() {
    assert_eq!(Point::new(5, 10), Point::new(5, 10));
}

#[test]
fn test_point_inequality() {
    assert_ne!(Point::new(5, 10), Point::new(6, 10));
    assert_ne!(Point::new(5, 10), Point::new(5, 11));
}

#[test]
fn test_point_clone() {
    let p = Point::new(5, 10);
    let clone = p;
    assert_eq!(p, clone);
}

// ============================================================
// SelectionType Tests
// ============================================================

#[test]
fn test_selection_type_normal() {
    let t = SelectionType::Normal;
    assert_eq!(t, SelectionType::Normal);
}

#[test]
fn test_selection_type_word() {
    let t = SelectionType::Word;
    assert_eq!(t, SelectionType::Word);
}

#[test]
fn test_selection_type_line() {
    let t = SelectionType::Line;
    assert_eq!(t, SelectionType::Line);
}

#[test]
fn test_selection_type_block() {
    let t = SelectionType::Block;
    assert_eq!(t, SelectionType::Block);
}

#[test]
fn test_selection_type_inequality() {
    assert_ne!(SelectionType::Normal, SelectionType::Word);
    assert_ne!(SelectionType::Normal, SelectionType::Line);
    assert_ne!(SelectionType::Normal, SelectionType::Block);
    assert_ne!(SelectionType::Word, SelectionType::Line);
}

// ============================================================
// Selection Creation Tests
// ============================================================

#[test]
fn test_selection_new() {
    let sel = Selection::new();
    assert!(!sel.active);
    assert!(sel.is_empty());
    assert_eq!(sel.selection_type, SelectionType::Normal);
}

#[test]
fn test_selection_default_equals_new() {
    let s1 = Selection::new();
    let s2 = Selection::default();
    assert_eq!(s1, s2);
}

// ============================================================
// Selection Start Tests
// ============================================================

#[test]
fn test_selection_start_normal() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    assert!(sel.active);
    assert_eq!(sel.start.col, 5);
    assert_eq!(sel.start.row, 10);
    assert_eq!(sel.end.col, 5);
    assert_eq!(sel.end.row, 10);
    assert_eq!(sel.selection_type, SelectionType::Normal);
}

#[test]
fn test_selection_start_word() {
    let mut sel = Selection::new();
    sel.start(Point::new(0, 0), SelectionType::Word);
    assert_eq!(sel.selection_type, SelectionType::Word);
    assert!(sel.active);
}

#[test]
fn test_selection_start_line() {
    let mut sel = Selection::new();
    sel.start(Point::new(0, 0), SelectionType::Line);
    assert_eq!(sel.selection_type, SelectionType::Line);
}

#[test]
fn test_selection_start_block() {
    let mut sel = Selection::new();
    sel.start(Point::new(0, 0), SelectionType::Block);
    assert_eq!(sel.selection_type, SelectionType::Block);
}

// ============================================================
// Selection Update Tests
// ============================================================

#[test]
fn test_selection_update() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(20, 15));
    assert_eq!(sel.end.col, 20);
    assert_eq!(sel.end.row, 15);
}

#[test]
fn test_selection_update_when_inactive() {
    let mut sel = Selection::new();
    sel.update(Point::new(20, 15));
    // Should not change end when inactive
    assert_eq!(sel.end.col, 0);
    assert_eq!(sel.end.row, 0);
}

#[test]
fn test_selection_update_multiple() {
    let mut sel = Selection::new();
    sel.start(Point::new(0, 0), SelectionType::Normal);
    sel.update(Point::new(10, 5));
    sel.update(Point::new(20, 10));
    assert_eq!(sel.end.col, 20);
    assert_eq!(sel.end.row, 10);
}

// ============================================================
// Selection Bounds Tests
// ============================================================

#[test]
fn test_selection_bounds_start_before_end() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(20, 15));
    let (start, end) = sel.bounds();
    assert_eq!(start.row, 10);
    assert_eq!(end.row, 15);
}

#[test]
fn test_selection_bounds_start_after_end() {
    let mut sel = Selection::new();
    sel.start(Point::new(20, 15), SelectionType::Normal);
    sel.update(Point::new(5, 10));
    let (start, end) = sel.bounds();
    assert_eq!(start.row, 10);
    assert_eq!(end.row, 15);
}

#[test]
fn test_selection_bounds_same_row_start_before() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(20, 10));
    let (start, end) = sel.bounds();
    assert_eq!(start.col, 5);
    assert_eq!(end.col, 20);
}

#[test]
fn test_selection_bounds_same_row_start_after() {
    let mut sel = Selection::new();
    sel.start(Point::new(20, 10), SelectionType::Normal);
    sel.update(Point::new(5, 10));
    let (start, end) = sel.bounds();
    assert_eq!(start.col, 5);
    assert_eq!(end.col, 20);
}

// ============================================================
// Selection Contains Tests - Normal
// ============================================================

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
    assert!(!sel.contains(10, 9));
    assert!(!sel.contains(10, 11));
}

#[test]
fn test_selection_contains_multi_line() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(15, 12));

    // First line: from col 5 to end
    assert!(sel.contains(5, 10));
    assert!(sel.contains(100, 10));
    assert!(!sel.contains(4, 10));

    // Middle line: entire line
    assert!(sel.contains(0, 11));
    assert!(sel.contains(100, 11));

    // Last line: from start to col 15
    assert!(sel.contains(0, 12));
    assert!(sel.contains(15, 12));
    assert!(!sel.contains(16, 12));
}

#[test]
fn test_selection_contains_inactive() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(15, 10));
    sel.clear();
    assert!(!sel.contains(10, 10));
}

#[test]
fn test_selection_contains_outside_range() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(15, 12));
    assert!(!sel.contains(0, 9));
    assert!(!sel.contains(0, 13));
}

// ============================================================
// Selection Contains Tests - Line
// ============================================================

#[test]
fn test_selection_line_contains_entire_row() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Line);
    sel.update(Point::new(15, 12));

    assert!(sel.contains(0, 10));
    assert!(sel.contains(100, 10));
    assert!(sel.contains(0, 11));
    assert!(sel.contains(0, 12));
    assert!(sel.contains(100, 12));
    assert!(!sel.contains(0, 9));
    assert!(!sel.contains(0, 13));
}

#[test]
fn test_selection_line_single_row() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Line);
    sel.update(Point::new(15, 10));
    assert!(sel.contains(0, 10));
    assert!(sel.contains(100, 10));
}

// ============================================================
// Selection Contains Tests - Block
// ============================================================

#[test]
fn test_selection_block_rectangular() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Block);
    sel.update(Point::new(15, 12));

    assert!(sel.contains(5, 10));
    assert!(sel.contains(15, 10));
    assert!(sel.contains(10, 11));
    assert!(sel.contains(5, 12));
    assert!(sel.contains(15, 12));

    assert!(!sel.contains(4, 10));
    assert!(!sel.contains(16, 10));
    assert!(!sel.contains(4, 11));
    assert!(!sel.contains(16, 11));
}

#[test]
fn test_selection_block_reversed() {
    let mut sel = Selection::new();
    sel.start(Point::new(15, 12), SelectionType::Block);
    sel.update(Point::new(5, 10));

    assert!(sel.contains(5, 10));
    assert!(sel.contains(15, 12));
    assert!(sel.contains(10, 11));
}

// ============================================================
// Selection State Tests
// ============================================================

#[test]
fn test_selection_is_multiline_true() {
    let mut sel = Selection::new();
    sel.start(Point::new(0, 0), SelectionType::Normal);
    sel.update(Point::new(10, 5));
    assert!(sel.is_multiline());
}

#[test]
fn test_selection_is_multiline_false() {
    let mut sel = Selection::new();
    sel.start(Point::new(0, 5), SelectionType::Normal);
    sel.update(Point::new(10, 5));
    assert!(!sel.is_multiline());
}

#[test]
fn test_selection_is_multiline_inactive() {
    let sel = Selection::new();
    assert!(!sel.is_multiline());
}

#[test]
fn test_selection_is_empty_when_inactive() {
    let sel = Selection::new();
    assert!(sel.is_empty());
}

#[test]
fn test_selection_is_empty_same_point() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    // start == end so it's "empty"
    assert!(sel.is_empty());
}

#[test]
fn test_selection_is_not_empty_different_points() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(10, 10));
    assert!(!sel.is_empty());
}

// ============================================================
// Clear Tests
// ============================================================

#[test]
fn test_selection_clear() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(20, 15));
    sel.clear();
    assert!(!sel.active);
}

#[test]
fn test_selection_clear_then_start() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.clear();
    sel.start(Point::new(0, 0), SelectionType::Word);
    assert!(sel.active);
    assert_eq!(sel.selection_type, SelectionType::Word);
}

// ============================================================
// Finish Tests
// ============================================================

#[test]
fn test_selection_finish() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(20, 15));
    sel.finish();
    // Selection should still be active
    assert!(sel.active);
}

// ============================================================
// Clone/Eq Tests
// ============================================================

#[test]
fn test_selection_clone() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    let clone = sel.clone();
    assert_eq!(sel, clone);
}

#[test]
fn test_selection_inequality() {
    let mut s1 = Selection::new();
    let s2 = Selection::new();
    s1.start(Point::new(5, 10), SelectionType::Normal);
    assert_ne!(s1, s2);
}

// ============================================================
// Negative Row Tests (scrollback)
// ============================================================

#[test]
fn test_selection_negative_rows() {
    let mut sel = Selection::new();
    sel.start(Point::new(0, -5), SelectionType::Normal);
    sel.update(Point::new(10, 2));
    assert!(sel.contains(0, -5));
    assert!(sel.contains(5, 0));
    assert!(sel.contains(10, 2));
    assert!(!sel.contains(0, -6));
    assert!(!sel.contains(0, 3));
}

#[test]
fn test_selection_word_type_contains_like_normal() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Word);
    sel.update(Point::new(15, 10));
    // Word type behaves like Normal for contains
    assert!(sel.contains(5, 10));
    assert!(sel.contains(15, 10));
    assert!(!sel.contains(4, 10));
    assert!(!sel.contains(16, 10));
}
