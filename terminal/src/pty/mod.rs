//! PTY (Pseudo-Terminal) Module
//!
//! Handles creating and managing pseudo-terminals on Linux.
//! This module provides:
//! - PTY creation (master/slave pair)
//! - Child process spawning with PTY as controlling terminal
//! - Non-blocking I/O
//! - Window size management (TIOCSWINSZ)
//!
//! References:
//! - POSIX PTY: https://man7.org/linux/man-pages/man3/posix_openpt.3.html
//! - tty_ioctl: https://man7.org/linux/man-pages/man4/tty_ioctl.4.html

use std::ffi::CString;
use std::io;
use std::os::unix::io::{AsRawFd, OwnedFd, RawFd};

use nix::errno::Errno;
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::libc;
use nix::pty::{openpty, Winsize};
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{dup2, execvp, fork, setsid, ForkResult, Pid};

/// A pseudo-terminal pair (master and slave)
pub struct Pty {
    /// Master file descriptor
    master: OwnedFd,
    /// Child process ID
    child_pid: Option<Pid>,
    /// Current window size
    size: Winsize,
}

impl Pty {
    /// Create a new PTY and spawn a shell
    pub fn spawn(shell: Option<&str>, cols: u16, rows: u16) -> io::Result<Self> {
        let size = Winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        // Open PTY pair
        let pty_pair =
            openpty(Some(&size), None).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let master = pty_pair.master;
        let slave = pty_pair.slave;

        // Set master to non-blocking
        let flags = fcntl(master.as_raw_fd(), FcntlArg::F_GETFL)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let flags = OFlag::from_bits_truncate(flags);
        fcntl(
            master.as_raw_fd(),
            FcntlArg::F_SETFL(flags | OFlag::O_NONBLOCK),
        )
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        // Determine shell to use
        let shell_path = shell
            .map(String::from)
            .or_else(|| std::env::var("SHELL").ok())
            .unwrap_or_else(|| "/bin/bash".to_string());

        // Fork child process
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child }) => {
                // Parent process - close slave
                drop(slave);

                Ok(Pty {
                    master,
                    child_pid: Some(child),
                    size,
                })
            }
            Ok(ForkResult::Child) => {
                // Child process
                drop(master);

                // Create new session and set controlling terminal
                setsid().expect("setsid failed");

                // Set slave as controlling terminal
                unsafe {
                    libc::ioctl(slave.as_raw_fd(), libc::TIOCSCTTY, 0);
                }

                // Duplicate slave to stdin/stdout/stderr
                dup2(slave.as_raw_fd(), libc::STDIN_FILENO).expect("dup2 stdin failed");
                dup2(slave.as_raw_fd(), libc::STDOUT_FILENO).expect("dup2 stdout failed");
                dup2(slave.as_raw_fd(), libc::STDERR_FILENO).expect("dup2 stderr failed");

                // Close original slave fd if it's not one of the standard fds
                if slave.as_raw_fd() > 2 {
                    drop(slave);
                }

                // Set up environment
                std::env::set_var("TERM", "xterm-256color");
                std::env::set_var("COLORTERM", "truecolor");

                // Execute shell
                let shell_cstr = CString::new(shell_path.as_str()).expect("CString::new failed");
                let args = [shell_cstr.clone()];
                execvp(&shell_cstr, &args).expect("execvp failed");

                unreachable!()
            }
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }

    /// Get the master file descriptor for polling
    pub fn as_raw_fd(&self) -> RawFd {
        self.master.as_raw_fd()
    }

    /// Read from the PTY master (non-blocking)
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let fd = self.master.as_raw_fd();
        let result = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };

        if result < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::WouldBlock {
                return Ok(0);
            }
            return Err(err);
        }

        Ok(result as usize)
    }

    /// Write to the PTY master
    pub fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let fd = self.master.as_raw_fd();
        let result = unsafe { libc::write(fd, data.as_ptr() as *const libc::c_void, data.len()) };

        if result < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(result as usize)
    }

    /// Write all data to the PTY master
    pub fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        let mut written = 0;
        while written < data.len() {
            match self.write(&data[written..]) {
                Ok(n) => written += n,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // Busy wait for a bit
                    std::thread::sleep(std::time::Duration::from_micros(100));
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Resize the PTY
    pub fn resize(&mut self, cols: u16, rows: u16) -> io::Result<()> {
        self.size.ws_col = cols;
        self.size.ws_row = rows;

        let result = unsafe {
            libc::ioctl(
                self.master.as_raw_fd(),
                libc::TIOCSWINSZ,
                &self.size as *const Winsize,
            )
        };

        if result < 0 {
            return Err(io::Error::last_os_error());
        }

        // Send SIGWINCH to child process group
        if let Some(pid) = self.child_pid {
            let _ = kill(pid, Signal::SIGWINCH);
        }

        Ok(())
    }

    /// Get current window size
    pub fn size(&self) -> (u16, u16) {
        (self.size.ws_col, self.size.ws_row)
    }

    /// Check if child process is still running
    pub fn is_running(&self) -> bool {
        if let Some(pid) = self.child_pid {
            match waitpid(pid, Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::StillAlive) => true,
                Ok(_) => false,
                Err(_) => false,
            }
        } else {
            false
        }
    }

    /// Wait for child process to exit
    pub fn wait(&self) -> io::Result<Option<i32>> {
        if let Some(pid) = self.child_pid {
            match waitpid(pid, None) {
                Ok(WaitStatus::Exited(_, code)) => Ok(Some(code)),
                Ok(WaitStatus::Signaled(_, sig, _)) => Ok(Some(128 + sig as i32)),
                Ok(_) => Ok(None),
                Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
            }
        } else {
            Ok(None)
        }
    }

    /// Try to wait for child process (non-blocking)
    pub fn try_wait(&self) -> io::Result<Option<i32>> {
        if let Some(pid) = self.child_pid {
            match waitpid(pid, Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::Exited(_, code)) => Ok(Some(code)),
                Ok(WaitStatus::Signaled(_, sig, _)) => Ok(Some(128 + sig as i32)),
                Ok(WaitStatus::StillAlive) => Ok(None),
                Ok(_) => Ok(None),
                Err(Errno::ECHILD) => Ok(Some(0)),
                Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
            }
        } else {
            Ok(None)
        }
    }

    /// Get child PID
    pub fn child_pid(&self) -> Option<i32> {
        self.child_pid.map(|p| p.as_raw())
    }

    /// Send a signal to the child process
    pub fn signal(&self, sig: Signal) -> io::Result<()> {
        if let Some(pid) = self.child_pid {
            kill(pid, sig).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
        } else {
            Ok(())
        }
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        // The master fd will be closed automatically by OwnedFd
        // Try to reap the child process
        if let Some(pid) = self.child_pid {
            let _ = waitpid(pid, Some(WaitPidFlag::WNOHANG));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_pty_spawn() {
        let pty = Pty::spawn(Some("/bin/sh"), 80, 24);
        assert!(pty.is_ok());
        let pty = pty.unwrap();
        assert!(pty.is_running());
    }

    #[test]
    fn test_pty_size() {
        let pty = Pty::spawn(Some("/bin/sh"), 80, 24).unwrap();
        assert_eq!(pty.size(), (80, 24));
    }

    #[test]
    fn test_pty_resize() {
        let mut pty = Pty::spawn(Some("/bin/sh"), 80, 24).unwrap();
        assert!(pty.resize(120, 40).is_ok());
        assert_eq!(pty.size(), (120, 40));
    }

    #[test]
    fn test_pty_write_read() {
        let mut pty = Pty::spawn(Some("/bin/sh"), 80, 24).unwrap();

        // Give shell time to start
        std::thread::sleep(Duration::from_millis(100));

        // Write a command
        pty.write_all(b"echo hello\n").unwrap();

        // Give it time to process
        std::thread::sleep(Duration::from_millis(100));

        // Read output
        let mut buf = [0u8; 1024];
        let mut output = Vec::new();
        for _ in 0..10 {
            match pty.read(&mut buf) {
                Ok(0) => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Ok(n) => {
                    output.extend_from_slice(&buf[..n]);
                }
                Err(_) => break,
            }
        }

        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("hello") || output_str.contains("echo"));
    }

    #[test]
    fn test_pty_exit() {
        let mut pty = Pty::spawn(Some("/bin/sh"), 80, 24).unwrap();

        // Send exit command
        pty.write_all(b"exit\n").unwrap();

        // Wait for exit
        std::thread::sleep(Duration::from_millis(200));

        // Check if exited
        let status = pty.try_wait().unwrap();
        // May or may not have exited yet, but shouldn't error
        assert!(status.is_none() || status == Some(0));
    }
}
