//! PTY (pseudoterminal) management
//!
//! Handles creation and management of the PTY master/slave pair.

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};

use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::pty::{grantpt, posix_openpt, ptsname, unlockpt, PtyMaster};
use nix::sys::termios::{self, SetArg};

use crate::error::{Error, Result};
use crate::size::WindowSize;

/// A pseudoterminal master
pub struct Pty {
    /// The PTY master file descriptor
    master: PtyMaster,
    /// File wrapper for I/O
    file: File,
    /// Path to the slave PTY
    slave_path: String,
}

impl Pty {
    /// Create a new PTY
    pub fn new() -> Result<Self> {
        // Open PTY master
        let master = posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY)?;

        // Grant access to slave
        grantpt(&master)?;

        // Unlock slave
        unlockpt(&master)?;

        // Get slave path
        let slave_path = unsafe { ptsname(&master)? };

        // Create file wrapper for I/O
        let fd = master.as_raw_fd();
        let file = unsafe { File::from_raw_fd(libc::dup(fd)) };

        Ok(Self {
            master,
            file,
            slave_path,
        })
    }

    /// Get the path to the slave PTY
    pub fn slave_path(&self) -> &str {
        &self.slave_path
    }

    /// Get the raw file descriptor of the master
    pub fn as_raw_fd(&self) -> RawFd {
        self.master.as_raw_fd()
    }

    /// Set the PTY to non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<()> {
        let fd = self.master.as_raw_fd();
        let flags = fcntl(fd, FcntlArg::F_GETFL)?;
        let flags = OFlag::from_bits_truncate(flags);

        let new_flags = if nonblocking {
            flags | OFlag::O_NONBLOCK
        } else {
            flags & !OFlag::O_NONBLOCK
        };

        fcntl(fd, FcntlArg::F_SETFL(new_flags))?;
        Ok(())
    }

    /// Set the window size of the PTY
    pub fn set_window_size(&self, size: WindowSize) -> Result<()> {
        let ws = size.to_winsize();
        let fd = self.master.as_raw_fd();

        // Note: On macOS, ioctl expects c_ulong for request parameter
        let result = unsafe { libc::ioctl(fd, libc::TIOCSWINSZ as libc::c_ulong, &ws) };

        if result == -1 {
            Err(Error::WindowSize(io::Error::last_os_error().to_string()))
        } else {
            Ok(())
        }
    }

    /// Get the current window size of the PTY
    pub fn get_window_size(&self) -> Result<WindowSize> {
        let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
        let fd = self.master.as_raw_fd();

        // Note: On macOS, ioctl expects c_ulong for request parameter
        let result = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ as libc::c_ulong, &mut ws) };

        if result == -1 {
            Err(Error::WindowSize(io::Error::last_os_error().to_string()))
        } else {
            Ok(WindowSize::from(ws))
        }
    }

    /// Read from the PTY master
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }

    /// Write to the PTY master
    pub fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    /// Write all bytes to the PTY master
    pub fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.file.write_all(buf)
    }

    /// Flush the PTY master
    pub fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }

    /// Try to read from the PTY (non-blocking)
    /// Returns Ok(0) if no data available (EAGAIN/EWOULDBLOCK)
    pub fn try_read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.file.read(buf) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(e),
        }
    }
}

impl AsRawFd for Pty {
    fn as_raw_fd(&self) -> RawFd {
        self.master.as_raw_fd()
    }
}

impl AsFd for Pty {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.master.as_fd()
    }
}

/// Open the slave side of a PTY
pub fn open_slave(path: &str) -> Result<OwnedFd> {
    use std::ffi::CString;

    let path_cstr = CString::new(path).map_err(|e| Error::PtyCreation(e.to_string()))?;

    let fd = unsafe { libc::open(path_cstr.as_ptr(), libc::O_RDWR | libc::O_NOCTTY) };

    if fd < 0 {
        return Err(Error::PtyCreation(io::Error::last_os_error().to_string()));
    }

    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

/// Configure the slave PTY for terminal use
pub fn configure_slave(fd: RawFd) -> Result<()> {
    // Get current terminal settings
    // SAFETY: fd is a valid file descriptor from open_slave
    let borrowed_fd = unsafe { BorrowedFd::borrow_raw(fd) };
    let mut termios = termios::tcgetattr(borrowed_fd)?;

    // Configure for raw mode with some modifications
    // This is similar to cfmakeraw but we keep some settings

    // Input flags: disable most processing
    termios.input_flags &= !(termios::InputFlags::IGNBRK
        | termios::InputFlags::BRKINT
        | termios::InputFlags::PARMRK
        | termios::InputFlags::ISTRIP
        | termios::InputFlags::INLCR
        | termios::InputFlags::IGNCR
        | termios::InputFlags::ICRNL
        | termios::InputFlags::IXON);

    // Output flags: enable OPOST and ONLCR for proper newline handling
    // ONLCR converts LF to CR+LF on output, which is standard terminal behavior
    termios.output_flags |= termios::OutputFlags::OPOST | termios::OutputFlags::ONLCR;

    // Local flags: disable echo and canonical mode
    termios.local_flags &= !(termios::LocalFlags::ECHO
        | termios::LocalFlags::ECHONL
        | termios::LocalFlags::ICANON
        | termios::LocalFlags::ISIG
        | termios::LocalFlags::IEXTEN);

    // Control flags: 8-bit characters
    termios.control_flags &= !(termios::ControlFlags::CSIZE | termios::ControlFlags::PARENB);
    termios.control_flags |= termios::ControlFlags::CS8;

    // Control characters
    termios.control_chars[termios::SpecialCharacterIndices::VMIN as usize] = 1;
    termios.control_chars[termios::SpecialCharacterIndices::VTIME as usize] = 0;

    // Apply settings
    // SAFETY: fd is a valid file descriptor from open_slave
    let borrowed_fd = unsafe { BorrowedFd::borrow_raw(fd) };
    termios::tcsetattr(borrowed_fd, SetArg::TCSANOW, &termios)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_creation() {
        let pty = Pty::new();
        assert!(pty.is_ok());

        let pty = pty.unwrap();
        assert!(!pty.slave_path().is_empty());
        // Linux uses /dev/pts/*, macOS uses /dev/ttys*
        #[cfg(target_os = "linux")]
        assert!(pty.slave_path().starts_with("/dev/pts/"));
        #[cfg(target_os = "macos")]
        assert!(pty.slave_path().starts_with("/dev/ttys"));
    }

    #[test]
    fn test_pty_window_size() {
        let pty = Pty::new().unwrap();

        let size = WindowSize::new(120, 40);
        pty.set_window_size(size).unwrap();

        let retrieved = pty.get_window_size().unwrap();
        assert_eq!(retrieved.cols, 120);
        assert_eq!(retrieved.rows, 40);
    }

    #[test]
    fn test_pty_nonblocking() {
        let pty = Pty::new().unwrap();
        assert!(pty.set_nonblocking(true).is_ok());
        assert!(pty.set_nonblocking(false).is_ok());
    }
}
