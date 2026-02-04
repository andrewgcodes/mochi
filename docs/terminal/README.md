# Mochi Terminal Emulator

A real Linux terminal emulator built from scratch in Rust. This project implements VT/xterm-style terminal emulation without relying on any existing terminal emulator libraries.

## Project Goals

1. **Real Terminal Emulation**: Run actual shells and applications via PTY, not a simulation
2. **Correct Escape Sequence Parsing**: Implement VT100/VT220/xterm escape sequences properly
3. **Clean Architecture**: Separate concerns between parsing, screen model, and rendering
4. **Comprehensive Testing**: Unit tests, golden tests, integration tests, and fuzzing
5. **Documentation**: Document all supported sequences and behaviors

## Non-Goals

- Wrapping existing terminal libraries (libvte, termwiz, etc.)
- Perfect compatibility with every terminal application (we target common use cases)
- Windows/macOS support in the initial version (Linux-first)

## Architecture

The terminal emulator is organized into four crates:

### mochi-core

Platform-independent terminal logic:
- **Screen Model**: Grid of cells with attributes, cursor state, scroll regions
- **Cell**: Character with foreground/background colors and text attributes
- **Line**: Row of cells with wrap flag
- **Scrollback**: Ring buffer for terminal history
- **Selection**: Text selection logic for copy/paste
- **Snapshot**: Serialization for testing and debugging

### mochi-parser

VT/xterm escape sequence parser:
- **State Machine**: Handles ESC, CSI, OSC, DCS sequences
- **Streaming**: Processes arbitrary chunk boundaries correctly
- **UTF-8**: Proper multi-byte character decoding
- **Actions**: Semantic operations (Print, Execute, CsiDispatch, etc.)

### mochi-pty

Linux PTY handling:
- **PTY Creation**: posix_openpt, grantpt, unlockpt, ptsname
- **Child Spawning**: Fork/exec with proper session setup
- **I/O**: Non-blocking read/write to PTY master
- **Resize**: TIOCSWINSZ propagation

### mochi-term

GUI application:
- **Performer**: Applies parsed actions to screen state
- **Input Encoder**: Converts keyboard/mouse events to terminal sequences
- **Renderer**: GPU-accelerated rendering (planned)

## Building

```bash
cd terminal
cargo build
```

## Running

```bash
cd terminal
cargo run
```

## Testing

```bash
cd terminal
cargo test
```

## Supported Features

### Control Characters (C0)
- BEL (0x07): Bell
- BS (0x08): Backspace
- HT (0x09): Horizontal tab
- LF (0x0A): Line feed
- VT (0x0B): Vertical tab (treated as LF)
- FF (0x0C): Form feed (treated as LF)
- CR (0x0D): Carriage return
- ESC (0x1B): Escape sequence introducer

### ESC Sequences
- ESC 7: Save cursor (DECSC)
- ESC 8: Restore cursor (DECRC)
- ESC D: Index (IND)
- ESC E: Next line (NEL)
- ESC H: Horizontal tab set (HTS)
- ESC M: Reverse index (RI)

### CSI Sequences
- CSI A: Cursor up (CUU)
- CSI B: Cursor down (CUD)
- CSI C: Cursor forward (CUF)
- CSI D: Cursor backward (CUB)
- CSI G: Cursor character absolute (CHA)
- CSI H: Cursor position (CUP)
- CSI J: Erase in display (ED)
- CSI K: Erase in line (EL)
- CSI L: Insert lines (IL)
- CSI M: Delete lines (DL)
- CSI P: Delete characters (DCH)
- CSI X: Erase characters (ECH)
- CSI @: Insert characters (ICH)
- CSI d: Line position absolute (VPA)
- CSI f: Horizontal and vertical position (HVP)
- CSI g: Tab clear (TBC)
- CSI m: Select graphic rendition (SGR)
- CSI r: Set scroll region (DECSTBM)
- CSI s: Save cursor position
- CSI u: Restore cursor position

### SGR (Select Graphic Rendition)
- 0: Reset all attributes
- 1: Bold
- 2: Faint
- 3: Italic
- 4: Underline
- 7: Inverse
- 8: Hidden
- 9: Strikethrough
- 22: Normal intensity
- 23: Not italic
- 24: Not underlined
- 27: Not inverse
- 28: Not hidden
- 29: Not strikethrough
- 30-37: Foreground colors (ANSI)
- 38;5;N: Foreground 256-color
- 38;2;R;G;B: Foreground truecolor
- 39: Default foreground
- 40-47: Background colors (ANSI)
- 48;5;N: Background 256-color
- 48;2;R;G;B: Background truecolor
- 49: Default background
- 90-97: Bright foreground colors
- 100-107: Bright background colors

### DEC Private Modes
- ?25: Cursor visibility (DECTCEM)
- ?1049: Alternate screen buffer
- ?2004: Bracketed paste mode
- ?1000: X10 mouse tracking
- ?1002: Button event mouse tracking
- ?1003: Any event mouse tracking
- ?1006: SGR mouse encoding

### OSC Sequences
- OSC 0: Set icon name and window title
- OSC 2: Set window title
- OSC 8: Hyperlinks
- OSC 52: Clipboard operations

## Dependencies

This project does NOT use any terminal emulator libraries. All terminal logic is implemented from scratch.

Allowed dependencies:
- **nix**: Safe Rust bindings to POSIX APIs (PTY, signals)
- **winit**: Window creation and event handling
- **wgpu**: GPU rendering
- **fontdue**: Font rasterization
- **unicode-width/segmentation**: Unicode character properties
- **serde/serde_json**: Serialization for testing

## References

- [Xterm Control Sequences](https://www.x.org/docs/xterm/ctlseqs.pdf)
- [ECMA-48](https://ecma-international.org/wp-content/uploads/ECMA-48_5th_edition_june_1991.pdf)
- [VT220 Programmer Reference](https://vt100.net/dec/ek-vt220-rm-001.pdf)
- [Linux PTY man pages](https://www.man7.org/linux/man-pages/man3/posix_openpt.3.html)

## License

MIT
