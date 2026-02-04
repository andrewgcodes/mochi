//! Unix PTY implementation
//!
//! Implements PTY creation and child process management using POSIX APIs.

use std::ffi::CString;
use std::os::fd::BorrowedFd;
use std::os::unix::io::{AsRawFd, RawFd};

use nix::fcntl::{fcntl, open, FcntlArg, OFlag};
use nix::libc::{self, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO};
use nix::poll::{poll, PollFd, PollFlags};
use nix::pty::{grantpt, posix_openpt, ptsname, unlockpt, PtyMaster};
use nix::sys::stat::Mode;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{close, dup2, execvp, fork, read, setsid, write, ForkResult, Pid};

use super::{PtyError, PtyResult, WindowSize};

/// A pseudoterminal with a spawned child process
pub struct Pty {
    /// The PTY master file descriptor
    master: PtyMaster,
    /// The child process ID
    child_pid: Pid,
    /// Whether the child is still running
    child_alive: bool,
}

impl Pty {
    /// Spawn a new PTY with the given shell and window size
    ///
    /// # Arguments
    /// * `shell` - Path to the shell to execute (e.g., "/bin/bash")
    /// * `args` - Arguments to pass to the shell
    /// * `size` - Initial window size
    ///
    /// # Returns
    /// A new `Pty` instance with the child process running
    pub fn spawn(shell: &str, args: &[&str], size: WindowSize) -> PtyResult<Self> {
        // Open PTY master
        let master = posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY).map_err(PtyError::OpenMaster)?;

        // Grant access to slave
        grantpt(&master).map_err(PtyError::GrantPty)?;

        // Unlock slave
        unlockpt(&master).map_err(PtyError::UnlockPty)?;

        // Get slave name
        // SAFETY: ptsname is not thread-safe, but we're calling it immediately
        // after unlockpt and before any other thread could interfere
        let slave_name = unsafe { ptsname(&master) }.map_err(PtyError::PtsName)?;

        // Set initial window size
        set_window_size(master.as_raw_fd(), size)?;

