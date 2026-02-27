//! Comprehensive tests for terminal line representation

use terminal_core::{CellAttributes, Line};

// ============================================================
// Line Creation Tests
// ============================================================

#[test]
fn test_line_new() {
    let line = Line::new(80);
    assert_eq!(line.cols(), 80);
    assert!(!line.wrapped);
}

#[test]
fn test_line_new_small() {
    let line = Line::new(1);
    assert_eq!(line.cols(), 1);
}

#[test]
fn test_line_new_all_empty() {
    let line = Line::new(10);
    for i in 0..10 {
        assert!(line.cell(i).is_empty());
    }
}

#[test]
fn test_line_new_not_wrapped() {
    let line = Line::new(10);
    assert!(!line.wrapped);
}

#[test]
fn test_line_with_attrs() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    let line = Line::with_attrs(10, attrs);
    assert_eq!(line.cols(), 10);
    assert!(line.cell(0).attrs.bold);
}

#[test]
fn test_line_with_attrs_all_cells() {
    let mut attrs = CellAttributes::new();
    attrs.italic = true;
    let line = Line::with_attrs(5, attrs);
    for i in 0..5 {
        assert!(line.cell(i).attrs.italic);
    }
}

// ============================================================
// Cell Access Tests
// ============================================================

#[test]
fn test_line_cell() {
    let line = Line::new(10);
    let cell = line.cell(0);
    assert!(cell.is_empty());
}

#[test]
fn test_line_cell_mut() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('A');
    assert_eq!(line.cell(0).display_char(), 'A');
}

#[test]
fn test_line_get_valid() {
    let line = Line::new(10);
    assert!(line.get(0).is_some());
    assert!(line.get(9).is_some());
}

#[test]
fn test_line_get_out_of_bounds() {
    let line = Line::new(10);
    assert!(line.get(10).is_none());
    assert!(line.get(100).is_none());
}

#[test]
fn test_line_get_mut_valid() {
    let mut line = Line::new(10);
    assert!(line.get_mut(0).is_some());
}

#[test]
fn test_line_get_mut_out_of_bounds() {
    let mut line = Line::new(10);
    assert!(line.get_mut(10).is_none());
}

// ============================================================
// Clear Tests
// ============================================================

#[test]
fn test_line_clear() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('A');
    line.cell_mut(5).set_char('B');
    line.wrapped = true;
    line.clear(CellAttributes::default());
    assert!(line.cell(0).is_empty());
    assert!(line.cell(5).is_empty());
    assert!(!line.wrapped);
}

#[test]
fn test_line_clear_with_attrs() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    let mut line = Line::new(10);
    line.clear(attrs);
    assert!(line.cell(0).attrs.bold);
}

#[test]
fn test_line_clear_from() {
    let mut line = Line::new(10);
    for i in 0..10 {
        line.cell_mut(i).set_char((b'A' + i as u8) as char);
    }
    line.clear_from(5, CellAttributes::default());
    for i in 0..5 {
        assert_eq!(line.cell(i).display_char(), (b'A' + i as u8) as char);
    }
    for i in 5..10 {
        assert!(line.cell(i).is_empty());
    }
}

#[test]
fn test_line_clear_from_zero() {
    let mut line = Line::new(10);
    for i in 0..10 {
        line.cell_mut(i).set_char('X');
    }
    line.clear_from(0, CellAttributes::default());
    for i in 0..10 {
        assert!(line.cell(i).is_empty());
    }
}

#[test]
fn test_line_clear_to() {
    let mut line = Line::new(10);
    for i in 0..10 {
        line.cell_mut(i).set_char((b'A' + i as u8) as char);
    }
    line.clear_to(4, CellAttributes::default());
    for i in 0..=4 {
        assert!(line.cell(i).is_empty());
    }
    for i in 5..10 {
        assert_eq!(line.cell(i).display_char(), (b'A' + i as u8) as char);
    }
}

#[test]
fn test_line_clear_to_last() {
    let mut line = Line::new(10);
    for i in 0..10 {
        line.cell_mut(i).set_char('X');
    }
    line.clear_to(9, CellAttributes::default());
    for i in 0..10 {
        assert!(line.cell(i).is_empty());
    }
}

