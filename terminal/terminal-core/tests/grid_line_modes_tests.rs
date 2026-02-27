//! Comprehensive tests for Grid, Line, and Modes
//!
//! ~150 tests covering grid operations, line manipulation, and terminal modes.

use terminal_core::{Cell, CellAttributes, Color, Dimensions, Grid, Line, Modes};

// ============================================================================
// Line Tests (~50 tests)
// ============================================================================

#[test]
fn test_line_new_correct_cols() {
    let line = Line::new(80);
    assert_eq!(line.cols(), 80);
}

#[test]
fn test_line_new_not_wrapped() {
    let line = Line::new(80);
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
fn test_line_with_attrs() {
    let mut attrs = CellAttributes::default();
    attrs.bold = true;
    let line = Line::with_attrs(10, attrs);
    assert_eq!(line.cols(), 10);
    assert!(line.cell(0).attrs.bold);
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
    line.get_mut(5).unwrap().set_char('X');
    assert_eq!(line.cell(5).display_char(), 'X');
}

#[test]
fn test_line_get_mut_out_of_bounds() {
    let mut line = Line::new(10);
    assert!(line.get_mut(10).is_none());
}

#[test]
fn test_line_cell_set_and_read() {
    let mut line = Line::new(80);
    line.cell_mut(0).set_char('H');
    line.cell_mut(1).set_char('i');
    assert_eq!(line.cell(0).display_char(), 'H');
    assert_eq!(line.cell(1).display_char(), 'i');
}

#[test]
fn test_line_clear() {
    let mut line = Line::new(10);
    for i in 0..10 {
        line.cell_mut(i).set_char('X');
    }
    line.wrapped = true;
    line.clear(CellAttributes::default());
    assert!(!line.wrapped);
    for i in 0..10 {
        assert!(line.cell(i).is_empty());
    }
}

#[test]
fn test_line_clear_with_attrs() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('A');
    let mut attrs = CellAttributes::default();
    attrs.bg = Color::Indexed(1);
    line.clear(attrs);
    assert!(line.cell(0).is_empty());
    assert_eq!(line.cell(0).attrs.bg, Color::Indexed(1));
}

#[test]
fn test_line_clear_from() {
    let mut line = Line::new(10);
    for i in 0..10 {
        line.cell_mut(i).set_char((b'A' + i as u8) as char);
    }
    line.clear_from(5, CellAttributes::default());
    for i in 0..5 {
        assert!(!line.cell(i).is_empty());
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
    for i in 0..5 {
        assert!(line.cell(i).is_empty());
    }
    for i in 5..10 {
        assert!(!line.cell(i).is_empty());
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
    for i in 0..10 {
        line.cell_mut(i).set_char((b'A' + i as u8) as char);
    }
    line.resize(5, CellAttributes::default());
    assert_eq!(line.cols(), 5);
    assert_eq!(line.cell(4).display_char(), 'E');
}

#[test]
fn test_line_resize_same() {
    let mut line = Line::new(10);
    line.resize(10, CellAttributes::default());
    assert_eq!(line.cols(), 10);
}

#[test]
fn test_line_resize_grow_with_attrs() {
    let mut attrs = CellAttributes::default();
    attrs.bg = Color::Indexed(4);
    let mut line = Line::new(5);
    line.resize(10, attrs);
    assert_eq!(line.cell(7).attrs.bg, Color::Indexed(4));
}

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
fn test_line_insert_cells_past_end() {
    let mut line = Line::new(5);
    line.insert_cells(10, 2, CellAttributes::default());
    assert_eq!(line.cols(), 5); // Should not change
}

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
    line.delete_cells(0, 3, CellAttributes::default());
    assert_eq!(line.cell(0).display_char(), 'D');
    assert_eq!(line.cell(1).display_char(), 'E');
}

#[test]
fn test_line_delete_cells_past_end() {
    let mut line = Line::new(5);
    line.delete_cells(10, 2, CellAttributes::default());
    assert_eq!(line.cols(), 5);
}

#[test]
fn test_line_erase_cells() {
    let mut line = Line::new(10);
    for i in 0..10 {
        line.cell_mut(i).set_char((b'A' + i as u8) as char);
    }
    line.erase_cells(3, 4, CellAttributes::default());
    assert_eq!(line.cell(2).display_char(), 'C');
    assert!(line.cell(3).is_empty());
    assert!(line.cell(6).is_empty());
    assert_eq!(line.cell(7).display_char(), 'H');
}

#[test]
fn test_line_text_basic() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('H');
    line.cell_mut(1).set_char('e');
    line.cell_mut(2).set_char('l');
    line.cell_mut(3).set_char('l');
    line.cell_mut(4).set_char('o');
    assert_eq!(line.text(), "Hello");
}

