# Mochi Terminal Phase 2 Plan

## Scope

Transform Mochi Terminal from "basic but working" to "modern and customizable" by implementing:
1. Robust configuration system with XDG support and CLI overrides
2. Themes with light/dark mode support and runtime switching
3. Font customization with runtime reload
4. Keybinding customization
5. UX polish (selection, scrollback search, hyperlinks)
6. Security hardening for escape sequences
7. Comprehensive test coverage

## Critical Bug Fix

**PASTE NOT WORKING**: The `handle_paste()` function exists but is never called. Must be fixed as part of M4/M5.

## Milestones

### M1: Config System Foundation
- [ ] Add `clap` dependency for CLI argument parsing
- [ ] Implement `--config` flag to override config path
- [ ] Add environment variable support (MOCHI_CONFIG, MOCHI_THEME, etc.)
- [ ] Implement config precedence: CLI > env vars > config file > defaults
- [ ] Add config validation with helpful error messages
- [ ] Create `docs/terminal/config.example.toml`
- [ ] Create `docs/terminal/config.md` schema documentation
- [ ] Add tests for config parsing, precedence, validation

### M2: Themes + Light/Dark Mode
- [ ] Rename existing themes to mochi-dark, mochi-light
- [ ] Ensure 4+ built-in themes (already have 6: dark, light, solarized-dark, solarized-light, dracula, nord)
- [ ] Add runtime theme switching via keybinding (Ctrl+Shift+T)
- [ ] Add custom theme file loading support
- [ ] Update renderer to apply theme changes without restart
- [ ] Add theme-related tests
- [ ] Take screenshots: theme_dark.png, theme_light.png, theme_palette_grid.png

### M3: Font Customization + Layout
- [ ] Add font family configuration (with system font discovery)
- [ ] Add font weight/style support
- [ ] Add fallback fonts list
- [ ] Add cell padding / line height configuration
- [ ] Implement runtime font reload
- [ ] Handle missing fonts gracefully
- [ ] Recompute cell size and PTY dimensions on font change
- [ ] Add font-related tests
- [ ] Take screenshots: font_small.png, font_medium.png, font_large.png

### M4: Keybinding Customization
- [ ] Create keybinding configuration structure
- [ ] Implement default keybindings:
  - Copy: Ctrl+Shift+C
  - Paste: Ctrl+Shift+V (FIX THE BUG!)
  - Find: Ctrl+Shift+F
  - Reload config: Ctrl+Shift+R
  - Toggle theme: Ctrl+Shift+T
- [ ] Add keybinding parsing and validation
- [ ] Ensure keybindings don't interfere with normal typing
- [ ] Add keybinding tests
- [ ] Document keybindings

### M5: UX Polish
- [ ] Selection improvements:
  - Single click + drag (already works)
  - Double click word select (already implemented in selection.rs)
  - Triple click line select (already implemented in selection.rs)
  - Wire up selection to mouse events in app.rs
- [ ] Scrollback search UI:
  - Find bar overlay
  - Highlight matches
  - Next/prev navigation (Enter/Shift+Enter)
  - Close with Esc
- [ ] Hyperlink UX:
  - Render OSC 8 hyperlinks with underline
  - Ctrl+click to open
  - Never auto-open
- [ ] Take screenshots: selection_word.png, search_bar.png, hyperlink_hover.png

### M6: Config Reload + Safety
- [ ] Implement config reload via keybinding (Ctrl+Shift+R)
- [ ] Optional: Add file watcher (inotify) for auto-reload
- [ ] Handle reload errors gracefully (keep old config, show error)
- [ ] Security hardening:
  - OSC 52 clipboard disabled by default (already done)
  - Max payload size (already done)
  - User indication when clipboard modified
  - Title update throttling
- [ ] Create `docs/terminal/security.md`
- [ ] Add reload and security tests

### M7: Compatibility Testing
- [ ] Run vttest and document results
- [ ] Fix any regressions
- [ ] Create `docs/terminal/phase2/compatibility.md`

## Commit Plan

Each commit should:
1. Compile successfully
2. Pass all tests
3. Update relevant docs/tests

Commit message format: `phase2(Mx.y): short description`

Example commits:
- `phase2(M1.1): add clap dependency and CLI argument parsing`
- `phase2(M1.2): implement config precedence with tests`
- `phase2(M4.1): fix paste keybinding - wire up handle_paste()`
- `phase2(M5.1): implement scrollback search UI`

## Acceptance Criteria

### Build & Run
- [ ] Terminal builds in CI and locally
- [ ] Terminal launches and runs an interactive shell
- [ ] No crashes when resizing, changing font, changing theme, reloading config

### Config
- [ ] Config file loads from XDG default path
- [ ] --config override works
- [ ] Config precedence works and is tested
- [ ] Example config and docs exist

### Themes
- [ ] Built-in mochi-dark and mochi-light exist
- [ ] User can switch theme at runtime (keybinding)
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
- [ ] Word/line selection works
- [ ] Search bar works and highlights matches + next/prev navigation
- [ ] Hyperlinks are safe (no auto-open) and have usable UX

### Security
- [ ] Clipboard escape sequences are guarded with safe defaults
- [ ] Title update behavior cannot be abused trivially
- [ ] docs/terminal/security.md updated

### Compatibility & Regression
- [ ] vttest run notes are documented
- [ ] All automated tests pass
- [ ] No TODOs/placeholders left
- [ ] Documentation updated everywhere

## Timeline

1. **Phase 2.1-2.4**: Reconnaissance and planning (DONE)
2. **M1**: Config system (1-2 commits)
3. **M4**: Keybindings + paste fix (1-2 commits) - prioritize paste fix
4. **M2**: Themes (1-2 commits)
5. **M3**: Fonts (1-2 commits)
6. **M5**: UX polish (2-3 commits)
7. **M6**: Config reload + security (1-2 commits)
8. **M7**: Compatibility testing (1 commit)
9. **Final**: Screenshots, video, PR
