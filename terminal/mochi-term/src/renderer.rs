//! Terminal renderer using softbuffer (CPU rendering)
//!
//! Renders the terminal screen to a software buffer.

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::rc::Rc;

use fontdue::{Font, FontSettings};
use softbuffer::{Context, Surface};
use terminal_core::{Color, Screen, Selection};
use winit::window::Window;

use crate::config::ColorScheme;

/// Cell dimensions in pixels
#[derive(Debug, Clone, Copy)]
pub struct CellSize {
    pub width: f32,
    pub height: f32,
    pub baseline: f32,
}

/// Glyph cache entry
struct GlyphEntry {
    /// Bitmap data (alpha values)
    bitmap: Vec<u8>,
    /// Width in pixels
    width: usize,
    /// Height in pixels
    height: usize,
    /// X offset from cell origin
    xmin: i32,
    /// Y offset from baseline
    ymin: i32,
}

/// Terminal renderer
pub struct Renderer {
    /// Softbuffer context
    #[allow(dead_code)]
    context: Context<Rc<Window>>,
    /// Softbuffer surface
    surface: Surface<Rc<Window>, Rc<Window>>,
    /// Font
    font: Font,
    /// Bold font (optional)
    bold_font: Option<Font>,
    /// Glyph cache
    glyph_cache: HashMap<(char, bool), GlyphEntry>,
    /// Cell size
    cell_size: CellSize,
    /// Color scheme
    colors: ColorScheme,
    /// Current width
    width: u32,
    /// Current height
    height: u32,
    /// Current font size (scaled for HiDPI)
    font_size: f32,
}

impl Renderer {
    /// Create a new renderer
    pub fn new(
        window: Rc<Window>,
        font_size: f32,
        colors: ColorScheme,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let context = Context::new(window.clone())?;
        let surface = Surface::new(&context, window.clone())?;

        // Load default font (bundled in assets for cross-platform support)
        let font_data = include_bytes!("../assets/DejaVuSansMono.ttf");
        let font = Font::from_bytes(font_data as &[u8], FontSettings::default())?;

        // Load bold font (also bundled)
        let bold_font_data = include_bytes!("../assets/DejaVuSansMono-Bold.ttf");
        let bold_font = Font::from_bytes(bold_font_data as &[u8], FontSettings::default()).ok();

        // Scale font size for HiDPI displays
        let scale_factor = window.scale_factor() as f32;
        let scaled_font_size = font_size * scale_factor;

        // Calculate cell size
        let metrics = font.metrics('M', scaled_font_size);
        let cell_size = CellSize {
            width: metrics.advance_width.ceil(),
            height: (scaled_font_size * 1.4).ceil(),
            baseline: scaled_font_size,
        };

        let size = window.inner_size();

        Ok(Self {
            context,
            surface,
            font,
            bold_font,
            glyph_cache: HashMap::new(),
            cell_size,
            colors,
            width: size.width,
            height: size.height,
            font_size: scaled_font_size,
        })
    }

    /// Get cell size
    pub fn cell_size(&self) -> CellSize {
        self.cell_size
    }

    /// Get current font size
    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    /// Change font size and recalculate cell dimensions
    pub fn set_font_size(&mut self, font_size: f32) {
        self.font_size = font_size;

        // Recalculate cell size
        let metrics = self.font.metrics('M', font_size);
        self.cell_size = CellSize {
            width: metrics.advance_width.ceil(),
            height: (font_size * 1.4).ceil(),
            baseline: font_size,
        };

        // Clear glyph cache since font size changed
        self.glyph_cache.clear();
    }

    /// Resize the renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    /// Render the terminal screen
    pub fn render(
        &mut self,
        screen: &Screen,
        selection: &Selection,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let width = self.width;
        let height = self.height;

        if width == 0 || height == 0 {
            return Ok(());
        }

        // Resize surface
        self.surface.resize(
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
        )?;

        // Pre-cache colors we'll need
        let bg_color = self.colors.background_rgb();
        let fg_color = self.colors.foreground_rgb();
        let sel_color = self.colors.selection_rgb();
        let cursor_color = self.colors.cursor_rgb();
        let cell_width_px = self.cell_size.width;
        let cell_height_px = self.cell_size.height;
        let baseline = self.cell_size.baseline;

        // Pre-cache all glyphs we'll need
        let cols = screen.cols();
        let rows = screen.rows();
        for row in 0..rows {
            let line = screen.line(row);
            for col in 0..cols {
                let cell = line.cell(col);
                if !cell.is_continuation() && !cell.is_empty() {
                    let c = cell.display_char();
                    if c != ' ' {
                        self.ensure_glyph_cached(c, cell.attrs.bold);
                    }
                }
            }
        }

        let mut buffer = self.surface.buffer_mut()?;

        // Clear with background color
        let bg_pixel = Self::rgb_to_pixel(bg_color.0, bg_color.1, bg_color.2);
        buffer.fill(bg_pixel);

        let cursor = screen.cursor();

        // Render each cell
        for row in 0..rows {
            let line = screen.line(row);

            for col in 0..cols {
                let cell = line.cell(col);

                // Skip continuation cells
                if cell.is_continuation() {
                    continue;
                }

                let x = (col as f32 * cell_width_px) as i32;
                let y = (row as f32 * cell_height_px) as i32;

                // Determine colors
                let is_selected = selection.contains(col, row as isize);
                let is_cursor = cursor.visible && cursor.row == row && cursor.col == col;

                let (fg, bg) = if is_selected {
                    (fg_color, sel_color)
                } else if is_cursor {
                    (bg_color, cursor_color)
                } else {
                    let fg = Self::resolve_color_static(
                        &self.colors,
                        &cell.attrs.effective_fg(),
                        true,
                        fg_color,
                        bg_color,
                    );
                    let bg = Self::resolve_color_static(
                        &self.colors,
                        &cell.attrs.effective_bg(),
                        false,
                        fg_color,
                        bg_color,
                    );
                    (fg, bg)
                };

                // Draw background
                let cell_w = (cell.width() as f32 * cell_width_px) as i32;
                let cell_h = cell_height_px as i32;
                Self::fill_rect_static(&mut buffer, x, y, cell_w, cell_h, bg, width, height);

                // Draw character
                let c = cell.display_char();
                if c != ' ' && !cell.is_empty() {
                    if let Some(glyph) = self.glyph_cache.get(&(c, cell.attrs.bold)) {
                        Self::draw_glyph_static(
                            &mut buffer,
                            x,
                            y,
                            glyph,
                            fg,
                            baseline,
                            width,
                            height,
                        );
                    }
                }
            }
        }

        // Present
        buffer.present()?;

        Ok(())
    }

