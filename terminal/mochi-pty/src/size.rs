//! Window size management for PTY

use std::os::unix::io::RawFd;

/// Window size in rows, columns, and pixels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    /// Number of rows (lines)
    pub rows: u16,
    /// Number of columns (characters per line)
    pub cols: u16,
    /// Width in pixels (optional, can be 0)
    pub pixel_width: u16,
    /// Height in pixels (optional, can be 0)
    pub pixel_height: u16,
}

impl WindowSize {
    /// Create a new window size
    pub fn new(rows: u16, cols: u16) -> Self {
        WindowSize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }
    }

    /// Create a new window size with pixel dimensions
    pub fn with_pixels(rows: u16, cols: u16, pixel_width: u16, pixel_height: u16) -> Self {
        WindowSize {
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

    /// Create from libc winsize structure
    pub fn from_winsize(ws: libc::winsize) -> Self {
        WindowSize {
            rows: ws.ws_row,
            cols: ws.ws_col,
            pixel_width: ws.ws_xpixel,
            pixel_height: ws.ws_ypixel,
        }
    }

    /// Set the window size on a file descriptor (PTY master)
    pub fn set_on_fd(&self, fd: RawFd) -> std::io::Result<()> {
        let ws = self.to_winsize();
        let result = unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, &ws) };
        if result == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Get the window size from a file descriptor
    pub fn get_from_fd(fd: RawFd) -> std::io::Result<Self> {
        let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) };
        if result == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(WindowSize::from_winsize(ws))
        }
    }
}

impl Default for WindowSize {
    fn default() -> Self {
        WindowSize::new(24, 80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_size_new() {
        let size = WindowSize::new(24, 80);
        assert_eq!(size.rows, 24);
        assert_eq!(size.cols, 80);
        assert_eq!(size.pixel_width, 0);
        assert_eq!(size.pixel_height, 0);
    }

    #[test]
    fn test_window_size_with_pixels() {
        let size = WindowSize::with_pixels(24, 80, 640, 480);
        assert_eq!(size.rows, 24);
        assert_eq!(size.cols, 80);
        assert_eq!(size.pixel_width, 640);
        assert_eq!(size.pixel_height, 480);
    }

    #[test]
    fn test_winsize_conversion() {
        let size = WindowSize::with_pixels(30, 100, 800, 600);
        let ws = size.to_winsize();
        let size2 = WindowSize::from_winsize(ws);
        assert_eq!(size, size2);
    }
}
