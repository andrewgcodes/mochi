//! Terminal line representation
//!
//! A line represents a row of cells in the terminal grid,
//! with metadata about wrapping behavior.

use serde::{Deserialize, Serialize};

use super::cell::{Cell, CellAttributes, Color};

/// A row of cells in the terminal
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    /// The cells in this line
    cells: Vec<Cell>,
    /// True if this line was soft-wrapped from the previous line
    /// (i.e., the previous line overflowed and continued onto this line)
    wrapped: bool,
}

impl Line {
    /// Create a new line with the specified number of columns
    pub fn new(cols: usize) -> Self {
        Self {
            cells: vec![Cell::new(); cols],
            wrapped: false,
        }
    }

    /// Create a new line with specified columns and default attributes
    pub fn with_attrs(cols: usize, fg: Color, bg: Color, attrs: CellAttributes) -> Self {
        let cells = (0..cols)
            .map(|_| {
                let mut cell = Cell::new();
                cell.set_fg(fg);
                cell.set_bg(bg);
                cell.set_attrs(attrs);
                cell
            })
            .collect();
        Self {
            cells,
            wrapped: false,
        }
    }

    /// Get the number of columns in this line
    pub fn cols(&self) -> usize {
        self.cells.len()
    }

    /// Get a reference to a cell at the given column
    pub fn cell(&self, col: usize) -> Option<&Cell> {
        self.cells.get(col)
    }

    /// Get a mutable reference to a cell at the given column
    pub fn cell_mut(&mut self, col: usize) -> Option<&mut Cell> {
        self.cells.get_mut(col)
    }

    /// Get all cells
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Get mutable reference to all cells
    pub fn cells_mut(&mut self) -> &mut [Cell] {
        &mut self.cells
    }

    /// Check if this line is wrapped from the previous line
    pub fn is_wrapped(&self) -> bool {
        self.wrapped
    }

    /// Set the wrapped flag
    pub fn set_wrapped(&mut self, wrapped: bool) {
        self.wrapped = wrapped;
    }

    /// Resize the line to a new number of columns
    /// If growing, new cells are initialized with default values
    /// If shrinking, cells are truncated
    pub fn resize(&mut self, cols: usize) {
        self.cells.resize_with(cols, Cell::new);
    }

    /// Resize the line with specific background color for new cells
    pub fn resize_with_bg(&mut self, cols: usize, bg: Color) {
        let old_len = self.cells.len();
        self.cells.resize_with(cols, Cell::new);
        // Set background color on new cells
        for cell in self.cells.iter_mut().skip(old_len) {
            cell.set_bg(bg);
        }
    }

