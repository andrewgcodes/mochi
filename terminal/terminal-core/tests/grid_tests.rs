//! Comprehensive tests for terminal grid

use terminal_core::{CellAttributes, Dimensions, Grid};

// ============================================================
// Grid Creation Tests
// ============================================================

#[test]
fn test_grid_new_dimensions() {
    let grid = Grid::new(Dimensions::new(80, 24));
    assert_eq!(grid.cols(), 80);
    assert_eq!(grid.rows(), 24);
}

#[test]
fn test_grid_new_small() {
    let grid = Grid::new(Dimensions::new(1, 1));
    assert_eq!(grid.cols(), 1);
    assert_eq!(grid.rows(), 1);
}

#[test]
fn test_grid_new_all_empty() {
    let grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        for col in 0..10 {
            assert!(grid.line(row).cell(col).is_empty());
        }
    }
}

#[test]
fn test_grid_dimensions_method() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let dims = grid.dimensions();
    assert_eq!(dims.cols, 80);
    assert_eq!(dims.rows, 24);
}

// ============================================================
// Grid Line Access Tests
// ============================================================

#[test]
fn test_grid_line() {
    let grid = Grid::new(Dimensions::new(10, 5));
    let line = grid.line(0);
    assert_eq!(line.cols(), 10);
}

#[test]
fn test_grid_line_mut() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('X');
    assert_eq!(grid.line(0).cell(0).display_char(), 'X');
}

#[test]
fn test_grid_get_line_valid() {
    let grid = Grid::new(Dimensions::new(10, 5));
    assert!(grid.get_line(0).is_some());
    assert!(grid.get_line(4).is_some());
}

#[test]
fn test_grid_get_line_out_of_bounds() {
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
fn test_grid_get_line_mut_out_of_bounds() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    assert!(grid.get_line_mut(5).is_none());
}

// ============================================================
// Grid Clear Tests
// ============================================================

#[test]
fn test_grid_clear_all() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row).cell_mut(0).set_char('X');
    }
    grid.clear(CellAttributes::default());
    for row in 0..5 {
        assert!(grid.line(row).cell(0).is_empty());
    }
}

#[test]
fn test_grid_clear_with_attrs() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.clear(attrs);
    assert!(grid.line(0).cell(0).attrs.bold);
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
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
    assert_eq!(grid.line(1).cell(0).display_char(), 'B');
    assert!(grid.line(2).cell(0).is_empty());
    assert!(grid.line(3).cell(0).is_empty());
}

#[test]
fn test_grid_clear_below_with_col() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    for col in 0..10 {
        grid.line_mut(1).cell_mut(col).set_char('X');
    }
    grid.clear_below(1, 5, CellAttributes::default());
    // Cells before col 5 on row 1 should remain
    assert_eq!(grid.line(1).cell(0).display_char(), 'X');
    assert_eq!(grid.line(1).cell(4).display_char(), 'X');
    // Cells from col 5 on row 1 should be cleared
    assert!(grid.line(1).cell(5).is_empty());
}

#[test]
fn test_grid_clear_below_out_of_bounds() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.clear_below(10, 0, CellAttributes::default()); // out of bounds
    assert_eq!(grid.line(0).cell(0).display_char(), 'A'); // unchanged
}

#[test]
fn test_grid_clear_above() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.clear_above(2, 0, CellAttributes::default());
    assert!(grid.line(0).cell(0).is_empty());
    assert!(grid.line(1).cell(0).is_empty());
    assert!(grid.line(2).cell(0).is_empty()); // col 0 is cleared (clear_to is inclusive)
    assert_eq!(grid.line(3).cell(0).display_char(), 'D');
}

#[test]
fn test_grid_clear_above_out_of_bounds() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.clear_above(10, 0, CellAttributes::default());
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
}

// ============================================================
// Grid Scroll Up Tests
// ============================================================

#[test]
fn test_grid_scroll_up_returns_scrolled_lines() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    let scrolled = grid.scroll_up(0, 4, 2, CellAttributes::default());
    assert_eq!(scrolled.len(), 2);
    assert_eq!(scrolled[0].cell(0).display_char(), 'A');
    assert_eq!(scrolled[1].cell(0).display_char(), 'B');
}

#[test]
fn test_grid_scroll_up_remaining_content() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.scroll_up(0, 4, 2, CellAttributes::default());
    assert_eq!(grid.line(0).cell(0).display_char(), 'C');
    assert_eq!(grid.line(1).cell(0).display_char(), 'D');
    assert_eq!(grid.line(2).cell(0).display_char(), 'E');
    assert!(grid.line(3).cell(0).is_empty());
    assert!(grid.line(4).cell(0).is_empty());
}

#[test]
fn test_grid_scroll_up_one_line() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    for row in 0..3 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    let scrolled = grid.scroll_up(0, 2, 1, CellAttributes::default());
    assert_eq!(scrolled.len(), 1);
    assert_eq!(grid.line(0).cell(0).display_char(), 'B');
    assert_eq!(grid.line(1).cell(0).display_char(), 'C');
    assert!(grid.line(2).cell(0).is_empty());
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
    assert!(grid.line(3).cell(0).is_empty());
    assert_eq!(grid.line(4).cell(0).display_char(), 'E');
}

