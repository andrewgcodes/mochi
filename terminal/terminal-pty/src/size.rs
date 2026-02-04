//! Window size for PTY

/// Window size in characters and pixels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    /// Number of rows (characters)
    pub rows: u16,
    /// Number of columns (characters)
    pub cols: u16,
    /// Width in pixels (optional, can be 0)
    pub pixel_width: u16,
    /// Height in pixels (optional, can be 0)
    pub pixel_height: u16,
}

impl WindowSize {
    /// Create a new window size
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }
    }

    /// Create a window size with pixel dimensions
    pub fn with_pixels(cols: u16, rows: u16, pixel_width: u16, pixel_height: u16) -> Self {
        Self {
            rows,
            cols,
            pixel_width,
            pixel_height,
        }
    }

    /// Convert to libc winsize structure
    pub fn to_winsize(&self) -> libc::winsize {
        libc::winsize {
            ws_row: self.rows,
            ws_col: self.cols,
            ws_xpixel: self.pixel_width,
            ws_ypixel: self.pixel_height,
        }
    }
}

impl Default for WindowSize {
    fn default() -> Self {
        Self::new(80, 24)
    }
}

impl From<libc::winsize> for WindowSize {
    fn from(ws: libc::winsize) -> Self {
        Self {
            rows: ws.ws_row,
            cols: ws.ws_col,
            pixel_width: ws.ws_xpixel,
            pixel_height: ws.ws_ypixel,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_size_default() {
        let size = WindowSize::default();
        assert_eq!(size.cols, 80);
        assert_eq!(size.rows, 24);
    }

    #[test]
    fn test_window_size_new() {
        let size = WindowSize::new(120, 40);
        assert_eq!(size.cols, 120);
        assert_eq!(size.rows, 40);
        assert_eq!(size.pixel_width, 0);
        assert_eq!(size.pixel_height, 0);
    }

    #[test]
    fn test_window_size_with_pixels() {
        let size = WindowSize::with_pixels(80, 24, 800, 600);
        assert_eq!(size.cols, 80);
        assert_eq!(size.rows, 24);
        assert_eq!(size.pixel_width, 800);
        assert_eq!(size.pixel_height, 600);
    }

    #[test]
    fn test_to_winsize() {
        let size = WindowSize::new(80, 24);
        let ws = size.to_winsize();
        assert_eq!(ws.ws_col, 80);
        assert_eq!(ws.ws_row, 24);
    }
}
