# Mochi Terminal Phase 2 Plan

## Overview

Phase 2 transforms Mochi Terminal from "basic but working" to "modern and customizable" by adding a robust configuration system, theme support, font customization, keybinding customization, UX polish, and security hardening.

## Scope

### In Scope

- Robust configuration system with CLI overrides and precedence
- Theme engine with built-in themes and custom theme loading
- Runtime theme switching via keybinding
- Font customization (family, size, fallback, spacing)
- Keybinding customization for common actions
- Selection improvements (word/line selection)
- Scrollback search UI
- Hyperlink UX improvements
- Security hardening for escape sequences
- Comprehensive testing and documentation

### Out of Scope

- GPU rendering (future phase)
- Tabs/splits (future phase)
- Shell integration (future phase)
- Windows/macOS native builds (Linux only for now)

## Milestones and Commit Plan

### M1: Config System Foundation

**Goal:** Robust configuration with CLI overrides, env vars, and clear precedence.

Commits:
- `phase2(M1.1): add clap for CLI argument parsing`
- `phase2(M1.2): implement config precedence (CLI > env > file > defaults)`
- `phase2(M1.3): add config validation with clear error messages`
- `phase2(M1.4): add config.example.toml and config.md documentation`
- `phase2(M1.5): add config parsing and precedence tests`

Deliverables:
- `--config /path/to/config.toml` CLI flag
- `MOCHI_CONFIG` environment variable support
- Clear error messages for invalid config
- `docs/terminal/config.example.toml`
- `docs/terminal/config.md`

### M2: Themes + Light/Dark Mode

**Goal:** Theme engine with built-in themes and runtime switching.

Commits:
- `phase2(M2.1): add theme file loading from external TOML files`
- `phase2(M2.2): implement runtime theme switching`
- `phase2(M2.3): add Ctrl+Shift+T keybinding for theme toggle`
- `phase2(M2.4): add theme documentation and screenshots`
- `phase2(M2.5): add theme parsing and ANSI palette tests`

Deliverables:
- Built-in themes: mochi-dark, mochi-light, solarized-dark, solarized-light, dracula, nord
- Custom theme loading from file
- Runtime theme switching via Ctrl+Shift+T
- `docs/terminal/themes.md`
- Screenshots of all themes

### M3: Font Customization + Layout

**Goal:** Configurable fonts with fallback and runtime reload.

Commits:
- `phase2(M3.1): add font family configuration`
- `phase2(M3.2): implement font fallback chain`
- `phase2(M3.3): add cell padding and line height configuration`
- `phase2(M3.4): implement font hot-reload on config change`
- `phase2(M3.5): add font documentation and tests`

Deliverables:
- Configurable font_family, font_size, font_weight
- Fallback fonts for missing glyphs
- Cell padding (x/y) configuration
- Line height multiplier
- `docs/terminal/fonts.md`
- Screenshots at different font sizes

### M4: Keybinding Customization

**Goal:** Configurable keybindings for common actions.

Commits:
- `phase2(M4.1): add keybinding configuration structure`
- `phase2(M4.2): implement keybinding parser and matcher`
- `phase2(M4.3): add default keybindings (copy/paste/find/reload/toggle-theme)`
- `phase2(M4.4): integrate keybindings into app event handling`
- `phase2(M4.5): add keybinding documentation and tests`

Deliverables:
- Configurable keybindings in config.toml
- Default shortcuts: Ctrl+Shift+C (copy), Ctrl+Shift+V (paste), Ctrl+Shift+F (find), Ctrl+Shift+R (reload), Ctrl+Shift+T (toggle-theme)
- `docs/terminal/keybindings.md`

### M5: UX Polish

**Goal:** Selection improvements, search UI, hyperlink UX.

Commits:
- `phase2(M5.1): implement double-click word selection`
- `phase2(M5.2): implement triple-click line selection`
- `phase2(M5.3): add scrollback search UI overlay`
- `phase2(M5.4): implement search highlighting and navigation`
- `phase2(M5.5): improve hyperlink rendering and interaction`
- `phase2(M5.6): add selection and search tests`

Deliverables:
- Double-click selects word
- Triple-click selects line
- Find bar overlay (Ctrl+Shift+F)
- Search highlights with Enter/Shift+Enter navigation
- Hyperlinks underlined and clickable (Ctrl+click)
- Screenshots of selection and search

### M6: Config Reload + Safety

**Goal:** Runtime config reload and security hardening.

Commits:
- `phase2(M6.1): implement config reload via keybinding`
- `phase2(M6.2): add config reload error handling`
- `phase2(M6.3): add optional file watcher for auto-reload`
- `phase2(M6.4): add OSC 52 clipboard guards and user indication`
- `phase2(M6.5): add title update throttling`
- `phase2(M6.6): add security documentation`

