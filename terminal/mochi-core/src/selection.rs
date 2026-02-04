//! Text selection for copy/paste
//!
//! Handles rectangular and line-based selection of terminal content.

use serde::{Deserialize, Serialize};

/// A point in the terminal (row, column)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub row: usize,
    pub col: usize,
}

impl Point {
    pub fn new(row: usize, col: usize) -> Self {
        Point { row, col }
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.row.cmp(&other.row) {
            std::cmp::Ordering::Equal => self.col.cmp(&other.col),
            ord => ord,
        }
    }
}

/// Selection type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionType {
    /// Normal character-based selection
    Normal,
    /// Line-based selection (select entire lines)
    Line,
    /// Rectangular/block selection
    Block,
    /// Word selection (double-click)
    Word,
}

/// A text selection in the terminal
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    /// Selection type
    pub selection_type: SelectionType,
    /// Start point (anchor)
    pub start: Point,
    /// End point (current position)
    pub end: Point,
    /// Whether the selection is active
    pub active: bool,
}

impl Selection {
    /// Create a new selection starting at the given point
    pub fn new(start: Point, selection_type: SelectionType) -> Self {
        Selection {
            selection_type,
            start,
            end: start,
            active: true,
        }
    }

    /// Update the end point of the selection
    pub fn update(&mut self, end: Point) {
        self.end = end;
    }

    /// Get the normalized selection bounds (start <= end)
    pub fn bounds(&self) -> (Point, Point) {
        if self.start <= self.end {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }

    /// Check if a point is within the selection
    pub fn contains(&self, point: Point) -> bool {
        let (start, end) = self.bounds();

        match self.selection_type {
            SelectionType::Normal | SelectionType::Word => {
                if point.row < start.row || point.row > end.row {
                    return false;
                }
                if point.row == start.row && point.col < start.col {
                    return false;
                }
                if point.row == end.row && point.col > end.col {
                    return false;
                }
                true
            }
            SelectionType::Line => {
                point.row >= start.row && point.row <= end.row
            }
            SelectionType::Block => {
                let (min_col, max_col) = if start.col <= end.col {
                    (start.col, end.col)
                } else {
                    (end.col, start.col)
                };
                point.row >= start.row
                    && point.row <= end.row
                    && point.col >= min_col
                    && point.col <= max_col
            }
        }
    }

    /// Get the column range for a given row in the selection
    /// Returns None if the row is not in the selection
    pub fn columns_for_row(&self, row: usize, total_cols: usize) -> Option<(usize, usize)> {
        let (start, end) = self.bounds();

        if row < start.row || row > end.row {
            return None;
        }

        match self.selection_type {
            SelectionType::Normal | SelectionType::Word => {
                let start_col = if row == start.row { start.col } else { 0 };
                let end_col = if row == end.row { end.col } else { total_cols.saturating_sub(1) };
                Some((start_col, end_col))
            }
            SelectionType::Line => {
                Some((0, total_cols.saturating_sub(1)))
            }
            SelectionType::Block => {
                let (min_col, max_col) = if self.start.col <= self.end.col {
                    (self.start.col, self.end.col)
                } else {
                    (self.end.col, self.start.col)
                };
                Some((min_col.min(total_cols.saturating_sub(1)), max_col.min(total_cols.saturating_sub(1))))
            }
        }
    }

    /// Clear the selection
    pub fn clear(&mut self) {
        self.active = false;
    }

    /// Check if the selection is empty (start == end)
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

impl Default for Selection {
    fn default() -> Self {
        Selection {
            selection_type: SelectionType::Normal,
            start: Point::new(0, 0),
            end: Point::new(0, 0),
            active: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_ordering() {
        let p1 = Point::new(0, 5);
        let p2 = Point::new(1, 0);
        let p3 = Point::new(0, 10);

        assert!(p1 < p2);
        assert!(p1 < p3);
        assert!(p2 > p3);
    }

    #[test]
    fn test_selection_bounds() {
        let mut sel = Selection::new(Point::new(5, 10), SelectionType::Normal);
        sel.update(Point::new(2, 5));

        let (start, end) = sel.bounds();
        assert_eq!(start, Point::new(2, 5));
        assert_eq!(end, Point::new(5, 10));
    }

    #[test]
    fn test_selection_contains_normal() {
        let mut sel = Selection::new(Point::new(1, 5), SelectionType::Normal);
        sel.update(Point::new(3, 10));

        // Points within selection
        assert!(sel.contains(Point::new(1, 5)));
        assert!(sel.contains(Point::new(1, 10)));
        assert!(sel.contains(Point::new(2, 0)));
        assert!(sel.contains(Point::new(2, 50)));
        assert!(sel.contains(Point::new(3, 0)));
        assert!(sel.contains(Point::new(3, 10)));

        // Points outside selection
        assert!(!sel.contains(Point::new(0, 0)));
        assert!(!sel.contains(Point::new(1, 4)));
        assert!(!sel.contains(Point::new(3, 11)));
        assert!(!sel.contains(Point::new(4, 0)));
    }

    #[test]
    fn test_selection_contains_block() {
        let mut sel = Selection::new(Point::new(1, 5), SelectionType::Block);
        sel.update(Point::new(3, 10));

        // Points within block
        assert!(sel.contains(Point::new(1, 5)));
        assert!(sel.contains(Point::new(2, 7)));
        assert!(sel.contains(Point::new(3, 10)));

        // Points outside block
        assert!(!sel.contains(Point::new(2, 4)));
        assert!(!sel.contains(Point::new(2, 11)));
    }

    #[test]
    fn test_selection_columns_for_row() {
        let mut sel = Selection::new(Point::new(1, 5), SelectionType::Normal);
        sel.update(Point::new(3, 10));

        assert_eq!(sel.columns_for_row(0, 80), None);
        assert_eq!(sel.columns_for_row(1, 80), Some((5, 79)));
        assert_eq!(sel.columns_for_row(2, 80), Some((0, 79)));
        assert_eq!(sel.columns_for_row(3, 80), Some((0, 10)));
        assert_eq!(sel.columns_for_row(4, 80), None);
    }
}
