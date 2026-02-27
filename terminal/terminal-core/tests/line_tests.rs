//! Comprehensive tests for terminal line representation

use terminal_core::{CellAttributes, Color, Line};

// ============================================================================
// Line Creation
// ============================================================================

#[test]
fn test_line_new_cols() {
    let line = Line::new(80);
    assert_eq!(line.cols(), 80);
}

#[test]
fn test_line_new_not_wrapped() {
    let line = Line::new(10);
    assert!(!line.wrapped);
}

#[test]
fn test_line_new_all_empty() {
    let line = Line::new(10);
    for i in 0..10 {
        assert!(line.cell(i).is_empty());
    }
}

#[test]
fn test_line_new_single_col() {
    let line = Line::new(1);
    assert_eq!(line.cols(), 1);
}

#[test]
fn test_line_new_large() {
    let line = Line::new(500);
    assert_eq!(line.cols(), 500);
}

#[test]
fn test_line_with_attrs() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    let line = Line::with_attrs(10, attrs);
    assert_eq!(line.cols(), 10);
    assert!(line.cell(0).attrs.bold);
    assert!(line.cell(9).attrs.bold);
}

#[test]
fn test_line_with_attrs_fg_color() {
    let mut attrs = CellAttributes::new();
    attrs.fg = Color::Indexed(3);
    let line = Line::with_attrs(5, attrs);
    assert_eq!(line.cell(0).attrs.fg, Color::Indexed(3));
}

// ============================================================================
// Line Cell Access
// ============================================================================

#[test]
fn test_line_cell_get() {
    let line = Line::new(10);
    assert!(line.get(0).is_some());
    assert!(line.get(9).is_some());
    assert!(line.get(10).is_none());
}

#[test]
fn test_line_cell_get_mut() {
    let mut line = Line::new(10);
    assert!(line.get_mut(0).is_some());
    assert!(line.get_mut(10).is_none());
}

#[test]
fn test_line_cell_set_and_get() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('A');
    assert_eq!(line.cell(0).display_char(), 'A');
}

#[test]
fn test_line_cell_mut_modify_attrs() {
    let mut line = Line::new(10);
    line.cell_mut(5).attrs.bold = true;
    assert!(line.cell(5).attrs.bold);
}

// ============================================================================
// Line::clear
// ============================================================================

#[test]
fn test_line_clear_all() {
    let mut line = Line::new(10);
    for i in 0..10 {
        line.cell_mut(i).set_char('X');
    }
    line.clear(CellAttributes::default());
    for i in 0..10 {
        assert!(line.cell(i).is_empty());
    }
}

#[test]
fn test_line_clear_resets_wrapped() {
    let mut line = Line::new(10);
    line.wrapped = true;
    line.clear(CellAttributes::default());
    assert!(!line.wrapped);
}

#[test]
fn test_line_clear_applies_attrs() {
    let mut attrs = CellAttributes::new();
    attrs.bg = Color::Indexed(1);
    let mut line = Line::new(5);
    line.clear(attrs);
    assert_eq!(line.cell(0).attrs.bg, Color::Indexed(1));
}

// ============================================================================
// Line::clear_from
// ============================================================================

#[test]
fn test_line_clear_from_middle() {
    let mut line = Line::new(10);
    for i in 0..10 {
        line.cell_mut(i).set_char((b'A' + i as u8) as char);
    }
    line.clear_from(5, CellAttributes::default());
    assert_eq!(line.cell(4).display_char(), 'E');
    assert!(line.cell(5).is_empty());
    assert!(line.cell(9).is_empty());
}

#[test]
fn test_line_clear_from_start() {
    let mut line = Line::new(5);
    for i in 0..5 {
        line.cell_mut(i).set_char('X');
    }
    line.clear_from(0, CellAttributes::default());
    for i in 0..5 {
        assert!(line.cell(i).is_empty());
    }
}

#[test]
fn test_line_clear_from_end() {
    let mut line = Line::new(5);
    for i in 0..5 {
        line.cell_mut(i).set_char('X');
    }
    line.clear_from(4, CellAttributes::default());
    assert_eq!(line.cell(3).display_char(), 'X');
    assert!(line.cell(4).is_empty());
}

// ============================================================================
// Line::clear_to
// ============================================================================

#[test]
fn test_line_clear_to_middle() {
    let mut line = Line::new(10);
    for i in 0..10 {
        line.cell_mut(i).set_char((b'A' + i as u8) as char);
    }
    line.clear_to(4, CellAttributes::default());
    assert!(line.cell(0).is_empty());
    assert!(line.cell(4).is_empty());
    assert_eq!(line.cell(5).display_char(), 'F');
}

#[test]
fn test_line_clear_to_last() {
    let mut line = Line::new(5);
    for i in 0..5 {
        line.cell_mut(i).set_char('X');
    }
    line.clear_to(4, CellAttributes::default());
    for i in 0..5 {
        assert!(line.cell(i).is_empty());
    }
}

#[test]
fn test_line_clear_to_first() {
    let mut line = Line::new(5);
    for i in 0..5 {
        line.cell_mut(i).set_char('X');
    }
    line.clear_to(0, CellAttributes::default());
    assert!(line.cell(0).is_empty());
    assert_eq!(line.cell(1).display_char(), 'X');
}

// ============================================================================
// Line::resize
// ============================================================================

#[test]
fn test_line_resize_grow() {
    let mut line = Line::new(5);
    line.cell_mut(0).set_char('A');
    line.resize(10, CellAttributes::default());
    assert_eq!(line.cols(), 10);
    assert_eq!(line.cell(0).display_char(), 'A');
    assert!(line.cell(9).is_empty());
}

