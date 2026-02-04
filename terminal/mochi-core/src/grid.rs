//! Terminal grid representation
//!
//! The grid is a 2D array of cells representing the visible terminal area.
//! It supports operations like scrolling, inserting/deleting lines, and resizing.

use serde::{Deserialize, Serialize};

use crate::cell::Cell;
use crate::line::Line;

/// A 2D grid of terminal cells
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grid {
    /// Lines in the grid (row 0 is at the top)
    lines: Vec<Line>,
    /// Number of columns
    cols: usize,
    /// Number of rows
    rows: usize,
}

impl Grid {
    /// Create a new grid with the given dimensions
    pub fn new(rows: usize, cols: usize) -> Self {
        let lines = (0..rows).map(|_| Line::new(cols)).collect();
        Grid { lines, cols, rows }
    }

    /// Get the number of rows
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Get the number of columns
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Get a reference to a line
    pub fn line(&self, row: usize) -> &Line {
        &self.lines[row]
    }

    /// Get a mutable reference to a line
    pub fn line_mut(&mut self, row: usize) -> &mut Line {
        &mut self.lines[row]
    }

    /// Get a reference to a cell
    pub fn cell(&self, row: usize, col: usize) -> &Cell {
        self.lines[row].cell(col)
    }

    /// Get a mutable reference to a cell
    pub fn cell_mut(&mut self, row: usize, col: usize) -> &mut Cell {
        self.lines[row].cell_mut(col)
    }

    /// Resize the grid
    /// New cells are filled with defaults, excess cells are truncated
    pub fn resize(&mut self, rows: usize, cols: usize) {
        // Resize existing lines
        for line in &mut self.lines {
            line.resize(cols);
        }

        // Add or remove lines
        if rows > self.rows {
            for _ in self.rows..rows {
                self.lines.push(Line::new(cols));
            }
        } else if rows < self.rows {
            self.lines.truncate(rows);
        }

        self.rows = rows;
        self.cols = cols;
    }

    /// Clear the entire grid
    pub fn clear(&mut self) {
        for line in &mut self.lines {
            line.clear();
        }
    }

    /// Clear all cells from cursor position to end of screen
    pub fn clear_below(&mut self, row: usize, col: usize) {
        if row >= self.rows {
            return;
        }
        // Clear from cursor to end of current line
        self.lines[row].clear_from(col);
        // Clear all lines below
        for r in (row + 1)..self.rows {
            self.lines[r].clear();
        }
    }

    /// Clear all cells from start of screen to cursor position
    pub fn clear_above(&mut self, row: usize, col: usize) {
        // Clear all lines above
        for r in 0..row {
            self.lines[r].clear();
        }
        // Clear from start of line to cursor
        if row < self.rows {
            self.lines[row].clear_to(col);
        }
    }

    /// Scroll the region [top, bottom] up by n lines
    /// Lines scrolled off the top are returned (for scrollback)
    /// New blank lines appear at the bottom
    pub fn scroll_up(&mut self, top: usize, bottom: usize, n: usize) -> Vec<Line> {
        if top >= bottom || top >= self.rows || n == 0 {
            return Vec::new();
        }
        let bottom = bottom.min(self.rows - 1);
        let n = n.min(bottom - top + 1);

        // Collect lines that will be scrolled off
        let mut scrolled_off = Vec::with_capacity(n);
        for i in 0..n {
            scrolled_off.push(std::mem::replace(&mut self.lines[top + i], Line::new(self.cols)));
        }

        // Shift lines up
        for i in top..(bottom + 1 - n) {
            self.lines.swap(i, i + n);
        }

        // Clear the new lines at the bottom of the region
        for i in (bottom + 1 - n)..=bottom {
            self.lines[i].clear();
        }

        scrolled_off
    }

    /// Scroll the region [top, bottom] down by n lines
    /// Lines scrolled off the bottom are lost
    /// New blank lines appear at the top
    pub fn scroll_down(&mut self, top: usize, bottom: usize, n: usize) {
        if top >= bottom || top >= self.rows || n == 0 {
            return;
        }
        let bottom = bottom.min(self.rows - 1);
        let n = n.min(bottom - top + 1);

        // Shift lines down
        for i in ((top + n)..=bottom).rev() {
            self.lines.swap(i, i - n);
        }

        // Clear the new lines at the top of the region
        for i in top..(top + n) {
            self.lines[i].clear();
        }
    }

    /// Insert n blank lines at row, shifting lines down within [row, bottom]
    /// Lines shifted past bottom are lost
    pub fn insert_lines(&mut self, row: usize, bottom: usize, n: usize) {
        if row > bottom || row >= self.rows {
            return;
        }
        self.scroll_down(row, bottom, n);
    }

