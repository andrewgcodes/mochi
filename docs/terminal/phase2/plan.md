# Mochi Terminal Phase 2 Plan

## Scope

Transform the Mochi Terminal from "basic but working" to "modern and customizable" by implementing:

1. Robust configuration system with XDG conventions and CLI overrides
2. Theme engine with runtime switching
3. Font customization with safe runtime reload
4. Keybinding customization
5. UX polish (selection, search, hyperlinks)
6. Security hardening
7. Comprehensive testing

## Milestones and Commit Plan

### M1 - Config System Foundation

| Commit | Description |
|--------|-------------|
| M1.1 | Add clap for CLI argument parsing with --config flag |
| M1.2 | Implement config precedence (CLI > env > file > defaults) |
| M1.3 | Add config validation with clear error messages |
| M1.4 | Create example config and documentation |
| M1.5 | Add config parsing tests and precedence tests |

### M2 - Themes + Light/Dark Mode

| Commit | Description |
|--------|-------------|
| M2.1 | Add runtime theme switching infrastructure |
| M2.2 | Implement theme toggle keybinding (Ctrl+Shift+T) |
| M2.3 | Add custom theme file loading from config |
| M2.4 | Create theme documentation and examples |
| M2.5 | Add theme parsing and ANSI color tests |

### M3 - Font Customization + Layout

| Commit | Description |
|--------|-------------|
| M3.1 | Add font family configuration with fallback |
| M3.2 | Add cell padding and line height configuration |
| M3.3 | Implement runtime font reload with PTY resize |
| M3.4 | Add font configuration documentation |
| M3.5 | Add font configuration tests |

### M4 - Keybinding Customization

| Commit | Description |
|--------|-------------|
| M4.1 | Add keybinding configuration structure |
| M4.2 | Implement keybinding parser and action mapper |
| M4.3 | Add default keybindings (copy/paste/find/reload/toggle-theme) |
| M4.4 | Add keybinding documentation |
| M4.5 | Add keybinding parsing and mapping tests |

### M5 - UX Polish

| Commit | Description |
|--------|-------------|
| M5.1 | Implement word selection (double-click) |
| M5.2 | Implement line selection (triple-click) |
| M5.3 | Add scrollback search UI overlay |
| M5.4 | Add search navigation (Enter/Shift+Enter) |
| M5.5 | Improve hyperlink UX (Ctrl+click to open) |
| M5.6 | Add selection and search tests |

### M6 - Config Reload + Safety

| Commit | Description |
|--------|-------------|
| M6.1 | Implement config reload keybinding (Ctrl+Shift+R) |
| M6.2 | Add reload error handling (keep old config on failure) |
| M6.3 | Add title update throttling |
| M6.4 | Document security considerations |
| M6.5 | Add reload and security tests |

### M7 - Compatibility Testing

| Commit | Description |
|--------|-------------|
| M7.1 | Run vttest and document results |
| M7.2 | Fix any regressions found |
| M7.3 | Final documentation updates |

## Acceptance Criteria Checklist

### Build & Run
- [ ] Terminal builds in CI and locally
- [ ] Terminal launches and runs an interactive shell
- [ ] No crashes when resizing, changing font, changing theme, reloading config

### Config
- [ ] Config file loads from XDG default path (~/.config/mochi/config.toml)
- [ ] --config override works
- [ ] Config precedence works and is tested
- [ ] Example config (docs/terminal/config.example.toml) exists
- [ ] Config schema doc (docs/terminal/config.md) exists

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
- [ ] Default shortcuts work: copy/paste/find/reload/toggle-theme
- [ ] User can override keybindings in config
- [ ] Keybinding parsing is tested

### UX Polish
- [ ] Word selection (double-click) works
- [ ] Line selection (triple-click) works
- [ ] Search bar works and highlights matches + next/prev navigation
- [ ] Hyperlinks are safe (no auto-open) and have usable UX (Ctrl+click)

### Security
- [ ] Clipboard escape sequences (OSC 52) are disabled by default
- [ ] OSC 52 has max payload size limit
- [ ] Title update behavior has throttling
- [ ] docs/terminal/security.md exists and is updated

### Compatibility & Regression
- [ ] vttest run notes are documented (docs/terminal/phase2/compatibility.md)
- [ ] All automated tests pass
- [ ] No TODOs/placeholders left
- [ ] Documentation updated everywhere it should be

## Testing Strategy

### Unit Tests (T1)
- Config parsing/validation
- Theme parsing + application
- Keybinding parsing + event mapping
- Selection math (word boundaries, line selection)
- Search indexing

### Golden Tests (T2)
- ANSI palette grid test
- Theme switch test

### Integration Tests (T3)
- PTY resize propagation when font changes
- Config reload behavior

### Manual Tests (T4)
- Scripts in scripts/tests/manual/
- Screenshots in docs/terminal/phase2/screenshots/

## File Structure

```
docs/terminal/
├── config.md                    # Config schema documentation
├── config.example.toml          # Example configuration
├── security.md                  # Security documentation
└── phase2/
    ├── architecture-notes.md    # Architecture documentation
    ├── plan.md                  # This file
    ├── compatibility.md         # vttest results
    └── screenshots/
        ├── baseline/            # Before Phase 2
        ├── theme_dark.png
        ├── theme_light.png
        ├── font_small.png
        ├── font_medium.png
        ├── font_large.png
        ├── selection_word.png
        ├── search_bar.png
        └── hyperlink_hover.png
```

## Timeline

This plan will be executed in a single branch (`phase2-modern-config-theming`) with many small commits, each building and passing tests.
