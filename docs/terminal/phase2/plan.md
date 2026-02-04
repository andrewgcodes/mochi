# Phase 2 Implementation Plan

## Scope

Transform Mochi Terminal from "basic but working" to "modern and customizable" by adding:
- Robust configuration system with CLI overrides
- Theme system with runtime switching
- Font customization with safe reload
- Keybinding customization
- UX polish (selection, search, hyperlinks)
- Security hardening

## Milestones and Commit Plan

### M1: Config System Foundation
- [ ] M1.1: Add clap for CLI argument parsing (--config flag)
- [ ] M1.2: Add environment variable support (MOCHI_* vars)
- [ ] M1.3: Implement config precedence (CLI > env > file > defaults)
- [ ] M1.4: Add config validation with clear error messages
- [ ] M1.5: Create docs/terminal/config.example.toml
- [ ] M1.6: Create docs/terminal/config.md schema documentation
- [ ] M1.7: Add unit tests for config parsing and precedence

### M2: Themes + Light/Dark Mode
- [ ] M2.1: Rename existing themes to mochi-dark, mochi-light
- [ ] M2.2: Add runtime theme switching via keybinding
- [ ] M2.3: Add custom theme file loading
- [ ] M2.4: Document theme format in docs/terminal/themes.md
- [ ] M2.5: Add golden tests for ANSI palette
- [ ] M2.6: Take screenshots of themes

### M3: Font Customization + Layout
- [ ] M3.1: Add font family configuration
- [ ] M3.2: Add fallback font chain
- [ ] M3.3: Add cell padding / line height config
- [ ] M3.4: Ensure font changes trigger PTY resize
- [ ] M3.5: Handle missing fonts gracefully
- [ ] M3.6: Add tests for font config and resize
- [ ] M3.7: Take screenshots of font sizes

### M4: Keybinding Customization
- [ ] M4.1: Create keybinding configuration structure
- [ ] M4.2: Implement default shortcuts (copy/paste/find/reload/toggle-theme)
- [ ] M4.3: Add keybinding parsing from config
- [ ] M4.4: Add tests for keybinding parsing and mapping
- [ ] M4.5: Document keybindings in config.md

### M5: UX Polish
- [ ] M5.1: Implement double-click word selection
- [ ] M5.2: Implement triple-click line selection
- [ ] M5.3: Add scrollback search UI (find bar)
- [ ] M5.4: Add search highlighting and navigation
- [ ] M5.5: Improve hyperlink UX (underline, ctrl+click)
- [ ] M5.6: Add tests for selection and search
- [ ] M5.7: Take screenshots of selection and search

### M6: Config Reload + Security
- [ ] M6.1: Add runtime config reload via keybinding
- [ ] M6.2: Add file watcher (optional)
- [ ] M6.3: Handle reload failures gracefully
- [ ] M6.4: Add title update throttling
- [ ] M6.5: Document security in docs/terminal/security.md
- [ ] M6.6: Add tests for reload and security

### M7: No Regressions
- [ ] M7.1: Run vttest and document results
- [ ] M7.2: Fix any regressions
- [ ] M7.3: Update docs/terminal/phase2/compatibility.md
- [ ] M7.4: Final test pass

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

## Commit Message Format

All commits must follow this format:
```
phase2(Mx.y): short description
```

Examples:
- `phase2(M1.1): add clap CLI argument parsing`
- `phase2(M2.2): implement runtime theme switching`
- `phase2(M5.3): add scrollback search UI`

## Branch

Branch name: `phase2-modern-config-theming`

## Timeline

This is a living document. Update as implementation progresses.

Last updated: 2026-02-04
