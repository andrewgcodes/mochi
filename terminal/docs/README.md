# Mochi Terminal Emulator

A real VT/xterm-style terminal emulator built from scratch in Rust.

## Overview

Mochi Terminal is a genuine terminal emulator that:
- Runs a real shell and applications via a PTY (pseudo-terminal)
- Correctly parses VT/xterm escape sequences
- Maintains a deterministic screen model
- Renders a GUI using modern graphics APIs

**Important**: This is NOT a wrapper around an existing terminal library. All terminal emulation logic is implemented from scratch.

## Features

### Implemented
- PTY management (posix_openpt, grantpt, unlockpt, ptsname)
- Child process spawning with proper session/controlling terminal setup
- Window resize propagation (TIOCSWINSZ + SIGWINCH)
- UTF-8 text handling with wide character support
- 16-color, 256-color, and truecolor (24-bit) support
- Cursor movement and positioning (CUP, CUU, CUD, CUF, CUB, etc.)
- Erase operations (ED, EL, ECH)
- Insert/delete operations (ICH, DCH, IL, DL)
- Scroll regions (DECSTBM)
- SGR attributes (bold, italic, underline, inverse, etc.)
- Alternate screen buffer
- Bracketed paste mode
- Mouse reporting modes (X10, VT200, SGR, URXVT)
- OSC sequences (window title, hyperlinks)
- Cursor styles (block, underline, bar)
- Application cursor/keypad modes
- Charset designation (ASCII, DEC Special Graphics, UK)
- Scrollback buffer with configurable size

### Planned
- OSC 52 clipboard (with security controls)
- Selection and copy/paste
- Font configuration
- Color scheme customization
- Performance optimizations

## Architecture

The terminal is split into four crates:

```
terminal/
├── mochi-core/     # Platform-independent terminal state
├── mochi-parser/   # Escape sequence parser
├── mochi-pty/      # Linux PTY management
└── mochi-term/     # GUI application
```

### mochi-core
Contains the terminal state machine including:
- Cell representation with attributes
- Grid/screen model
- Cursor state
- Scrollback buffer
- Selection logic
- Snapshot serialization for testing

### mochi-parser
Streaming escape sequence parser that converts bytes to semantic actions:
- C0/C1 control characters
- ESC sequences
- CSI sequences with parameters
- OSC sequences
- DCS sequences (consumed but not fully implemented)

### mochi-pty
Linux-specific PTY management:
- PTY creation and setup
- Child process spawning
- Non-blocking I/O
- Window size management

### mochi-term
GUI application that ties everything together:
- Window creation with winit
- Rendering with software rasterizer (wgpu optional)
- Keyboard input encoding
- Mouse input encoding
- Event loop coordination

## Building

```bash
cd terminal
cargo build --release
```

## Running

```bash
cargo run --release -p mochi-term
```

## Testing

```bash
# Run all tests
cargo test

# Run specific crate tests
cargo test -p mochi-core
cargo test -p mochi-parser
cargo test -p mochi-pty
```

## Documentation

- [Architecture](architecture.md) - Detailed architecture documentation
- [Escape Sequences](escape-sequences.md) - Supported escape sequences
- [Security](security.md) - Security considerations
- [ADRs](adr/) - Architecture Decision Records

## Compatibility

The terminal aims to be compatible with xterm-256color. It should work with:
- bash, zsh, fish
- vim, nvim
- htop, top
- less, man
- tmux (basic functionality)
- ssh

## License

MIT License - see LICENSE file for details.

## References

- [Xterm Control Sequences](https://www.x.org/docs/xterm/ctlseqs.pdf)
- [ECMA-48](https://ecma-international.org/wp-content/uploads/ECMA-48_5th_edition_june_1991.pdf)
- [VT220 Programmer Reference](https://vt100.net/dec/ek-vt220-rm-001.pdf)
- [vttest](https://invisible-island.net/vttest/)
