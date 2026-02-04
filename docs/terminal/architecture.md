# Architecture

This document describes the architecture of the Mochi terminal emulator.

## Overview

The terminal emulator is organized into four crates with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────────┐
│                       mochi-term                            │
│  (GUI application: window, rendering, input, config)        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐ │
│  │  terminal-core  │  │ terminal-parser │  │ terminal-pty│ │
│  │  (screen model) │  │ (escape parser) │  │ (PTY mgmt)  │ │
│  └─────────────────┘  └─────────────────┘  └─────────────┘ │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Crate Responsibilities

### terminal-core

The core crate contains the platform-independent terminal state model:

**Cell**: A single character cell containing:
- Unicode content (grapheme cluster)
- Display attributes (colors, bold, italic, etc.)
- Width (1 for normal, 2 for wide, 0 for continuation)
- Hyperlink ID

**Line**: A row of cells with:
- Fixed column count
- Wrap flag (soft-wrapped from previous line)
- Insert/delete operations

**Grid**: The visible screen area:
- 2D array of lines
- Scroll operations within regions
- Clear operations

**Screen**: Complete terminal state:
- Primary and alternate grids
- Cursor state (position, style, visibility, attributes)
- Scroll region
- Mode flags
- Scrollback buffer
- Selection state
- Tab stops

**Scrollback**: Ring buffer of lines that have scrolled off the top.

**Selection**: Text selection state for copy operations.

**Snapshot**: Serializable representation of terminal state for testing.

### terminal-parser

The parser crate handles escape sequence parsing:

**Parser**: State machine that processes bytes and emits actions:
- Ground state (normal text)
- Escape state (ESC received)
- CSI state (control sequence)
- OSC state (operating system command)
- DCS state (device control string)

**Action**: Semantic operations emitted by the parser:
- Print(char)
- Control(byte)
- Csi(CsiAction)
- Esc(EscAction)
- Osc(OscAction)

**Utf8Decoder**: Streaming UTF-8 decoder with proper error handling.

**Params**: CSI parameter parsing with subparameter support.

Key design decisions:
- Streaming: Parser handles arbitrary chunk boundaries
- Deterministic: Same input always produces same output
- No allocations in hot path (parameters stored in fixed buffer)

### terminal-pty

The PTY crate handles Linux pseudoterminal management:

**Pty**: PTY master file descriptor:
- Creation via posix_openpt/grantpt/unlockpt
- Window size control (TIOCSWINSZ)
- Non-blocking I/O

**Child**: Child process attached to PTY:
- Fork/exec with proper session setup
- Signal handling (SIGWINCH, SIGHUP)
- Read/write to child

### mochi-term

The application crate ties everything together:

**App**: Main application state:
- Window management (winit)
- Event loop
- Terminal instance
- Renderer
- Clipboard

**Terminal**: Combines parser and screen:
- Processes PTY output through parser
- Applies actions to screen
- Handles title changes, bell, etc.

**Renderer**: CPU-based rendering:
- Font rasterization (fontdue)
- Glyph caching
- Cell rendering with attributes
- Cursor and selection overlay

**Config**: Configuration system:
- Font settings
- Colors
- Terminal options

## Data Flow

### Input (keyboard/mouse to child)

```
User Input → winit Event → encode_key() → PTY write → Child Process
```

1. winit delivers keyboard/mouse events
2. Input encoder converts to terminal sequences
3. Sequences written to PTY master
4. Child process receives on stdin

### Output (child to screen)

```
Child Process → PTY read → Parser → Actions → Screen → Renderer → Display
```

1. Child writes to stdout/stderr
2. PTY master receives data
3. Parser processes bytes into actions
4. Actions applied to screen model
5. Renderer draws screen to window

## Threading Model

Currently single-threaded:
- Event loop handles window events
- PTY polling integrated into event loop
- Rendering happens synchronously

Future optimization: Move PTY I/O to separate thread.

## Memory Management

**Scrollback**: Bounded ring buffer prevents unbounded growth. Default 10,000 lines.

**Glyph Cache**: HashMap of rendered glyphs. Could be bounded with LRU eviction.

**Parser Buffers**: Fixed-size buffers for parameters and strings.

## Testing Strategy

**Unit Tests**: Each crate has comprehensive unit tests for individual components.

**Snapshot Tests**: Parser and screen operations tested with deterministic snapshots.

**Integration Tests**: PTY tests spawn real processes and verify behavior.

**Fuzzing**: Parser designed to handle arbitrary input without panicking.