// ============================================================
// Resize Tests
// ============================================================

#[test]
fn test_line_resize_grow() {
    let mut line = Line::new(5);
    line.cell_mut(0).set_char('A');
    line.resize(10, CellAttributes::default());
    assert_eq!(line.cols(), 10);
    assert_eq!(line.cell(0).display_char(), 'A');
    assert!(line.cell(5).is_empty());
}

#[test]
fn test_line_resize_shrink() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('A');
    line.cell_mut(9).set_char('B');
    line.resize(5, CellAttributes::default());
    assert_eq!(line.cols(), 5);
    assert_eq!(line.cell(0).display_char(), 'A');
}

#[test]
fn test_line_resize_same() {
    let mut line = Line::new(10);
    line.resize(10, CellAttributes::default());
    assert_eq!(line.cols(), 10);
}

#[test]
fn test_line_resize_grow_with_attrs() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    let mut line = Line::new(5);
    line.resize(10, attrs);
    assert!(line.cell(5).attrs.bold);
}

// ============================================================
// Insert Cells Tests
// ============================================================

#[test]
fn test_line_insert_cells() {
    let mut line = Line::new(5);
    for i in 0..5 {
        line.cell_mut(i).set_char((b'A' + i as u8) as char);
    }
    line.insert_cells(2, 2, CellAttributes::default());
    assert_eq!(line.cell(0).display_char(), 'A');
    assert_eq!(line.cell(1).display_char(), 'B');
    assert!(line.cell(2).is_empty());
    assert!(line.cell(3).is_empty());
    assert_eq!(line.cell(4).display_char(), 'C');
}

#[test]
fn test_line_insert_cells_at_start() {
    let mut line = Line::new(5);
    for i in 0..5 {
        line.cell_mut(i).set_char((b'A' + i as u8) as char);
    }
    line.insert_cells(0, 2, CellAttributes::default());
    assert!(line.cell(0).is_empty());
    assert!(line.cell(1).is_empty());
    assert_eq!(line.cell(2).display_char(), 'A');
}

#[test]
fn test_line_insert_cells_preserves_length() {
    let mut line = Line::new(10);
    for i in 0..10 {
        line.cell_mut(i).set_char('X');
    }
    line.insert_cells(5, 3, CellAttributes::default());
    assert_eq!(line.cols(), 10);
}

#[test]
fn test_line_insert_cells_out_of_bounds() {
    let mut line = Line::new(5);
    line.cell_mut(0).set_char('A');
    line.insert_cells(10, 2, CellAttributes::default());
    // Should do nothing
    assert_eq!(line.cell(0).display_char(), 'A');
}

// ============================================================
// Delete Cells Tests
// ============================================================

#[test]
fn test_line_delete_cells() {
    let mut line = Line::new(5);
    for i in 0..5 {
        line.cell_mut(i).set_char((b'A' + i as u8) as char);
    }
    line.delete_cells(1, 2, CellAttributes::default());
    assert_eq!(line.cell(0).display_char(), 'A');
    assert_eq!(line.cell(1).display_char(), 'D');
    assert_eq!(line.cell(2).display_char(), 'E');
    assert!(line.cell(3).is_empty());
    assert!(line.cell(4).is_empty());
}

#[test]
fn test_line_delete_cells_at_start() {
    let mut line = Line::new(5);
    for i in 0..5 {
        line.cell_mut(i).set_char((b'A' + i as u8) as char);
    }
    line.delete_cells(0, 2, CellAttributes::default());
    assert_eq!(line.cell(0).display_char(), 'C');
    assert_eq!(line.cell(1).display_char(), 'D');
    assert_eq!(line.cell(2).display_char(), 'E');
    assert!(line.cell(3).is_empty());
}

#[test]
fn test_line_delete_cells_preserves_length() {
    let mut line = Line::new(10);
    line.delete_cells(0, 3, CellAttributes::default());
    assert_eq!(line.cols(), 10);
}

#[test]
fn test_line_delete_cells_out_of_bounds() {
    let mut line = Line::new(5);
    line.cell_mut(0).set_char('A');
    line.delete_cells(10, 2, CellAttributes::default());
    assert_eq!(line.cell(0).display_char(), 'A');
}

