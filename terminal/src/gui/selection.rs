//! Selection
//!
//! Handles text selection in the terminal.

use crate::core::Screen;

/// Text selection state
#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// Start position (col, row, scroll_offset)
    start: Option<(usize, usize, usize)>,
    /// End position (col, row, scroll_offset)
    end: Option<(usize, usize, usize)>,
}

impl Selection {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new selection
    pub fn start(&mut self, col: usize, row: usize, scroll_offset: usize) {
        self.start = Some((col, row, scroll_offset));
        self.end = Some((col, row, scroll_offset));
    }

    /// Update the selection end point
    pub fn update(&mut self, col: usize, row: usize, scroll_offset: usize) {
        self.end = Some((col, row, scroll_offset));
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

    /// Get the selected text from the screen
    pub fn get_text(&self, screen: &Screen) -> Option<String> {
        let (start_col, start_row, _) = self.start?;
        let (end_col, end_row, _) = self.end?;

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
                    // Get the character from the cell content
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
