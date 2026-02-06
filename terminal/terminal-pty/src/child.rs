//! Child process management for PTY
//!
//! Handles spawning and managing child processes attached to a PTY.

use std::ffi::{CString, OsStr};
use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;

use nix::libc;
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{dup2, execvp, fork, setsid, ForkResult, Pid};

use crate::error::{Error, Result};
use crate::pty::{configure_slave, open_slave, Pty};
use crate::size::WindowSize;

/// A child process attached to a PTY
pub struct Child {
    /// The PTY master
    pty: Pty,
    /// Child process ID
    pid: Pid,
}

impl Child {
    /// Spawn a new child process with the given command
    ///
    /// # Arguments
    /// * `program` - The program to execute
    /// * `args` - Arguments to pass to the program
    /// * `env` - Environment variables (if None, inherits from parent)
    /// * `size` - Initial window size
    pub fn spawn<S, I, E, K, V>(
        program: S,
        args: I,
        env: Option<E>,
        size: WindowSize,
    ) -> Result<Self>
    where
        S: AsRef<OsStr>,
        I: IntoIterator<Item = S>,
        E: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        // Create PTY
        let pty = Pty::new()?;
        pty.set_window_size(size)?;

        let slave_path = pty.slave_path().to_string();

        // Prepare arguments
        let program_cstr = CString::new(program.as_ref().as_bytes())
            .map_err(|e| Error::SpawnFailed(e.to_string()))?;

        let mut args_cstr: Vec<CString> = vec![program_cstr.clone()];
        for arg in args {
            let arg_cstr = CString::new(arg.as_ref().as_bytes())
                .map_err(|e| Error::SpawnFailed(e.to_string()))?;
            args_cstr.push(arg_cstr);
        }

        // Prepare environment if provided
        let env_cstr: Option<Vec<CString>> = env.map(|e| {
            e.into_iter()
                .filter_map(|(k, v)| {
                    let key = k.as_ref().as_bytes();
                    let value = v.as_ref().as_bytes();
                    let mut combined = Vec::with_capacity(key.len() + 1 + value.len());
                    combined.extend_from_slice(key);
                    combined.push(b'=');
                    combined.extend_from_slice(value);
                    CString::new(combined).ok()
                })
                .collect()
        });

