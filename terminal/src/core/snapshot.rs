//! Deterministic snapshot generation
//!
//! Snapshots capture the complete terminal state in a serializable format
//! for testing and debugging. Given the same byte stream, the terminal
//! must produce identical snapshots.

use serde::{Deserialize, Serialize};

use super::cell::{Cell, Color, Style};
use super::cursor::{Cursor, CursorShape};
use super::screen::{Modes, MouseEncoding, MouseMode, Screen};

/// A complete snapshot of the terminal state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Screen dimensions
    pub cols: usize,
    pub rows: usize,
    /// Visible grid content (row-major)
    pub grid: Vec<Vec<CellSnapshot>>,
    /// Cursor state
    pub cursor: CursorSnapshot,
    /// Scroll region
    pub scroll_top: usize,
    pub scroll_bottom: usize,
    /// Terminal modes
    pub modes: ModesSnapshot,
    /// Window title
    pub title: String,
    /// Whether on alternate screen
    pub alternate_screen: bool,
    /// Scrollback line count
    pub scrollback_lines: usize,
}

/// Snapshot of a single cell
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellSnapshot {
    /// Character content
    pub content: String,
    /// Foreground color
    pub fg: ColorSnapshot,
    /// Background color
    pub bg: ColorSnapshot,
    /// Style attributes
    pub style: StyleSnapshot,
    /// Cell width (0 for continuation, 1 normal, 2 wide)
    pub width: u8,
}

/// Snapshot of a color
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ColorSnapshot {
    Default,
    Indexed { index: u8 },
    Rgb { r: u8, g: u8, b: u8 },
}

/// Snapshot of style attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StyleSnapshot {
    #[serde(default, skip_serializing_if = "is_false")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub faint: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub underline: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub blink: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub inverse: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub strikethrough: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Snapshot of cursor state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorSnapshot {
    pub col: usize,
    pub row: usize,
    pub visible: bool,
    pub shape: String,
    pub blinking: bool,
}

/// Snapshot of terminal modes
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModesSnapshot {
    #[serde(default, skip_serializing_if = "is_false")]
    pub application_cursor: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub application_keypad: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub bracketed_paste: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub focus_reporting: bool,
    pub mouse_tracking: String,
    pub mouse_encoding: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub linefeed_mode: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub reverse_video: bool,
}

impl From<&Color> for ColorSnapshot {
    fn from(color: &Color) -> Self {
        match color {
            Color::Default => ColorSnapshot::Default,
            Color::Indexed(i) => ColorSnapshot::Indexed { index: *i },
            Color::Rgb(r, g, b) => ColorSnapshot::Rgb {
                r: *r,
                g: *g,
                b: *b,
            },
        }
    }
}

impl From<&Style> for StyleSnapshot {
    fn from(style: &Style) -> Self {
        StyleSnapshot {
            bold: style.bold,
            faint: style.faint,
            italic: style.italic,
            underline: style.underline,
            blink: style.blink,
            inverse: style.inverse,
            hidden: style.hidden,
            strikethrough: style.strikethrough,
        }
    }
}

impl From<&Cell> for CellSnapshot {
    fn from(cell: &Cell) -> Self {
        CellSnapshot {
            content: cell.content.clone(),
            fg: ColorSnapshot::from(&cell.fg),
            bg: ColorSnapshot::from(&cell.bg),
            style: StyleSnapshot::from(&cell.style),
            width: cell.width,
        }
    }
}

impl From<&Cursor> for CursorSnapshot {
    fn from(cursor: &Cursor) -> Self {
        CursorSnapshot {
            col: cursor.col,
            row: cursor.row,
            visible: cursor.visible,
            shape: match cursor.shape {
                CursorShape::Block => "block".to_string(),
                CursorShape::Underline => "underline".to_string(),
                CursorShape::Bar => "bar".to_string(),
            },
            blinking: cursor.blinking,
        }
    }
}

impl From<&Modes> for ModesSnapshot {
    fn from(modes: &Modes) -> Self {
        ModesSnapshot {
            application_cursor: modes.application_cursor,
            application_keypad: modes.application_keypad,
            bracketed_paste: modes.bracketed_paste,
            focus_reporting: modes.focus_reporting,
            mouse_tracking: match modes.mouse_tracking {
                MouseMode::None => "none".to_string(),
                MouseMode::X10 => "x10".to_string(),
                MouseMode::Normal => "normal".to_string(),
                MouseMode::Highlight => "highlight".to_string(),
                MouseMode::ButtonEvent => "button_event".to_string(),
                MouseMode::AnyEvent => "any_event".to_string(),
            },
            mouse_encoding: match modes.mouse_encoding {
                MouseEncoding::X10 => "x10".to_string(),
                MouseEncoding::Utf8 => "utf8".to_string(),
                MouseEncoding::Sgr => "sgr".to_string(),
                MouseEncoding::Urxvt => "urxvt".to_string(),
            },
            linefeed_mode: modes.linefeed_mode,
            reverse_video: modes.reverse_video,
        }
    }
}

