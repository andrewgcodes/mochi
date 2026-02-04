//! PTY (Pseudo-Terminal) implementation for Linux.
//!
//! This module provides functionality to:
//! - Create PTY master/slave pairs using POSIX APIs
//! - Spawn child processes attached to PTYs
//! - Handle PTY I/O and resizing
//!
//! The implementation uses the modern POSIX PTY API:
//! - posix_openpt() to open the master
//! - grantpt() to set permissions
//! - unlockpt() to unlock the slave
//! - ptsname() to get the slave device path

use crate::error::PtyError;
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::libc;
use nix::pty::{grantpt, posix_openpt, ptsname, unlockpt, PtyMaster};
use nix::sys::signal::{self, Signal};
use nix::sys::termios::{self, SetArg, Termios};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{close, dup2, execvp, fork, read, setsid, write, ForkResult, Pid};
use std::ffi::CString;
use std::fs::File;
use std::io::{self, Read, Write as IoWrite};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl Default for PtySize {
    fn default() -> Self {
        PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

impl PtySize {
    pub fn new(cols: u16, rows: u16) -> Self {
        PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }
    }

    fn to_winsize(&self) -> libc::winsize {
        libc::winsize {
            ws_row: self.rows,
            ws_col: self.cols,
            ws_xpixel: self.pixel_width,
            ws_ypixel: self.pixel_height,
        }
    }
}

pub struct Pty {
    master: OwnedFd,
    slave_path: PathBuf,
    child_pid: Option<Pid>,
}

impl Pty {
    pub fn new() -> Result<Self, PtyError> {
        let master = posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY)
            .map_err(PtyError::OpenMaster)?;

        grantpt(&master).map_err(PtyError::GrantPty)?;
        unlockpt(&master).map_err(PtyError::UnlockPty)?;

        let slave_name = unsafe { ptsname(&master) }.map_err(PtyError::GetSlaveName)?;
        let slave_path = PathBuf::from(slave_name);

        let raw_fd = master.as_raw_fd();
        std::mem::forget(master);
        let owned_fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };

        Ok(Pty {
            master: owned_fd,
            slave_path,
            child_pid: None,
        })
    }

    pub fn spawn(&mut self, shell: Option<&str>, size: PtySize) -> Result<(), PtyError> {
        self.set_size(size)?;

        let shell = shell
            .map(String::from)
            .or_else(|| std::env::var("SHELL").ok())
            .unwrap_or_else(|| "/bin/bash".to_string());

        let slave_path = self.slave_path.clone();

        match unsafe { fork() }.map_err(PtyError::Fork)? {
            ForkResult::Parent { child } => {
                self.child_pid = Some(child);
                Ok(())
            }
            ForkResult::Child => {
                drop(unsafe { OwnedFd::from_raw_fd(self.master.as_raw_fd()) });

                setsid().map_err(PtyError::Setsid)?;

                let slave_fd = nix::fcntl::open(
                    slave_path.as_path(),
                    OFlag::O_RDWR,
                    nix::sys::stat::Mode::empty(),
                )
                .map_err(PtyError::OpenSlave)?;

                unsafe {
                    if libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) < 0 {
                        std::process::exit(1);
                    }
                }

                dup2(slave_fd, libc::STDIN_FILENO).unwrap();
                dup2(slave_fd, libc::STDOUT_FILENO).unwrap();
                dup2(slave_fd, libc::STDERR_FILENO).unwrap();

                if slave_fd > libc::STDERR_FILENO {
                    close(slave_fd).ok();
                }

                std::env::set_var("TERM", "xterm-256color");

                let shell_cstr = CString::new(shell.as_str()).unwrap();
                let args = [shell_cstr.clone()];

                execvp(&shell_cstr, &args).map_err(PtyError::Exec)?;

                unreachable!()
            }
        }
    }

    pub fn set_size(&self, size: PtySize) -> Result<(), PtyError> {
        let winsize = size.to_winsize();
        unsafe {
            if libc::ioctl(self.master.as_raw_fd(), libc::TIOCSWINSZ, &winsize) < 0 {
                return Err(PtyError::SetWindowSize(nix::Error::last()));
            }
        }
        Ok(())
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), PtyError> {
        let flags = fcntl(self.master.as_raw_fd(), FcntlArg::F_GETFL)
            .map_err(PtyError::SetNonBlocking)?;

        let flags = OFlag::from_bits_truncate(flags);
        let new_flags = if nonblocking {
            flags | OFlag::O_NONBLOCK
        } else {
            flags & !OFlag::O_NONBLOCK
        };

        fcntl(self.master.as_raw_fd(), FcntlArg::F_SETFL(new_flags))
            .map_err(PtyError::SetNonBlocking)?;

        Ok(())
    }

    pub fn master_fd(&self) -> RawFd {
        self.master.as_raw_fd()
    }

    pub fn child_pid(&self) -> Option<Pid> {
        self.child_pid
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        match read(self.master.as_raw_fd(), buf) {
            Ok(n) => Ok(n),
            Err(nix::Error::EAGAIN) | Err(nix::Error::EWOULDBLOCK) => {
                Err(io::Error::new(io::ErrorKind::WouldBlock, "would block"))
            }
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        match write(&self.master, buf) {
            Ok(n) => Ok(n),
            Err(nix::Error::EAGAIN) | Err(nix::Error::EWOULDBLOCK) => {
                Err(io::Error::new(io::ErrorKind::WouldBlock, "would block"))
            }
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }

    pub fn check_child(&self) -> Option<Result<(), PtyError>> {
        if let Some(pid) = self.child_pid {
            match waitpid(pid, Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::Exited(_, status)) => {
                    if status == 0 {
                        Some(Ok(()))
                    } else {
                        Some(Err(PtyError::ChildExited(status)))
                    }
                }
                Ok(WaitStatus::Signaled(_, signal, _)) => {
                    Some(Err(PtyError::ChildKilled(signal as i32)))
                }
                Ok(WaitStatus::StillAlive) => None,
                Ok(_) => None,
                Err(_) => None,
            }
        } else {
            None
        }
    }

    pub fn signal_child(&self, signal: Signal) -> Result<(), PtyError> {
        if let Some(pid) = self.child_pid {
            signal::kill(pid, signal).map_err(|e| PtyError::Io(io::Error::new(io::ErrorKind::Other, e)))?;
        }
        Ok(())
    }

    pub fn wait(&self) -> Result<i32, PtyError> {
        if let Some(pid) = self.child_pid {
            match waitpid(pid, None) {
                Ok(WaitStatus::Exited(_, status)) => Ok(status),
                Ok(WaitStatus::Signaled(_, signal, _)) => Err(PtyError::ChildKilled(signal as i32)),
                Ok(_) => Ok(0),
                Err(e) => Err(PtyError::Io(io::Error::new(io::ErrorKind::Other, e))),
            }
        } else {
            Ok(0)
        }
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        if let Some(pid) = self.child_pid {
            let _ = signal::kill(pid, Signal::SIGHUP);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_pty_creation() {
        let pty = Pty::new();
        assert!(pty.is_ok());
    }

    #[test]
    fn test_pty_size() {
        let size = PtySize::new(120, 40);
        assert_eq!(size.cols, 120);
        assert_eq!(size.rows, 40);
    }

    #[test]
    fn test_pty_spawn_and_read() {
        let mut pty = Pty::new().unwrap();
        pty.spawn(Some("/bin/sh"), PtySize::default()).unwrap();
        pty.set_nonblocking(true).unwrap();

        thread::sleep(Duration::from_millis(100));

        pty.write(b"echo hello\n").unwrap();

        thread::sleep(Duration::from_millis(200));

        let mut buf = [0u8; 1024];
        let mut output = Vec::new();
        
        for _ in 0..10 {
            match pty.read(&mut buf) {
                Ok(n) if n > 0 => {
                    output.extend_from_slice(&buf[..n]);
                }
                _ => break,
            }
            thread::sleep(Duration::from_millis(50));
        }

        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("hello") || output_str.contains("echo"));

        pty.write(b"exit\n").unwrap();
        thread::sleep(Duration::from_millis(100));
    }

    #[test]
    fn test_pty_resize() {
        let pty = Pty::new().unwrap();
        let result = pty.set_size(PtySize::new(100, 50));
        assert!(result.is_ok());
    }
}