#[test]
fn test_line_text_trims_trailing_spaces() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('H');
    line.cell_mut(1).set_char('i');
    // Rest are empty/spaces
    assert_eq!(line.text(), "Hi");
}

#[test]
fn test_line_text_empty() {
    let line = Line::new(10);
    assert_eq!(line.text(), "");
}

#[test]
fn test_line_text_preserves_inner_spaces() {
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('A');
    // cells 1,2 are empty (spaces)
    line.cell_mut(3).set_char('B');
    assert_eq!(line.text(), "A  B");
}

#[test]
fn test_line_is_empty() {
    let line = Line::new(10);
    assert!(line.is_empty());
}

#[test]
fn test_line_is_not_empty() {
    let mut line = Line::new(10);
    line.cell_mut(5).set_char('X');
    assert!(!line.is_empty());
}

#[test]
fn test_line_iter() {
    let mut line = Line::new(3);
    line.cell_mut(0).set_char('A');
    line.cell_mut(1).set_char('B');
    line.cell_mut(2).set_char('C');
    let chars: Vec<char> = line.iter().map(|c| c.display_char()).collect();
    assert_eq!(chars, vec!['A', 'B', 'C']);
}

#[test]
fn test_line_iter_mut() {
    let mut line = Line::new(3);
    for cell in line.iter_mut() {
        cell.set_char('Z');
    }
    assert_eq!(line.cell(0).display_char(), 'Z');
    assert_eq!(line.cell(1).display_char(), 'Z');
    assert_eq!(line.cell(2).display_char(), 'Z');
}

#[test]
fn test_line_wrapped_flag() {
    let mut line = Line::new(10);
    line.wrapped = true;
    assert!(line.wrapped);
    line.wrapped = false;
    assert!(!line.wrapped);
}

#[test]
fn test_line_clone() {
    let mut line = Line::new(5);
    line.cell_mut(0).set_char('A');
    line.wrapped = true;
    let cloned = line.clone();
    assert_eq!(cloned.cell(0).display_char(), 'A');
    assert!(cloned.wrapped);
    assert_eq!(cloned.cols(), 5);
}

#[test]
fn test_line_partial_eq() {
    let a = Line::new(10);
    let b = Line::new(10);
    assert_eq!(a, b);
}

#[test]
fn test_line_not_equal_different_content() {
    let mut a = Line::new(10);
    let b = Line::new(10);
    a.cell_mut(0).set_char('X');
    assert_ne!(a, b);
}

#[test]
fn test_line_text_skips_continuation_cells() {
    let mut line = Line::new(5);
    line.cell_mut(0).set_content("中"); // Wide char
    line.cell_mut(1).set_continuation();
    line.cell_mut(2).set_char('A');
    let text = line.text();
    assert!(text.contains("中"));
    assert!(text.contains("A"));
}

#[test]
fn test_line_delete_more_than_available() {
    let mut line = Line::new(5);
    for i in 0..5 {
        line.cell_mut(i).set_char((b'A' + i as u8) as char);
    }
    line.delete_cells(2, 10, CellAttributes::default());
    assert_eq!(line.cols(), 5);
    assert_eq!(line.cell(0).display_char(), 'A');
    assert_eq!(line.cell(1).display_char(), 'B');
}

// ============================================================================
// Grid Tests (~60 tests)
// ============================================================================

#[test]
fn test_grid_new() {
    let grid = Grid::new(Dimensions::new(80, 24));
    assert_eq!(grid.cols(), 80);
    assert_eq!(grid.rows(), 24);
}

#[test]
fn test_grid_dimensions() {
    let grid = Grid::new(Dimensions::new(120, 40));
    let dims = grid.dimensions();
    assert_eq!(dims.cols, 120);
    assert_eq!(dims.rows, 40);
}

#[test]
fn test_grid_all_lines_empty() {
    let grid = Grid::new(Dimensions::new(80, 24));
    for row in 0..24 {
        assert!(grid.line(row).is_empty());
    }
}

