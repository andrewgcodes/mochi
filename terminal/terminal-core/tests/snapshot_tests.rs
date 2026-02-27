//! Comprehensive tests for terminal snapshots

use terminal_core::{
    CellAttributes, Color, Cursor, CursorStyle, Dimensions, Grid, Line, Modes, Scrollback, Snapshot,
};

// ============================================================
// Snapshot Creation Tests
// ============================================================

#[test]
fn test_snapshot_creation_basic() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert_eq!(snap.dimensions.cols, 80);
    assert_eq!(snap.dimensions.rows, 24);
}

#[test]
fn test_snapshot_cursor_position() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let mut cursor = Cursor::new();
    cursor.col = 10;
    cursor.row = 5;
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert_eq!(snap.cursor.col, 10);
    assert_eq!(snap.cursor.row, 5);
}

#[test]
fn test_snapshot_cursor_visible() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert!(snap.cursor.visible);
}

#[test]
fn test_snapshot_cursor_hidden() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let mut cursor = Cursor::new();
    cursor.visible = false;
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert!(!snap.cursor.visible);
}

#[test]
fn test_snapshot_cursor_style_block() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert_eq!(snap.cursor.style, "block");
}

#[test]
fn test_snapshot_cursor_style_underline() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let mut cursor = Cursor::new();
    cursor.style = CursorStyle::Underline;
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert_eq!(snap.cursor.style, "underline");
}

#[test]
fn test_snapshot_cursor_style_bar() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let mut cursor = Cursor::new();
    cursor.style = CursorStyle::Bar;
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert_eq!(snap.cursor.style, "bar");
}

// ============================================================
// Snapshot Title Tests
// ============================================================

#[test]
fn test_snapshot_with_title() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(
        &grid,
        &cursor,
        &modes,
        None,
        None,
        Some("Test Title"),
        false,
    );
    assert_eq!(snap.title, Some("Test Title".to_string()));
}

#[test]
fn test_snapshot_without_title() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert_eq!(snap.title, None);
}

// ============================================================
// Snapshot Modes Tests
// ============================================================

#[test]
fn test_snapshot_modes_defaults() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert!(!snap.modes.origin_mode);
    assert!(snap.modes.auto_wrap);
    assert!(snap.modes.cursor_visible);
    assert!(!snap.modes.alternate_screen);
    assert!(!snap.modes.bracketed_paste);
    assert!(!snap.modes.insert_mode);
}

#[test]
fn test_snapshot_modes_modified() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let cursor = Cursor::new();
    let mut modes = Modes::new();
    modes.origin_mode = true;
    modes.alternate_screen = true;
    modes.bracketed_paste = true;
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert!(snap.modes.origin_mode);
    assert!(snap.modes.alternate_screen);
    assert!(snap.modes.bracketed_paste);
}

// ============================================================
// Snapshot Scroll Region Tests
// ============================================================

#[test]
fn test_snapshot_no_scroll_region() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert_eq!(snap.scroll_region, None);
}

#[test]
fn test_snapshot_with_scroll_region() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, Some((5, 20)), None, false);
    assert_eq!(snap.scroll_region, Some((5, 20)));
}

// ============================================================
// Snapshot Screen Content Tests
// ============================================================

#[test]
fn test_snapshot_screen_text_empty() {
    let grid = Grid::new(Dimensions::new(10, 3));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    // All lines should be empty
    for line in &snap.screen {
        assert_eq!(line.text, "");
    }
}

#[test]
fn test_snapshot_screen_text_with_content() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    grid.line_mut(0).cell_mut(0).set_char('H');
    grid.line_mut(0).cell_mut(1).set_char('i');
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert_eq!(snap.screen[0].text, "Hi");
}

#[test]
fn test_snapshot_screen_line_count() {
    let grid = Grid::new(Dimensions::new(10, 5));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert_eq!(snap.screen.len(), 5);
}

#[test]
fn test_snapshot_screen_wrapped_line() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    grid.line_mut(0).wrapped = true;
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert!(snap.screen[0].wrapped);
}

// ============================================================
// Snapshot Scrollback Tests
// ============================================================

#[test]
fn test_snapshot_without_scrollback() {
    let grid = Grid::new(Dimensions::new(10, 3));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert!(snap.scrollback.is_none());
}

#[test]
fn test_snapshot_with_scrollback_requested() {
    let grid = Grid::new(Dimensions::new(10, 3));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let mut sb = Scrollback::new(100);
    let mut line = Line::new(10);
    line.cell_mut(0).set_char('X');
    sb.push(line);
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, Some(&sb), None, None, true);
    assert!(snap.scrollback.is_some());
    let sb_lines = snap.scrollback.unwrap();
    assert_eq!(sb_lines.len(), 1);
    assert!(sb_lines[0].text.contains('X'));
}

