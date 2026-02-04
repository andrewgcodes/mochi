# Mochi Terminal Compatibility Notes

This document describes the compatibility testing results and known limitations of the Mochi terminal emulator.

## vttest Results

vttest (VT100 test program) version 2.7 was used to verify terminal compatibility.

### Test Summary

The terminal successfully displays the vttest menu and handles basic VT100 escape sequences. The following features have been verified:

**Working Features:**
- Basic text display and cursor positioning
- ANSI color codes (16 colors + 256 color mode + true color)
- Cursor movement sequences (CUU, CUD, CUF, CUB, CUP)
- Screen clearing (ED, EL)
- Line insertion/deletion (IL, DL)
- Character insertion/deletion (ICH, DCH)
- Scroll regions (DECSTBM)
- Alternate screen buffer (DECSET 1049)
- Cursor visibility (DECTCEM)
- Auto-wrap mode (DECAWM)
- Origin mode (DECOM)
- Bracketed paste mode (DECSET 2004)
- Mouse tracking modes (1000, 1002, 1003, 1006)
- SGR attributes (bold, italic, underline, inverse, etc.)
- Window title setting (OSC 0/2)
- Hyperlinks (OSC 8)

**Known Limitations:**
- Double-width/double-height characters (DECDWL/DECDHL) are not fully supported
- Some VT52 mode features may not work correctly
- Device status reports (DSR) responses are not implemented
- Some DCS sequences are ignored

### Screenshots

- `vttest_menu.png` - vttest main menu displaying correctly

## Terminal Emulation

### Supported Standards

Mochi Terminal aims to be compatible with:
- VT100/VT102 basic sequences
- VT220 extended sequences (partial)
- xterm extensions (partial)
- ECMA-48 control functions

### Escape Sequence Support

#### C0 Control Characters
- BEL (0x07) - Bell/alert
- BS (0x08) - Backspace
- HT (0x09) - Horizontal tab
- LF (0x0A) - Line feed
- VT (0x0B) - Vertical tab (treated as LF)
- FF (0x0C) - Form feed (treated as LF)
- CR (0x0D) - Carriage return
- SO (0x0E) - Shift out (G1 character set)
- SI (0x0F) - Shift in (G0 character set)

#### ESC Sequences
- ESC 7 - Save cursor (DECSC)
- ESC 8 - Restore cursor (DECRC)
- ESC D - Index (IND)
- ESC M - Reverse index (RI)
- ESC E - Next line (NEL)
- ESC H - Horizontal tab set (HTS)
- ESC c - Full reset (RIS)
- ESC = - Application keypad mode
- ESC > - Normal keypad mode
- ESC ( C - Designate G0 character set
- ESC ) C - Designate G1 character set
- ESC # 8 - DEC alignment test

#### CSI Sequences
- CSI n @ - Insert characters (ICH)
- CSI n A - Cursor up (CUU)
- CSI n B - Cursor down (CUD)
- CSI n C - Cursor forward (CUF)
- CSI n D - Cursor back (CUB)
- CSI n E - Cursor next line (CNL)
- CSI n F - Cursor previous line (CPL)
- CSI n G - Cursor horizontal absolute (CHA)
- CSI n ; m H - Cursor position (CUP)
- CSI n J - Erase in display (ED)
- CSI n K - Erase in line (EL)
- CSI n L - Insert lines (IL)
- CSI n M - Delete lines (DL)
- CSI n P - Delete characters (DCH)
- CSI n S - Scroll up (SU)
- CSI n T - Scroll down (SD)
- CSI n X - Erase characters (ECH)
- CSI n d - Vertical position absolute (VPA)
- CSI n g - Tab clear (TBC)
- CSI n h - Set mode (SM)
- CSI n l - Reset mode (RM)
- CSI n m - Select graphic rendition (SGR)
- CSI n ; m r - Set scroll region (DECSTBM)
- CSI s - Save cursor (ANSI.SYS)
- CSI u - Restore cursor (ANSI.SYS)
- CSI ? n h - DEC private mode set (DECSET)
- CSI ? n l - DEC private mode reset (DECRST)
- CSI n SP q - Set cursor style (DECSCUSR)

#### DEC Private Modes
- 1 - Cursor keys mode (DECCKM)
- 6 - Origin mode (DECOM)
- 7 - Auto-wrap mode (DECAWM)
- 25 - Cursor visibility (DECTCEM)
- 47 - Alternate screen buffer
- 1000 - VT200 mouse tracking
- 1002 - Button event mouse tracking
- 1003 - Any event mouse tracking
- 1004 - Focus events
- 1006 - SGR mouse mode
- 1047 - Alternate screen buffer (with clear)
- 1048 - Save/restore cursor
- 1049 - Alternate screen buffer (combined)
- 2004 - Bracketed paste mode

#### OSC Sequences
- OSC 0 - Set icon name and window title
- OSC 1 - Set icon name
- OSC 2 - Set window title
- OSC 4 - Set/query color palette
- OSC 7 - Set current directory
- OSC 8 - Hyperlinks
- OSC 10 - Set foreground color
- OSC 11 - Set background color
- OSC 12 - Set cursor color
- OSC 52 - Clipboard (disabled by default for security)
- OSC 104 - Reset color
- OSC 110 - Reset foreground color
- OSC 111 - Reset background color
- OSC 112 - Reset cursor color

### SGR (Select Graphic Rendition) Support

- 0 - Reset all attributes
- 1 - Bold
- 2 - Faint/dim
- 3 - Italic
- 4 - Underline
- 5 - Blink (slow)
- 7 - Inverse/reverse
- 8 - Hidden/invisible
- 9 - Strikethrough
- 21-29 - Reset individual attributes
- 30-37 - Set foreground color (8 colors)
- 38;5;n - Set foreground color (256 colors)
- 38;2;r;g;b - Set foreground color (true color)
- 39 - Default foreground color
- 40-47 - Set background color (8 colors)
- 48;5;n - Set background color (256 colors)
- 48;2;r;g;b - Set background color (true color)
- 49 - Default background color
- 90-97 - Set bright foreground color
- 100-107 - Set bright background color

## Application Compatibility

### Tested Applications

The following applications have been tested and work correctly:

- **bash/zsh** - Interactive shell with prompt, history, tab completion
- **vim/nvim** - Text editor with syntax highlighting, multiple buffers
- **htop** - System monitor with colors and interactive UI
- **less** - Pager with search and navigation
- **tmux** - Terminal multiplexer (basic functionality)
- **git** - Version control with colored output
- **man** - Manual pages with formatting

### Known Issues

1. **tmux**: Some advanced features may not work correctly due to missing terminal capabilities
2. **Screen**: Similar limitations as tmux
3. **Complex Unicode**: Some complex Unicode sequences (combining characters, emoji ZWJ sequences) may not render correctly

## TERM Environment Variable

Mochi Terminal sets `TERM=xterm-256color` by default, which provides good compatibility with most applications. If you experience issues, you can try:

- `TERM=xterm` for basic compatibility
- `TERM=vt100` for minimal compatibility

## Reporting Compatibility Issues

If you encounter compatibility issues with specific applications or escape sequences, please report them with:

1. The application name and version
2. The specific escape sequence or feature that doesn't work
3. Expected behavior vs actual behavior
4. Steps to reproduce