#[test]
fn test_grid_line_access() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let line = grid.line(0);
    assert_eq!(line.cols(), 80);
}

#[test]
fn test_grid_line_mut_access() {
    let mut grid = Grid::new(Dimensions::new(80, 24));
    grid.line_mut(0).cell_mut(0).set_char('A');
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
}

#[test]
fn test_grid_get_line_valid() {
    let grid = Grid::new(Dimensions::new(80, 24));
    assert!(grid.get_line(0).is_some());
    assert!(grid.get_line(23).is_some());
}

#[test]
fn test_grid_get_line_out_of_bounds() {
    let grid = Grid::new(Dimensions::new(80, 24));
    assert!(grid.get_line(24).is_none());
}

#[test]
fn test_grid_get_line_mut_valid() {
    let mut grid = Grid::new(Dimensions::new(80, 24));
    assert!(grid.get_line_mut(0).is_some());
}

#[test]
fn test_grid_get_line_mut_out_of_bounds() {
    let mut grid = Grid::new(Dimensions::new(80, 24));
    assert!(grid.get_line_mut(24).is_none());
}

#[test]
fn test_grid_clear() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.line_mut(4).cell_mut(9).set_char('Z');
    grid.clear(CellAttributes::default());
    for row in 0..5 {
        assert!(grid.line(row).is_empty());
    }
}

#[test]
fn test_grid_clear_with_attrs() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    let mut attrs = CellAttributes::default();
    attrs.bg = Color::Indexed(2);
    grid.clear(attrs);
    assert_eq!(grid.line(0).cell(0).attrs.bg, Color::Indexed(2));
}

#[test]
fn test_grid_clear_below() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.clear_below(2, 0, CellAttributes::default());
    assert!(!grid.line(0).is_empty());
    assert!(!grid.line(1).is_empty());
    assert!(grid.line(2).is_empty());
    assert!(grid.line(3).is_empty());
    assert!(grid.line(4).is_empty());
}

#[test]
fn test_grid_clear_above() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.clear_above(2, 9, CellAttributes::default());
    assert!(grid.line(0).is_empty());
    assert!(grid.line(1).is_empty());
    assert!(grid.line(2).is_empty());
    assert!(!grid.line(3).is_empty());
    assert!(!grid.line(4).is_empty());
}

#[test]
fn test_grid_scroll_up_one() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    let scrolled = grid.scroll_up(0, 4, 1, CellAttributes::default());
    assert_eq!(scrolled.len(), 1);
    assert_eq!(grid.line(0).cell(0).display_char(), 'B');
    assert_eq!(grid.line(3).cell(0).display_char(), 'E');
    assert!(grid.line(4).is_empty());
}

#[test]
fn test_grid_scroll_up_multiple() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    let scrolled = grid.scroll_up(0, 4, 3, CellAttributes::default());
    assert_eq!(scrolled.len(), 3);
    assert_eq!(grid.line(0).cell(0).display_char(), 'D');
    assert_eq!(grid.line(1).cell(0).display_char(), 'E');
}

#[test]
fn test_grid_scroll_up_returns_scrolled_lines() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.line_mut(1).cell_mut(0).set_char('B');
    grid.line_mut(2).cell_mut(0).set_char('C');
    let scrolled = grid.scroll_up(0, 2, 1, CellAttributes::default());
    assert_eq!(scrolled.len(), 1);
    assert_eq!(scrolled[0].cell(0).display_char(), 'A');
}

#[test]
fn test_grid_scroll_down() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.scroll_down(0, 4, 1, CellAttributes::default());
    assert!(grid.line(0).is_empty());
    assert_eq!(grid.line(1).cell(0).display_char(), 'A');
    assert_eq!(grid.line(4).cell(0).display_char(), 'D');
}

#[test]
fn test_grid_scroll_down_multiple() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.scroll_down(0, 4, 2, CellAttributes::default());
    assert!(grid.line(0).is_empty());
    assert!(grid.line(1).is_empty());
    assert_eq!(grid.line(2).cell(0).display_char(), 'A');
}

#[test]
fn test_grid_scroll_region() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    // Scroll only rows 1-3
    grid.scroll_up(1, 3, 1, CellAttributes::default());
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
    assert_eq!(grid.line(1).cell(0).display_char(), 'C');
    assert_eq!(grid.line(2).cell(0).display_char(), 'D');
    assert!(grid.line(3).is_empty());
    assert_eq!(grid.line(4).cell(0).display_char(), 'E');
}

