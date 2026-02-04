//! Terminal renderer
//!
//! This module handles rendering the terminal to a window using wgpu.
//! It includes:
//! - Font rasterization and glyph caching
//! - Cell rendering with colors and attributes
//! - Cursor rendering
//! - Selection highlighting

use std::collections::HashMap;

use fontdue::{Font, FontSettings};
use mochi_core::cell::{Cell, CellFlags};
use mochi_core::color::Color;
use mochi_core::cursor::CursorStyle;
use mochi_core::Term;

use crate::config::{ColorScheme, Config, Rgb};

/// Glyph cache entry
#[derive(Debug)]
pub struct GlyphInfo {
    /// Rasterized bitmap (grayscale)
    pub bitmap: Vec<u8>,
    /// Bitmap width
    pub width: usize,
    /// Bitmap height
    pub height: usize,
    /// Horizontal offset from cursor
    pub offset_x: i32,
    /// Vertical offset from baseline
    pub offset_y: i32,
    /// Advance width
    pub advance: f32,
}

/// Font renderer with glyph caching
pub struct FontRenderer {
    /// The loaded font
    font: Font,
    /// Font size in pixels
    font_size: f32,
    /// Glyph cache
    cache: HashMap<char, GlyphInfo>,
    /// Cell width in pixels
    pub cell_width: f32,
    /// Cell height in pixels
    pub cell_height: f32,
    /// Baseline offset from top of cell
    pub baseline: f32,
}

impl FontRenderer {
    /// Create a new font renderer
    pub fn new(font_data: &[u8], font_size: f32) -> Result<Self, String> {
        let font = Font::from_bytes(font_data, FontSettings::default())
            .map_err(|e| format!("Failed to load font: {}", e))?;

        // Calculate cell dimensions based on font metrics
        let metrics = font.metrics('M', font_size);
        let cell_width = metrics.advance_width;
        let cell_height = font_size * 1.2; // Line height
        let baseline = font_size;

        Ok(FontRenderer {
            font,
            font_size,
            cache: HashMap::new(),
            cell_width,
            cell_height,
            baseline,
        })
    }

    /// Create with default monospace font
    pub fn with_default_font(font_size: f32) -> Result<Self, String> {
        // Use a built-in font or system font
        // For now, we'll use a simple approach
        let font_data = include_bytes!("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf");
        Self::new(font_data, font_size)
    }

    /// Get or rasterize a glyph
    pub fn get_glyph(&mut self, c: char) -> &GlyphInfo {
        if !self.cache.contains_key(&c) {
            let (metrics, bitmap) = self.font.rasterize(c, self.font_size);
            let glyph = GlyphInfo {
                bitmap,
                width: metrics.width,
                height: metrics.height,
                offset_x: metrics.xmin,
                offset_y: metrics.ymin,
                advance: metrics.advance_width,
            };
            self.cache.insert(c, glyph);
        }
        self.cache.get(&c).unwrap()
    }

    /// Clear the glyph cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

/// Software renderer for the terminal
pub struct SoftwareRenderer {
    /// Font renderer
    pub font: FontRenderer,
    /// Color scheme
    pub colors: ColorScheme,
    /// Framebuffer (RGBA)
    pub framebuffer: Vec<u8>,
    /// Framebuffer width
    pub width: u32,
    /// Framebuffer height
    pub height: u32,
}

impl SoftwareRenderer {
    /// Create a new software renderer
    pub fn new(config: &Config) -> Result<Self, String> {
        let font = FontRenderer::with_default_font(config.font_size)?;

        let width = (config.cols as f32 * font.cell_width) as u32;
        let height = (config.rows as f32 * font.cell_height) as u32;
        let framebuffer = vec![0u8; (width * height * 4) as usize];

        Ok(SoftwareRenderer {
            font,
            colors: config.colors.clone(),
            framebuffer,
            width,
            height,
        })
    }

    /// Resize the framebuffer
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.framebuffer = vec![0u8; (width * height * 4) as usize];
    }

    /// Calculate terminal dimensions from pixel size
    pub fn calc_dimensions(&self, pixel_width: u32, pixel_height: u32) -> (usize, usize) {
        let cols = (pixel_width as f32 / self.font.cell_width) as usize;
        let rows = (pixel_height as f32 / self.font.cell_height) as usize;
        (rows.max(1), cols.max(1))
    }

    /// Render the terminal to the framebuffer
    pub fn render(&mut self, term: &Term) {
        // Clear with background color
        self.clear(self.colors.background);

        let screen = term.screen();
        let rows = screen.rows();
        let cols = screen.cols();

        // Render cells
        for row in 0..rows {
            for col in 0..cols {
                let cell = screen.grid.cell(row, col);
                self.render_cell(row, col, cell, term);
            }
        }

        // Render cursor
        if screen.cursor.visible {
            self.render_cursor(
                screen.cursor.row,
                screen.cursor.col,
                screen.cursor.style,
            );
        }

        // Render selection
        if term.selection.active {
            self.render_selection(term);
        }
    }

