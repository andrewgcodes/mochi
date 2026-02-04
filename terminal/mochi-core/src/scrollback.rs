//! Scrollback buffer implementation
//!
//! The scrollback buffer stores lines that have scrolled off the top of the screen.
//! It uses a ring buffer for efficient memory usage with a configurable maximum size.

use serde::{Deserialize, Serialize};

use crate::line::Line;

/// Default maximum scrollback lines
pub const DEFAULT_SCROLLBACK_LINES: usize = 10000;

/// Scrollback buffer using a ring buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scrollback {
    /// Ring buffer of lines
    lines: Vec<Line>,
    /// Index of the oldest line in the buffer
    start: usize,
    /// Number of lines currently in the buffer
    len: usize,
    /// Maximum number of lines to store
    max_lines: usize,
}

impl Scrollback {
    /// Create a new scrollback buffer with the given maximum size
    pub fn new(max_lines: usize) -> Self {
        Scrollback {
            lines: Vec::new(),
            start: 0,
            len: 0,
            max_lines,
        }
    }

    /// Get the number of lines in the scrollback
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the scrollback is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the maximum number of lines
    pub fn max_lines(&self) -> usize {
        self.max_lines
    }

    /// Push a line to the scrollback buffer
    pub fn push(&mut self, line: Line) {
        if self.max_lines == 0 {
            return;
        }

        if self.lines.len() < self.max_lines {
            // Buffer not yet full, just append
            self.lines.push(line);
            self.len += 1;
        } else {
            // Buffer full, overwrite oldest
            let index = (self.start + self.len) % self.max_lines;
            self.lines[index] = line;
            if self.len == self.max_lines {
                // Move start forward (oldest line is overwritten)
                self.start = (self.start + 1) % self.max_lines;
            } else {
                self.len += 1;
            }
        }
    }

    /// Push multiple lines to the scrollback buffer
    pub fn push_lines(&mut self, lines: Vec<Line>) {
        for line in lines {
            self.push(line);
        }
    }

    /// Get a line by index (0 = oldest line in scrollback)
    pub fn get(&self, index: usize) -> Option<&Line> {
        if index >= self.len {
            return None;
        }
        let actual_index = (self.start + index) % self.lines.len();
        Some(&self.lines[actual_index])
    }

    /// Get a line by index from the bottom (0 = most recent line)
    pub fn get_from_bottom(&self, index: usize) -> Option<&Line> {
        if index >= self.len {
            return None;
        }
        self.get(self.len - 1 - index)
    }

    /// Clear the scrollback buffer
    pub fn clear(&mut self) {
        self.lines.clear();
        self.start = 0;
        self.len = 0;
    }

    /// Set the maximum number of lines
    /// If the new max is smaller, oldest lines are discarded
    pub fn set_max_lines(&mut self, max_lines: usize) {
        if max_lines < self.len {
            // Need to discard oldest lines
            let to_discard = self.len - max_lines;
            self.start = (self.start + to_discard) % self.lines.len();
            self.len = max_lines;
        }
        self.max_lines = max_lines;

        // Compact the buffer if needed
        if max_lines < self.lines.len() {
            let mut new_lines = Vec::with_capacity(max_lines);
            for i in 0..self.len {
                let actual_index = (self.start + i) % self.lines.len();
                new_lines.push(self.lines[actual_index].clone());
            }
            self.lines = new_lines;
            self.start = 0;
        }
    }

    /// Iterate over lines from oldest to newest
    pub fn iter(&self) -> ScrollbackIter<'_> {
        ScrollbackIter {
            scrollback: self,
            index: 0,
        }
    }

    /// Get all lines as a vector (oldest first)
    pub fn to_vec(&self) -> Vec<Line> {
        self.iter().cloned().collect()
    }
}

impl Default for Scrollback {
    fn default() -> Self {
        Self::new(DEFAULT_SCROLLBACK_LINES)
    }
}

/// Iterator over scrollback lines
pub struct ScrollbackIter<'a> {
    scrollback: &'a Scrollback,
    index: usize,
}

impl<'a> Iterator for ScrollbackIter<'a> {
    type Item = &'a Line;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.scrollback.len {
            return None;
        }
        let line = self.scrollback.get(self.index);
        self.index += 1;
        line
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.scrollback.len - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for ScrollbackIter<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrollback_new() {
        let sb = Scrollback::new(100);
        assert_eq!(sb.len(), 0);
        assert!(sb.is_empty());
        assert_eq!(sb.max_lines(), 100);
    }

    #[test]
    fn test_scrollback_push() {
        let mut sb = Scrollback::new(100);
        let mut line = Line::new(80);
        line.cell_mut(0).c = "A".to_string();
        sb.push(line);

        assert_eq!(sb.len(), 1);
        assert_eq!(sb.get(0).unwrap().cell(0).c, "A");
    }

    #[test]
    fn test_scrollback_ring_buffer() {
        let mut sb = Scrollback::new(3);

        for i in 0..5 {
            let mut line = Line::new(10);
            line.cell_mut(0).c = format!("{}", i);
            sb.push(line);
        }

        // Should only have last 3 lines
        assert_eq!(sb.len(), 3);
        assert_eq!(sb.get(0).unwrap().cell(0).c, "2");
        assert_eq!(sb.get(1).unwrap().cell(0).c, "3");
        assert_eq!(sb.get(2).unwrap().cell(0).c, "4");
    }

    #[test]
    fn test_scrollback_get_from_bottom() {
        let mut sb = Scrollback::new(100);

        for i in 0..5 {
            let mut line = Line::new(10);
            line.cell_mut(0).c = format!("{}", i);
            sb.push(line);
        }

        assert_eq!(sb.get_from_bottom(0).unwrap().cell(0).c, "4");
        assert_eq!(sb.get_from_bottom(1).unwrap().cell(0).c, "3");
        assert_eq!(sb.get_from_bottom(4).unwrap().cell(0).c, "0");
    }

    #[test]
    fn test_scrollback_clear() {
        let mut sb = Scrollback::new(100);
        sb.push(Line::new(80));
        sb.push(Line::new(80));

        sb.clear();
        assert!(sb.is_empty());
    }

    #[test]
    fn test_scrollback_iter() {
        let mut sb = Scrollback::new(100);

        for i in 0..5 {
            let mut line = Line::new(10);
            line.cell_mut(0).c = format!("{}", i);
            sb.push(line);
        }

        let chars: Vec<String> = sb.iter().map(|l| l.cell(0).c.clone()).collect();
        assert_eq!(chars, vec!["0", "1", "2", "3", "4"]);
    }

    #[test]
    fn test_scrollback_set_max_lines() {
        let mut sb = Scrollback::new(100);

        for i in 0..10 {
            let mut line = Line::new(10);
            line.cell_mut(0).c = format!("{}", i);
            sb.push(line);
        }

        sb.set_max_lines(5);
        assert_eq!(sb.len(), 5);
        assert_eq!(sb.max_lines(), 5);
        // Should have kept the newest 5 lines
        assert_eq!(sb.get(0).unwrap().cell(0).c, "5");
        assert_eq!(sb.get(4).unwrap().cell(0).c, "9");
    }

    #[test]
    fn test_scrollback_zero_max() {
        let mut sb = Scrollback::new(0);
        sb.push(Line::new(80));
        assert!(sb.is_empty());
    }
}
