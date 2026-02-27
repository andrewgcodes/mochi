//! Comprehensive tests for text selection

use terminal_core::{Point, Selection, SelectionType};

// ============================================================================
// Point
// ============================================================================

#[test]
fn test_point_new() {
    let p = Point::new(5, 10);
    assert_eq!(p.col, 5);
    assert_eq!(p.row, 10);
}

#[test]
fn test_point_new_zero() {
    let p = Point::new(0, 0);
    assert_eq!(p.col, 0);
    assert_eq!(p.row, 0);
}

#[test]
fn test_point_new_negative_row() {
    let p = Point::new(5, -3);
    assert_eq!(p.row, -3);
}

#[test]
fn test_point_equality() {
    assert_eq!(Point::new(5, 10), Point::new(5, 10));
    assert_ne!(Point::new(5, 10), Point::new(6, 10));
    assert_ne!(Point::new(5, 10), Point::new(5, 11));
}

#[test]
fn test_point_clone() {
    let p = Point::new(5, 10);
    let p2 = p;
    assert_eq!(p, p2);
}

// ============================================================================
// SelectionType
// ============================================================================

#[test]
fn test_selection_type_normal() {
    let t = SelectionType::Normal;
    assert_eq!(t, SelectionType::Normal);
}

#[test]
fn test_selection_type_word() {
    let t = SelectionType::Word;
    assert_ne!(t, SelectionType::Normal);
}

#[test]
fn test_selection_type_line() {
    let t = SelectionType::Line;
    assert_ne!(t, SelectionType::Normal);
    assert_ne!(t, SelectionType::Word);
}

#[test]
fn test_selection_type_block() {
    let t = SelectionType::Block;
    assert_ne!(t, SelectionType::Normal);
    assert_ne!(t, SelectionType::Line);
}

// ============================================================================
// Selection Creation
// ============================================================================

#[test]
fn test_selection_new_inactive() {
    let sel = Selection::new();
    assert!(!sel.active);
}

#[test]
fn test_selection_new_empty() {
    let sel = Selection::new();
    assert!(sel.is_empty());
}

#[test]
fn test_selection_new_type_normal() {
    let sel = Selection::new();
    assert_eq!(sel.selection_type, SelectionType::Normal);
}

#[test]
fn test_selection_default_trait() {
    let sel = Selection::default();
    assert!(!sel.active);
    assert!(sel.is_empty());
}

// ============================================================================
// Selection::start
// ============================================================================

#[test]
fn test_selection_start_activates() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    assert!(sel.active);
}

#[test]
fn test_selection_start_sets_position() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    assert_eq!(sel.start.col, 5);
    assert_eq!(sel.start.row, 10);
}

#[test]
fn test_selection_start_sets_end_to_start() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    assert_eq!(sel.end.col, 5);
    assert_eq!(sel.end.row, 10);
}

#[test]
fn test_selection_start_sets_type() {
    let mut sel = Selection::new();
    sel.start(Point::new(0, 0), SelectionType::Word);
    assert_eq!(sel.selection_type, SelectionType::Word);
}

#[test]
fn test_selection_start_type_line() {
    let mut sel = Selection::new();
    sel.start(Point::new(0, 0), SelectionType::Line);
    assert_eq!(sel.selection_type, SelectionType::Line);
}

#[test]
fn test_selection_start_type_block() {
    let mut sel = Selection::new();
    sel.start(Point::new(0, 0), SelectionType::Block);
    assert_eq!(sel.selection_type, SelectionType::Block);
}

// ============================================================================
// Selection::update
// ============================================================================

#[test]
fn test_selection_update_changes_end() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(20, 15));
    assert_eq!(sel.end.col, 20);
    assert_eq!(sel.end.row, 15);
}

#[test]
fn test_selection_update_preserves_start() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(20, 15));
    assert_eq!(sel.start.col, 5);
    assert_eq!(sel.start.row, 10);
}

#[test]
fn test_selection_update_inactive_noop() {
    let mut sel = Selection::new();
    sel.update(Point::new(20, 15));
    assert_eq!(sel.end.col, 0); // Not updated because inactive
}

#[test]
fn test_selection_update_multiple() {
    let mut sel = Selection::new();
    sel.start(Point::new(0, 0), SelectionType::Normal);
    sel.update(Point::new(5, 5));
    sel.update(Point::new(10, 10));
    assert_eq!(sel.end.col, 10);
    assert_eq!(sel.end.row, 10);
}

// ============================================================================
// Selection::clear
// ============================================================================

#[test]
fn test_selection_clear_deactivates() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.clear();
    assert!(!sel.active);
}

#[test]
fn test_selection_clear_makes_contains_false() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(15, 12));
    sel.clear();
    assert!(!sel.contains(10, 11));
}

// ============================================================================
// Selection::bounds
// ============================================================================

#[test]
fn test_selection_bounds_forward() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(15, 12));
    let (start, end) = sel.bounds();
    assert_eq!(start.row, 10);
    assert_eq!(end.row, 12);
}

#[test]
fn test_selection_bounds_reversed() {
    let mut sel = Selection::new();
    sel.start(Point::new(15, 12), SelectionType::Normal);
    sel.update(Point::new(5, 10));
    let (start, end) = sel.bounds();
    assert_eq!(start.row, 10);
    assert_eq!(end.row, 12);
}

#[test]
fn test_selection_bounds_same_row_forward() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(15, 10));
    let (start, end) = sel.bounds();
    assert_eq!(start.col, 5);
    assert_eq!(end.col, 15);
}

#[test]
fn test_selection_bounds_same_row_reversed() {
    let mut sel = Selection::new();
    sel.start(Point::new(15, 10), SelectionType::Normal);
    sel.update(Point::new(5, 10));
    let (start, end) = sel.bounds();
    assert_eq!(start.col, 5);
    assert_eq!(end.col, 15);
}

