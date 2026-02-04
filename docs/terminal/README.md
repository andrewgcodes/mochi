# Mochi Terminal Emulator

A real Linux terminal emulator built from scratch in Rust. This implementation does not use any terminal emulation libraries - all parsing, screen management, and rendering is implemented from first principles.

## Overview

Mochi Terminal is a VT/xterm-compatible terminal emulator that:

- Runs a real shell via PTY (pseudo-terminal)
- Correctly parses VT100/xterm escape sequences
- Maintains a screen model with scrollback
- Renders a GUI with proper font support
- Supports UTF-8, colors (16/256/truecolor), mouse, and clipboard

## Architecture

The terminal is organized into several independent modules:

```
terminal/
├── src/
│   ├── core/           # Platform-independent terminal core
│   │   ├── cell.rs     # Cell representation with colors/styles
│   │   ├── cursor.rs   # Cursor state and positioning
│   │   ├── screen.rs   # Screen model (primary + alternate)
│   │   ├── scrollback.rs # Scrollback ring buffer
│   │   └── snapshot.rs # Deterministic state snapshots
│   ├── parser/         # Escape sequence parser
│   │   ├── actions.rs  # Parsed terminal actions
│   │   └── state.rs    # Parser state machine
│   ├── pty/            # PTY and child process management
│   │   └── linux.rs    # Linux-specific PTY code
│   ├── input/          # Keyboard/mouse input encoding
│   ├── renderer/       # GUI rendering
│   │   ├── font.rs     # Font rasterization
│   │   └── window.rs   # Window management
│   ├── terminal.rs     # Terminal executor (ties it all together)
│   ├── main.rs         # GUI application entry point
│   ├── headless.rs     # Headless runner for testing
│   └── lib.rs          # Library exports
```

## Design Principles

### 1. Deterministic Core

The terminal core is completely deterministic: given the same byte stream, it will always produce the same screen state. This enables reliable testing and debugging.

### 2. No Terminal Libraries

This implementation explicitly does NOT use:
- libvte, VTE, or any GTK terminal widget
- termwiz, vt100, or any Rust terminal emulation crate
- libtsm, kmscon, or any other terminal emulation library
- Code copied from xterm, alacritty, kitty, wezterm, etc.

Allowed dependencies are limited to:
- Window/event handling (winit)
- Rendering (wgpu)
- Font rasterization (fontdue)
- Unicode width tables (unicode-width)
- System calls (nix, libc)
- Clipboard (arboard)

### 3. Streaming Parser

The parser handles arbitrary chunk boundaries, allowing incremental processing of PTY output. This is essential for real-world use where data arrives in unpredictable chunks.

### 4. Separation of Concerns

- **Parser**: Converts bytes to semantic actions
- **Screen**: Maintains terminal state
- **Renderer**: Draws the screen (no terminal logic)
- **PTY**: Manages child process (no parsing)

## Building

```bash
cd terminal
cargo build --release
```

## Running

```bash
# Run the terminal emulator
cargo run --release

# Run the headless test tool
cargo run --release --bin mochi-headless -- --help
```

## Testing

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_terminal_colors
```

## Supported Features

See [escape-sequences.md](../escape-sequences.md) for the full coverage matrix.

### Currently Implemented

- **Text**: UTF-8 with combining characters and wide character support
- **Colors**: 16 standard, 256 indexed, and 24-bit true color
- **Cursor**: Movement, visibility, save/restore, multiple styles
- **Erase**: In display (ED), in line (EL), characters (ECH)
- **Insert/Delete**: Lines (IL/DL), characters (ICH/DCH)
- **Scroll**: Region (DECSTBM), up/down (SU/SD)
- **Modes**: Origin, autowrap, insert, bracketed paste, mouse tracking
- **Alternate Screen**: Full support with cursor save/restore
- **Mouse**: X10, normal, button-event, any-event tracking with SGR encoding
- **OSC**: Window title, hyperlinks, clipboard (with security controls)

### Known Limitations

- DEC Special Graphics (line drawing) character set not fully implemented
- Some advanced DCS sequences not supported
- Sixel graphics not supported
- Emoji with ZWJ sequences may not render correctly

## Security

See [security.md](../security.md) for security considerations, especially regarding:
- OSC 52 clipboard access
- OSC 8 hyperlinks
- Resource limits (scrollback, sequence length)

## References

- [Xterm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)
- [ECMA-48](https://ecma-international.org/publications-and-standards/standards/ecma-48/)
- [VT220 Programmer Reference](https://vt100.net/docs/vt220-rm/)
- [vttest](https://invisible-island.net/vttest/)

## License

MIT License - see LICENSE file for details.
