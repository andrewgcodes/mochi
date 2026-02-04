# Phase 2 Compatibility Notes

This document describes the compatibility status of Mochi Terminal after Phase 2 implementation.

## Test Results Summary

All automated tests pass (166 tests total):
- terminal-core: 76 tests
- terminal-parser: 33 tests
- terminal-pty: 11 tests
- mochi-term: 46 tests

## Supported Features

### VT/xterm Escape Sequences

Mochi Terminal supports a comprehensive set of VT/xterm escape sequences:

**Cursor Control:**
- CUU (Cursor Up), CUD (Cursor Down), CUF (Cursor Forward), CUB (Cursor Back)
- CUP (Cursor Position), HVP (Horizontal and Vertical Position)
- CNL (Cursor Next Line), CPL (Cursor Previous Line)
- CHA (Cursor Horizontal Absolute), VPA (Vertical Position Absolute)
- Save/Restore Cursor Position (DECSC/DECRC)

**Erase Functions:**
- ED (Erase in Display) - modes 0, 1, 2, 3
- EL (Erase in Line) - modes 0, 1, 2
- ECH (Erase Character)

**Line Operations:**
- IL (Insert Lines), DL (Delete Lines)
- ICH (Insert Characters), DCH (Delete Characters)

**Scrolling:**
- SU (Scroll Up), SD (Scroll Down)
- DECSTBM (Set Top and Bottom Margins)
- Scrollback buffer with configurable size (default 10,000 lines)

**Character Attributes (SGR):**
- Bold, Dim, Italic, Underline, Blink, Inverse, Hidden, Strikethrough
- 8 standard colors (foreground and background)
- 8 bright colors
- 256-color palette
- 24-bit true color (RGB)
- Default color reset

**Screen Modes:**
- Alternate screen buffer (DECSET 1049)
- Origin mode (DECOM)
- Auto-wrap mode (DECAWM)
- Cursor visibility (DECTCEM)
- Application cursor keys (DECCKM)
- Bracketed paste mode

**Mouse Support:**
- X10 mouse tracking
- Normal tracking mode
- Button event tracking
- Any event tracking
- SGR extended mouse reporting

**OSC Sequences:**
- OSC 0/1/2: Window title
- OSC 8: Hyperlinks
- OSC 52: Clipboard (disabled by default for security)

### Phase 2 Features

**Configuration System:**
- XDG-compliant config file location (~/.config/mochi/config.toml)
- CLI argument overrides
- Environment variable support (MOCHI_* prefix)
- Clear precedence: CLI > env > config file > defaults
- Validation with helpful error messages

**Themes:**
- 6 built-in themes: dark, light, solarized-dark, solarized-light, dracula, nord
- Runtime theme switching (Ctrl+Shift+T)
- Custom theme support via config file
- Full ANSI 16-color palette customization
- Search match highlight colors

**Font Customization:**
- Configurable font family and size
- Line height multiplier (1.0 - 3.0)
- Runtime font size adjustment (Ctrl+Plus/Minus)
- Graceful fallback for missing fonts

**Keybindings:**
- Copy: Ctrl+Shift+C
- Paste: Ctrl+Shift+V
- Search: Ctrl+Shift+F
- Reload Config: Ctrl+Shift+R
- Toggle Theme: Ctrl+Shift+T
- Font zoom: Ctrl+Plus/Minus/0

**Selection:**
- Click and drag selection
- Double-click word selection
- Triple-click line selection
- Copy to clipboard

**Search:**
- Search bar overlay (Ctrl+Shift+F)
- Match highlighting with distinct colors
- Navigate matches with Enter/Shift+Enter
- Close with Escape

**Security:**
- OSC 52 clipboard disabled by default
- Configurable payload size limits
- Title update throttling (100ms minimum interval)

## Known Limitations

### Not Yet Implemented

1. **Ligatures**: Font ligatures are not currently supported
2. **URL Detection**: Automatic URL detection in plain text (OSC 8 hyperlinks work)
3. **File Watcher**: Automatic config reload on file change (manual reload via Ctrl+Shift+R works)
4. **Custom Keybindings**: Keybindings are currently fixed (customization planned)

### Compatibility Notes

1. **Font Rendering**: Uses CPU-based rendering via fontdue. Some complex scripts may not render perfectly.

2. **Wide Characters**: CJK and other wide characters are supported but emoji rendering may vary.

3. **Terminal Size**: Minimum terminal size is enforced to prevent rendering issues.

4. **Mouse Tracking**: When mouse tracking is enabled by applications, selection is disabled.

## Application Compatibility

Tested and working with:
- bash, zsh (interactive shells)
- vim, nvim (text editors)
- htop, top (system monitors)
- less, more (pagers)
- tmux (terminal multiplexer - basic functionality)
- git (version control)
- cargo, npm, python (development tools)

## Regression Testing

All existing tests continue to pass after Phase 2 changes. The test suite covers:
- Escape sequence parsing
- Screen model operations
- Cursor movement and positioning
- Character attributes
- Scrolling and scroll regions
- Alternate screen buffer
- Selection logic
- Configuration parsing and validation
- Theme application
- Input encoding

## Future Improvements

Potential areas for future enhancement:
1. GPU-accelerated rendering
2. Font ligature support
3. Automatic URL detection
4. File watcher for config changes
5. Custom keybinding configuration
6. More terminal emulation tests (vttest, esctest)
