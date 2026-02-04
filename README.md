# Mochi Terminal

A real Linux terminal emulator built from scratch in Rust. No terminal emulation libraries - everything is implemented from first principles.

## Overview

Mochi is a VT/xterm-compatible terminal emulator that:
- Runs a real shell and applications via PTY
- Correctly parses escape sequences using a state machine based on the VT500-series model
- Maintains a screen model with scrollback buffer
- Renders a GPU-accelerated GUI using wgpu

## Quick Start

```bash
# Build the terminal
cd terminal
cargo build --release --features gui

# Run the terminal
cargo run --release --features gui --bin mochi_term
```

## Features

- **Full VT/xterm escape sequence support**: Cursor movement, erase operations, SGR attributes, scroll regions, alternate screen, and more
- **Color support**: 16-color, 256-color, and truecolor (24-bit RGB)
- **UTF-8**: Full Unicode support including wide characters (CJK) and combining marks
- **PTY**: Proper pseudoterminal handling with resize support
- **GPU rendering**: Hardware-accelerated rendering with wgpu
- **Security**: OSC 52 clipboard disabled by default, hyperlink URI validation

## Documentation

- [Terminal README](terminal/README.md) - Detailed usage and configuration
- [Architecture](docs/architecture.md) - System design and module structure
- [Escape Sequences](docs/escape-sequences.md) - Coverage matrix of supported sequences
- [Security](docs/security.md) - Security considerations and controls
- [Testing](docs/testing.md) - Testing strategy and instructions

## Project Structure

```
mochi/
├── terminal/           # Terminal emulator implementation
│   ├── src/
│   │   ├── core/      # Platform-independent screen model
│   │   ├── parser/    # Escape sequence parser
│   │   ├── pty/       # PTY management (Linux)
│   │   ├── frontend/  # GUI renderer
│   │   └── app/       # Configuration and glue
│   ├── tests/         # Integration and golden tests
│   └── benches/       # Performance benchmarks
└── docs/              # Documentation
```

## Testing

```bash
cd terminal
cargo test
```

The test suite includes 66 tests:
- 54 golden tests for escape sequence parsing
- 11 PTY integration tests
- 1 documentation test

## License

See LICENSE file for details.
