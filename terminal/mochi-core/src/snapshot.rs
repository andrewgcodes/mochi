//! Terminal snapshot for testing and debugging
//!
//! Snapshots capture the complete terminal state in a serializable format
//! for deterministic testing and debugging.

use serde::{Deserialize, Serialize};

use crate::cell::Cell;
use crate::cursor::Cursor;
use crate::line::Line;
use crate::screen::ScreenMode;

/// A snapshot of the terminal state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Grid dimensions
    pub rows: usize,
    pub cols: usize,
    /// Grid content (row-major order)
    pub cells: Vec<Vec<CellSnapshot>>,
    /// Cursor state
    pub cursor: CursorSnapshot,
    /// Screen mode flags
    pub mode: ScreenMode,
    /// Scroll region
    pub scroll_top: usize,
    pub scroll_bottom: usize,
    /// Window title (if set)
    pub title: Option<String>,
}

/// Snapshot of a single cell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellSnapshot {
    pub c: String,
    pub fg: String,
    pub bg: String,
    pub flags: u16,
}

impl From<&Cell> for CellSnapshot {
    fn from(cell: &Cell) -> Self {
        CellSnapshot {
            c: cell.c.clone(),
            fg: format!("{:?}", cell.fg),
            bg: format!("{:?}", cell.bg),
            flags: cell.flags.bits(),
        }
    }
}

/// Snapshot of cursor state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorSnapshot {
    pub row: usize,
    pub col: usize,
    pub visible: bool,
    pub style: String,
}

impl From<&Cursor> for CursorSnapshot {
    fn from(cursor: &Cursor) -> Self {
        CursorSnapshot {
            row: cursor.row,
            col: cursor.col,
            visible: cursor.visible,
            style: format!("{:?}", cursor.style),
        }
    }
}

impl Snapshot {
    /// Create a snapshot from screen state
    pub fn from_screen(
        grid: &crate::grid::Grid,
        cursor: &Cursor,
        mode: &ScreenMode,
        scroll_top: usize,
        scroll_bottom: usize,
        title: Option<String>,
    ) -> Self {
        let cells: Vec<Vec<CellSnapshot>> = (0..grid.rows())
            .map(|row| {
                (0..grid.cols())
                    .map(|col| CellSnapshot::from(grid.cell(row, col)))
                    .collect()
            })
            .collect();

        Snapshot {
            rows: grid.rows(),
            cols: grid.cols(),
            cells,
            cursor: CursorSnapshot::from(cursor),
            mode: mode.clone(),
            scroll_top,
            scroll_bottom,
            title,
        }
    }

    /// Get the text content of the screen
    pub fn text(&self) -> String {
        self.cells
            .iter()
            .map(|row| {
                let line: String = row.iter().map(|c| c.c.as_str()).collect();
                line.trim_end().to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get the text content of a specific row
    pub fn row_text(&self, row: usize) -> String {
        if row >= self.cells.len() {
            return String::new();
        }
        let line: String = self.cells[row].iter().map(|c| c.c.as_str()).collect();
        line.trim_end().to_string()
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Create a simple text-based representation for debugging
    pub fn to_debug_string(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("Size: {}x{}\n", self.cols, self.rows));
        s.push_str(&format!(
            "Cursor: ({}, {}) visible={}\n",
            self.cursor.row, self.cursor.col, self.cursor.visible
        ));
        s.push_str(&format!(
            "Scroll region: {}-{}\n",
            self.scroll_top, self.scroll_bottom
        ));
        s.push_str("---\n");

        for (row_idx, row) in self.cells.iter().enumerate() {
            let line: String = row.iter().map(|c| c.c.as_str()).collect();
            let cursor_marker = if row_idx == self.cursor.row {
                format!(" <- cursor at col {}", self.cursor.col)
            } else {
                String::new()
            };
            s.push_str(&format!("{:3}|{}|{}\n", row_idx, line, cursor_marker));
        }

        s
    }
}

/// Simplified snapshot for golden tests (just text content)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextSnapshot {
    pub rows: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
}

impl TextSnapshot {
    pub fn from_snapshot(snapshot: &Snapshot) -> Self {
        TextSnapshot {
            rows: (0..snapshot.rows)
                .map(|r| snapshot.row_text(r))
                .collect(),
            cursor_row: snapshot.cursor.row,
            cursor_col: snapshot.cursor.col,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Grid;
    use crate::screen::ScreenMode;

    #[test]
    fn test_snapshot_text() {
        let mut grid = Grid::new(3, 10);
        grid.cell_mut(0, 0).c = "H".to_string();
        grid.cell_mut(0, 1).c = "e".to_string();
        grid.cell_mut(0, 2).c = "l".to_string();
        grid.cell_mut(0, 3).c = "l".to_string();
        grid.cell_mut(0, 4).c = "o".to_string();

        let cursor = Cursor::default();
        let mode = ScreenMode::new();

        let snapshot = Snapshot::from_screen(&grid, &cursor, &mode, 0, 2, None);

        assert_eq!(snapshot.row_text(0), "Hello");
        assert_eq!(snapshot.row_text(1), "");
    }

    #[test]
    fn test_snapshot_json_roundtrip() {
        let mut grid = Grid::new(2, 5);
        grid.cell_mut(0, 0).c = "A".to_string();

        let cursor = Cursor::default();
        let mode = ScreenMode::new();

        let snapshot = Snapshot::from_screen(&grid, &cursor, &mode, 0, 1, Some("Test".to_string()));
        let json = snapshot.to_json();
        let restored = Snapshot::from_json(&json).unwrap();

        assert_eq!(restored.rows, 2);
        assert_eq!(restored.cols, 5);
        assert_eq!(restored.title, Some("Test".to_string()));
    }
}
