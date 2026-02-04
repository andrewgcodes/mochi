# Mochi Terminal - Phase 2 Implementation Plan

## Scope

Transform Mochi Terminal from "basic but working" to "modern and customizable" by implementing:

1. Robust configuration system with CLI overrides
2. Theme system with runtime switching
3. Font customization with safe runtime reload
4. Keybinding customization
5. UX polish (selection, search, hyperlinks)
6. Security hardening for dangerous escape sequences
7. Comprehensive test coverage

## Branch

`phase2-modern-config-theming`

## Commit Convention

All commits follow the format: `phase2(Mx.y): short description`

Example: `phase2(M3.2): add XDG config loader with precedence and tests`

## Milestones

### M1: Config System Foundation

**Commits**:
- M1.1: Add CLI argument parsing with clap
- M1.2: Add environment variable support for config options
- M1.3: Implement config precedence (CLI > env > file > defaults)
- M1.4: Add config validation with clear error messages
- M1.5: Create example config and documentation
- M1.6: Add config parsing tests

**Deliverables**:
- `--config` flag for custom config path
- Environment variables: `MOCHI_FONT_SIZE`, `MOCHI_THEME`, etc.
- Clear error messages on invalid config
- `docs/terminal/config.example.toml`
- `docs/terminal/config.md`

### M2: Themes + Light/Dark Mode

**Commits**:
- M2.1: Refactor theme system for runtime switching
- M2.2: Add theme toggle keybinding (Ctrl+Shift+T)
- M2.3: Add Monokai and Gruvbox themes
- M2.4: Add theme documentation
- M2.5: Add theme tests and screenshots

**Deliverables**:
- Built-in themes: mochi-dark, mochi-light, solarized-dark, solarized-light, dracula, nord, monokai, gruvbox
- Runtime theme switching via keybinding
- Theme affects: background, foreground, ANSI 16, selection, cursor
- Screenshots: theme_dark.png, theme_light.png, theme_palette_grid.png

### M3: Font Customization

**Commits**:
- M3.1: Add font family configuration
- M3.2: Add font fallback support
- M3.3: Add cell padding/line height configuration
- M3.4: Ensure PTY resize on font changes
- M3.5: Add font configuration tests
- M3.6: Add font screenshots

**Deliverables**:
- Config options: font_family, font_size, font_weight, fallback_fonts, cell_padding
- Runtime font size change (already exists: Ctrl+=/-)
- Graceful fallback for missing fonts
- Screenshots: font_small.png, font_medium.png, font_large.png

### M4: Keybinding Customization

**Commits**:
- M4.1: Add keybinding configuration structure
- M4.2: Implement keybinding parser
- M4.3: Add default keybindings (copy/paste/find/reload/toggle-theme)
- M4.4: Implement action dispatch system
- M4.5: Add keybinding tests
- M4.6: Document keybindings

**Deliverables**:
- Default keybindings:
  - Copy: Ctrl+Shift+C
  - Paste: Ctrl+Shift+V
  - Find: Ctrl+Shift+F
  - Reload Config: Ctrl+Shift+R
  - Toggle Theme: Ctrl+Shift+T
- User-configurable keybindings in config file
- Keybinding documentation

### M5: UX Polish

**Commits**:
- M5.1: Implement word selection (double-click)
- M5.2: Implement line selection (triple-click)
- M5.3: Add copy-on-select option
- M5.4: Implement search bar UI overlay
- M5.5: Add search highlighting and navigation
- M5.6: Improve hyperlink UX (Ctrl+click to open)
- M5.7: Add selection and search tests
- M5.8: Add UX screenshots

**Deliverables**:
- Double-click: select word
- Triple-click: select line
- Search bar with Ctrl+Shift+F
- Search highlights matches, Enter/Shift+Enter navigates
- Hyperlinks: Ctrl+click to open (never auto-open)
- Screenshots: selection_word.png, search_bar.png, hyperlink_hover.png

### M6: Config Reload + Safety

**Commits**:
- M6.1: Implement config reload keybinding
- M6.2: Add reload error handling (keep old config on failure)
- M6.3: Add OSC 52 clipboard security controls
- M6.4: Add title update throttling
- M6.5: Update security documentation
- M6.6: Add security tests

**Deliverables**:
- Reload config via Ctrl+Shift+R
- Error toast/status on reload failure
- OSC 52: disabled by default, max payload size, user indication
- Title throttling to prevent DoS
- Updated `docs/terminal/security.md`

### M7: Compatibility Testing

**Commits**:
- M7.1: Run vttest and document results
- M7.2: Fix any regressions found
- M7.3: Create compatibility documentation
- M7.4: Final cleanup and polish

**Deliverables**:
- vttest results in `docs/terminal/phase2/compatibility.md`
- All regressions fixed
- All tests passing

## Testing Strategy

### Unit Tests
- Config parsing/validation
- Theme parsing and application
- Keybinding parsing and event mapping
- Selection math (word boundaries, line selection)
- Search indexing

### Golden Tests
- ANSI palette grid rendering
- Theme switch verification
- Font size change layout

### Integration Tests
- PTY spawn and output processing
- Resize propagation on font change

### Manual Tests
- Scripts in `scripts/tests/manual/`
- Screenshots in `docs/terminal/phase2/screenshots/`

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
- [ ] Clipboard escape sequences are guarded with safe defaults, size limits, and user visibility
- [ ] Title update behavior cannot be abused trivially (throttling/limits)
- [ ] docs/terminal/security.md updated

### Compatibility & Regression
- [ ] vttest run notes are documented
- [ ] All automated tests pass
- [ ] No TODOs/placeholders left
- [ ] Documentation updated everywhere it should be

## Timeline

Each milestone should result in multiple small commits that:
1. Compile successfully
2. Pass all tests
3. Include relevant documentation updates

The PR will be created early and updated incrementally.
