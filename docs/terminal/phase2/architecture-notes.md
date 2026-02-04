# Mochi Terminal Architecture Notes

## Overview

Mochi Terminal is a VT/xterm-compatible terminal emulator built from scratch in Rust. It consists of four main crates organized in a layered architecture.

## Crate Structure

### terminal-core
Platform-independent terminal screen model providing:
- `Screen`: Main screen state with grid, cursor, scrollback, and modes
- `Grid`: 2D grid of cells representing the visible terminal
- `Cell` / `CellAttributes`: Character storage with styling (colors, bold, italic, etc.)
- `Cursor` / `CursorStyle`: Cursor position and appearance
- `Scrollback`: Ring buffer for scrollback history
- `Selection`: Text selection state (normal, word, line, block)
- `Modes`: Terminal mode flags (auto-wrap, origin mode, mouse tracking, etc.)
- `Color`: Terminal color representation (default, indexed 0-255, RGB)
- `Charset` / `CharsetState`: Character set handling (ASCII, DEC Special Graphics)

Key design: Deterministic - same operations always produce same state.

### terminal-parser
Streaming VT/xterm escape sequence parser:
- `Parser` / `ParserState`: State machine for parsing byte streams
- `Action`: Parsed terminal actions (Print, Control, Esc, Csi, Osc, Dcs, etc.)
- `CsiAction`: CSI sequence data (params, intermediates, final byte)
- `EscAction`: ESC sequence types (SaveCursor, RestoreCursor, Index, etc.)
- `OscAction`: OSC sequence types (SetTitle, Hyperlink, Clipboard, etc.)
- `Params`: Parameter parsing for CSI sequences

Key design: Handles arbitrary chunk boundaries (streaming), UTF-8 aware.

### terminal-pty
Linux pseudoterminal management:
- `Pty`: PTY file descriptor wrapper with non-blocking I/O
- `Child`: Child process with PTY, stdin/stdout/stderr
- `WindowSize`: Terminal dimensions for TIOCSWINSZ
- `Error` / `Result`: PTY-specific error handling

Key design: Uses posix_openpt/grantpt/unlockpt on Linux, openpty on macOS.

### mochi-term (main application)
GUI application tying everything together:
- `App`: Main application state and event loop (winit)
- `Terminal`: Integrates parser + screen, handles escape sequences
- `Renderer`: CPU-based rendering using softbuffer + fontdue
- `Config` / `ColorScheme` / `ThemeName`: Configuration and theming
- `input`: Keyboard/mouse encoding to terminal sequences

## Data Flow

```
User Input (keyboard/mouse)
    |
    v
App::handle_key_input() / handle_mouse_*()
    |
    v
input::encode_key() / encode_mouse()
    |
    v
Child::write_all() -> PTY master fd
    |
    v
Shell/Application (bash, vim, etc.)
    |
    v
PTY slave fd -> PTY master fd
    |
    v
App::poll_pty() -> Child::pty_mut().try_read()
    |
    v
Terminal::process(data)
    |
    v
Parser::parse() -> Actions
    |
    v
Terminal::handle_action() -> Screen mutations
    |
    v
App::render() -> Renderer::render(screen, selection, scroll_offset)
    |
    v
softbuffer Surface -> Window
```

## Key Integration Points for Phase 2

### Config System (M1)
- Current: `Config::load()` reads from `~/.config/mochi/config.toml` via `dirs` crate
- Enhancement needed: CLI args parsing, environment variable support, precedence logic
- Location: `mochi-term/src/config.rs`

### Themes (M2)
- Current: 6 built-in themes (Dark, Light, SolarizedDark, SolarizedLight, Dracula, Nord)
- Current: `Config::effective_colors()` returns ColorScheme based on theme
- Enhancement needed: Runtime theme switching, custom theme file loading
- Location: `mochi-term/src/config.rs`, `mochi-term/src/renderer.rs`

### Font Customization (M3)
- Current: Bundled DejaVuSansMono fonts, font_size in config
- Current: `Renderer::set_font_size()` exists but only changes size
- Enhancement needed: Font family selection, fallback fonts, runtime reload
- Location: `mochi-term/src/renderer.rs`

### Keybindings (M4)
- Current: Hardcoded Ctrl+=/- for font zoom in `App::handle_key_input()`
- Enhancement needed: Configurable keybindings, copy/paste/find/reload/toggle-theme
- Location: `mochi-term/src/app.rs`, new keybinding module

### UX Polish (M5)
- Selection: Already implemented in `terminal-core/src/selection.rs`
- Scrollback search: Not implemented, needs new UI overlay
- Hyperlinks: Parser supports OSC 8, rendering not implemented
- **PASTE BUG**: `App::handle_paste()` exists but is NEVER CALLED (dead_code)
- Location: `mochi-term/src/app.rs`, `terminal-core/src/selection.rs`

### Config Reload (M6)
- Current: Config loaded once at startup
- Enhancement needed: Runtime reload via keybinding, file watcher
- Location: `mochi-term/src/app.rs`, `mochi-term/src/config.rs`

### Security (M6)
- Current: OSC 52 clipboard disabled by default, max size configurable
- Enhancement needed: User indication when clipboard modified, title throttling
- Location: `mochi-term/src/terminal.rs`

## Critical Bug: Paste Not Working

The `handle_paste()` function in `app.rs` (lines 455-473) is marked `#[allow(dead_code)]` and is never called. The keybinding for Ctrl+Shift+V needs to be added to `handle_key_input()`.

## File Locations Summary

| Feature | Primary File(s) |
|---------|----------------|
| Config loading | `mochi-term/src/config.rs` |
| CLI args | `mochi-term/src/main.rs` (needs clap) |
| Themes | `mochi-term/src/config.rs` |
| Rendering | `mochi-term/src/renderer.rs` |
| Keybindings | `mochi-term/src/app.rs` |
| Selection | `terminal-core/src/selection.rs` |
| Parser | `terminal-parser/src/parser.rs` |
| Screen model | `terminal-core/src/screen.rs` |
| PTY | `terminal-pty/src/pty.rs`, `child.rs` |