// ============================================================
// Erase Cells Tests
// ============================================================

#[test]
fn test_line_erase_cells() {
    let mut line = Line::new(10);
    for i in 0..10 {
        line.cell_mut(i).set_char('X');
    }
    line.erase_cells(3, 4, CellAttributes::default());
    assert_eq!(line.cell(2).display_char(), 'X');
    assert!(line.cell(3).is_empty());
    assert!(line.cell(4).is_empty());
    assert!(line.cell(5).is_empty());
    assert!(line.cell(6).is_empty());
    assert_eq!(line.cell(7).display_char(), 'X');
}

#[test]
fn test_line_erase_cells_overflow() {
    let mut line = Line::new(5);
    for i in 0..5 {
        line.cell_mut(i).set_char('X');
    }
    line.erase_cells(3, 10, CellAttributes::default());
    assert_eq!(line.cell(2).display_char(), 'X');
    assert!(line.cell(3).is_empty());
    assert!(line.cell(4).is_empty());
}

// ============================================================
// Text Extraction Tests
// ============================================================

#[test]
fn test_line_text_simple() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('H');
    line.cell_mut(1).set_char('i');
    assert_eq!(line.text(), "Hi");
}

#[test]
fn test_line_text_empty() {
    let line = Line::new(10);
    assert_eq!(line.text(), "");
}

#[test]
fn test_line_text_trims_trailing_spaces() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('A');
    line.cell_mut(1).set_char('B');
    // Remaining cells are empty/space - should be trimmed
    assert_eq!(line.text(), "AB");
}

#[test]
fn test_line_text_preserves_middle_spaces() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('A');
    // cell 1 is empty/space
    line.cell_mut(2).set_char('B');
    assert_eq!(line.text(), "A B");
}

#[test]
fn test_line_text_with_wide_char() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('中');
    line.cell_mut(1).set_continuation();
    line.cell_mut(2).set_char('文');
    // The continuation cell should be skipped
    let text = line.text();
    assert!(text.contains('中'));
    assert!(text.contains('文'));
}

// ============================================================
// is_empty Tests
// ============================================================

#[test]
fn test_line_is_empty_true() {
    let line = Line::new(10);
    assert!(line.is_empty());
}

#[test]
fn test_line_is_empty_false() {
    let mut line = Line::new(10);
    line.cell_mut(5).set_char('X');
    assert!(!line.is_empty());
}

// ============================================================
// Iterator Tests
// ============================================================

#[test]
fn test_line_iter_count() {
    let line = Line::new(10);
    assert_eq!(line.iter().count(), 10);
}

#[test]
fn test_line_iter_mut_modify() {
    let mut line = Line::new(5);
    for cell in line.iter_mut() {
        cell.set_char('X');
    }
    for i in 0..5 {
        assert_eq!(line.cell(i).display_char(), 'X');
    }
}

// ============================================================
// Clone/Eq Tests
// ============================================================

#[test]
fn test_line_clone() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('A');
    let clone = line.clone();
    assert_eq!(line, clone);
}

#[test]
fn test_line_equality() {
    let line1 = Line::new(10);
    let line2 = Line::new(10);
    assert_eq!(line1, line2);
}

#[test]
fn test_line_inequality_content() {
    let mut line1 = Line::new(10);
    let line2 = Line::new(10);
    line1.cell_mut(0).set_char('A');
    assert_ne!(line1, line2);
}

#[test]
fn test_line_inequality_wrapped() {
    let mut line1 = Line::new(10);
    let line2 = Line::new(10);
    line1.wrapped = true;
    assert_ne!(line1, line2);
}

fn set_continuation(cell: &mut terminal_core::Cell) {
    cell.set_continuation();
}

#[test]
fn test_line_text_continuation_skipped() {
    let mut line = Line::new(4);
    line.cell_mut(0).set_char('A');
    set_continuation(line.cell_mut(1));
    line.cell_mut(2).set_char('B');
    let text = line.text();
    assert!(text.contains('A'));
    assert!(text.contains('B'));
}
