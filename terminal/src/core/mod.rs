//! Terminal core module - platform-independent screen model
//!
//! This module contains all terminal state logic with no platform dependencies.
//! Given the same byte sequence, it produces identical screen state.

mod cell;
mod cursor;
mod line;
mod modes;
mod screen;
mod scrollback;
pub mod snapshot;

pub use cell::{Cell, CellAttributes, Color};
pub use cursor::{Cursor, CursorStyle};
pub use line::Line;
pub use modes::{Modes, MouseEncoding, MouseMode};
pub use screen::Screen;
pub use scrollback::Scrollback;
pub use snapshot::{CompactSnapshot, Snapshot};
