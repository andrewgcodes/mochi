//! Terminal line representation
//!
//! A line is a row of cells in the terminal grid.

use serde::{Deserialize, Serialize};

use crate::cell::{Cell, CellAttributes};

/// A single line in the terminal grid
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    /// Cells in this line
    cells: Vec<Cell>,
    /// Whether this line was soft-wrapped (continuation of previous line)
    pub wrapped: bool,
}

impl Line {
    /// Create a new line with the specified number of columns
    pub fn new(cols: usize) -> Self {
        Self {
            cells: vec![Cell::new(); cols],
            wrapped: false,
        }
    }

    /// Create a new line with specified columns and attributes
    pub fn with_attrs(cols: usize, attrs: CellAttributes) -> Self {
        let cells = (0..cols)
            .map(|_| {
                let mut cell = Cell::new();
                cell.attrs = attrs;
                cell
            })
            .collect();
        Self {
            cells,
            wrapped: false,
        }
    }

    /// Get the number of columns
    pub fn cols(&self) -> usize {
        self.cells.len()
    }

    /// Get a reference to a cell
    pub fn get(&self, col: usize) -> Option<&Cell> {
        self.cells.get(col)
    }

    /// Get a mutable reference to a cell
    pub fn get_mut(&mut self, col: usize) -> Option<&mut Cell> {
        self.cells.get_mut(col)
    }

    /// Get cell at column, panics if out of bounds
    pub fn cell(&self, col: usize) -> &Cell {
        &self.cells[col]
    }

    /// Get mutable cell at column, panics if out of bounds
    pub fn cell_mut(&mut self, col: usize) -> &mut Cell {
        &mut self.cells[col]
    }

    /// Clear the entire line with given attributes
    pub fn clear(&mut self, attrs: CellAttributes) {
        for cell in &mut self.cells {
            cell.clear(attrs);
        }
        self.wrapped = false;
    }

    /// Clear from column to end of line
    pub fn clear_from(&mut self, col: usize, attrs: CellAttributes) {
        for cell in self.cells.iter_mut().skip(col) {
            cell.clear(attrs);
        }
    }

    /// Clear from start of line to column (inclusive)
    pub fn clear_to(&mut self, col: usize, attrs: CellAttributes) {
        for cell in self.cells.iter_mut().take(col + 1) {
            cell.clear(attrs);
        }
    }

    /// Resize the line to a new column count
    pub fn resize(&mut self, cols: usize, attrs: CellAttributes) {
        if cols > self.cells.len() {
            // Extend with empty cells
            self.cells.resize_with(cols, || {
                let mut cell = Cell::new();
                cell.attrs = attrs;
                cell
            });
        } else {
            self.cells.truncate(cols);
        }
    }

    /// Insert n blank cells at column, shifting cells right
    /// Cells that shift past the end are lost
    pub fn insert_cells(&mut self, col: usize, n: usize, attrs: CellAttributes) {
        if col >= self.cells.len() {
            return;
        }

        // Remove cells from the end
        let remove_count = n.min(self.cells.len() - col);
        for _ in 0..remove_count {
            self.cells.pop();
        }

        // Insert blank cells at position
        for _ in 0..n.min(self.cells.len() + n - col) {
            let mut cell = Cell::new();
            cell.attrs = attrs;
            if col < self.cells.len() {
                self.cells.insert(col, cell);
            } else {
                self.cells.push(cell);
            }
        }

        // Ensure we maintain the correct size
        while self.cells.len() < col + n {
            let mut cell = Cell::new();
            cell.attrs = attrs;
            self.cells.push(cell);
        }
    }

    /// Delete n cells at column, shifting cells left
    /// New cells at the end are filled with attrs
    pub fn delete_cells(&mut self, col: usize, n: usize, attrs: CellAttributes) {
        if col >= self.cells.len() {
            return;
        }

        let original_len = self.cells.len();
        let delete_count = n.min(self.cells.len() - col);

        // Remove cells
        for _ in 0..delete_count {
            if col < self.cells.len() {
                self.cells.remove(col);
            }
        }

        // Add blank cells at the end
        while self.cells.len() < original_len {
            let mut cell = Cell::new();
            cell.attrs = attrs;
            self.cells.push(cell);
        }
    }