    /// Delete n lines at row, shifting lines up within [row, bottom]
    /// New blank lines appear at bottom
    pub fn delete_lines(&mut self, row: usize, bottom: usize, n: usize) {
        if row > bottom || row >= self.rows {
            return;
        }
        // We don't need the scrolled off lines here
        let _ = self.scroll_up(row, bottom, n);
    }

    /// Get iterator over lines
    pub fn iter(&self) -> impl Iterator<Item = &Line> {
        self.lines.iter()
    }

    /// Get mutable iterator over lines
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Line> {
        self.lines.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_new() {
        let grid = Grid::new(24, 80);
        assert_eq!(grid.rows(), 24);
        assert_eq!(grid.cols(), 80);
    }

    #[test]
    fn test_grid_cell_access() {
        let mut grid = Grid::new(24, 80);
        grid.cell_mut(0, 0).c = "A".to_string();
        assert_eq!(grid.cell(0, 0).c, "A");
    }

    #[test]
    fn test_grid_resize_grow() {
        let mut grid = Grid::new(10, 10);
        grid.cell_mut(5, 5).c = "X".to_string();

        grid.resize(20, 20);
        assert_eq!(grid.rows(), 20);
        assert_eq!(grid.cols(), 20);
        assert_eq!(grid.cell(5, 5).c, "X");
    }

    #[test]
    fn test_grid_resize_shrink() {
        let mut grid = Grid::new(20, 20);
        grid.cell_mut(5, 5).c = "X".to_string();
        grid.cell_mut(15, 15).c = "Y".to_string();

        grid.resize(10, 10);
        assert_eq!(grid.rows(), 10);
        assert_eq!(grid.cols(), 10);
        assert_eq!(grid.cell(5, 5).c, "X");
        // Cell at 15,15 is gone
    }

    #[test]
    fn test_grid_scroll_up() {
        let mut grid = Grid::new(5, 10);
        for i in 0..5 {
            grid.cell_mut(i, 0).c = format!("{}", i);
        }

        let scrolled = grid.scroll_up(0, 4, 2);
        assert_eq!(scrolled.len(), 2);
        assert_eq!(scrolled[0].cell(0).c, "0");
        assert_eq!(scrolled[1].cell(0).c, "1");

        assert_eq!(grid.cell(0, 0).c, "2");
        assert_eq!(grid.cell(1, 0).c, "3");
        assert_eq!(grid.cell(2, 0).c, "4");
        assert_eq!(grid.cell(3, 0).c, " ");
        assert_eq!(grid.cell(4, 0).c, " ");
    }

    #[test]
    fn test_grid_scroll_down() {
        let mut grid = Grid::new(5, 10);
        for i in 0..5 {
            grid.cell_mut(i, 0).c = format!("{}", i);
        }

        grid.scroll_down(0, 4, 2);

        assert_eq!(grid.cell(0, 0).c, " ");
        assert_eq!(grid.cell(1, 0).c, " ");
        assert_eq!(grid.cell(2, 0).c, "0");
        assert_eq!(grid.cell(3, 0).c, "1");
        assert_eq!(grid.cell(4, 0).c, "2");
    }

    #[test]
    fn test_grid_scroll_region() {
        let mut grid = Grid::new(5, 10);
        for i in 0..5 {
            grid.cell_mut(i, 0).c = format!("{}", i);
        }

        // Scroll only middle region
        grid.scroll_up(1, 3, 1);

        assert_eq!(grid.cell(0, 0).c, "0"); // unchanged
        assert_eq!(grid.cell(1, 0).c, "2"); // shifted up
        assert_eq!(grid.cell(2, 0).c, "3"); // shifted up
        assert_eq!(grid.cell(3, 0).c, " "); // new blank
        assert_eq!(grid.cell(4, 0).c, "4"); // unchanged
    }

    #[test]
    fn test_grid_clear_below() {
        let mut grid = Grid::new(5, 10);
        for i in 0..5 {
            for j in 0..10 {
                grid.cell_mut(i, j).c = "X".to_string();
            }
        }

        grid.clear_below(2, 5);

        // Row 0 and 1 unchanged
        assert_eq!(grid.cell(0, 0).c, "X");
        assert_eq!(grid.cell(1, 9).c, "X");
        // Row 2: cols 0-4 unchanged, 5-9 cleared
        assert_eq!(grid.cell(2, 4).c, "X");
        assert_eq!(grid.cell(2, 5).c, " ");
        // Rows 3-4 cleared
        assert_eq!(grid.cell(3, 0).c, " ");
        assert_eq!(grid.cell(4, 9).c, " ");
    }
}
