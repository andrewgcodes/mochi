//! Terminal Window
//!
//! Manages the terminal window, event loop, and rendering pipeline.
//! Uses winit for window management and software rendering.

use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton as WinitMouseButton, VirtualKeyCode};

use crate::core::Color;
use crate::input::{self, Key, Modifiers, MouseButton};

use super::font::{FontError, FontRenderer};

/// Terminal window configuration
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Window title
    pub title: String,
    /// Initial width in pixels
    pub width: u32,
    /// Initial height in pixels
    pub height: u32,
    /// Font size in pixels
    pub font_size: f32,
    /// Background color (RGB)
    pub background: [f32; 3],
    /// Foreground color (RGB)
    pub foreground: [f32; 3],
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Mochi Terminal".to_string(),
            width: 800,
            height: 600,
            font_size: 16.0,
            background: [0.0, 0.0, 0.0],       // Black
            foreground: [0.9, 0.9, 0.9],       // Light gray
        }
    }
}

/// Color palette for the terminal (16 standard colors + defaults)
#[derive(Debug, Clone)]
pub struct ColorPalette {
    /// Standard 16 colors (0-15)
    pub colors: [[f32; 3]; 16],
    /// Default foreground
    pub foreground: [f32; 3],
    /// Default background
    pub background: [f32; 3],
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            colors: [
                // Standard colors (0-7)
                [0.0, 0.0, 0.0],       // 0: Black
                [0.8, 0.0, 0.0],       // 1: Red
                [0.0, 0.8, 0.0],       // 2: Green
                [0.8, 0.8, 0.0],       // 3: Yellow
                [0.0, 0.0, 0.8],       // 4: Blue
                [0.8, 0.0, 0.8],       // 5: Magenta
                [0.0, 0.8, 0.8],       // 6: Cyan
                [0.75, 0.75, 0.75],    // 7: White
                // Bright colors (8-15)
                [0.5, 0.5, 0.5],       // 8: Bright Black (Gray)
                [1.0, 0.0, 0.0],       // 9: Bright Red
                [0.0, 1.0, 0.0],       // 10: Bright Green
                [1.0, 1.0, 0.0],       // 11: Bright Yellow
                [0.0, 0.0, 1.0],       // 12: Bright Blue
                [1.0, 0.0, 1.0],       // 13: Bright Magenta
                [0.0, 1.0, 1.0],       // 14: Bright Cyan
                [1.0, 1.0, 1.0],       // 15: Bright White
            ],
            foreground: [0.9, 0.9, 0.9],
            background: [0.0, 0.0, 0.0],
        }
    }
}

impl ColorPalette {
    /// Convert a terminal Color to RGB floats
    pub fn resolve(&self, color: &Color, is_foreground: bool) -> [f32; 3] {
        match color {
            Color::Default => {
                if is_foreground {
                    self.foreground
                } else {
                    self.background
                }
            }
            Color::Indexed(i) => {
                if *i < 16 {
                    self.colors[*i as usize]
                } else if *i < 232 {
                    // 216 color cube (16-231)
                    let i = *i - 16;
                    let r = (i / 36) % 6;
                    let g = (i / 6) % 6;
                    let b = i % 6;
                    [
                        if r == 0 { 0.0 } else { (r as f32 * 40.0 + 55.0) / 255.0 },
                        if g == 0 { 0.0 } else { (g as f32 * 40.0 + 55.0) / 255.0 },
                        if b == 0 { 0.0 } else { (b as f32 * 40.0 + 55.0) / 255.0 },
                    ]
                } else {
                    // Grayscale (232-255)
                    let gray = (*i - 232) as f32 * 10.0 + 8.0;
                    let v = gray / 255.0;
                    [v, v, v]
                }
            }
            Color::Rgb(r, g, b) => [*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0],
        }
    }
}

/// The terminal window and renderer
pub struct TerminalWindow {
    /// Window configuration
    config: WindowConfig,
    /// Font renderer
    font: FontRenderer,
    /// Color palette
    palette: ColorPalette,
    /// Current grid dimensions
    cols: usize,
    rows: usize,
}

impl TerminalWindow {
    /// Create a new terminal window
    pub fn new(config: WindowConfig) -> Result<Self, FontError> {
        let font = FontRenderer::with_default_font(config.font_size)?;

        // Calculate initial grid size
        let (cols, rows) = font.calculate_grid_size(config.width, config.height);

        Ok(Self {
            config,
            font,
            palette: ColorPalette::default(),
            cols,
            rows,
        })
    }

    /// Get the current grid dimensions
    pub fn grid_size(&self) -> (usize, usize) {
        (self.cols, self.rows)
    }

    /// Update grid size based on new window dimensions
    pub fn update_size(&mut self, width: u32, height: u32) -> (usize, usize) {
        let (cols, rows) = self.font.calculate_grid_size(width, height);
        self.cols = cols;
        self.rows = rows;
        (cols, rows)
    }

