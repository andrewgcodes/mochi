# Mochi Terminal Architecture

## Overview

Mochi Terminal is structured as a Rust workspace with four crates, each with a specific responsibility. This separation ensures:
- Testability (core logic can be tested without GUI)
- Portability (core is platform-independent)
- Maintainability (clear module boundaries)

```
┌─────────────────────────────────────────────────────────────┐
│                      mochi-term (GUI)                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   winit     │  │  renderer   │  │    event_loop       │ │
│  │  (window)   │  │  (drawing)  │  │  (coordination)     │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     mochi-core (State)                      │
│  ┌─────────┐  ┌─────────┐  ┌──────────┐  ┌──────────────┐  │
│  │  Term   │  │ Screen  │  │   Grid   │  │  Scrollback  │  │
│  │ (main)  │  │ (view)  │  │ (cells)  │  │   (history)  │  │
│  └─────────┘  └─────────┘  └──────────┘  └──────────────┘  │
└─────────────────────────────────────────────────────────────┘
        ▲                                           │
        │                                           │
┌───────┴───────┐                          ┌───────▼───────┐
│ mochi-parser  │                          │   mochi-pty   │
│   (parsing)   │                          │    (I/O)      │
│               │                          │               │
│ bytes→actions │                          │ PTY + child   │
└───────────────┘                          └───────────────┘
```

## Crate Details

### mochi-core

The core crate contains all terminal state management. It is completely platform-independent and has no GUI dependencies.

#### Key Types

**Cell** (`cell.rs`)
- Represents a single character cell in the terminal
- Contains: character (String for grapheme clusters), foreground color, background color, flags (bold, italic, etc.), hyperlink ID
- Supports wide characters (CJK) and combining marks

**Line** (`line.rs`)
- A row of cells
- Tracks whether the line was soft-wrapped
- Provides operations: insert, delete, erase, clear

**Grid** (`grid.rs`)
- 2D array of lines
- Handles scrolling within the visible area
- Returns scrolled-off lines for scrollback

**Screen** (`screen.rs`)
- Contains grid, cursor, scroll region, mode flags
- Implements terminal operations (erase, insert/delete, scroll)
- Manages tab stops

**Scrollback** (`scrollback.rs`)
- Ring buffer of historical lines
- Configurable maximum size (default 10,000 lines)
- Preserves cell attributes

**Term** (`term.rs`)
- Main terminal state machine
- Manages primary and alternate screen buffers
- Handles charset designation
- Provides snapshot for rendering

**Snapshot** (`snapshot.rs`)
- Serializable representation of terminal state
- Used for golden tests and debugging
- JSON serialization support

### mochi-parser

The parser converts a byte stream into semantic terminal actions.

#### State Machine

```
                    ┌──────────┐
                    │  Ground  │◄─────────────────┐
                    └────┬─────┘                  │
                         │ ESC                    │
                         ▼                        │
                    ┌──────────┐                  │
              ┌─────│  Escape  │─────┐            │
              │     └────┬─────┘     │            │
              │          │           │            │
         [    │     intermediate     │ ]          │ final
              ▼          ▼           ▼            │
        ┌──────────┐ ┌──────────┐ ┌──────────┐   │
        │ CsiEntry │ │EscInterm │ │ OscString│   │
        └────┬─────┘ └────┬─────┘ └────┬─────┘   │
             │            │            │          │
             ▼            │            │          │
        ┌──────────┐      │            │          │
        │ CsiParam │──────┴────────────┴──────────┘
        └──────────┘
```

#### Actions

- `Print(char)` - Print a character
- `Execute(u8)` - Execute C0 control
- `CsiDispatch` - CSI sequence with params
- `EscDispatch` - ESC sequence
- `OscDispatch` - OSC sequence
- `DcsHook/Put/Unhook` - DCS handling

#### Streaming Support

The parser maintains state between calls, allowing arbitrary chunk boundaries:
```rust
let mut parser = Parser::new();
parser.parse(b"\x1b[");  // Partial CSI
parser.parse(b"5A");     // Complete: cursor up 5
```

### mochi-pty

Linux-specific PTY management.

