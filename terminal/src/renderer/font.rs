//! Font Rendering
//!
//! Handles font loading, glyph rasterization, and texture atlas management.
//! Uses fontdue for simple, fast font rasterization.

use std::collections::HashMap;
use std::path::Path;

use fontdue::{Font, FontSettings, Metrics};

/// A rendered glyph with its metrics and bitmap
#[derive(Debug, Clone)]
pub struct RasterizedGlyph {
    /// Glyph metrics
    pub metrics: Metrics,
    /// Bitmap data (grayscale, 1 byte per pixel)
    pub bitmap: Vec<u8>,
}

/// Font renderer that handles glyph rasterization and caching
pub struct FontRenderer {
    /// The loaded font
    font: Font,
    /// Font size in pixels
    font_size: f32,
    /// Cached glyphs: char -> RasterizedGlyph
    glyph_cache: HashMap<char, RasterizedGlyph>,
    /// Cell width in pixels
    cell_width: f32,
    /// Cell height in pixels
    cell_height: f32,
    /// Baseline offset from top of cell
    baseline: f32,
}

impl FontRenderer {
    /// Create a new font renderer with the given font file and size
    pub fn new(font_path: &Path, font_size: f32) -> Result<Self, FontError> {
        let font_data = std::fs::read(font_path).map_err(|e| FontError::IoError(e.to_string()))?;

        Self::from_bytes(&font_data, font_size)
    }

    /// Create a font renderer from font data bytes
    pub fn from_bytes(font_data: &[u8], font_size: f32) -> Result<Self, FontError> {
        let font = Font::from_bytes(font_data, FontSettings::default())
            .map_err(|e| FontError::ParseError(e.to_string()))?;

        // Calculate cell dimensions based on font metrics
        // Use 'M' as reference for width (em-width)
        let (metrics, _) = font.rasterize('M', font_size);
        let cell_width = metrics.advance_width;

        // Calculate height from line metrics
        let line_metrics = font
            .horizontal_line_metrics(font_size)
            .ok_or_else(|| FontError::ParseError("No line metrics".to_string()))?;

        let cell_height = line_metrics.new_line_size;
        let baseline = line_metrics.ascent;

        Ok(Self {
            font,
            font_size,
            glyph_cache: HashMap::new(),
            cell_width,
            cell_height,
            baseline,
        })
    }

    /// Create a font renderer with a built-in fallback font
    pub fn with_default_font(font_size: f32) -> Result<Self, FontError> {
        // Try to load system monospace fonts in order of preference
        let font_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
            "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf",
            "/usr/share/fonts/truetype/freefont/FreeMono.ttf",
        ];

        for path in &font_paths {
            if let Ok(renderer) = Self::new(Path::new(path), font_size) {
                log::info!("Loaded font: {}", path);
                return Ok(renderer);
            }
        }

        Err(FontError::NoFontFound)
    }

    /// Get the cell width in pixels
    pub fn cell_width(&self) -> f32 {
        self.cell_width
    }

    /// Get the cell height in pixels
    pub fn cell_height(&self) -> f32 {
        self.cell_height
    }

    /// Get the baseline offset from top of cell
    pub fn baseline(&self) -> f32 {
        self.baseline
    }

    /// Get the font size
    pub fn font_size(&self) -> f32 {
        self.font_size
    }

    /// Rasterize a character, using cache if available
    pub fn rasterize(&mut self, c: char) -> &RasterizedGlyph {
        if !self.glyph_cache.contains_key(&c) {
            let (metrics, bitmap) = self.font.rasterize(c, self.font_size);
            self.glyph_cache.insert(
                c,
                RasterizedGlyph {
                    metrics,
                    bitmap,
                },
            );
        }
        self.glyph_cache.get(&c).unwrap()
    }

    /// Rasterize a character and return owned data (for texture upload)
    pub fn rasterize_owned(&mut self, c: char) -> RasterizedGlyph {
        self.rasterize(c).clone()
    }

    /// Clear the glyph cache
    pub fn clear_cache(&mut self) {
        self.glyph_cache.clear();
    }

    /// Calculate grid dimensions for a given pixel size
    pub fn calculate_grid_size(&self, pixel_width: u32, pixel_height: u32) -> (usize, usize) {
        let cols = (pixel_width as f32 / self.cell_width).floor() as usize;
        let rows = (pixel_height as f32 / self.cell_height).floor() as usize;
        (cols.max(1), rows.max(1))
    }

    /// Calculate pixel position for a cell
    pub fn cell_to_pixel(&self, col: usize, row: usize) -> (f32, f32) {
        let x = col as f32 * self.cell_width;
        let y = row as f32 * self.cell_height;
        (x, y)
    }

    /// Calculate cell position from pixel coordinates
    pub fn pixel_to_cell(&self, x: f32, y: f32) -> (usize, usize) {
        let col = (x / self.cell_width).floor() as usize;
        let row = (y / self.cell_height).floor() as usize;
        (col, row)
    }
}

/// Font-related errors
#[derive(Debug)]
pub enum FontError {
    IoError(String),
    ParseError(String),
    NoFontFound,
}

impl std::fmt::Display for FontError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontError::IoError(e) => write!(f, "Font IO error: {}", e),
            FontError::ParseError(e) => write!(f, "Font parse error: {}", e),
            FontError::NoFontFound => write!(f, "No suitable font found"),
        }
    }
}

impl std::error::Error for FontError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_grid_size() {
        // Create a mock font renderer with known cell dimensions
        // We can't easily test without a real font, so skip if no font available
        if let Ok(renderer) = FontRenderer::with_default_font(16.0) {
            let (cols, rows) = renderer.calculate_grid_size(800, 600);
            assert!(cols > 0);
            assert!(rows > 0);
        }
    }

    #[test]
    fn test_cell_to_pixel() {
        if let Ok(renderer) = FontRenderer::with_default_font(16.0) {
            let (x, y) = renderer.cell_to_pixel(0, 0);
            assert_eq!(x, 0.0);
            assert_eq!(y, 0.0);

            let (x, y) = renderer.cell_to_pixel(1, 1);
            assert!(x > 0.0);
            assert!(y > 0.0);
        }
    }

    #[test]
    fn test_pixel_to_cell() {
        if let Ok(renderer) = FontRenderer::with_default_font(16.0) {
            let (col, row) = renderer.pixel_to_cell(0.0, 0.0);
            assert_eq!(col, 0);
            assert_eq!(row, 0);
        }
    }
}
