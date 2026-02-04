# Mochi Terminal Architecture

This document describes the architecture of the Mochi terminal emulator.

## Module Overview

### 1. terminal-core (Platform Independent)

The core module contains all terminal state logic with no platform dependencies.

#### Cell (`core/cell.rs`)
Represents a single character cell in the terminal grid.

```rust
struct Cell {
    // The character(s) in this cell - may be empty, single char, or grapheme cluster
    content: String,
    // Foreground color
    fg: Color,
    // Background color  
    bg: Color,
    // Text attributes
    attrs: CellAttributes,
    // Hyperlink ID (for OSC 8), 0 = no link
    hyperlink_id: u32,
}

struct CellAttributes {
    bold: bool,
    faint: bool,
    italic: bool,
    underline: bool,
    blink: bool,
    inverse: bool,
    hidden: bool,
    strikethrough: bool,
}

enum Color {
    Default,
    Indexed(u8),      // 0-255 palette
    Rgb(u8, u8, u8),  // True color
}
```

#### Line (`core/line.rs`)
A row of cells with metadata.

```rust
struct Line {
    cells: Vec<Cell>,
    // True if this line was soft-wrapped from the previous line
    wrapped: bool,
}
```

#### Screen (`core/screen.rs`)
The visible terminal grid plus all state.

```rust
struct Screen {
    // Primary screen buffer
    primary: Grid,
    // Alternate screen buffer (for full-screen apps)
    alternate: Grid,
    // Which buffer is active
    active_buffer: BufferType,
    // Scrollback history (primary screen only)
    scrollback: Scrollback,
    // Cursor state
    cursor: Cursor,
    // Saved cursor (for DECSC/DECRC)
    saved_cursor: Option<Cursor>,
    // Saved cursor for alternate screen
    saved_cursor_alt: Option<Cursor>,
    // Terminal modes
    modes: Modes,
    // Current SGR attributes for new characters
    current_attrs: CellAttributes,
    current_fg: Color,
    current_bg: Color,
    // Tab stops
    tab_stops: Vec<bool>,
    // Scroll region (top, bottom) - 0-indexed, inclusive
    scroll_region: (usize, usize),
    // Terminal dimensions
    cols: usize,
    rows: usize,
}

struct Cursor {
    row: usize,
    col: usize,
    visible: bool,
    style: CursorStyle,
}

enum CursorStyle {
    Block,
    Underline,
    Bar,
}

struct Modes {
    // DECAWM - Auto wrap mode
    autowrap: bool,
    // DECOM - Origin mode (cursor relative to scroll region)
    origin: bool,
    // IRM - Insert/Replace mode
    insert: bool,
    // LNM - Line feed/new line mode
    linefeed_newline: bool,
    // DECCKM - Cursor key mode (application vs normal)
    cursor_keys_application: bool,
    // DECKPAM/DECKPNM - Keypad mode
    keypad_application: bool,
    // Bracketed paste mode
    bracketed_paste: bool,
    // Mouse reporting modes
    mouse_mode: MouseMode,
    // Focus reporting
    focus_reporting: bool,
    // Alternate screen active
    alternate_screen: bool,
}

enum MouseMode {
    None,
    X10,           // Button press only
    Normal,        // Button press and release
    ButtonMotion,  // Press, release, and motion while pressed
    AnyMotion,     // All motion events
}
```

#### Scrollback (`core/scrollback.rs`)
Ring buffer for terminal history.

```rust
struct Scrollback {
    lines: VecDeque<Line>,
    max_lines: usize,
}
```

### 2. terminal-parser (Platform Independent)

Stateful parser converting bytes to terminal actions.

#### State Machine (`parser/state.rs`)

Based on the VT500-series parser model from https://vt100.net/emu/dec_ansi_parser