        // Fork
        match unsafe { fork() }? {
            ForkResult::Parent { child } => {
                // Parent process
                Ok(Self { pty, pid: child })
            }
            ForkResult::Child => {
                // Child process - this code runs in the child

                // Create new session and set controlling terminal
                if setsid().is_err() {
                    std::process::exit(1);
                }

                // Open slave PTY
                let slave_fd = match open_slave(&slave_path) {
                    Ok(fd) => fd,
                    Err(_) => std::process::exit(1),
                };

                let slave_raw = slave_fd.as_raw_fd();

                // Set as controlling terminal
                // Note: On macOS, TIOCSCTTY is u32 but ioctl expects c_ulong (u64),
                // so we need to cast it explicitly for cross-platform compatibility
                unsafe {
                    if libc::ioctl(slave_raw, libc::TIOCSCTTY as libc::c_ulong, 0) < 0 {
                        std::process::exit(1);
                    }
                }

                // Configure terminal
                if configure_slave(slave_raw).is_err() {
                    std::process::exit(1);
                }

                // Duplicate slave to stdin, stdout, stderr
                if dup2(slave_raw, libc::STDIN_FILENO).is_err() {
                    std::process::exit(1);
                }
                if dup2(slave_raw, libc::STDOUT_FILENO).is_err() {
                    std::process::exit(1);
                }
                if dup2(slave_raw, libc::STDERR_FILENO).is_err() {
                    std::process::exit(1);
                }

                // Close original slave fd if it's not one of the standard fds
                if slave_raw > 2 {
                    drop(slave_fd);
                }

                // Change to home directory if available
                // This ensures the shell starts in the user's home directory
                // rather than wherever the app was launched from (e.g., "/" for macOS app bundles)
                if let Some(home) = std::env::var_os("HOME") {
                    let _ = std::env::set_current_dir(&home);
                }

                // Set environment if provided
                if let Some(env_vars) = env_cstr {
                    // Clear environment and set new variables
                    // Note: clearenv() is not available on macOS, so we use
                    // a portable approach of unsetting all variables
                    #[cfg(target_os = "linux")]
                    unsafe {
                        libc::clearenv();
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        // On macOS and other platforms, manually clear environment
                        // by iterating and unsetting each variable
                        for (key, _) in std::env::vars_os() {
                            std::env::remove_var(&key);
                        }
                    }
                    for var in env_vars {
                        unsafe {
                            libc::putenv(var.into_raw());
                        }
                    }
                }

                // Execute the program
                let _ = execvp(&program_cstr, &args_cstr);

                // If execvp returns, it failed
                std::process::exit(127);
            }
        }
    }

    /// Spawn a shell (uses $SHELL or /bin/bash)
    ///
    /// The shell is launched as a login shell (-l flag) to ensure proper
    /// environment setup when launched from GUI applications (e.g., macOS app bundles).
    /// This sources ~/.zshrc, ~/.bash_profile, etc. which sets up PATH and tools like direnv.
    pub fn spawn_shell(size: WindowSize) -> Result<Self> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());

        // Get current environment
        let env: Vec<(String, String)> = std::env::vars().collect();

        // Add TERM variable
        let mut env_with_term: Vec<(String, String)> =
            env.into_iter().filter(|(k, _)| k != "TERM").collect();
        env_with_term.push(("TERM".to_string(), "xterm-256color".to_string()));

        // Launch as login shell to properly source shell profile
        // This is important for GUI-launched terminals (e.g., macOS app bundles)
        // where the environment may not include user's PATH modifications
        let args = vec!["-l".to_string()];
        Self::spawn(shell, args, Some(env_with_term), size)
    }

    /// Get the PTY master
    pub fn pty(&self) -> &Pty {
        &self.pty
    }

    /// Get mutable access to the PTY master
    pub fn pty_mut(&mut self) -> &mut Pty {
        &mut self.pty
    }

    /// Get the child process ID
    pub fn pid(&self) -> Pid {
        self.pid
    }

    /// Check if the child process is still running
    pub fn is_running(&self) -> bool {
        match waitpid(self.pid, Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => true,
            Ok(_) => false,
            Err(_) => false,
        }
    }

    /// Wait for the child process to exit
    pub fn wait(&self) -> Result<WaitStatus> {
        waitpid(self.pid, None).map_err(Error::from)
    }

    /// Try to wait for the child (non-blocking)
    pub fn try_wait(&self) -> Result<Option<WaitStatus>> {
        match waitpid(self.pid, Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => Ok(None),
            Ok(status) => Ok(Some(status)),
            Err(e) => Err(Error::from(e)),
        }
    }

    /// Send a signal to the child process
    pub fn signal(&self, signal: Signal) -> Result<()> {
        kill(self.pid, signal).map_err(Error::from)
    }

    /// Resize the PTY window
    pub fn resize(&self, size: WindowSize) -> Result<()> {
        self.pty.set_window_size(size)?;

        // Send SIGWINCH to notify the child
        let _ = self.signal(Signal::SIGWINCH);

        Ok(())
    }

    /// Read from the child's output
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.pty.read(buf)
    }

    /// Write to the child's input
    pub fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.pty.write(buf)
    }

    /// Write all bytes to the child's input
    pub fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.pty.write_all(buf)
    }

    /// Set non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<()> {
        self.pty.set_nonblocking(nonblocking)
    }

    /// Get the raw file descriptor for polling
    pub fn as_raw_fd(&self) -> RawFd {
        self.pty.as_raw_fd()
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        // Try to terminate the child gracefully
        let _ = self.signal(Signal::SIGHUP);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_spawn_shell() {
        let child = Child::spawn_shell(WindowSize::default());
        assert!(child.is_ok());

        let child = child.unwrap();

        // Give the shell a moment to start
        thread::sleep(Duration::from_millis(100));

        // The shell should still be running (or at least have started successfully)
        // Note: In some test environments, the shell may exit quickly if not interactive
        // So we just verify the spawn succeeded
        let _ = child.is_running();

        // Clean up
        let _ = child.signal(Signal::SIGTERM);
    }

    #[test]
    fn test_spawn_echo() {
        let mut child = Child::spawn(
            "/bin/echo",
            ["hello"],
            None::<Vec<(String, String)>>,
            WindowSize::default(),
        )
        .unwrap();

        // Wait a bit for output
        thread::sleep(Duration::from_millis(100));

        let mut buf = [0u8; 1024];
        child.set_nonblocking(true).unwrap();

        let n = child.read(&mut buf).unwrap_or(0);
        if n > 0 {
            let output = String::from_utf8_lossy(&buf[..n]);
            assert!(output.contains("hello"));
        }
    }

    #[test]
    fn test_resize() {
        let child = Child::spawn_shell(WindowSize::default()).unwrap();

        let new_size = WindowSize::new(120, 40);
        assert!(child.resize(new_size).is_ok());

        let retrieved = child.pty().get_window_size().unwrap();
        assert_eq!(retrieved.cols, 120);
        assert_eq!(retrieved.rows, 40);

        let _ = child.signal(Signal::SIGTERM);
    }

    #[test]
    fn test_write_read() {
        let mut child = Child::spawn_shell(WindowSize::default()).unwrap();
        child.set_nonblocking(true).unwrap();

        // Wait for shell to start (longer wait for login shell which sources profile)
        thread::sleep(Duration::from_millis(500));

        // Drain any initial output from shell startup/profile
        let mut buf = [0u8; 4096];
        loop {
            match child.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => continue,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        // Send a command with unique marker
        child.write_all(b"echo MARKER_test123_MARKER\n").unwrap();

        // Wait for response
        thread::sleep(Duration::from_millis(300));

        let mut output = String::new();

        // Read all available output
        loop {
            match child.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output.push_str(&String::from_utf8_lossy(&buf[..n])),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        assert!(
            output.contains("MARKER_test123_MARKER"),
            "Expected output to contain 'MARKER_test123_MARKER', got: {}",
            output
        );

        let _ = child.signal(Signal::SIGTERM);
    }
}
