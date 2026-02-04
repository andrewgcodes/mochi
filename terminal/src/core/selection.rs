//! Selection handling for terminal text selection
//!
//! This module provides functionality for selecting text in the terminal,
//! including mouse-based selection and clipboard integration.

use serde::{Deserialize, Serialize};

/// A position in the terminal grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionPoint {
    /// Row index (0-based, can be negative for scrollback)
    pub row: i32,
    /// Column index (0-based)
    pub col: usize,
}

impl SelectionPoint {
    /// Create a new selection point
    pub fn new(row: i32, col: usize) -> Self {
        Self { row, col }
    }

    /// Check if this point is before another point
    pub fn is_before(&self, other: &SelectionPoint) -> bool {
        if self.row != other.row {
            self.row < other.row
        } else {
            self.col < other.col
        }
    }
}

/// Selection type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SelectionType {
    /// Character-level selection (default)
    #[default]
    Normal,
    /// Word-level selection (double-click)
    Word,
    /// Line-level selection (triple-click)
    Line,
    /// Block/rectangular selection (Alt+drag)
    Block,
}

/// Represents a text selection in the terminal
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    /// Start point of the selection (where the mouse was pressed)
    start: SelectionPoint,
    /// End point of the selection (current mouse position)
    end: SelectionPoint,
    /// Type of selection
    selection_type: SelectionType,
    /// Whether the selection is active (mouse is being dragged)
    active: bool,
}

impl Selection {
    /// Create a new selection starting at the given point
    pub fn new(start: SelectionPoint, selection_type: SelectionType) -> Self {
        Self {
            start,
            end: start,
            selection_type,
            active: true,
        }
    }

    /// Create a normal (character) selection
    pub fn normal(row: i32, col: usize) -> Self {
        Self::new(SelectionPoint::new(row, col), SelectionType::Normal)
    }

    /// Create a word selection
    pub fn word(row: i32, col: usize) -> Self {
        Self::new(SelectionPoint::new(row, col), SelectionType::Word)
    }

    /// Create a line selection
    pub fn line(row: i32, col: usize) -> Self {
        Self::new(SelectionPoint::new(row, col), SelectionType::Line)
    }

    /// Create a block selection
    pub fn block(row: i32, col: usize) -> Self {
        Self::new(SelectionPoint::new(row, col), SelectionType::Block)
    }

    /// Update the end point of the selection
    pub fn update(&mut self, row: i32, col: usize) {
        self.end = SelectionPoint::new(row, col);
    }

    /// Finish the selection (mouse released)
    pub fn finish(&mut self) {
        self.active = false;
    }

    /// Check if the selection is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Check if the selection is empty (start == end)
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Get the selection type
    pub fn selection_type(&self) -> SelectionType {
        self.selection_type
    }

    /// Get the normalized start and end points (start is always before end)
    pub fn normalized(&self) -> (SelectionPoint, SelectionPoint) {
        if self.start.is_before(&self.end) {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }

    /// Check if a cell at (row, col) is within the selection
    pub fn contains(&self, row: i32, col: usize, _cols: usize) -> bool {
        let (start, end) = self.normalized();

        match self.selection_type {
            SelectionType::Normal | SelectionType::Word => {
                if row < start.row || row > end.row {
                    return false;
                }
                if row == start.row && row == end.row {
                    // Single line selection
                    col >= start.col && col <= end.col
                } else if row == start.row {
                    // First line of multi-line selection
                    col >= start.col
                } else if row == end.row {
                    // Last line of multi-line selection
                    col <= end.col
                } else {
                    // Middle lines are fully selected
                    true
                }
            },
            SelectionType::Line => {
                // Entire lines are selected
                row >= start.row && row <= end.row
            },
            SelectionType::Block => {
                // Rectangular selection
                let min_col = start.col.min(end.col);
                let max_col = start.col.max(end.col);
                row >= start.row && row <= end.row && col >= min_col && col <= max_col
            },
        }
    }

    /// Get the range of rows covered by the selection
    pub fn row_range(&self) -> (i32, i32) {
        let (start, end) = self.normalized();
        (start.row, end.row)
    }

    /// Get the column range for a specific row
    pub fn col_range(&self, row: i32, cols: usize) -> Option<(usize, usize)> {
        let (start, end) = self.normalized();

        if row < start.row || row > end.row {
            return None;
        }

        match self.selection_type {
            SelectionType::Normal | SelectionType::Word => {
                if row == start.row && row == end.row {
                    Some((start.col, end.col))
                } else if row == start.row {
                    Some((start.col, cols.saturating_sub(1)))
                } else if row == end.row {
                    Some((0, end.col))
                } else {
                    Some((0, cols.saturating_sub(1)))
                }
            },
            SelectionType::Line => Some((0, cols.saturating_sub(1))),
            SelectionType::Block => {
                let min_col = start.col.min(end.col);
                let max_col = start.col.max(end.col);
                Some((min_col, max_col))
            },
        }
    }
}

/// Selection manager that handles selection state and text extraction
#[derive(Debug, Default)]
pub struct SelectionManager {
    /// Current selection, if any
    selection: Option<Selection>,
}

impl SelectionManager {
    /// Create a new selection manager
    pub fn new() -> Self {
        Self { selection: None }
    }