#### PTY Creation

```rust
let pty = Pty::open()?;           // posix_openpt + grantpt + unlockpt
let slave_path = pty.slave_path(); // ptsname
pty.set_size(WindowSize::new(24, 80))?;
```

#### Child Spawning

```rust
let child = ChildBuilder::new("/bin/bash")?
    .env("TERM", "xterm-256color")
    .size(WindowSize::new(24, 80))
    .spawn()?;
```

The child process setup:
1. Fork
2. Create new session (setsid)
3. Open slave PTY
4. Set as controlling terminal (TIOCSCTTY)
5. Duplicate to stdin/stdout/stderr
6. Execute program

#### Resize Handling

```rust
child.resize(WindowSize::new(30, 100))?;
// Sets TIOCSWINSZ and sends SIGWINCH
```

### mochi-term

The GUI application that ties everything together.

#### Event Loop

```
┌─────────────────────────────────────────────────────┐
│                    Event Loop                        │
│                                                      │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐      │
│  │  Window  │───▶│  Input   │───▶│   PTY    │      │
│  │  Events  │    │ Encoding │    │  Write   │      │
│  └──────────┘    └──────────┘    └──────────┘      │
│                                                      │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐      │
│  │   PTY    │───▶│  Parser  │───▶│   Term   │      │
│  │   Read   │    │          │    │  Update  │      │
│  └──────────┘    └──────────┘    └──────────┘      │
│                                                      │
│  ┌──────────┐    ┌──────────┐                       │
│  │   Term   │───▶│ Renderer │───▶ Display          │
│  │ Snapshot │    │          │                       │
│  └──────────┘    └──────────┘                       │
└─────────────────────────────────────────────────────┘
```

#### Input Encoding

Keyboard input is encoded to terminal sequences:
- Normal keys: UTF-8 bytes
- Ctrl+key: Control characters (Ctrl+C = 0x03)
- Alt+key: ESC prefix (Alt+x = ESC x)
- Arrow keys: CSI sequences (Up = ESC [ A)
- Function keys: CSI sequences (F1 = ESC [ 11 ~)

Mouse input (when enabled):
- X10 encoding: CSI M Cb Cx Cy
- SGR encoding: CSI < Pb ; Px ; Py M

#### Rendering

The renderer:
1. Gets a snapshot from Term
2. Iterates over cells
3. Resolves colors (considering inverse, hidden, etc.)
4. Rasterizes glyphs (with caching)
5. Draws to framebuffer
6. Blits to window

## Data Flow

### Input Path

```
Keyboard/Mouse
      │
      ▼
  winit event
      │
      ▼
  encode_key() / encode_mouse()
      │
      ▼
  PTY write
      │
      ▼
  Child process (shell/app)
```

### Output Path

```
Child process output
      │
      ▼
  PTY read
      │
      ▼
  Parser::parse()
      │
      ▼
  Vec<Action>
      │
      ▼
  Performer::perform()
      │
      ▼
  Term state update
      │
      ▼
  Term::snapshot()
      │
      ▼
  Renderer::render()
      │
      ▼
  Display
```

## Determinism

The terminal core is fully deterministic:
- Same input bytes always produce same state
- No randomness or timing dependencies
- Enables reliable golden tests

This is achieved by:
- Separating I/O (PTY) from state (Term)
- No GUI types in core
- Snapshot serialization for comparison

## Thread Safety

Current design is single-threaded:
- Event loop polls PTY and window events
- All state updates happen on main thread
- Rendering happens on main thread

Future optimization could use:
- Separate thread for PTY I/O
- Separate thread for rendering
- Message passing between threads

## Memory Management

### Scrollback Limits

Scrollback is bounded by `max_lines` (default 10,000):
- Old lines are discarded when limit reached
- Ring buffer avoids memory reallocation

### Glyph Cache

Font renderer caches rasterized glyphs:
- HashMap<char, GlyphInfo>
- Cleared on font change
- Bounded by character set usage

### Parser Buffers

Parser buffers have security limits:
- OSC payload: 64KB max
- DCS payload: 64KB max
- Prevents memory exhaustion from malicious input
