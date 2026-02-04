//! Renderer
//!
//! Renders the terminal screen to a window using softbuffer.

use std::sync::Arc;

use softbuffer::GraphicsContext;
use winit::window::Window;

use super::font::FontRenderer;
use super::Selection;
use crate::core::{Color, Screen, Style};

/// Default color palette (xterm 256 colors)
const PALETTE_16: [(u8, u8, u8); 16] = [
    (0, 0, 0),       // 0: Black
    (205, 0, 0),     // 1: Red
    (0, 205, 0),     // 2: Green
    (205, 205, 0),   // 3: Yellow
    (0, 0, 238),     // 4: Blue
    (205, 0, 205),   // 5: Magenta
    (0, 205, 205),   // 6: Cyan
    (229, 229, 229), // 7: White
    (127, 127, 127), // 8: Bright Black
    (255, 0, 0),     // 9: Bright Red
    (0, 255, 0),     // 10: Bright Green
    (255, 255, 0),   // 11: Bright Yellow
    (92, 92, 255),   // 12: Bright Blue
    (255, 0, 255),   // 13: Bright Magenta
    (0, 255, 255),   // 14: Bright Cyan
    (255, 255, 255), // 15: Bright White
];

/// Terminal renderer using softbuffer
pub struct Renderer {
    graphics: GraphicsContext,
    font: FontRenderer,
    width: u32,
    height: u32,
    cell_width: usize,
    cell_height: usize,
    default_fg: (u8, u8, u8),
    default_bg: (u8, u8, u8),
    cursor_color: (u8, u8, u8),
}

