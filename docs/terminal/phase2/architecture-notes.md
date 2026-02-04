# Mochi Terminal Phase 2 - Architecture Notes

## Module Map

The Mochi terminal emulator is organized into 4 crates within a Cargo workspace:

```
terminal/
├── Cargo.toml           # Workspace configuration
├── terminal-core/       # Platform-independent screen model
├── terminal-parser/     # VT/xterm escape sequence parser
├── terminal-pty/        # Linux PTY management
└── mochi-term/          # GUI application (winit + softbuffer)
```

### 1. terminal-core

The core crate provides the terminal screen model and is designed to be deterministic and platform-independent.

**Key modules:**
- `cell.rs` - Cell and CellAttributes (character + styling)
- `color.rs` - Color enum (Default, Indexed, Rgb)
- `cursor.rs` - Cursor state and SavedCursor
- `grid.rs` - 2D grid of cells
- `line.rs` - Single line of cells
- `screen.rs` - Complete terminal state (grids, cursor, scrollback, modes)
- `scrollback.rs` - Ring buffer for scrollback history
- `selection.rs` - Text selection state
- `modes.rs` - Terminal mode flags (DEC modes, etc.)
- `charset.rs` - Character set translation (G0-G3)
- `snapshot.rs` - Screen state serialization

**Key types:**
- `Screen` - Main interface, contains primary/alternate grids, cursor, scrollback, modes
- `Cell` - Character + attributes + hyperlink_id
- `CellAttributes` - Bold, italic, underline, colors, etc.
- `Color` - Default | Indexed(u8) | Rgb(u8, u8, u8)
- `Cursor` - Position, attributes, visibility, style
- `Selection` - Start/end points, selection type

### 2. terminal-parser

Streaming parser for VT/xterm escape sequences.

**Key modules:**
- `parser.rs` - State machine parser
- `action.rs` - Semantic actions produced by parser
- `params.rs` - CSI parameter parsing
- `utf8.rs` - UTF-8 decoder

**Key types:**
- `Parser` - State machine with parse() method
- `Action` - Print, Control, Esc, Csi, Osc, Dcs, etc.
- `CsiAction` - CSI sequence with params, intermediates, final byte
- `OscAction` - OSC commands (title, hyperlink, clipboard, etc.)
- `EscAction` - ESC sequences (save/restore cursor, index, etc.)

### 3. terminal-pty

Linux PTY management using nix crate.

**Key modules:**
- `pty.rs` - PTY creation and configuration
- `child.rs` - Child process spawning and management
- `size.rs` - Window size (cols, rows, pixels)

**Key types:**
- `Pty` - PTY file descriptor wrapper
- `Child` - Spawned child process
- `WindowSize` - Terminal dimensions

### 4. mochi-term

GUI application using winit for windowing and softbuffer for CPU-based rendering.

**Key modules:**
- `main.rs` - Entry point, config loading
- `app.rs` - Application state and event loop
- `config.rs` - Configuration loading and theme definitions
- `renderer.rs` - CPU-based rendering with fontdue
- `terminal.rs` - Terminal state integrating parser and screen
- `input.rs` - Keyboard and mouse input encoding
- `event.rs` - Event types

**Key types:**
- `App` - Main application state
- `Config` - Configuration with themes, fonts, etc.
- `Renderer` - Rendering context with glyph cache
- `Terminal` - Parser + Screen integration

## Data Flow

### Input Flow
```
Keyboard/Mouse Event (winit)
    → App::handle_key_input() / handle_mouse_input()
    → input::encode_key() / encode_mouse()
    → PTY write (Child::write())
    → Shell/Application
```

### Output Flow
```
PTY read (Child::read())
    → App::poll_pty()
    → Terminal::process()
    → Parser::parse() → Actions
    → Terminal::handle_action()
    → Screen mutations
    → App::render()
    → Renderer::render()
    → softbuffer display
```

## Where to Add Phase 2 Features

### M1: Config System Foundation

**Location:** `mochi-term/src/config.rs`

The config system already exists but needs enhancement:
- Add CLI argument parsing in `main.rs` (use `clap` or manual parsing)
- Add environment variable support in `Config::load()`
- Add proper XDG path handling (already uses `dirs` crate)
- Add config validation with clear error messages
- Add `--config` flag override

**Layering:** Config is loaded once at startup and passed to App. No changes to core/parser/pty needed.

### M2: Themes + Light/Dark Mode

**Location:** `mochi-term/src/config.rs` (theme definitions), `mochi-term/src/app.rs` (runtime switching)

The theme system already exists with 6 built-in themes. Need to add:
- Runtime theme switching via keybinding in `App::handle_key_input()`
- Custom theme file loading
- Theme format documentation

**Layering:** Themes only affect the renderer. No changes to core/parser/pty needed.

### M3: Font Customization

**Location:** `mochi-term/src/renderer.rs`, `mochi-term/src/config.rs`

Current state:
- Uses bundled DejaVuSansMono fonts
- Font size can be changed at runtime (Ctrl+/-)
- No configurable font family

Need to add:
- System font loading (fontconfig or manual path)
- Fallback fonts list
- Cell padding / line height configuration
- Proper PTY resize when font metrics change

**Layering:** Font changes affect renderer and require PTY resize. Changes in:
- `config.rs` - Font configuration
- `renderer.rs` - Font loading and cell size calculation
- `app.rs` - PTY resize on font change

### M4: Keybinding Customization

**Location:** `mochi-term/src/config.rs` (keybinding config), `mochi-term/src/app.rs` (keybinding handling)

Need to add:
- Keybinding configuration structure
- Keybinding parsing
- Action dispatch in `App::handle_key_input()`

**Layering:** Keybindings only affect the app layer. No changes to core/parser/pty needed.

### M5: UX Polish

**Selection improvements:**
- `terminal-core/src/selection.rs` - Word/line selection logic
- `mochi-term/src/app.rs` - Double/triple click handling

**Scrollback search:**
- `mochi-term/src/app.rs` - Search state and UI overlay
- `mochi-term/src/renderer.rs` - Search highlight rendering

**Hyperlink UX:**
- `mochi-term/src/app.rs` - Hyperlink click handling
- `mochi-term/src/renderer.rs` - Hyperlink styling

**Layering:** Selection logic is in core, but search UI and hyperlink handling are in mochi-term.

### M6: Config Reload + Security

**Location:** `mochi-term/src/app.rs` (reload handling), `mochi-term/src/config.rs` (validation)

Need to add:
- Config reload keybinding
- Optional file watcher (inotify)
- Error handling on reload failure
- Security documentation

**Layering:** Config reload affects app layer only.

## Existing Features to Preserve

- UTF-8 support
- 16/256/truecolor support
- Cursor styles (block, underline, bar)
- Alternate screen buffer
- Scroll regions
- Scrollback buffer (10,000 lines default)
- Bracketed paste mode
- Mouse reporting (SGR format)
- OSC sequences (title, hyperlink, clipboard)
- Selection and clipboard (arboard)
- Font zoom (Ctrl+/-)

## Testing Strategy

- Unit tests: Already exist in each crate
- Integration tests: Can add PTY-based tests
- Golden tests: Can add screen snapshot tests
- Manual tests: Run terminal and verify visually
