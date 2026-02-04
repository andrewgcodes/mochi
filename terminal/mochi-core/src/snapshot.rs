//! Snapshot serialization for terminal state.
//!
//! Used for golden tests and debugging. Provides a deterministic
//! representation of the terminal state that can be compared across runs.

use crate::cell::Attributes;
use crate::color::Color;
use crate::line::Line;
use crate::screen::Screen;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub cols: usize,
    pub rows: usize,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub cursor_visible: bool,
    pub lines: Vec<LineSnapshot>,
    pub using_alternate: bool,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineSnapshot {
    pub text: String,
    pub cells: Vec<CellSnapshot>,
    pub wrapped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellSnapshot {
    pub char: char,
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attributes,
}

impl Snapshot {
    pub fn from_screen(screen: &Screen) -> Self {
        let mut lines = Vec::with_capacity(screen.rows());

        for row in 0..screen.rows() {
            if let Some(line) = screen.get_line(row) {
                lines.push(LineSnapshot::from_line(line));
            }
        }

        Snapshot {
            cols: screen.cols(),
            rows: screen.rows(),
            cursor_row: screen.cursor().row,
            cursor_col: screen.cursor().col,
            cursor_visible: screen.modes.cursor_visible,
            lines,
            using_alternate: screen.is_using_alternate(),
            title: screen.title.clone(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn text_content(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn visible_text(&self) -> Vec<String> {
        self.lines.iter().map(|l| l.text.clone()).collect()
    }
}

impl LineSnapshot {
    pub fn from_line(line: &Line) -> Self {
        let mut text = String::new();
        let mut cells = Vec::with_capacity(line.len());

        for cell in line.cells() {
            if !cell.is_wide_continuation() {
                text.push(cell.character);
            }
            cells.push(CellSnapshot {
                char: cell.character,
                fg: cell.fg,
                bg: cell.bg,
                attrs: cell.attrs,
            });
        }

        LineSnapshot {
            text: text.trim_end().to_string(),
            cells,
            wrapped: line.wrapped,
        }
    }
}

impl PartialEq for Snapshot {
    fn eq(&self, other: &Self) -> bool {
        self.cols == other.cols
            && self.rows == other.rows
            && self.cursor_row == other.cursor_row
            && self.cursor_col == other.cursor_col
            && self.lines.len() == other.lines.len()
            && self
                .lines
                .iter()
                .zip(other.lines.iter())
                .all(|(a, b)| a.text == b.text && a.wrapped == b.wrapped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_from_screen() {
        let mut screen = Screen::new(80, 24);
        screen.put_char('H');
        screen.put_char('i');

        let snapshot = Snapshot::from_screen(&screen);
        assert_eq!(snapshot.cols, 80);
        assert_eq!(snapshot.rows, 24);
        assert_eq!(snapshot.cursor_col, 2);
        assert_eq!(snapshot.lines[0].text, "Hi");
    }

    #[test]
    fn test_snapshot_json_roundtrip() {
        let mut screen = Screen::new(80, 24);
        screen.put_char('T');
        screen.put_char('e');
        screen.put_char('s');
        screen.put_char('t');

        let snapshot = Snapshot::from_screen(&screen);
        let json = snapshot.to_json();
        let restored = Snapshot::from_json(&json).unwrap();

        assert_eq!(snapshot, restored);
    }

    #[test]
    fn test_text_content() {
        let mut screen = Screen::new(80, 24);
        screen.put_char('A');
        screen.linefeed();
        screen.carriage_return();
        screen.put_char('B');

        let snapshot = Snapshot::from_screen(&screen);
        let text = snapshot.text_content();
        assert!(text.contains("A"));
        assert!(text.contains("B"));
    }
}
