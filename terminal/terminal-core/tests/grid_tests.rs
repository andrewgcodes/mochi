//! Comprehensive tests for terminal grid operations

use terminal_core::{CellAttributes, Color, Dimensions, Grid};

// ============================================================================
// Grid Creation
// ============================================================================

#[test]
fn test_grid_new_dimensions() {
    let grid = Grid::new(Dimensions::new(80, 24));
    assert_eq!(grid.cols(), 80);
    assert_eq!(grid.rows(), 24);
}

#[test]
fn test_grid_new_dimensions_struct() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let dims = grid.dimensions();
    assert_eq!(dims.cols, 80);
    assert_eq!(dims.rows, 24);
}

#[test]
fn test_grid_new_small() {
    let grid = Grid::new(Dimensions::new(1, 1));
    assert_eq!(grid.cols(), 1);
    assert_eq!(grid.rows(), 1);
}

#[test]
fn test_grid_new_wide() {
    let grid = Grid::new(Dimensions::new(300, 5));
    assert_eq!(grid.cols(), 300);
    assert_eq!(grid.rows(), 5);
}

#[test]
fn test_grid_new_all_empty() {
    let grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        assert!(grid.line(row).is_empty());
    }
}

// ============================================================================
// Grid Line Access
// ============================================================================

#[test]
fn test_grid_line_access() {
    let grid = Grid::new(Dimensions::new(10, 5));
    let line = grid.line(0);
    assert_eq!(line.cols(), 10);
}

#[test]
fn test_grid_line_mut_modify() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
}

#[test]
fn test_grid_get_line_valid() {
    let grid = Grid::new(Dimensions::new(10, 5));
    assert!(grid.get_line(0).is_some());
    assert!(grid.get_line(4).is_some());
}

#[test]
fn test_grid_get_line_invalid() {
    let grid = Grid::new(Dimensions::new(10, 5));
    assert!(grid.get_line(5).is_none());
    assert!(grid.get_line(100).is_none());
}

#[test]
fn test_grid_get_line_mut_valid() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    assert!(grid.get_line_mut(0).is_some());
}

#[test]
fn test_grid_get_line_mut_invalid() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    assert!(grid.get_line_mut(5).is_none());
}

// ============================================================================
// Grid::clear
// ============================================================================

#[test]
fn test_grid_clear_all() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.line_mut(4).cell_mut(9).set_char('Z');
    grid.clear(CellAttributes::default());
    assert!(grid.line(0).cell(0).is_empty());
    assert!(grid.line(4).cell(9).is_empty());
}

#[test]
fn test_grid_clear_with_attrs() {
    let mut attrs = CellAttributes::new();
    attrs.bg = Color::Indexed(1);
    let mut grid = Grid::new(Dimensions::new(5, 3));
    grid.clear(attrs);
    assert_eq!(grid.line(0).cell(0).attrs.bg, Color::Indexed(1));
}

// ============================================================================
// Grid::clear_below
// ============================================================================

#[test]
fn test_grid_clear_below() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.clear_below(2, 5, CellAttributes::default());
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
    assert_eq!(grid.line(1).cell(0).display_char(), 'B');
    // Row 2: cleared from col 5 onward
    assert_eq!(grid.line(2).cell(0).display_char(), 'C');
    // Rows 3-4: fully cleared
    assert!(grid.line(3).is_empty());
    assert!(grid.line(4).is_empty());
}

#[test]
fn test_grid_clear_below_from_start() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    for row in 0..3 {
        grid.line_mut(row).cell_mut(0).set_char('X');
    }
    grid.clear_below(0, 0, CellAttributes::default());
    assert!(grid.line(1).is_empty());
    assert!(grid.line(2).is_empty());
}

#[test]
fn test_grid_clear_below_out_of_bounds() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.clear_below(10, 0, CellAttributes::default()); // Out of bounds - no-op
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
}

// ============================================================================
// Grid::clear_above
// ============================================================================

#[test]
fn test_grid_clear_above() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.clear_above(2, 3, CellAttributes::default());
    assert!(grid.line(0).is_empty());
    assert!(grid.line(1).is_empty());
    // Row 2: cleared from start to col 3
    assert!(grid.line(2).cell(0).is_empty());
    assert_eq!(grid.line(3).cell(0).display_char(), 'D');
    assert_eq!(grid.line(4).cell(0).display_char(), 'E');
}

#[test]
fn test_grid_clear_above_out_of_bounds() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.clear_above(10, 0, CellAttributes::default()); // Out of bounds - no-op
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
}

// ============================================================================
// Grid::scroll_up
// ============================================================================

