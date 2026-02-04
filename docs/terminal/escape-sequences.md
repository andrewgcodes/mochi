# Escape Sequence Coverage

This document lists the escape sequences supported by the Mochi terminal emulator.

## C0 Control Characters

| Code | Name | Implemented | Notes |
|------|------|-------------|-------|
| 0x07 | BEL | Yes | Bell/alert |
| 0x08 | BS | Yes | Backspace |
| 0x09 | HT | Yes | Horizontal tab |
| 0x0A | LF | Yes | Line feed |
| 0x0B | VT | Yes | Vertical tab (treated as LF) |
| 0x0C | FF | Yes | Form feed (treated as LF) |
| 0x0D | CR | Yes | Carriage return |
| 0x1B | ESC | Yes | Escape |

## ESC Sequences

| Sequence | Name | Implemented | Notes |
|----------|------|-------------|-------|
| ESC 7 | DECSC | Yes | Save cursor |
| ESC 8 | DECRC | Yes | Restore cursor |
| ESC D | IND | Yes | Index (move down, scroll if needed) |
| ESC E | NEL | Yes | Next line |
| ESC H | HTS | Yes | Horizontal tab set |
| ESC M | RI | Yes | Reverse index |
| ESC c | RIS | Yes | Full reset |
| ESC ( B | G0 ASCII | Yes | Designate G0 charset |
| ESC ( 0 | G0 Special | Yes | DEC Special Graphics |

## CSI Sequences

### Cursor Movement

| Sequence | Name | Implemented | Notes |
|----------|------|-------------|-------|
| CSI n A | CUU | Yes | Cursor up |
| CSI n B | CUD | Yes | Cursor down |
| CSI n C | CUF | Yes | Cursor forward |
| CSI n D | CUB | Yes | Cursor back |
| CSI n E | CNL | Yes | Cursor next line |
| CSI n F | CPL | Yes | Cursor previous line |
| CSI n G | CHA | Yes | Cursor horizontal absolute |
| CSI n ; m H | CUP | Yes | Cursor position |
| CSI n d | VPA | Yes | Vertical position absolute |
| CSI n ; m f | HVP | Yes | Horizontal and vertical position |
| CSI s | SCP | Yes | Save cursor position |
| CSI u | RCP | Yes | Restore cursor position |

### Erase Functions

| Sequence | Name | Implemented | Notes |
|----------|------|-------------|-------|
| CSI n J | ED | Yes | Erase in display (0=below, 1=above, 2=all, 3=scrollback) |
| CSI n K | EL | Yes | Erase in line (0=right, 1=left, 2=all) |
| CSI n X | ECH | Yes | Erase characters |

### Insert/Delete

| Sequence | Name | Implemented | Notes |
|----------|------|-------------|-------|
| CSI n @ | ICH | Yes | Insert characters |
| CSI n P | DCH | Yes | Delete characters |
| CSI n L | IL | Yes | Insert lines |
| CSI n M | DL | Yes | Delete lines |

### Scroll Region

| Sequence | Name | Implemented | Notes |
|----------|------|-------------|-------|
| CSI t ; b r | DECSTBM | Yes | Set scroll region |
| CSI n S | SU | Yes | Scroll up |
| CSI n T | SD | Yes | Scroll down |

### Tab Control

| Sequence | Name | Implemented | Notes |
|----------|------|-------------|-------|
| CSI n g | TBC | Yes | Tab clear (0=current, 3=all) |

### SGR (Select Graphic Rendition)

| Code | Effect | Implemented | Notes |
|------|--------|-------------|-------|
| 0 | Reset | Yes | |
| 1 | Bold | Yes | |
| 2 | Faint | Yes | |
| 3 | Italic | Yes | |
| 4 | Underline | Yes | |
| 5 | Blink | Yes | Rendered as bold |
| 7 | Inverse | Yes | |
| 8 | Hidden | Yes | |
| 9 | Strikethrough | Yes | |
| 22 | Normal intensity | Yes | |
| 23 | Not italic | Yes | |
| 24 | Not underlined | Yes | |
| 25 | Not blinking | Yes | |
| 27 | Not inverse | Yes | |
| 28 | Not hidden | Yes | |
| 29 | Not strikethrough | Yes | |
| 30-37 | Foreground color | Yes | Standard colors |
| 38;5;n | Foreground 256 | Yes | |
| 38;2;r;g;b | Foreground RGB | Yes | |
| 39 | Default foreground | Yes | |
| 40-47 | Background color | Yes | Standard colors |
| 48;5;n | Background 256 | Yes | |
| 48;2;r;g;b | Background RGB | Yes | |
| 49 | Default background | Yes | |
| 90-97 | Bright foreground | Yes | |
| 100-107 | Bright background | Yes | |

### DEC Private Modes

| Sequence | Mode | Implemented | Notes |
|----------|------|-------------|-------|
| CSI ? 1 h/l | DECCKM | Yes | Application cursor keys |
| CSI ? 6 h/l | DECOM | Yes | Origin mode |
| CSI ? 7 h/l | DECAWM | Yes | Auto-wrap mode |
| CSI ? 12 h/l | | Yes | Cursor blink |
| CSI ? 25 h/l | DECTCEM | Yes | Cursor visible |
| CSI ? 47 h/l | | Yes | Alternate screen (legacy) |
| CSI ? 1000 h/l | | Yes | Mouse X10 mode |
| CSI ? 1002 h/l | | Yes | Mouse button tracking |
| CSI ? 1003 h/l | | Yes | Mouse any-event tracking |
| CSI ? 1004 h/l | | Yes | Focus events |
| CSI ? 1006 h/l | | Yes | SGR mouse mode |
| CSI ? 1049 h/l | | Yes | Alternate screen with save/restore |
| CSI ? 2004 h/l | | Yes | Bracketed paste mode |

### Cursor Style

| Sequence | Style | Implemented | Notes |
|----------|-------|-------------|-------|
| CSI 0 SP q | Default | Yes | |
| CSI 1 SP q | Blinking block | Yes | |
| CSI 2 SP q | Steady block | Yes | |
| CSI 3 SP q | Blinking underline | Yes | |
| CSI 4 SP q | Steady underline | Yes | |
| CSI 5 SP q | Blinking bar | Yes | |
| CSI 6 SP q | Steady bar | Yes | |

## OSC Sequences

| Sequence | Function | Implemented | Notes |
|----------|----------|-------------|-------|
| OSC 0 ; text ST | Set icon name and title | Yes | |
| OSC 2 ; text ST | Set title | Yes | |
| OSC 8 ; params ; uri ST | Hyperlink | Yes | |
| OSC 52 ; c ; data ST | Clipboard | Yes | Security controls required |

## Not Yet Implemented

The following features are not yet implemented:

- DCS sequences (device control strings)
- Sixel graphics
- ReGIS graphics
- Soft fonts
- Some less common DEC private modes

## References

- [XTerm Control Sequences](https://www.x.org/docs/xterm/ctlseqs.pdf)
- [ECMA-48](https://ecma-international.org/wp-content/uploads/ECMA-48_5th_edition_june_1991.pdf)
- [VT220 Programmer Reference](https://vt100.net/dec/ek-vt220-rm-001.pdf)
