# Mochi Terminal Phase 2 - Architecture Notes

## Overview

This document describes the architecture of the Mochi terminal emulator as understood during Phase 2 development. The terminal is built as a Rust workspace with four crates that maintain clear separation of concerns.

## Crate Architecture

```
terminal/
├── Cargo.toml          # Workspace definition
├── terminal-core/      # Screen model, grid, cells, selection
├── terminal-parser/    # VT/xterm escape sequence parsing
├── terminal-pty/       # PTY management and child process handling
└── mochi-term/         # GUI application (winit + softbuffer + fontdue)
```

### terminal-core (Pure Data Model)

This crate contains the terminal's screen model with no I/O dependencies.

**Key Types:**
- `Cell` - Single character cell with attributes (char, fg/bg color, bold, italic, etc.)
- `CellAttributes` - Styling information (colors, bold, italic, underline, etc.)
- `Line` - Row of cells with wrap flag
- `Grid` - 2D array of lines representing visible screen
- `Screen` - Primary/alternate grids, cursor, modes, scrollback, selection
- `Scrollback` - Ring buffer of historical lines
- `Selection` - Text selection state (Normal, Word, Line, Block types)
- `Cursor` - Position and style information
- `Modes` - Terminal mode flags (insert, origin, autowrap, mouse tracking, etc.)
- `Color` - Color representation (Default, Indexed, Rgb)
- `Charset` / `CharsetState` - Character set translation (ASCII, DEC Special Graphics)

**Key Interfaces:**
- `Screen::print(char)` - Write character at cursor
- `Screen::scroll_up/down()` - Scroll within region
- `Screen::resize(cols, rows)` - Handle terminal resize
- `Screen::enter/exit_alternate_screen()` - Alternate buffer support
- `Selection::start/update/finish/clear()` - Selection management

### terminal-parser (Escape Sequence Parser)

State machine parser for VT/xterm escape sequences.

**Key Types:**
- `Parser` - Main state machine
- `ParserState` - Current parser state (Ground, Escape, CsiEntry, OscString, etc.)
- `Action` - Parsed action (Print, Control, Esc, Csi, Osc, etc.)
- `CsiAction` - CSI sequence with params, intermediates, final byte
- `OscAction` - OSC command (SetTitle, Hyperlink, Clipboard, etc.)
- `EscAction` - Simple escape sequences (SaveCursor, Index, etc.)
- `Params` - Parameter buffer for CSI sequences

**Key Interfaces:**
- `Parser::parse(byte) -> Option<Action>` - Feed byte, get action
- `Parser::parse_collect(bytes) -> Vec<Action>` - Parse multiple bytes

### terminal-pty (PTY Management)

Platform-specific PTY creation and child process management.

**Key Types:**
- `Pty` - Raw PTY file descriptor wrapper
- `Child` - Spawned shell process with PTY
- `WindowSize` - Terminal dimensions (cols, rows, pixel width/height)

**Key Interfaces:**
- `Child::spawn(shell, size)` - Spawn shell in PTY
- `Child::read(buf)` - Read from PTY (non-blocking)
- `Child::write_all(data)` - Write to PTY
- `Child::resize(size)` - Send TIOCSWINSZ

### mochi-term (GUI Application)

The main application crate that ties everything together.

**Key Types:**
- `App` - Main application state and event loop
- `Terminal` - Combines Screen + Parser, processes PTY output
- `Renderer` - CPU-based rendering with fontdue
- `Config` - Application configuration (font, theme, scrollback, etc.)
- `ColorScheme` - Theme colors (fg, bg, cursor, selection, ANSI 16)

**Key Files:**
- `main.rs` - Entry point, loads config, runs App
- `app.rs` - Event loop, input handling, window management
- `terminal.rs` - Terminal state, escape sequence handling
- `renderer.rs` - Font rasterization, cell rendering
- `config.rs` - Configuration loading and theme definitions
- `input.rs` - Keyboard/mouse encoding for PTY