    /// Start a new selection
    pub fn start(&mut self, row: i32, col: usize, selection_type: SelectionType) {
        self.selection = Some(Selection::new(
            SelectionPoint::new(row, col),
            selection_type,
        ));
    }

    /// Start a normal selection
    pub fn start_normal(&mut self, row: i32, col: usize) {
        self.start(row, col, SelectionType::Normal);
    }

    /// Start a word selection
    pub fn start_word(&mut self, row: i32, col: usize) {
        self.start(row, col, SelectionType::Word);
    }

    /// Start a line selection
    pub fn start_line(&mut self, row: i32, col: usize) {
        self.start(row, col, SelectionType::Line);
    }

    /// Start a block selection
    pub fn start_block(&mut self, row: i32, col: usize) {
        self.start(row, col, SelectionType::Block);
    }

    /// Update the current selection
    pub fn update(&mut self, row: i32, col: usize) {
        if let Some(ref mut selection) = self.selection {
            selection.update(row, col);
        }
    }

    /// Finish the current selection
    pub fn finish(&mut self) {
        if let Some(ref mut selection) = self.selection {
            selection.finish();
        }
    }

    /// Clear the current selection
    pub fn clear(&mut self) {
        self.selection = None;
    }

    /// Get the current selection
    pub fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    /// Check if there is an active selection
    pub fn has_selection(&self) -> bool {
        self.selection.is_some()
    }

    /// Check if a cell is selected
    pub fn is_selected(&self, row: i32, col: usize, cols: usize) -> bool {
        self.selection
            .as_ref()
            .map(|s| s.contains(row, col, cols))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_point_ordering() {
        let p1 = SelectionPoint::new(0, 5);
        let p2 = SelectionPoint::new(0, 10);
        let p3 = SelectionPoint::new(1, 0);

        assert!(p1.is_before(&p2));
        assert!(p1.is_before(&p3));
        assert!(p2.is_before(&p3));
        assert!(!p2.is_before(&p1));
    }

    #[test]
    fn test_selection_contains_single_line() {
        let mut selection = Selection::normal(0, 5);
        selection.update(0, 10);

        assert!(!selection.contains(0, 4, 80));
        assert!(selection.contains(0, 5, 80));
        assert!(selection.contains(0, 7, 80));
        assert!(selection.contains(0, 10, 80));
        assert!(!selection.contains(0, 11, 80));
    }

    #[test]
    fn test_selection_contains_multi_line() {
        let mut selection = Selection::normal(0, 5);
        selection.update(2, 10);

        // First line
        assert!(!selection.contains(0, 4, 80));
        assert!(selection.contains(0, 5, 80));
        assert!(selection.contains(0, 79, 80));

        // Middle line
        assert!(selection.contains(1, 0, 80));
        assert!(selection.contains(1, 40, 80));
        assert!(selection.contains(1, 79, 80));

        // Last line
        assert!(selection.contains(2, 0, 80));
        assert!(selection.contains(2, 10, 80));
        assert!(!selection.contains(2, 11, 80));
    }

    #[test]
    fn test_selection_line_mode() {
        let mut selection = Selection::line(1, 5);
        selection.update(3, 10);

        // Entire lines should be selected
        assert!(!selection.contains(0, 0, 80));
        assert!(selection.contains(1, 0, 80));
        assert!(selection.contains(1, 79, 80));
        assert!(selection.contains(2, 0, 80));
        assert!(selection.contains(3, 0, 80));
        assert!(selection.contains(3, 79, 80));
        assert!(!selection.contains(4, 0, 80));
    }

    #[test]
    fn test_selection_block_mode() {
        let mut selection = Selection::block(1, 10);
        selection.update(3, 20);

        // Block selection
        assert!(!selection.contains(0, 15, 80));
        assert!(!selection.contains(1, 9, 80));
        assert!(selection.contains(1, 10, 80));
        assert!(selection.contains(1, 15, 80));
        assert!(selection.contains(1, 20, 80));
        assert!(!selection.contains(1, 21, 80));
        assert!(selection.contains(2, 15, 80));
        assert!(!selection.contains(4, 15, 80));
    }

    #[test]
    fn test_selection_manager() {
        let mut manager = SelectionManager::new();
        assert!(!manager.has_selection());

        manager.start_normal(0, 5);
        assert!(manager.has_selection());

        manager.update(0, 10);
        assert!(manager.is_selected(0, 7, 80));
        assert!(!manager.is_selected(0, 15, 80));

        manager.clear();
        assert!(!manager.has_selection());
    }
}
