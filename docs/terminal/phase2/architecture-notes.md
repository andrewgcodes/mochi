# Phase 2 Architecture Notes

## Module Map

The Mochi terminal emulator is organized into four crates:

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

### terminal-core
Platform-independent terminal state model:
- `Cell`: Single character cell with content, attributes, width, hyperlink ID
- `Line`: Row of cells with wrap flag
- `Grid`: 2D array of lines with scroll operations
- `Screen`: Complete terminal state (primary/alternate grids, cursor, modes, scrollback, selection)
- `Scrollback`: Ring buffer for scrolled-off lines
- `Selection`: Text selection state (normal, word, line, block)
- `Modes`: Terminal mode flags (cursor keys, mouse tracking, bracketed paste, etc.)
- `Color`: Color representation (Default, Indexed, RGB)

### terminal-parser
VT/xterm escape sequence parser:
- `Parser`: State machine (Ground, Escape, CSI, OSC, DCS states)
- `Action`: Semantic operations (Print, Control, Csi, Esc, Osc)
- `Params`: CSI parameter parsing with subparameter support
- `Utf8Decoder`: Streaming UTF-8 decoder

### terminal-pty
Linux PTY management:
- `Pty`: PTY master file descriptor with window size control
- `Child`: Child process attached to PTY with signal handling
- `WindowSize`: Terminal dimensions in cells and pixels

### mochi-term
GUI application:
- `App`: Main application state, event loop, window management
- `Terminal`: Combines parser and screen, processes PTY output
- `Renderer`: CPU-based rendering with fontdue, glyph caching
- `Config`: Configuration loading and validation
- `input`: Keyboard/mouse encoding to terminal sequences

## Key Data Types

### Config (mochi-term/src/config.rs)
```rust
struct Config {
    font_family: String,
    font_size: f32,
    scrollback_lines: usize,
    dimensions: (u16, u16),
    theme: ThemeName,
    colors: ColorScheme,
    osc52_clipboard: bool,
    osc52_max_size: usize,
    shell: Option<String>,
    cursor_style: String,
    cursor_blink: bool,
}
```

### ColorScheme (mochi-term/src/config.rs)
```rust
struct ColorScheme {
    foreground: String,
    background: String,
    cursor: String,
    selection: String,
    ansi: [String; 16],
}
```

### ThemeName (mochi-term/src/config.rs)
```rust
enum ThemeName {
    Dark,
    Light,
    SolarizedDark,
    SolarizedLight,
    Dracula,
    Nord,
    Custom,
}
```

## Data Flow

### Input (keyboard/mouse to child)
```
User Input → winit Event → encode_key() → PTY write → Child Process
```

### Output (child to screen)
```
Child Process → PTY read → Parser → Actions → Screen → Renderer → Display
```

## Where to Add Phase 2 Features

### Config System (M1)
- Extend `Config` struct in `mochi-term/src/config.rs`
- Add CLI argument parsing in `mochi-term/src/main.rs`
- Add config validation and error handling
- Add environment variable support

### Themes (M2)
- Already have 6 built-in themes in `ColorScheme`
- Add runtime theme switching in `App`
- Add keybinding for theme toggle

### Font Customization (M3)
- Extend `Config` with font fallback list, cell padding
- Modify `Renderer` to support font family selection
- Add runtime font reload in `App`

### Keybindings (M4)
- Add `Keybindings` struct to config
- Add keybinding parsing and action mapping
- Handle shortcuts in `App::handle_key_input`
- **FIX: `handle_paste` is never called - need to add Ctrl+Shift+V handling**

### UX Polish (M5)
- Extend `Selection` for word/line detection
- Add search UI overlay in `Renderer`
- Add search state in `App`

### Config Reload (M6)
- Add file watcher (inotify) in `App`
- Add reload keybinding
- Add error handling for failed reloads

## Current Issues Identified

1. **Paste not working**: `handle_paste()` method exists but is never called from `handle_key_input()`. Need to add Ctrl+Shift+V handling.

2. **No CLI argument parsing**: Config only loads from file, no `--config` override.

3. **No keybinding customization**: Shortcuts are hardcoded.

4. **No search UI**: No find bar or search functionality.

5. **No config reload**: Must restart terminal to apply config changes.