States:
- `Ground` - Normal character processing
- `Escape` - After ESC, waiting for next byte
- `EscapeIntermediate` - ESC followed by intermediate byte(s)
- `CsiEntry` - After CSI (ESC [), collecting parameters
- `CsiParam` - Collecting CSI parameters
- `CsiIntermediate` - CSI with intermediate byte(s)
- `CsiIgnore` - Invalid CSI, consuming until final byte
- `OscString` - Collecting OSC payload
- `DcsEntry` - After DCS
- `DcsParam` - DCS parameters
- `DcsIntermediate` - DCS intermediate
- `DcsPassthrough` - DCS data
- `DcsIgnore` - Invalid DCS
- `SosPmApcString` - SOS/PM/APC string (consumed and ignored)

#### Actions (`parser/actions.rs`)

```rust
enum TerminalAction {
    // Print a character to the screen
    Print(char),
    // Execute a C0 control character
    Execute(u8),
    // CSI sequence with parameters
    CsiDispatch {
        params: Vec<u16>,
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    // OSC sequence
    OscDispatch {
        params: Vec<Vec<u8>>,
    },
    // ESC sequence (non-CSI)
    EscDispatch {
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    // DCS sequence
    DcsDispatch {
        params: Vec<u16>,
        intermediates: Vec<u8>,
        final_byte: u8,
        data: Vec<u8>,
    },
    // Hook for DCS start
    DcsHook {
        params: Vec<u16>,
        intermediates: Vec<u8>,
        final_byte: u8,
    },
    // DCS data byte
    DcsPut(u8),
    // DCS end
    DcsUnhook,
}
```

### 3. PTY Module (Linux Specific)

Handles pseudoterminal creation and child process management.

```rust
struct Pty {
    master_fd: RawFd,
    child_pid: Pid,
}

impl Pty {
    // Create PTY and spawn shell
    fn spawn(shell: &str, size: (u16, u16)) -> Result<Self>;
    
    // Read from PTY (non-blocking)
    fn read(&self, buf: &mut [u8]) -> Result<usize>;
    
    // Write to PTY
    fn write(&self, data: &[u8]) -> Result<usize>;
    
    // Resize PTY
    fn resize(&self, cols: u16, rows: u16) -> Result<()>;
    
    // Check if child is still running
    fn is_alive(&self) -> bool;
}
```

PTY creation sequence:
1. `posix_openpt(O_RDWR | O_NOCTTY)` - Open master
2. `grantpt(master_fd)` - Grant access to slave
3. `unlockpt(master_fd)` - Unlock slave
4. `ptsname(master_fd)` - Get slave device path
5. `fork()` - Create child process
6. In child:
   - `setsid()` - Create new session
   - Open slave device (becomes controlling terminal)
   - `dup2()` to stdin/stdout/stderr
   - `exec()` the shell
7. In parent:
   - Set master to non-blocking
   - Return Pty handle

### 4. Frontend (GUI)

#### Window (`frontend/window.rs`)
Window creation and event loop using winit.

#### Renderer (`frontend/renderer.rs`)
GPU rendering using wgpu.

Rendering pipeline:
1. Build glyph atlas from font
2. For each visible cell:
   - Draw background quad with bg color
   - Draw glyph from atlas with fg color
   - Apply underline/strikethrough if needed
3. Draw cursor overlay
4. Draw selection overlay

#### Input (`frontend/input.rs`)
Keyboard and mouse input encoding.

Keyboard encoding follows xterm conventions:
- Normal keys: UTF-8 bytes
- Arrow keys: `ESC [ A/B/C/D` (normal) or `ESC O A/B/C/D` (application)
- Function keys: `ESC [ 11~` through `ESC [ 24~`
- Modifiers affect encoding (shift, alt, ctrl)

Mouse encoding (SGR 1006 mode):
- Press: `ESC [ < button ; x ; y M`
- Release: `ESC [ < button ; x ; y m`

### 5. Application Glue

#### Config (`app/config.rs`)
Configuration loading and defaults.

```rust
struct Config {
    font_family: String,
    font_size: f32,
    scrollback_lines: usize,
    // Color palette (16 base + 240 extended)
    colors: [Color; 256],
    // Security settings
    allow_osc52_read: bool,
    allow_osc52_write: bool,
    osc52_max_bytes: usize,
}
```

## Data Flow

### Input Flow
```
Keyboard/Mouse Event
       │
       ▼
   Input Encoder
       │
       ▼
   Byte Sequence
       │
       ▼
   PTY Master (write)
       │
       ▼
   Child Process
```

### Output Flow
```
Child Process
       │
       ▼
   PTY Master (read)
       │
       ▼
   Byte Buffer
       │
       ▼
   Parser (bytes → actions)
       │
       ▼
   Screen (apply actions)
       │
       ▼
   Renderer (draw snapshot)
       │
       ▼
   Display
```

## Threading Model

Single-threaded event loop with non-blocking I/O:

```
┌─────────────────────────────────────────┐
│              Event Loop                 │
│  ┌─────────────────────────────────┐   │
│  │  1. Poll PTY for output         │   │
│  │  2. Parse and apply to screen   │   │
│  │  3. Poll window for input       │   │
│  │  4. Encode and write to PTY     │   │
│  │  5. Render if damaged           │   │
│  │  6. Repeat                      │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

## Determinism

The terminal-core and terminal-parser modules are fully deterministic:
- Given the same byte sequence, they produce identical screen state
- No use of system time, random numbers, or external state
- This enables reliable snapshot testing

## Error Handling

- Invalid escape sequences: Consume and ignore, log at debug level
- Invalid UTF-8: Replace with U+FFFD (replacement character)
- PTY errors: Propagate to application for handling
- Render errors: Log and attempt recovery