    /// Clear all cells in the line
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.reset();
        }
        self.wrapped = false;
    }

    /// Clear all cells with a specific background color
    pub fn clear_with_bg(&mut self, bg: Color) {
        for cell in &mut self.cells {
            cell.erase(bg);
        }
        self.wrapped = false;
    }

    /// Clear cells from the given column to the end of the line
    pub fn clear_from(&mut self, col: usize, bg: Color) {
        for cell in self.cells.iter_mut().skip(col) {
            cell.erase(bg);
        }
    }

    /// Clear cells from the start of the line to the given column (inclusive)
    pub fn clear_to(&mut self, col: usize, bg: Color) {
        for cell in self.cells.iter_mut().take(col + 1) {
            cell.erase(bg);
        }
    }

    /// Insert blank cells at the given column, shifting existing cells right
    /// Cells that shift past the end are lost
    pub fn insert_cells(&mut self, col: usize, count: usize, bg: Color) {
        if col >= self.cells.len() {
            return;
        }

        // Shift cells right
        let shift_count = count.min(self.cells.len() - col);
        for i in (col + shift_count..self.cells.len()).rev() {
            self.cells[i] = self.cells[i - shift_count].clone();
        }

        // Clear the inserted cells
        for cell in self.cells.iter_mut().skip(col).take(shift_count) {
            cell.erase(bg);
        }
    }

    /// Delete cells at the given column, shifting remaining cells left
    /// New cells at the end are blank
    pub fn delete_cells(&mut self, col: usize, count: usize, bg: Color) {
        let len = self.cells.len();
        if col >= len {
            return;
        }

        let delete_count = count.min(len - col);

        // Shift cells left
        for i in col..len - delete_count {
            self.cells[i] = self.cells[i + delete_count].clone();
        }

        // Clear the cells at the end
        let start = len - delete_count;
        for cell in self.cells.iter_mut().skip(start) {
            cell.erase(bg);
        }
    }

    /// Erase characters starting at the given column
    pub fn erase_chars(&mut self, col: usize, count: usize, bg: Color) {
        let end = (col + count).min(self.cells.len());
        for cell in self.cells.iter_mut().skip(col).take(end - col) {
            cell.erase(bg);
        }
    }

    /// Get the text content of this line as a string
    pub fn text(&self) -> String {
        let mut result = String::new();
        for cell in &self.cells {
            // Skip continuation cells (width 0) - they are part of a wide character
            if cell.width() == 0 {
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

    /// Check if the line is empty (all cells are empty or whitespace)
    pub fn is_empty(&self) -> bool {
        self.cells
            .iter()
            .all(|c| c.is_empty() || c.content() == " ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_new() {
        let line = Line::new(80);
        assert_eq!(line.cols(), 80);
        assert!(!line.is_wrapped());
        assert!(line.is_empty());
    }

    #[test]
    fn test_line_cell_access() {
        let mut line = Line::new(10);
        line.cell_mut(5).unwrap().set_content('X');
        assert_eq!(line.cell(5).unwrap().content(), "X");
        assert!(line.cell(10).is_none());
    }

    #[test]
    fn test_line_resize() {
        let mut line = Line::new(10);
        line.cell_mut(5).unwrap().set_content('A');

        // Grow
        line.resize(20);
        assert_eq!(line.cols(), 20);
        assert_eq!(line.cell(5).unwrap().content(), "A");
        assert!(line.cell(15).unwrap().is_empty());

        // Shrink
        line.resize(5);
        assert_eq!(line.cols(), 5);
    }

    #[test]
    fn test_line_clear() {
        let mut line = Line::new(10);
        line.cell_mut(0).unwrap().set_content('A');
        line.cell_mut(5).unwrap().set_content('B');
        line.set_wrapped(true);

        line.clear();

        assert!(line.is_empty());
        assert!(!line.is_wrapped());
    }

    #[test]
    fn test_line_clear_from() {
        let mut line = Line::new(10);
        for i in 0..10 {
            line.cell_mut(i)
                .unwrap()
                .set_content((b'A' + i as u8) as char);
        }

        line.clear_from(5, Color::Default);

        assert_eq!(line.cell(4).unwrap().content(), "E");
        assert!(line.cell(5).unwrap().is_empty());
        assert!(line.cell(9).unwrap().is_empty());
    }

    #[test]
    fn test_line_clear_to() {
        let mut line = Line::new(10);
        for i in 0..10 {
            line.cell_mut(i)
                .unwrap()
                .set_content((b'A' + i as u8) as char);
        }

        line.clear_to(4, Color::Default);

        assert!(line.cell(0).unwrap().is_empty());
        assert!(line.cell(4).unwrap().is_empty());
        assert_eq!(line.cell(5).unwrap().content(), "F");
    }

    #[test]
    fn test_line_insert_cells() {
        let mut line = Line::new(10);
        for i in 0..10 {
            line.cell_mut(i)
                .unwrap()
                .set_content((b'A' + i as u8) as char);
        }

        line.insert_cells(2, 3, Color::Default);

        assert_eq!(line.cell(0).unwrap().content(), "A");
        assert_eq!(line.cell(1).unwrap().content(), "B");
        assert!(line.cell(2).unwrap().is_empty());
        assert!(line.cell(3).unwrap().is_empty());
        assert!(line.cell(4).unwrap().is_empty());
        assert_eq!(line.cell(5).unwrap().content(), "C");
        assert_eq!(line.cell(6).unwrap().content(), "D");
    }

    #[test]
    fn test_line_delete_cells() {
        let mut line = Line::new(10);
        for i in 0..10 {
            line.cell_mut(i)
                .unwrap()
                .set_content((b'A' + i as u8) as char);
        }

        line.delete_cells(2, 3, Color::Default);

        assert_eq!(line.cell(0).unwrap().content(), "A");
        assert_eq!(line.cell(1).unwrap().content(), "B");
        assert_eq!(line.cell(2).unwrap().content(), "F");
        assert_eq!(line.cell(3).unwrap().content(), "G");
        assert!(line.cell(7).unwrap().is_empty());
    }

    #[test]
    fn test_line_text() {
        let mut line = Line::new(10);
        line.cell_mut(0).unwrap().set_content('H');
        line.cell_mut(1).unwrap().set_content('i');
        // Leave rest empty

        assert_eq!(line.text(), "Hi");
    }
}