#[test]
fn test_line_resize_shrink() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('A');
    line.cell_mut(9).set_char('Z');
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
    attrs.italic = true;
    let mut line = Line::new(3);
    line.resize(5, attrs);
    assert!(line.cell(3).attrs.italic);
    assert!(line.cell(4).attrs.italic);
}

// ============================================================================
// Line::insert_cells
// ============================================================================

#[test]
fn test_line_insert_cells_middle() {
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
    line.insert_cells(0, 1, CellAttributes::default());
    assert!(line.cell(0).is_empty());
    assert_eq!(line.cell(1).display_char(), 'A');
}

#[test]
fn test_line_insert_cells_preserves_size() {
    let mut line = Line::new(10);
    line.insert_cells(3, 2, CellAttributes::default());
    assert_eq!(line.cols(), 10);
}

#[test]
fn test_line_insert_cells_beyond_end() {
    let mut line = Line::new(5);
    line.insert_cells(10, 2, CellAttributes::default()); // Should be no-op
    assert_eq!(line.cols(), 5);
}

// ============================================================================
// Line::delete_cells
// ============================================================================

#[test]
fn test_line_delete_cells_middle() {
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
    line.delete_cells(0, 1, CellAttributes::default());
    assert_eq!(line.cell(0).display_char(), 'B');
}

#[test]
fn test_line_delete_cells_preserves_size() {
    let mut line = Line::new(10);
    line.delete_cells(3, 2, CellAttributes::default());
    assert_eq!(line.cols(), 10);
}

#[test]
fn test_line_delete_cells_beyond_end() {
    let mut line = Line::new(5);
    line.delete_cells(10, 2, CellAttributes::default()); // Should be no-op
    assert_eq!(line.cols(), 5);
}

// ============================================================================
// Line::erase_cells
// ============================================================================

#[test]
fn test_line_erase_cells_middle() {
    let mut line = Line::new(10);
    for i in 0..10 {
        line.cell_mut(i).set_char('X');
    }
    line.erase_cells(3, 4, CellAttributes::default());
    assert_eq!(line.cell(2).display_char(), 'X');
    assert!(line.cell(3).is_empty());
    assert!(line.cell(6).is_empty());
    assert_eq!(line.cell(7).display_char(), 'X');
}

#[test]
fn test_line_erase_cells_no_shift() {
    let mut line = Line::new(5);
    for i in 0..5 {
        line.cell_mut(i).set_char((b'A' + i as u8) as char);
    }
    line.erase_cells(2, 1, CellAttributes::default());
    assert_eq!(line.cell(1).display_char(), 'B');
    assert!(line.cell(2).is_empty());
    assert_eq!(line.cell(3).display_char(), 'D');
}

// ============================================================================
// Line::text
// ============================================================================

#[test]
fn test_line_text_simple() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('H');
    line.cell_mut(1).set_char('i');
    assert_eq!(line.text(), "Hi");
}

#[test]
fn test_line_text_trims_trailing_spaces() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('A');
    assert_eq!(line.text(), "A");
}

#[test]
fn test_line_text_empty_line() {
    let line = Line::new(10);
    assert_eq!(line.text(), "");
}

#[test]
fn test_line_text_with_gaps() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('A');
    line.cell_mut(5).set_char('B');
    assert_eq!(line.text(), "A    B");
}

#[test]
fn test_line_text_full_line() {
    let mut line = Line::new(5);
    for i in 0..5 {
        line.cell_mut(i).set_char((b'A' + i as u8) as char);
    }
    assert_eq!(line.text(), "ABCDE");
}

#[test]
fn test_line_text_skips_continuation() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('中'); // Wide char
    line.cell_mut(1).set_continuation();
    line.cell_mut(2).set_char('A');
    let text = line.text();
    assert!(text.contains('中'));
    assert!(text.contains('A'));
}

// ============================================================================
// Line::is_empty
// ============================================================================

#[test]
fn test_line_is_empty_new() {
    let line = Line::new(10);
    assert!(line.is_empty());
}

#[test]
fn test_line_is_empty_with_content() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('A');
    assert!(!line.is_empty());
}

#[test]
fn test_line_is_empty_after_clear() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('A');
    line.clear(CellAttributes::default());
    assert!(line.is_empty());
}

// ============================================================================
// Line::wrapped flag
// ============================================================================

#[test]
fn test_line_wrapped_default_false() {
    let line = Line::new(10);
    assert!(!line.wrapped);
}

#[test]
fn test_line_wrapped_set() {
    let mut line = Line::new(10);
    line.wrapped = true;
    assert!(line.wrapped);
}

#[test]
fn test_line_wrapped_cleared_on_clear() {
    let mut line = Line::new(10);
    line.wrapped = true;
    line.clear(CellAttributes::default());
    assert!(!line.wrapped);
}

// ============================================================================
// Line iteration
// ============================================================================

#[test]
fn test_line_iter_count() {
    let line = Line::new(10);
    assert_eq!(line.iter().count(), 10);
}

#[test]
fn test_line_iter_content() {
    let mut line = Line::new(3);
    line.cell_mut(0).set_char('A');
    line.cell_mut(1).set_char('B');
    line.cell_mut(2).set_char('C');
    let chars: Vec<char> = line.iter().map(|c| c.display_char()).collect();
    assert_eq!(chars, vec!['A', 'B', 'C']);
}

#[test]
fn test_line_iter_mut() {
    let mut line = Line::new(5);
    for cell in line.iter_mut() {
        cell.set_char('X');
    }
    for i in 0..5 {
        assert_eq!(line.cell(i).display_char(), 'X');
    }
}
