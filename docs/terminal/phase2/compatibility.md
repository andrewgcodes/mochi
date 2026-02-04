# Mochi Terminal Compatibility

This document describes the terminal emulation compatibility of Mochi Terminal.

## Overview

Mochi Terminal aims to be compatible with xterm and VT100/VT220 terminal standards. It implements a subset of escape sequences commonly used by modern applications.

## Supported Features

### Control Sequences (CSI)

Mochi Terminal supports the following CSI sequences:

| Sequence | Description | Status |
|----------|-------------|--------|
| CSI n A | Cursor Up | Supported |
| CSI n B | Cursor Down | Supported |
| CSI n C | Cursor Forward | Supported |
| CSI n D | Cursor Back | Supported |
| CSI n E | Cursor Next Line | Supported |
| CSI n F | Cursor Previous Line | Supported |
| CSI n G | Cursor Horizontal Absolute | Supported |
| CSI n ; m H | Cursor Position | Supported |
| CSI n J | Erase in Display | Supported |
| CSI n K | Erase in Line | Supported |
| CSI n L | Insert Lines | Supported |
| CSI n M | Delete Lines | Supported |
| CSI n P | Delete Characters | Supported |
| CSI n @ | Insert Characters | Supported |
| CSI n S | Scroll Up | Supported |
| CSI n T | Scroll Down | Supported |
| CSI n X | Erase Characters | Supported |
| CSI n m | SGR (Select Graphic Rendition) | Supported |
| CSI n ; m r | Set Scrolling Region | Supported |
| CSI s | Save Cursor Position | Supported |
| CSI u | Restore Cursor Position | Supported |
| CSI ? n h | DEC Private Mode Set | Partial |
| CSI ? n l | DEC Private Mode Reset | Partial |

### SGR (Select Graphic Rendition)

| Code | Description | Status |
|------|-------------|--------|
| 0 | Reset | Supported |
| 1 | Bold | Supported |
| 2 | Faint/Dim | Supported |
| 3 | Italic | Supported |
| 4 | Underline | Supported |
| 5 | Blink | Supported (rendered as bold) |
| 7 | Inverse | Supported |
| 8 | Hidden | Supported |
| 9 | Strikethrough | Supported |
| 22 | Normal intensity | Supported |
| 23 | Not italic | Supported |
| 24 | Not underlined | Supported |
| 25 | Not blinking | Supported |
| 27 | Not inverse | Supported |
| 28 | Not hidden | Supported |
| 29 | Not strikethrough | Supported |
| 30-37 | Foreground color (8 colors) | Supported |
| 38;5;n | Foreground color (256 colors) | Supported |
| 38;2;r;g;b | Foreground color (24-bit) | Supported |
| 39 | Default foreground | Supported |
| 40-47 | Background color (8 colors) | Supported |
| 48;5;n | Background color (256 colors) | Supported |
| 48;2;r;g;b | Background color (24-bit) | Supported |
| 49 | Default background | Supported |
| 90-97 | Bright foreground colors | Supported |
| 100-107 | Bright background colors | Supported |

### DEC Private Modes

| Mode | Description | Status |
|------|-------------|--------|
| 1 | Application Cursor Keys | Supported |
| 7 | Auto-wrap Mode | Supported |
| 12 | Cursor Blink | Supported |
| 25 | Cursor Visible | Supported |
| 47 | Alternate Screen Buffer | Supported |
| 1000 | Mouse Tracking (X10) | Supported |
| 1002 | Mouse Button Event Tracking | Supported |
| 1003 | Mouse Any Event Tracking | Supported |
| 1006 | SGR Mouse Mode | Supported |
| 1049 | Alternate Screen + Save Cursor | Supported |
| 2004 | Bracketed Paste Mode | Supported |

### OSC (Operating System Command)

| Sequence | Description | Status |
|----------|-------------|--------|
| OSC 0 ; title ST | Set Window Title | Supported |
| OSC 1 ; title ST | Set Icon Name | Supported |
| OSC 2 ; title ST | Set Window Title | Supported |
| OSC 8 ; params ; uri ST | Hyperlinks | Supported |
| OSC 52 ; c ; data ST | Clipboard | Not Supported (security) |

### Escape Sequences

| Sequence | Description | Status |
|----------|-------------|--------|
| ESC 7 | Save Cursor (DECSC) | Supported |
| ESC 8 | Restore Cursor (DECRC) | Supported |
| ESC D | Index (IND) | Supported |
| ESC E | Next Line (NEL) | Supported |
| ESC H | Horizontal Tab Set | Supported |
| ESC M | Reverse Index (RI) | Supported |
| ESC c | Full Reset (RIS) | Supported |
| ESC ( 0 | DEC Special Graphics | Supported |
| ESC ( B | ASCII Character Set | Supported |

## Application Compatibility

### Tested Applications

The following applications have been tested with Mochi Terminal:

| Application | Status | Notes |
|-------------|--------|-------|
| bash | Works | Full functionality |
| zsh | Works | Full functionality |
| vim | Works | All modes work correctly |
| nvim | Works | All modes work correctly |
| htop | Works | Colors and layout correct |
| less | Works | Scrolling and search work |
| tmux | Partial | Basic functionality works |
| man | Works | Formatting correct |
| git | Works | Colors and paging work |
| ls --color | Works | Colors display correctly |

### Known Issues

1. **tmux**: Some advanced features may not work correctly due to incomplete terminfo support.

2. **Complex Unicode**: Some complex Unicode sequences (combining characters, emoji with modifiers) may not render correctly.

3. **Sixel Graphics**: Not supported.

4. **ReGIS Graphics**: Not supported.

## vttest Results

vttest is a standard terminal emulator test suite. The following tests have been run:

### Test Status

| Test Category | Status | Notes |
|---------------|--------|-------|
| Cursor Movement | Pass | All basic cursor movements work |
| Screen Features | Pass | Scrolling, clearing work |
| Character Sets | Partial | DEC Special Graphics supported |
| Double-Size Characters | Not Supported | |
| Keyboard | Pass | Standard keys work |
| Mouse | Pass | X10, SGR modes work |
| Color | Pass | 256 and 24-bit colors work |

### Not Yet Tested

- Full vttest suite (requires manual testing)
- esctest2 suite

## TERM Environment Variable

Mochi Terminal sets `TERM=xterm-256color` by default. This provides good compatibility with most applications.

For applications that require specific terminfo entries, you may need to:

1. Use a different TERM value
2. Install custom terminfo entries

## Recommendations

For best compatibility:

1. Use `TERM=xterm-256color` (default)
2. Enable bracketed paste mode in your shell
3. Use UTF-8 encoding
4. Ensure your shell supports 24-bit color if using true color themes

## Future Improvements

The following features are planned for future releases:

1. Sixel graphics support
2. Custom terminfo entry
3. More complete DEC private mode support
4. Improved Unicode handling (grapheme clusters)

## References

- [XTerm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)
- [ECMA-48](https://www.ecma-international.org/publications-and-standards/standards/ecma-48/)
- [VT100 User Guide](https://vt100.net/docs/vt100-ug/)