#[test]
fn test_grid_scroll_up_invalid_bounds() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    let scrolled = grid.scroll_up(5, 10, 1, CellAttributes::default());
    assert!(scrolled.is_empty());
}

#[test]
fn test_grid_scroll_up_top_gt_bottom() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    let scrolled = grid.scroll_up(3, 1, 1, CellAttributes::default());
    assert!(scrolled.is_empty());
}

#[test]
fn test_grid_scroll_up_clamp_n() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    // n=100 but region is only 5 lines
    let scrolled = grid.scroll_up(0, 4, 100, CellAttributes::default());
    assert_eq!(scrolled.len(), 5);
}

// ============================================================
// Grid Scroll Down Tests
// ============================================================

#[test]
fn test_grid_scroll_down_basic() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.scroll_down(0, 4, 2, CellAttributes::default());
    assert!(grid.line(0).cell(0).is_empty());
    assert!(grid.line(1).cell(0).is_empty());
    assert_eq!(grid.line(2).cell(0).display_char(), 'A');
    assert_eq!(grid.line(3).cell(0).display_char(), 'B');
    assert_eq!(grid.line(4).cell(0).display_char(), 'C');
}

#[test]
fn test_grid_scroll_down_one_line() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    for row in 0..3 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.scroll_down(0, 2, 1, CellAttributes::default());
    assert!(grid.line(0).cell(0).is_empty());
    assert_eq!(grid.line(1).cell(0).display_char(), 'A');
    assert_eq!(grid.line(2).cell(0).display_char(), 'B');
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
    assert!(grid.line(1).cell(0).is_empty());
    assert_eq!(grid.line(2).cell(0).display_char(), 'B');
    assert_eq!(grid.line(3).cell(0).display_char(), 'C');
    assert_eq!(grid.line(4).cell(0).display_char(), 'E');
}

#[test]
fn test_grid_scroll_down_invalid() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.scroll_down(5, 10, 1, CellAttributes::default());
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
}

// ============================================================
// Grid Insert Lines Tests
// ============================================================

#[test]
fn test_grid_insert_lines_basic() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for row in 0..5 {
        grid.line_mut(row)
            .cell_mut(0)
            .set_char((b'A' + row as u8) as char);
    }
    grid.insert_lines(1, 2, 4, CellAttributes::default());
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
    assert!(grid.line(1).cell(0).is_empty());
    assert!(grid.line(2).cell(0).is_empty());
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
    assert!(grid.line(0).cell(0).is_empty());
    assert_eq!(grid.line(1).cell(0).display_char(), 'A');
    assert_eq!(grid.line(2).cell(0).display_char(), 'B');
}

#[test]
fn test_grid_insert_lines_out_of_bounds() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.insert_lines(10, 1, 4, CellAttributes::default());
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
}

// ============================================================
// Grid Delete Lines Tests
// ============================================================

#[test]
fn test_grid_delete_lines_basic() {
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
    assert!(grid.line(3).cell(0).is_empty());
    assert!(grid.line(4).cell(0).is_empty());
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
    assert!(grid.line(2).cell(0).is_empty());
}

#[test]
fn test_grid_delete_lines_out_of_bounds() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.delete_lines(10, 1, 4, CellAttributes::default());
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
}

// ============================================================
// Grid Resize Tests
// ============================================================

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
fn test_grid_resize_same() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.resize(Dimensions::new(10, 5), CellAttributes::default());
    assert_eq!(grid.cols(), 10);
    assert_eq!(grid.rows(), 5);
    assert_eq!(grid.line(0).cell(0).display_char(), 'A');
}

#[test]
fn test_grid_resize_wider_only() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.resize(Dimensions::new(20, 5), CellAttributes::default());
    assert_eq!(grid.cols(), 20);
    assert_eq!(grid.rows(), 5);
    // Line should have been extended
    assert_eq!(grid.line(0).cols(), 20);
}

#[test]
fn test_grid_resize_taller_only() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.resize(Dimensions::new(10, 10), CellAttributes::default());
    assert_eq!(grid.cols(), 10);
    assert_eq!(grid.rows(), 10);
}

// ============================================================
// Grid Iterator Tests
// ============================================================

#[test]
fn test_grid_iter_count() {
    let grid = Grid::new(Dimensions::new(10, 5));
    assert_eq!(grid.iter().count(), 5);
}

#[test]
fn test_grid_iter_mut_modify() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    for line in grid.iter_mut() {
        line.cell_mut(0).set_char('X');
    }
    for row in 0..5 {
        assert_eq!(grid.line(row).cell(0).display_char(), 'X');
    }
}

// ============================================================
// Grid Clone/Eq Tests
// ============================================================

#[test]
fn test_grid_clone() {
    let mut grid = Grid::new(Dimensions::new(10, 5));
    grid.line_mut(0).cell_mut(0).set_char('A');
    let clone = grid.clone();
    assert_eq!(grid, clone);
}

#[test]
fn test_grid_equality() {
    let grid1 = Grid::new(Dimensions::new(10, 5));
    let grid2 = Grid::new(Dimensions::new(10, 5));
    assert_eq!(grid1, grid2);
}

#[test]
fn test_grid_inequality() {
    let mut grid1 = Grid::new(Dimensions::new(10, 5));
    let grid2 = Grid::new(Dimensions::new(10, 5));
    grid1.line_mut(0).cell_mut(0).set_char('A');
    assert_ne!(grid1, grid2);
}