#[test]
fn test_grid_insert_lines() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.insert_lines(1, 2, 4, CellAttributes::default());
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
    assert!(grid.line(1).is_empty());
    assert!(grid.line(2).is_empty());
    assert_eq!(grid.line(3).cell(0).display_char(), 'B');
    assert_eq!(grid.line(4).cell(0).display_char(), 'C');
}

#[test]
fn test_grid_delete_lines() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.delete_lines(1, 2, 4, CellAttributes::default());
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
    assert_eq!(grid.line(1).cell(0).display_char(), 'D');
    assert_eq!(grid.line(2).cell(0).display_char(), 'E');
    assert!(grid.line(3).is_empty());
    assert!(grid.line(4).is_empty());
}

#[test]
fn test_grid_resize_larger() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.resize(Dimensions::new(20, 10), CellAttributes::default());
    assert_eq!(grid.cols(), 20);
    assert_eq!(grid.rows(), 10);
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
}

#[test]
fn test_grid_resize_smaller() {
    let mut grid = Grid::new(Dimensions::new(20, 10));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.resize(Dimensions::new(10, 5), CellAttributes::default());
    assert_eq!(grid.cols(), 10);
    assert_eq!(grid.rows(), 5);
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
}

#[test]
fn test_grid_resize_cols_only() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.resize(Dimensions::new(20, 5), CellAttributes::default());
    assert_eq!(grid.cols(), 20);
    assert_eq!(grid.rows(), 5);
}

#[test]
fn test_grid_resize_rows_only() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.resize(Dimensions::new(10, 10), CellAttributes::default());
    assert_eq!(grid.cols(), 10);
    assert_eq!(grid.rows(), 10);
}

#[test]
fn test_grid_iter() {
    let grid = Grid::new(Dimensions::new(10, 3));
    let count = grid.iter().count();
    assert_eq!(count, 3);
}

#[test]
fn test_grid_iter_mut() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    for line in grid.iter_mut() {
        line.cell_mut(0).set_char('X');
    }
    for row in 0..3 {
        assert_eq!(grid.line(row).cell(0).display_char(), 'X');
    }
}

#[test]
fn test_grid_small_1x1() {
    let grid = Grid::new(Dimensions::new(1, 1));
    assert_eq!(grid.cols(), 1);
    assert_eq!(grid.rows(), 1);
}

#[test]
fn test_grid_write_all_cells() {
    let mut grid = Grid::new(Dimensions::new(5, 5));
    for row in 0..5 {
        for col in 0..5 {
            grid.line_mut(row).cell_mut(col).set_char('X');
        }
    }
    for row in 0..5 {
        for col in 0..5 {
            assert_eq!(grid.line(row).cell(col).display_char(), 'X');
        }
    }
}

#[test]
fn test_grid_scroll_up_all() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    for row in 0..3 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.scroll_up(0, 2, 3, CellAttributes::default());
    for row in 0..3 {
        assert!(grid.line(row).is_empty());
    }
}

#[test]
fn test_grid_scroll_down_all() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    for row in 0..3 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.scroll_down(0, 2, 3, CellAttributes::default());
    for row in 0..3 {
        assert!(grid.line(row).is_empty());
    }
}

#[test]
fn test_grid_insert_lines_at_top() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.insert_lines(0, 2, 4, CellAttributes::default());
    assert!(grid.line(0).is_empty());
    assert!(grid.line(1).is_empty());
    assert_eq!(grid.line(2).cell(0).display_char(), 'A');
}

#[test]
fn test_grid_delete_lines_at_top() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.delete_lines(0, 2, 4, CellAttributes::default());
    assert_eq!(grid.line(0).cell(0).display_char(), 'C');
    assert_eq!(grid.line(1).cell(0).display_char(), 'D');
    assert_eq!(grid.line(2).cell(0).display_char(), 'E');
}

#[test]
fn test_grid_scroll_preserves_outside_region() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    // Scroll rows 2-3 up by 1
    grid.scroll_up(2, 3, 1, CellAttributes::default());
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
    assert_eq!(grid.line(1).cell(0).display_char(), 'B');
    assert_eq!(grid.line(2).cell(0).display_char(), 'D');
    assert!(grid.line(3).is_empty());
    assert_eq!(grid.line(4).cell(0).display_char(), 'E');
}

