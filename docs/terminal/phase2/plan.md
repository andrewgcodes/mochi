# Mochi Terminal - Phase 2 Implementation Plan

## Scope

Transform the Mochi terminal from "basic but working" to "modern and customizable" by implementing:
- Robust configuration system with XDG support and CLI overrides
- Theme system with runtime switching
- Font customization with safe runtime reload
- Keybinding customization
- UX polish (selection, search, hyperlinks)
- Security hardening

## Milestones and Commit Plan

### M1: Config System Foundation
- [ ] Add clap dependency for CLI argument parsing
- [ ] Implement `--config` flag for config file override
- [ ] Implement XDG config path precedence
- [ ] Add config validation with clear error messages
- [ ] Create example config file
- [ ] Add comprehensive tests

Commits:
1. `phase2(M1.1): add clap dependency and CLI argument parsing`
2. `phase2(M1.2): implement XDG config path precedence`
3. `phase2(M1.3): add config validation with error messages`
4. `phase2(M1.4): add example config and documentation`
5. `phase2(M1.5): add config system tests`

### M2: Themes + Light/Dark Mode
- [ ] Verify 6 built-in themes work correctly
- [ ] Add runtime theme switching via keybinding
- [ ] Add theme configuration documentation
- [ ] Add theme tests and screenshots

Commits:
1. `phase2(M2.1): add runtime theme switching support`
2. `phase2(M2.2): add theme toggle keybinding (Ctrl+Shift+T)`
3. `phase2(M2.3): add theme tests and documentation`

### M3: Font Customization
- [ ] Add font family configuration
- [ ] Add font fallback list support
- [ ] Add cell padding/line height configuration
- [ ] Implement runtime font reload
- [ ] Add font tests and screenshots

Commits:
1. `phase2(M3.1): add font family and fallback configuration`
2. `phase2(M3.2): implement runtime font reload`
3. `phase2(M3.3): add font customization tests`

### M4: Keybinding Customization
- [ ] Create keybinding system
- [ ] Add default shortcuts (copy, paste, find, reload, toggle theme)
- [ ] **FIX PASTE BUG**: Wire up handle_paste() to Ctrl+Shift+V
- [ ] Add keybinding configuration support
- [ ] Add keybinding tests

Commits:
1. `phase2(M4.1): add keybinding system with default shortcuts`
2. `phase2(M4.2): fix paste functionality (Ctrl+Shift+V)`
3. `phase2(M4.3): add copy functionality (Ctrl+Shift+C)`
4. `phase2(M4.4): add keybinding configuration support`
5. `phase2(M4.5): add keybinding tests`

### M5: UX Polish
- [ ] Implement mouse selection (click+drag, double-click word, triple-click line)
- [ ] Implement scrollback search UI
- [ ] Improve hyperlink UX
- [ ] Add selection and search tests

Commits:
1. `phase2(M5.1): implement mouse selection with word/line modes`
2. `phase2(M5.2): implement scrollback search UI`
3. `phase2(M5.3): improve hyperlink UX`
4. `phase2(M5.4): add selection and search tests`

### M6: Config Reload + Security
- [ ] Add config reload keybinding (Ctrl+Shift+R)
- [ ] Add file watcher for auto-reload (optional)
- [ ] Implement security hardening for clipboard sequences
- [ ] Add title update throttling
- [ ] Update security documentation

Commits:
1. `phase2(M6.1): add config reload keybinding`
2. `phase2(M6.2): add clipboard sequence security controls`
3. `phase2(M6.3): add title update throttling`
4. `phase2(M6.4): update security documentation`

### M7: No Regressions
- [ ] Run vttest and document results
- [ ] Ensure all tests pass
- [ ] Fix any regressions
- [ ] Final documentation review

Commits:
1. `phase2(M7.1): add vttest compatibility notes`
2. `phase2(M7.2): final documentation and cleanup`

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

## Timeline

This is a living document. Progress will be tracked as commits are made.

## Known Constraints

1. Font is currently bundled (DejaVuSansMono.ttf) - need to support system fonts
2. No fontconfig integration yet - may need to add for font discovery
3. Paste functionality exists but is not wired up - critical bug to fix