    /// Erase n cells starting at column (replace with blanks, don't shift)
    pub fn erase_cells(&mut self, col: usize, n: usize, attrs: CellAttributes) {
        for i in col..col.saturating_add(n).min(self.cells.len()) {
            self.cells[i].clear(attrs);
        }
    }

    /// Get the text content of the line (for selection/copy)
    pub fn text(&self) -> String {
        let mut result = String::new();
        for cell in &self.cells {
            if cell.is_continuation() {
                continue;
            }
            let content = cell.content();
            if content.is_empty() {
                result.push(' ');
            } else {
                result.push_str(content);
            }
        }
        // Trim trailing spaces
        result.trim_end().to_string()
    }

    /// Check if line is empty (all cells are empty/space)
    pub fn is_empty(&self) -> bool {
        self.cells.iter().all(|c| c.is_empty())
    }

    /// Iterator over cells
    pub fn iter(&self) -> impl Iterator<Item = &Cell> {
        self.cells.iter()
    }

    /// Mutable iterator over cells
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Cell> {
        self.cells.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_new() {
        let line = Line::new(80);
        assert_eq!(line.cols(), 80);
        assert!(!line.wrapped);
    }

    #[test]
    fn test_line_get_cell() {
        let mut line = Line::new(80);
        line.cell_mut(0).set_char('A');
        assert_eq!(line.cell(0).display_char(), 'A');
    }

    #[test]
    fn test_line_clear() {
        let mut line = Line::new(80);
        line.cell_mut(0).set_char('A');
        line.cell_mut(1).set_char('B');
        line.clear(CellAttributes::default());
        assert!(line.cell(0).is_empty());
        assert!(line.cell(1).is_empty());
    }

    #[test]
    fn test_line_clear_from() {
        let mut line = Line::new(10);
        for i in 0..10 {
            line.cell_mut(i).set_char((b'A' + i as u8) as char);
        }
        line.clear_from(5, CellAttributes::default());
        assert_eq!(line.cell(4).display_char(), 'E');
        assert!(line.cell(5).is_empty());
    }

    #[test]
    fn test_line_clear_to() {
        let mut line = Line::new(10);
        for i in 0..10 {
            line.cell_mut(i).set_char((b'A' + i as u8) as char);
        }
        line.clear_to(4, CellAttributes::default());
        assert!(line.cell(4).is_empty());
        assert_eq!(line.cell(5).display_char(), 'F');
    }

    #[test]
    fn test_line_text() {
        let mut line = Line::new(10);
        line.cell_mut(0).set_char('H');
        line.cell_mut(1).set_char('i');
        assert_eq!(line.text(), "Hi");
    }

    #[test]
    fn test_line_insert_cells() {
        let mut line = Line::new(5);
        for i in 0..5 {
            line.cell_mut(i).set_char((b'A' + i as u8) as char);
        }
        // Line is: A B C D E
        line.insert_cells(2, 2, CellAttributes::default());
        // Should be: A B _ _ C (D E shifted out)
        assert_eq!(line.cell(0).display_char(), 'A');
        assert_eq!(line.cell(1).display_char(), 'B');
        assert!(line.cell(2).is_empty());
        assert!(line.cell(3).is_empty());
        assert_eq!(line.cell(4).display_char(), 'C');
    }

    #[test]
    fn test_line_delete_cells() {
        let mut line = Line::new(5);
        for i in 0..5 {
            line.cell_mut(i).set_char((b'A' + i as u8) as char);
        }
        // Line is: A B C D E
        line.delete_cells(1, 2, CellAttributes::default());
        // Should be: A D E _ _
        assert_eq!(line.cell(0).display_char(), 'A');
        assert_eq!(line.cell(1).display_char(), 'D');
        assert_eq!(line.cell(2).display_char(), 'E');
        assert!(line.cell(3).is_empty());
        assert!(line.cell(4).is_empty());
    }
}