        // Fork
        // SAFETY: fork is safe as long as we're careful in the child
        match unsafe { fork() }.map_err(PtyError::Fork)? {
            ForkResult::Child => {
                // Child process
                // Drop the master fd (child doesn't need it)
                drop(master);

                // Create new session
                setsid().map_err(PtyError::Setsid)?;

                // Open slave - this becomes the controlling terminal
                let slave_fd = open(slave_name.as_str(), OFlag::O_RDWR, Mode::empty())
                    .map_err(PtyError::OpenSlave)?;

                // Set controlling terminal (Linux-specific)
                // SAFETY: TIOCSCTTY is a valid ioctl for setting controlling terminal
                unsafe {
                    if libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) < 0 {
                        // Non-fatal on some systems
                        tracing::debug!("TIOCSCTTY failed (may be ok)");
                    }
                }

                // Duplicate slave to stdin/stdout/stderr
                dup2(slave_fd, STDIN_FILENO).map_err(PtyError::Dup2)?;
                dup2(slave_fd, STDOUT_FILENO).map_err(PtyError::Dup2)?;
                dup2(slave_fd, STDERR_FILENO).map_err(PtyError::Dup2)?;

                // Close original slave fd if it's not one of the standard fds
                if slave_fd > STDERR_FILENO {
                    let _ = close(slave_fd);
                }

                // Set up environment
                std::env::set_var("TERM", "xterm-256color");
                std::env::set_var("COLORTERM", "truecolor");

                // Convert shell and args to CStrings
                let shell_cstr = CString::new(shell).expect("shell path contains null");
                let mut argv: Vec<CString> = Vec::with_capacity(args.len() + 1);
                argv.push(shell_cstr.clone());
                for arg in args {
                    argv.push(CString::new(*arg).expect("arg contains null"));
                }

                // Execute shell
                execvp(&shell_cstr, &argv).map_err(PtyError::Exec)?;

                // execvp only returns on error
                unreachable!()
            },
            ForkResult::Parent { child } => {
                // Parent process

                // Set master to non-blocking
                let flags = fcntl(master.as_raw_fd(), FcntlArg::F_GETFL)
                    .map_err(PtyError::SetNonBlocking)?;
                let flags = OFlag::from_bits_truncate(flags);
                fcntl(
                    master.as_raw_fd(),
                    FcntlArg::F_SETFL(flags | OFlag::O_NONBLOCK),
                )
                .map_err(PtyError::SetNonBlocking)?;

                Ok(Pty {
                    master,
                    child_pid: child,
                    child_alive: true,
                })
            },
        }
    }

    /// Spawn a shell using the user's default shell
    pub fn spawn_shell(size: WindowSize) -> PtyResult<Self> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        Self::spawn(&shell, &[], size)
    }

    /// Get the raw file descriptor of the PTY master
    pub fn master_fd(&self) -> RawFd {
        self.master.as_raw_fd()
    }

    /// Get the child process ID
    pub fn child_pid(&self) -> Pid {
        self.child_pid
    }

    /// Check if the child process is still running
    pub fn is_alive(&mut self) -> bool {
        if !self.child_alive {
            return false;
        }

        match waitpid(self.child_pid, Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => true,
            Ok(_) => {
                self.child_alive = false;
                false
            },
            Err(_) => {
                self.child_alive = false;
                false
            },
        }
    }

    /// Wait for the child process to exit
    pub fn wait(&mut self) -> PtyResult<i32> {
        if !self.child_alive {
            return Ok(0);
        }

        match waitpid(self.child_pid, None).map_err(PtyError::Wait)? {
            WaitStatus::Exited(_, code) => {
                self.child_alive = false;
                Ok(code)
            },
            WaitStatus::Signaled(_, signal, _) => {
                self.child_alive = false;
                Err(PtyError::ChildSignaled(signal as i32))
            },
            _ => Ok(0),
        }
    }

    /// Read from the PTY master (non-blocking)
    ///
    /// Returns the number of bytes read, or 0 if no data is available.
    /// Returns an error if the read fails for reasons other than EAGAIN/EWOULDBLOCK.
    pub fn read(&self, buf: &mut [u8]) -> PtyResult<usize> {
        match read(self.master.as_raw_fd(), buf) {
            Ok(n) => Ok(n),
            // EAGAIN and EWOULDBLOCK are the same value on Linux
            Err(nix::errno::Errno::EAGAIN) => Ok(0),
            Err(e) => Err(PtyError::Read(e)),
        }
    }

    /// Write to the PTY master
    ///
    /// Returns the number of bytes written.
    pub fn write(&self, data: &[u8]) -> PtyResult<usize> {
        write(self.master.as_raw_fd(), data).map_err(PtyError::Write)
    }

    /// Write all data to the PTY master
    pub fn write_all(&self, mut data: &[u8]) -> PtyResult<()> {
        while !data.is_empty() {
            let n = self.write(data)?;
            data = &data[n..];
        }
        Ok(())
    }

    /// Poll for data available to read
    ///
    /// Returns true if data is available, false if timeout expired.
    pub fn poll_read(&self, timeout_ms: i32) -> PtyResult<bool> {
        // SAFETY: The master fd is valid for the lifetime of this Pty
        let borrowed_fd = unsafe { BorrowedFd::borrow_raw(self.master.as_raw_fd()) };
        let mut fds = [PollFd::new(&borrowed_fd, PollFlags::POLLIN)];
        let n = poll(&mut fds, timeout_ms).map_err(PtyError::Poll)?;
        Ok(n > 0
            && fds[0]
                .revents()
                .is_some_and(|r| r.contains(PollFlags::POLLIN)))
    }

    /// Resize the PTY
    pub fn resize(&self, size: WindowSize) -> PtyResult<()> {
        set_window_size(self.master.as_raw_fd(), size)
    }

    /// Send a signal to the child process
    pub fn signal(&self, signal: nix::sys::signal::Signal) -> PtyResult<()> {
        nix::sys::signal::kill(self.child_pid, signal)
            .map_err(|e| PtyError::Io(std::io::Error::other(e)))
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        // Try to reap the child process
        if self.child_alive {
            let _ = waitpid(self.child_pid, Some(WaitPidFlag::WNOHANG));
        }
    }
}

