//! GUI Renderer Module
//!
//! Handles window creation, font rendering, and drawing the terminal grid.
//! Uses winit for window management and wgpu for GPU-accelerated rendering.

mod font;
mod window;

pub use font::FontRenderer;
pub use window::{ColorPalette, SoftwareRenderer, TerminalWindow, WindowConfig};
