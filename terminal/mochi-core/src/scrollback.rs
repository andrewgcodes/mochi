//! Scrollback buffer for terminal history.
//!
//! Implements a ring buffer of lines that stores terminal history
//! when lines scroll off the top of the screen.

use crate::line::Line;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scrollback {
    lines: VecDeque<Line>,
    max_lines: usize,
}

impl Scrollback {
    pub fn new(max_lines: usize) -> Self {
        Scrollback {
            lines: VecDeque::new(),
            max_lines,
        }
    }

    pub fn push(&mut self, line: Line) {
        if self.lines.len() >= self.max_lines {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    pub fn pop(&mut self) -> Option<Line> {
        self.lines.pop_back()
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn max_lines(&self) -> usize {
        self.max_lines
    }

    pub fn set_max_lines(&mut self, max_lines: usize) {
        self.max_lines = max_lines;
        while self.lines.len() > max_lines {
            self.lines.pop_front();
        }
    }

    pub fn get(&self, index: usize) -> Option<&Line> {
        self.lines.get(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Line> {
        self.lines.iter()
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn get_from_end(&self, offset: usize) -> Option<&Line> {
        if offset >= self.lines.len() {
            None
        } else {
            self.lines.get(self.lines.len() - 1 - offset)
        }
    }
}

impl Default for Scrollback {
    fn default() -> Self {
        Scrollback::new(crate::DEFAULT_SCROLLBACK_LINES)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_len() {
        let mut sb = Scrollback::new(100);
        assert_eq!(sb.len(), 0);
        
        sb.push(Line::new(80));
        assert_eq!(sb.len(), 1);
        
        sb.push(Line::new(80));
        assert_eq!(sb.len(), 2);
    }

    #[test]
    fn test_max_lines_limit() {
        let mut sb = Scrollback::new(3);
        
        for i in 0..5 {
            let mut line = Line::new(80);
            line.set(0, crate::cell::Cell::new(('A' as u8 + i as u8) as char));
            sb.push(line);
        }
        
        assert_eq!(sb.len(), 3);
        assert_eq!(sb.get(0).unwrap().get(0).unwrap().character, 'C');
        assert_eq!(sb.get(1).unwrap().get(0).unwrap().character, 'D');
        assert_eq!(sb.get(2).unwrap().get(0).unwrap().character, 'E');
    }

    #[test]
    fn test_pop() {
        let mut sb = Scrollback::new(100);
        sb.push(Line::new(80));
        sb.push(Line::new(80));
        
        assert_eq!(sb.len(), 2);
        sb.pop();
        assert_eq!(sb.len(), 1);
    }

    #[test]
    fn test_get_from_end() {
        let mut sb = Scrollback::new(100);
        
        for i in 0..5 {
            let mut line = Line::new(80);
            line.set(0, crate::cell::Cell::new(('A' as u8 + i as u8) as char));
            sb.push(line);
        }
        
        assert_eq!(sb.get_from_end(0).unwrap().get(0).unwrap().character, 'E');
        assert_eq!(sb.get_from_end(1).unwrap().get(0).unwrap().character, 'D');
        assert_eq!(sb.get_from_end(4).unwrap().get(0).unwrap().character, 'A');
        assert!(sb.get_from_end(5).is_none());
    }

    #[test]
    fn test_clear() {
        let mut sb = Scrollback::new(100);
        sb.push(Line::new(80));
        sb.push(Line::new(80));
        sb.clear();
        assert_eq!(sb.len(), 0);
    }
}
