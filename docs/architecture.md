# Mochi Terminal Architecture

This document describes the architecture of the Mochi terminal emulator.

## Overview

Mochi is structured as a layered system with clear separation between parsing, state management, and rendering:

```
┌─────────────────────────────────────────────────────────────┐
│                      GUI Application                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Window    │  │   Input     │  │     Renderer        │  │
│  │  (winit)    │  │  Encoding   │  │  (font + colors)    │  │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
└─────────┼────────────────┼─────────────────────┼────────────┘
          │                │                     │
          │                │                     │ reads
          │                │                     ▼
┌─────────┼────────────────┼─────────────────────────────────┐
│         │                │         Terminal                 │
│         │                │    ┌─────────────────────┐       │
│         │                │    │      Screen         │       │
│         │                │    │  (cells, cursor,    │       │
│         │                │    │   modes, regions)   │       │
│         │                │    └──────────┬──────────┘       │
│         │                │               │                  │
│         │                │               │ applies          │
│         │                │               ▼                  │
│         │                │    ┌─────────────────────┐       │
│         │                │    │      Parser         │       │
│         │                │    │  (state machine,    │       │
│         │                │    │   UTF-8 decode)     │       │
│         │                │    └──────────┬──────────┘       │
└─────────┼────────────────┼───────────────┼──────────────────┘
          │                │               │
          │                │               │ reads
          │                ▼               ▼
┌─────────┼─────────────────────────────────────────────────┐
│         │                    PTY                           │
│         │         ┌─────────────────────┐                  │
│         │         │   Master FD         │◄─── output ──┐   │
│         │         │   (read/write)      │              │   │
│         │         └─────────────────────┘              │   │
│         │                    │                         │   │
│         │                    │ fork/exec               │   │
│         │                    ▼                         │   │
│         │         ┌─────────────────────┐              │   │
│         └────────►│   Child Process     │──────────────┘   │
│           input   │   (shell/app)       │                  │
│                   └─────────────────────┘                  │
└────────────────────────────────────────────────────────────┘
```

## Module Details

### 1. Core (`src/core/`)

The core module is platform-independent and contains all terminal state management.

#### Cell (`cell.rs`)

Represents a single character cell in the terminal grid:

```rust
pub struct Cell {
    pub character: char,      // Unicode scalar value
    pub style: Style,         // Foreground, background, attributes
    pub width: u8,            // Display width (1 for normal, 2 for wide)
    pub hyperlink_id: Option<u32>,  // OSC 8 hyperlink reference
}

pub struct Style {
    pub foreground: Color,
    pub background: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub faint: bool,
    pub strikethrough: bool,
    pub hidden: bool,
}
```

#### Cursor (`cursor.rs`)

Manages cursor state including position and appearance:

```rust
pub struct Cursor {
    pub row: usize,
    pub col: usize,
    pub visible: bool,
    pub style: CursorStyle,
    pub blinking: bool,
}

pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}
```

#### Screen (`screen.rs`)

The main screen model containing the visible grid and all terminal state:

```rust
pub struct Screen {
    // Grid data
    pub cells: Vec<Vec<Cell>>,
    pub cols: usize,
    pub rows: usize,
    
    // Cursor
    pub cursor: Cursor,
    pub saved_cursor: Option<SavedCursor>,
    
    // Scroll region
    pub scroll_top: usize,
    pub scroll_bottom: usize,
    
    // Tab stops
    pub tab_stops: Vec<bool>,
    
    // Mode flags
    pub modes: TerminalModes,
    
    // Current style for new characters
    pub current_style: Style,
    
    // Alternate screen
    pub alternate: Option<Box<AlternateScreen>>,
}
```

#### Scrollback (`scrollback.rs`)

A bounded ring buffer for scrollback history:

```rust
pub struct Scrollback {
    lines: VecDeque<Line>,
    capacity: usize,
}
```

#### Snapshot (`snapshot.rs`)

Serializable representation of terminal state for testing:

```rust
pub struct Snapshot {
    pub cols: usize,
    pub rows: usize,
    pub cursor: CursorSnapshot,
    pub lines: Vec<LineSnapshot>,
    pub modes: ModesSnapshot,
}
```

### 2. Parser (`src/parser/`)

The parser converts a byte stream into semantic terminal actions.

#### State Machine (`state.rs`)

Implements a VT500-series compatible state machine:

```
States:
  GROUND → Normal character processing
  ESCAPE → After ESC, waiting for sequence type
  ESCAPE_INTERMEDIATE → ESC with intermediate bytes
  CSI_ENTRY → After CSI (ESC [), collecting parameters
  CSI_PARAM → Collecting CSI parameters
  CSI_INTERMEDIATE → CSI with intermediate bytes
  CSI_IGNORE → Ignoring malformed CSI
  OSC_STRING → Collecting OSC payload
  DCS_ENTRY → Device Control String entry
  DCS_PARAM → DCS parameters
  DCS_INTERMEDIATE → DCS intermediate
  DCS_PASSTHROUGH → DCS data
  DCS_IGNORE → Ignoring malformed DCS
```

The parser handles:
- UTF-8 decoding with proper continuation byte handling
- Arbitrary chunk boundaries (streaming)
- All C0 control characters
- ESC sequences (including charset designation)
- CSI sequences with parameters and intermediates
- OSC sequences (terminated by BEL or ST)
- DCS sequences (passthrough)