    /// Get the font renderer
    pub fn font(&self) -> &FontRenderer {
        &self.font
    }

    /// Get mutable font renderer
    pub fn font_mut(&mut self) -> &mut FontRenderer {
        &mut self.font
    }

    /// Get the color palette
    pub fn palette(&self) -> &ColorPalette {
        &self.palette
    }

    /// Convert a winit virtual key code to terminal input bytes
    pub fn encode_key(
        &self,
        keycode: Option<VirtualKeyCode>,
        modifiers: Modifiers,
        application_cursor: bool,
        application_keypad: bool,
    ) -> Option<Vec<u8>> {
        let keycode = keycode?;

        // Check for special keys first
        if let Some(key) = self.map_special_key(keycode) {
            return Some(input::encode_key(
                key,
                modifiers,
                application_cursor,
                application_keypad,
            ));
        }

        None
    }

    /// Encode a character input
    pub fn encode_char_input(&self, c: char, modifiers: Modifiers) -> Vec<u8> {
        input::encode_char(c, modifiers)
    }

    /// Map winit key to our Key enum
    fn map_special_key(&self, key: VirtualKeyCode) -> Option<Key> {
        match key {
            VirtualKeyCode::Up => Some(Key::Up),
            VirtualKeyCode::Down => Some(Key::Down),
            VirtualKeyCode::Left => Some(Key::Left),
            VirtualKeyCode::Right => Some(Key::Right),
            VirtualKeyCode::Home => Some(Key::Home),
            VirtualKeyCode::End => Some(Key::End),
            VirtualKeyCode::PageUp => Some(Key::PageUp),
            VirtualKeyCode::PageDown => Some(Key::PageDown),
            VirtualKeyCode::Insert => Some(Key::Insert),
            VirtualKeyCode::Delete => Some(Key::Delete),
            VirtualKeyCode::Back => Some(Key::Backspace),
            VirtualKeyCode::Tab => Some(Key::Tab),
            VirtualKeyCode::Return => Some(Key::Enter),
            VirtualKeyCode::Escape => Some(Key::Escape),
            VirtualKeyCode::F1 => Some(Key::F1),
            VirtualKeyCode::F2 => Some(Key::F2),
            VirtualKeyCode::F3 => Some(Key::F3),
            VirtualKeyCode::F4 => Some(Key::F4),
            VirtualKeyCode::F5 => Some(Key::F5),
            VirtualKeyCode::F6 => Some(Key::F6),
            VirtualKeyCode::F7 => Some(Key::F7),
            VirtualKeyCode::F8 => Some(Key::F8),
            VirtualKeyCode::F9 => Some(Key::F9),
            VirtualKeyCode::F10 => Some(Key::F10),
            VirtualKeyCode::F11 => Some(Key::F11),
            VirtualKeyCode::F12 => Some(Key::F12),
            _ => None,
        }
    }

    /// Convert winit mouse button to our MouseButton enum
    pub fn map_mouse_button(&self, button: WinitMouseButton) -> Option<MouseButton> {
        match button {
            WinitMouseButton::Left => Some(MouseButton::Left),
            WinitMouseButton::Middle => Some(MouseButton::Middle),
            WinitMouseButton::Right => Some(MouseButton::Right),
            WinitMouseButton::Other(3) => Some(MouseButton::Button4),
            WinitMouseButton::Other(4) => Some(MouseButton::Button5),
            _ => None,
        }
    }

    /// Convert pixel coordinates to cell coordinates
    pub fn pixel_to_cell(&self, x: f64, y: f64) -> (usize, usize) {
        self.font.pixel_to_cell(x as f32, y as f32)
    }
}

/// Simple software renderer for the terminal
/// This is a basic implementation that renders to a pixel buffer
pub struct SoftwareRenderer {
    /// Pixel buffer (RGBA)
    buffer: Vec<u8>,
    /// Buffer width
    width: u32,
    /// Buffer height
    height: u32,
}

impl SoftwareRenderer {
    /// Create a new software renderer
    pub fn new(width: u32, height: u32) -> Self {
        let buffer = vec![0u8; (width * height * 4) as usize];
        Self {
            buffer,
            width,
            height,
        }
    }

