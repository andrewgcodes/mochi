# Linux PTY (Pseudoterminal) Research

This document summarizes research on implementing PTY handling for the terminal emulator.

## Overview

A pseudoterminal (PTY) is a pair of virtual character devices:
- **Master**: Controlled by the terminal emulator
- **Slave**: Appears as a real terminal to the child process

Data flow:
```
Terminal Emulator ←→ PTY Master ←→ PTY Slave ←→ Shell/Application
```

## POSIX PTY API

### Opening the Master

```c
#include <stdlib.h>
#include <fcntl.h>

int master_fd = posix_openpt(O_RDWR | O_NOCTTY);
```

- `O_RDWR`: Open for reading and writing
- `O_NOCTTY`: Don't make this the controlling terminal for the process

### Granting Access

```c
#include <stdlib.h>

int result = grantpt(master_fd);
```

Changes ownership and permissions of the slave device:
- Owner: Real UID of calling process
- Group: Unspecified (typically `tty`)
- Mode: Readable and writable by owner, writable by group

### Unlocking the Slave

```c
#include <stdlib.h>

int result = unlockpt(master_fd);
```

Unlocks the slave so it can be opened.

### Getting Slave Name

```c
#include <stdlib.h>

char *slave_name = ptsname(master_fd);
// Returns something like "/dev/pts/3"
```

Note: `ptsname()` is not thread-safe. Use `ptsname_r()` for thread safety.

## Spawning the Child Process

### Fork and Setup

```c
pid_t pid = fork();

if (pid == 0) {
    // Child process
    
    // Create new session (become session leader)
    setsid();
    
    // Open slave - this becomes the controlling terminal
    int slave_fd = open(slave_name, O_RDWR);
    
    // Duplicate to stdin/stdout/stderr
    dup2(slave_fd, STDIN_FILENO);
    dup2(slave_fd, STDOUT_FILENO);
    dup2(slave_fd, STDERR_FILENO);
    
    // Close original slave fd if not 0, 1, or 2
    if (slave_fd > STDERR_FILENO) {
        close(slave_fd);
    }
    
    // Close master fd (inherited from parent)
    close(master_fd);
    
    // Execute shell
    execvp(shell, args);
    
    // If exec fails
    _exit(1);
}

// Parent process
// Close slave fd (we only need master)
// Set master to non-blocking for polling
```

### Session and Controlling Terminal

- `setsid()`: Creates a new session, child becomes session leader
- Opening the slave PTY makes it the controlling terminal
- This allows the shell to receive signals (SIGINT, SIGTSTP, etc.)

## Window Size

### Setting Size

```c
#include <sys/ioctl.h>

struct winsize ws = {
    .ws_row = rows,
    .ws_col = cols,
    .ws_xpixel = 0,  // Optional pixel dimensions
    .ws_ypixel = 0,
};

ioctl(master_fd, TIOCSWINSZ, &ws);
```

### SIGWINCH

When the window size changes:
1. Call `ioctl(TIOCSWINSZ)` on the master
2. The kernel sends `SIGWINCH` to the foreground process group
3. Applications (vim, less, etc.) handle SIGWINCH to redraw

## Non-blocking I/O

For responsive terminal emulation, use non-blocking I/O:

```c
#include <fcntl.h>

int flags = fcntl(master_fd, F_GETFL);
fcntl(master_fd, F_SETFL, flags | O_NONBLOCK);
```

Then use `poll()` or `epoll()` to wait for data:

```c
#include <poll.h>

struct pollfd pfd = {
    .fd = master_fd,
    .events = POLLIN,
};

int ready = poll(&pfd, 1, timeout_ms);
if (ready > 0 && (pfd.revents & POLLIN)) {
    // Data available to read
}
```

## Reading and Writing

### Reading from Master

```c
char buf[4096];
ssize_t n = read(master_fd, buf, sizeof(buf));

if (n > 0) {
    // Process n bytes of output from child
} else if (n == 0) {
    // EOF - child closed the PTY
} else if (errno == EAGAIN || errno == EWOULDBLOCK) {
    // No data available (non-blocking)
} else {
    // Error
}
```

### Writing to Master

```c
ssize_t n = write(master_fd, data, len);
// Bytes written to master appear as input to child
```

## Child Process Lifecycle

### Detecting Child Exit

Use `waitpid()` with `WNOHANG`:

```c
int status;
pid_t result = waitpid(child_pid, &status, WNOHANG);

if (result == child_pid) {
    if (WIFEXITED(status)) {
        int exit_code = WEXITSTATUS(status);
    } else if (WIFSIGNALED(status)) {
        int signal = WTERMSIG(status);
    }
}
```

Or handle `SIGCHLD` signal.

### Closing the PTY

When done:
1. Close master fd
2. Wait for child to exit (if not already)

## Environment Variables

Set these for the child:

```c
setenv("TERM", "xterm-256color", 1);  // Or custom terminfo
setenv("COLORTERM", "truecolor", 1);   // Indicate truecolor support
setenv("COLUMNS", "80", 1);            // Initial size
setenv("LINES", "24", 1);
```

## Error Handling

Common errors:
- `ENOENT`: No more PTY devices available
- `EACCES`: Permission denied on slave
- `EIO`: I/O error (child may have exited)
- `EAGAIN`: No data available (non-blocking)

## Rust Implementation with nix

```rust
use nix::fcntl::{open, OFlag};
use nix::pty::{posix_openpt, grantpt, unlockpt, ptsname};
use nix::sys::stat::Mode;
use nix::unistd::{fork, ForkResult, setsid, dup2, close, execvp};
use nix::sys::termios;
use nix::libc::{STDIN_FILENO, STDOUT_FILENO, STDERR_FILENO};

// Open master
let master = posix_openpt(OFlag::O_RDWR | OFlag::O_NOCTTY)?;

// Grant and unlock
grantpt(&master)?;
unlockpt(&master)?;

// Get slave name
let slave_name = unsafe { ptsname(&master)? };

match unsafe { fork()? } {
    ForkResult::Child => {
        // Create new session
        setsid()?;
        
        // Open slave
        let slave = open(slave_name.as_str(), OFlag::O_RDWR, Mode::empty())?;
        
        // Dup to stdio
        dup2(slave, STDIN_FILENO)?;
        dup2(slave, STDOUT_FILENO)?;
        dup2(slave, STDERR_FILENO)?;
        
        if slave > STDERR_FILENO {
            close(slave)?;
        }
        
        // Exec shell
        execvp(shell, &args)?;
    }
    ForkResult::Parent { child } => {
        // Store child pid, set master non-blocking, etc.
    }
}
```

## References

1. pty(7) man page: https://man7.org/linux/man-pages/man7/pty.7.html
2. posix_openpt(3): https://man7.org/linux/man-pages/man3/posix_openpt.3.html
3. grantpt(3): https://man7.org/linux/man-pages/man3/grantpt.3.html
4. unlockpt(3): https://man7.org/linux/man-pages/man3/unlockpt.3.html
5. ptsname(3): https://man7.org/linux/man-pages/man3/ptsname.3.html
6. tty_ioctl(4): https://man7.org/linux/man-pages/man4/tty_ioctl.4.html
