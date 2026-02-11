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

/// Information about a tab for rendering
pub struct TabInfo<'a> {
    pub title: &'a str,
}

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
    /// Bold font (lazily loaded on first use)
    bold_font: Option<Font>,
    /// Whether we've attempted to load the bold font
    bold_font_loaded: bool,
    /// Fallback fonts for emoji and symbols (lazily loaded)
    fallback_fonts: Vec<Font>,
    /// Whether we've attempted to load fallback fonts
    fallback_fonts_loaded: bool,
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
    ///
    /// Performance optimizations:
    /// - Bold font is loaded lazily on first use (saves ~10-20ms on startup)
    /// - Common ASCII glyphs are pre-cached for faster first render
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

        // Bold font is loaded lazily on first use to improve startup time
        // Most terminal sessions don't use bold text immediately

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

        // Pre-cache common ASCII glyphs for faster first render
        let mut glyph_cache = HashMap::with_capacity(128);
        for c in ' '..='~' {
            let (metrics, bitmap) = font.rasterize(c, scaled_font_size);
            let entry = GlyphEntry {
                bitmap,
                width: metrics.width,
                height: metrics.height,
                xmin: metrics.xmin,
                ymin: metrics.ymin,
            };
            glyph_cache.insert((c, false), entry);
        }

        Ok(Self {
            context,
            surface,
            font,
            bold_font: None,
            bold_font_loaded: false,
            fallback_fonts: Vec::new(),
            fallback_fonts_loaded: false,
            glyph_cache,
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

    /// Set the color scheme (for theme switching)
    pub fn set_colors(&mut self, colors: ColorScheme) {
        self.colors = colors;
    }

    /// Render the terminal screen
    pub fn render(
        &mut self,
        screen: &Screen,
        selection: &Selection,
        scroll_offset: usize,
        tab_bar_height: u32,
        tabs: &[TabInfo<'_>],
        active_tab: usize,
        renaming: Option<(usize, &str, usize)>,
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

        let cols = screen.cols();
        let rows = screen.rows();
        let scrollback = screen.scrollback();
        let scrollback_len = scrollback.len();

        // Pre-cache glyphs for tab titles (and rename text if active)
        for tab in tabs {
            for c in tab.title.chars() {
                if c != ' ' {
                    self.ensure_glyph_cached(c, false);
                }
            }
        }
        if let Some((_, rename_text, _)) = &renaming {
            for c in rename_text.chars() {
                if c != ' ' {
                    self.ensure_glyph_cached(c, false);
                }
            }
        }
        self.ensure_glyph_cached('+', false);
        self.ensure_glyph_cached('x', false);

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

        // Draw tab bar
        if tab_bar_height > 0 && !tabs.is_empty() {
            Self::draw_tab_bar_static(
                &mut buffer,
                &self.glyph_cache,
                tabs,
                active_tab,
                tab_bar_height,
                width,
                height,
                &self.cell_size,
                bg_color,
                fg_color,
                renaming.as_ref().copied(),
            );
        }

        let cursor = screen.cursor();

        // Render each cell
        for row in 0..rows {
            // Calculate which line to render based on scroll offset
            let (line, is_from_scrollback, actual_screen_row) = if scroll_offset > 0 {
                let scrollback_row = scrollback_len.saturating_sub(scroll_offset) + row;
                if scrollback_row < scrollback_len {
                    // This row comes from scrollback
                    if let Some(sb_line) = scrollback.get(scrollback_row) {
                        (sb_line, true, None)
                    } else {
                        continue;
                    }
                } else {
                    // This row comes from screen
                    let screen_row = scrollback_row - scrollback_len;
                    if screen_row < rows {
                        (screen.line(screen_row), false, Some(screen_row))
                    } else {
                        continue;
                    }
                }
            } else {
                (screen.line(row), false, Some(row))
            };

            for col in 0..cols.min(line.cols()) {
                let cell = line.cell(col);

                // Skip continuation cells
                if cell.is_continuation() {
                    continue;
                }

                let x = (col as f32 * cell_width_px) as i32;
                let y = (row as f32 * cell_height_px) as i32 + tab_bar_height as i32;

                // Determine colors
                // Don't highlight empty selections (single click without drag)
                let is_selected = !selection.is_empty() && selection.contains(col, row as isize);
                // Check if this is the cursor position (regardless of visibility)
                let is_cursor_position = !is_from_scrollback
                    && scroll_offset == 0
                    && actual_screen_row == Some(cursor.row)
                    && cursor.col == col;
                // Solid cursor when visible, outline when hidden
                let is_solid_cursor = is_cursor_position && cursor.visible;
                let is_outline_cursor = is_cursor_position && !cursor.visible;

                let (fg, bg) = if is_selected {
                    (fg_color, sel_color)
                } else if is_solid_cursor {
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

                // Draw outline cursor when cursor is hidden (provides visual feedback)
                if is_outline_cursor {
                    Self::draw_rect_outline_static(
                        &mut buffer,
                        x,
                        y,
                        cell_w,
                        cell_h,
                        cursor_color,
                        width,
                        height,
                    );
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
                tab_bar_height,
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
        y_offset: u32,
    ) {
        let scrollbar_width = 12; // Wider for easier clicking
        let scrollbar_x = buf_width.saturating_sub(scrollbar_width) as i32;
        let scrollbar_height = buf_height.saturating_sub(y_offset) as i32;
        let y_off = y_offset as i32;

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
            y_off,
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
            y_off + thumb_y,
            scrollbar_width as i32 - 2,
            thumb_height,
            thumb_color,
            buf_width,
            buf_height,
        );
    }

    /// Ensure a glyph is cached
    ///
    /// Bold font is loaded lazily on first use to improve startup time
    fn ensure_glyph_cached(&mut self, c: char, bold: bool) {
        let key = (c, bold);
        if self.glyph_cache.contains_key(&key) {
            return;
        }

        // Lazy load bold font on first use
        if bold && !self.bold_font_loaded {
            self.bold_font_loaded = true;
            let bold_font_data = include_bytes!("../assets/DejaVuSansMono-Bold.ttf");
            self.bold_font =
                Font::from_bytes(bold_font_data as &[u8], FontSettings::default()).ok();
        }

        // Lazy load fallback fonts on first use (for emoji and symbols)
        if !self.fallback_fonts_loaded {
            self.fallback_fonts_loaded = true;
            self.load_fallback_fonts();
        }

        let font = if bold {
            self.bold_font.as_ref().unwrap_or(&self.font)
        } else {
            &self.font
        };

        // Check if primary font has this glyph (glyph index 0 means missing)
        let has_glyph = font.lookup_glyph_index(c) != 0;

        // Try fallback fonts if primary font doesn't have the glyph
        let (metrics, bitmap) = if has_glyph {
            font.rasterize(c, self.cell_size.baseline)
        } else {
            // Try each fallback font
            let mut found = None;
            for fallback in &self.fallback_fonts {
                if fallback.lookup_glyph_index(c) != 0 {
                    found = Some(fallback.rasterize(c, self.cell_size.baseline));
                    break;
                }
            }
            // Use primary font as last resort (will show tofu/replacement char)
            found.unwrap_or_else(|| font.rasterize(c, self.cell_size.baseline))
        };

        let entry = GlyphEntry {
            bitmap,
            width: metrics.width,
            height: metrics.height,
            xmin: metrics.xmin,
            ymin: metrics.ymin,
        };

        self.glyph_cache.insert(key, entry);
    }

    fn load_fallback_fonts(&mut self) {
        // System font paths for emoji and symbol fonts
        let fallback_paths: &[&str] = if cfg!(target_os = "macos") {
            &[
                "/System/Library/Fonts/Apple Color Emoji.ttc",
                "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
                "/Library/Fonts/Arial Unicode.ttf",
                "/System/Library/Fonts/Supplemental/Symbola.ttf",
            ]
        } else {
            // Linux paths
            &[
                "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
                "/usr/share/fonts/noto-emoji/NotoColorEmoji.ttf",
                "/usr/share/fonts/google-noto-emoji/NotoColorEmoji.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/TTF/DejaVuSans.ttf",
                "/usr/share/fonts/truetype/unifont/unifont.ttf",
                "/usr/share/fonts/unifont/unifont.ttf",
            ]
        };

        for path in fallback_paths {
            if let Ok(data) = std::fs::read(path) {
                if let Ok(font) = Font::from_bytes(data, FontSettings::default()) {
                    self.fallback_fonts.push(font);
                    log::debug!("Loaded fallback font: {}", path);
                }
            }
        }

        if self.fallback_fonts.is_empty() {
            log::warn!("No fallback fonts found for emoji/symbol support");
        }
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

    /// Draw a rectangle outline (hollow rectangle) for cursor indication
    #[allow(clippy::too_many_arguments)]
    fn draw_rect_outline_static(
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
        let thickness = 2; // 2-pixel thick outline for visibility

        // Draw top and bottom edges
        for dy in 0..thickness {
            // Top edge
            let py_top = y + dy;
            // Bottom edge
            let py_bottom = y + h - 1 - dy;

            for dx in 0..w {
                let px = x + dx;

                // Top edge
                if py_top >= 0 && py_top < buf_height as i32 && px >= 0 && px < buf_width as i32 {
                    let idx = (py_top as u32 * buf_width + px as u32) as usize;
                    if idx < buffer.len() {
                        buffer[idx] = pixel;
                    }
                }

                // Bottom edge
                if py_bottom >= 0
                    && py_bottom < buf_height as i32
                    && px >= 0
                    && px < buf_width as i32
                {
                    let idx = (py_bottom as u32 * buf_width + px as u32) as usize;
                    if idx < buffer.len() {
                        buffer[idx] = pixel;
                    }
                }
            }
        }

        // Draw left and right edges (excluding corners already drawn)
        for dy in thickness..(h - thickness) {
            let py = y + dy;
            if py < 0 || py >= buf_height as i32 {
                continue;
            }

            for dx in 0..thickness {
                // Left edge
                let px_left = x + dx;
                // Right edge
                let px_right = x + w - 1 - dx;

                if px_left >= 0 && px_left < buf_width as i32 {
                    let idx = (py as u32 * buf_width + px_left as u32) as usize;
                    if idx < buffer.len() {
                        buffer[idx] = pixel;
                    }
                }

                if px_right >= 0 && px_right < buf_width as i32 {
                    let idx = (py as u32 * buf_width + px_right as u32) as usize;
                    if idx < buffer.len() {
                        buffer[idx] = pixel;
                    }
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

    #[allow(clippy::too_many_arguments)]
    fn draw_tab_bar_static(
        buffer: &mut [u32],
        glyph_cache: &HashMap<(char, bool), GlyphEntry>,
        tabs: &[TabInfo<'_>],
        active_tab: usize,
        tab_bar_height: u32,
        buf_width: u32,
        buf_height: u32,
        cell_size: &CellSize,
        bg_color: (u8, u8, u8),
        fg_color: (u8, u8, u8),
        renaming: Option<(usize, &str, usize)>,
    ) {
        let tab_padding: u32 = 10;
        let close_btn_width: u32 = 20;
        let new_tab_btn_width: u32 = 32;
        let tab_max_width: u32 = 200;

        let tab_bar_bg = Self::blend_color(bg_color, (0, 0, 0), 0.3);
        let active_tab_bg = bg_color;
        let inactive_tab_bg = Self::blend_color(tab_bar_bg, bg_color, 0.3);
        let inactive_fg = Self::blend_color(fg_color, bg_color, 0.4);
        let separator_color = Self::blend_color(bg_color, (128, 128, 128), 0.3);
        let close_color = Self::blend_color(fg_color, (200, 80, 80), 0.5);

        Self::fill_rect_static(
            buffer,
            0,
            0,
            buf_width as i32,
            tab_bar_height as i32,
            tab_bar_bg,
            buf_width,
            buf_height,
        );

        let num_tabs = tabs.len() as u32;
        let available_width = buf_width.saturating_sub(new_tab_btn_width);
        let tab_width = if num_tabs > 0 {
            (available_width / num_tabs).min(tab_max_width)
        } else {
            tab_max_width
        };

        for (i, tab) in tabs.iter().enumerate() {
            let is_active = i == active_tab;
            let tab_x = (i as u32 * tab_width) as i32;
            let tab_bg = if is_active {
                active_tab_bg
            } else {
                inactive_tab_bg
            };
            let text_color = if is_active { fg_color } else { inactive_fg };

            Self::fill_rect_static(
                buffer,
                tab_x,
                0,
                tab_width as i32,
                tab_bar_height as i32,
                tab_bg,
                buf_width,
                buf_height,
            );

            if is_active {
                let accent = Self::blend_color(fg_color, (100, 149, 237), 0.5);
                Self::fill_rect_static(
                    buffer,
                    tab_x,
                    (tab_bar_height - 2) as i32,
                    tab_width as i32,
                    2,
                    accent,
                    buf_width,
                    buf_height,
                );
            }

            if i < tabs.len() - 1 {
                Self::fill_rect_static(
                    buffer,
                    tab_x + tab_width as i32 - 1,
                    4,
                    1,
                    (tab_bar_height - 8) as i32,
                    separator_color,
                    buf_width,
                    buf_height,
                );
            }

            let text_x = tab_x + tab_padding as i32;
            let text_y = ((tab_bar_height as f32 - cell_size.height) / 2.0).max(0.0) as i32;
            let max_text_width = tab_width.saturating_sub(tab_padding * 2 + close_btn_width) as i32;

            let (is_renaming_this, rename_text, rename_cursor) = match renaming {
                Some((idx, txt, cur)) if idx == i => (true, Some(txt), Some(cur)),
                _ => (false, None, None),
            };
            let draw_text = rename_text.unwrap_or(tab.title);

            Self::draw_text_static(
                buffer,
                glyph_cache,
                draw_text,
                text_x,
                text_y,
                text_color,
                cell_size.width,
                cell_size.baseline,
                buf_width,
                buf_height,
                max_text_width,
            );

            // Draw caret if renaming this tab
            if is_renaming_this {
                if let Some(cur) = rename_cursor {
                    let max_chars = (max_text_width as f32 / cell_size.width).floor() as usize;
                    let visible_cur = cur.min(max_chars);
                    let caret_x = text_x + (visible_cur as f32 * cell_size.width) as i32;
                    let caret_h = cell_size.height as i32 - 2;
                    let caret_y = text_y + 1;
                    let caret_color = Self::blend_color(text_color, (255, 255, 255), 0.2);
                    Self::fill_rect_static(
                        buffer,
                        caret_x,
                        caret_y,
                        2,
                        caret_h,
                        caret_color,
                        buf_width,
                        buf_height,
                    );
                }
            }

            if tabs.len() > 1 {
                let close_x = tab_x + tab_width as i32 - close_btn_width as i32;
                let close_y = text_y;
                if let Some(glyph) = glyph_cache.get(&('x', false)) {
                    Self::draw_glyph_static(
                        buffer,
                        close_x,
                        close_y,
                        glyph,
                        close_color,
                        cell_size.baseline,
                        buf_width,
                        buf_height,
                    );
                }
            }
        }

        let plus_btn_x = (num_tabs * tab_width) as i32;
        let plus_text_x = plus_btn_x + ((new_tab_btn_width as f32 - cell_size.width) / 2.0) as i32;
        let plus_text_y = ((tab_bar_height as f32 - cell_size.height) / 2.0).max(0.0) as i32;
        let plus_bg = Self::blend_color(tab_bar_bg, bg_color, 0.15);
        Self::fill_rect_static(
            buffer,
            plus_btn_x,
            0,
            new_tab_btn_width as i32,
            tab_bar_height as i32,
            plus_bg,
            buf_width,
            buf_height,
        );
        if let Some(glyph) = glyph_cache.get(&('+', false)) {
            Self::draw_glyph_static(
                buffer,
                plus_text_x,
                plus_text_y,
                glyph,
                fg_color,
                cell_size.baseline,
                buf_width,
                buf_height,
            );
        }

        Self::fill_rect_static(
            buffer,
            0,
            (tab_bar_height - 1) as i32,
            buf_width as i32,
            1,
            separator_color,
            buf_width,
            buf_height,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_text_static(
        buffer: &mut [u32],
        glyph_cache: &HashMap<(char, bool), GlyphEntry>,
        text: &str,
        x: i32,
        y: i32,
        color: (u8, u8, u8),
        cell_width: f32,
        baseline: f32,
        buf_width: u32,
        buf_height: u32,
        max_width: i32,
    ) {
        let mut cx = x;
        for ch in text.chars() {
            if cx - x >= max_width {
                break;
            }
            if ch != ' ' {
                if let Some(glyph) = glyph_cache.get(&(ch, false)) {
                    Self::draw_glyph_static(
                        buffer, cx, y, glyph, color, baseline, buf_width, buf_height,
                    );
                }
            }
            cx += cell_width as i32;
        }
    }

    fn blend_color(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
        (
            (a.0 as f32 * (1.0 - t) + b.0 as f32 * t) as u8,
            (a.1 as f32 * (1.0 - t) + b.1 as f32 * t) as u8,
            (a.2 as f32 * (1.0 - t) + b.2 as f32 * t) as u8,
        )
    }
}
