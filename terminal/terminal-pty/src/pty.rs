//! PTY (pseudoterminal) management
//!
//! Handles creation and management of the PTY master/slave pair.
//!
//! Note: On macOS, posix_openpt() returns a file descriptor that isn't fully
//! functional until the slave side is opened. Operations like ioctl(TIOCSWINSZ)
//! fail with ENOTTY until then. We use openpty() on macOS which opens both
//! master and slave at once, avoiding this issue.

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd};

use nix::fcntl::{fcntl, FcntlArg, OFlag};
#[cfg(target_os = "linux")]
use nix::pty::{grantpt, posix_openpt, ptsname, unlockpt, PtyMaster};
#[cfg(target_os = "macos")]
use nix::pty::openpty;
use nix::sys::termios::{self, SetArg};

use crate::error::{Error, Result};
use crate::size::WindowSize;

/// A pseudoterminal master
#[cfg(target_os = "linux")]
pub struct Pty {
    /// The PTY master file descriptor
    master: PtyMaster,
    /// File wrapper for I/O
    file: File,
    /// Path to the slave PTY
    slave_path: String,
}

/// A pseudoterminal master (macOS version using openpty)
#[cfg(target_os = "macos")]
pub struct Pty {
    /// The PTY master file descriptor
    master_fd: RawFd,
    /// The PTY slave file descriptor (kept open to make master functional)
    _slave_fd: OwnedFd,
    /// File wrapper for I/O
    file: File,
    /// Path to the slave PTY
    slave_path: String,
}

#[cfg(target_os = "linux")]
impl Pty {
    /// Create a new PTY
    pub fn new() -> Result<Self> {
        let master = posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY)?;
        grantpt(&master)?;
        unlockpt(&master)?;
        let slave_path = unsafe { ptsname(&master)? };
        let fd = master.as_raw_fd();
        let file = unsafe { File::from_raw_fd(libc::dup(fd)) };
        Ok(Self { master, file, slave_path })
    }

    pub fn slave_path(&self) -> &str { &self.slave_path }
    pub fn master_fd(&self) -> RawFd { self.master.as_raw_fd() }

    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<()> {
        let fd = self.master.as_raw_fd();
        let flags = fcntl(fd, FcntlArg::F_GETFL)?;
        let flags = OFlag::from_bits_truncate(flags);
        let new_flags = if nonblocking { flags | OFlag::O_NONBLOCK } else { flags & !OFlag::O_NONBLOCK };
        fcntl(fd, FcntlArg::F_SETFL(new_flags))?;
        Ok(())
    }

    pub fn set_window_size(&self, size: WindowSize) -> Result<()> {
        let ws = size.to_winsize();
        let fd = self.master.as_raw_fd();
        let result = unsafe { libc::ioctl(fd, libc::TIOCSWINSZ as libc::c_ulong, &ws) };
        if result == -1 { Err(Error::WindowSize(io::Error::last_os_error().to_string())) } else { Ok(()) }
    }

    pub fn get_window_size(&self) -> Result<WindowSize> {
        let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
        let fd = self.master.as_raw_fd();
        let result = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ as libc::c_ulong, &mut ws) };
        if result == -1 { Err(Error::WindowSize(io::Error::last_os_error().to_string())) } else { Ok(WindowSize::from(ws)) }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> { self.file.read(buf) }
    pub fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.file.write(buf) }
    pub fn write_all(&mut self, buf: &[u8]) -> io::Result<()> { self.file.write_all(buf) }
    pub fn flush(&mut self) -> io::Result<()> { self.file.flush() }

    pub fn try_read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.file.read(buf) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(e),
        }
    }
}

#[cfg(target_os = "linux")]
impl AsRawFd for Pty {
    fn as_raw_fd(&self) -> RawFd { self.master.as_raw_fd() }
}

#[cfg(target_os = "linux")]
impl AsFd for Pty {
    fn as_fd(&self) -> BorrowedFd<'_> { self.master.as_fd() }
}

