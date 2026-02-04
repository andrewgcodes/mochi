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
    /// Line height multiplier
    line_height: f32,
}

impl Renderer {
    /// Create a new renderer
    pub fn new(
        window: Rc<Window>,
        font_size: f32,
        line_height: f32,
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

        // Calculate cell size using configurable line height
        let metrics = font.metrics('M', scaled_font_size);
        let cell_size = CellSize {
            width: metrics.advance_width.ceil(),
            height: (scaled_font_size * line_height).ceil(),
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
            line_height,
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

        // Recalculate cell size using stored line height
        let metrics = self.font.metrics('M', font_size);
        self.cell_size = CellSize {
            width: metrics.advance_width.ceil(),
            height: (font_size * self.line_height).ceil(),
            baseline: font_size,
        };

        // Clear glyph cache since font size changed
        self.glyph_cache.clear();
    }

    /// Set line height and recalculate cell dimensions
    #[allow(dead_code)]
    pub fn set_line_height(&mut self, line_height: f32) {
        self.line_height = line_height;

        // Recalculate cell size
        let metrics = self.font.metrics('M', self.font_size);
        self.cell_size = CellSize {
            width: metrics.advance_width.ceil(),
            height: (self.font_size * line_height).ceil(),
            baseline: self.font_size,
        };
    }

    /// Set the color scheme/theme
    pub fn set_theme(&mut self, theme: crate::config::ThemeName) {
        use crate::config::ThemeName;
        self.colors = match theme {
            ThemeName::Dark => ColorScheme::dark(),
            ThemeName::Light => ColorScheme::light(),
            ThemeName::SolarizedDark => ColorScheme::solarized_dark(),
            ThemeName::SolarizedLight => ColorScheme::solarized_light(),
            ThemeName::Dracula => ColorScheme::dracula(),
            ThemeName::Nord => ColorScheme::nord(),
            ThemeName::Custom => self.colors.clone(), // Keep current custom colors
        };
    }

    /// Get current color scheme
    #[allow(dead_code)]
    pub fn colors(&self) -> &ColorScheme {
        &self.colors
    }

    /// Resize the renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    /// Render the terminal screen
    ///
    /// # Arguments
    /// * `screen` - The terminal screen to render
    /// * `selection` - Current text selection
    /// * `scroll_offset` - Lines scrolled back from bottom
    /// * `search_matches` - Optional search matches as (line_idx, start_col, end_col)
    /// * `current_match_idx` - Index of the currently highlighted match
    pub fn render(
        &mut self,
        screen: &Screen,
        selection: &Selection,
        scroll_offset: usize,
        search_matches: Option<&[(usize, usize, usize)]>,
        current_match_idx: Option<usize>,
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
        let search_match_color = self.colors.search_match_rgb();
        let search_match_current_color = self.colors.search_match_current_rgb();
        let cell_width_px = self.cell_size.width;
        let cell_height_px = self.cell_size.height;
        let baseline = self.cell_size.baseline;

        let cols = screen.cols();
        let rows = screen.rows();
        let scrollback = screen.scrollback();
        let scrollback_len = scrollback.len();

        // Pre-cache all glyphs we'll need (from both screen and scrollback if scrolled)
        for row in 0..rows {
            let line = if scroll_offset > 0 {
                // Calculate which line to show
                let scrollback_row = scrollback_len.saturating_sub(scroll_offset) + row;
                if scrollback_row < scrollback_len {
                    // This row comes from scrollback
                    if let Some(sb_line) = scrollback.get(scrollback_row) {
                        for col in 0..cols.min(sb_line.cols()) {
                            let cell = sb_line.cell(col);
                            if !cell.is_continuation() && !cell.is_empty() {
                                let c = cell.display_char();
                                if c != ' ' {
                                    self.ensure_glyph_cached(c, cell.attrs.bold);
                                }
                            }
                        }
                    }
                    continue;
                } else {
                    // This row comes from screen
                    let screen_row = scrollback_row - scrollback_len;
                    if screen_row < rows {
                        screen.line(screen_row)
                    } else {
                        continue;
                    }
                }
            } else {
                screen.line(row)
            };

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
            // Calculate which line to render based on scroll offset
            let (line, is_from_scrollback, actual_screen_row, line_index) = if scroll_offset > 0 {
                let scrollback_row = scrollback_len.saturating_sub(scroll_offset) + row;
                if scrollback_row < scrollback_len {
                    // This row comes from scrollback
                    if let Some(sb_line) = scrollback.get(scrollback_row) {
                        (sb_line, true, None, scrollback_row)
                    } else {
                        continue;
                    }
                } else {
                    // This row comes from screen
                    let screen_row = scrollback_row - scrollback_len;
                    if screen_row < rows {
                        (
                            screen.line(screen_row),
                            false,
                            Some(screen_row),
                            scrollback_len + screen_row,
                        )
                    } else {
                        continue;
                    }
                }
            } else {
                (screen.line(row), false, Some(row), scrollback_len + row)
            };

            for col in 0..cols.min(line.cols()) {
                let cell = line.cell(col);

                // Skip continuation cells
                if cell.is_continuation() {
                    continue;
                }

                let x = (col as f32 * cell_width_px) as i32;
                let y = (row as f32 * cell_height_px) as i32;

                // Determine colors
                let is_selected = selection.contains(col, row as isize);
                let is_cursor = !is_from_scrollback
                    && scroll_offset == 0
                    && cursor.visible
                    && actual_screen_row == Some(cursor.row)
                    && cursor.col == col;

                // Check if this cell is part of a search match
                let (is_search_match, is_current_match) = if let Some(matches) = search_matches {
                    let mut found_match = false;
                    let mut is_current = false;
                    for (idx, &(match_line, start_col, end_col)) in matches.iter().enumerate() {
                        if match_line == line_index && col >= start_col && col < end_col {
                            found_match = true;
                            if current_match_idx == Some(idx) {
                                is_current = true;
                            }
                            break;
                        }
                    }
                    (found_match, is_current)
                } else {
                    (false, false)
                };

                let (fg, bg) = if is_selected {
                    (fg_color, sel_color)
                } else if is_cursor {
                    (bg_color, cursor_color)
                } else if is_current_match {
                    (fg_color, search_match_current_color)
                } else if is_search_match {
                    (fg_color, search_match_color)
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

        // Draw scrollbar if there's scrollback content
        if scrollback_len > 0 {
            Self::draw_scrollbar_static(
                &mut buffer,
                scroll_offset,
                scrollback_len,
                rows,
                width,
                height,
            );
        }

        // Present
        buffer.present()?;

        Ok(())
    }

    /// Draw a scrollbar on the right side of the terminal (static version)
    fn draw_scrollbar_static(
        buffer: &mut [u32],
        scroll_offset: usize,
        scrollback_len: usize,
        visible_rows: usize,
        buf_width: u32,
        buf_height: u32,
    ) {
        let scrollbar_width = 8;
        let scrollbar_x = buf_width.saturating_sub(scrollbar_width) as i32;
        let scrollbar_height = buf_height as i32;

        // Total content = scrollback + visible screen
        let total_lines = scrollback_len + visible_rows;

        // Calculate thumb size (proportional to visible content)
        let thumb_height =
            ((visible_rows as f32 / total_lines as f32) * scrollbar_height as f32).max(20.0) as i32;

        // Calculate thumb position
        // When scroll_offset = 0, thumb is at bottom
        // When scroll_offset = scrollback_len, thumb is at top
        let scroll_range = scrollbar_height - thumb_height;
        let thumb_y = if scrollback_len > 0 {
            ((scrollback_len - scroll_offset) as f32 / scrollback_len as f32 * scroll_range as f32)
                as i32
        } else {
            scroll_range
        };

        // Draw scrollbar track (semi-transparent dark)
        let track_color = (40, 40, 40);
        Self::fill_rect_static(
            buffer,
            scrollbar_x,
            0,
            scrollbar_width as i32,
            scrollbar_height,
            track_color,
            buf_width,
            buf_height,
        );

        // Draw scrollbar thumb
        let thumb_color = if scroll_offset > 0 {
            (120, 120, 120) // Brighter when scrolled
        } else {
            (80, 80, 80) // Dimmer at bottom
        };
        Self::fill_rect_static(
            buffer,
            scrollbar_x + 1,
            thumb_y,
            scrollbar_width as i32 - 2,
            thumb_height,
            thumb_color,
            buf_width,
            buf_height,
        );
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