#### Actions (`actions.rs`)

Semantic actions produced by the parser:

```rust
pub enum Action {
    Print(char),
    Execute(u8),           // C0 control
    CsiDispatch { ... },   // CSI sequence
    EscDispatch { ... },   // ESC sequence
    OscDispatch { ... },   // OSC sequence
    DcsHook { ... },       // DCS start
    DcsPut(u8),            // DCS data
    DcsUnhook,             // DCS end
}
```

### 3. PTY (`src/pty/`)

Linux-specific pseudo-terminal management.

#### Linux PTY (`linux.rs`)

```rust
pub struct Pty {
    master_fd: RawFd,
    child_pid: Pid,
}

impl Pty {
    pub fn spawn(shell: Option<&str>, size: WindowSize, env: &[(&str, &str)]) -> Result<Self>;
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    pub fn write(&mut self, buf: &[u8]) -> Result<usize>;
    pub fn resize(&mut self, size: WindowSize) -> Result<()>;
    pub fn is_child_alive(&self) -> bool;
}
```

The PTY module handles:
- Opening master/slave pair with `posix_openpt`
- Granting and unlocking the slave
- Forking and setting up the child process
- Creating a new session and controlling terminal
- Non-blocking I/O on the master
- Window size changes via `TIOCSWINSZ`
- SIGWINCH signal to child

### 4. Input (`src/input/`)

Encodes keyboard and mouse events into terminal escape sequences.

```rust
// Keyboard encoding
pub fn encode_key(key: Key, mods: Modifiers, app_cursor: bool, app_keypad: bool) -> Vec<u8>;
pub fn encode_char(c: char, mods: Modifiers) -> Vec<u8>;

// Mouse encoding
pub fn encode_mouse(
    button: MouseButton,
    event: MouseEventType,
    col: u16,
    row: u16,
    mods: Modifiers,
    mode: MouseMode,
    encoding: MouseEncoding,
) -> Option<Vec<u8>>;

// Special sequences
pub fn encode_focus(focused: bool) -> Vec<u8>;
pub fn encode_bracketed_paste(start: bool) -> Vec<u8>;
```

### 5. Renderer (`src/renderer/`)

GUI rendering using winit for windowing and fontdue for font rasterization.

#### Font Renderer (`font.rs`)

```rust
pub struct FontRenderer {
    font: Font,
    size: f32,
    cell_width: f32,
    cell_height: f32,
    glyph_cache: HashMap<(char, bool, bool), GlyphBitmap>,
}
```

#### Window (`window.rs`)

```rust
pub struct TerminalWindow {
    config: WindowConfig,
    font: FontRenderer,
    palette: ColorPalette,
}

pub struct ColorPalette {
    colors: [Rgb; 256],  // 16 standard + 216 cube + 24 grayscale
}
```

### 6. Terminal (`src/terminal.rs`)

The main executor that ties everything together:

```rust
pub struct Terminal {
    screen: Screen,
    parser: Parser,
    scrollback: Scrollback,
    title: String,
    hyperlinks: HashMap<u32, String>,
}

impl Terminal {
    pub fn process(&mut self, data: &[u8]);
    pub fn screen(&self) -> &Screen;
    pub fn resize(&mut self, cols: usize, rows: usize);
}
```

## Data Flow

### Output (PTY → Screen)

1. PTY master fd becomes readable
2. Read bytes into buffer
3. Feed bytes to Parser
4. Parser produces Actions
5. Terminal applies Actions to Screen
6. Screen state updated
7. Renderer draws Screen

### Input (Keyboard → PTY)

1. Window receives key event
2. Input encoder converts to escape sequence
3. Write sequence to PTY master fd
4. Child process receives input

### Resize

1. Window resized
2. Calculate new grid dimensions from pixel size
3. Resize Screen (reflow text if needed)
4. Update PTY window size via ioctl
5. Send SIGWINCH to child

## Testing Strategy

### Unit Tests

Each module has comprehensive unit tests:
- Cell: style manipulation, color conversion
- Cursor: movement, bounds checking
- Screen: all operations (print, erase, scroll, etc.)
- Parser: state transitions, chunk boundaries
- Input: encoding correctness

### Golden Tests

Deterministic snapshot tests:
1. Input: byte sequence
2. Process through Terminal
3. Generate Snapshot
4. Compare to expected Snapshot

### Integration Tests

PTY-driven tests:
1. Spawn shell via PTY
2. Send commands
3. Read output
4. Verify screen state

### Fuzzing

Parser fuzzing with arbitrary bytes:
- No panics
- No infinite loops
- Bounded memory usage

## Performance Considerations

### Damage Tracking

The screen tracks which regions have changed to minimize redraw:
- Line-level dirty flags
- Cursor movement tracking
- Scroll optimization

### Glyph Caching

Font renderer caches rasterized glyphs:
- Key: (character, bold, italic)
- Value: bitmap + metrics

### Ring Buffer Scrollback

Scrollback uses a ring buffer to avoid allocation during scroll:
- Fixed capacity
- O(1) push/pop
- Bounded memory

## Security

See [security.md](security.md) for details on:
- OSC 52 clipboard restrictions
- OSC 8 hyperlink handling
- Resource limits
- Input sanitization