#[cfg(target_os = "macos")]
impl Pty {
    /// Create a new PTY using openpty (required on macOS for full functionality)
    pub fn new() -> Result<Self> {
        let result = openpty(None, None)?;
        let master_fd = result.master.as_raw_fd();
        let slave_fd = result.slave.as_raw_fd();
        let slave_path = unsafe {
            let name = libc::ttyname(slave_fd);
            if name.is_null() { return Err(Error::PtyCreation("Failed to get slave path".to_string())); }
            std::ffi::CStr::from_ptr(name).to_string_lossy().into_owned()
        };
        let file = unsafe { File::from_raw_fd(libc::dup(master_fd)) };
        let master_fd = unsafe { libc::dup(master_fd) };
        Ok(Self { master_fd, _slave_fd: result.slave, file, slave_path })
    }

    pub fn slave_path(&self) -> &str { &self.slave_path }
    pub fn master_fd(&self) -> RawFd { self.master_fd }

    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<()> {
        let fd = self.master_fd;
        let flags = fcntl(fd, FcntlArg::F_GETFL)?;
        let flags = OFlag::from_bits_truncate(flags);
        let new_flags = if nonblocking { flags | OFlag::O_NONBLOCK } else { flags & !OFlag::O_NONBLOCK };
        fcntl(fd, FcntlArg::F_SETFL(new_flags))?;
        Ok(())
    }

    pub fn set_window_size(&self, size: WindowSize) -> Result<()> {
        let ws = size.to_winsize();
        let fd = self.master_fd;
        let result = unsafe { libc::ioctl(fd, libc::TIOCSWINSZ as libc::c_ulong, &ws) };
        if result == -1 { Err(Error::WindowSize(io::Error::last_os_error().to_string())) } else { Ok(()) }
    }

    pub fn get_window_size(&self) -> Result<WindowSize> {
        let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
        let fd = self.master_fd;
        let result = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ as libc::c_ulong, &mut ws) };
        if result == -1 { Err(Error::WindowSize(io::Error::last_os_error().to_string())) } else { Ok(WindowSize::from(ws)) }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> { self.file.read(buf) }
    pub fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.file.write(buf) }
    pub fn write_all(&mut self, buf: &[u8]) -> io::Result<()> { self.file.write_all(buf) }
    pub fn flush(&mut self) -> io::Result<()> { self.file.flush() }

    pub fn try_read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.file.read(buf) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(e),
        }
    }
}

#[cfg(target_os = "macos")]
impl AsRawFd for Pty {
    fn as_raw_fd(&self) -> RawFd { self.master_fd }
}

#[cfg(target_os = "macos")]
impl AsFd for Pty {
    fn as_fd(&self) -> BorrowedFd<'_> { unsafe { BorrowedFd::borrow_raw(self.master_fd) } }
}

pub fn open_slave(path: &str) -> Result<OwnedFd> {
    use std::ffi::CString;
    let path_cstr = CString::new(path).map_err(|e| Error::PtyCreation(e.to_string()))?;
    let fd = unsafe { libc::open(path_cstr.as_ptr(), libc::O_RDWR | libc::O_NOCTTY) };
    if fd < 0 { return Err(Error::PtyCreation(io::Error::last_os_error().to_string())); }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

pub fn configure_slave(fd: RawFd) -> Result<()> {
    let borrowed_fd = unsafe { BorrowedFd::borrow_raw(fd) };
    let mut termios = termios::tcgetattr(borrowed_fd)?;
    termios.input_flags &= !(termios::InputFlags::IGNBRK
        | termios::InputFlags::BRKINT | termios::InputFlags::PARMRK
        | termios::InputFlags::ISTRIP | termios::InputFlags::INLCR
        | termios::InputFlags::IGNCR | termios::InputFlags::ICRNL
        | termios::InputFlags::IXON);
    termios.output_flags |= termios::OutputFlags::OPOST | termios::OutputFlags::ONLCR;
    termios.local_flags &= !(termios::LocalFlags::ECHO | termios::LocalFlags::ECHONL
        | termios::LocalFlags::ICANON | termios::LocalFlags::ISIG
        | termios::LocalFlags::IEXTEN);
    termios.control_flags &= !(termios::ControlFlags::CSIZE | termios::ControlFlags::PARENB);
    termios.control_flags |= termios::ControlFlags::CS8;
    termios.control_chars[termios::SpecialCharacterIndices::VMIN as usize] = 1;
    termios.control_chars[termios::SpecialCharacterIndices::VTIME as usize] = 0;
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
