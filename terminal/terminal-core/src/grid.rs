//! Terminal grid - the visible screen area
//!
//! The grid is a 2D array of cells representing the visible terminal area.

use serde::{Deserialize, Serialize};

use crate::cell::CellAttributes;
use crate::line::Line;
use crate::Dimensions;

/// The terminal grid (visible screen area)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grid {
    /// Lines in the grid (row 0 is top)
    lines: Vec<Line>,
    /// Number of columns
    cols: usize,
    /// Number of rows
    rows: usize,
}

impl Grid {
    /// Create a new grid with the specified dimensions
    pub fn new(dims: Dimensions) -> Self {
        let lines = (0..dims.rows).map(|_| Line::new(dims.cols)).collect();
        Self {
            lines,
            cols: dims.cols,
            rows: dims.rows,
        }
    }

    /// Get grid dimensions
    pub fn dimensions(&self) -> Dimensions {
        Dimensions {
            cols: self.cols,
            rows: self.rows,
        }
    }

    /// Get number of columns
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Get number of rows
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Get a reference to a line
    pub fn line(&self, row: usize) -> &Line {
        &self.lines[row]
    }

    /// Get a mutable reference to a line
    pub fn line_mut(&mut self, row: usize) -> &mut Line {
        &mut self.lines[row]
    }

    /// Get a line, returning None if out of bounds
    pub fn get_line(&self, row: usize) -> Option<&Line> {
        self.lines.get(row)
    }

    /// Get a mutable line, returning None if out of bounds
    pub fn get_line_mut(&mut self, row: usize) -> Option<&mut Line> {
        self.lines.get_mut(row)
    }

    /// Clear the entire grid
    pub fn clear(&mut self, attrs: CellAttributes) {
        for line in &mut self.lines {
            line.clear(attrs);
        }
    }

    /// Clear from cursor position to end of screen
    pub fn clear_below(&mut self, row: usize, col: usize, attrs: CellAttributes) {
        if row >= self.rows {
            return;
        }

        // Clear from cursor to end of current line
        self.lines[row].clear_from(col, attrs);

        // Clear all lines below
        for line in self.lines.iter_mut().skip(row + 1) {
            line.clear(attrs);
        }
    }

    /// Clear from start of screen to cursor position
    pub fn clear_above(&mut self, row: usize, col: usize, attrs: CellAttributes) {
        if row >= self.rows {
            return;
        }

        // Clear all lines above
        for line in self.lines.iter_mut().take(row) {
            line.clear(attrs);
        }

        // Clear from start of current line to cursor
        self.lines[row].clear_to(col, attrs);
    }

    /// Scroll up: move lines up, add blank line at bottom
    /// Lines that scroll off the top are returned for scrollback
    /// Only scrolls within the specified region (top..=bottom)
    pub fn scroll_up(
        &mut self,
        top: usize,
        bottom: usize,
        n: usize,
        attrs: CellAttributes,
    ) -> Vec<Line> {
        if top >= self.rows || bottom >= self.rows || top > bottom {
            return Vec::new();
        }

        let n = n.min(bottom - top + 1);
        let mut scrolled_out = Vec::with_capacity(n);

        for _ in 0..n {
            // Remove line from top of region
            let line = self.lines.remove(top);
            scrolled_out.push(line);

            // Insert blank line at bottom of region
            self.lines
                .insert(bottom, Line::with_attrs(self.cols, attrs));
        }

        scrolled_out
    }

    /// Scroll down: move lines down, add blank line at top
    /// Only scrolls within the specified region (top..=bottom)
    pub fn scroll_down(&mut self, top: usize, bottom: usize, n: usize, attrs: CellAttributes) {
        if top >= self.rows || bottom >= self.rows || top > bottom {
            return;
        }

        let n = n.min(bottom - top + 1);

        for _ in 0..n {
            // Remove line from bottom of region
            self.lines.remove(bottom);

            // Insert blank line at top of region
            self.lines.insert(top, Line::with_attrs(self.cols, attrs));
        }
    }

    /// Insert n blank lines at row, scrolling lines down
    /// Lines that scroll past bottom of region are lost
    pub fn insert_lines(&mut self, row: usize, n: usize, bottom: usize, attrs: CellAttributes) {
        if row >= self.rows || row > bottom {
            return;
        }

        let n = n.min(bottom - row + 1);

        for _ in 0..n {
            // Remove line from bottom of region
            if bottom < self.lines.len() {
                self.lines.remove(bottom);
            }

            // Insert blank line at row
            self.lines.insert(row, Line::with_attrs(self.cols, attrs));
        }
    }

    /// Delete n lines at row, scrolling lines up
    /// Blank lines are added at bottom of region
    pub fn delete_lines(&mut self, row: usize, n: usize, bottom: usize, attrs: CellAttributes) {
        if row >= self.rows || row > bottom {
            return;
        }

        let n = n.min(bottom - row + 1);

        for _ in 0..n {
            // Remove line at row
            if row < self.lines.len() {
                self.lines.remove(row);
            }

            // Insert blank line at bottom of region
            let insert_pos = bottom.min(self.lines.len());
            self.lines
                .insert(insert_pos, Line::with_attrs(self.cols, attrs));
        }
    }

