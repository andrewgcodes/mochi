# Mochi Terminal Architecture Notes

This document describes the architecture of the Mochi Terminal emulator as of Phase 1 completion, serving as a reference for Phase 2 development.

## Overview

Mochi Terminal is a VT/xterm-compatible terminal emulator built from scratch in Rust. It consists of four crates organized in a workspace, with clear separation of concerns between parsing, state management, PTY handling, and rendering.

## Crate Structure

```
terminal/
├── terminal-parser/    # Escape sequence parsing (no dependencies on other crates)
├── terminal-core/      # Screen model and state (no dependencies on other crates)
├── terminal-pty/       # PTY management (Linux-specific)
└── mochi-term/         # GUI application (depends on all above)
```

### Dependency Graph

```
mochi-term
    ├── terminal-core
    ├── terminal-parser
    └── terminal-pty
```

The core and parser crates are independent of each other, allowing for clean testing and potential reuse.

## Crate Details

### terminal-parser

Location: `terminal/terminal-parser/`

Purpose: Streaming parser for VT/xterm escape sequences. Converts a byte stream into semantic terminal actions.

Key Types:
- `Parser` - State machine parser based on VT500 model
- `ParserState` - Enum of parser states (Ground, Escape, CsiEntry, CsiParam, OscString, etc.)
- `Action` - Enum of parsed actions (Print, Control, Esc, Csi, Osc, Dcs, etc.)
- `CsiAction` - CSI sequence with params, intermediates, final byte, private flag
- `OscAction` - OSC commands (SetTitle, SetColor, Hyperlink, Clipboard, etc.)
- `EscAction` - ESC sequences (SaveCursor, RestoreCursor, Index, DesignateG0-G3, etc.)

Design Notes:
- Handles arbitrary chunk boundaries (streaming)
- UTF-8 decoding built-in
- Maximum OSC length: 65536 bytes (DoS protection)
- Maximum intermediates: 4 bytes

### terminal-core

Location: `terminal/terminal-core/`

Purpose: Platform-independent terminal screen model. Deterministic state machine for terminal emulation.

Key Types:
- `Screen` - Main interface tying together grid, cursor, scrollback, modes
- `Grid` - 2D array of cells (primary and alternate screens)
- `Line` - Single row of cells with wrapped flag
- `Cell` - Character + attributes + hyperlink ID
- `CellAttributes` - Colors, bold, italic, underline, inverse, etc.
- `Color` - Enum (Default, Indexed(0-255), Rgb{r,g,b})
- `Cursor` - Position, visibility, style, attributes, origin mode
- `Scrollback` - Ring buffer of scrolled-off lines
- `Selection` - Text selection state (Normal, Word, Line, Block types)
- `Modes` - Terminal mode flags (auto-wrap, cursor visible, mouse tracking, etc.)
- `Dimensions` - cols/rows tuple
- `CharsetState` - G0-G3 character set designations

Design Notes:
- Primary and alternate screen buffers
- Scroll region support (DECSTBM)
- Tab stops with configurable positions
- Hyperlink registry (OSC 8)
- Unicode width handling via `unicode-width` crate

### terminal-pty

Location: `terminal/terminal-pty/`

Purpose: Linux pseudoterminal management for spawning and managing child processes.

Key Types:
- `Child` - Child process with PTY
- `Pty` - Raw PTY file descriptor wrapper
- `WindowSize` - cols/rows for TIOCSWINSZ

Design Notes:
- Uses `posix_openpt`, `grantpt`, `unlockpt`, `ptsname`
- Non-blocking I/O support
- Session and controlling terminal setup
- Window size propagation via ioctl

### mochi-term

Location: `terminal/mochi-term/`

Purpose: GUI application that ties everything together.

Key Types:
- `App` - Main application state and event loop
- `Terminal` - Integrates parser and screen, handles action dispatch
- `Renderer` - CPU-based rendering using softbuffer and fontdue
- `Config` - Configuration with themes, fonts, colors
- `ColorScheme` - Theme colors (foreground, background, cursor, selection, ANSI 16)
- `ThemeName` - Enum of built-in themes (Dark, Light, SolarizedDark, etc.)

