//! Text selection for copy/paste operations.
//!
//! Handles mouse-based text selection in the terminal grid.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionPoint {
    pub row: i64,
    pub col: usize,
}

impl SelectionPoint {
    pub fn new(row: i64, col: usize) -> Self {
        SelectionPoint { row, col }
    }
}

impl PartialOrd for SelectionPoint {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SelectionPoint {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.row.cmp(&other.row) {
            std::cmp::Ordering::Equal => self.col.cmp(&other.col),
            ord => ord,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionType {
    Normal,
    Block,
    Line,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub start: SelectionPoint,
    pub end: SelectionPoint,
    pub selection_type: SelectionType,
    pub active: bool,
}

impl Selection {
    pub fn new() -> Self {
        Selection {
            start: SelectionPoint::new(0, 0),
            end: SelectionPoint::new(0, 0),
            selection_type: SelectionType::Normal,
            active: false,
        }
    }

    pub fn start_selection(&mut self, row: i64, col: usize, selection_type: SelectionType) {
        self.start = SelectionPoint::new(row, col);
        self.end = SelectionPoint::new(row, col);
        self.selection_type = selection_type;
        self.active = true;
    }

    pub fn update_selection(&mut self, row: i64, col: usize) {
        if self.active {
            self.end = SelectionPoint::new(row, col);
        }
    }

    pub fn end_selection(&mut self) {
        self.active = false;
    }

    pub fn clear(&mut self) {
        self.active = false;
        self.start = SelectionPoint::new(0, 0);
        self.end = SelectionPoint::new(0, 0);
    }

    pub fn normalized(&self) -> (SelectionPoint, SelectionPoint) {
        if self.start <= self.end {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }

    pub fn contains(&self, row: i64, col: usize) -> bool {
        if !self.active && self.start == self.end {
            return false;
        }

        let (start, end) = self.normalized();
        let point = SelectionPoint::new(row, col);

        match self.selection_type {
            SelectionType::Normal => {
                if row < start.row || row > end.row {
                    return false;
                }
                if row == start.row && row == end.row {
                    col >= start.col && col <= end.col
                } else if row == start.row {
                    col >= start.col
                } else if row == end.row {
                    col <= end.col
                } else {
                    true
                }
            }
            SelectionType::Block => {
                let min_col = start.col.min(end.col);
                let max_col = start.col.max(end.col);
                row >= start.row && row <= end.row && col >= min_col && col <= max_col
            }
            SelectionType::Line => {
                row >= start.row && row <= end.row
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        !self.active && self.start == self.end
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_contains_normal() {
        let mut sel = Selection::new();
        sel.start_selection(1, 5, SelectionType::Normal);
        sel.update_selection(3, 10);

        assert!(!sel.contains(0, 5));
        assert!(sel.contains(1, 5));
        assert!(sel.contains(1, 10));
        assert!(sel.contains(2, 0));
        assert!(sel.contains(2, 50));
        assert!(sel.contains(3, 0));
        assert!(sel.contains(3, 10));
        assert!(!sel.contains(3, 11));
        assert!(!sel.contains(4, 0));
    }

    #[test]
    fn test_selection_contains_block() {
        let mut sel = Selection::new();
        sel.start_selection(1, 5, SelectionType::Block);
        sel.update_selection(3, 10);

        assert!(sel.contains(1, 5));
        assert!(sel.contains(1, 10));
        assert!(sel.contains(2, 7));
        assert!(!sel.contains(2, 4));
        assert!(!sel.contains(2, 11));
    }

    #[test]
    fn test_selection_contains_line() {
        let mut sel = Selection::new();
        sel.start_selection(1, 5, SelectionType::Line);
        sel.update_selection(3, 10);

        assert!(!sel.contains(0, 0));
        assert!(sel.contains(1, 0));
        assert!(sel.contains(2, 100));
        assert!(sel.contains(3, 0));
        assert!(!sel.contains(4, 0));
    }

    #[test]
    fn test_selection_normalized() {
        let mut sel = Selection::new();
        sel.start_selection(5, 10, SelectionType::Normal);
        sel.update_selection(2, 5);

        let (start, end) = sel.normalized();
        assert_eq!(start.row, 2);
        assert_eq!(start.col, 5);
        assert_eq!(end.row, 5);
        assert_eq!(end.col, 10);
    }
}
