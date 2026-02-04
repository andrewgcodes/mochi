# Mochi Terminal Architecture

## Overview

Mochi Terminal is organized as a layered architecture with clear separation between platform-independent core logic and platform-specific components.

```
┌─────────────────────────────────────────────────────────────┐
│                        GUI Layer                             │
│  ┌─────────┐  ┌──────────┐  ┌─────────┐  ┌───────────────┐  │
│  │ Window  │  │ Renderer │  │  Input  │  │   Selection   │  │
│  │ (winit) │  │(softbuf) │  │ Encoder │  │   Clipboard   │  │
│  └─────────┘  └──────────┘  └─────────┘  └───────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     Core Layer (Platform-Independent)        │
│  ┌─────────────────────────┐  ┌───────────────────────────┐ │
│  │        Screen           │  │         Parser            │ │
│  │  ┌──────┐ ┌──────────┐  │  │  ┌─────────────────────┐  │ │
│  │  │ Grid │ │ Cursor   │  │  │  │   State Machine     │  │ │
│  │  └──────┘ └──────────┘  │  │  │   (ESC/CSI/OSC)     │  │ │
│  │  ┌──────┐ ┌──────────┐  │  │  └─────────────────────┘  │ │
│  │  │Scroll│ │  Modes   │  │  │  ┌─────────────────────┐  │ │
│  │  │ back │ │          │  │  │  │   UTF-8 Decoder     │  │ │
│  │  └──────┘ └──────────┘  │  │  └─────────────────────┘  │ │
│  └─────────────────────────┘  └───────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     PTY Layer (Linux-Specific)               │
│  ┌─────────────────────────────────────────────────────────┐│
│  │                    PTY Manager                           ││
│  │  • Master/Slave pair creation                           ││
│  │  • Child process spawning                               ││
│  │  • Non-blocking I/O                                     ││
│  │  • Window size management (TIOCSWINSZ)                  ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

## Module Boundaries

### `core` Module

The core module is completely platform-independent and contains no GUI or system-specific code.

**Responsibilities:**
- Screen state management (grid, cursor, scrollback)
- Cell attributes (colors, styles)
- Terminal modes (origin, autowrap, insert, etc.)
- Tab stops
- Scroll regions

**Key Types:**
- `Screen`: Complete terminal state
- `Grid`: 2D array of cells
- `Cell`: Character with attributes
- `Cursor`: Position and attributes
- `Scrollback`: Ring buffer of scrolled lines
- `Modes`: Terminal mode flags

**Invariants:**
- Given the same sequence of operations, produces identical state
- No I/O or system calls
- No GUI dependencies

### `parser` Module

The parser converts byte streams into semantic actions.

**Responsibilities:**
- UTF-8 decoding
- C0 control character handling
- ESC sequence parsing
- CSI sequence parsing (with parameters)
- OSC sequence parsing
- DCS/APC/PM/SOS handling (consumed but not interpreted)

**Key Types:**
- `Parser`: Stateful parser
- `Action`: Semantic operation (Print, Control, Csi, Osc, Esc, etc.)
- `CsiAction`: Parsed CSI with params and final char
- `OscAction`: Parsed OSC command

**Invariants:**
- Handles arbitrary chunk boundaries correctly
- Never panics on invalid input
- Bounded memory usage (max params, max string length)

### `pty` Module

Linux-specific PTY management.

**Responsibilities:**
- PTY creation (openpty)
- Child process spawning (fork/exec)
- Session and controlling terminal setup
- Non-blocking I/O
- Window size propagation (TIOCSWINSZ + SIGWINCH)
- Child process lifecycle management

**Key Types:**
- `Pty`: PTY handle with child process

### `gui` Module

GUI rendering and input handling.

**Responsibilities:**
- Window creation and management
- Font loading and glyph rasterization
- Screen rendering
- Keyboard input encoding
- Mouse event handling
- Selection management
- Clipboard integration

**Key Types:**
- `Renderer`: Renders screen to window
- `FontRenderer`: Glyph rasterization
- `InputEncoder`: Key to escape sequence encoding
- `Selection`: Text selection state

## Data Flow

### Input (User → Shell)

```
Keyboard Event
    │
    ▼
InputEncoder.encode_key()
    │
    ▼
Escape Sequence Bytes
    │
    ▼
PTY.write()
    │
    ▼
Child Process (Shell)
```

### Output (Shell → Display)

```
Child Process Output
    │
    ▼
PTY.read()
    │
    ▼
Parser.parse()
    │
    ▼
Vec<Action>
    │
    ▼
Screen.apply_action() (for each action)
    │
    ▼
Screen State Updated
    │
    ▼
Renderer.render()
    │
    ▼
Window Display
```

## Threading Model

The terminal uses a single-threaded event loop:

1. Poll for window events (keyboard, mouse, resize)
2. Poll for PTY output (non-blocking read)
3. Parse PTY output and apply to screen
4. Render screen to window
5. Repeat

This simplifies synchronization and ensures deterministic behavior.

## Memory Management

### Screen Buffer
- Primary screen: `rows × cols` cells
- Alternate screen: `rows × cols` cells
- Scrollback: Ring buffer with configurable max lines (default 10,000)

### Glyph Cache
- HashMap of character → rasterized bitmap
- Grows as new characters are encountered
- Not bounded (could be improved with LRU)

### Parser State
- Fixed-size buffers for CSI params (max 32)
- Bounded string buffers for OSC/DCS (max 64KB)

## Error Handling

### Parser Errors
- Invalid UTF-8: Emit replacement character (U+FFFD)
- Invalid escape sequences: Consume and ignore
- Truncated sequences: Wait for more input

### PTY Errors
- Read errors: Log and continue
- Write errors: Log and continue
- Child exit: Notify GUI to close

### Rendering Errors
- Font loading failure: Fatal error
- Surface errors: Attempt recovery

## Security Considerations

See [security.md](security.md) for detailed security documentation.

Key points:
- OSC 52 clipboard disabled by default
- Bounded buffer sizes prevent DoS
- No automatic URL opening
- Careful handling of untrusted input

## Testing Strategy

### Unit Tests
- Core module: Screen operations, cursor movement, scrolling
- Parser module: Sequence parsing, chunk boundaries, UTF-8

### Golden Tests
- Input bytes → expected screen snapshot
- Stored in `tests/golden/`

### Integration Tests
- PTY spawning and communication
- End-to-end with real shell

### Fuzz Testing
- Parser fuzzing with arbitrary bytes
- Invariant: no panics, bounded memory