#[test]
fn test_grid_large_resize() {
    let mut grid = Grid::new(Dimensions::new(10, 10));
    grid.resize(Dimensions::new(200, 100), CellAttributes::default());
    assert_eq!(grid.cols(), 200);
    assert_eq!(grid.rows(), 100);
}

// ============================================================================
// Modes Tests (~40 tests)
// ============================================================================

#[test]
fn test_modes_new_defaults() {
    let modes = Modes::new();
    assert!(!modes.insert_mode);
    assert!(!modes.linefeed_mode);
    assert!(!modes.cursor_keys_application);
    assert!(modes.ansi_mode);
    assert!(!modes.column_132);
    assert!(!modes.smooth_scroll);
    assert!(!modes.reverse_video);
    assert!(!modes.origin_mode);
    assert!(modes.auto_wrap);
    assert!(modes.auto_repeat);
    assert!(modes.cursor_visible);
}

#[test]
fn test_modes_mouse_defaults() {
    let modes = Modes::new();
    assert!(!modes.mouse_x10);
    assert!(!modes.mouse_vt200);
    assert!(!modes.mouse_button_event);
    assert!(!modes.mouse_any_event);
    assert!(!modes.mouse_sgr);
}

#[test]
fn test_modes_screen_defaults() {
    let modes = Modes::new();
    assert!(!modes.alternate_screen);
    assert!(!modes.bracketed_paste);
    assert!(!modes.focus_events);
    assert!(!modes.synchronized_output);
}

#[test]
fn test_set_dec_mode_cursor_keys() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1, true);
    assert!(modes.cursor_keys_application);
    modes.set_dec_mode(1, false);
    assert!(!modes.cursor_keys_application);
}

#[test]
fn test_set_dec_mode_ansi() {
    let mut modes = Modes::new();
    modes.set_dec_mode(2, false);
    assert!(!modes.ansi_mode);
}

#[test]
fn test_set_dec_mode_column_132() {
    let mut modes = Modes::new();
    modes.set_dec_mode(3, true);
    assert!(modes.column_132);
}

#[test]
fn test_set_dec_mode_smooth_scroll() {
    let mut modes = Modes::new();
    modes.set_dec_mode(4, true);
    assert!(modes.smooth_scroll);
}

#[test]
fn test_set_dec_mode_reverse_video() {
    let mut modes = Modes::new();
    modes.set_dec_mode(5, true);
    assert!(modes.reverse_video);
}

#[test]
fn test_set_dec_mode_origin() {
    let mut modes = Modes::new();
    modes.set_dec_mode(6, true);
    assert!(modes.origin_mode);
}

#[test]
fn test_set_dec_mode_auto_wrap() {
    let mut modes = Modes::new();
    modes.set_dec_mode(7, false);
    assert!(!modes.auto_wrap);
    modes.set_dec_mode(7, true);
    assert!(modes.auto_wrap);
}

#[test]
fn test_set_dec_mode_auto_repeat() {
    let mut modes = Modes::new();
    modes.set_dec_mode(8, false);
    assert!(!modes.auto_repeat);
}

#[test]
fn test_set_dec_mode_mouse_x10() {
    let mut modes = Modes::new();
    modes.set_dec_mode(9, true);
    assert!(modes.mouse_x10);
}

#[test]
fn test_set_dec_mode_cursor_visible() {
    let mut modes = Modes::new();
    modes.set_dec_mode(25, false);
    assert!(!modes.cursor_visible);
    modes.set_dec_mode(25, true);
    assert!(modes.cursor_visible);
}

#[test]
fn test_set_dec_mode_mouse_vt200() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1000, true);
    assert!(modes.mouse_vt200);
}

#[test]
fn test_set_dec_mode_mouse_button_event() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1002, true);
    assert!(modes.mouse_button_event);
}

#[test]
fn test_set_dec_mode_mouse_any_event() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1003, true);
    assert!(modes.mouse_any_event);
}

#[test]
fn test_set_dec_mode_focus_events() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1004, true);
    assert!(modes.focus_events);
}

#[test]
fn test_set_dec_mode_mouse_sgr() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1006, true);
    assert!(modes.mouse_sgr);
}

