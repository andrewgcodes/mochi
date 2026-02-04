//! Selection
//!
//! Handles text selection in the terminal.

use crate::core::Screen;

/// Text selection state
#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// Start position (col, row) - row is absolute (includes scroll offset)
    start: Option<(usize, usize)>,
    /// End position (col, row) - row is absolute (includes scroll offset)
    end: Option<(usize, usize)>,
}

impl Selection {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new selection at the given position
    /// Row should be absolute (screen row + scroll offset)
    pub fn start(&mut self, col: usize, row: usize) {
        self.start = Some((col, row));
        self.end = Some((col, row));
    }

    /// Update the selection end point
    /// Row should be absolute (screen row + scroll offset)
    pub fn update(&mut self, col: usize, row: usize) {
        self.end = Some((col, row));
    }

    /// Clear the selection
    pub fn clear(&mut self) {
        self.start = None;
        self.end = None;
    }

    /// Check if there is an active selection
    pub fn is_active(&self) -> bool {
        self.start.is_some() && self.end.is_some()
    }

    /// Check if a cell is within the selection
    /// Row should be absolute (screen row + scroll offset)
    pub fn contains(&self, col: usize, row: usize) -> bool {
        let Some((start_col, start_row)) = self.start else {
            return false;
        };
        let Some((end_col, end_row)) = self.end else {
            return false;
        };

        // Normalize start and end
        let ((s_col, s_row), (e_col, e_row)) = if (start_row, start_col) <= (end_row, end_col) {
            ((start_col, start_row), (end_col, end_row))
        } else {
            ((end_col, end_row), (start_col, start_row))
        };

        if row < s_row || row > e_row {
            return false;
        }

        if row == s_row && row == e_row {
            // Single line selection
            col >= s_col && col <= e_col
        } else if row == s_row {
            // First line of multi-line selection
            col >= s_col
        } else if row == e_row {
            // Last line of multi-line selection
            col <= e_col
        } else {
            // Middle line - entire line is selected
            true
        }
    }

    /// Get the selected text from the screen
    pub fn get_text(&self, screen: &Screen, _scroll_offset: usize) -> Option<String> {
        let (start_col, start_row) = self.start?;
        let (end_col, end_row) = self.end?;

        // Normalize start and end
        let (start, end) = if (start_row, start_col) <= (end_row, end_col) {
            ((start_col, start_row), (end_col, end_row))
        } else {
            ((end_col, end_row), (start_col, start_row))
        };

        let mut text = String::new();
        let grid = screen.grid();

        for row_idx in start.1..=end.1.min(grid.rows().saturating_sub(1)) {
            if let Some(row) = grid.row(row_idx) {
                let col_start = if row_idx == start.1 { start.0 } else { 0 };
                let col_end = if row_idx == end.1 {
                    end.0
                } else {
                    row.cells.len().saturating_sub(1)
                };

                for col in col_start..=col_end.min(row.cells.len().saturating_sub(1)) {
                    let content = &row.cells[col].content;
                    if content.is_empty() {
                        text.push(' ');
                    } else {
                        text.push_str(content);
                    }
                }

                if row_idx != end.1 {
                    text.push('\n');
                }
            }
        }

        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }
}
