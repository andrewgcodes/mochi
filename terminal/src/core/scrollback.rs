//! Scrollback buffer implementation
//!
//! The scrollback buffer stores lines that have scrolled off the top of the
//! visible screen. It's implemented as a ring buffer with a configurable
//! maximum size.

use serde::{Deserialize, Serialize};

use super::cell::Cell;

/// A line in the terminal, consisting of cells and metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    /// The cells in this line
    pub cells: Vec<Cell>,
    /// Whether this line was soft-wrapped (continued from previous line)
    pub wrapped: bool,
}

impl Line {
    /// Create a new line with the given number of columns
    pub fn new(cols: usize) -> Self {
        Self {
            cells: vec![Cell::default(); cols],
            wrapped: false,
        }
    }

    /// Create a line from existing cells
    pub fn from_cells(cells: Vec<Cell>, wrapped: bool) -> Self {
        Self { cells, wrapped }
    }

    /// Resize the line to a new column count
    pub fn resize(&mut self, cols: usize) {
        if cols > self.cells.len() {
            self.cells.resize(cols, Cell::default());
        } else {
            self.cells.truncate(cols);
        }
    }

    /// Clear all cells in the line
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.clear();
        }
        self.wrapped = false;
    }

    /// Get a cell at the given column
    pub fn get(&self, col: usize) -> Option<&Cell> {
        self.cells.get(col)
    }

    /// Get a mutable cell at the given column
    pub fn get_mut(&mut self, col: usize) -> Option<&mut Cell> {
        self.cells.get_mut(col)
    }

    /// Check if the line is empty (all cells are empty)
    pub fn is_empty(&self) -> bool {
        self.cells.iter().all(|c| c.is_empty())
    }

    /// Get the rightmost non-empty column (for selection/copy)
    pub fn last_non_empty(&self) -> Option<usize> {
        self.cells
            .iter()
            .enumerate()
            .rev()
            .find(|(_, c)| !c.is_empty())
            .map(|(i, _)| i)
    }

    /// Extract text content from the line
    pub fn to_string(&self) -> String {
        let mut s = String::new();
        for cell in &self.cells {
            if cell.is_continuation() {
                continue; // Skip continuation cells for wide chars
            }
            if cell.is_empty() {
                s.push(' ');
            } else {
                s.push_str(&cell.content);
            }
        }
        // Trim trailing spaces
        s.trim_end().to_string()
    }
}

/// Ring buffer for scrollback lines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scrollback {
    /// The lines in the scrollback buffer
    lines: Vec<Line>,
    /// Index of the oldest line (head of ring buffer)
    head: usize,
    /// Number of lines currently in the buffer
    len: usize,
    /// Maximum number of lines to store
    capacity: usize,
}

impl Scrollback {
    /// Create a new scrollback buffer with the given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            lines: Vec::with_capacity(capacity.min(1000)), // Don't pre-allocate too much
            head: 0,
            len: 0,
            capacity,
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

    /// Get the maximum capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Push a line into the scrollback buffer
    pub fn push(&mut self, line: Line) {
        if self.capacity == 0 {
            return;
        }

        if self.lines.len() < self.capacity {
            // Buffer not yet full, just append
            self.lines.push(line);
            self.len += 1;
        } else {
            // Buffer full, overwrite oldest
            let idx = (self.head + self.len) % self.capacity;
            self.lines[idx] = line;
            if self.len < self.capacity {
                self.len += 1;
            } else {
                // Move head forward (oldest line is overwritten)
                self.head = (self.head + 1) % self.capacity;
            }
        }
    }

    /// Get a line by index (0 = oldest line in scrollback)
    pub fn get(&self, index: usize) -> Option<&Line> {
        if index >= self.len {
            return None;
        }
        let actual_idx = (self.head + index) % self.lines.len();
        self.lines.get(actual_idx)
    }

    /// Get a line by index from the end (0 = most recent line)
    pub fn get_from_end(&self, index: usize) -> Option<&Line> {
        if index >= self.len {
            return None;
        }
        self.get(self.len - 1 - index)
    }

    /// Clear all lines from the scrollback
    pub fn clear(&mut self) {
        self.lines.clear();
        self.head = 0;
        self.len = 0;
    }