impl Snapshot {
    /// Create a snapshot from the current screen state
    pub fn from_screen(screen: &Screen) -> Self {
        let mut grid = Vec::with_capacity(screen.rows());

        for row in 0..screen.rows() {
            let mut row_cells = Vec::with_capacity(screen.cols());
            for col in 0..screen.cols() {
                if let Some(cell) = screen.get_cell(col, row) {
                    row_cells.push(CellSnapshot::from(cell));
                } else {
                    row_cells.push(CellSnapshot {
                        content: String::new(),
                        fg: ColorSnapshot::Default,
                        bg: ColorSnapshot::Default,
                        style: StyleSnapshot::default(),
                        width: 1,
                    });
                }
            }
            grid.push(row_cells);
        }

        Snapshot {
            cols: screen.cols(),
            rows: screen.rows(),
            grid,
            cursor: CursorSnapshot::from(screen.cursor()),
            scroll_top: screen.scroll_top(),
            scroll_bottom: screen.scroll_bottom(),
            modes: ModesSnapshot::from(&screen.modes),
            title: screen.title.clone(),
            alternate_screen: screen.modes.alternate_screen,
            scrollback_lines: screen.scrollback().len(),
        }
    }

    /// Convert snapshot to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Parse snapshot from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Get a simple text representation of the screen (for debugging)
    pub fn to_text(&self) -> String {
        let mut result = String::new();

        for row in &self.grid {
            for cell in row {
                if cell.width == 0 {
                    continue; // Skip continuation cells
                }
                if cell.content.is_empty() {
                    result.push(' ');
                } else {
                    result.push_str(&cell.content);
                }
            }
            // Trim trailing spaces and add newline
            while result.ends_with(' ') {
                result.pop();
            }
            result.push('\n');
        }

        // Remove trailing empty lines
        while result.ends_with("\n\n") {
            result.pop();
        }

        result
    }

    /// Compare two snapshots for equality (ignoring non-essential fields)
    pub fn content_equals(&self, other: &Snapshot) -> bool {
        if self.cols != other.cols || self.rows != other.rows {
            return false;
        }

        for (row_a, row_b) in self.grid.iter().zip(other.grid.iter()) {
            for (cell_a, cell_b) in row_a.iter().zip(row_b.iter()) {
                if cell_a != cell_b {
                    return false;
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_from_screen() {
        let mut screen = Screen::new(10, 3, 100);
        screen.print_char('H');
        screen.print_char('i');

        let snapshot = Snapshot::from_screen(&screen);

        assert_eq!(snapshot.cols, 10);
        assert_eq!(snapshot.rows, 3);
        assert_eq!(snapshot.grid[0][0].content, "H");
        assert_eq!(snapshot.grid[0][1].content, "i");
        assert_eq!(snapshot.cursor.col, 2);
        assert_eq!(snapshot.cursor.row, 0);
    }

    #[test]
    fn test_snapshot_to_text() {
        let mut screen = Screen::new(10, 3, 100);
        screen.print_char('A');
        screen.print_char('B');
        screen.linefeed();
        screen.carriage_return();
        screen.print_char('C');

        let snapshot = Snapshot::from_screen(&screen);
        let text = snapshot.to_text();

        assert!(text.contains("AB"));
        assert!(text.contains("C"));
    }

    #[test]
    fn test_snapshot_json_roundtrip() {
        let mut screen = Screen::new(5, 2, 100);
        screen.print_char('X');
        screen.cursor_mut().style.bold = true;
        screen.cursor_mut().fg = Color::RED;
        screen.print_char('Y');

        let snapshot = Snapshot::from_screen(&screen);
        let json = snapshot.to_json().unwrap();
        let restored = Snapshot::from_json(&json).unwrap();

        assert!(snapshot.content_equals(&restored));
    }

    #[test]
    fn test_color_snapshot() {
        assert_eq!(ColorSnapshot::from(&Color::Default), ColorSnapshot::Default);
        assert_eq!(
            ColorSnapshot::from(&Color::Indexed(5)),
            ColorSnapshot::Indexed { index: 5 }
        );
        assert_eq!(
            ColorSnapshot::from(&Color::Rgb(255, 128, 0)),
            ColorSnapshot::Rgb {
                r: 255,
                g: 128,
                b: 0
            }
        );
    }

    #[test]
    fn test_style_snapshot() {
        let style = Style {
            bold: true,
            underline: true,
            ..Default::default()
        };

        let snapshot = StyleSnapshot::from(&style);
        assert!(snapshot.bold);
        assert!(snapshot.underline);
        assert!(!snapshot.italic);
    }
}
