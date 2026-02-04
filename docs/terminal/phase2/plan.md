# Mochi Terminal - Phase 2 Implementation Plan

## Scope

Phase 2 modernizes the Mochi terminal emulator with:
1. Robust configuration system with XDG support and CLI overrides
2. Theme system with runtime switching
3. Font customization with runtime reload
4. Configurable keybindings
5. UX polish: selection improvements, scrollback search, hyperlink UX
6. Security hardening for escape sequences
7. Bug fixes (paste functionality)

## Branch

`phase2-modern-config-theming`

## Commit Convention

Format: `phase2(Mx.y): short description`

Example: `phase2(M1.2): add XDG config loader with precedence and tests`

## Milestones

### M1: Config System Foundation

**Commits:**
- M1.1: Add clap for CLI argument parsing
- M1.2: Implement XDG config path resolution
- M1.3: Add config validation with error messages
- M1.4: Implement config precedence (CLI > env > file > defaults)
- M1.5: Create example config and documentation
- M1.6: Add config parsing tests

**Deliverables:**
- `--config` CLI flag for custom config path
- XDG_CONFIG_HOME support (~/.config/mochi/config.toml)
- Clear error messages for invalid config
- docs/terminal/config.example.toml
- docs/terminal/config.md

### M2: Themes + Light/Dark Mode

**Commits:**
- M2.1: Refactor theme system for runtime switching
- M2.2: Add theme toggle keybinding (Ctrl+Shift+T)
- M2.3: Ensure all 6 themes are complete and correct
- M2.4: Add theme tests and screenshots

**Deliverables:**
- Runtime theme switching via Ctrl+Shift+T
- 6 built-in themes: Dark, Light, SolarizedDark, SolarizedLight, Dracula, Nord
- Theme affects: background, foreground, ANSI palette, selection, cursor

### M3: Font Customization + Layout

**Commits:**
- M3.1: Add font family configuration
- M3.2: Add font fallback support
- M3.3: Implement runtime font reload
- M3.4: Add font configuration tests

**Deliverables:**
- Configurable font family, size, weight
- Fallback fonts for missing glyphs
- Runtime font changes update grid and PTY size
- Graceful handling of missing fonts

### M4: Keybinding Customization

**Commits:**
- M4.1: Create keybinding system infrastructure
- M4.2: Implement default keybindings (copy/paste/find/reload/theme)
- M4.3: Add keybinding configuration parsing
- M4.4: Fix paste functionality bug
- M4.5: Add keybinding tests

**Deliverables:**
- Default shortcuts: Ctrl+Shift+C (copy), Ctrl+Shift+V (paste), Ctrl+Shift+F (find), Ctrl+Shift+R (reload), Ctrl+Shift+T (theme)
- User-configurable keybindings in config
- Working paste functionality

### M5: UX Polish

**Commits:**
- M5.1: Implement mouse selection (click+drag)
- M5.2: Add word selection (double-click)
- M5.3: Add line selection (triple-click)
- M5.4: Implement scrollback search UI
- M5.5: Add search navigation (Enter/Shift+Enter)
- M5.6: Improve hyperlink UX (Ctrl+click)
- M5.7: Add selection and search tests

**Deliverables:**
- Mouse-based text selection
- Word/line selection with double/triple click
- Search bar overlay with match highlighting
- Safe hyperlink handling (no auto-open)

### M6: Config Reload + Security

**Commits:**
- M6.1: Implement config reload via keybinding
- M6.2: Add reload error handling (keep previous config)
- M6.3: Add title update throttling
- M6.4: Verify OSC 52 clipboard security defaults
- M6.5: Update security documentation

**Deliverables:**
- Ctrl+Shift+R reloads config at runtime
- Graceful reload failure handling
- Title update throttling to prevent DoS
- OSC 52 clipboard disabled by default
- Updated docs/terminal/security.md

### M7: Compatibility & Regression Testing

**Commits:**
- M7.1: Run vttest and document results
- M7.2: Fix any regressions found
- M7.3: Final documentation updates

**Deliverables:**
- docs/terminal/phase2/compatibility.md with vttest results
- All tests passing
- No regressions from Phase 2 changes

## Acceptance Criteria Checklist

### Build & Run
- [ ] Terminal builds in CI and locally
- [ ] Terminal launches and runs interactive shell
- [ ] No crashes on resize, font change, theme change, config reload

### Config
- [ ] Config loads from XDG default path
- [ ] --config override works
- [ ] Config precedence works and tested
- [ ] Example config and docs exist

### Themes
- [ ] Built-in mochi-dark and mochi-light exist
- [ ] User can switch theme at runtime (Ctrl+Shift+T)
- [ ] ANSI palette correct and tested
- [ ] Screenshots exist for themes

### Fonts
- [ ] User can set font family and size via config
- [ ] Fallback behavior documented and works
- [ ] Font changes update grid size and PTY correctly
- [ ] Screenshots exist for font sizing

### Keybindings
- [ ] Default shortcuts work: copy/paste/find/reload/theme
- [ ] User can override keybindings in config
- [ ] Keybinding parsing tested

### UX Polish
- [ ] Word/line selection works
- [ ] Search bar works with highlights + navigation
- [ ] Hyperlinks safe (no auto-open)

### Security
- [ ] Clipboard escape sequences guarded
- [ ] Title update throttling implemented
- [ ] docs/terminal/security.md updated

### Compatibility
- [ ] vttest results documented
- [ ] All automated tests pass
- [ ] No TODOs/placeholders left

## Testing Strategy

### Unit Tests
- Config parsing/validation
- Theme parsing/application
- Keybinding parsing/event mapping
- Selection math
- Search indexing

### Integration Tests
- PTY resize on font change
- Config reload

### Manual Tests
- Theme switching screenshots
- Font size screenshots
- Selection screenshots
- Search bar screenshots

## Timeline

Each milestone should be completed with working tests before moving to the next. Commits should be atomic and each should compile and pass tests.

## Known Issues to Address

1. **Paste bug**: `handle_paste()` in app.rs is never called
2. **No mouse selection**: Selection struct exists but not wired to mouse events
3. **No search UI**: Need to implement search overlay
4. **No keybinding system**: All shortcuts are hardcoded
