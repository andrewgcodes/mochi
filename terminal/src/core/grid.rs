//! Terminal Grid
//!
//! A 2D grid of cells representing the visible terminal area.

use serde::{Deserialize, Serialize};

use super::cell::{Cell, Color};

/// A row of cells in the terminal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    /// The cells in this row
    pub cells: Vec<Cell>,
    /// Whether this row was soft-wrapped from the previous line
    pub wrapped: bool,
}

impl Row {
    pub fn new(cols: usize) -> Self {
        Self {
            cells: vec![Cell::default(); cols],
            wrapped: false,
        }
    }

    pub fn resize(&mut self, cols: usize) {
        self.cells.resize(cols, Cell::default());
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.clear();
        }
        self.wrapped = false;
    }

    pub fn erase(&mut self, bg: Color) {
        for cell in &mut self.cells {
            cell.erase(bg);
        }
        self.wrapped = false;
    }

    /// Erase cells from start to end (inclusive)
    pub fn erase_range(&mut self, start: usize, end: usize, bg: Color) {
        let end = end.min(self.cells.len().saturating_sub(1));
        for i in start..=end {
            if i < self.cells.len() {
                self.cells[i].erase(bg);
            }
        }
    }

    /// Get the length of content (excluding trailing empty cells)
    pub fn content_len(&self) -> usize {
        self.cells
            .iter()
            .rposition(|c| !c.is_empty())
            .map(|i| i + 1)
            .unwrap_or(0)
    }
}

/// The terminal grid - a 2D array of cells
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    /// The rows in the grid
    rows: Vec<Row>,
    /// Number of columns
    cols: usize,
    /// Number of rows
    num_rows: usize,
}

