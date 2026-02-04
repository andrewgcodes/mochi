# Mochi Terminal Phase 2 - Implementation Plan

## Scope

Transform the Mochi terminal emulator from "basic but working" to "modern and customizable" by implementing:

1. Robust configuration system with XDG support and CLI overrides
2. Theme engine with light/dark mode and runtime switching
3. Font customization with family, size, fallback, and runtime reload
4. Keybinding customization for common actions
5. UX polish: improved selection, scrollback search, hyperlink UX
6. Security hardening for escape sequences
7. Comprehensive test coverage with no regressions

## Branch and Commit Strategy

- **Branch name:** `phase2-modern-config-theming`
- **Commit format:** `phase2(Mx.y): short description`
- **Each commit must:** compile, pass tests, update relevant docs

## Milestones

### M1: Config System Foundation

**Goal:** Implement a robust configuration system with clear precedence rules.

**Deliverables:**
- CLI argument parsing with `--config` flag
- Environment variable support for key settings
- Config precedence: CLI > env > file > defaults
- Clear error messages for invalid config
- Example config file and documentation

**Commits:**
- `phase2(M1.1): add clap dependency and CLI argument parsing`
- `phase2(M1.2): implement environment variable config support`
- `phase2(M1.3): implement config precedence and validation`
- `phase2(M1.4): add example config and documentation`

**Tests:**
- Parse valid config (multiple variants)
- Reject invalid config with useful error
- Precedence tests (CLI overrides config, etc.)
- Snapshot tests: config -> computed effective settings

**Files to modify:**
- `mochi-term/Cargo.toml` - add clap
- `mochi-term/src/main.rs` - CLI parsing
- `mochi-term/src/config.rs` - env vars, validation, precedence
- `docs/terminal/config.md` - schema documentation
- `docs/terminal/config.example.toml` - example config

### M2: Themes + Light/Dark Mode

**Goal:** Implement a theme engine with built-in themes and runtime switching.

**Deliverables:**
- Built-in themes: mochi-dark, mochi-light (already exist as Dark, Light)
- 2 additional curated themes (already have Solarized, Dracula, Nord)
- Runtime theme switching via keybinding
- Theme file loading from external paths
- Theme documentation

**Commits:**
- `phase2(M2.1): add runtime theme switching keybinding (Ctrl+Shift+T)`
- `phase2(M2.2): add theme file loading from custom paths`
- `phase2(M2.3): add theme documentation and screenshots`

**Tests:**
- Unit tests: theme parsing, applying to renderer state
- Golden tests: ANSI color mapping verification
- Manual screenshots: theme_dark.png, theme_light.png, theme_palette_grid.png

**Files to modify:**
- `mochi-term/src/app.rs` - theme toggle keybinding
- `mochi-term/src/config.rs` - theme file loading
- `docs/terminal/themes.md` - theme documentation

### M3: Font Customization + Layout

**Goal:** Allow users to customize fonts with runtime reload support.

**Deliverables:**
- Configurable font family (with system font discovery)
- Font size configuration
- Font fallback list
- Cell padding/line height configuration
- Runtime font reload without restart

**Commits:**
- `phase2(M3.1): implement system font discovery and loading`
- `phase2(M3.2): add font fallback list support`
- `phase2(M3.3): add cell padding and line height configuration`
- `phase2(M3.4): implement runtime font reload with PTY resize`

**Tests:**
- Config parsing tests for font settings
- Unit test for pixel size -> rows/cols mapping
- Integration test: font size change -> PTY resize
- Manual screenshots: font_small.png, font_medium.png, font_large.png

**Files to modify:**
- `mochi-term/src/renderer.rs` - font loading, fallback, cell size
- `mochi-term/src/app.rs` - font reload, PTY resize
- `mochi-term/src/config.rs` - font configuration options

### M4: Keybinding Customization

**Goal:** Allow users to customize keybindings for common actions.

**Deliverables:**
- Keybinding configuration in config file
- Default keybindings:
  - Copy: Ctrl+Shift+C
  - Paste: Ctrl+Shift+V
  - Find: Ctrl+Shift+F
  - Reload config: Ctrl+Shift+R
  - Toggle theme: Ctrl+Shift+T
- Keybinding documentation

