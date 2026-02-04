//! Mochi Terminal Core
//!
//! This crate provides the platform-independent terminal emulator core:
//! - Screen model with primary and alternate buffers
//! - Cell representation with character and attributes
//! - Cursor state management
//! - Scrollback buffer
//! - Deterministic state transitions for testing
//!
//! This crate has NO GUI dependencies and can be used headlessly for testing.

pub mod cell;
pub mod color;
pub mod cursor;
pub mod grid;
pub mod line;
pub mod screen;
pub mod scrollback;
pub mod selection;
pub mod snapshot;
pub mod term;

pub use cell::{Cell, CellFlags};
pub use color::Color;
pub use cursor::{Cursor, CursorStyle};
pub use grid::Grid;
pub use line::Line;
pub use screen::Screen;
pub use scrollback::Scrollback;
pub use selection::Selection;
pub use snapshot::Snapshot;
pub use term::{Term, TermMode};