impl Renderer {
    /// Create a new renderer for the given window
    pub fn new(window: Arc<Window>, font_size: f32) -> Result<Self, Box<dyn std::error::Error>> {
        // Safety: window is valid for the lifetime of the renderer
        let graphics = unsafe { GraphicsContext::new(&*window, &*window) }
            .map_err(|e| format!("Failed to create graphics context: {}", e))?;

        let font = FontRenderer::new(font_size)?;
        let (cell_width, cell_height) = font.cell_size();

        let size = window.inner_size();

        Ok(Self {
            graphics,
            font,
            width: size.width,
            height: size.height,
            cell_width,
            cell_height,
            default_fg: (229, 229, 229),
            default_bg: (0, 0, 0),
            cursor_color: (255, 255, 255),
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

    /// Get cell dimensions
    pub fn cell_size(&self) -> (usize, usize) {
        (self.cell_width, self.cell_height)
    }

    /// Convert a Color to RGB
    fn color_to_rgb(&self, color: Color, is_fg: bool) -> (u8, u8, u8) {
        match color {
            Color::Default => {
                if is_fg {
                    self.default_fg
                } else {
                    self.default_bg
                }
            }
            Color::Indexed(idx) => {
                if idx < 16 {
                    PALETTE_16[idx as usize]
                } else if idx < 232 {
                    // 216 color cube (6x6x6)
                    let idx = idx - 16;
                    let r = (idx / 36) % 6;
                    let g = (idx / 6) % 6;
                    let b = idx % 6;
                    let to_val = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
                    (to_val(r), to_val(g), to_val(b))
                } else {
                    // 24 grayscale colors
                    let gray = 8 + (idx - 232) * 10;
                    (gray, gray, gray)
                }
            }
            Color::Rgb(r, g, b) => (r, g, b),
        }
    }

    /// Render the screen
    pub fn render(&mut self, screen: &Screen, selection: &Selection, scroll_offset: usize) {
        let width = self.width as usize;
        let height = self.height as usize;

        if width == 0 || height == 0 {
            return;
        }

        // Create buffer
        let mut buffer = vec![self.pack_color(self.default_bg); width * height];

        let grid = screen.grid();
        let (cols, rows) = self.grid_size();
        let cursor = &screen.cursor;

        // Render each cell
        for row in 0..rows.min(screen.rows()) {
            for col in 0..cols.min(screen.cols()) {
                if let Some(cell) = grid.cell(col, row) {
                    let is_selected = selection.contains(col, row + scroll_offset);
                    let is_cursor = cursor.row == row && cursor.col == col && cursor.visible;

                    // Determine colors
                    let (mut fg, mut bg) = if cell.style.inverse {
                        (
                            self.color_to_rgb(cell.bg, false),
                            self.color_to_rgb(cell.fg, true),
                        )
                    } else {
                        (
                            self.color_to_rgb(cell.fg, true),
                            self.color_to_rgb(cell.bg, false),
                        )
                    };

                    // Selection inverts colors
                    if is_selected {
                        std::mem::swap(&mut fg, &mut bg);
                    }

                    // Cursor handling
                    if is_cursor && cursor.visible {
                        bg = self.cursor_color;
                        fg = self.default_bg;
                    }

                    // Apply faint
                    if cell.style.faint {
                        fg = (fg.0 / 2, fg.1 / 2, fg.2 / 2);
                    }

                    // Draw cell background
                    self.fill_rect(
                        &mut buffer,
                        width,
                        col * self.cell_width,
                        row * self.cell_height,
                        self.cell_width,
                        self.cell_height,
                        bg,
                    );

                    // Draw character
                    let content = &cell.content;
                    if !content.is_empty() && !cell.style.hidden {
                        let c = content.chars().next().unwrap_or(' ');
                        if c != ' ' && !c.is_control() {
                            self.draw_char(
                                &mut buffer,
                                width,
                                height,
                                col,
                                row,
                                c,
                                fg,
                                &cell.style,
                            );
                        }
                    }

                    // Draw underline
                    if cell.style.underline {
                        let y = row * self.cell_height + self.cell_height - 2;
                        if y < height {
                            for x in
                                (col * self.cell_width)..((col + 1) * self.cell_width).min(width)
                            {
                                buffer[y * width + x] = self.pack_color(fg);
                            }
                        }
                    }

                    // Draw strikethrough
                    if cell.style.strikethrough {
                        let y = row * self.cell_height + self.cell_height / 2;
                        if y < height {
                            for x in
                                (col * self.cell_width)..((col + 1) * self.cell_width).min(width)
                            {
                                buffer[y * width + x] = self.pack_color(fg);
                            }
                        }
                    }
                }
            }
        }

        // Present the buffer using softbuffer 0.2 API
        self.graphics
            .set_buffer(&buffer, width as u16, height as u16);
    }

    /// Pack RGB color into u32
    fn pack_color(&self, (r, g, b): (u8, u8, u8)) -> u32 {
        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }

    /// Fill a rectangle with a color
    #[allow(clippy::too_many_arguments)]
    fn fill_rect(
        &self,
        buffer: &mut [u32],
        buf_width: usize,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        color: (u8, u8, u8),
    ) {
        let packed = self.pack_color(color);
        let buf_height = buffer.len() / buf_width;

        for dy in 0..h {
            let py = y + dy;
            if py >= buf_height {
                break;
            }
            for dx in 0..w {
                let px = x + dx;
                if px >= buf_width {
                    break;
                }
                buffer[py * buf_width + px] = packed;
            }
        }
    }

    /// Draw a character at the given cell position
    #[allow(clippy::too_many_arguments)]
    fn draw_char(
        &mut self,
        buffer: &mut [u32],
        buf_width: usize,
        buf_height: usize,
        col: usize,
        row: usize,
        c: char,
        fg: (u8, u8, u8),
        style: &Style,
    ) {
        // Get baseline first before mutable borrow
        let baseline = self.font.baseline();
        let cell_width = self.cell_width;
        let cell_height = self.cell_height;

        let glyph = self.font.rasterize(c, style.bold, style.italic);

        let cell_x = col * cell_width;
        let cell_y = row * cell_height;

        // Calculate glyph position
        let glyph_x = cell_x as i32 + glyph.xmin;
        let glyph_y = cell_y as i32 + baseline as i32 - glyph.ymin - glyph.height as i32;

        // Copy glyph data to avoid borrow issues
        let glyph_width = glyph.width;
        let glyph_height = glyph.height;
        let glyph_bitmap = glyph.bitmap.clone();

        // Draw glyph pixels
        for gy in 0..glyph_height {
            let py = glyph_y + gy as i32;
            if py < 0 || py >= buf_height as i32 {
                continue;
            }
            let py = py as usize;

            for gx in 0..glyph_width {
                let px = glyph_x + gx as i32;
                if px < 0 || px >= buf_width as i32 {
                    continue;
                }
                let px = px as usize;

                let alpha = glyph_bitmap[gy * glyph_width + gx];
                if alpha > 0 {
                    let existing = buffer[py * buf_width + px];
                    let bg_r = ((existing >> 16) & 0xFF) as u8;
                    let bg_g = ((existing >> 8) & 0xFF) as u8;
                    let bg_b = (existing & 0xFF) as u8;

                    // Alpha blend
                    let blend = |fg_val: u8, bg_val: u8, a: u8| -> u8 {
                        let fg_val = fg_val as u32;
                        let bg_val = bg_val as u32;
                        let a = a as u32;
                        ((fg_val * a + bg_val * (255 - a)) / 255) as u8
                    };

                    let r = blend(fg.0, bg_r, alpha);
                    let g = blend(fg.1, bg_g, alpha);
                    let b = blend(fg.2, bg_b, alpha);

                    buffer[py * buf_width + px] = Self::pack_color_static((r, g, b));
                }
            }
        }
    }

    /// Pack RGB color into u32 (static version)
    fn pack_color_static((r, g, b): (u8, u8, u8)) -> u32 {
        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
}
