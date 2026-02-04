//! Mochi Terminal Core
//!
//! This crate provides the core terminal emulation logic including:
//! - Screen model (grid of cells)
//! - Cell representation (character, attributes, colors)
//! - Cursor state and movement
//! - Scrollback buffer
//! - Terminal modes and flags
//!
//! This crate is platform-independent and contains no GUI or PTY code.
//! It is designed to be deterministic: given the same sequence of operations,
//! it will always produce the same screen state.

pub mod cell;
pub mod color;
pub mod cursor;
pub mod line;
pub mod screen;
pub mod scrollback;
pub mod selection;
pub mod snapshot;

pub use cell::{Attributes, Cell};
pub use color::Color;
pub use cursor::{Cursor, CursorStyle};
pub use line::Line;
pub use screen::Screen;
pub use scrollback::Scrollback;
pub use selection::Selection;
pub use snapshot::Snapshot;

pub const DEFAULT_COLS: usize = 80;
pub const DEFAULT_ROWS: usize = 24;
pub const DEFAULT_SCROLLBACK_LINES: usize = 10000;