    /// Clear the framebuffer with a color
    fn clear(&mut self, color: Rgb) {
        for pixel in self.framebuffer.chunks_exact_mut(4) {
            pixel[0] = color.r;
            pixel[1] = color.g;
            pixel[2] = color.b;
            pixel[3] = 255;
        }
    }

    /// Render a single cell
    fn render_cell(&mut self, row: usize, col: usize, cell: &Cell, term: &Term) {
        let x = (col as f32 * self.font.cell_width) as i32;
        let y = (row as f32 * self.font.cell_height) as i32;

        // Get colors
        let (fg, bg) = self.resolve_colors(cell, term);

        // Draw background
        self.fill_rect(
            x,
            y,
            self.font.cell_width as i32,
            self.font.cell_height as i32,
            bg,
        );

        // Skip if cell is empty or a wide char spacer
        if cell.c == " " || cell.c.is_empty() || cell.flags.contains(CellFlags::WIDE_CHAR_SPACER) {
            return;
        }

        // Draw character
        for c in cell.c.chars() {
            // Get glyph info and copy what we need to avoid borrow issues
            let glyph = self.font.get_glyph(c);
            let glyph_bitmap = glyph.bitmap.clone();
            let glyph_width = glyph.width;
            let glyph_height = glyph.height;
            let glyph_offset_x = glyph.offset_x;
            let glyph_offset_y = glyph.offset_y;
            let baseline = self.font.baseline;

            let gx = x + glyph_offset_x;
            let gy = y + (baseline as i32) - glyph_offset_y - glyph_height as i32;

            self.draw_glyph_bitmap(gx, gy, &glyph_bitmap, glyph_width, glyph_height, fg);
        }

        // Draw underline
        if cell.flags.contains(CellFlags::UNDERLINE) {
            let uy = y + self.font.cell_height as i32 - 2;
            self.fill_rect(x, uy, self.font.cell_width as i32, 1, fg);
        }

        // Draw strikethrough
        if cell.flags.contains(CellFlags::STRIKETHROUGH) {
            let sy = y + (self.font.cell_height as i32) / 2;
            self.fill_rect(x, sy, self.font.cell_width as i32, 1, fg);
        }
    }

    /// Resolve cell colors considering attributes
    fn resolve_colors(&self, cell: &Cell, _term: &Term) -> (Rgb, Rgb) {
        let mut fg = self.color_to_rgb(&cell.fg, true);
        let mut bg = self.color_to_rgb(&cell.bg, false);

        // Handle inverse
        if cell.flags.contains(CellFlags::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
        }

        // Handle hidden
        if cell.flags.contains(CellFlags::HIDDEN) {
            fg = bg;
        }

        // Handle faint
        if cell.flags.contains(CellFlags::FAINT) {
            fg = Rgb::new(fg.r / 2, fg.g / 2, fg.b / 2);
        }

        (fg, bg)
    }

    /// Convert a terminal color to RGB
    fn color_to_rgb(&self, color: &Color, is_fg: bool) -> Rgb {
        match color {
            Color::Default => {
                if is_fg {
                    self.colors.foreground
                } else {
                    self.colors.background
                }
            }
            Color::Named(named) => {
                let index = *named as usize;
                if index < 16 {
                    self.colors.palette[index]
                } else {
                    self.colors.foreground
                }
            }
            Color::Indexed(index) => {
                let index = *index as usize;
                if index < 16 {
                    self.colors.palette[index]
                } else if index < 232 {
                    // 6x6x6 color cube
                    let index = index - 16;
                    let r = (index / 36) % 6;
                    let g = (index / 6) % 6;
                    let b = index % 6;
                    let cube = [0u8, 95, 135, 175, 215, 255];
                    Rgb::new(cube[r], cube[g], cube[b])
                } else {
                    // Grayscale
                    let gray = ((index - 232) * 10 + 8) as u8;
                    Rgb::new(gray, gray, gray)
                }
            }
            Color::Rgb(rgb) => Rgb::new(rgb.r, rgb.g, rgb.b),
        }
    }

    /// Draw a glyph bitmap (takes raw bitmap data to avoid borrow issues)
    fn draw_glyph_bitmap(&mut self, x: i32, y: i32, bitmap: &[u8], width: usize, height: usize, color: Rgb) {
        for gy in 0..height {
            for gx in 0..width {
                let px = x + gx as i32;
                let py = y + gy as i32;

                if px < 0 || py < 0 || px >= self.width as i32 || py >= self.height as i32 {
                    continue;
                }

                let alpha = bitmap[gy * width + gx];
                if alpha == 0 {
                    continue;
                }

                let idx = ((py as u32 * self.width + px as u32) * 4) as usize;
                if idx + 3 < self.framebuffer.len() {
                    // Alpha blend
                    let a = alpha as f32 / 255.0;
                    let inv_a = 1.0 - a;
                    self.framebuffer[idx] =
                        (color.r as f32 * a + self.framebuffer[idx] as f32 * inv_a) as u8;
                    self.framebuffer[idx + 1] =
                        (color.g as f32 * a + self.framebuffer[idx + 1] as f32 * inv_a) as u8;
                    self.framebuffer[idx + 2] =
                        (color.b as f32 * a + self.framebuffer[idx + 2] as f32 * inv_a) as u8;
                }
            }
        }
    }