Key Files:
- `main.rs` - Entry point, logging setup, config loading
- `app.rs` - Event loop, window management, input handling
- `terminal.rs` - Parser integration, CSI/OSC/ESC handling, SGR processing
- `renderer.rs` - Glyph caching, cell rendering, scrollbar
- `config.rs` - Config loading, theme definitions, color parsing
- `input.rs` - Keyboard/mouse encoding to escape sequences

## Data Flow

### Input Flow (User -> Shell)

```
Keyboard/Mouse Event (winit)
    ↓
App::handle_key_input() / handle_mouse_input()
    ↓
input::encode_key() / encode_mouse()
    ↓
Child::write_all() (to PTY)
    ↓
Shell process receives input
```

### Output Flow (Shell -> Display)

```
Shell writes to PTY
    ↓
App::poll_pty() reads from Child
    ↓
Terminal::process(bytes)
    ↓
Parser::parse() produces Actions
    ↓
Terminal::handle_action() updates Screen
    ↓
App::render() calls Renderer::render()
    ↓
Renderer draws to softbuffer Surface
    ↓
Window displays frame
```

## Current Configuration System

Location: `mochi-term/src/config.rs`

Current State:
- Config struct with font_family, font_size, scrollback_lines, dimensions, theme, colors, osc52_clipboard, osc52_max_size, shell, cursor_style, cursor_blink
- Loads from `~/.config/mochi/config.toml` (XDG convention)
- Falls back to defaults if file missing or invalid
- ThemeName enum with Dark, Light, SolarizedDark, SolarizedLight, Dracula, Nord, Custom
- ColorScheme with foreground, background, cursor, selection, ansi[16]

Missing for Phase 2:
- CLI argument parsing (--config override)
- Environment variable support
- Config precedence documentation and testing
- Runtime config reload
- Config validation with error messages
- Example config file

## Current Theme System

Location: `mochi-term/src/config.rs`

Current State:
- 6 built-in themes: Dark, Light, SolarizedDark, SolarizedLight, Dracula, Nord
- Custom theme support via colors field when theme=Custom
- effective_colors() method returns ColorScheme based on theme setting
- Renderer receives ColorScheme at construction

Missing for Phase 2:
- Runtime theme switching (keybinding)
- Theme file loading from external files
- Theme documentation
- ANSI palette verification tests

## Current Font System

Location: `mochi-term/src/renderer.rs`

Current State:
- Bundled DejaVuSansMono.ttf and DejaVuSansMono-Bold.ttf in assets/
- fontdue for rasterization
- Glyph cache (HashMap keyed by (char, bold))
- Cell size calculation from font metrics
- HiDPI scaling (font_size * scale_factor)
- Runtime font size change via Ctrl+/Ctrl- (or Cmd on macOS)
- Font size range: 8.0 to 72.0 points

Missing for Phase 2:
- Configurable font family
- Fallback fonts for missing glyphs
- Cell padding / line height configuration
- Ligature support (optional)
- Font discovery documentation

## Current Input Handling

Location: `mochi-term/src/input.rs` and `app.rs`

Current State:
- Keyboard encoding for characters, control keys, function keys, arrows
- Mouse encoding for press/release/move/scroll (X10, VT200, SGR modes)
- Bracketed paste support
- Focus event encoding
- Font zoom shortcuts (Ctrl+/Ctrl-/Ctrl+0)

Missing for Phase 2:
- Configurable keybindings
- Copy/paste shortcuts
- Find/search shortcut
- Config reload shortcut
- Theme toggle shortcut

## Current Selection System

Location: `terminal-core/src/selection.rs`

Current State:
- Selection struct with start/end points, type, active flag
- SelectionType: Normal, Word, Line, Block
- contains() method for hit testing
- bounds() for normalized start/end

Missing for Phase 2:
- Double-click word selection (UI integration)
- Triple-click line selection (UI integration)
- Selection to clipboard (copy action)

## Current Security Measures

Location: Various

