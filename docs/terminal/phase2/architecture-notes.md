# Mochi Terminal Phase 2 - Architecture Notes

This document captures the current architecture of the Mochi terminal emulator as understood during the Phase 2 reconnaissance phase.

## Overview

Mochi is a Linux terminal emulator built from scratch in Rust. It does not wrap any existing terminal widget or library - all VT/xterm escape sequence parsing, screen model management, and rendering are implemented from first principles.

## Crate Structure

The project is organized as a Cargo workspace with 4 crates:

```
terminal/
├── Cargo.toml          # Workspace definition
├── terminal-core/      # Screen model, grid, cells, selection
├── terminal-parser/    # VT/xterm escape sequence parser
├── terminal-pty/       # PTY management and child process
└── mochi-term/         # GUI application (winit + softbuffer + fontdue)
```

### terminal-core

The core data structures for terminal state management.

**Key Types:**
- `Screen` - Complete terminal state (primary/alternate grids, cursor, scrollback, selection, modes)
- `Grid` - 2D array of lines representing visible screen area
- `Line` - Row of cells with wrap flag
- `Cell` - Unicode content, attributes (colors, bold, italic, etc.), width, hyperlink_id
- `Cursor` - Position, style (Block/Underline/Bar), visibility, blinking, attributes
- `Scrollback` - Ring buffer of lines that scrolled off top (default 10,000 lines)
- `Selection` - Text selection state (Normal, Word, Line, Block types)
- `Modes` - Terminal mode flags (insert, auto-wrap, mouse tracking, bracketed paste, etc.)
- `Color` - Default, Indexed(0-255), RGB
- `Dimensions` - cols/rows tuple

**Files:**
- `lib.rs` - Module exports
- `screen.rs` - Screen implementation (898 lines)
- `grid.rs` - Grid implementation (364 lines)
- `line.rs` - Line implementation (291 lines)
- `cell.rs` - Cell and CellAttributes (255 lines)
- `cursor.rs` - Cursor and SavedCursor (236 lines)
- `color.rs` - Color enum and palette conversion (166 lines)
- `scrollback.rs` - Ring buffer scrollback (301 lines)
- `selection.rs` - Selection logic (270 lines)
- `modes.rs` - Terminal modes (219 lines)
- `charset.rs` - Character set handling (DEC special graphics)
- `snapshot.rs` - Screen state serialization

### terminal-parser

State machine for parsing VT/xterm escape sequences.

**Key Types:**
- `Parser` - State machine with UTF-8 decoder
- `ParserState` - Ground, Escape, CsiEntry, CsiParam, OscString, DcsPassthrough, etc.
- `Action` - Print(char), Control(u8), Esc(EscAction), Csi(CsiAction), Osc(OscAction), etc.
- `EscAction` - SaveCursor, RestoreCursor, Index, ReverseIndex, DesignateG0/G1/G2/G3, etc.
- `CsiAction` - params, intermediates, final_byte, private flag
- `OscAction` - SetTitle, SetColor, Hyperlink, Clipboard, etc.
- `Params` - Parameter parsing with subparameter support

**Files:**
- `lib.rs` - Module exports
- `parser.rs` - Main parser state machine (934 lines)
- `action.rs` - Action types (187 lines)
- `params.rs` - Parameter parsing
- `utf8.rs` - UTF-8 decoder

### terminal-pty

PTY management and child process handling.

**Key Types:**
- `Pty` - Pseudoterminal file descriptor wrapper
- `Child` - Child process with PTY
- `WindowSize` - Terminal dimensions in chars and pixels

**Files:**
- `lib.rs` - Module exports
- `pty.rs` - PTY creation and management
- `child.rs` - Child process spawning
- `size.rs` - Window size handling

### mochi-term

The GUI application that ties everything together.

**Key Types:**
- `App` - Main application state, event loop, window management
- `Config` - Configuration with font, colors, themes, security settings
- `Terminal` - Integrates parser and screen, handles escape sequence actions
- `Renderer` - CPU-based rendering with softbuffer + fontdue
- `ColorScheme` - Theme colors (foreground, background, cursor, selection, ANSI 16)
- `ThemeName` - Dark, Light, SolarizedDark, SolarizedLight, Dracula, Nord, Custom

**Files:**
- `main.rs` - Entry point, config loading
- `app.rs` - Application event loop (561 lines)
- `config.rs` - Configuration types and loading (382 lines)
- `terminal.rs` - Terminal integration (806 lines)
- `renderer.rs` - CPU rendering (550 lines)
- `input.rs` - Keyboard/mouse encoding (372 lines)
- `event.rs` - Custom events