#[test]
fn test_grid_scroll_up_full() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    let scrolled = grid.scroll_up(0, 4, 1, CellAttributes::default());
    assert_eq!(scrolled.len(), 1);
    assert_eq!(scrolled[0].cell(0).display_char(), 'A');
    assert_eq!(grid.line(0).cell(0).display_char(), 'B');
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
fn test_grid_scroll_up_region() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.scroll_up(1, 3, 1, CellAttributes::default());
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
    assert_eq!(grid.line(1).cell(0).display_char(), 'C');
    assert_eq!(grid.line(2).cell(0).display_char(), 'D');
    assert!(grid.line(3).is_empty());
    assert_eq!(grid.line(4).cell(0).display_char(), 'E');
}

#[test]
fn test_grid_scroll_up_invalid_region() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    let scrolled = grid.scroll_up(3, 1, 1, CellAttributes::default()); // top > bottom
    assert!(scrolled.is_empty());
}

#[test]
fn test_grid_scroll_up_n_exceeds_region() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    let scrolled = grid.scroll_up(0, 4, 100, CellAttributes::default());
    assert_eq!(scrolled.len(), 5); // Clamped to region size
}

// ============================================================================
// Grid::scroll_down
// ============================================================================

#[test]
fn test_grid_scroll_down_full() {
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
fn test_grid_scroll_down_region() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.scroll_down(1, 3, 1, CellAttributes::default());
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
    assert!(grid.line(1).is_empty());
    assert_eq!(grid.line(2).cell(0).display_char(), 'B');
    assert_eq!(grid.line(3).cell(0).display_char(), 'C');
    assert_eq!(grid.line(4).cell(0).display_char(), 'E');
}

#[test]
fn test_grid_scroll_down_invalid_region() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.scroll_down(3, 1, 1, CellAttributes::default()); // top > bottom - no-op
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
}

// ============================================================================
// Grid::insert_lines
// ============================================================================

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
fn test_grid_insert_lines_at_top() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    for row in 0..3 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.insert_lines(0, 1, 2, CellAttributes::default());
    assert!(grid.line(0).is_empty());
    assert_eq!(grid.line(1).cell(0).display_char(), 'A');
    assert_eq!(grid.line(2).cell(0).display_char(), 'B');
}

#[test]
fn test_grid_insert_lines_out_of_bounds() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.insert_lines(10, 1, 4, CellAttributes::default()); // Out of bounds - no-op
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
}

// ============================================================================
// Grid::delete_lines
// ============================================================================

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
fn test_grid_delete_lines_at_top() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    for row in 0..3 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.delete_lines(0, 1, 2, CellAttributes::default());
    assert_eq!(grid.line(0).cell(0).display_char(), 'B');
    assert_eq!(grid.line(1).cell(0).display_char(), 'C');
    assert!(grid.line(2).is_empty());
}

#[test]
fn test_grid_delete_lines_out_of_bounds() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.delete_lines(10, 1, 4, CellAttributes::default()); // No-op
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
}

// ============================================================================
// Grid::resize
// ============================================================================

#[test]
fn test_grid_resize_grow_cols() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.resize(Dimensions::new(20, 5), CellAttributes::default());
    assert_eq!(grid.cols(), 20);
    assert_eq!(grid.rows(), 5);
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
}

#[test]
fn test_grid_resize_grow_rows() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.resize(Dimensions::new(10, 10), CellAttributes::default());
    assert_eq!(grid.cols(), 10);
    assert_eq!(grid.rows(), 10);
    assert!(grid.line(9).is_empty());
}

#[test]
fn test_grid_resize_shrink_cols() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.resize(Dimensions::new(5, 5), CellAttributes::default());
    assert_eq!(grid.cols(), 5);
}

#[test]
fn test_grid_resize_shrink_rows() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.resize(Dimensions::new(10, 3), CellAttributes::default());
    assert_eq!(grid.rows(), 3);
}

#[test]
fn test_grid_resize_both() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.resize(Dimensions::new(20, 10), CellAttributes::default());
    assert_eq!(grid.cols(), 20);
    assert_eq!(grid.rows(), 10);
}

#[test]
fn test_grid_resize_preserves_content() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.line_mut(0).cell_mut(1).set_char('B');
    grid.resize(Dimensions::new(20, 10), CellAttributes::default());
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
    assert_eq!(grid.line(0).cell(1).display_char(), 'B');
}

// ============================================================================
// Grid iteration
// ============================================================================

#[test]
fn test_grid_iter_count() {
    let grid = Grid::new(Dimensions::new(10, 5));
    assert_eq!(grid.iter().count(), 5);
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