    /// Iterate over all lines from oldest to newest
    pub fn iter(&self) -> ScrollbackIter<'_> {
        ScrollbackIter {
            scrollback: self,
            index: 0,
        }
    }

    /// Set a new capacity, potentially truncating old lines
    pub fn set_capacity(&mut self, new_capacity: usize) {
        if new_capacity == self.capacity {
            return;
        }

        if new_capacity == 0 {
            self.clear();
            self.capacity = 0;
            return;
        }

        // If shrinking, we need to keep only the most recent lines
        if new_capacity < self.len {
            let mut new_lines = Vec::with_capacity(new_capacity);
            let start = self.len - new_capacity;
            for i in start..self.len {
                if let Some(line) = self.get(i) {
                    new_lines.push(line.clone());
                }
            }
            self.lines = new_lines;
            self.head = 0;
            self.len = new_capacity;
        }

        self.capacity = new_capacity;
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
        let line = self.scrollback.get(self.index)?;
        self.index += 1;
        Some(line)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.scrollback.len.saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for ScrollbackIter<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_new() {
        let line = Line::new(80);
        assert_eq!(line.cells.len(), 80);
        assert!(!line.wrapped);
        assert!(line.is_empty());
    }

    #[test]
    fn test_line_resize() {
        let mut line = Line::new(80);
        line.resize(40);
        assert_eq!(line.cells.len(), 40);
        line.resize(100);
        assert_eq!(line.cells.len(), 100);
    }

    #[test]
    fn test_line_to_string() {
        let mut line = Line::new(10);
        line.cells[0].set_char('H');
        line.cells[1].set_char('i');
        assert_eq!(line.to_string(), "Hi");
    }

    #[test]
    fn test_line_last_non_empty() {
        let mut line = Line::new(10);
        assert_eq!(line.last_non_empty(), None);
        line.cells[3].set_char('X');
        assert_eq!(line.last_non_empty(), Some(3));
        line.cells[7].set_char('Y');
        assert_eq!(line.last_non_empty(), Some(7));
    }

    #[test]
    fn test_scrollback_push_and_get() {
        let mut sb = Scrollback::new(5);
        assert!(sb.is_empty());

        for i in 0..3 {
            let mut line = Line::new(10);
            line.cells[0].set_char(char::from_digit(i as u32, 10).unwrap());
            sb.push(line);
        }

        assert_eq!(sb.len(), 3);
        assert_eq!(sb.get(0).unwrap().cells[0].display_char(), '0');
        assert_eq!(sb.get(2).unwrap().cells[0].display_char(), '2');
    }

    #[test]
    fn test_scrollback_ring_buffer() {
        let mut sb = Scrollback::new(3);

        // Push 5 lines into a buffer of capacity 3
        for i in 0..5 {
            let mut line = Line::new(10);
            line.cells[0].set_char(char::from_digit(i as u32, 10).unwrap());
            sb.push(line);
        }

        // Should only have the last 3 lines (2, 3, 4)
        assert_eq!(sb.len(), 3);
        assert_eq!(sb.get(0).unwrap().cells[0].display_char(), '2');
        assert_eq!(sb.get(1).unwrap().cells[0].display_char(), '3');
        assert_eq!(sb.get(2).unwrap().cells[0].display_char(), '4');
    }

    #[test]
    fn test_scrollback_get_from_end() {
        let mut sb = Scrollback::new(5);
        for i in 0..3 {
            let mut line = Line::new(10);
            line.cells[0].set_char(char::from_digit(i as u32, 10).unwrap());
            sb.push(line);
        }

        assert_eq!(sb.get_from_end(0).unwrap().cells[0].display_char(), '2');
        assert_eq!(sb.get_from_end(2).unwrap().cells[0].display_char(), '0');
    }

    #[test]
    fn test_scrollback_iter() {
        let mut sb = Scrollback::new(5);
        for i in 0..3 {
            let mut line = Line::new(10);
            line.cells[0].set_char(char::from_digit(i as u32, 10).unwrap());
            sb.push(line);
        }

        let chars: Vec<char> = sb.iter().map(|l| l.cells[0].display_char()).collect();
        assert_eq!(chars, vec!['0', '1', '2']);
    }

    #[test]
    fn test_scrollback_zero_capacity() {
        let mut sb = Scrollback::new(0);
        sb.push(Line::new(10));
        assert!(sb.is_empty());
    }

    #[test]
    fn test_scrollback_set_capacity() {
        let mut sb = Scrollback::new(10);
        for i in 0..5 {
            let mut line = Line::new(10);
            line.cells[0].set_char(char::from_digit(i as u32, 10).unwrap());
            sb.push(line);
        }

        // Shrink to 3 - should keep lines 2, 3, 4
        sb.set_capacity(3);
        assert_eq!(sb.len(), 3);
        assert_eq!(sb.get(0).unwrap().cells[0].display_char(), '2');
    }
}