Current State:
- OSC 52 clipboard disabled by default (osc52_clipboard: false)
- OSC 52 max payload size: 100,000 bytes
- Title length limit: 4096 characters
- OSC string max length: 65536 bytes

Missing for Phase 2:
- User-visible indication when clipboard modified
- Title update throttling
- Security documentation

## Where to Add Phase 2 Features

### Config System (M1)

Files to modify:
- `mochi-term/src/config.rs` - Add CLI parsing, env var support, validation
- `mochi-term/src/main.rs` - Add clap for CLI args
- New: `docs/terminal/config.md` - Config documentation
- New: `docs/terminal/config.example.toml` - Example config

### Themes (M2)

Files to modify:
- `mochi-term/src/config.rs` - Theme file loading
- `mochi-term/src/app.rs` - Theme switching keybinding
- `mochi-term/src/renderer.rs` - Theme hot-reload
- New: `docs/terminal/themes.md` - Theme documentation

### Fonts (M3)

Files to modify:
- `mochi-term/src/config.rs` - Font config fields
- `mochi-term/src/renderer.rs` - Font loading, fallbacks, padding
- New: `docs/terminal/fonts.md` - Font documentation

### Keybindings (M4)

Files to modify:
- `mochi-term/src/config.rs` - Keybinding config
- `mochi-term/src/app.rs` - Keybinding dispatch
- New: `mochi-term/src/keybindings.rs` - Keybinding system
- New: `docs/terminal/keybindings.md` - Keybinding documentation

### UX Polish (M5)

Files to modify:
- `mochi-term/src/app.rs` - Double/triple click, search UI
- `terminal-core/src/selection.rs` - Word boundary detection
- New: `mochi-term/src/search.rs` - Search functionality

### Config Reload + Safety (M6)

Files to modify:
- `mochi-term/src/app.rs` - Reload handling, error display
- `mochi-term/src/config.rs` - Reload method
- `mochi-term/src/terminal.rs` - Title throttling
- New: `docs/terminal/security.md` - Security documentation

## Testing Infrastructure

Current Tests:
- 139 tests across all crates
- Unit tests for parser, screen, selection, modes, cells, etc.
- Integration tests for PTY spawning and I/O

Test Locations:
- `terminal-parser/src/parser.rs` - Parser tests
- `terminal-core/src/screen.rs` - Screen tests
- `terminal-core/src/selection.rs` - Selection tests
- `terminal-core/src/modes.rs` - Modes tests
- `terminal-core/src/cell.rs` - Cell tests
- `terminal-pty/src/child.rs` - PTY tests
- `mochi-term/src/terminal.rs` - Terminal integration tests
- `mochi-term/src/input.rs` - Input encoding tests
- `mochi-term/src/config.rs` - Config tests

CI:
- GitHub Actions workflow in `.github/workflows/ci.yml`
- Jobs: build, test, lint (clippy), format check

## Dependencies

Key External Dependencies:
- `winit` - Window creation and event loop
- `softbuffer` - CPU-based rendering surface
- `fontdue` - Font rasterization
- `arboard` - Clipboard access
- `toml` - Config file parsing
- `serde` - Serialization
- `dirs` - XDG directory paths
- `unicode-width` - Character width calculation
- `log` / `env_logger` - Logging

## Notes for Phase 2 Implementation

1. The config system already has a good foundation with TOML parsing and theme support. The main additions are CLI args, env vars, precedence, and validation.

2. Theme switching at runtime requires updating the Renderer's colors field and triggering a redraw. The Renderer already has a `colors` field that can be updated.

3. Font changes require clearing the glyph cache and recalculating cell size. The `set_font_size` method already does this, so font family changes should follow the same pattern.

4. Keybindings should be handled in `App::handle_key_input` before the key is sent to the PTY. A keybinding system should intercept configured shortcuts.

5. Selection improvements need UI integration in `App` for double/triple click detection, with the actual selection logic already in `terminal-core`.

6. The search UI should be an overlay that doesn't affect terminal state. It should search through scrollback + visible buffer.

7. Config reload should be atomic - parse new config, validate, then swap. On error, keep old config and show error to user.