    /// Fill a rectangle
    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgb) {
        for py in y.max(0)..(y + h).min(self.height as i32) {
            for px in x.max(0)..(x + w).min(self.width as i32) {
                let idx = ((py as u32 * self.width + px as u32) * 4) as usize;
                if idx + 3 < self.framebuffer.len() {
                    self.framebuffer[idx] = color.r;
                    self.framebuffer[idx + 1] = color.g;
                    self.framebuffer[idx + 2] = color.b;
                    self.framebuffer[idx + 3] = 255;
                }
            }
        }
    }

    /// Render the cursor
    fn render_cursor(&mut self, row: usize, col: usize, style: CursorStyle) {
        let x = (col as f32 * self.font.cell_width) as i32;
        let y = (row as f32 * self.font.cell_height) as i32;
        let w = self.font.cell_width as i32;
        let h = self.font.cell_height as i32;

        match style {
            CursorStyle::Block => {
                // Invert the cell
                for py in y.max(0)..(y + h).min(self.height as i32) {
                    for px in x.max(0)..(x + w).min(self.width as i32) {
                        let idx = ((py as u32 * self.width + px as u32) * 4) as usize;
                        if idx + 2 < self.framebuffer.len() {
                            self.framebuffer[idx] = 255 - self.framebuffer[idx];
                            self.framebuffer[idx + 1] = 255 - self.framebuffer[idx + 1];
                            self.framebuffer[idx + 2] = 255 - self.framebuffer[idx + 2];
                        }
                    }
                }
            }
            CursorStyle::Underline => {
                self.fill_rect(x, y + h - 2, w, 2, self.colors.cursor);
            }
            CursorStyle::Bar => {
                self.fill_rect(x, y, 2, h, self.colors.cursor);
            }
        }
    }

    /// Render selection highlighting
    fn render_selection(&mut self, term: &Term) {
        let screen = term.screen();
        let cols = screen.cols();

        for row in 0..screen.rows() {
            if let Some((start_col, end_col)) =
                term.selection.columns_for_row(row, cols)
            {
                let x = (start_col as f32 * self.font.cell_width) as i32;
                let y = (row as f32 * self.font.cell_height) as i32;
                let w = ((end_col - start_col + 1) as f32 * self.font.cell_width) as i32;
                let h = self.font.cell_height as i32;

                // Draw selection overlay (semi-transparent)
                for py in y.max(0)..(y + h).min(self.height as i32) {
                    for px in x.max(0)..(x + w).min(self.width as i32) {
                        let idx = ((py as u32 * self.width + px as u32) * 4) as usize;
                        if idx + 2 < self.framebuffer.len() {
                            // Blend with selection color
                            let a = 0.3f32;
                            let inv_a = 1.0 - a;
                            self.framebuffer[idx] = (self.colors.selection.r as f32 * a
                                + self.framebuffer[idx] as f32 * inv_a)
                                as u8;
                            self.framebuffer[idx + 1] = (self.colors.selection.g as f32 * a
                                + self.framebuffer[idx + 1] as f32 * inv_a)
                                as u8;
                            self.framebuffer[idx + 2] = (self.colors.selection.b as f32 * a
                                + self.framebuffer[idx + 2] as f32 * inv_a)
                                as u8;
                        }
                    }
                }
            }
        }
    }

    /// Get the framebuffer as RGBA bytes
    pub fn framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_to_rgb() {
        let config = Config::default();
        let renderer = SoftwareRenderer::new(&config);

        // Skip test if font loading fails (e.g., in CI without fonts)
        if renderer.is_err() {
            return;
        }
        let renderer = renderer.unwrap();

        let fg = renderer.color_to_rgb(&Color::Default, true);
        assert_eq!(fg, config.colors.foreground);

        let bg = renderer.color_to_rgb(&Color::Default, false);
        assert_eq!(bg, config.colors.background);
    }

    #[test]
    fn test_calc_dimensions() {
        let config = Config::default();
        let renderer = SoftwareRenderer::new(&config);

        if renderer.is_err() {
            return;
        }
        let renderer = renderer.unwrap();

        let (rows, cols) = renderer.calc_dimensions(800, 600);
        assert!(rows > 0);
        assert!(cols > 0);
    }
}
