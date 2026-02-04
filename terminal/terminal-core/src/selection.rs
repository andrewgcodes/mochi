//! Text selection for copy/paste
//!
//! Handles selection state and text extraction.

use serde::{Deserialize, Serialize};

/// A point in the terminal (column, row)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub col: usize,
    pub row: isize, // Can be negative for scrollback
}

impl Point {
    pub fn new(col: usize, row: isize) -> Self {
        Self { col, row }
    }
}

/// Selection type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionType {
    /// Normal character-by-character selection
    Normal,
    /// Word selection (double-click)
    Word,
    /// Line selection (triple-click)
    Line,
    /// Block/rectangular selection
    Block,
}

/// Selection state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    /// Selection type
    pub selection_type: SelectionType,
    /// Start point (anchor)
    pub start: Point,
    /// End point (cursor)
    pub end: Point,
    /// Whether selection is active
    pub active: bool,
}

impl Selection {
    /// Create a new inactive selection
    pub fn new() -> Self {
        Self {
            selection_type: SelectionType::Normal,
            start: Point::new(0, 0),
            end: Point::new(0, 0),
            active: false,
        }
    }

    /// Start a new selection at the given point
    pub fn start(&mut self, point: Point, selection_type: SelectionType) {
        self.selection_type = selection_type;
        self.start = point;
        self.end = point;
        self.active = true;
    }

    /// Update the selection end point
    pub fn update(&mut self, point: Point) {
        if self.active {
            self.end = point;
        }
    }

    /// Finish the selection
    pub fn finish(&mut self) {
        // Selection remains active but no longer being dragged
    }

    /// Clear the selection
    pub fn clear(&mut self) {
        self.active = false;
    }

    /// Get the normalized selection bounds (start <= end)
    pub fn bounds(&self) -> (Point, Point) {
        let (start, end) = if self.start.row < self.end.row
            || (self.start.row == self.end.row && self.start.col <= self.end.col)
        {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        };
        (start, end)
    }

    /// Check if a cell is within the selection
    pub fn contains(&self, col: usize, row: isize) -> bool {
        if !self.active {
            return false;
        }

        let (start, end) = self.bounds();

        match self.selection_type {
            SelectionType::Normal | SelectionType::Word => {
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
            SelectionType::Line => row >= start.row && row <= end.row,
            SelectionType::Block => {
                let (min_col, max_col) = if start.col <= end.col {
                    (start.col, end.col)
                } else {
                    (end.col, start.col)
                };
                row >= start.row && row <= end.row && col >= min_col && col <= max_col
            }
        }
    }

    /// Check if selection spans multiple lines
    pub fn is_multiline(&self) -> bool {
        self.active && self.start.row != self.end.row
    }

    /// Check if selection is empty (start == end)
    pub fn is_empty(&self) -> bool {
        !self.active || (self.start.col == self.end.col && self.start.row == self.end.row)
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
    fn test_selection_new() {
        let sel = Selection::new();
        assert!(!sel.active);
        assert!(sel.is_empty());
    }

    #[test]
    fn test_selection_start() {
        let mut sel = Selection::new();
        sel.start(Point::new(5, 10), SelectionType::Normal);

        assert!(sel.active);
        assert_eq!(sel.start.col, 5);
        assert_eq!(sel.start.row, 10);
    }

    #[test]
    fn test_selection_update() {
        let mut sel = Selection::new();
        sel.start(Point::new(5, 10), SelectionType::Normal);
        sel.update(Point::new(20, 15));

        assert_eq!(sel.end.col, 20);
        assert_eq!(sel.end.row, 15);
    }

    #[test]
    fn test_selection_bounds_normalized() {
        let mut sel = Selection::new();
        sel.start(Point::new(20, 15), SelectionType::Normal);
        sel.update(Point::new(5, 10));

        let (start, end) = sel.bounds();
        assert_eq!(start.row, 10);
        assert_eq!(end.row, 15);
    }

    #[test]
    fn test_selection_contains_single_line() {
        let mut sel = Selection::new();
        sel.start(Point::new(5, 10), SelectionType::Normal);
        sel.update(Point::new(15, 10));

        assert!(sel.contains(5, 10));
        assert!(sel.contains(10, 10));
        assert!(sel.contains(15, 10));
        assert!(!sel.contains(4, 10));
        assert!(!sel.contains(16, 10));
        assert!(!sel.contains(10, 9));
    }

    #[test]
    fn test_selection_contains_multi_line() {
        let mut sel = Selection::new();
        sel.start(Point::new(5, 10), SelectionType::Normal);
        sel.update(Point::new(15, 12));

        // First line: from col 5 to end
        assert!(sel.contains(5, 10));
        assert!(sel.contains(100, 10));
        assert!(!sel.contains(4, 10));

        // Middle line: entire line
        assert!(sel.contains(0, 11));
        assert!(sel.contains(100, 11));

        // Last line: from start to col 15
        assert!(sel.contains(0, 12));
        assert!(sel.contains(15, 12));
        assert!(!sel.contains(16, 12));
    }

    #[test]
    fn test_selection_line_type() {
        let mut sel = Selection::new();
        sel.start(Point::new(5, 10), SelectionType::Line);
        sel.update(Point::new(15, 12));

        // Line selection includes entire lines
        assert!(sel.contains(0, 10));
        assert!(sel.contains(100, 10));
        assert!(sel.contains(0, 11));
        assert!(sel.contains(0, 12));
        assert!(sel.contains(100, 12));
        assert!(!sel.contains(0, 9));
        assert!(!sel.contains(0, 13));
    }

    #[test]
    fn test_selection_block_type() {
        let mut sel = Selection::new();
        sel.start(Point::new(5, 10), SelectionType::Block);
        sel.update(Point::new(15, 12));

        // Block selection is rectangular
        assert!(sel.contains(5, 10));
        assert!(sel.contains(15, 10));
        assert!(sel.contains(10, 11));
        assert!(sel.contains(5, 12));
        assert!(sel.contains(15, 12));

        assert!(!sel.contains(4, 10));
        assert!(!sel.contains(16, 10));
        assert!(!sel.contains(4, 11));
        assert!(!sel.contains(16, 11));
    }

    #[test]
    fn test_selection_clear() {
        let mut sel = Selection::new();
        sel.start(Point::new(5, 10), SelectionType::Normal);
        sel.update(Point::new(15, 12));

        sel.clear();

        assert!(!sel.active);
        assert!(!sel.contains(10, 11));
    }
}
