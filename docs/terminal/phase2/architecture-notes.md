# Mochi Terminal Architecture Notes

This document describes the architecture of the Mochi Terminal emulator as of Phase 2 development.

## Overview

Mochi Terminal is a VT/xterm-compatible terminal emulator built from scratch in Rust. It does not use any terminal emulation libraries - all parsing, screen management, and rendering are implemented directly.

## Crate Structure

The terminal is organized into four crates with clear separation of concerns:

```
terminal/
├── mochi-term/       # Main application (GUI, config, rendering)
├── terminal-core/    # Platform-independent terminal state
├── terminal-parser/  # VT/xterm escape sequence parser
└── terminal-pty/     # Linux PTY management
```

### terminal-core (Platform-Independent State)

This crate provides the core data structures for terminal emulation. It is designed to be deterministic - given the same sequence of operations, it produces the same screen state.

Key types:
- `Screen` - Main interface tying together grid, cursor, scrollback, modes, and selection
- `Grid` - 2D array of cells representing visible terminal content
- `Line` - Single row of cells with wrap tracking
- `Cell` - Individual character with attributes (colors, bold, italic, etc.)
- `CellAttributes` - Text styling (fg/bg color, bold, italic, underline, etc.)
- `Cursor` - Position, style, visibility, and pending attributes
- `Scrollback` - Ring buffer of historical lines
- `Selection` - Text selection state (Normal, Word, Line, Block types)
- `Modes` - Terminal mode flags (insert, autowrap, mouse tracking, etc.)
- `Color` - Color representation (Default, Indexed 0-255, RGB)
- `Dimensions` - Terminal size (cols, rows)
- `CharsetState` - G0-G3 character set designation and shifting

### terminal-parser (Escape Sequence Parser)

Streaming parser that converts byte input into semantic terminal actions. Handles arbitrary chunk boundaries and supports UTF-8.

Key types:
- `Parser` - State machine for parsing escape sequences
- `ParserState` - Current parser state (Ground, Escape, CSI, OSC, etc.)
- `Action` - Parsed action enum (Print, Control, Esc, Csi, Osc, Dcs, etc.)
- `CsiAction` - CSI sequence with params, intermediates, final byte
- `EscAction` - ESC sequence actions (SaveCursor, Index, DesignateG0, etc.)
- `OscAction` - OSC sequence actions (SetTitle, SetHyperlink, etc.)
- `Params` - Parameter iterator for CSI sequences

Supported sequences:
- C0 control characters (BEL, BS, HT, LF, CR, etc.)
- ESC sequences (DECSC, DECRC, IND, RI, NEL, HTS, RIS, charset designation)
- CSI sequences (cursor movement, erase, scroll, SGR, modes, etc.)
- OSC sequences (title, hyperlinks)
- DCS sequences (parsed but not fully implemented)

### terminal-pty (PTY Management)

Linux-specific PTY functionality for spawning and managing child processes.

Key types:
- `Pty` - Pseudoterminal file descriptor management
- `Child` - Child process with PTY attachment
- `WindowSize` - Terminal dimensions for TIOCSWINSZ
- `Error` / `Result` - PTY-specific error handling

Features:
- PTY creation via posix_openpt/grantpt/unlockpt
- Child process spawning with proper session setup
- Non-blocking I/O
- Window size management (TIOCSWINSZ)

### mochi-term (Main Application)

The GUI application that ties everything together.

Key modules:
- `main.rs` - Entry point, logging setup, config loading
- `app.rs` - Application state, event loop, window management
- `terminal.rs` - Terminal state integrating parser and screen
- `renderer.rs` - CPU-based rendering with softbuffer and fontdue
- `config.rs` - Configuration loading/saving, themes, color schemes
- `input.rs` - Keyboard/mouse input encoding to escape sequences
- `event.rs` - Terminal event types

## Data Flow

```
User Input (keyboard/mouse)
    │
    ▼
┌─────────────────┐
│   App (winit)   │ ◄── Window events, resize, focus
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  input.rs       │ ◄── Encode to escape sequences
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   PTY (write)   │ ◄── Send to child process
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Shell/Program  │ ◄── Child process
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   PTY (read)    │ ◄── Read output
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│     Parser      │ ◄── Parse escape sequences
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│    Terminal     │ ◄── Handle actions, update screen
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│     Screen      │ ◄── Grid, cursor, scrollback, modes
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│    Renderer     │ ◄── Rasterize glyphs, composite
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   softbuffer    │ ◄── Present to window
└─────────────────┘
```

## Key Data Types

### Cell and Attributes

```rust
struct Cell {
    c: char,           // Character (or '\0' for empty)
    attrs: CellAttributes,
    hyperlink_id: u32, // 0 = no hyperlink
}

struct CellAttributes {
    fg: Color,
    bg: Color,
    bold: bool,
    faint: bool,
    italic: bool,
    underline: bool,
    blink: bool,
    inverse: bool,
    hidden: bool,
    strikethrough: bool,
}
```

