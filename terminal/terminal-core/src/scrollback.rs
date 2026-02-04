//! Scrollback buffer for terminal history
//!
//! Implements a ring buffer of lines that have scrolled off the top of the screen.

use serde::{Deserialize, Serialize};

use crate::line::Line;

/// Default maximum scrollback lines
pub const DEFAULT_SCROLLBACK_SIZE: usize = 10000;

/// Scrollback buffer using a ring buffer implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scrollback {
    /// Ring buffer of lines
    lines: Vec<Line>,
    /// Maximum number of lines to store
    max_lines: usize,
    /// Start index in the ring buffer
    start: usize,
    /// Number of lines currently stored
    len: usize,
}

impl Scrollback {
    /// Create a new scrollback buffer with the specified maximum size
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: Vec::with_capacity(max_lines.min(1000)), // Don't pre-allocate too much
            max_lines,
            start: 0,
            len: 0,
        }
    }

    /// Get the maximum number of lines
    pub fn max_lines(&self) -> usize {
        self.max_lines
    }

    /// Get the current number of lines
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the scrollback is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
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
            let idx = (self.start + self.len) % self.max_lines;
            self.lines[idx] = line;
            if self.len < self.max_lines {
                self.len += 1;
            } else {
                // Move start forward (oldest line is overwritten)
                self.start = (self.start + 1) % self.max_lines;
            }
        }
    }

    /// Push multiple lines to the scrollback buffer
    pub fn push_lines(&mut self, lines: Vec<Line>) {
        for line in lines {
            self.push(line);
        }
    }

    /// Get a line by index (0 = oldest, len-1 = newest)
    pub fn get(&self, index: usize) -> Option<&Line> {
        if index >= self.len {
            return None;
        }
        let actual_idx = (self.start + index) % self.lines.len();
        self.lines.get(actual_idx)
    }

    /// Get a line from the end (0 = newest, len-1 = oldest)
    pub fn get_from_end(&self, index: usize) -> Option<&Line> {
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

    /// Resize the maximum scrollback size
    pub fn resize(&mut self, max_lines: usize) {
        if max_lines == self.max_lines {
            return;
        }

        if max_lines == 0 {
            self.clear();
            self.max_lines = 0;
            return;
        }

        if max_lines < self.len {
            // Need to drop oldest lines
            let to_drop = self.len - max_lines;
            self.start = (self.start + to_drop) % self.lines.len();
            self.len = max_lines;
        }

        // Reorganize buffer if needed
        if self.start != 0 && !self.lines.is_empty() {
            let mut new_lines = Vec::with_capacity(max_lines.min(self.len));
            for i in 0..self.len {
                if let Some(line) = self.get(i) {
                    new_lines.push(line.clone());
                }
            }
            self.lines = new_lines;
            self.start = 0;
        }

        self.max_lines = max_lines;
        self.lines.truncate(max_lines);
    }

    /// Iterator over lines from oldest to newest
    pub fn iter(&self) -> impl Iterator<Item = &Line> {
        ScrollbackIter {
            scrollback: self,
            index: 0,
        }
    }

    /// Iterator over lines from newest to oldest
    pub fn iter_rev(&self) -> impl Iterator<Item = &Line> {
        ScrollbackRevIter {
            scrollback: self,
            index: 0,
        }
    }
}

impl Default for Scrollback {
    fn default() -> Self {
        Self::new(DEFAULT_SCROLLBACK_SIZE)
    }
}

/// Iterator over scrollback lines (oldest to newest)
struct ScrollbackIter<'a> {
    scrollback: &'a Scrollback,
    index: usize,
}

impl<'a> Iterator for ScrollbackIter<'a> {
    type Item = &'a Line;

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.scrollback.get(self.index)?;
        self.index += 1;
        Some(line)
    }
}

/// Iterator over scrollback lines (newest to oldest)
struct ScrollbackRevIter<'a> {
    scrollback: &'a Scrollback,
    index: usize,
}

impl<'a> Iterator for ScrollbackRevIter<'a> {
    type Item = &'a Line;

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.scrollback.get_from_end(self.index)?;
        self.index += 1;
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_line(text: &str) -> Line {
        let mut line = Line::new(text.len().max(10));
        for (i, c) in text.chars().enumerate() {
            line.cell_mut(i).set_char(c);
        }
        line
    }

    #[test]
    fn test_scrollback_new() {
        let sb = Scrollback::new(100);
        assert_eq!(sb.max_lines(), 100);
        assert_eq!(sb.len(), 0);
        assert!(sb.is_empty());
    }

    #[test]
    fn test_scrollback_push() {
        let mut sb = Scrollback::new(100);
        sb.push(make_line("line1"));
        sb.push(make_line("line2"));

        assert_eq!(sb.len(), 2);
        assert_eq!(sb.get(0).unwrap().text(), "line1");
        assert_eq!(sb.get(1).unwrap().text(), "line2");
    }

    #[test]
    fn test_scrollback_ring_buffer() {
        let mut sb = Scrollback::new(3);
        sb.push(make_line("line1"));
        sb.push(make_line("line2"));
        sb.push(make_line("line3"));
        sb.push(make_line("line4")); // Should overwrite line1

        assert_eq!(sb.len(), 3);
        assert_eq!(sb.get(0).unwrap().text(), "line2");
        assert_eq!(sb.get(1).unwrap().text(), "line3");
        assert_eq!(sb.get(2).unwrap().text(), "line4");
    }

    #[test]
    fn test_scrollback_get_from_end() {
        let mut sb = Scrollback::new(100);
        sb.push(make_line("line1"));
        sb.push(make_line("line2"));
        sb.push(make_line("line3"));

        assert_eq!(sb.get_from_end(0).unwrap().text(), "line3");
        assert_eq!(sb.get_from_end(1).unwrap().text(), "line2");
        assert_eq!(sb.get_from_end(2).unwrap().text(), "line1");
    }

    #[test]
    fn test_scrollback_clear() {
        let mut sb = Scrollback::new(100);
        sb.push(make_line("line1"));
        sb.push(make_line("line2"));

        sb.clear();

        assert!(sb.is_empty());
        assert_eq!(sb.len(), 0);
    }

    #[test]
    fn test_scrollback_iter() {
        let mut sb = Scrollback::new(100);
        sb.push(make_line("line1"));
        sb.push(make_line("line2"));
        sb.push(make_line("line3"));

        let texts: Vec<_> = sb.iter().map(|l| l.text()).collect();
        assert_eq!(texts, vec!["line1", "line2", "line3"]);
    }

    #[test]
    fn test_scrollback_iter_rev() {
        let mut sb = Scrollback::new(100);
        sb.push(make_line("line1"));
        sb.push(make_line("line2"));
        sb.push(make_line("line3"));

        let texts: Vec<_> = sb.iter_rev().map(|l| l.text()).collect();
        assert_eq!(texts, vec!["line3", "line2", "line1"]);
    }

    #[test]
    fn test_scrollback_resize_smaller() {
        let mut sb = Scrollback::new(100);
        for i in 0..10 {
            sb.push(make_line(&format!("line{}", i)));
        }

        sb.resize(5);

        assert_eq!(sb.len(), 5);
        assert_eq!(sb.max_lines(), 5);
        // Should keep newest lines
        assert_eq!(sb.get(0).unwrap().text(), "line5");
    }
}
