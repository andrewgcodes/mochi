//! Scrollback Buffer
//!
//! A ring buffer that stores lines scrolled off the top of the terminal.

use serde::{Deserialize, Serialize};

use super::grid::Row;

/// Scrollback buffer - stores lines that have scrolled off the top
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scrollback {
    /// The lines in the scrollback buffer (oldest first)
    lines: Vec<Row>,
    /// Maximum number of lines to store
    max_lines: usize,
}

impl Scrollback {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: Vec::new(),
            max_lines,
        }
    }

    /// Add lines to the scrollback buffer
    pub fn push(&mut self, rows: Vec<Row>) {
        for row in rows {
            if self.lines.len() >= self.max_lines {
                self.lines.remove(0);
            }
            self.lines.push(row);
        }
    }

    /// Get the number of lines in scrollback
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Check if scrollback is empty
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Get a line from scrollback (0 = oldest)
    pub fn get(&self, index: usize) -> Option<&Row> {
        self.lines.get(index)
    }

    /// Get lines from the end of scrollback (for display)
    /// Returns up to `count` lines, starting from `offset` lines from the end
    pub fn get_recent(&self, offset: usize, count: usize) -> Vec<&Row> {
        let start = self.lines.len().saturating_sub(offset + count);
        let end = self.lines.len().saturating_sub(offset);
        self.lines[start..end].iter().collect()
    }

    /// Clear the scrollback buffer
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// Set the maximum number of lines
    pub fn set_max_lines(&mut self, max: usize) {
        self.max_lines = max;
        while self.lines.len() > max {
            self.lines.remove(0);
        }
    }

    /// Get the maximum number of lines
    pub fn max_lines(&self) -> usize {
        self.max_lines
    }

    /// Iterate over all lines (oldest first)
    pub fn iter(&self) -> impl Iterator<Item = &Row> {
        self.lines.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrollback_new() {
        let sb = Scrollback::new(1000);
        assert!(sb.is_empty());
        assert_eq!(sb.max_lines(), 1000);
    }

    #[test]
    fn test_scrollback_push() {
        let mut sb = Scrollback::new(100);
        sb.push(vec![Row::new(80), Row::new(80)]);
        assert_eq!(sb.len(), 2);
    }

    #[test]
    fn test_scrollback_overflow() {
        let mut sb = Scrollback::new(3);
        for i in 0..5 {
            let mut row = Row::new(80);
            row.cells[0].content = i.to_string();
            sb.push(vec![row]);
        }
        assert_eq!(sb.len(), 3);
        // Should have lines 2, 3, 4 (oldest removed)
        assert_eq!(sb.get(0).unwrap().cells[0].content, "2");
        assert_eq!(sb.get(2).unwrap().cells[0].content, "4");
    }

    #[test]
    fn test_scrollback_get_recent() {
        let mut sb = Scrollback::new(100);
        for i in 0..10 {
            let mut row = Row::new(80);
            row.cells[0].content = i.to_string();
            sb.push(vec![row]);
        }

        // Get last 3 lines
        let recent = sb.get_recent(0, 3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].cells[0].content, "7");
        assert_eq!(recent[2].cells[0].content, "9");

        // Get 3 lines with offset 2
        let recent = sb.get_recent(2, 3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].cells[0].content, "5");
        assert_eq!(recent[2].cells[0].content, "7");
    }

    #[test]
    fn test_scrollback_clear() {
        let mut sb = Scrollback::new(100);
        sb.push(vec![Row::new(80), Row::new(80)]);
        sb.clear();
        assert!(sb.is_empty());
    }
}
