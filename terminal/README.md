# Mochi Terminal Emulator

A real Linux terminal emulator built from scratch in Rust. This terminal emulator runs a real shell/apps via PTY, correctly parses VT/xterm escape sequences, maintains a screen model with scrollback, and renders a GPU-accelerated GUI.

## Features

### Core Terminal Features
- Full VT/xterm-compatible escape sequence parsing
- Screen model with primary and alternate screen buffers
- Scrollback buffer (configurable, default 10,000 lines)
- UTF-8 support with wide character handling (CJK)
- 16-color, 256-color, and truecolor (24-bit RGB) support
- All standard text attributes (bold, italic, underline, inverse, strikethrough, etc.)

### Escape Sequence Support
- C0 control characters (BEL, BS, HT, LF, CR, etc.)
- ESC sequences (DECSC/DECRC, IND, RI, NEL, HTS, etc.)
- CSI sequences:
  - Cursor movement (CUU, CUD, CUF, CUB, CUP, etc.)
  - Erase operations (ED, EL, ECH)
  - Insert/delete operations (ICH, DCH, IL, DL)
  - Scroll region (DECSTBM)
  - SGR (full color and attribute support)
  - Mode set/reset (cursor visibility, alternate screen, mouse modes, etc.)
- OSC sequences:
  - Window title (OSC 0, 1, 2)
  - Hyperlinks (OSC 8) with security validation
  - Clipboard (OSC 52) - disabled by default for security

### PTY Support
- Full PTY lifecycle management
- Resize propagation (SIGWINCH)
- Non-blocking I/O

### GUI (requires `gui` feature)
- GPU-accelerated rendering with wgpu
- Font rasterization with fontdue
- Glyph caching with texture atlas
- Clipboard integration

## Building

### Library only (no GUI)
```bash
cd terminal
cargo build --release
```

### With GUI support
```bash
cd terminal
cargo build --release --features gui
```

## Running

### Headless mode (for testing)
```bash
cargo run --bin mochi_headless -- input.txt
```

### Interactive terminal (requires GUI)
```bash
cargo run --release --features gui --bin mochi_term
```

## Testing

Run all tests:
```bash
cargo test
```

The test suite includes:
- 54 golden tests for escape sequence parsing
- 11 PTY integration tests
- Unit tests for core components

## Configuration

Configuration is loaded from `~/.config/mochi/config.json`. Example:

```json
{
  "font_family": "monospace",
  "font_size": 12.0,
  "scrollback_lines": 10000,
  "colors": {
    "foreground": [255, 255, 255],
    "background": [0, 0, 0]
  },
  "security": {
    "osc52_read": false,
    "osc52_write": false,
    "allow_file_urls": false
  }
}
```

## Terminfo

A terminfo entry is provided in `assets/terminfo/mochi.terminfo`. To install:

```bash
tic -x assets/terminfo/mochi.terminfo
export TERM=mochi
```

Alternatively, use `TERM=xterm-256color` which we aim to be compatible with.

## Architecture

The terminal is organized into several modules:

- **core**: Platform-independent screen model and state management
  - `screen.rs`: Screen grid, cursor, scrollback
  - `cell.rs`: Cell representation with colors and attributes
  - `parser/`: VT/xterm escape sequence parser
  - `selection.rs`: Text selection handling
  
- **pty**: Linux PTY management
  - Spawn child processes
  - Handle resize events
  - Non-blocking I/O

- **frontend**: GUI rendering (feature-gated)
  - `renderer.rs`: GPU-accelerated rendering with wgpu
  - `input.rs`: Keyboard and mouse input encoding

- **app**: Application configuration and glue

## Security

### OSC 52 Clipboard
OSC 52 clipboard access is disabled by default to prevent clipboard exfiltration attacks. Enable with caution in the configuration.

### Hyperlinks (OSC 8)
Hyperlink URIs are validated to only allow safe schemes (http, https, mailto, file). JavaScript and data URIs are rejected.

## Documentation

- [Architecture Overview](../docs/architecture.md)
- [Escape Sequence Coverage](../docs/escape-sequences.md)
- [Security Considerations](../docs/security.md)
- [Testing Guide](../docs/testing.md)

## License

See the repository root for license information.
