//! Renderer
//!
//! Renders the terminal screen to a window using softbuffer.

use std::sync::Arc;
use winit::window::Window;

use super::Selection;
use crate::core::Screen;

/// Terminal renderer using softbuffer
pub struct Renderer {
    width: u32,
    height: u32,
    cell_width: usize,
    cell_height: usize,
}

impl Renderer {
    /// Create a new renderer for the given window
    pub fn new(_window: Arc<Window>, font_size: f32) -> Result<Self, Box<dyn std::error::Error>> {
        // Calculate cell dimensions based on font size
        let cell_width = (font_size * 0.6) as usize;
        let cell_height = (font_size * 1.2) as usize;

        Ok(Self {
            width: 800,
            height: 600,
            cell_width: cell_width.max(1),
            cell_height: cell_height.max(1),
        })
    }

    /// Resize the renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    /// Get the grid size in columns and rows
    pub fn grid_size(&self) -> (usize, usize) {
        let cols = (self.width as usize) / self.cell_width;
        let rows = (self.height as usize) / self.cell_height;
        (cols.max(1), rows.max(1))
    }

    /// Convert pixel coordinates to cell coordinates
    pub fn pixel_to_cell(&self, x: f64, y: f64) -> (usize, usize) {
        let col = (x as usize) / self.cell_width;
        let row = (y as usize) / self.cell_height;
        (col, row)
    }

    /// Render the screen
    pub fn render(&mut self, _screen: &Screen, _selection: &Selection, _scroll_offset: usize) {
        // Rendering implementation would go here
        // For now this is a stub
    }
}