Deliverables:
- Ctrl+Shift+R reloads config
- Error display on reload failure
- Optional inotify file watcher
- OSC 52 disabled by default, max size enforced
- Title update rate limiting
- `docs/terminal/security.md`

### M7: Compatibility and Regression Testing

**Goal:** Ensure no regressions and document compatibility.

Commits:
- `phase2(M7.1): run vttest and document results`
- `phase2(M7.2): fix any regressions found`
- `phase2(M7.3): add compatibility documentation`

Deliverables:
- vttest results documented
- Any regressions fixed
- `docs/terminal/phase2/compatibility.md`

## Acceptance Criteria Checklist

### Build & Run
- [ ] Terminal builds in CI and locally
- [ ] Terminal launches and runs an interactive shell
- [ ] No crashes when resizing, changing font, changing theme, reloading config

### Config
- [ ] Config file loads from XDG default path (~/.config/mochi/config.toml)
- [ ] --config override works
- [ ] MOCHI_CONFIG env var works
- [ ] Config precedence works and is tested (CLI > env > file > defaults)
- [ ] Invalid config shows clear error message
- [ ] Example config exists (docs/terminal/config.example.toml)
- [ ] Config documentation exists (docs/terminal/config.md)

### Themes
- [ ] Built-in mochi-dark and mochi-light exist
- [ ] Additional themes exist (solarized-dark, solarized-light, dracula, nord)
- [ ] Custom theme loading from file works
- [ ] User can switch theme at runtime (Ctrl+Shift+T)
- [ ] ANSI palette is correct and tested
- [ ] Screenshots exist showing all themes

### Fonts
- [ ] User can set font family via config
- [ ] User can set font size via config
- [ ] Fallback fonts work for missing glyphs
- [ ] Cell padding is configurable
- [ ] Font changes update grid size and PTY rows/cols correctly
- [ ] Font documentation exists
- [ ] Screenshots exist for font sizing

### Keybindings
- [ ] Default shortcuts work: copy (Ctrl+Shift+C), paste (Ctrl+Shift+V), find (Ctrl+Shift+F), reload (Ctrl+Shift+R), toggle-theme (Ctrl+Shift+T)
- [ ] User can override keybindings in config
- [ ] Keybinding parsing is tested
- [ ] Keybinding documentation exists

### UX Polish
- [ ] Single click + drag selects text
- [ ] Double-click selects word
- [ ] Triple-click selects line
- [ ] Selection respects wrapped lines
- [ ] Search bar works (Ctrl+Shift+F)
- [ ] Search highlights matches
- [ ] Enter/Shift+Enter navigate search results
- [ ] Esc closes search bar
- [ ] Hyperlinks are rendered (underlined)
- [ ] Hyperlinks are safe (no auto-open)
- [ ] Ctrl+click opens/copies hyperlink

### Security
- [ ] OSC 52 clipboard is disabled by default
- [ ] OSC 52 has max payload size enforced
- [ ] OSC 52 shows user-visible indication when clipboard modified
- [ ] Title update has throttling/limits
- [ ] Security documentation exists (docs/terminal/security.md)

### Compatibility & Regression
- [ ] vttest run notes are documented
- [ ] All automated tests pass
- [ ] No TODOs/placeholders left in code
- [ ] Documentation updated everywhere

## Testing Strategy

### Unit Tests
- Config parsing and validation
- Theme parsing and application
- Keybinding parsing and event mapping
- Selection math (word boundaries, line selection)
- Search indexing and navigation

### Golden Tests
- ANSI palette grid (verify colors)
- Theme switch (same input, different theme)
- Font size change (layout verification)

### Integration Tests
- PTY spawn and resize
- Config reload success/failure
- Keybinding dispatch

### Manual Tests
- All keybindings work
- Theme switching visual verification
- Font changes visual verification
- Selection behavior
- Search UI
- Hyperlink interaction

## Timeline

This is a rough estimate; actual time will depend on complexity discovered during implementation.

- M1 (Config): 2-3 hours
- M2 (Themes): 2-3 hours
- M3 (Fonts): 2-3 hours
- M4 (Keybindings): 2-3 hours
- M5 (UX Polish): 3-4 hours
- M6 (Safety): 2-3 hours
- M7 (Compatibility): 1-2 hours

Total: ~15-20 hours of implementation

## Risk Mitigation

1. **Font loading complexity**: Start with bundled fonts, add system font discovery as enhancement
2. **Search performance**: Use simple linear search initially, optimize if needed
3. **File watcher flakiness**: Make file watcher optional, ensure tests don't depend on it
4. **Theme hot-reload**: Ensure renderer can update colors without restart

## References

- XTerm Control Sequences: https://invisible-island.net/xterm/ctlseqs/ctlseqs.pdf
- ECMA-48: https://www.ecma-international.org/publications-and-standards/standards/ecma-48/
- vttest: Terminal emulation test suite
