# Mochi Terminal Emulator

A real Linux terminal emulator built from scratch in Rust.

## Overview

Mochi is a VT/xterm-style terminal emulator that:
- Runs a real shell and applications via PTY
- Correctly parses escape sequences
- Maintains a proper screen model
- Renders a GUI with CPU-based rendering

## Features

### Implemented
- UTF-8 text with wide character support (CJK, emoji)
- 16/256/truecolor support
- Cursor styles (block, underline, bar) and visibility
- Alternate screen buffer
- Scroll regions (DECSTBM)
- Scrollback buffer (configurable size)
- Bracketed paste mode
- Mouse reporting (X10, VT200, SGR 1006)
- OSC sequences (window title, hyperlinks, clipboard)
- Selection and clipboard copy/paste

### Escape Sequence Support
See [escape-sequences.md](escape-sequences.md) for detailed coverage.

## Architecture

The terminal is split into four crates:

### terminal-core
Platform-independent screen model:
- Grid of cells with attributes
- Cursor state and movement
- Scrollback ring buffer
- Selection handling
- Mode flags

### terminal-parser
VT/xterm escape sequence parser:
- Streaming parser (handles arbitrary chunk boundaries)
- CSI, OSC, ESC, DCS sequence support
- UTF-8 decoding
- Deterministic behavior

### terminal-pty
Linux PTY management:
- PTY creation and configuration
- Child process spawning
- Window size propagation (SIGWINCH)
- Non-blocking I/O

### mochi-term
GUI application:
- Window creation (winit)
- CPU-based rendering (softbuffer + fontdue)
- Keyboard and mouse input handling
- Configuration system

## Building

```bash
cd terminal
cargo build --release
```

## Running

```bash
cargo run --release
```

Or run the built binary:
```bash
./target/release/mochi
```

## Configuration

Configuration file location: `~/.config/mochi/config.toml`

Example configuration:
```toml
[font]
family = "monospace"
size = 14.0

[terminal]
scrollback_size = 10000

[colors]
foreground = "#d4d4d4"
background = "#1e1e1e"
```

## Testing

Run all tests:
```bash
cargo test
```

Run tests for a specific crate:
```bash
cargo test -p terminal-core
cargo test -p terminal-parser
cargo test -p terminal-pty
```

## Dependencies

This terminal emulator is built from scratch without using terminal emulation libraries. The following dependencies are used:

- **winit**: Window creation and event handling
- **softbuffer**: CPU-based rendering
- **fontdue**: Font rasterization
- **nix**: POSIX system calls for PTY
- **arboard**: Clipboard integration
- **serde/toml**: Configuration
- **unicode-width**: Character width calculation

## License

MIT
