//! Terminal Core Module
//!
//! Platform-independent terminal state management. This module contains:
//! - Screen model (primary and alternate screens)
//! - Cell representation with attributes
//! - Cursor state and positioning
//! - Scrollback buffer
//! - Deterministic snapshot generation
//!
//! The core is designed to be completely deterministic: given the same
//! sequence of terminal actions, it will always produce the same state.

mod cell;
mod cursor;
mod screen;
mod scrollback;
mod snapshot;

pub use cell::{Cell, Color, Style};
pub use cursor::Cursor;
pub use screen::{Modes, MouseEncoding, MouseMode, Screen};
pub use scrollback::Scrollback;
pub use snapshot::Snapshot;
