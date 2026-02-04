//! Terminal Core - Platform-independent terminal screen model
//!
//! This crate provides the core data structures and logic for terminal emulation:
//! - Screen grid with cells containing characters and attributes
//! - Cursor state management
//! - Scrollback buffer
//! - Mode flags and terminal state
//!
//! This crate is designed to be deterministic: given the same sequence of operations,
//! it will always produce the same screen state.

mod cell;
mod charset;
mod color;
mod cursor;
mod grid;
mod line;
mod modes;
mod screen;
mod scrollback;
pub mod selection;
mod snapshot;

pub use cell::{Cell, CellAttributes};
pub use charset::{parse_charset_designation, Charset, CharsetState};
pub use color::Color;
pub use cursor::{Cursor, CursorStyle};
pub use grid::Grid;
pub use line::Line;
pub use modes::Modes;
pub use screen::Screen;
pub use scrollback::Scrollback;
pub use selection::Selection;
pub use snapshot::Snapshot;

/// Terminal dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    pub cols: usize,
    pub rows: usize,
}

impl Dimensions {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self { cols, rows }
    }
}

impl Default for Dimensions {
    fn default() -> Self {
        Self { cols: 80, rows: 24 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimensions_default() {
        let dims = Dimensions::default();
        assert_eq!(dims.cols, 80);
        assert_eq!(dims.rows, 24);
    }
}
