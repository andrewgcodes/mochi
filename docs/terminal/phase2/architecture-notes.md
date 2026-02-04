# Mochi Terminal - Phase 2 Architecture Notes

## Overview

The Mochi terminal emulator is a Rust-based terminal emulator built from scratch without using any terminal emulation libraries. It implements VT/xterm escape sequence parsing, maintains a screen model, and renders to a GUI window on Linux.

## Crate Structure

The project is organized into 4 crates in a Cargo workspace:

```
terminal/
├── Cargo.toml          # Workspace root
├── terminal-core/      # Platform-independent screen model
├── terminal-parser/    # VT/xterm escape sequence parser
├── terminal-pty/       # Linux PTY management
└── mochi-term/         # GUI application (main binary)
```

### 1. terminal-core

Platform-independent terminal screen model. This crate is deterministic - given the same sequence of operations, it produces the same screen state.

Key types:
- `Screen` - Main terminal state, ties together grid, cursor, scrollback, modes
- `Grid` - 2D array of cells (primary and alternate screens)
- `Line` - Single row of cells with wrap flag
- `Cell` - Single character with attributes (colors, bold, italic, etc.)
- `CellAttributes` - Styling attributes for a cell
- `Cursor` - Cursor position and state
- `Scrollback` - Ring buffer for scrolled-off lines
- `Selection` - Text selection state (Normal, Word, Line, Block types)
- `Modes` - Terminal mode flags (auto-wrap, origin mode, etc.)
- `Color` - Color representation (Default, Indexed, RGB)
- `CharsetState` - Character set translation (G0-G3, DEC special graphics)

### 2. terminal-parser

Streaming VT/xterm escape sequence parser. Converts byte stream into semantic terminal actions.

Key types:
- `Parser` - State machine parser
- `ParserState` - Parser states (Ground, Escape, CsiEntry, OscString, etc.)
- `Action` - Semantic actions (Print, Control, Csi, Osc, Esc)
- `CsiAction` - CSI sequence data (params, intermediates, final byte)
- `OscAction` - OSC commands (SetTitle, SetHyperlink, Clipboard, etc.)
- `EscAction` - ESC sequence actions (SaveCursor, Index, etc.)
- `Params` - CSI parameter parsing

### 3. terminal-pty

Linux PTY (pseudoterminal) management.

Key types:
- `Pty` - PTY master file descriptor wrapper
- `Child` - Child process with PTY
- `WindowSize` - Terminal dimensions (cols, rows, pixel width/height)

### 4. mochi-term

GUI application that ties everything together.

Key types:
- `App` - Main application state, event loop
- `Terminal` - Combines parser and screen, processes input
- `Renderer` - CPU-based rendering with softbuffer + fontdue
- `Config` - Configuration (themes, fonts, settings)
- `ColorScheme` - Theme colors (foreground, background, ANSI palette)

## Data Flow

```
User Input (keyboard/mouse)
    │
    ▼
┌─────────────────┐
│  App (winit)    │ ──► Encode input to escape sequences
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  Child (PTY)    │ ──► Write to PTY master
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  Shell/Program  │ ──► Process input, generate output
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  Child (PTY)    │ ◄── Read from PTY master
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  Parser         │ ──► Parse escape sequences into Actions
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  Terminal       │ ──► Apply actions to Screen
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  Renderer       │ ──► Render Screen to pixel buffer
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  softbuffer     │ ──► Display in window
└─────────────────┘
```

## Threading Model

The application is single-threaded with non-blocking I/O:
- PTY reads are non-blocking (O_NONBLOCK)
- Event loop polls for window events and PTY data
- No separate threads for PTY I/O

## Key Files for Phase 2 Changes

### Config System (M1)
- `mochi-term/src/config.rs` - Config struct, loading, themes
- Need to add: CLI argument parsing, XDG support, validation

### Themes (M2)
- `mochi-term/src/config.rs` - ThemeName enum, ColorScheme struct
- Already has 6 themes: Dark, Light, SolarizedDark, SolarizedLight, Dracula, Nord
- Need to add: Runtime theme switching via keybinding

### Font Customization (M3)
- `mochi-term/src/renderer.rs` - Font loading, glyph caching, cell sizing
- `mochi-term/src/app.rs` - Font size change handlers (Ctrl+/-)
- Need to add: Font family config, fallback fonts, runtime reload

### Keybindings (M4)
- `mochi-term/src/app.rs` - Key event handling in handle_key_input()
- `mochi-term/src/input.rs` - Key encoding for terminal
- Need to add: Configurable keybinding system

### Selection & Search (M5)
- `terminal-core/src/selection.rs` - Selection struct and logic
- `mochi-term/src/app.rs` - Mouse handling (currently minimal)
- Need to add: Mouse selection, search UI overlay

### Security (M6)
- `mochi-term/src/terminal.rs` - OSC handling (clipboard, title)
- `mochi-term/src/config.rs` - osc52_clipboard, osc52_max_size settings
- Need to add: Title throttling, config reload

## Known Issues to Fix

### Paste Bug
The `handle_paste()` method in `app.rs` exists but is marked `#[allow(dead_code)]` and is never called. The keybinding for paste (Ctrl+Shift+V) is not implemented.

Location: `mochi-term/src/app.rs:454-473`

### Missing Mouse Selection
The `Selection` struct in terminal-core has full selection logic, but the App doesn't implement mouse-based selection. Mouse events are handled minimally.

## Dependencies

Key dependencies (from Cargo.toml):
- `winit` - Window creation and event loop
- `softbuffer` - CPU-based window rendering
- `fontdue` - Font rasterization
- `arboard` - Clipboard access
- `toml` - Config file parsing
- `serde` - Serialization
- `dirs` - XDG directory paths
- `unicode-width` - Character width calculation
- `env_logger` - Logging

## Where to Add New Features

### Config System
Add to `config.rs`:
- CLI argument struct (use `clap` or manual parsing)
- Config validation with error messages
- XDG_CONFIG_HOME support (already uses `dirs` crate)

### Theme Switching
Add to `app.rs`:
- Keybinding handler for theme toggle
- Method to apply new theme to renderer

### Keybinding System
Create new file `mochi-term/src/keybindings.rs`:
- KeyAction enum
- Keybinding struct (modifiers + key -> action)
- Default keybindings
- Config parsing for custom keybindings

### Search UI
Create new file `mochi-term/src/search.rs`:
- SearchState struct
- Search overlay rendering
- Match highlighting

### Selection
Add to `app.rs`:
- Mouse press/drag/release handlers
- Double-click word selection
- Triple-click line selection
- Copy to clipboard on selection
