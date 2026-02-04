//! Snapshot serialization for testing
//!
//! Provides serializable snapshots of terminal state for golden tests.

use serde::{Deserialize, Serialize};

use super::cell::{CellAttributes, Color};
use super::cursor::CursorStyle;
use super::modes::{MouseEncoding, MouseMode};
use super::screen::Screen;

/// A serializable snapshot of the terminal state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Terminal dimensions
    pub rows: usize,
    pub cols: usize,
    /// Cursor state
    pub cursor: CursorSnapshot,
    /// Screen lines
    pub lines: Vec<LineSnapshot>,
    /// Terminal modes
    pub modes: ModesSnapshot,
    /// Current attributes
    pub current_fg: Color,
    pub current_bg: Color,
    pub current_attrs: CellAttributes,
    /// Scroll region
    pub scroll_top: usize,
    pub scroll_bottom: usize,
    /// Window title
    pub title: String,
}

/// Snapshot of cursor state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorSnapshot {
    pub row: usize,
    pub col: usize,
    pub visible: bool,
    pub style: CursorStyle,
    pub pending_wrap: bool,
}

/// Snapshot of a line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineSnapshot {
    pub cells: Vec<CellSnapshot>,
    pub wrapped: bool,
}

/// Snapshot of a cell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellSnapshot {
    pub content: String,
    pub fg: Color,
    pub bg: Color,
    pub attrs: CellAttributes,
}

/// Snapshot of terminal modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModesSnapshot {
    pub autowrap: bool,
    pub origin: bool,
    pub insert: bool,
    pub cursor_keys_application: bool,
    pub keypad_application: bool,
    pub bracketed_paste: bool,
    pub mouse_mode: MouseMode,
    pub mouse_encoding: MouseEncoding,
    pub focus_reporting: bool,
    pub alternate_screen: bool,
}

impl Snapshot {
    /// Create a snapshot from a screen
    pub fn from_screen(screen: &Screen) -> Self {
        let grid = screen.grid();
        let cursor = screen.cursor();
        let modes = screen.modes();

        let lines: Vec<LineSnapshot> = (0..screen.rows())
            .map(|row| {
                let line = grid.line(row).unwrap();
                LineSnapshot {
                    cells: line
                        .cells()
                        .iter()
                        .map(|cell| CellSnapshot {
                            content: cell.content().to_string(),
                            fg: cell.fg(),
                            bg: cell.bg(),
                            attrs: *cell.attrs(),
                        })
                        .collect(),
                    wrapped: line.is_wrapped(),
                }
            })
            .collect();

        Self {
            rows: screen.rows(),
            cols: screen.cols(),
            cursor: CursorSnapshot {
                row: cursor.row(),
                col: cursor.col(),
                visible: cursor.is_visible(),
                style: cursor.style(),
                pending_wrap: cursor.pending_wrap(),
            },
            lines,
            modes: ModesSnapshot {
                autowrap: modes.autowrap,
                origin: modes.origin,
                insert: modes.insert,
                cursor_keys_application: modes.cursor_keys_application,
                keypad_application: modes.keypad_application,
                bracketed_paste: modes.bracketed_paste,
                mouse_mode: modes.mouse_mode,
                mouse_encoding: modes.mouse_encoding,
                focus_reporting: modes.focus_reporting,
                alternate_screen: modes.alternate_screen,
            },
            current_fg: screen.current_fg(),
            current_bg: screen.current_bg(),
            current_attrs: *screen.current_attrs(),
            scroll_top: 0, // TODO: expose scroll region
            scroll_bottom: screen.rows() - 1,
            title: screen.title().to_string(),
        }
    }

