# Mochi Terminal Phase 2 - Implementation Plan

## Scope

Transform the Mochi terminal emulator from "basic but working" to "modern and customizable" by implementing:
1. Robust configuration system with CLI overrides
2. Theme system with runtime switching
3. Font customization with runtime reload
4. Keybinding customization
5. UX polish (selection, search, hyperlinks)
6. Security hardening for escape sequences
7. Comprehensive testing and documentation

## Milestones

### M1 - Config System Foundation

**Deliverables:**
- [ ] Add clap for CLI argument parsing
- [ ] Implement `--config` flag for custom config path
- [ ] Add environment variable support (MOCHI_*)
- [ ] Implement config precedence: CLI > env > file > defaults
- [ ] Add detailed validation with error messages
- [ ] Create `docs/terminal/config.example.toml`
- [ ] Create `docs/terminal/config.md` schema documentation

**Tests:**
- [ ] Parse valid config (multiple variants)
- [ ] Reject invalid config with useful error
- [ ] Precedence tests (CLI overrides config, etc.)
- [ ] Snapshot tests for effective settings

### M2 - Themes + Light/Dark Mode

**Deliverables:**
- [ ] Verify existing themes: mochi-dark, mochi-light, solarized-dark, solarized-light, dracula, nord
- [ ] Add runtime theme switching via keybinding (Ctrl+Shift+T)
- [ ] Document theme format in `docs/terminal/themes.md`
- [ ] Add custom theme file loading support

**Tests:**
- [ ] Theme parsing unit tests
- [ ] Theme application to renderer
- [ ] ANSI color mapping verification
- [ ] Screenshots: theme_dark.png, theme_light.png, theme_palette_grid.png

### M3 - Font Customization + Layout

**Deliverables:**
- [ ] Add font fallback chain configuration
- [ ] Add cell padding (x/y) configuration
- [ ] Add line height multiplier configuration
- [ ] Implement runtime font reload without restart
- [ ] Handle missing fonts gracefully with fallback
- [ ] Document font discovery in `docs/terminal/fonts.md`

**Tests:**
- [ ] Config parsing for font settings
- [ ] Pixel size -> rows/cols mapping
- [ ] PTY resize on font change
- [ ] Screenshots: font_small.png, font_medium.png, font_large.png

### M4 - Keybinding Customization

**Deliverables:**
- [ ] **FIX PASTE BUG**: Wire up handle_paste() to Ctrl+Shift+V
- [ ] Add keybinding configuration section to config
- [ ] Implement default shortcuts:
  - Copy: Ctrl+Shift+C
  - Paste: Ctrl+Shift+V
  - Find: Ctrl+Shift+F
  - Reload config: Ctrl+Shift+R
  - Toggle theme: Ctrl+Shift+T
- [ ] Support modifiers: Ctrl/Alt/Shift/Super
- [ ] Document keybindings in `docs/terminal/keybindings.md`

**Tests:**
- [ ] Keybinding config parsing
- [ ] Key event -> action mapping
- [ ] Manual test checklist for all shortcuts

### M5 - UX Polish: Selection, Search, Hyperlinks

**Deliverables:**
- [ ] Implement double-click word selection
- [ ] Implement triple-click line selection
- [ ] Add scrollback search UI overlay
- [ ] Implement search highlighting
- [ ] Add Enter/Shift+Enter for next/prev match
- [ ] Implement OSC 8 hyperlink rendering
- [ ] Add URL detection (optional)
- [ ] Ensure no auto-open of links

**Tests:**
- [ ] Selection range logic
- [ ] Word boundary detection
- [ ] Search indexing and navigation
- [ ] Screenshots: selection_word.png, search_bar.png, hyperlink_hover.png

### M6 - Config Reload + Safety

**Deliverables:**
- [ ] Implement config reload via Ctrl+Shift+R
- [ ] Add optional file watcher (inotify)
- [ ] Handle reload failure gracefully (keep old config)
- [ ] Verify OSC 52 clipboard is disabled by default
- [ ] Add max payload size enforcement
- [ ] Add user-visible indication for clipboard modification
- [ ] Implement title update throttling
- [ ] Create `docs/terminal/security.md`

**Tests:**
- [ ] Reload success updates settings
- [ ] Reload failure keeps old settings
- [ ] Clipboard payload size limit enforcement

### M7 - Compatibility and Regression Testing

**Deliverables:**
- [ ] Run vttest and document results
- [ ] Fix any regressions from Phase 2
- [ ] Document pre-existing bugs as issues
- [ ] Create `docs/terminal/phase2/compatibility.md`

## Acceptance Criteria Checklist

### Build & Run
- [ ] Terminal builds in CI and locally
- [ ] Terminal launches and runs interactive shell
- [ ] No crashes on resize, font change, theme change, config reload

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
- [ ] Search bar works with highlights + next/prev navigation
- [ ] Hyperlinks are safe (no auto-open) and have usable UX

### Security
- [ ] Clipboard escape sequences are guarded with safe defaults
- [ ] Title update behavior cannot be abused (throttling)
- [ ] docs/terminal/security.md updated

### Compatibility & Regression
- [ ] vttest run notes documented
- [ ] All automated tests pass
- [ ] No TODOs/placeholders left
- [ ] Documentation updated everywhere

## Commit Plan

Each commit follows format: `phase2(Mx.y): short description`

1. `phase2(M1.1): add clap CLI argument parsing`
2. `phase2(M1.2): implement config precedence with env vars`
3. `phase2(M1.3): add config validation and error messages`
4. `phase2(M1.4): add config documentation and example`
5. `phase2(M2.1): add runtime theme switching keybinding`
6. `phase2(M2.2): add custom theme file loading`
7. `phase2(M2.3): add theme documentation`
8. `phase2(M3.1): add font fallback chain support`
9. `phase2(M3.2): add cell padding and line height config`
10. `phase2(M3.3): implement runtime font reload`
11. `phase2(M4.1): fix paste bug - wire handle_paste to keybinding`
12. `phase2(M4.2): add keybinding configuration system`
13. `phase2(M4.3): implement all default keybindings`
14. `phase2(M5.1): implement word and line selection`
15. `phase2(M5.2): add scrollback search UI`
16. `phase2(M5.3): implement hyperlink rendering and interaction`
17. `phase2(M6.1): implement config reload keybinding`
18. `phase2(M6.2): add security guards for OSC sequences`
19. `phase2(M6.3): add title update throttling`
20. `phase2(M7.1): run vttest and document results`
21. `phase2(M7.2): fix any regressions and finalize docs`

## Timeline

This is a comprehensive implementation that will be done in a single PR with many commits. Each milestone builds on the previous, with testing at each step.
