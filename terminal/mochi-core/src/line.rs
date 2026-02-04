//! Terminal line representation
//!
//! A line is a row of cells in the terminal grid.
//! Lines track whether they were soft-wrapped (for proper selection/copy).

use serde::{Deserialize, Serialize};

use crate::cell::Cell;

/// A single line in the terminal grid
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    /// Cells in this line
    cells: Vec<Cell>,
    /// Whether this line was soft-wrapped from the previous line
    pub wrapped: bool,
}

impl Line {
    /// Create a new line with the given number of columns
    pub fn new(cols: usize) -> Self {
        Line {
            cells: vec![Cell::default(); cols],
            wrapped: false,
        }
    }

    /// Get the number of columns in this line
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Check if the line is empty (all default cells)
    pub fn is_empty(&self) -> bool {
        self.cells.iter().all(|c| c.is_empty())
    }

    /// Get a reference to a cell
    pub fn get(&self, col: usize) -> Option<&Cell> {
        self.cells.get(col)
    }

    /// Get a mutable reference to a cell
    pub fn get_mut(&mut self, col: usize) -> Option<&mut Cell> {
        self.cells.get_mut(col)
    }

    /// Get a reference to a cell, panicking if out of bounds
    pub fn cell(&self, col: usize) -> &Cell {
        &self.cells[col]
    }

    /// Get a mutable reference to a cell, panicking if out of bounds
    pub fn cell_mut(&mut self, col: usize) -> &mut Cell {
        &mut self.cells[col]
    }

    /// Resize the line to a new number of columns
    pub fn resize(&mut self, cols: usize) {
        self.cells.resize(cols, Cell::default());
    }

    /// Clear all cells in the line
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.reset();
        }
        self.wrapped = false;
    }

    /// Clear cells from start_col to end of line
    pub fn clear_from(&mut self, start_col: usize) {
        for col in start_col..self.cells.len() {
            self.cells[col].reset();
        }
    }

    /// Clear cells from start of line to end_col (inclusive)
    pub fn clear_to(&mut self, end_col: usize) {
        let end = (end_col + 1).min(self.cells.len());
        for col in 0..end {
            self.cells[col].reset();
        }
    }

    /// Clear cells in range [start_col, end_col)
    pub fn clear_range(&mut self, start_col: usize, end_col: usize) {
        let start = start_col.min(self.cells.len());
        let end = end_col.min(self.cells.len());
        for col in start..end {
            self.cells[col].reset();
        }
    }

    /// Insert n blank cells at position, shifting cells right
    /// Cells that shift past the end are lost
    pub fn insert_cells(&mut self, col: usize, n: usize) {
        if col >= self.cells.len() {
            return;
        }
        let cols = self.cells.len();
        // Shift cells right
        for i in (col..cols.saturating_sub(n)).rev() {
            self.cells[i + n] = self.cells[i].clone();
        }
        // Clear inserted cells
        for i in col..(col + n).min(cols) {
            self.cells[i].reset();
        }
    }

    /// Delete n cells at position, shifting cells left
    /// Blank cells are inserted at the end
    pub fn delete_cells(&mut self, col: usize, n: usize) {
        if col >= self.cells.len() {
            return;
        }
        let cols = self.cells.len();
        let n = n.min(cols - col);
        // Shift cells left
        for i in col..(cols - n) {
            self.cells[i] = self.cells[i + n].clone();
        }
        // Clear cells at end
        for i in (cols - n)..cols {
            self.cells[i].reset();
        }
    }

    /// Erase n cells at position (replace with blanks, no shifting)
    pub fn erase_cells(&mut self, col: usize, n: usize) {
        let end = (col + n).min(self.cells.len());
        for i in col..end {
            self.cells[i].reset();
        }
    }

    /// Get iterator over cells
    pub fn iter(&self) -> impl Iterator<Item = &Cell> {
        self.cells.iter()
    }

    /// Get mutable iterator over cells
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Cell> {
        self.cells.iter_mut()
    }

    /// Get the text content of this line (for selection/copy)
    pub fn text(&self) -> String {
        let mut s = String::new();
        for cell in &self.cells {
            s.push_str(&cell.c);
        }
        // Trim trailing spaces
        s.trim_end().to_string()
    }

    /// Get text content in a range of columns
    pub fn text_range(&self, start: usize, end: usize) -> String {
        let start = start.min(self.cells.len());
        let end = end.min(self.cells.len());
        let mut s = String::new();
        for i in start..end {
            s.push_str(&self.cells[i].c);
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_new() {
        let line = Line::new(80);
        assert_eq!(line.len(), 80);
        assert!(line.is_empty());
        assert!(!line.wrapped);
    }

    #[test]
    fn test_line_cell_access() {
        let mut line = Line::new(80);
        line.cell_mut(0).c = "A".to_string();
        assert_eq!(line.cell(0).c, "A");
        assert!(!line.is_empty());
    }

    #[test]
    fn test_line_clear() {
        let mut line = Line::new(80);
        line.cell_mut(0).c = "A".to_string();
        line.cell_mut(10).c = "B".to_string();
        line.wrapped = true;

        line.clear();
        assert!(line.is_empty());
        assert!(!line.wrapped);
    }

    #[test]
    fn test_line_clear_from() {
        let mut line = Line::new(10);
        for i in 0..10 {
            line.cell_mut(i).c = format!("{}", i);
        }

        line.clear_from(5);
        assert_eq!(line.cell(4).c, "4");
        assert_eq!(line.cell(5).c, " ");
        assert_eq!(line.cell(9).c, " ");
    }

    #[test]
    fn test_line_insert_cells() {
        let mut line = Line::new(10);
        for i in 0..10 {
            line.cell_mut(i).c = format!("{}", i);
        }

        line.insert_cells(2, 3);
        assert_eq!(line.cell(0).c, "0");
        assert_eq!(line.cell(1).c, "1");
        assert_eq!(line.cell(2).c, " "); // inserted
        assert_eq!(line.cell(3).c, " "); // inserted
        assert_eq!(line.cell(4).c, " "); // inserted
        assert_eq!(line.cell(5).c, "2"); // shifted
        assert_eq!(line.cell(6).c, "3"); // shifted
    }

    #[test]
    fn test_line_delete_cells() {
        let mut line = Line::new(10);
        for i in 0..10 {
            line.cell_mut(i).c = format!("{}", i);
        }

        line.delete_cells(2, 3);
        assert_eq!(line.cell(0).c, "0");
        assert_eq!(line.cell(1).c, "1");
        assert_eq!(line.cell(2).c, "5"); // shifted
        assert_eq!(line.cell(3).c, "6"); // shifted
        assert_eq!(line.cell(7).c, " "); // cleared
        assert_eq!(line.cell(8).c, " "); // cleared
        assert_eq!(line.cell(9).c, " "); // cleared
    }

    #[test]
    fn test_line_text() {
        let mut line = Line::new(10);
        line.cell_mut(0).c = "H".to_string();
        line.cell_mut(1).c = "e".to_string();
        line.cell_mut(2).c = "l".to_string();
        line.cell_mut(3).c = "l".to_string();
        line.cell_mut(4).c = "o".to_string();

        assert_eq!(line.text(), "Hello");
    }
}