// ============================================================================
// Selection::contains - Normal
// ============================================================================

#[test]
fn test_selection_contains_normal_single_line() {
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
fn test_selection_contains_normal_multi_line_first() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(15, 12));

    assert!(sel.contains(5, 10));
    assert!(sel.contains(50, 10)); // After start col on first line
    assert!(!sel.contains(4, 10)); // Before start col
}

#[test]
fn test_selection_contains_normal_multi_line_middle() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(15, 12));

    assert!(sel.contains(0, 11));
    assert!(sel.contains(50, 11));
    assert!(sel.contains(100, 11));
}

#[test]
fn test_selection_contains_normal_multi_line_last() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(15, 12));

    assert!(sel.contains(0, 12));
    assert!(sel.contains(15, 12));
    assert!(!sel.contains(16, 12));
}

#[test]
fn test_selection_contains_normal_outside_rows() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Normal);
    sel.update(Point::new(15, 12));

    assert!(!sel.contains(10, 9));
    assert!(!sel.contains(10, 13));
}

#[test]
fn test_selection_contains_inactive() {
    let sel = Selection::new();
    assert!(!sel.contains(0, 0));
}

// ============================================================================
// Selection::contains - Line
// ============================================================================

#[test]
fn test_selection_contains_line_full_rows() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Line);
    sel.update(Point::new(15, 12));

    assert!(sel.contains(0, 10));
    assert!(sel.contains(100, 10));
    assert!(sel.contains(0, 11));
    assert!(sel.contains(100, 11));
    assert!(sel.contains(0, 12));
    assert!(sel.contains(100, 12));
}

#[test]
fn test_selection_contains_line_outside() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Line);
    sel.update(Point::new(15, 12));

    assert!(!sel.contains(0, 9));
    assert!(!sel.contains(0, 13));
}

#[test]
fn test_selection_contains_line_single_row() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Line);
    sel.update(Point::new(15, 10));

    assert!(sel.contains(0, 10));
    assert!(sel.contains(100, 10));
    assert!(!sel.contains(0, 9));
    assert!(!sel.contains(0, 11));
}

// ============================================================================
// Selection::contains - Block
// ============================================================================

#[test]
fn test_selection_contains_block_inside() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Block);
    sel.update(Point::new(15, 12));

    assert!(sel.contains(5, 10));
    assert!(sel.contains(10, 11));
    assert!(sel.contains(15, 12));
}

#[test]
fn test_selection_contains_block_outside_cols() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Block);
    sel.update(Point::new(15, 12));

    assert!(!sel.contains(4, 11));
    assert!(!sel.contains(16, 11));
}

#[test]
fn test_selection_contains_block_outside_rows() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Block);
    sel.update(Point::new(15, 12));

    assert!(!sel.contains(10, 9));
    assert!(!sel.contains(10, 13));
}

#[test]
fn test_selection_contains_block_reversed() {
    let mut sel = Selection::new();
    sel.start(Point::new(15, 12), SelectionType::Block);
    sel.update(Point::new(5, 10));

    assert!(sel.contains(10, 11));
    assert!(!sel.contains(4, 11));
    assert!(!sel.contains(16, 11));
}

// ============================================================================
// Selection::is_multiline
// ============================================================================

#[test]
fn test_selection_is_multiline_true() {
    let mut sel = Selection::new();
    sel.start(Point::new(0, 0), SelectionType::Normal);
    sel.update(Point::new(0, 5));
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

// ============================================================================
// Selection::is_empty
// ============================================================================

#[test]
fn test_selection_is_empty_new() {
    let sel = Selection::new();
    assert!(sel.is_empty());
}

#[test]
fn test_selection_is_empty_same_point() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 5), SelectionType::Normal);
    assert!(sel.is_empty()); // Start == end
}

#[test]
fn test_selection_is_empty_different_points() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 5), SelectionType::Normal);
    sel.update(Point::new(10, 5));
    assert!(!sel.is_empty());
}

#[test]
fn test_selection_is_empty_after_clear() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 5), SelectionType::Normal);
    sel.update(Point::new(10, 10));
    sel.clear();
    assert!(sel.is_empty());
}

// ============================================================================
// Selection::finish
// ============================================================================

#[test]
fn test_selection_finish_keeps_active() {
    let mut sel = Selection::new();
    sel.start(Point::new(5, 5), SelectionType::Normal);
    sel.update(Point::new(10, 10));
    sel.finish();
    assert!(sel.active); // Selection stays active after finish
}

// ============================================================================
// Selection with negative rows (scrollback)
// ============================================================================

#[test]
fn test_selection_negative_row() {
    let mut sel = Selection::new();
    sel.start(Point::new(0, -5), SelectionType::Normal);
    sel.update(Point::new(10, 5));
    assert!(sel.contains(5, 0));
    assert!(sel.contains(5, -3));
}

#[test]
fn test_selection_both_negative() {
    let mut sel = Selection::new();
    sel.start(Point::new(0, -10), SelectionType::Normal);
    sel.update(Point::new(10, -5));
    assert!(sel.contains(5, -7));
    assert!(!sel.contains(5, -11));
    assert!(!sel.contains(5, -4));
}

// ============================================================================
// Word selection (future feature)
// ============================================================================

#[test]
fn test_selection_word_type_same_as_normal_contains() {
    // Word selection uses same contains logic as Normal
    let mut sel = Selection::new();
    sel.start(Point::new(5, 10), SelectionType::Word);
    sel.update(Point::new(15, 10));

    assert!(sel.contains(5, 10));
    assert!(sel.contains(10, 10));
    assert!(sel.contains(15, 10));
    assert!(!sel.contains(4, 10));
}
