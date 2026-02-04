//! Frontend module for GUI rendering
//!
//! This module provides the graphical user interface for the terminal emulator.
//! It uses winit for window management and wgpu for GPU-accelerated rendering.

// Input handling is always available (no GUI dependency)
pub mod input;

// GUI renderer (requires gui feature)
#[cfg(feature = "gui")]
pub mod renderer;

pub use input::*;

#[cfg(feature = "gui")]
pub use renderer::*;
