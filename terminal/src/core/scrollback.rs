//! Scrollback buffer implementation
//!
//! A ring buffer that stores terminal history lines for scrolling back.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use super::line::Line;

/// Default maximum scrollback lines
pub const DEFAULT_SCROLLBACK_LINES: usize = 10000;

/// Scrollback buffer - stores terminal history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scrollback {
    /// Lines in the scrollback buffer (oldest first)
    lines: VecDeque<Line>,
    /// Maximum number of lines to keep
    max_lines: usize,
}

impl Default for Scrollback {
    fn default() -> Self {
        Self::new(DEFAULT_SCROLLBACK_LINES)
    }
}

impl Scrollback {
    /// Create a new scrollback buffer with the specified maximum lines
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            max_lines,
        }
    }

    /// Get the number of lines currently in the buffer
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Get the maximum number of lines
    pub fn max_lines(&self) -> usize {
        self.max_lines
    }

    /// Set the maximum number of lines
    /// If the new max is smaller, oldest lines are removed
    pub fn set_max_lines(&mut self, max_lines: usize) {
        self.max_lines = max_lines;
        while self.lines.len() > self.max_lines {
            self.lines.pop_front();
        }
    }

    /// Push a line to the scrollback buffer
    /// If the buffer is full, the oldest line is removed
    pub fn push(&mut self, line: Line) {
        if self.max_lines == 0 {
            return;
        }
        if self.lines.len() >= self.max_lines {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    /// Push multiple lines to the scrollback buffer
    pub fn push_lines(&mut self, lines: impl IntoIterator<Item = Line>) {
        for line in lines {
            self.push(line);
        }
    }

    /// Pop a line from the end of the scrollback buffer
    /// Used when scrolling down (reverse index at top of screen)
    pub fn pop(&mut self) -> Option<Line> {
        self.lines.pop_back()
    }

    /// Get a line at the given index (0 = oldest)
    pub fn get(&self, index: usize) -> Option<&Line> {
        self.lines.get(index)
    }

    /// Get a line from the end (0 = most recent)
    pub fn get_from_end(&self, index: usize) -> Option<&Line> {
        if index >= self.lines.len() {
            None
        } else {
            self.lines.get(self.lines.len() - 1 - index)
        }
    }

    /// Iterate over all lines (oldest first)
    pub fn iter(&self) -> impl Iterator<Item = &Line> {
        self.lines.iter()
    }

    /// Iterate over lines in reverse (newest first)
    pub fn iter_rev(&self) -> impl Iterator<Item = &Line> {
        self.lines.iter().rev()
    }

    /// Clear all lines from the buffer
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// Get the text content of all lines
    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.text())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrollback_new() {
        let sb = Scrollback::new(100);
        assert!(sb.is_empty());
        assert_eq!(sb.len(), 0);
        assert_eq!(sb.max_lines(), 100);
    }

    #[test]
    fn test_scrollback_push() {
        let mut sb = Scrollback::new(100);
        sb.push(Line::new(80));
        assert_eq!(sb.len(), 1);
        sb.push(Line::new(80));
        assert_eq!(sb.len(), 2);
    }

    #[test]
    fn test_scrollback_overflow() {
        let mut sb = Scrollback::new(3);

        for i in 0..5 {
            let mut line = Line::new(10);
            line.cell_mut(0)
                .unwrap()
                .set_content(('A' as u8 + i as u8) as char);
            sb.push(line);
        }

        // Should only have last 3 lines
        assert_eq!(sb.len(), 3);
        assert_eq!(sb.get(0).unwrap().cell(0).unwrap().content(), "C");
        assert_eq!(sb.get(1).unwrap().cell(0).unwrap().content(), "D");
        assert_eq!(sb.get(2).unwrap().cell(0).unwrap().content(), "E");
    }

    #[test]
    fn test_scrollback_get_from_end() {
        let mut sb = Scrollback::new(100);

        for i in 0..5 {
            let mut line = Line::new(10);
            line.cell_mut(0)
                .unwrap()
                .set_content(('A' as u8 + i as u8) as char);
            sb.push(line);
        }

        assert_eq!(sb.get_from_end(0).unwrap().cell(0).unwrap().content(), "E");
        assert_eq!(sb.get_from_end(1).unwrap().cell(0).unwrap().content(), "D");
        assert_eq!(sb.get_from_end(4).unwrap().cell(0).unwrap().content(), "A");
        assert!(sb.get_from_end(5).is_none());
    }

    #[test]
    fn test_scrollback_pop() {
        let mut sb = Scrollback::new(100);

        for i in 0..3 {
            let mut line = Line::new(10);
            line.cell_mut(0)
                .unwrap()
                .set_content(('A' as u8 + i as u8) as char);
            sb.push(line);
        }

        let popped = sb.pop().unwrap();
        assert_eq!(popped.cell(0).unwrap().content(), "C");
        assert_eq!(sb.len(), 2);
    }

    #[test]
    fn test_scrollback_set_max_lines() {
        let mut sb = Scrollback::new(100);

        for i in 0..10 {
            let mut line = Line::new(10);
            line.cell_mut(0)
                .unwrap()
                .set_content(('A' as u8 + i as u8) as char);
            sb.push(line);
        }

        sb.set_max_lines(3);
        assert_eq!(sb.len(), 3);
        // Should keep the most recent 3
        assert_eq!(sb.get(0).unwrap().cell(0).unwrap().content(), "H");
    }

    #[test]
    fn test_scrollback_zero_max() {
        let mut sb = Scrollback::new(0);
        sb.push(Line::new(80));
        assert!(sb.is_empty());
    }

    #[test]
    fn test_scrollback_clear() {
        let mut sb = Scrollback::new(100);
        sb.push(Line::new(80));
        sb.push(Line::new(80));
        sb.clear();
        assert!(sb.is_empty());
    }
}