impl Grid {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            rows: (0..rows).map(|_| Row::new(cols)).collect(),
            cols,
            num_rows: rows,
        }
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.num_rows
    }

    /// Get a reference to a cell
    pub fn cell(&self, col: usize, row: usize) -> Option<&Cell> {
        self.rows.get(row).and_then(|r| r.cells.get(col))
    }

    /// Get a mutable reference to a cell
    pub fn cell_mut(&mut self, col: usize, row: usize) -> Option<&mut Cell> {
        self.rows.get_mut(row).and_then(|r| r.cells.get_mut(col))
    }

    /// Get a reference to a row
    pub fn row(&self, row: usize) -> Option<&Row> {
        self.rows.get(row)
    }

    /// Get a mutable reference to a row
    pub fn row_mut(&mut self, row: usize) -> Option<&mut Row> {
        self.rows.get_mut(row)
    }

    /// Resize the grid
    pub fn resize(&mut self, cols: usize, rows: usize) {
        // Resize existing rows
        for row in &mut self.rows {
            row.resize(cols);
        }

        // Add or remove rows
        use std::cmp::Ordering;
        match rows.cmp(&self.num_rows) {
            Ordering::Greater => {
                for _ in self.num_rows..rows {
                    self.rows.push(Row::new(cols));
                }
            }
            Ordering::Less => {
                self.rows.truncate(rows);
            }
            Ordering::Equal => {}
        }

        self.cols = cols;
        self.num_rows = rows;
    }

    /// Clear the entire grid
    pub fn clear(&mut self) {
        for row in &mut self.rows {
            row.clear();
        }
    }

    /// Erase the entire grid with a background color
    pub fn erase(&mut self, bg: Color) {
        for row in &mut self.rows {
            row.erase(bg);
        }
    }

    /// Scroll the grid up by n lines within a region
    /// Lines scrolled out of the top are returned for scrollback
    pub fn scroll_up(&mut self, n: usize, top: usize, bottom: usize, bg: Color) -> Vec<Row> {
        let n = n.min(bottom - top + 1);
        let mut scrolled_out = Vec::new();

        for _ in 0..n {
            if top < self.rows.len() {
                let row = self.rows.remove(top);
                scrolled_out.push(row);
            }
            if bottom < self.rows.len() {
                let mut new_row = Row::new(self.cols);
                new_row.erase(bg);
                self.rows.insert(bottom, new_row);
            } else {
                let mut new_row = Row::new(self.cols);
                new_row.erase(bg);
                self.rows.push(new_row);
            }
        }

        // Ensure we still have the right number of rows
        while self.rows.len() < self.num_rows {
            self.rows.push(Row::new(self.cols));
        }
        self.rows.truncate(self.num_rows);

        scrolled_out
    }

    /// Scroll the grid down by n lines within a region
    pub fn scroll_down(&mut self, n: usize, top: usize, bottom: usize, bg: Color) {
        let n = n.min(bottom - top + 1);

        for _ in 0..n {
            if bottom < self.rows.len() {
                self.rows.remove(bottom);
            }
            let mut new_row = Row::new(self.cols);
            new_row.erase(bg);
            if top <= self.rows.len() {
                self.rows.insert(top, new_row);
            }
        }

        // Ensure we still have the right number of rows
        while self.rows.len() < self.num_rows {
            self.rows.push(Row::new(self.cols));
        }
        self.rows.truncate(self.num_rows);
    }

    /// Insert n blank lines at the cursor row, scrolling down
    pub fn insert_lines(&mut self, row: usize, n: usize, top: usize, bottom: usize, bg: Color) {
        if row < top || row > bottom {
            return;
        }
        let effective_bottom = bottom.min(self.num_rows.saturating_sub(1));
        let n = n.min(effective_bottom - row + 1);

        for _ in 0..n {
            if effective_bottom < self.rows.len() {
                self.rows.remove(effective_bottom);
            }
            let mut new_row = Row::new(self.cols);
            new_row.erase(bg);
            if row <= self.rows.len() {
                self.rows.insert(row, new_row);
            }
        }

        // Ensure we still have the right number of rows
        while self.rows.len() < self.num_rows {
            self.rows.push(Row::new(self.cols));
        }
        self.rows.truncate(self.num_rows);
    }

    /// Delete n lines at the cursor row, scrolling up
    pub fn delete_lines(&mut self, row: usize, n: usize, top: usize, bottom: usize, bg: Color) {
        if row < top || row > bottom {
            return;
        }
        let effective_bottom = bottom.min(self.num_rows.saturating_sub(1));
        let n = n.min(effective_bottom - row + 1);

        for _ in 0..n {
            if row < self.rows.len() {
                self.rows.remove(row);
            }
            let mut new_row = Row::new(self.cols);
            new_row.erase(bg);
            if effective_bottom <= self.rows.len() {
                self.rows.insert(effective_bottom, new_row);
            } else {
                self.rows.push(new_row);
            }
        }

        // Ensure we still have the right number of rows
        while self.rows.len() < self.num_rows {
            self.rows.push(Row::new(self.cols));
        }
        self.rows.truncate(self.num_rows);
    }

    /// Insert n blank characters at position, shifting existing chars right
    pub fn insert_chars(&mut self, col: usize, row: usize, n: usize, bg: Color) {
        if let Some(r) = self.rows.get_mut(row) {
            for _ in 0..n {
                if col < r.cells.len() {
                    r.cells.pop();
                    let cell = Cell {
                        bg,
                        ..Default::default()
                    };
                    r.cells.insert(col, cell);
                }
            }
        }
    }

    /// Delete n characters at position, shifting remaining chars left
    pub fn delete_chars(&mut self, col: usize, row: usize, n: usize, bg: Color) {
        if let Some(r) = self.rows.get_mut(row) {
            for _ in 0..n {
                if col < r.cells.len() {
                    r.cells.remove(col);
                    let cell = Cell {
                        bg,
                        ..Default::default()
                    };
                    r.cells.push(cell);
                }
            }
        }
    }

    /// Erase n characters at position (replace with blanks, don't shift)
    pub fn erase_chars(&mut self, col: usize, row: usize, n: usize, bg: Color) {
        if let Some(r) = self.rows.get_mut(row) {
            for i in col..(col + n).min(r.cells.len()) {
                r.cells[i].erase(bg);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_new() {
        let grid = Grid::new(80, 24);
        assert_eq!(grid.cols(), 80);
        assert_eq!(grid.rows(), 24);
    }

    #[test]
    fn test_grid_cell_access() {
        let mut grid = Grid::new(80, 24);

        // Set a cell
        if let Some(cell) = grid.cell_mut(10, 5) {
            cell.content = "A".to_string();
        }

        // Read it back
        assert_eq!(grid.cell(10, 5).unwrap().content, "A");
    }

    #[test]
    fn test_grid_resize() {
        let mut grid = Grid::new(80, 24);
        grid.resize(120, 40);
        assert_eq!(grid.cols(), 120);
        assert_eq!(grid.rows(), 40);

        grid.resize(40, 10);
        assert_eq!(grid.cols(), 40);
        assert_eq!(grid.rows(), 10);
    }

    #[test]
    fn test_grid_scroll_up() {
        let mut grid = Grid::new(80, 5);

        // Put content in first row
        if let Some(cell) = grid.cell_mut(0, 0) {
            cell.content = "A".to_string();
        }

        // Scroll up
        let scrolled = grid.scroll_up(1, 0, 4, Color::Default);
        assert_eq!(scrolled.len(), 1);
        assert_eq!(scrolled[0].cells[0].content, "A");

        // First row should now be empty
        assert!(grid.cell(0, 0).unwrap().is_empty());
    }

    #[test]
    fn test_grid_scroll_down() {
        let mut grid = Grid::new(80, 5);

        // Put content in last row
        if let Some(cell) = grid.cell_mut(0, 4) {
            cell.content = "Z".to_string();
        }

        // Scroll down
        grid.scroll_down(1, 0, 4, Color::Default);

        // Last row content should have been pushed out
        // First row should be empty
        assert!(grid.cell(0, 0).unwrap().is_empty());
    }

    #[test]
    fn test_row_content_len() {
        let mut row = Row::new(80);
        assert_eq!(row.content_len(), 0);

        row.cells[5].content = "A".to_string();
        assert_eq!(row.content_len(), 6);

        row.cells[10].content = "B".to_string();
        assert_eq!(row.content_len(), 11);
    }

    #[test]
    fn test_insert_delete_chars() {
        let mut grid = Grid::new(10, 1);

        // Set up "ABCDE" in first 5 cells
        for (i, c) in "ABCDE".chars().enumerate() {
            if let Some(cell) = grid.cell_mut(i, 0) {
                cell.content = c.to_string();
            }
        }

        // Insert 2 chars at position 2
        grid.insert_chars(2, 0, 2, Color::Default);

        // Should now be "AB  CDE" (with DE pushed to positions 4,5,6 and last chars gone)
        assert_eq!(grid.cell(0, 0).unwrap().content, "A");
        assert_eq!(grid.cell(1, 0).unwrap().content, "B");
        assert!(grid.cell(2, 0).unwrap().is_empty());
        assert!(grid.cell(3, 0).unwrap().is_empty());
        assert_eq!(grid.cell(4, 0).unwrap().content, "C");
    }
}