## Data Flow

```
User Input (keyboard/mouse)
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│  App (mochi-term)                                           │
│  ┌─────────────────┐    ┌─────────────────┐                │
│  │ Input Handler   │───▶│ PTY Child       │                │
│  │ (encode_key)    │    │ (write_all)     │                │
│  └─────────────────┘    └────────┬────────┘                │
│                                  │                          │
│                                  ▼                          │
│                         ┌─────────────────┐                │
│                         │ PTY Child       │                │
│                         │ (read)          │                │
│                         └────────┬────────┘                │
│                                  │                          │
│                                  ▼                          │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Terminal                                             │   │
│  │  ┌─────────────┐    ┌─────────────┐                 │   │
│  │  │ Parser      │───▶│ Screen      │                 │   │
│  │  │ (parse)     │    │ (handle_*)  │                 │   │
│  │  └─────────────┘    └─────────────┘                 │   │
│  └─────────────────────────────────────────────────────┘   │
│                                  │                          │
│                                  ▼                          │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Renderer                                             │   │
│  │  - Rasterize glyphs (fontdue)                       │   │
│  │  - Render cells to buffer (softbuffer)              │   │
│  │  - Present to window (winit)                        │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Current Configuration System

The existing `Config` struct in `mochi-term/src/config.rs` already has:
- Font settings (family, size)
- Scrollback lines
- Window dimensions
- Theme selection (ThemeName enum)
- Color scheme (ColorScheme struct with ANSI 16 + special colors)
- OSC 52 clipboard settings (enabled flag, max size)
- Shell command
- Cursor style and blink

**Current Loading:**
- Reads from `~/.config/mochi/config.toml` via `dirs::config_dir()`
- Falls back to defaults if file doesn't exist
- No CLI override support yet
- No environment variable support yet

## Phase 2 Integration Points

### M1 - Config System Enhancement
- Add CLI argument parsing (clap) to `main.rs`
- Add environment variable support to `Config::load()`
- Implement config precedence: CLI > env > file > defaults
- Add validation with detailed error messages

### M2 - Theme System Enhancement
- Existing themes: Dark, Light, SolarizedDark, SolarizedLight, Dracula, Nord
- Need: Runtime theme switching via keybinding
- Need: Theme file loading from custom paths

### M3 - Font Customization
- Existing: font_family, font_size in Config
- Existing: Renderer::set_font_size() with cell recalculation
- Need: Font fallback chain
- Need: Cell padding / line height configuration
- Need: Runtime font reload

### M4 - Keybinding System
- **CRITICAL BUG**: `handle_paste()` in app.rs is dead code (never called)
- Need: Keybinding configuration in config file
- Need: Default keybindings (Ctrl+Shift+C/V/F/R/T)
- Need: Keybinding dispatch in event handler

### M5 - UX Polish
- Existing: Selection with Normal, Word, Line, Block types
- Need: Double-click word selection, triple-click line selection
- Need: Scrollback search UI overlay
- Need: Hyperlink hover/click handling

### M6 - Config Reload + Security
- Need: Runtime config reload via keybinding
- Need: Optional file watcher (inotify)
- Existing: OSC 52 clipboard with enable flag and max size
- Need: Title update throttling

## Key Observations

1. **Paste Bug**: The `handle_paste()` function exists but is marked `#[allow(dead_code)]` and never called. This is why paste doesn't work.

2. **Theme System**: Already has 6 built-in themes with full ANSI 16 palette support.

3. **Font Handling**: Uses fontdue for rasterization with glyph caching. Cell size is computed from font metrics.

4. **Rendering**: CPU-based rendering via softbuffer. No GPU acceleration.

5. **Selection**: Supports multiple selection types but click detection for word/line selection needs implementation.

6. **Mouse Handling**: Supports X10, VT200, button event, any event, and SGR mouse modes.

7. **Bracketed Paste**: Already implemented in `input.rs` via `encode_bracketed_paste()`.
