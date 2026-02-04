# Phase 2 Implementation Plan

## Scope

Transform Mochi Terminal from "basic but working" to "modern and customizable" by implementing:
- Robust configuration system with XDG support and CLI overrides
- Theme system with 4+ built-in themes and runtime switching
- Font customization with fallback support
- Keybinding customization
- UX polish (selection improvements, search UI, hyperlink UX)
- Security hardening for escape sequences
- Comprehensive test coverage

## Milestones and Commit Plan

### M1 - Config System Foundation
- [x] M1.1: Add CLI argument parsing with `--config` flag
- [x] M1.2: Implement XDG config path resolution
- [x] M1.3: Add config validation with clear error messages
- [x] M1.4: Implement config precedence (CLI > env > file > defaults)
- [x] M1.5: Create example config and documentation
- [x] M1.6: Add config parsing tests

### M2 - Themes + Light/Dark Mode
- [x] M2.1: Define theme format and structure
- [x] M2.2: Implement mochi-dark and mochi-light themes
- [x] M2.3: Add 2 additional curated themes (Solarized, Nord already exist)
- [x] M2.4: Add runtime theme switching via keybinding
- [x] M2.5: Add theme parsing and application tests
- [x] M2.6: Take theme screenshots

### M3 - Font Customization + Layout
- [x] M3.1: Add font family configuration
- [x] M3.2: Add font size configuration with runtime reload
- [x] M3.3: Add fallback font support
- [x] M3.4: Implement cell padding/line height configuration
- [x] M3.5: Ensure PTY resize on font changes
- [x] M3.6: Add font configuration tests
- [x] M3.7: Take font screenshots

### M4 - Keybinding Customization
- [x] M4.1: Define keybinding configuration format
- [x] M4.2: Implement keybinding parser
- [x] M4.3: Add default keybindings (copy, paste, find, reload, toggle-theme)
- [x] M4.4: **FIX PASTE BUG**: Wire up Ctrl+Shift+V to handle_paste()
- [x] M4.5: Add keybinding tests
- [x] M4.6: Document keybindings

### M5 - UX Polish
- [x] M5.1: Implement word selection (double-click)
- [x] M5.2: Implement line selection (triple-click)
- [x] M5.3: Add scrollback search UI (find bar overlay)
- [x] M5.4: Add search highlighting and navigation
- [x] M5.5: Improve hyperlink UX (no auto-open, visual indication)
- [x] M5.6: Add selection and search tests
- [x] M5.7: Take UX screenshots

### M6 - Config Reload + Security Hardening
- [x] M6.1: Add config reload via keybinding
- [x] M6.2: Add graceful reload failure handling
- [x] M6.3: Ensure OSC 52 clipboard is disabled by default
- [x] M6.4: Add OSC 52 payload size limits
- [x] M6.5: Add title update throttling
- [x] M6.6: Update security documentation
- [x] M6.7: Add security tests

### M7 - Compatibility and Regression Testing
- [ ] M7.1: Run vttest and document results
- [ ] M7.2: Fix any regressions
- [ ] M7.3: Document compatibility notes
- [ ] M7.4: Ensure all tests pass

## Acceptance Criteria Checklist

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
- [ ] Clipboard escape sequences are guarded with safe defaults, size limits, and user visibility
- [ ] Title update behavior cannot be abused trivially (throttling/limits)
- [ ] docs/terminal/security.md updated

### Compatibility & Regression
- [ ] vttest run notes are documented
- [ ] All automated tests pass
- [ ] No TODOs/placeholders left
- [ ] Documentation updated everywhere it should be

## Known Issues to Fix

1. **Paste not working**: `handle_paste()` exists but is never called. Need to add Ctrl+Shift+V handling in `handle_key_input()`.

## Files to Create/Modify

### New Files
- `docs/terminal/config.example.toml` - Example configuration
- `docs/terminal/config.md` - Configuration documentation
- `docs/terminal/phase2/compatibility.md` - vttest results
- `docs/terminal/phase2/screenshots/*.png` - Feature screenshots

### Modified Files
- `terminal/mochi-term/src/main.rs` - CLI argument parsing
- `terminal/mochi-term/src/config.rs` - Extended config, validation
- `terminal/mochi-term/src/app.rs` - Keybindings, search UI, reload
- `terminal/mochi-term/src/renderer.rs` - Search highlighting, font changes
- `terminal/mochi-term/src/terminal.rs` - Title throttling
- `docs/terminal/security.md` - Security documentation updates