    /// Ensure a glyph is cached
    fn ensure_glyph_cached(&mut self, c: char, bold: bool) {
        let key = (c, bold);
        if self.glyph_cache.contains_key(&key) {
            return;
        }

        let font = if bold {
            self.bold_font.as_ref().unwrap_or(&self.font)
        } else {
            &self.font
        };

        let (metrics, bitmap) = font.rasterize(c, self.cell_size.baseline);

        let entry = GlyphEntry {
            bitmap,
            width: metrics.width,
            height: metrics.height,
            xmin: metrics.xmin,
            ymin: metrics.ymin,
        };

        self.glyph_cache.insert(key, entry);
    }

    /// Fill a rectangle with a color (static version)
    #[allow(clippy::too_many_arguments)]
    fn fill_rect_static(
        buffer: &mut [u32],
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        color: (u8, u8, u8),
        buf_width: u32,
        buf_height: u32,
    ) {
        let pixel = Self::rgb_to_pixel(color.0, color.1, color.2);

        for dy in 0..h {
            let py = y + dy;
            if py < 0 || py >= buf_height as i32 {
                continue;
            }

            for dx in 0..w {
                let px = x + dx;
                if px < 0 || px >= buf_width as i32 {
                    continue;
                }

                let idx = (py as u32 * buf_width + px as u32) as usize;
                if idx < buffer.len() {
                    buffer[idx] = pixel;
                }
            }
        }
    }

    /// Draw a glyph (static version)
    #[allow(clippy::too_many_arguments)]
    fn draw_glyph_static(
        buffer: &mut [u32],
        x: i32,
        y: i32,
        glyph: &GlyphEntry,
        color: (u8, u8, u8),
        baseline: f32,
        buf_width: u32,
        buf_height: u32,
    ) {
        if glyph.width == 0 || glyph.height == 0 {
            return;
        }

        // Calculate glyph position
        let gx = x + glyph.xmin;
        let gy = y + (baseline as i32) - glyph.ymin - glyph.height as i32;

        for dy in 0..glyph.height {
            let py = gy + dy as i32;
            if py < 0 || py >= buf_height as i32 {
                continue;
            }

            for dx in 0..glyph.width {
                let px = gx + dx as i32;
                if px < 0 || px >= buf_width as i32 {
                    continue;
                }

                let alpha = glyph.bitmap[dy * glyph.width + dx];
                if alpha == 0 {
                    continue;
                }

                let idx = (py as u32 * buf_width + px as u32) as usize;
                if idx < buffer.len() {
                    if alpha == 255 {
                        buffer[idx] = Self::rgb_to_pixel(color.0, color.1, color.2);
                    } else {
                        // Alpha blend
                        let existing = buffer[idx];
                        let er = ((existing >> 16) & 0xFF) as u8;
                        let eg = ((existing >> 8) & 0xFF) as u8;
                        let eb = (existing & 0xFF) as u8;

                        let a = alpha as u32;
                        let ia = 255 - a;

                        let r = ((color.0 as u32 * a + er as u32 * ia) / 255) as u8;
                        let g = ((color.1 as u32 * a + eg as u32 * ia) / 255) as u8;
                        let b = ((color.2 as u32 * a + eb as u32 * ia) / 255) as u8;

                        buffer[idx] = Self::rgb_to_pixel(r, g, b);
                    }
                }
            }
        }
    }

    /// Resolve a terminal color to RGB (static version)
    fn resolve_color_static(
        colors: &ColorScheme,
        color: &Color,
        is_fg: bool,
        fg_default: (u8, u8, u8),
        bg_default: (u8, u8, u8),
    ) -> (u8, u8, u8) {
        match color {
            Color::Default => {
                if is_fg {
                    fg_default
                } else {
                    bg_default
                }
            }
            Color::Indexed(idx) => {
                if *idx < 16 {
                    colors.ansi_rgb(*idx as usize)
                } else {
                    color.to_rgb()
                }
            }
            Color::Rgb { r, g, b } => (*r, *g, *b),
        }
    }

    /// Convert RGB to pixel value (ARGB format)
    fn rgb_to_pixel(r: u8, g: u8, b: u8) -> u32 {
        0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
}
