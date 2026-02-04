# Mochi Terminal Architecture Notes - Phase 2

This document describes the current architecture of the Mochi terminal emulator as understood during Phase 2 planning.

## Crate Structure

The terminal is organized into four crates with clear separation of concerns:

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

## terminal-core

Platform-independent terminal state model. Key types:

### Cell & CellAttributes
- `Cell`: Single character cell with content, attributes, width, hyperlink_id
- `CellAttributes`: fg/bg color, bold, faint, italic, underline, blink, inverse, hidden, strikethrough

### Color
- `Color` enum: Default, Indexed(u8), Rgb{r,g,b}
- Supports 16 standard ANSI, 256-color palette, and 24-bit true color

### Cursor & CursorStyle
- `Cursor`: row, col, visible, blinking, style, attrs, pending_wrap, origin_mode, hyperlink_id
- `CursorStyle`: Block, Underline, Bar

### Grid & Line
- `Grid`: 2D array of lines with scroll operations
- `Line`: Row of cells with wrap flag, insert/delete operations

### Screen
- Primary and alternate grids
- Cursor state and saved cursor
- Scroll region
- Tab stops
- Selection state
- Hyperlink registry
- Character set state

### Scrollback
- Ring buffer of lines that scrolled off top
- Configurable max size (default 10,000 lines)

### Selection
- Start/end points
- Selection type (Normal, Block, Line)
- Contains check for rendering

### Modes
- cursor_keys_application, auto_wrap, origin_mode, linefeed_mode
- insert_mode, cursor_visible, bracketed_paste, focus_events
- mouse_vt200, mouse_button_event, mouse_any_event, mouse_sgr
- alternate_screen

## terminal-parser

VT/xterm escape sequence parser. Key types:

### Parser
State machine with states: Ground, Escape, EscapeIntermediate, CsiEntry, CsiParam, CsiIntermediate, OscString, DcsEntry, DcsParam, DcsIntermediate, DcsPassthrough

### Action
- Print(char)
- Control(byte)
- Esc(EscAction)
- Csi(CsiAction)
- Osc(OscAction)
- Dcs, Apc, Pm, Sos
- Invalid(Vec<u8>)

### CsiAction
- params: Params
- intermediates: Vec<u8>
- final_byte: u8
- private: bool

### OscAction
- SetTitle(String)
- SetIconName(String)
- Hyperlink { id, uri }
- Clipboard { selection, data }
- Unknown

## terminal-pty

Linux PTY management:

### Pty
- PTY master file descriptor
- Window size control (TIOCSWINSZ)
- Non-blocking I/O

### Child
- Fork/exec with proper session setup
- Signal handling
- Read/write to child

### WindowSize
- cols, rows, pixel_width, pixel_height

## mochi-term

GUI application tying everything together:

### App
- Window management (winit)
- Event loop
- Terminal instance
- Renderer
- Clipboard (arboard)
- Mouse state
- Scroll offset

### Terminal
- Combines Parser and Screen
- Processes PTY output through parser
- Applies actions to screen
- Handles title changes, bell

### Renderer
- CPU-based rendering (softbuffer + fontdue)
- Bundled fonts: DejaVuSansMono, DejaVuSansMono-Bold
- Glyph caching
- Cell rendering with attributes
- Cursor and selection overlay
- Scrollbar rendering

### Config
Current config structure:
- font_family, font_size
- scrollback_lines
- dimensions (cols, rows)
- theme (ThemeName enum)
- colors (ColorScheme)
- osc52_clipboard, osc52_max_size
- shell
- cursor_style, cursor_blink

### ColorScheme
- foreground, background, cursor, selection (hex strings)
- ansi[16] (hex strings)
- Built-in themes: dark, light, solarized_dark, solarized_light, dracula, nord

### Input
- Key encoding (encode_key)
- Mouse encoding (encode_mouse)
- Bracketed paste encoding
- Focus event encoding

## Data Flow

### Input (keyboard/mouse to child)
```
User Input → winit Event → encode_key() → PTY write → Child Process
```

### Output (child to screen)
```
Child Process → PTY read → Parser → Actions → Screen → Renderer → Display
```

## Phase 2 Integration Points

### Config System (M1)
- Extend Config struct with CLI args support
- Add config validation with error messages
- Implement XDG config path with override

### Themes (M2)
- ColorScheme already has built-in themes
- Need runtime theme switching via keybinding
- Renderer.colors needs to be mutable for theme changes

### Font Customization (M3)
- Renderer currently uses bundled fonts only
- Need font discovery/loading from system
- set_font_size exists but need full font family support
- Cell size recalculation already implemented

### Keybindings (M4)
- App.handle_key_input currently handles zoom shortcuts
- Need keybinding config parsing
- Need action dispatch system

### UX Polish (M5)
- Selection exists but needs word/line selection
- Need search UI overlay
- Hyperlinks exist in Screen but need better UX

### Config Reload (M6)
- Config.load() exists
- Need file watcher or keybinding trigger
- Need error handling for reload failures

## Test Coverage

Current tests (139 total):
- terminal-core: 76 tests
- terminal-parser: 33 tests
- terminal-pty: 11 tests
- mochi-term: 19 tests

All tests pass as of Phase 2 baseline.