    /// Resize the grid to new dimensions
    pub fn resize(&mut self, dims: Dimensions, attrs: CellAttributes) {
        // Resize existing lines
        for line in &mut self.lines {
            line.resize(dims.cols, attrs);
        }

        // Add or remove rows
        if dims.rows > self.rows {
            for _ in self.rows..dims.rows {
                self.lines.push(Line::with_attrs(dims.cols, attrs));
            }
        } else {
            self.lines.truncate(dims.rows);
        }

        self.cols = dims.cols;
        self.rows = dims.rows;
    }

    /// Iterator over lines
    pub fn iter(&self) -> impl Iterator<Item = &Line> {
        self.lines.iter()
    }

    /// Mutable iterator over lines
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Line> {
        self.lines.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_new() {
        let grid = Grid::new(Dimensions::new(80, 24));
        assert_eq!(grid.cols(), 80);
        assert_eq!(grid.rows(), 24);
    }

    #[test]
    fn test_grid_clear() {
        let mut grid = Grid::new(Dimensions::new(10, 5));
        grid.line_mut(0).cell_mut(0).set_char('A');
        grid.clear(CellAttributes::default());
        assert!(grid.line(0).cell(0).is_empty());
    }

    #[test]
    fn test_grid_scroll_up() {
        let mut grid = Grid::new(Dimensions::new(10, 5));
        for row in 0..5 {
            grid.line_mut(row)
                .cell_mut(0)
                .set_char((b'A' + row as u8) as char);
        }
        // Grid is: A, B, C, D, E

        let scrolled = grid.scroll_up(0, 4, 2, CellAttributes::default());

        // Should be: C, D, E, _, _
        assert_eq!(scrolled.len(), 2);
        assert_eq!(scrolled[0].cell(0).display_char(), 'A');
        assert_eq!(scrolled[1].cell(0).display_char(), 'B');
        assert_eq!(grid.line(0).cell(0).display_char(), 'C');
        assert_eq!(grid.line(1).cell(0).display_char(), 'D');
        assert_eq!(grid.line(2).cell(0).display_char(), 'E');
        assert!(grid.line(3).cell(0).is_empty());
        assert!(grid.line(4).cell(0).is_empty());
    }

    #[test]
    fn test_grid_scroll_down() {
        let mut grid = Grid::new(Dimensions::new(10, 5));
        for row in 0..5 {
            grid.line_mut(row)
                .cell_mut(0)
                .set_char((b'A' + row as u8) as char);
        }
        // Grid is: A, B, C, D, E

        grid.scroll_down(0, 4, 2, CellAttributes::default());

        // Should be: _, _, A, B, C
        assert!(grid.line(0).cell(0).is_empty());
        assert!(grid.line(1).cell(0).is_empty());
        assert_eq!(grid.line(2).cell(0).display_char(), 'A');
        assert_eq!(grid.line(3).cell(0).display_char(), 'B');
        assert_eq!(grid.line(4).cell(0).display_char(), 'C');
    }

    #[test]
    fn test_grid_scroll_region() {
        let mut grid = Grid::new(Dimensions::new(10, 5));
        for row in 0..5 {
            grid.line_mut(row)
                .cell_mut(0)
                .set_char((b'A' + row as u8) as char);
        }
        // Grid is: A, B, C, D, E

        // Scroll only middle region (rows 1-3)
        grid.scroll_up(1, 3, 1, CellAttributes::default());

        // Should be: A, C, D, _, E
        assert_eq!(grid.line(0).cell(0).display_char(), 'A');
        assert_eq!(grid.line(1).cell(0).display_char(), 'C');
        assert_eq!(grid.line(2).cell(0).display_char(), 'D');
        assert!(grid.line(3).cell(0).is_empty());
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
        // Grid is: A, B, C, D, E

        grid.insert_lines(1, 2, 4, CellAttributes::default());

        // Should be: A, _, _, B, C (D, E pushed out)
        assert_eq!(grid.line(0).cell(0).display_char(), 'A');
        assert!(grid.line(1).cell(0).is_empty());
        assert!(grid.line(2).cell(0).is_empty());
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
        // Grid is: A, B, C, D, E

        grid.delete_lines(1, 2, 4, CellAttributes::default());

        // Should be: A, D, E, _, _
        assert_eq!(grid.line(0).cell(0).display_char(), 'A');
        assert_eq!(grid.line(1).cell(0).display_char(), 'D');
        assert_eq!(grid.line(2).cell(0).display_char(), 'E');
        assert!(grid.line(3).cell(0).is_empty());
        assert!(grid.line(4).cell(0).is_empty());
    }

    #[test]
    fn test_grid_resize() {
        let mut grid = Grid::new(Dimensions::new(10, 5));
        grid.line_mut(0).cell_mut(0).set_char('A');

        grid.resize(Dimensions::new(20, 10), CellAttributes::default());

        assert_eq!(grid.cols(), 20);
        assert_eq!(grid.rows(), 10);
        assert_eq!(grid.line(0).cell(0).display_char(), 'A');
    }
}
