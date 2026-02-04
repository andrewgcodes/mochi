# Mochi Terminal - Phase 2 Architecture Notes

## Overview

Mochi Terminal is a VT/xterm-compatible terminal emulator built from scratch in Rust. It uses a 4-crate workspace architecture with clear separation of concerns.

## Crate Structure

```
terminal/
├── Cargo.toml              # Workspace configuration
├── terminal-core/          # Core terminal state (screen, grid, cursor, etc.)
├── terminal-parser/        # VT/xterm escape sequence parser
├── terminal-pty/           # Linux PTY management
└── mochi-term/             # GUI application (winit + softbuffer + fontdue)
```

### 1. terminal-core

**Purpose**: Core terminal state management without any I/O or rendering concerns.

**Key Types**:
- `Screen` - Main interface tying together grid, cursor, scrollback, modes, selection
- `Grid` - 2D array of cells (primary and alternate screens)
- `Line` - Single row of cells with wrap tracking
- `Cell` / `CellAttributes` - Individual character cell with styling
- `Cursor` / `SavedCursor` - Cursor position and attributes
- `Scrollback` - Ring buffer for scrolled-off lines
- `Selection` - Text selection state (Normal/Word/Line/Block types)
- `Modes` - Terminal mode flags (auto-wrap, origin, mouse tracking, etc.)
- `Color` - Color enum (Default, Indexed 0-255, RGB)
- `CharsetState` - G0-G3 character set designations

**Key Features**:
- Primary/alternate screen support (for vim, htop, etc.)
- Scroll regions (DECSTBM)
- Tab stops
- Hyperlink registry
- Unicode width handling

### 2. terminal-parser

**Purpose**: Streaming parser for VT/xterm escape sequences.

**Key Types**:
- `Parser` / `ParserState` - State machine for parsing byte streams
- `Action` - Parsed action enum (Print, Control, Esc, Csi, Osc, Dcs, etc.)
- `CsiAction` - CSI sequence with params, intermediates, final byte
- `EscAction` - ESC sequence actions
- `OscAction` - OSC sequence actions (title, hyperlinks, clipboard, colors)
- `Params` - Parameter list for CSI sequences

**Design**:
- Handles arbitrary chunk boundaries (streaming)
- UTF-8 aware
- Deterministic state machine

### 3. terminal-pty

**Purpose**: Linux pseudoterminal management.

**Key Types**:
- `Pty` - PTY master/slave pair
- `Child` - Child process with PTY
- `WindowSize` - Terminal dimensions for TIOCSWINSZ

**Features**:
- PTY creation via `posix_openpt` (Linux) or `openpty` (macOS)
- Session setup with `setsid`, `TIOCSCTTY`
- Non-blocking I/O
- Window size management

### 4. mochi-term

**Purpose**: GUI application integrating all components.

**Key Modules**:
- `app.rs` - Main application state and event loop
- `config.rs` - Configuration loading/saving (TOML, XDG paths)
- `renderer.rs` - CPU-based rendering (softbuffer + fontdue)
- `terminal.rs` - Terminal state integrating parser and screen
- `input.rs` - Keyboard/mouse input encoding
- `event.rs` - Event types

**Dependencies**:
- `winit 0.29` - Window creation and event loop
- `softbuffer 0.4` - CPU-based framebuffer
- `fontdue 0.9` - Font rasterization
- `arboard 3.4` - Clipboard access
- `toml 0.8` - Config file parsing
- `dirs 5.0` - XDG directory paths

## Data Flow

### Input Flow (User -> Shell)
```
User Input (keyboard/mouse)
    ↓
winit WindowEvent
    ↓
App::handle_key_input() / handle_mouse_input()
    ↓
input.rs encode_key() / encode_mouse()
    ↓
Child::write_all() -> PTY master
    ↓
Shell process reads from PTY slave
```

### Output Flow (Shell -> Display)
```
Shell writes to PTY slave
    ↓
App::poll_pty() reads from PTY master
    ↓
Terminal::process(bytes)
    ↓
Parser::parse() -> Actions
    ↓
Terminal::handle_action() -> Screen mutations
    ↓
App::render()
    ↓
Renderer::render(screen, selection, scroll_offset)
    ↓
softbuffer Surface::present()
```

## Threading Model

**Single-threaded**: All operations run on the main thread.
- Event loop polls for window events
- PTY is set to non-blocking mode
- `poll_pty()` called in `AboutToWait` event

## Memory Management

- **Scrollback**: Bounded ring buffer (default 10,000 lines)
- **Glyph Cache**: HashMap cleared on font size change
- **Parser Buffers**: Fixed-size internal buffers

## Current Config System

Located in `mochi-term/src/config.rs`:

```rust
pub struct Config {
    pub font_family: String,      // "monospace"
    pub font_size: f32,           // 14.0
    pub scrollback_lines: usize,  // 10000
    pub dimensions: (u16, u16),   // (80, 24)
    pub theme: ThemeName,         // Dark/Light/SolarizedDark/etc.
    pub colors: ColorScheme,      // Custom colors when theme=Custom
    pub osc52_clipboard: bool,    // false (disabled for security)
    pub osc52_max_size: usize,    // 100000
    pub shell: Option<String>,    // None (use $SHELL)
    pub cursor_style: String,     // "block"
    pub cursor_blink: bool,       // true
}
```

**Config Path**: `~/.config/mochi/config.toml` (XDG)

**Existing Themes**: Dark, Light, SolarizedDark, SolarizedLight, Dracula, Nord, Custom

## Where to Add Phase 2 Features

### M1: Config System Foundation
- Extend `config.rs` with CLI argument parsing (use `clap` or manual)
- Add config validation with clear error messages
- Add environment variable support
- Keep existing `Config::load()` but add precedence logic

### M2: Themes
- Themes already exist in `config.rs` (ColorScheme methods)
- Need to add runtime theme switching (keybinding -> update renderer colors)
- Renderer stores `colors: ColorScheme` - need method to update it

### M3: Font Customization
- `Renderer::new()` loads bundled DejaVu fonts
- `Renderer::set_font_size()` exists - clears glyph cache, recalculates cell size
- Need to add font family loading (fontconfig or file path)
- Need fallback font support for missing glyphs

### M4: Keybinding Customization
- Currently hardcoded in `App::handle_key_input()`
- Need keybinding config struct and action dispatch
- Actions: Copy, Paste, Find, ReloadConfig, ToggleTheme

### M5: UX Polish
- Selection exists in `terminal-core/src/selection.rs`
- Need word/line selection (double/triple click)
- Need search UI overlay (not in terminal state)
- Hyperlinks exist (OSC 8) - need click handling

### M6: Config Reload + Safety
- Add reload keybinding
- Add file watcher (optional)
- OSC 52 already has `osc52_clipboard` flag and `osc52_max_size`
- Title throttling: `Screen::set_title()` already limits to 4096 chars

## Key Invariants to Preserve

1. **Parser is stateless between calls** - can be reset without side effects
2. **Screen owns all terminal state** - cursor, grid, modes, selection
3. **Renderer is pure** - takes screen snapshot, produces pixels
4. **PTY is non-blocking** - poll in event loop, don't block
5. **No terminal emulation dependencies** - all VT parsing is custom

## Testing Strategy

- **Unit tests**: In each crate's `tests` module
- **Integration tests**: `Terminal::process()` with escape sequences
- **Golden tests**: Snapshot screen state after processing input
- **Manual tests**: Visual inspection with screenshots