    /// Get the text content of the screen
    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(|line| {
                let text: String = line
                    .cells
                    .iter()
                    .map(|cell| {
                        if cell.content.is_empty() {
                            ' '
                        } else {
                            cell.content.chars().next().unwrap_or(' ')
                        }
                    })
                    .collect();
                text.trim_end().to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Compare two snapshots and return differences
    pub fn diff(&self, other: &Snapshot) -> Vec<String> {
        let mut diffs = Vec::new();

        if self.rows != other.rows {
            diffs.push(format!("rows: {} vs {}", self.rows, other.rows));
        }
        if self.cols != other.cols {
            diffs.push(format!("cols: {} vs {}", self.cols, other.cols));
        }
        if self.cursor.row != other.cursor.row {
            diffs.push(format!(
                "cursor.row: {} vs {}",
                self.cursor.row, other.cursor.row
            ));
        }
        if self.cursor.col != other.cursor.col {
            diffs.push(format!(
                "cursor.col: {} vs {}",
                self.cursor.col, other.cursor.col
            ));
        }

        // Compare lines
        for (i, (a, b)) in self.lines.iter().zip(other.lines.iter()).enumerate() {
            for (j, (ca, cb)) in a.cells.iter().zip(b.cells.iter()).enumerate() {
                if ca.content != cb.content {
                    diffs.push(format!(
                        "cell[{},{}].content: {:?} vs {:?}",
                        i, j, ca.content, cb.content
                    ));
                }
                if ca.fg != cb.fg {
                    diffs.push(format!("cell[{},{}].fg: {:?} vs {:?}", i, j, ca.fg, cb.fg));
                }
                if ca.bg != cb.bg {
                    diffs.push(format!("cell[{},{}].bg: {:?} vs {:?}", i, j, ca.bg, cb.bg));
                }
            }
        }

        diffs
    }
}

/// A compact snapshot for simple tests (just text and cursor)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactSnapshot {
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub text: Vec<String>,
}

impl CompactSnapshot {
    /// Create a compact snapshot from a screen
    pub fn from_screen(screen: &Screen) -> Self {
        let grid = screen.grid();
        let text: Vec<String> = (0..screen.rows())
            .map(|row| grid.line(row).map(|line| line.text()).unwrap_or_default())
            .collect();

        Self {
            cursor_row: screen.cursor().row(),
            cursor_col: screen.cursor().col(),
            text,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_from_screen() {
        let mut screen = Screen::new(10, 5);
        screen.apply(crate::parser::TerminalAction::Print('H'));
        screen.apply(crate::parser::TerminalAction::Print('i'));

        let snapshot = Snapshot::from_screen(&screen);

        assert_eq!(snapshot.rows, 5);
        assert_eq!(snapshot.cols, 10);
        assert_eq!(snapshot.cursor.row, 0);
        assert_eq!(snapshot.cursor.col, 2);
        assert_eq!(snapshot.lines[0].cells[0].content, "H");
        assert_eq!(snapshot.lines[0].cells[1].content, "i");
    }

    #[test]
    fn test_snapshot_text() {
        let mut screen = Screen::new(10, 3);
        screen.apply(crate::parser::TerminalAction::Print('A'));
        screen.apply(crate::parser::TerminalAction::Print('B'));
        screen.apply(crate::parser::TerminalAction::Execute(0x0A)); // LF
        screen.apply(crate::parser::TerminalAction::Print('C'));

        let snapshot = Snapshot::from_screen(&screen);
        let text = snapshot.text();

        assert!(text.contains("AB"));
        assert!(text.contains("C"));
    }

    #[test]
    fn test_compact_snapshot() {
        let mut screen = Screen::new(10, 3);
        screen.apply(crate::parser::TerminalAction::Print('X'));

        let snapshot = CompactSnapshot::from_screen(&screen);

        assert_eq!(snapshot.cursor_row, 0);
        assert_eq!(snapshot.cursor_col, 1);
        assert_eq!(snapshot.text[0], "X");
    }

    #[test]
    fn test_snapshot_serialization() {
        let screen = Screen::new(10, 5);
        let snapshot = Snapshot::from_screen(&screen);

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: Snapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(snapshot.rows, restored.rows);
        assert_eq!(snapshot.cols, restored.cols);
    }
}
