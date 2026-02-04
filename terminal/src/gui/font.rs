//! Font Renderer
//!
//! Handles font loading and glyph rasterization using fontdue.

use fontdue::{Font, FontSettings};
use std::collections::HashMap;

/// A rasterized glyph
#[derive(Clone)]
pub struct RasterizedGlyph {
    pub width: usize,
    pub height: usize,
    pub bitmap: Vec<u8>,
    pub advance_width: f32,
    pub xmin: i32,
    pub ymin: i32,
}

/// Font renderer for terminal text
pub struct FontRenderer {
    font: Font,
    font_size: f32,
    cell_width: usize,
    cell_height: usize,
    baseline: usize,
    glyph_cache: HashMap<(char, bool, bool), RasterizedGlyph>,
}

impl FontRenderer {
    /// Create a new font renderer with the specified font size
    pub fn new(font_size: f32) -> Result<Self, Box<dyn std::error::Error>> {
        let font_data = Self::load_font_data()?;
        let font = Font::from_bytes(font_data, FontSettings::default())
            .map_err(|e| format!("Failed to load font: {}", e))?;

        // Calculate cell dimensions based on font metrics
        let metrics = font.metrics('M', font_size);
        let cell_width = metrics.advance_width.ceil() as usize;
        let cell_height = (font_size * 1.4).ceil() as usize;
        let baseline = (font_size * 1.1).ceil() as usize;

        Ok(Self {
            font,
            font_size,
            cell_width: cell_width.max(1),
            cell_height: cell_height.max(1),
            baseline,
            glyph_cache: HashMap::new(),
        })
    }

    /// Load font data from system fonts
    fn load_font_data() -> Result<&'static [u8], Box<dyn std::error::Error>> {
        // Platform-specific font paths
        #[cfg(target_os = "macos")]
        let font_paths: &[&str] = &[
            // macOS system fonts
            "/System/Library/Fonts/Menlo.ttc",
            "/System/Library/Fonts/Monaco.ttf",
            "/System/Library/Fonts/SFMono-Regular.otf",
            "/Library/Fonts/SF-Mono-Regular.otf",
            // User-installed fonts
            "~/Library/Fonts/DejaVuSansMono.ttf",
            "/Library/Fonts/DejaVuSansMono.ttf",
        ];

        #[cfg(target_os = "linux")]
        let font_paths: &[&str] = &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
            "/usr/share/fonts/dejavu/DejaVuSansMono.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
            "/usr/share/fonts/liberation-mono/LiberationMono-Regular.ttf",
            "/usr/share/fonts/truetype/freefont/FreeMono.ttf",
            "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf",
            "/usr/share/fonts/opentype/fira/FiraMono-Regular.otf",
        ];

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let font_paths: &[&str] = &[];

        for path in font_paths {
            // Expand ~ to home directory on macOS
            let expanded_path = if path.starts_with("~/") {
                if let Some(home) = std::env::var_os("HOME") {
                    let home_str = home.to_string_lossy();
                    path.replacen("~", &home_str, 1)
                } else {
                    path.to_string()
                }
            } else {
                path.to_string()
            };

            if let Ok(data) = std::fs::read(&expanded_path) {
                let leaked: &'static [u8] = Box::leak(data.into_boxed_slice());
                return Ok(leaked);
            }
        }

        #[cfg(target_os = "macos")]
        return Err(
            "No suitable monospace font found. Menlo and Monaco should be available on macOS."
                .into(),
        );

        #[cfg(target_os = "linux")]
        return Err("No suitable monospace font found. Please install dejavu-fonts, liberation-fonts, or ubuntu-fonts.".into());

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        return Err("Unsupported platform for font loading.".into());
    }

    /// Get the cell dimensions
    pub fn cell_size(&self) -> (usize, usize) {
        (self.cell_width, self.cell_height)
    }

    /// Get the baseline offset
    pub fn baseline(&self) -> usize {
        self.baseline
    }

    /// Rasterize a glyph
    pub fn rasterize(&mut self, c: char, bold: bool, italic: bool) -> &RasterizedGlyph {
        let key = (c, bold, italic);

        self.glyph_cache.entry(key).or_insert_with(|| {
            let (metrics, bitmap) = self.font.rasterize(c, self.font_size);

            RasterizedGlyph {
                width: metrics.width,
                height: metrics.height,
                bitmap,
                advance_width: metrics.advance_width,
                xmin: metrics.xmin,
                ymin: metrics.ymin,
            }
        })
    }

    /// Get metrics for a character without rasterizing
    pub fn metrics(&self, c: char) -> fontdue::Metrics {
        self.font.metrics(c, self.font_size)
    }

    /// Clear the glyph cache
    pub fn clear_cache(&mut self) {
        self.glyph_cache.clear();
    }
}