## Data Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                         User Input                               │
│  (keyboard, mouse, paste)                                        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      App (mochi-term)                            │
│  - Handles window events via winit                               │
│  - Encodes input to terminal escape sequences (input.rs)         │
│  - Manages clipboard via arboard                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      PTY (terminal-pty)                          │
│  - Writes encoded input to PTY master                            │
│  - Reads output from child process                               │
│  - Handles window resize (TIOCSWINSZ)                            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Parser (terminal-parser)                      │
│  - Parses byte stream into Actions                               │
│  - Handles UTF-8 decoding                                        │
│  - State machine for escape sequences                            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Terminal (mochi-term)                          │
│  - Dispatches Actions to Screen                                  │
│  - Handles CSI, OSC, ESC sequences                               │
│  - Manages title, bell, hyperlinks                               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Screen (terminal-core)                        │
│  - Updates grid cells                                            │
│  - Manages cursor position                                       │
│  - Handles scrollback                                            │
│  - Manages selection                                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Renderer (mochi-term)                          │
│  - Rasterizes glyphs via fontdue                                 │
│  - Draws cells to pixel buffer                                   │
│  - Displays via softbuffer                                       │
└─────────────────────────────────────────────────────────────────┘
```

## Current Configuration System

The existing config system (`mochi-term/src/config.rs`) provides:

**Config struct fields:**
- `font_family: String` - Font family name (default: "DejaVu Sans Mono")
- `font_size: f32` - Font size in points (default: 14.0)
- `scrollback_lines: usize` - Scrollback buffer size (default: 10000)
- `dimensions: Option<(u16, u16)>` - Initial cols/rows
- `theme: ThemeName` - Selected theme
- `colors: Option<ColorScheme>` - Custom color overrides
- `osc52_clipboard: bool` - OSC 52 clipboard support (default: false)
- `osc52_max_size: usize` - Max clipboard payload (default: 100KB)
- `shell: Option<String>` - Shell to run
- `cursor_style: CursorStyle` - Block/Underline/Bar
- `cursor_blink: bool` - Cursor blinking

**Built-in themes:**
- Dark (default)
- Light
- SolarizedDark
- SolarizedLight
- Dracula
- Nord
- Custom (user-defined)

**Config loading:**
- Path: `~/.config/mochi/config.toml` (via `dirs` crate)
- Format: TOML
- No CLI argument support currently
- No environment variable support currently
- No config precedence system

## Where to Add Phase 2 Features

### M1: Config System Foundation

**Changes needed:**
1. Add CLI argument parsing to `main.rs` (use `clap` crate)
2. Add environment variable support to `config.rs`
3. Implement config precedence: CLI > env > file > defaults
4. Add config validation with clear error messages
5. Create example config file

**Files to modify:**
- `mochi-term/src/main.rs` - Add CLI parsing
- `mochi-term/src/config.rs` - Add env vars, validation, precedence
- `mochi-term/Cargo.toml` - Add clap dependency

### M2: Themes + Light/Dark Mode

**Changes needed:**
1. Add runtime theme switching via keybinding
2. Add theme file loading from external paths
3. Ensure theme applies to all rendering (already mostly done)
4. Add 2 more built-in themes (already have 6, need 4 total per spec)

**Files to modify:**
- `mochi-term/src/app.rs` - Add theme toggle keybinding
- `mochi-term/src/config.rs` - Add theme file loading
- `mochi-term/src/renderer.rs` - Ensure theme applies everywhere

### M3: Font Customization + Layout

**Changes needed:**
1. Implement font family selection (currently hardcoded to bundled font)
2. Add font fallback list support
3. Add cell padding/line height configuration
4. Implement runtime font reload
5. Ensure font changes trigger PTY resize

**Files to modify:**
- `mochi-term/src/renderer.rs` - Font loading, fallback, cell size
- `mochi-term/src/app.rs` - Font reload, PTY resize
- `mochi-term/src/config.rs` - Font configuration options

### M4: Keybinding Customization

**Changes needed:**
1. Add keybinding configuration to config file
2. Implement keybinding parser
3. Add default keybindings for copy/paste/find/reload/toggle-theme
4. Ensure keybindings don't interfere with terminal input

**Files to modify:**
- `mochi-term/src/config.rs` - Keybinding configuration
- `mochi-term/src/app.rs` - Keybinding handling
- New file: `mochi-term/src/keybindings.rs` - Keybinding system

### M5: UX Polish

**Changes needed:**
1. Implement word/line selection (double/triple click)
2. Add scrollback search UI (find bar overlay)
3. Improve hyperlink UX (hover, ctrl+click)

**Files to modify:**
- `mochi-term/src/app.rs` - Click handling, search UI
- `terminal-core/src/selection.rs` - Word/line selection logic
- `mochi-term/src/renderer.rs` - Search highlight rendering

### M6: Config Reload + Security

**Changes needed:**
1. Add config reload keybinding
2. Add file watcher (optional, use `notify` crate)
3. Handle reload failures gracefully
4. Add clipboard OSC security (already partially implemented)
5. Add title update throttling

**Files to modify:**
- `mochi-term/src/app.rs` - Reload handling, file watcher
- `mochi-term/src/config.rs` - Reload logic
- `mochi-term/src/terminal.rs` - Title throttling

### M7: No Regressions

**Changes needed:**
1. Run vttest and document results
2. Fix any regressions introduced by Phase 2
3. Ensure all existing tests pass

## Threading Model

Currently single-threaded. The main event loop in `App::run()`:
1. Polls winit events
2. Polls PTY for output
3. Renders frame
4. Repeats

This is simple but may need adjustment for file watching or async operations.

## Memory Considerations

- Scrollback: Bounded ring buffer (default 10,000 lines)
- Glyph cache: HashMap keyed by (char, bold) - unbounded but practical
- Parser buffers: Fixed size (MAX_OSC_LEN = 65536)

## Test Coverage

Current test count: 139 tests across all crates
- terminal-core: 76 tests
- terminal-parser: 33 tests
- terminal-pty: 11 tests
- mochi-term: 19 tests

Tests cover:
- Cell/line/grid operations
- Cursor movement
- Selection logic
- Scrollback ring buffer
- Parser state machine
- UTF-8 decoding
- PTY creation
- Input encoding
- Config parsing

## Dependencies

Key dependencies (from Cargo.toml files):
- `winit` - Window creation and event loop
- `softbuffer` - CPU-based pixel buffer display
- `fontdue` - Font rasterization
- `nix` - Unix system calls (PTY)
- `arboard` - Clipboard access
- `serde` + `toml` - Configuration parsing
- `dirs` - XDG directory paths
- `unicode-width` - Character width calculation
- `env_logger` - Logging

## CI/CD

GitHub Actions workflow (`.github/workflows/ci.yml`):
- Runs on: ubuntu-latest, macos-latest
- Jobs: build, test, clippy, fmt
- All must pass for PR merge
