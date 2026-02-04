//! PTY (pseudo-terminal) creation and management
//!
//! This module provides the low-level PTY operations:
//! - Opening PTY master/slave pairs using posix_openpt
//! - Setting up the PTY for use
//! - Non-blocking I/O

use std::ffi::CStr;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};

use crate::size::WindowSize;

/// A PTY master file descriptor
#[derive(Debug)]
pub struct Pty {
    /// Master file descriptor
    master: File,
    /// Path to the slave device
    slave_path: String,
}

impl Pty {
    /// Open a new PTY master
    pub fn open() -> io::Result<Self> {
        // Open PTY master using posix_openpt
        let master_fd = unsafe { libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY) };
        if master_fd < 0 {
            return Err(io::Error::last_os_error());
        }

        // Grant access to the slave
        if unsafe { libc::grantpt(master_fd) } != 0 {
            unsafe { libc::close(master_fd) };
            return Err(io::Error::last_os_error());
        }

        // Unlock the slave
        if unsafe { libc::unlockpt(master_fd) } != 0 {
            unsafe { libc::close(master_fd) };
            return Err(io::Error::last_os_error());
        }

        // Get the slave path
        let slave_path = unsafe {
            let ptr = libc::ptsname(master_fd);
            if ptr.is_null() {
                libc::close(master_fd);
                return Err(io::Error::last_os_error());
            }
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        };

        // Convert to File for RAII
        let master = unsafe { File::from_raw_fd(master_fd) };

        Ok(Pty { master, slave_path })
    }

    /// Get the master file descriptor
    pub fn master_fd(&self) -> RawFd {
        self.master.as_raw_fd()
    }

    /// Get the path to the slave device
    pub fn slave_path(&self) -> &str {
        &self.slave_path
    }

    /// Open the slave device
    pub fn open_slave(&self) -> io::Result<File> {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NOCTTY)
            .open(&self.slave_path)
    }

    /// Set the window size
    pub fn set_size(&self, size: WindowSize) -> io::Result<()> {
        size.set_on_fd(self.master_fd())
    }

    /// Get the current window size
    pub fn get_size(&self) -> io::Result<WindowSize> {
        WindowSize::get_from_fd(self.master_fd())
    }

    /// Set non-blocking mode on the master
    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        let fd = self.master_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }

        let new_flags = if nonblocking {
            flags | libc::O_NONBLOCK
        } else {
            flags & !libc::O_NONBLOCK
        };

        if unsafe { libc::fcntl(fd, libc::F_SETFL, new_flags) } < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    /// Read from the PTY master (non-blocking if set)
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.master.read(buf)
    }

    /// Write to the PTY master
    pub fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.master.write(buf)
    }

    /// Write all bytes to the PTY master
    pub fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.master.write_all(buf)
    }

    /// Flush the PTY master
    pub fn flush(&mut self) -> io::Result<()> {
        self.master.flush()
    }

    /// Take the master file (consumes the Pty)
    pub fn into_master(self) -> File {
        self.master
    }

    /// Get a reference to the master file
    pub fn master(&self) -> &File {
        &self.master
    }

    /// Get a mutable reference to the master file
    pub fn master_mut(&mut self) -> &mut File {
        &mut self.master
    }
}

impl AsRawFd for Pty {
    fn as_raw_fd(&self) -> RawFd {
        self.master_fd()
    }
}

impl Read for Pty {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.master.read(buf)
    }
}

impl Write for Pty {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.master.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.master.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_open() {
        let pty = Pty::open().expect("Failed to open PTY");
        assert!(pty.master_fd() >= 0);
        assert!(!pty.slave_path().is_empty());
        assert!(pty.slave_path().starts_with("/dev/pts/"));
    }

    #[test]
    fn test_pty_size() {
        let pty = Pty::open().expect("Failed to open PTY");
        let size = WindowSize::new(30, 100);
        pty.set_size(size).expect("Failed to set size");

        let got = pty.get_size().expect("Failed to get size");
        assert_eq!(got.rows, 30);
        assert_eq!(got.cols, 100);
    }

    #[test]
    fn test_pty_nonblocking() {
        let pty = Pty::open().expect("Failed to open PTY");
        pty.set_nonblocking(true).expect("Failed to set nonblocking");
        pty.set_nonblocking(false).expect("Failed to unset nonblocking");
    }

    #[test]
    fn test_pty_open_slave() {
        let pty = Pty::open().expect("Failed to open PTY");
        let _slave = pty.open_slave().expect("Failed to open slave");
    }
}