### Color

```rust
enum Color {
    Default,           // Use theme default
    Indexed(u8),       // 0-15 ANSI, 16-255 extended
    Rgb { r, g, b },   // True color
}
```

### Screen State

```rust
struct Screen {
    primary_grid: Grid,
    alternate_grid: Grid,
    using_alternate: bool,
    scrollback: Scrollback,
    cursor: Cursor,
    saved_cursor_primary: SavedCursor,
    saved_cursor_alternate: SavedCursor,
    modes: Modes,
    scroll_region: Option<(usize, usize)>,
    tab_stops: Vec<bool>,
    selection: Selection,
    title: String,
    hyperlinks: Vec<String>,
    charset: CharsetState,
}
```

### Terminal Modes

```rust
struct Modes {
    insert_mode: bool,
    linefeed_mode: bool,
    cursor_keys_application: bool,
    auto_wrap: bool,
    cursor_visible: bool,
    mouse_vt200: bool,
    mouse_button_event: bool,
    mouse_any_event: bool,
    mouse_sgr: bool,
    focus_events: bool,
    alternate_screen: bool,
    bracketed_paste: bool,
    // ... more
}
```

## Configuration System (Current State)

The current config system uses TOML and supports:
- Font family and size
- Scrollback lines
- Window dimensions
- Theme selection (Dark, Light, SolarizedDark, SolarizedLight, Dracula, Nord, Custom)
- Color scheme (16 ANSI colors + fg/bg/cursor/selection)
- OSC 52 clipboard settings (disabled by default)
- Shell command
- Cursor style and blink

Config location: `~/.config/mochi/config.toml` (via dirs crate)

## Rendering Pipeline

1. **Glyph Caching**: Characters are rasterized once using fontdue and cached
2. **Cell Iteration**: Iterate over visible cells (including scrollback if scrolled)
3. **Color Resolution**: Resolve Color enum to RGB using theme
4. **Background Fill**: Fill cell rectangle with background color
5. **Glyph Drawing**: Alpha-blend glyph bitmap onto background
6. **Selection Overlay**: Apply selection highlighting
7. **Cursor Overlay**: Draw cursor at current position
8. **Scrollbar**: Draw scrollbar if scrollback exists
9. **Present**: Copy buffer to window via softbuffer

## Where to Add Phase 2 Features

### Config System (M1)
- Extend `config.rs` with CLI argument parsing (clap)
- Add environment variable support
- Implement config validation with clear error messages
- Add precedence logic (CLI > env > file > defaults)

### Themes (M2)
- Theme definitions already exist in `config.rs` (ColorScheme)
- Add runtime theme switching in `app.rs`
- Add keybinding for theme toggle
- Renderer already accepts ColorScheme, just needs method to update it

### Font Customization (M3)
- Font loading already in `renderer.rs`
- Add font family selection (fontconfig or bundled fonts)
- Add fallback font chain
- Cell padding/line height adjustments in CellSize calculation
- PTY resize already handled in `app.rs` change_font_size

### Keybindings (M4)
- Add keybinding config to `config.rs`
- Add keybinding parser and action mapper
- Modify `handle_key_input` in `app.rs` to check custom bindings first
- Reserve Ctrl+Shift+* combinations for app shortcuts

### UX Polish (M5)
- Selection types defined in `terminal-core/selection.rs`
- Add word/line selection logic based on click count
- Add search UI overlay in renderer
- Add search state to App
- Hyperlink rendering already partially supported (hyperlink_id in Cell)

### Config Reload (M6)
- Add reload keybinding handler in `app.rs`
- Add file watcher (notify crate) for auto-reload
- Add error display mechanism (toast/status line)
- Security: OSC 52 already has osc52_clipboard and osc52_max_size settings

## Dependencies

Current dependencies (relevant to Phase 2):
- `winit 0.29` - Windowing and event loop
- `softbuffer 0.4` - CPU-based window surface
- `fontdue 0.9` - Font rasterization
- `arboard 3.4` - Clipboard access
- `toml 0.8` - Config parsing
- `serde 1.0` - Serialization
- `dirs 5.0` - XDG directory paths
- `env_logger 0.11` - Logging
- `unicode-width 0.1` - Character width calculation
- `nix 0.29` - Unix system calls (PTY)

Potential additions for Phase 2:
- `clap` - CLI argument parsing
- `notify` - File watching (optional)

## Test Coverage

Current test counts:
- mochi-term: 19 tests (config, terminal, input)
- terminal-core: 76 tests (screen, grid, cell, cursor, selection, etc.)
- terminal-parser: 33 tests (parser, params, utf8)
- terminal-pty: 11 tests (pty, child, size)

Total: 139 tests, all passing.
