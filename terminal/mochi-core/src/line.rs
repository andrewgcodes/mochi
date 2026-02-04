//! Line representation for terminal grid.
//!
//! A line is a row of cells with metadata about wrapping.

use crate::cell::Cell;
use crate::color::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    cells: Vec<Cell>,
    pub wrapped: bool,
}

impl Line {
    pub fn new(cols: usize) -> Self {
        Line {
            cells: vec![Cell::default(); cols],
            wrapped: false,
        }
    }

    pub fn with_cells(cells: Vec<Cell>) -> Self {
        Line {
            cells,
            wrapped: false,
        }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn get(&self, col: usize) -> Option<&Cell> {
        self.cells.get(col)
    }

    pub fn get_mut(&mut self, col: usize) -> Option<&mut Cell> {
        self.cells.get_mut(col)
    }

    pub fn set(&mut self, col: usize, cell: Cell) {
        if col < self.cells.len() {
            self.cells[col] = cell;
        }
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    pub fn cells_mut(&mut self) -> &mut [Cell] {
        &mut self.cells
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.clear();
        }
        self.wrapped = false;
    }

    pub fn clear_with_bg(&mut self, bg: Color) {
        for cell in &mut self.cells {
            cell.clear_with_bg(bg);
        }
        self.wrapped = false;
    }

    pub fn clear_range(&mut self, start: usize, end: usize) {
        let end = end.min(self.cells.len());
        for col in start..end {
            self.cells[col].clear();
        }
    }

    pub fn clear_range_with_bg(&mut self, start: usize, end: usize, bg: Color) {
        let end = end.min(self.cells.len());
        for col in start..end {
            self.cells[col].clear_with_bg(bg);
        }
    }

    pub fn resize(&mut self, new_cols: usize) {
        if new_cols > self.cells.len() {
            self.cells.resize(new_cols, Cell::default());
        } else {
            self.cells.truncate(new_cols);
        }
    }

    pub fn insert_cells(&mut self, col: usize, count: usize) {
        if col >= self.cells.len() {
            return;
        }
        let insert_count = count.min(self.cells.len() - col);
        for _ in 0..insert_count {
            self.cells.pop();
        }
        for _ in 0..insert_count {
            self.cells.insert(col, Cell::default());
        }
    }

    pub fn delete_cells(&mut self, col: usize, count: usize) {
        if col >= self.cells.len() {
            return;
        }
        let delete_count = count.min(self.cells.len() - col);
        for _ in 0..delete_count {
            if col < self.cells.len() {
                self.cells.remove(col);
            }
        }
        let cols = self.cells.len() + delete_count;
        self.cells.resize(cols, Cell::default());
    }

    pub fn text_content(&self) -> String {
        let mut s = String::new();
        for cell in &self.cells {
            if !cell.is_wide_continuation() {
                s.push(cell.character);
            }
        }
        s.trim_end().to_string()
    }
}

impl PartialEq for Line {
    fn eq(&self, other: &Self) -> bool {
        self.cells == other.cells && self.wrapped == other.wrapped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_line() {
        let line = Line::new(80);
        assert_eq!(line.len(), 80);
        assert!(!line.wrapped);
    }

    #[test]
    fn test_set_get_cell() {
        let mut line = Line::new(80);
        let cell = Cell::new('A');
        line.set(10, cell.clone());
        assert_eq!(line.get(10).unwrap().character, 'A');
    }

    #[test]
    fn test_clear_line() {
        let mut line = Line::new(80);
        line.set(0, Cell::new('A'));
        line.set(1, Cell::new('B'));
        line.clear();
        assert_eq!(line.get(0).unwrap().character, ' ');
        assert_eq!(line.get(1).unwrap().character, ' ');
    }

    #[test]
    fn test_clear_range() {
        let mut line = Line::new(80);
        for i in 0..10 {
            line.set(i, Cell::new(('A' as u8 + i as u8) as char));
        }
        line.clear_range(3, 7);
        assert_eq!(line.get(2).unwrap().character, 'C');
        assert_eq!(line.get(3).unwrap().character, ' ');
        assert_eq!(line.get(6).unwrap().character, ' ');
        assert_eq!(line.get(7).unwrap().character, 'H');
    }

    #[test]
    fn test_insert_cells() {
        let mut line = Line::new(10);
        for i in 0..10 {
            line.set(i, Cell::new(('A' as u8 + i as u8) as char));
        }
        line.insert_cells(3, 2);
        assert_eq!(line.get(2).unwrap().character, 'C');
        assert_eq!(line.get(3).unwrap().character, ' ');
        assert_eq!(line.get(4).unwrap().character, ' ');
        assert_eq!(line.get(5).unwrap().character, 'D');
        assert_eq!(line.len(), 10);
    }

    #[test]
    fn test_delete_cells() {
        let mut line = Line::new(10);
        for i in 0..10 {
            line.set(i, Cell::new(('A' as u8 + i as u8) as char));
        }
        line.delete_cells(3, 2);
        assert_eq!(line.get(2).unwrap().character, 'C');
        assert_eq!(line.get(3).unwrap().character, 'F');
        assert_eq!(line.get(4).unwrap().character, 'G');
        assert_eq!(line.len(), 10);
    }

    #[test]
    fn test_text_content() {
        let mut line = Line::new(80);
        line.set(0, Cell::new('H'));
        line.set(1, Cell::new('e'));
        line.set(2, Cell::new('l'));
        line.set(3, Cell::new('l'));
        line.set(4, Cell::new('o'));
        assert_eq!(line.text_content(), "Hello");
    }
}
