# Mochi Terminal Emulator

A terminal emulator built from scratch in Rust, without using any terminal emulation libraries.

## Overview

Mochi Terminal is a genuine VT/xterm-style terminal emulator that:
- Runs a real shell/applications via PTY (pseudo-terminal)
- Correctly parses escape sequences according to ECMA-48 and xterm documentation
- Maintains a proper screen model with scrollback
- Renders a GUI using software rendering

## Goals

1. **Correctness**: Properly implement VT/xterm escape sequences as documented
2. **Usability**: Support common TUI applications (vim, htop, tmux, less, man)
3. **Testability**: Deterministic behavior with comprehensive test coverage
4. **Security**: Safe handling of potentially dangerous sequences (OSC 52, etc.)
5. **Documentation**: Clear documentation of supported features and behavior

## Non-Goals

1. GPU-accelerated rendering (we use software rendering for simplicity)
2. Sixel graphics or other image protocols
3. Ligatures or advanced typography
4. Windows/macOS support (Linux only for now)

## Architecture

The terminal is organized into several modules:

### `core` - Screen Model (Platform-Independent)
- `Cell`: Character storage with attributes (colors, styles)
- `Grid`: 2D array of cells
- `Cursor`: Position and state management
- `Scrollback`: Ring buffer for scrolled-off lines
- `Screen`: Complete terminal state

### `parser` - Escape Sequence Parser (Platform-Independent)
- State machine based on ECMA-48 and xterm documentation
- Handles UTF-8 decoding
- Produces semantic `Action` values
- Supports incremental streaming (arbitrary chunk boundaries)

### `pty` - PTY Management (Linux-Specific)
- PTY creation and child process spawning
- Non-blocking I/O
- Window size management (TIOCSWINSZ)
- Signal handling

### `gui` - GUI Rendering
- Window creation with winit
- Font rendering with fontdue
- Keyboard/mouse input encoding
- Selection and clipboard integration

## Supported Features

See [escape-sequences.md](../escape-sequences.md) for detailed coverage.

### Implemented
- UTF-8 text with wide character support
- 16/256/truecolor support
- Cursor movement and positioning
- Screen/line erase operations
- Insert/delete line/character
- Scroll regions
- Alternate screen buffer
- Bracketed paste mode
- Mouse reporting (X10, SGR)
- OSC window title
- OSC 8 hyperlinks

### Not Yet Implemented
- DEC Special Graphics charset (line drawing)
- Sixel graphics
- OSC 52 clipboard (disabled by default for security)

## Building

```bash
cd terminal
cargo build --release
```

## Running

```bash
# Run the terminal emulator
cargo run --release --bin mochi-term

# Run the PTY relay (for testing)
cargo run --release --bin pty-relay

# Run the headless runner (for testing)
echo -e "Hello\x1b[31mRed\x1b[0m" | cargo run --release --bin headless-runner
```

## Testing

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test parser::tests::test_parser_csi_cursor_up
```

## Dependencies

This project does NOT use any terminal emulation libraries. All parsing and screen model logic is implemented from scratch.

See [ADR 0001](../adr/0001-terminal-language-and-stack.md) for the full dependency audit.

## References

- [Xterm Control Sequences](https://www.x.org/docs/xterm/ctlseqs.pdf)
- [ECMA-48](https://ecma-international.org/wp-content/uploads/ECMA-48_5th_edition_june_1991.pdf)
- [VT220 Programmer Reference](https://vt100.net/dec/ek-vt220-rm-001.pdf)
- [terminfo(5)](https://man7.org/linux/man-pages/man5/terminfo.5.html)
