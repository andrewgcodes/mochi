//! Terminal snapshot for testing and debugging
//!
//! Provides a serializable representation of terminal state.

use serde::{Deserialize, Serialize};

use crate::cursor::{Cursor, CursorStyle};
use crate::grid::Grid;
use crate::modes::Modes;
use crate::scrollback::Scrollback;
use crate::selection::Selection;
use crate::Dimensions;

/// A complete snapshot of terminal state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Grid dimensions
    pub dimensions: SnapshotDimensions,
    /// Cursor state
    pub cursor: SnapshotCursor,
    /// Screen content (rows of text with attributes)
    pub screen: Vec<SnapshotLine>,
    /// Scrollback content (if included)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scrollback: Option<Vec<SnapshotLine>>,
    /// Mode flags
    pub modes: SnapshotModes,
    /// Scroll region
    pub scroll_region: Option<(usize, usize)>,
    /// Window title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDimensions {
    pub cols: usize,
    pub rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotCursor {
    pub col: usize,
    pub row: usize,
    pub visible: bool,
    pub style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotLine {
    /// Text content of the line
    pub text: String,
    /// Whether line was soft-wrapped
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub wrapped: bool,
    /// Attribute spans (for detailed comparison)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attrs: Vec<SnapshotAttrSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotAttrSpan {
    pub start: usize,
    pub end: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub bold: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub underline: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub inverse: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotModes {
    pub origin_mode: bool,
    pub auto_wrap: bool,
    pub cursor_visible: bool,
    pub alternate_screen: bool,
    pub bracketed_paste: bool,
    pub insert_mode: bool,
}

impl Snapshot {
    /// Create a snapshot from terminal components
    pub fn from_terminal(
        grid: &Grid,
        cursor: &Cursor,
        modes: &Modes,
        scrollback: Option<&Scrollback>,
        scroll_region: Option<(usize, usize)>,
        title: Option<&str>,
        include_scrollback: bool,
    ) -> Self {
        let dims = grid.dimensions();

        let screen: Vec<SnapshotLine> = grid
            .iter()
            .map(|line| SnapshotLine {
                text: line.text(),
                wrapped: line.wrapped,
                attrs: extract_attr_spans(line),
            })
            .collect();

        let scrollback_lines = if include_scrollback {
            scrollback.map(|sb| {
                sb.iter()
                    .map(|line| SnapshotLine {
                        text: line.text(),
                        wrapped: line.wrapped,
                        attrs: extract_attr_spans(line),
                    })
                    .collect()
            })
        } else {
            None
        };

        Self {
            dimensions: SnapshotDimensions {
                cols: dims.cols,
                rows: dims.rows,
            },
            cursor: SnapshotCursor {
                col: cursor.col,
                row: cursor.row,
                visible: cursor.visible,
                style: match cursor.style {
                    CursorStyle::Block => "block".to_string(),
                    CursorStyle::Underline => "underline".to_string(),
                    CursorStyle::Bar => "bar".to_string(),
                },
            },
            screen,
            scrollback: scrollback_lines,
            modes: SnapshotModes {
                origin_mode: modes.origin_mode,
                auto_wrap: modes.auto_wrap,
                cursor_visible: modes.cursor_visible,
                alternate_screen: modes.alternate_screen,
                bracketed_paste: modes.bracketed_paste,
                insert_mode: modes.insert_mode,
            },
            scroll_region,
            title: title.map(|s| s.to_string()),
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

    /// Get a simple text representation of the screen
    pub fn screen_text(&self) -> String {
        self.screen
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Extract attribute spans from a line
fn extract_attr_spans(line: &crate::line::Line) -> Vec<SnapshotAttrSpan> {
    use crate::color::Color;

    let mut spans = Vec::new();
    let mut current_span: Option<SnapshotAttrSpan> = None;

    for (i, cell) in line.iter().enumerate() {
        if cell.is_continuation() {
            continue;
        }

        let attrs = &cell.attrs;
        let has_attrs = attrs.bold
            || attrs.italic
            || attrs.underline
            || attrs.inverse
            || attrs.fg != Color::Default
            || attrs.bg != Color::Default;

        if !has_attrs {
            // Close current span if any
            if let Some(mut span) = current_span.take() {
                span.end = i;
                spans.push(span);
            }
            continue;
        }

        let fg_str = match attrs.fg {
            Color::Default => None,
            Color::Indexed(idx) => Some(format!("idx:{}", idx)),
            Color::Rgb { r, g, b } => Some(format!("#{:02x}{:02x}{:02x}", r, g, b)),
        };

        let bg_str = match attrs.bg {
            Color::Default => None,
            Color::Indexed(idx) => Some(format!("idx:{}", idx)),
            Color::Rgb { r, g, b } => Some(format!("#{:02x}{:02x}{:02x}", r, g, b)),
        };

        // Check if we can extend current span
        if let Some(ref span) = current_span {
            if span.fg == fg_str
                && span.bg == bg_str
                && span.bold == attrs.bold
                && span.italic == attrs.italic
                && span.underline == attrs.underline
                && span.inverse == attrs.inverse
            {
                // Same attributes, continue span
                continue;
            } else {
                // Different attributes, close current span
                let mut span = current_span.take().unwrap();
                span.end = i;
                spans.push(span);
            }
        }

        // Start new span
        current_span = Some(SnapshotAttrSpan {
            start: i,
            end: i, // Will be updated
            fg: fg_str,
            bg: bg_str,
            bold: attrs.bold,
            italic: attrs.italic,
            underline: attrs.underline,
            inverse: attrs.inverse,
        });
    }

    // Close final span
    if let Some(mut span) = current_span {
        span.end = line.cols();
        spans.push(span);
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::CellAttributes;

    #[test]
    fn test_snapshot_creation() {
        let grid = Grid::new(Dimensions::new(80, 24));
        let cursor = Cursor::new();
        let modes = Modes::new();

        let snapshot =
            Snapshot::from_terminal(&grid, &cursor, &modes, None, None, Some("Test"), false);

        assert_eq!(snapshot.dimensions.cols, 80);
        assert_eq!(snapshot.dimensions.rows, 24);
        assert_eq!(snapshot.cursor.col, 0);
        assert_eq!(snapshot.cursor.row, 0);
        assert!(snapshot.cursor.visible);
        assert_eq!(snapshot.title, Some("Test".to_string()));
    }

    #[test]
    fn test_snapshot_json_roundtrip() {
        let grid = Grid::new(Dimensions::new(80, 24));
        let cursor = Cursor::new();
        let modes = Modes::new();

        let snapshot = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);

        let json = snapshot.to_json().unwrap();
        let parsed = Snapshot::from_json(&json).unwrap();

        assert_eq!(parsed.dimensions.cols, snapshot.dimensions.cols);
        assert_eq!(parsed.dimensions.rows, snapshot.dimensions.rows);
    }

    #[test]
    fn test_snapshot_screen_text() {
        let mut grid = Grid::new(Dimensions::new(10, 3));
        grid.line_mut(0).cell_mut(0).set_char('H');
        grid.line_mut(0).cell_mut(1).set_char('i');
        grid.line_mut(1).cell_mut(0).set_char('!');

        let cursor = Cursor::new();
        let modes = Modes::new();

        let snapshot = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);

        let text = snapshot.screen_text();
        assert!(text.contains("Hi"));
        assert!(text.contains("!"));
    }
}