    /// Resize the buffer
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.buffer.resize((width * height * 4) as usize, 0);
    }

    /// Clear the buffer with a color
    pub fn clear(&mut self, color: [f32; 3]) {
        let r = (color[0] * 255.0) as u8;
        let g = (color[1] * 255.0) as u8;
        let b = (color[2] * 255.0) as u8;

        for pixel in self.buffer.chunks_exact_mut(4) {
            pixel[0] = r;
            pixel[1] = g;
            pixel[2] = b;
            pixel[3] = 255;
        }
    }

    /// Fill a rectangle with a color
    pub fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: [f32; 3]) {
        let r = (color[0] * 255.0) as u8;
        let g = (color[1] * 255.0) as u8;
        let b = (color[2] * 255.0) as u8;

        for row in y..(y + h).min(self.height) {
            for col in x..(x + w).min(self.width) {
                let idx = ((row * self.width + col) * 4) as usize;
                if idx + 3 < self.buffer.len() {
                    self.buffer[idx] = r;
                    self.buffer[idx + 1] = g;
                    self.buffer[idx + 2] = b;
                    self.buffer[idx + 3] = 255;
                }
            }
        }
    }

    /// Draw a glyph bitmap at position with foreground color
    pub fn draw_glyph(
        &mut self,
        bitmap: &[u8],
        glyph_width: u32,
        glyph_height: u32,
        x: i32,
        y: i32,
        fg: [f32; 3],
        bg: [f32; 3],
    ) {
        let fg_r = (fg[0] * 255.0) as u8;
        let fg_g = (fg[1] * 255.0) as u8;
        let fg_b = (fg[2] * 255.0) as u8;
        let bg_r = (bg[0] * 255.0) as u8;
        let bg_g = (bg[1] * 255.0) as u8;
        let bg_b = (bg[2] * 255.0) as u8;

        for gy in 0..glyph_height {
            let py = y + gy as i32;
            if py < 0 || py >= self.height as i32 {
                continue;
            }

            for gx in 0..glyph_width {
                let px = x + gx as i32;
                if px < 0 || px >= self.width as i32 {
                    continue;
                }

                let glyph_idx = (gy * glyph_width + gx) as usize;
                let alpha = bitmap.get(glyph_idx).copied().unwrap_or(0);

                let idx = ((py as u32 * self.width + px as u32) * 4) as usize;
                if idx + 3 < self.buffer.len() {
                    // Alpha blend
                    let a = alpha as f32 / 255.0;
                    let inv_a = 1.0 - a;

                    self.buffer[idx] = (fg_r as f32 * a + bg_r as f32 * inv_a) as u8;
                    self.buffer[idx + 1] = (fg_g as f32 * a + bg_g as f32 * inv_a) as u8;
                    self.buffer[idx + 2] = (fg_b as f32 * a + bg_b as f32 * inv_a) as u8;
                    self.buffer[idx + 3] = 255;
                }
            }
        }
    }

    /// Get the pixel buffer
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Get buffer dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_palette_resolve_default() {
        let palette = ColorPalette::default();

        let fg = palette.resolve(&Color::Default, true);
        assert_eq!(fg, palette.foreground);

        let bg = palette.resolve(&Color::Default, false);
        assert_eq!(bg, palette.background);
    }

    #[test]
    fn test_color_palette_resolve_indexed() {
        let palette = ColorPalette::default();

        // Standard colors
        let red = palette.resolve(&Color::Indexed(1), true);
        assert_eq!(red, palette.colors[1]);

        // Bright colors
        let bright_red = palette.resolve(&Color::Indexed(9), true);
        assert_eq!(bright_red, palette.colors[9]);
    }

    #[test]
    fn test_color_palette_resolve_rgb() {
        let palette = ColorPalette::default();

        let color = palette.resolve(&Color::Rgb(255, 128, 0), true);
        assert!((color[0] - 1.0).abs() < 0.01);
        assert!((color[1] - 0.5).abs() < 0.01);
        assert!((color[2] - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_color_palette_resolve_256_grayscale() {
        let palette = ColorPalette::default();

        // Grayscale 232 should be dark gray
        let gray = palette.resolve(&Color::Indexed(232), true);
        assert!(gray[0] < 0.1);

        // Grayscale 255 should be light gray
        let light = palette.resolve(&Color::Indexed(255), true);
        assert!(light[0] > 0.9);
    }

    #[test]
    fn test_software_renderer_clear() {
        let mut renderer = SoftwareRenderer::new(10, 10);
        renderer.clear([1.0, 0.0, 0.0]); // Red

        // Check first pixel
        assert_eq!(renderer.buffer[0], 255); // R
        assert_eq!(renderer.buffer[1], 0);   // G
        assert_eq!(renderer.buffer[2], 0);   // B
        assert_eq!(renderer.buffer[3], 255); // A
    }

    #[test]
    fn test_software_renderer_fill_rect() {
        let mut renderer = SoftwareRenderer::new(10, 10);
        renderer.clear([0.0, 0.0, 0.0]);
        renderer.fill_rect(2, 2, 3, 3, [0.0, 1.0, 0.0]); // Green square

        // Check pixel inside rect
        let idx = ((3 * 10 + 3) * 4) as usize;
        assert_eq!(renderer.buffer[idx], 0);     // R
        assert_eq!(renderer.buffer[idx + 1], 255); // G
        assert_eq!(renderer.buffer[idx + 2], 0);   // B

        // Check pixel outside rect
        let idx = ((0 * 10 + 0) * 4) as usize;
        assert_eq!(renderer.buffer[idx], 0);     // R
        assert_eq!(renderer.buffer[idx + 1], 0); // G
        assert_eq!(renderer.buffer[idx + 2], 0); // B
    }
}
