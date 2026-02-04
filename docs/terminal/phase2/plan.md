# Mochi Terminal Phase 2 Plan: Modern Config + Theming + UX

## Scope

Transform the Mochi terminal from "basic but working" to "modern and customizable" by implementing:
- Robust configuration system with CLI overrides
- Theme engine with light/dark mode and custom themes
- Font customization with runtime reload
- Keybinding customization
- UX polish: improved selection, scrollback search, hyperlink UX
- Security hardening for dangerous escape sequences

## Branch

`phase2-modern-config-theming`

## Commit Message Format

`phase2(Mx.y): short description`

Example: `phase2(M1.1): add CLI argument parsing with clap`

## Milestones

### M1: Config System Foundation

**M1.1** Add CLI argument parsing
- Add `clap` dependency for argument parsing
- Support `--config <path>` to override config location
- Support `--version` and `--help`

**M1.2** Implement config precedence
- CLI flags > environment variables > config file > built-in defaults
- Document precedence in config.md

**M1.3** Add config validation
- Validate all config values on load
- Return clear error messages for invalid config
- Never silently ignore errors

**M1.4** Create example config and documentation
- docs/terminal/config.example.toml
- docs/terminal/config.md with schema

**M1.5** Add config tests
- Parse valid config variants
- Reject invalid config with useful errors
- Precedence tests

### M2: Themes + Light/Dark Mode

**M2.1** Refactor theme system
- Ensure mochi-dark and mochi-light are complete
- Add 2 additional curated themes (Solarized, Dracula already exist, add Nord, Gruvbox)
- Document theme format

**M2.2** Implement runtime theme switching
- Add keybinding for theme toggle (Ctrl+Shift+T)
- Update renderer colors without restart

**M2.3** Support custom theme files
- Load theme from file path
- Validate theme format

**M2.4** Add theme tests and screenshots
- Unit tests for theme parsing
- Golden tests for ANSI palette
- Screenshots: theme_dark.png, theme_light.png, theme_palette_grid.png

### M3: Font Customization + Layout

**M3.1** Implement font discovery
- Use fontconfig or system fonts
- Support font family string in config
- Fallback to bundled font if not found

**M3.2** Add font configuration options
- font_family, font_size, font_weight
- fallback_fonts list
- cell_padding_x, cell_padding_y, line_height

**M3.3** Implement runtime font reload
- Recalculate cell size on font change
- Resize PTY with new dimensions
- Clear glyph cache

**M3.4** Add font tests and screenshots
- Config parsing tests
- Pixel size to rows/cols mapping tests
- Screenshots: font_small.png, font_medium.png, font_large.png

### M4: Keybinding Customization

**M4.1** Define keybinding config format
- Support modifiers: Ctrl, Alt, Shift, Super
- Support key names and characters

**M4.2** Implement default keybindings
- Copy: Ctrl+Shift+C
- Paste: Ctrl+Shift+V
- Find: Ctrl+Shift+F
- Reload config: Ctrl+Shift+R
- Toggle theme: Ctrl+Shift+T

**M4.3** Add keybinding config parsing
- Parse keybindings from config file
- Validate keybinding format

**M4.4** Implement action dispatch
- Map key events to actions
- Execute actions (copy, paste, find, reload, toggle_theme)

**M4.5** Add keybinding tests
- Parse keybinding config
- Key event to action mapping tests

### M5: UX Polish

**M5.1** Improve selection
- Double-click selects word
- Triple-click selects line
- Document wrapped line behavior

**M5.2** Implement scrollback search UI
- Find bar overlay
- Highlight matches
- Enter/Shift+Enter navigate next/prev
- Esc closes find bar

**M5.3** Improve hyperlink UX
- Render OSC 8 hyperlinks as underlined
- Ctrl+click to open (never auto-open)
- Optional URL detection in text

**M5.4** Add UX tests and screenshots
- Selection range logic tests
- Word boundary tests
- Search indexing tests
- Screenshots: selection_word.png, search_bar.png, hyperlink_hover.png

### M6: Config Reload + Safety

**M6.1** Implement config reload keybinding
- Ctrl+Shift+R reloads config
- Show success/error feedback

**M6.2** Handle reload failures gracefully
- Keep previous config on failure
- Show error message to user
- Never crash

**M6.3** Security hardening
- OSC 52 clipboard disabled by default
- Max payload size for clipboard
- User-visible indication when clipboard modified
- Title update throttling

**M6.4** Update security documentation
- docs/terminal/security.md

**M6.5** Add reload and security tests
- Reload success updates settings
- Reload failure keeps old settings
- Clipboard payload size limit tests

### M7: Compatibility Testing

**M7.1** Run vttest
- Document results
- Fix any regressions

**M7.2** Document compatibility
- docs/terminal/phase2/compatibility.md
- Known limitations

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

This plan will be updated as implementation progresses and constraints are discovered.
