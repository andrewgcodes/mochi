# Mochi Terminal Phase 2 - Implementation Plan

## Scope

Transform the Mochi terminal emulator from "basic but working" to "modern and customizable" by implementing:

1. Robust configuration system with CLI overrides and XDG support
2. Theme system with runtime switching
3. Font customization with fallback support
4. Keybinding customization
5. UX polish: improved selection, scrollback search, hyperlink UX
6. Security hardening for dangerous escape sequences
7. Comprehensive test coverage

## Branch

`phase2-modern-config-theming`

## Milestones and Commit Plan

### M1: Config System Foundation

| Commit | Description |
|--------|-------------|
| M1.1 | Add clap for CLI argument parsing with --config flag |
| M1.2 | Implement XDG config path resolution with proper fallbacks |
| M1.3 | Add environment variable support for config overrides |
| M1.4 | Add config validation with clear error messages |
| M1.5 | Create example config file and schema documentation |
| M1.6 | Add config precedence tests |

### M2: Themes + Light/Dark Mode

| Commit | Description |
|--------|-------------|
| M2.1 | Refactor theme system to support external theme files |
| M2.2 | Add runtime theme switching via Ctrl+Shift+T keybinding |
| M2.3 | Add two additional curated themes (Gruvbox, OneDark) |
| M2.4 | Document theme format in docs/terminal/themes.md |
| M2.5 | Add theme parsing and application tests |
| M2.6 | Take theme screenshots |

### M3: Font Customization + Layout

| Commit | Description |
|--------|-------------|
| M3.1 | Add font family configuration with system font support |
| M3.2 | Add fallback fonts list configuration |
| M3.3 | Add cell padding and line height configuration |
| M3.4 | Ensure PTY resize on font metric changes |
| M3.5 | Add font configuration tests |
| M3.6 | Take font size screenshots |

### M4: Keybinding Customization

| Commit | Description |
|--------|-------------|
| M4.1 | Add keybinding configuration structure |
| M4.2 | Implement keybinding parser for modifier+key combos |
| M4.3 | Add default keybindings: copy, paste, find, reload, toggle-theme |
| M4.4 | Wire up keybinding actions in app event loop |
| M4.5 | Add keybinding parsing tests |
| M4.6 | Document keybindings in docs/terminal/keybindings.md |

### M5: UX Polish

| Commit | Description |
|--------|-------------|
| M5.1 | Add word selection on double-click |
| M5.2 | Add line selection on triple-click |
| M5.3 | Implement scrollback search UI with find bar overlay |
| M5.4 | Add search highlight rendering and next/prev navigation |
| M5.5 | Improve hyperlink UX with hover indication |
| M5.6 | Add selection and search tests |
| M5.7 | Take UX screenshots |

### M6: Config Reload + Security

| Commit | Description |
|--------|-------------|
| M6.1 | Implement config reload via Ctrl+Shift+R keybinding |
| M6.2 | Add error handling for reload failures (keep old config) |
| M6.3 | Add optional file watcher for auto-reload |
| M6.4 | Verify OSC 52 clipboard security (disabled by default, size limits) |
| M6.5 | Add title update throttling |
| M6.6 | Update docs/terminal/security.md |
| M6.7 | Add config reload tests |

### M7: Compatibility and Regression Testing

| Commit | Description |
|--------|-------------|
| M7.1 | Run vttest and document results |
| M7.2 | Fix any regressions found |
| M7.3 | Create compatibility documentation |
| M7.4 | Final cleanup and documentation review |

## Acceptance Criteria Checklist

### Build & Run
- [ ] Terminal builds in CI and locally
- [ ] Terminal launches and runs an interactive shell
- [ ] No crashes when resizing, changing font, changing theme, reloading config

### Config
- [ ] Config file loads from XDG default path (~/.config/mochi/config.toml)
- [ ] --config override works
- [ ] Config precedence works (CLI > env > file > defaults) and is tested
- [ ] Example config (docs/terminal/config.example.toml) exists
- [ ] Config schema docs (docs/terminal/config.md) exist

### Themes
- [ ] Built-in mochi-dark and mochi-light exist
- [ ] User can switch theme at runtime (Ctrl+Shift+T)
- [ ] ANSI palette is correct and tested
- [ ] Screenshots exist showing both themes

### Fonts
- [ ] User can set font family and size via config
- [ ] Fallback behavior is documented and works
- [ ] Font changes update grid size and PTY rows/cols correctly
- [ ] Screenshots exist for font sizing

### Keybindings
- [ ] Default shortcuts work: copy (Ctrl+Shift+C), paste (Ctrl+Shift+V), find (Ctrl+Shift+F), reload (Ctrl+Shift+R), toggle-theme (Ctrl+Shift+T)
- [ ] User can override keybindings in config
- [ ] Keybinding parsing is tested

### UX Polish
- [ ] Word selection (double-click) works
- [ ] Line selection (triple-click) works
- [ ] Search bar works and highlights matches + next/prev navigation
- [ ] Hyperlinks are safe (no auto-open) and have usable UX

### Security
- [ ] Clipboard escape sequences are guarded with safe defaults, size limits, and user visibility
- [ ] Title update behavior cannot be abused trivially (throttling/limits)
- [ ] docs/terminal/security.md updated

### Compatibility & Regression
- [ ] vttest run notes are documented
- [ ] All automated tests pass
- [ ] No TODOs/placeholders left
- [ ] Documentation updated everywhere

## Testing Strategy

### Unit Tests
- Config parsing/validation
- Theme parsing + application
- Keybinding parsing + event mapping
- Selection math (word/line boundaries)
- Search indexing

### Golden Tests
- ANSI palette grid test
- Theme switch test

### Integration Tests
- PTY resize on font change
- Config reload

### Manual Tests
- Run terminal with various apps (vim, htop, tmux)
- Test all keybindings
- Test theme switching
- Test font changes
- Take screenshots

## Screenshots Location

All screenshots stored in `docs/terminal/phase2/screenshots/`:
- `baseline/` - Baseline screenshots before Phase 2
- `theme_dark.png` - Dark theme
- `theme_light.png` - Light theme
- `theme_palette_grid.png` - ANSI color palette
- `font_small.png`, `font_medium.png`, `font_large.png` - Font sizes
- `selection_word.png` - Word selection
- `search_bar.png` - Search UI
- `hyperlink_hover.png` - Hyperlink hover