**Commits:**
- `phase2(M4.1): add keybinding configuration types`
- `phase2(M4.2): implement keybinding parser and event mapping`
- `phase2(M4.3): add default keybindings for all actions`
- `phase2(M4.4): add keybinding documentation`

**Tests:**
- Parse keybinding config
- Unit tests: key event -> action mapping
- Manual test checklist for all default shortcuts

**Files to modify:**
- `mochi-term/src/config.rs` - keybinding configuration
- `mochi-term/src/app.rs` - keybinding handling
- New: `mochi-term/src/keybindings.rs` - keybinding system
- `docs/terminal/keybindings.md` - documentation

### M5: UX Polish

**Goal:** Improve selection, add scrollback search, and enhance hyperlink UX.

**Deliverables:**
- Word selection (double-click)
- Line selection (triple-click)
- Scrollback search UI (find bar overlay)
- Hyperlink hover indication
- Ctrl+click to open hyperlinks

**Commits:**
- `phase2(M5.1): implement word selection on double-click`
- `phase2(M5.2): implement line selection on triple-click`
- `phase2(M5.3): add scrollback search UI with find bar`
- `phase2(M5.4): implement search highlighting and navigation`
- `phase2(M5.5): improve hyperlink UX with hover and ctrl+click`

**Tests:**
- Unit tests: selection range logic, word boundary detection
- Unit tests: search indexing and navigation
- Manual screenshots: selection_word.png, search_bar.png, hyperlink_hover.png

**Files to modify:**
- `mochi-term/src/app.rs` - click handling, search UI
- `terminal-core/src/selection.rs` - word/line selection logic
- `mochi-term/src/renderer.rs` - search highlight rendering
- New: `mochi-term/src/search.rs` - search functionality

### M6: Config Reload + Security

**Goal:** Enable runtime config reload and harden security for escape sequences.

**Deliverables:**
- Config reload via keybinding (Ctrl+Shift+R)
- Graceful error handling for reload failures
- OSC 52 clipboard security (disabled by default, size limits)
- Title update throttling
- Security documentation

**Commits:**
- `phase2(M6.1): implement config reload via keybinding`
- `phase2(M6.2): add graceful error handling for reload failures`
- `phase2(M6.3): add title update throttling`
- `phase2(M6.4): update security documentation`

**Tests:**
- Unit test: reload success updates effective settings
- Unit test: reload failure keeps old settings
- Unit test: clipboard OSC payload size limit
- Unit test: title update throttling

**Files to modify:**
- `mochi-term/src/app.rs` - reload handling
- `mochi-term/src/config.rs` - reload logic
- `mochi-term/src/terminal.rs` - title throttling
- `docs/terminal/security.md` - security documentation

### M7: No Regressions

**Goal:** Ensure all changes maintain compatibility and pass tests.

**Deliverables:**
- vttest run notes documented
- All automated tests pass
- No TODOs or placeholders
- Documentation complete

**Commits:**
- `phase2(M7.1): run vttest and document results`
- `phase2(M7.2): fix any regressions found`
- `phase2(M7.3): final documentation review`

**Tests:**
- Run full test suite
- Run vttest manually
- Visual inspection of all features

**Files to modify:**
- `docs/terminal/phase2/compatibility.md` - vttest results

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

## Timeline Estimate

Based on complexity and dependencies:

1. M1 (Config): ~2-3 hours
2. M2 (Themes): ~1-2 hours
3. M3 (Fonts): ~2-3 hours
4. M4 (Keybindings): ~2-3 hours
5. M5 (UX Polish): ~3-4 hours
6. M6 (Security): ~1-2 hours
7. M7 (Regressions): ~1-2 hours

Total: ~12-19 hours of implementation work

## Dependencies

New crates to add:
- `clap` - CLI argument parsing (for M1)
- `fontconfig` or similar - System font discovery (for M3, optional)

## Risk Mitigation

1. **Font loading failures:** Implement robust fallback to bundled font
2. **Config parsing errors:** Provide clear error messages, never crash
3. **Theme application bugs:** Test all color contexts (fg, bg, cursor, selection, ANSI)
4. **Keybinding conflicts:** Document reserved combinations, allow user override
5. **Search performance:** Limit search scope, use efficient algorithms
6. **Security vulnerabilities:** Default to safe settings, require explicit opt-in for risky features
