# Phase 2 Architecture Notes

This document captures key architectural observations and decisions for Phase 2 implementation.

## Current Architecture Summary

The Mochi terminal is organized into four crates:

### Crate Hierarchy

```
mochi-term (GUI application)
    |
    +-- terminal-core (screen model, selection, scrollback)
    +-- terminal-parser (escape sequence parsing)
    +-- terminal-pty (Linux PTY management)
```

### Key Data Flow

1. **Input**: User Input -> winit Event -> encode_key() -> PTY write -> Child Process
2. **Output**: Child Process -> PTY read -> Parser -> Actions -> Screen -> Renderer -> Display

## Where Phase 2 Features Fit

### M1: Config System Foundation

**Current State**: Basic config exists in `mochi-term/src/config.rs` with:
- `Config` struct with font, colors, dimensions, theme, OSC52 settings
- `ColorScheme` struct with foreground, background, cursor, selection, ANSI 16
- `ThemeName` enum with Dark, Light, SolarizedDark, SolarizedLight, Dracula, Nord, Custom
- XDG config path via `dirs::config_dir()`
- TOML serialization via `serde` and `toml`

**Changes Needed**:
- Add CLI argument parsing (clap) for `--config` override
- Add environment variable support (MOCHI_CONFIG, MOCHI_FONT_SIZE, etc.)
- Implement proper precedence: CLI > env vars > config file > defaults
- Add config validation with clear error messages
- Create example config and schema documentation

### M2: Themes + Light/Dark Mode

**Current State**: Themes already exist with 6 built-in themes:
- mochi-dark (default)
- mochi-light
- solarized-dark
- solarized-light
- dracula
- nord

**Changes Needed**:
- Add runtime theme switching via keybinding (Ctrl+Shift+T)
- Ensure theme changes propagate to renderer immediately
- Add custom theme file loading from external TOML
- Document theme format

### M3: Font Customization + Layout

**Current State**: Font handling in `mochi-term/src/renderer.rs`:
- Bundled DejaVuSansMono.ttf and DejaVuSansMono-Bold.ttf
- Font rasterization via fontdue
- Glyph caching in HashMap
- Cell size calculation based on font metrics
- `set_font_size()` method exists for runtime changes

**Changes Needed**:
- Add font family configuration (load from system fonts)
- Add fallback font chain for missing glyphs
- Add cell padding / line height configuration
- Ensure font changes trigger PTY resize (TIOCSWINSZ)
- Handle missing fonts gracefully

### M4: Keybinding Customization

**Current State**: Hardcoded shortcuts in `mochi-term/src/app.rs`:
- Ctrl+=/Ctrl+- for font zoom
- Ctrl+0 for font reset
- No copy/paste/find shortcuts yet

**Changes Needed**:
- Create keybinding configuration system
- Add default shortcuts: copy (Ctrl+Shift+C), paste (Ctrl+Shift+V), find (Ctrl+Shift+F), reload (Ctrl+Shift+R), toggle-theme (Ctrl+Shift+T)
- Parse keybinding config from TOML
- Map key events to actions

### M5: UX Polish

**Current State**:
- Selection exists in `terminal-core/src/selection.rs` with Normal, Word, Line, Block types
- Selection rendering in renderer
- No scrollback search UI
- OSC 8 hyperlink parsing exists in parser

**Changes Needed**:
- Implement double-click word selection, triple-click line selection
- Add scrollback search UI (find bar overlay)
- Improve hyperlink UX (underline, ctrl+click to open)

### M6: Config Reload + Security

**Current State**:
- OSC 52 clipboard disabled by default
- OSC 52 max size configurable
- No config reload mechanism

**Changes Needed**:
- Add runtime config reload via keybinding
- Add file watcher (optional, inotify)
- Add title update throttling
- Document security considerations

## Key Files to Modify

| File | Purpose | Phase 2 Changes |
|------|---------|-----------------|
| `mochi-term/src/main.rs` | Entry point | Add CLI parsing |
| `mochi-term/src/config.rs` | Configuration | Expand with CLI/env, validation |
| `mochi-term/src/app.rs` | Application state | Add keybindings, search UI |
| `mochi-term/src/renderer.rs` | Rendering | Font fallback, theme updates |
| `mochi-term/src/terminal.rs` | Terminal logic | Security hardening |
| `terminal-core/src/selection.rs` | Selection | Word/line selection logic |

## Dependencies to Add

- `clap` - CLI argument parsing
- `notify` (optional) - File watching for config reload

## Testing Strategy

1. **Unit Tests**: Config parsing, keybinding mapping, selection logic
2. **Golden Tests**: Theme application, ANSI palette rendering
3. **Integration Tests**: PTY resize on font change
4. **Manual Tests**: Visual inspection of themes, fonts, selection

## Risks and Mitigations

1. **Font loading failures**: Always have bundled fallback font
2. **Config parse errors**: Show clear error, use previous/default config
3. **Theme switch flicker**: Batch renderer updates
4. **Keybinding conflicts**: Document reserved combinations
