# Mochi Terminal - Phase 2 Architecture Notes

## Overview

This document describes the architecture of the Mochi terminal emulator as understood during Phase 2 implementation.

## Crate Structure

```
terminal/
├── terminal-core/     # Platform-independent screen model
├── terminal-parser/   # VT/xterm escape sequence parser
├── terminal-pty/      # Linux PTY management
└── mochi-term/        # GUI application
```

## Module Map

### terminal-core

| Module | Purpose |
|--------|---------|
| `cell.rs` | Cell struct with character, attributes, width, hyperlink ID |
| `line.rs` | Row of cells with wrap flag, insert/delete operations |
| `grid.rs` | 2D array of lines, scroll operations |
| `screen.rs` | Complete terminal state (grids, cursor, scrollback, modes) |
| `cursor.rs` | Cursor position, style, visibility, attributes |
| `modes.rs` | Terminal mode flags (DEC private modes, mouse, etc.) |
| `selection.rs` | Text selection state (normal, word, line, block) |
| `scrollback.rs` | Ring buffer for scrolled-off lines |
| `color.rs` | Color representation (default, indexed, RGB) |
| `charset.rs` | Character set translation (G0-G3, DEC special graphics) |
| `snapshot.rs` | Serializable terminal state for testing |

### terminal-parser

| Module | Purpose |
|--------|---------|
| `parser.rs` | State machine for escape sequence parsing |
| `action.rs` | Semantic actions (Print, Control, CSI, OSC, ESC, DCS) |
| `params.rs` | CSI parameter parsing with subparameter support |
| `utf8.rs` | Streaming UTF-8 decoder |

### terminal-pty

| Module | Purpose |
|--------|---------|
| `pty.rs` | PTY master creation and configuration |
| `child.rs` | Child process spawning and management |
| `size.rs` | Window size structures |
| `error.rs` | Error types |

### mochi-term

| Module | Purpose |
|--------|---------|
| `main.rs` | Entry point, config loading |
| `app.rs` | Application state, event loop, input handling |
| `config.rs` | Configuration structures, theme definitions |
| `renderer.rs` | CPU-based rendering with fontdue |
| `terminal.rs` | Integrates parser and screen |
| `input.rs` | Keyboard/mouse encoding to escape sequences |
| `event.rs` | Event types and timing |

## Key Data Types

### Screen State
- `Screen`: Primary container for all terminal state
- `Grid`: 2D array of `Line` objects
- `Line`: Array of `Cell` objects with metadata
- `Cell`: Character + `CellAttributes` + width + hyperlink ID

### Cursor
- `Cursor`: Position (row, col), style, visibility, attributes, origin mode
- `CursorStyle`: Block, Underline, Bar

### Colors
- `Color`: Default, Indexed(0-255), Rgb(r, g, b)
- `ColorScheme`: Foreground, background, cursor, selection, ANSI 16 palette

### Modes
- `Modes`: All terminal mode flags (auto-wrap, origin, mouse tracking, etc.)

## Data Flow

### Input Flow
```
User Input → winit Event → encode_key() → PTY write → Child Process
```

### Output Flow
```
Child Process → PTY read → Parser → Actions → Screen → Renderer → Display
```

## Where to Add Phase 2 Features

### Config System (M1)
- Extend `config.rs` with CLI argument parsing (add clap dependency)
- Add XDG path resolution
- Add config validation
- Add environment variable support

### Themes (M2)
- Already have 6 built-in themes in `config.rs`
- Add runtime theme switching in `app.rs`
- Add keybinding for theme toggle
- Update `renderer.rs` to accept theme changes

### Font Customization (M3)
- Extend `config.rs` with font family, fallback list
- Modify `renderer.rs` to load fonts dynamically
- Add font reload capability

### Keybindings (M4)
- Add `keybindings.rs` module in mochi-term
- Extend `config.rs` with keybinding configuration
- Modify `app.rs` to check keybindings before encoding keys
- **FIX**: Wire up `handle_paste()` which exists but is never called

### Selection & Search (M5)
- Selection logic exists in `terminal-core/selection.rs`
- Need to add mouse handling in `app.rs` for selection
- Add search UI overlay in `renderer.rs`
- Add search state management

### Config Reload (M6)
- Add file watcher (notify crate)
- Add reload keybinding
- Add error display mechanism

## Current Issues Identified

1. **Paste not working**: `handle_paste()` in `app.rs` is defined but never called
2. **No CLI arguments**: No way to specify config file path
3. **No keybinding system**: All shortcuts are hardcoded
4. **No mouse selection**: Selection state exists but not wired to mouse events
5. **No search UI**: No find/search functionality
6. **Font hardcoded**: DejaVuSansMono bundled, no customization

## Dependencies to Add

- `clap`: CLI argument parsing
- `notify`: File watching for config reload (optional)
- `dirs`: Already present for XDG paths

## Testing Strategy

- Unit tests for config parsing/validation
- Unit tests for keybinding parsing
- Golden tests for theme application
- Integration tests for PTY resize on font change
- Manual testing with screenshots