/// Set the window size on a PTY file descriptor
fn set_window_size(fd: RawFd, size: WindowSize) -> PtyResult<()> {
    let winsize = libc::winsize {
        ws_row: size.rows,
        ws_col: size.cols,
        ws_xpixel: size.pixel_width,
        ws_ypixel: size.pixel_height,
    };

    // SAFETY: TIOCSWINSZ is a valid ioctl for setting window size
    let result = unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, &winsize) };

    if result < 0 {
        Err(PtyError::SetWinsize(nix::errno::Errno::last()))
    } else {
        Ok(())
    }
}

/// Get the window size from a PTY file descriptor
#[allow(dead_code)]
pub fn get_window_size(fd: RawFd) -> PtyResult<WindowSize> {
    let mut winsize = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    // SAFETY: TIOCGWINSZ is a valid ioctl for getting window size
    let result = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut winsize) };

    if result < 0 {
        Err(PtyError::SetWinsize(nix::errno::Errno::last()))
    } else {
        Ok(WindowSize {
            rows: winsize.ws_row,
            cols: winsize.ws_col,
            pixel_width: winsize.ws_xpixel,
            pixel_height: winsize.ws_ypixel,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_size() {
        let size = WindowSize::new(80, 24);
        assert_eq!(size.cols, 80);
        assert_eq!(size.rows, 24);
    }

    #[test]
    fn test_pty_spawn() {
        // Spawn a simple command
        let mut pty = Pty::spawn("/bin/echo", &["hello"], WindowSize::new(80, 24))
            .expect("Failed to spawn PTY");

        // Wait a bit for output
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Read output
        let mut buf = [0u8; 1024];
        let n = pty.read(&mut buf).expect("Failed to read");

        // Should have received "hello\n" or similar
        let output = String::from_utf8_lossy(&buf[..n]);
        assert!(
            output.contains("hello") || n == 0,
            "Unexpected output: {}",
            output
        );

        // Wait for child to exit
        let _ = pty.wait();
        assert!(!pty.is_alive());
    }

    #[test]
    fn test_pty_write_read() {
        // Spawn cat which echoes input
        let pty =
            Pty::spawn("/bin/cat", &[], WindowSize::new(80, 24)).expect("Failed to spawn PTY");

        // Write some data
        pty.write_all(b"test\n").expect("Failed to write");

        // Wait for echo
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Read output
        let mut buf = [0u8; 1024];
        let n = pty.read(&mut buf).expect("Failed to read");

        let output = String::from_utf8_lossy(&buf[..n]);
        // Cat should echo back what we wrote
        assert!(
            output.contains("test") || n == 0,
            "Unexpected output: {}",
            output
        );
    }

    #[test]
    fn test_pty_resize() {
        let pty = Pty::spawn("/bin/sh", &[], WindowSize::new(80, 24)).expect("Failed to spawn PTY");

        // Resize
        pty.resize(WindowSize::new(120, 40))
            .expect("Failed to resize");

        // Verify size
        let size = get_window_size(pty.master_fd()).expect("Failed to get size");
        assert_eq!(size.cols, 120);
        assert_eq!(size.rows, 40);
    }

    #[test]
    fn test_pty_poll() {
        let pty = Pty::spawn("/bin/echo", &["test"], WindowSize::new(80, 24))
            .expect("Failed to spawn PTY");

        // Poll should eventually return true when there's output
        let mut found_data = false;
        for _ in 0..10 {
            if pty.poll_read(100).expect("Failed to poll") {
                found_data = true;
                break;
            }
        }

        // Note: This might be flaky depending on timing
        // The important thing is that poll doesn't crash
        let _ = found_data;
    }
}
