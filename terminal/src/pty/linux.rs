//! Linux PTY Implementation
//!
//! Uses POSIX PTY functions to create and manage pseudo-terminals.
//!
//! # References
//!
//! - posix_openpt(3): https://man7.org/linux/man-pages/man3/posix_openpt.3.html
//! - grantpt(3), unlockpt(3), ptsname(3)
//! - tty_ioctl(4): https://man7.org/linux/man-pages/man4/tty_ioctl.4.html

use std::ffi::CString;
use std::io;
use std::os::unix::io::{AsRawFd, BorrowedFd, RawFd};
use std::process;

use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::libc;
use nix::poll::{poll, PollFd, PollFlags};
use nix::pty::{grantpt, posix_openpt, ptsname, unlockpt, PtyMaster};
use nix::sys::signal::{self, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{close, dup2, execvp, fork, setsid, ForkResult, Pid};

use thiserror::Error;

/// PTY-related errors
#[derive(Error, Debug)]
pub enum PtyError {
    #[error("Failed to open PTY master: {0}")]
    OpenMaster(#[source] nix::Error),

    #[error("Failed to grant PTY: {0}")]
    Grant(#[source] nix::Error),

    #[error("Failed to unlock PTY: {0}")]
    Unlock(#[source] nix::Error),

    #[error("Failed to get slave name: {0}")]
    SlaveName(#[source] nix::Error),

    #[error("Failed to open slave: {0}")]
    OpenSlave(#[source] io::Error),

    #[error("Failed to fork: {0}")]
    Fork(#[source] nix::Error),

    #[error("Failed to execute shell: {0}")]
    Exec(#[source] nix::Error),

    #[error("Failed to set terminal attributes: {0}")]
    Termios(#[source] nix::Error),

    #[error("Failed to set window size: {0}")]
    WindowSize(#[source] nix::Error),

    #[error("IO error: {0}")]
    Io(#[source] io::Error),

    #[error("Child process exited")]
    ChildExited,
}

/// Window size for the PTY
#[derive(Debug, Clone, Copy)]
pub struct WindowSize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl Default for WindowSize {
    fn default() -> Self {
        Self {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

impl From<WindowSize> for libc::winsize {
    fn from(ws: WindowSize) -> Self {
        libc::winsize {
            ws_row: ws.rows,
            ws_col: ws.cols,
            ws_xpixel: ws.pixel_width,
            ws_ypixel: ws.pixel_height,
        }
    }
}

/// A pseudo-terminal with a child process
pub struct Pty {
    /// The PTY master file descriptor
    master: PtyMaster,
    /// Child process ID
    child_pid: Pid,
    /// Current window size
    window_size: WindowSize,
}

impl Pty {
    /// Create a new PTY and spawn a shell
    ///
    /// # Arguments
    ///
    /// * `shell` - Path to the shell to execute (e.g., "/bin/bash")
    /// * `size` - Initial window size
    /// * `env` - Additional environment variables
    pub fn spawn(
        shell: Option<&str>,
        size: WindowSize,
        env: &[(&str, &str)],
    ) -> Result<Self, PtyError> {
        // Open PTY master
        let master = posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY).map_err(PtyError::OpenMaster)?;

        // Grant and unlock the slave
        grantpt(&master).map_err(PtyError::Grant)?;
        unlockpt(&master).map_err(PtyError::Unlock)?;

        // Get slave name
        let slave_name = unsafe { ptsname(&master) }.map_err(PtyError::SlaveName)?;

        // Set master to non-blocking
        let master_fd = master.as_raw_fd();
        fcntl(master_fd, FcntlArg::F_SETFL(OFlag::O_NONBLOCK))
            .map_err(|e| PtyError::Io(io::Error::other(e)))?;

        // Fork the child process
        let child_pid = match unsafe { fork() }.map_err(PtyError::Fork)? {
            ForkResult::Parent { child } => child,
            ForkResult::Child => {
                // Child process
                Self::setup_child(&slave_name, shell, env, size);
                // setup_child should not return, but if it does, exit
                process::exit(1);
            }
        };

        // Set initial window size on master
        let ws: libc::winsize = size.into();
        unsafe {
            if libc::ioctl(master_fd, libc::TIOCSWINSZ, &ws) < 0 {
                // Non-fatal, just log
                log::warn!("Failed to set initial window size");
            }
        }

        Ok(Self {
            master,
            child_pid,
            window_size: size,
        })
    }

    /// Setup the child process (runs in forked child)
    fn setup_child(slave_name: &str, shell: Option<&str>, env: &[(&str, &str)], size: WindowSize) {
        // Create a new session
        if setsid().is_err() {
            process::exit(1);
        }

        // Open the slave PTY
        let slave_fd = match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(slave_name)
        {
            Ok(f) => f.as_raw_fd(),
            Err(_) => process::exit(1),
        };

        // Set the slave as the controlling terminal
        unsafe {
            if libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) < 0 {
                // Some systems don't require this, continue anyway
            }
        }

        // Set window size on slave
        let ws: libc::winsize = size.into();
        unsafe {
            libc::ioctl(slave_fd, libc::TIOCSWINSZ, &ws);
        }

        // Duplicate slave to stdin, stdout, stderr
        let _ = dup2(slave_fd, libc::STDIN_FILENO);
        let _ = dup2(slave_fd, libc::STDOUT_FILENO);
        let _ = dup2(slave_fd, libc::STDERR_FILENO);

        // Close the original slave fd if it's not one of the standard fds
        if slave_fd > libc::STDERR_FILENO {
            let _ = close(slave_fd);
        }

        // Set up environment
        for (key, value) in env {
            std::env::set_var(key, value);
        }

        // Set TERM if not already set
        if std::env::var("TERM").is_err() {
            std::env::set_var("TERM", "xterm-256color");
        }

        // Determine shell to use
        let shell_path = shell
            .map(String::from)
            .or_else(|| std::env::var("SHELL").ok())
            .unwrap_or_else(|| "/bin/bash".to_string());

        // Execute the shell
        let shell_cstr = CString::new(shell_path.as_str()).unwrap();
        let shell_name = shell_path.rsplit('/').next().unwrap_or("sh");
        let arg0 = CString::new(format!("-{}", shell_name)).unwrap(); // Login shell

        let args = [arg0.as_c_str()];

        // This should not return
        let _ = execvp(&shell_cstr, &args);

        // If exec failed, try /bin/sh
        let sh = CString::new("/bin/sh").unwrap();
        let sh_arg = CString::new("-sh").unwrap();
        let _ = execvp(&sh, &[sh_arg.as_c_str()]);

        process::exit(1);
    }

    /// Read data from the PTY master
    ///
    /// Returns the number of bytes read, or 0 if no data available (non-blocking)
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, PtyError> {
        let fd = self.master.as_raw_fd();
        let result = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };

        if result < 0 {
            let err = io::Error::last_os_error();
            if err.kind() == io::ErrorKind::WouldBlock {
                Ok(0)
            } else {
                Err(PtyError::Io(err))
            }
        } else {
            Ok(result as usize)
        }
    }

    /// Write data to the PTY master
    pub fn write(&mut self, data: &[u8]) -> Result<usize, PtyError> {
        // For writing, we might need to handle WouldBlock differently
        // For now, use blocking write by temporarily removing O_NONBLOCK
        let fd = self.master.as_raw_fd();

        // Get current flags
        let flags = fcntl(fd, FcntlArg::F_GETFL)
            .map_err(|e| PtyError::Io(io::Error::other(e)))?;

        // Remove O_NONBLOCK temporarily
        let flags_without_nonblock = OFlag::from_bits_truncate(flags) & !OFlag::O_NONBLOCK;
        fcntl(fd, FcntlArg::F_SETFL(flags_without_nonblock))
            .map_err(|e| PtyError::Io(io::Error::other(e)))?;

        let result = unsafe { libc::write(fd, data.as_ptr() as *const libc::c_void, data.len()) };

        // Restore O_NONBLOCK
        let _ = fcntl(fd, FcntlArg::F_SETFL(OFlag::from_bits_truncate(flags)));

        if result < 0 {
            Err(PtyError::Io(io::Error::last_os_error()))
        } else {
            Ok(result as usize)
        }
    }

    /// Write all data to the PTY master
    pub fn write_all(&mut self, data: &[u8]) -> Result<(), PtyError> {
        let mut written = 0;
        while written < data.len() {
            written += self.write(&data[written..])?;
        }
        Ok(())
    }

    /// Poll for readable data with timeout
    ///
    /// Returns true if data is available to read
    pub fn poll_read(&self, timeout_ms: i32) -> Result<bool, PtyError> {
        let fd = self.master.as_raw_fd();
        // SAFETY: We know the fd is valid because self.master is valid
        let borrowed_fd = unsafe { BorrowedFd::borrow_raw(fd) };
        let mut poll_fds = [PollFd::new(&borrowed_fd, PollFlags::POLLIN)];

        match poll(&mut poll_fds, timeout_ms) {
            Ok(n) if n > 0 => {
                let revents = poll_fds[0].revents().unwrap_or(PollFlags::empty());
                if revents.contains(PollFlags::POLLIN) {
                    Ok(true)
                } else if revents.contains(PollFlags::POLLHUP) {
                    Err(PtyError::ChildExited)
                } else {
                    Ok(false)
                }
            }
            Ok(_) => Ok(false),                         // Timeout
            Err(nix::errno::Errno::EINTR) => Ok(false), // Interrupted
            Err(e) => Err(PtyError::Io(io::Error::other(e))),
        }
    }

    /// Resize the PTY
    pub fn resize(&mut self, size: WindowSize) -> Result<(), PtyError> {
        let ws: libc::winsize = size.into();
        let fd = self.master.as_raw_fd();

        unsafe {
            if libc::ioctl(fd, libc::TIOCSWINSZ, &ws) < 0 {
                return Err(PtyError::WindowSize(nix::Error::last()));
            }
        }

        self.window_size = size;

        // Send SIGWINCH to the child process group
        let _ = signal::kill(self.child_pid, Signal::SIGWINCH);

        Ok(())
    }

    /// Get the current window size
    pub fn window_size(&self) -> WindowSize {
        self.window_size
    }

    /// Get the child process ID
    pub fn child_pid(&self) -> Pid {
        self.child_pid
    }

    /// Check if the child process is still running
    pub fn is_child_alive(&self) -> bool {
        match waitpid(self.child_pid, Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => true,
            Ok(_) => false,  // Child exited
            Err(_) => false, // Error checking, assume dead
        }
    }

    /// Wait for the child process to exit
    pub fn wait(&self) -> Result<i32, PtyError> {
        match waitpid(self.child_pid, None) {
            Ok(WaitStatus::Exited(_, code)) => Ok(code),
            Ok(WaitStatus::Signaled(_, sig, _)) => Ok(128 + sig as i32),
            Ok(_) => Ok(0),
            Err(e) => Err(PtyError::Io(io::Error::other(e))),
        }
    }

    /// Get the raw file descriptor for the master
    pub fn master_fd(&self) -> RawFd {
        self.master.as_raw_fd()
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        // Try to kill the child process gracefully
        let _ = signal::kill(self.child_pid, Signal::SIGHUP);

        // Wait briefly for it to exit
        for _ in 0..10 {
            if !self.is_child_alive() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Force kill if still alive
        if self.is_child_alive() {
            let _ = signal::kill(self.child_pid, Signal::SIGKILL);
            let _ = waitpid(self.child_pid, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_size_default() {
        let ws = WindowSize::default();
        assert_eq!(ws.rows, 24);
        assert_eq!(ws.cols, 80);
    }

    #[test]
    fn test_window_size_to_libc() {
        let ws = WindowSize {
            rows: 30,
            cols: 100,
            pixel_width: 800,
            pixel_height: 600,
        };
        let libc_ws: libc::winsize = ws.into();
        assert_eq!(libc_ws.ws_row, 30);
        assert_eq!(libc_ws.ws_col, 100);
        assert_eq!(libc_ws.ws_xpixel, 800);
        assert_eq!(libc_ws.ws_ypixel, 600);
    }

    #[test]
    fn test_pty_spawn_and_basic_io() {
        // Skip in CI if no PTY available
        if std::env::var("CI").is_ok() {
            return;
        }

        let size = WindowSize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        };

        let mut pty = Pty::spawn(Some("/bin/sh"), size, &[]).expect("Failed to spawn PTY");

        // Give the shell time to start
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Write a simple command
        pty.write_all(b"echo hello\n").expect("Failed to write");

        // Wait for output
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Read output
        let mut buf = [0u8; 1024];
        let n = pty.read(&mut buf).expect("Failed to read");

        // Should have received something
        assert!(n > 0 || pty.poll_read(100).unwrap_or(false));

        // Child should still be alive
        assert!(pty.is_child_alive());

        // Exit the shell
        pty.write_all(b"exit\n").expect("Failed to write exit");
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_pty_resize() {
        if std::env::var("CI").is_ok() {
            return;
        }

        let size = WindowSize::default();
        let mut pty = Pty::spawn(Some("/bin/sh"), size, &[]).expect("Failed to spawn PTY");

        let new_size = WindowSize {
            rows: 40,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        };

        pty.resize(new_size).expect("Failed to resize");
        assert_eq!(pty.window_size().rows, 40);
        assert_eq!(pty.window_size().cols, 120);

        // Clean up
        pty.write_all(b"exit\n").ok();
    }
}