#[test]
fn test_set_dec_mode_alternate_screen() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1049, true);
    assert!(modes.alternate_screen);
}

#[test]
fn test_set_dec_mode_bracketed_paste() {
    let mut modes = Modes::new();
    modes.set_dec_mode(2004, true);
    assert!(modes.bracketed_paste);
}

#[test]
fn test_set_dec_mode_synchronized_output() {
    let mut modes = Modes::new();
    modes.set_dec_mode(2026, true);
    assert!(modes.synchronized_output);
}

#[test]
fn test_get_dec_mode_cursor_keys() {
    let mut modes = Modes::new();
    assert!(!modes.get_dec_mode(1));
    modes.set_dec_mode(1, true);
    assert!(modes.get_dec_mode(1));
}

#[test]
fn test_get_dec_mode_all() {
    let modes = Modes::new();
    assert!(modes.get_dec_mode(2)); // ansi_mode default true
    assert!(!modes.get_dec_mode(3)); // column_132
    assert!(!modes.get_dec_mode(4)); // smooth_scroll
    assert!(!modes.get_dec_mode(5)); // reverse_video
    assert!(!modes.get_dec_mode(6)); // origin_mode
    assert!(modes.get_dec_mode(7)); // auto_wrap default true
    assert!(modes.get_dec_mode(8)); // auto_repeat default true
    assert!(modes.get_dec_mode(25)); // cursor_visible default true
}

#[test]
fn test_get_dec_mode_unknown() {
    let modes = Modes::new();
    assert!(!modes.get_dec_mode(9999));
}

#[test]
fn test_set_mode_insert() {
    let mut modes = Modes::new();
    modes.set_mode(4, true);
    assert!(modes.insert_mode);
    modes.set_mode(4, false);
    assert!(!modes.insert_mode);
}

#[test]
fn test_set_mode_linefeed() {
    let mut modes = Modes::new();
    modes.set_mode(20, true);
    assert!(modes.linefeed_mode);
}

#[test]
fn test_mouse_tracking_enabled() {
    let mut modes = Modes::new();
    assert!(!modes.mouse_tracking_enabled());

    modes.mouse_x10 = true;
    assert!(modes.mouse_tracking_enabled());
    modes.mouse_x10 = false;

    modes.mouse_vt200 = true;
    assert!(modes.mouse_tracking_enabled());
    modes.mouse_vt200 = false;

    modes.mouse_button_event = true;
    assert!(modes.mouse_tracking_enabled());
    modes.mouse_button_event = false;

    modes.mouse_any_event = true;
    assert!(modes.mouse_tracking_enabled());
}

#[test]
fn test_modes_reset() {
    let mut modes = Modes::new();
    modes.insert_mode = true;
    modes.origin_mode = true;
    modes.alternate_screen = true;
    modes.bracketed_paste = true;
    modes.mouse_sgr = true;
    modes.reset();
    assert!(!modes.insert_mode);
    assert!(!modes.origin_mode);
    assert!(!modes.alternate_screen);
    assert!(!modes.bracketed_paste);
    assert!(!modes.mouse_sgr);
    assert!(modes.auto_wrap); // defaults to true
    assert!(modes.cursor_visible); // defaults to true
}

#[test]
fn test_set_dec_mode_toggle() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1, true);
    assert!(modes.cursor_keys_application);
    modes.set_dec_mode(1, false);
    assert!(!modes.cursor_keys_application);
    modes.set_dec_mode(1, true);
    assert!(modes.cursor_keys_application);
}

#[test]
fn test_modes_multiple_mouse_modes() {
    let mut modes = Modes::new();
    modes.mouse_vt200 = true;
    modes.mouse_sgr = true;
    assert!(modes.mouse_tracking_enabled());
    assert!(modes.mouse_sgr);
}

#[test]
fn test_set_dec_mode_unknown_is_noop() {
    let mut modes = Modes::new();
    let original = modes.clone();
    modes.set_dec_mode(12345, true);
    // Unknown mode should not change anything that matters
    assert_eq!(modes.insert_mode, original.insert_mode);
    assert_eq!(modes.origin_mode, original.origin_mode);
}

#[test]
fn test_modes_clone() {
    let mut modes = Modes::new();
    modes.insert_mode = true;
    modes.bracketed_paste = true;
    let cloned = modes.clone();
    assert!(cloned.insert_mode);
    assert!(cloned.bracketed_paste);
}
