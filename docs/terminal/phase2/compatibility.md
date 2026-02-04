# Mochi Terminal Compatibility Report

This document describes the compatibility status of Mochi Terminal with standard terminal emulation tests and common applications.

## Test Suite Results

### Automated Tests

All 150 automated tests pass across all packages:

- **mochi-term**: 30 tests (config, input encoding, terminal state)
- **terminal-core**: 76 tests (cells, colors, cursor, grid, lines, modes, screen, scrollback, selection, snapshots)
- **terminal-parser**: 33 tests (actions, params, parser, UTF-8)
- **terminal-pty**: 11 tests (child process, PTY operations, window size)

### vttest Compatibility

vttest is the standard terminal emulation test suite. Mochi Terminal supports the following vttest categories:

**Supported Features:**
- Cursor movement (CUU, CUD, CUF, CUB, CUP, HVP)
- Cursor save/restore (DECSC, DECRC)
- Erase operations (ED, EL, ECH)
- Insert/delete lines (IL, DL)
- Insert/delete characters (ICH, DCH)
- Scroll regions (DECSTBM)
- Character attributes (SGR) including bold, italic, underline, inverse, colors
- ANSI colors (16 standard colors)
- 256-color palette (indexed colors)
- True color (24-bit RGB)
- Alternate screen buffer (DECSET 1049)
- Origin mode (DECOM)
- Autowrap mode (DECAWM)
- Bracketed paste mode
- Mouse tracking (X10, normal, SGR)
- Focus events
- Window title (OSC 0, 1, 2)
- Hyperlinks (OSC 8)

**Partially Supported:**
- DEC special graphics characters (line drawing)
- Tab stops (HTS, TBC) - basic support

**Not Supported (documented limitations):**
- Soft fonts (DECDLD)
- Sixel graphics
- ReGIS graphics
- Printer passthrough
- VT52 mode
- Double-width/double-height lines (DECDWL, DECDHL)

## Application Compatibility

### Shells
- **bash**: Full support for interactive use, line editing, history
- **zsh**: Full support including prompt themes
- **fish**: Full support

### Editors
- **vim/nvim**: Full support including syntax highlighting, split windows, visual mode
- **nano**: Full support
- **emacs (terminal)**: Basic support

### TUI Applications
- **htop**: Full support for process monitoring
- **less**: Full support for paging
- **man**: Full support for manual pages
- **tmux**: Basic support (some edge cases may have issues)
- **screen**: Basic support

### Development Tools
- **git**: Full support for interactive operations
- **cargo**: Full support including colored output
- **npm**: Full support

## Known Limitations

1. **No sixel/graphics support**: Image display in terminal is not supported
2. **No VT52 mode**: Legacy VT52 compatibility mode is not implemented
3. **Limited TERM support**: Uses xterm-256color, may not have full terminfo entry
4. **Glyph cache growth**: The glyph cache can grow unbounded with many unique characters

## Recommendations

For best compatibility:
- Set `TERM=xterm-256color` (default)
- Use applications that support modern terminal features
- Report any compatibility issues as GitHub issues

## Phase 2 Improvements

Phase 2 added the following compatibility improvements:
- Improved selection handling (word/line selection)
- Search functionality for scrollback
- Better hyperlink support
- Configurable themes with correct ANSI color palettes
- Font customization with proper cell size calculation
- Security hardening for OSC 52 clipboard sequences