#[test]
fn test_snapshot_scrollback_not_included_when_false() {
    let grid = Grid::new(Dimensions::new(10, 3));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let mut sb = Scrollback::new(100);
    let line = Line::new(10);
    sb.push(line);
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, Some(&sb), None, None, false);
    assert!(snap.scrollback.is_none());
}

// ============================================================
// Snapshot JSON Tests
// ============================================================

#[test]
fn test_snapshot_to_json() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    let json = snap.to_json().unwrap();
    assert!(json.contains("dimensions"));
    assert!(json.contains("cursor"));
    assert!(json.contains("screen"));
}

#[test]
fn test_snapshot_from_json() {
    let grid = Grid::new(Dimensions::new(80, 24));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    let json = snap.to_json().unwrap();
    let parsed = Snapshot::from_json(&json).unwrap();
    assert_eq!(parsed.dimensions.cols, 80);
    assert_eq!(parsed.dimensions.rows, 24);
}

#[test]
fn test_snapshot_json_roundtrip_preserves_content() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    grid.line_mut(0).cell_mut(0).set_char('A');
    grid.line_mut(1).cell_mut(0).set_char('B');
    let mut cursor = Cursor::new();
    cursor.col = 5;
    cursor.row = 2;
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, Some("Test"), false);
    let json = snap.to_json().unwrap();
    let parsed = Snapshot::from_json(&json).unwrap();
    assert_eq!(parsed.cursor.col, 5);
    assert_eq!(parsed.cursor.row, 2);
    assert_eq!(parsed.title, Some("Test".to_string()));
}

#[test]
fn test_snapshot_json_roundtrip_with_title() {
    let grid = Grid::new(Dimensions::new(10, 3));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, Some("My Title"), false);
    let json = snap.to_json().unwrap();
    let parsed = Snapshot::from_json(&json).unwrap();
    assert_eq!(parsed.title, Some("My Title".to_string()));
}

#[test]
fn test_snapshot_json_roundtrip_without_title() {
    let grid = Grid::new(Dimensions::new(10, 3));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    let json = snap.to_json().unwrap();
    let parsed = Snapshot::from_json(&json).unwrap();
    assert_eq!(parsed.title, None);
}

#[test]
fn test_snapshot_from_json_invalid() {
    let result = Snapshot::from_json("not valid json");
    assert!(result.is_err());
}

// ============================================================
// Snapshot screen_text Tests
// ============================================================

#[test]
fn test_snapshot_screen_text_method() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    grid.line_mut(0).cell_mut(0).set_char('H');
    grid.line_mut(0).cell_mut(1).set_char('i');
    grid.line_mut(1).cell_mut(0).set_char('!');
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    let text = snap.screen_text();
    assert!(text.contains("Hi"));
    assert!(text.contains("!"));
}

#[test]
fn test_snapshot_screen_text_all_empty() {
    let grid = Grid::new(Dimensions::new(10, 3));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    let text = snap.screen_text();
    // Should be lines separated by newlines, all empty
    let lines: Vec<&str> = text.split('\n').collect();
    assert_eq!(lines.len(), 3);
}

// ============================================================
// Snapshot Attr Spans Tests
// ============================================================

#[test]
fn test_snapshot_no_attrs_no_spans() {
    let grid = Grid::new(Dimensions::new(10, 3));
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    for line in &snap.screen {
        assert!(line.attrs.is_empty());
    }
}

#[test]
fn test_snapshot_with_bold_attr_span() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    let cell = grid.line_mut(0).cell_mut(0);
    cell.set_char('B');
    cell.attrs = attrs;
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert!(!snap.screen[0].attrs.is_empty());
    assert!(snap.screen[0].attrs[0].bold);
}

#[test]
fn test_snapshot_with_color_attr_span() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    let mut attrs = CellAttributes::new();
    attrs.fg = Color::Indexed(1);
    let cell = grid.line_mut(0).cell_mut(0);
    cell.set_char('R');
    cell.attrs = attrs;
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert!(!snap.screen[0].attrs.is_empty());
    assert!(snap.screen[0].attrs[0].fg.is_some());
}

#[test]
fn test_snapshot_with_rgb_color_attr() {
    let mut grid = Grid::new(Dimensions::new(10, 3));
    let mut attrs = CellAttributes::new();
    attrs.fg = Color::rgb(255, 0, 128);
    let cell = grid.line_mut(0).cell_mut(0);
    cell.set_char('P');
    cell.attrs = attrs;
    let cursor = Cursor::new();
    let modes = Modes::new();
    let snap = Snapshot::from_terminal(&grid, &cursor, &modes, None, None, None, false);
    assert!(!snap.screen[0].attrs.is_empty());
    let fg = snap.screen[0].attrs[0].fg.as_ref().unwrap();
    assert!(fg.starts_with('#'));
}
