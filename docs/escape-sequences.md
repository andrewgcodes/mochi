# Escape Sequence Coverage

This document tracks the escape sequences supported by Mochi Terminal.

## Legend

| Status | Meaning |
|--------|---------|
| Yes | Fully implemented and tested |
| Partial | Implemented but may have edge cases |
| No | Not implemented |
| N/A | Not applicable or intentionally unsupported |

## C0 Control Characters (0x00-0x1F)

| Code | Name | Implemented | Tests | Notes |
|------|------|-------------|-------|-------|
| 0x00 | NUL | Yes | Yes | Ignored |
| 0x07 | BEL | Yes | Yes | Bell (visual/audio handled by frontend) |
| 0x08 | BS | Yes | Yes | Backspace - move cursor left |
| 0x09 | HT | Yes | Yes | Horizontal tab |
| 0x0A | LF | Yes | Yes | Line feed |
| 0x0B | VT | Yes | Yes | Vertical tab (treated as LF) |
| 0x0C | FF | Yes | Yes | Form feed (treated as LF) |
| 0x0D | CR | Yes | Yes | Carriage return |
| 0x0E | SO | Partial | No | Shift Out (charset switching stub) |
| 0x0F | SI | Partial | No | Shift In (charset switching stub) |
| 0x18 | CAN | Yes | Yes | Cancel escape sequence |
| 0x1A | SUB | Yes | Yes | Substitute (cancel + replacement char) |
| 0x1B | ESC | Yes | Yes | Escape - start escape sequence |

## ESC Sequences (Non-CSI)

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| ESC 7 | DECSC | Yes | Yes | Save cursor position and attributes |
| ESC 8 | DECRC | Yes | Yes | Restore cursor position and attributes |
| ESC D | IND | Yes | Yes | Index - move down, scroll if at bottom |
| ESC M | RI | Yes | Yes | Reverse Index - move up, scroll if at top |
| ESC E | NEL | Yes | Yes | Next Line - CR + LF |
| ESC H | HTS | Yes | Yes | Horizontal Tab Set |
| ESC c | RIS | Yes | Yes | Full Reset |
| ESC = | DECKPAM | Partial | No | Application Keypad Mode |
| ESC > | DECKPNM | Partial | No | Normal Keypad Mode |
| ESC ( B | G0 ASCII | Partial | No | Select ASCII charset for G0 |
| ESC ( 0 | G0 DEC | No | No | Select DEC Special Graphics for G0 |
| ESC ) B | G1 ASCII | Partial | No | Select ASCII charset for G1 |
| ESC ) 0 | G1 DEC | No | No | Select DEC Special Graphics for G1 |
| ESC # 8 | DECALN | Yes | No | Screen Alignment Test (fill with 'E') |
| ESC N | SS2 | No | No | Single Shift 2 |
| ESC O | SS3 | No | No | Single Shift 3 |

## CSI Sequences

### Cursor Movement

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| CSI n A | CUU | Yes | Yes | Cursor Up |
| CSI n B | CUD | Yes | Yes | Cursor Down |
| CSI n C | CUF | Yes | Yes | Cursor Forward (Right) |
| CSI n D | CUB | Yes | Yes | Cursor Backward (Left) |
| CSI n E | CNL | Yes | Yes | Cursor Next Line |
| CSI n F | CPL | Yes | Yes | Cursor Previous Line |
| CSI n G | CHA | Yes | Yes | Cursor Horizontal Absolute |
| CSI n ; m H | CUP | Yes | Yes | Cursor Position |
| CSI n ; m f | HVP | Yes | Yes | Horizontal Vertical Position |
| CSI n d | VPA | Yes | Yes | Vertical Position Absolute |

### Erase Operations

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| CSI n J | ED | Yes | Yes | Erase in Display (0=below, 1=above, 2=all, 3=scrollback) |
| CSI n K | EL | Yes | Yes | Erase in Line (0=right, 1=left, 2=all) |
| CSI n X | ECH | Yes | Yes | Erase Characters |

### Insert/Delete Operations

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| CSI n @ | ICH | Yes | Yes | Insert Characters |
| CSI n P | DCH | Yes | Yes | Delete Characters |
| CSI n L | IL | Yes | Yes | Insert Lines |
| CSI n M | DL | Yes | Yes | Delete Lines |

### Scrolling

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| CSI n S | SU | Yes | Yes | Scroll Up |
| CSI n T | SD | Yes | Yes | Scroll Down |
| CSI t ; b r | DECSTBM | Yes | Yes | Set Scroll Region |

### SGR (Select Graphic Rendition)

| Code | Meaning | Implemented | Tests | Notes |
|------|---------|-------------|-------|-------|
| 0 | Reset | Yes | Yes | Reset all attributes |
| 1 | Bold | Yes | Yes | Bold/bright |
| 2 | Faint | Yes | Yes | Dim |
| 3 | Italic | Yes | Yes | Italic |
| 4 | Underline | Yes | Yes | Underline |
| 5 | Blink | Yes | No | Slow blink (rendered as static) |
| 6 | Rapid Blink | Yes | No | Fast blink (rendered as static) |
| 7 | Inverse | Yes | Yes | Reverse video |
| 8 | Hidden | Yes | Yes | Invisible |
| 9 | Strikethrough | Yes | Yes | Crossed out |
| 21 | Double Underline | Partial | No | Treated as single underline |
| 22 | Normal Intensity | Yes | Yes | Not bold, not faint |
| 23 | Not Italic | Yes | Yes | |
| 24 | Not Underline | Yes | Yes | |
| 25 | Not Blink | Yes | Yes | |
| 27 | Not Inverse | Yes | Yes | |
| 28 | Not Hidden | Yes | Yes | |
| 29 | Not Strikethrough | Yes | Yes | |
| 30-37 | Foreground Color | Yes | Yes | Standard colors |
| 38;5;n | 256 Foreground | Yes | Yes | 256-color palette |
| 38;2;r;g;b | RGB Foreground | Yes | Yes | True color |
| 39 | Default Foreground | Yes | Yes | |
| 40-47 | Background Color | Yes | Yes | Standard colors |
| 48;5;n | 256 Background | Yes | Yes | 256-color palette |
| 48;2;r;g;b | RGB Background | Yes | Yes | True color |
| 49 | Default Background | Yes | Yes | |
| 90-97 | Bright Foreground | Yes | Yes | Bright colors |
| 100-107 | Bright Background | Yes | Yes | Bright colors |

### Cursor Save/Restore

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| CSI s | SCP | Yes | Yes | Save Cursor Position |
| CSI u | RCP | Yes | Yes | Restore Cursor Position |

### Tab Operations

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| CSI 0 g | TBC | Yes | Yes | Clear tab stop at cursor |
| CSI 3 g | TBC | Yes | Yes | Clear all tab stops |

### Device Status

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| CSI 5 n | DSR | Yes | No | Device Status Report (responds OK) |
| CSI 6 n | CPR | Yes | No | Cursor Position Report |
| CSI c | DA | Yes | No | Primary Device Attributes |
| CSI > c | DA2 | Yes | No | Secondary Device Attributes |

### DEC Private Modes (CSI ? n h/l)

| Mode | Name | Implemented | Tests | Notes |
|------|------|-------------|-------|-------|
| 1 | DECCKM | Yes | Yes | Application Cursor Keys |
| 6 | DECOM | Yes | Yes | Origin Mode |
| 7 | DECAWM | Yes | Yes | Auto-wrap Mode |
| 25 | DECTCEM | Yes | Yes | Cursor Visible |
| 47 | Alternate Screen | Yes | Yes | Switch to alternate buffer |
| 1000 | Mouse Tracking | Yes | Yes | VT200 mouse tracking (X10 compatible) |
| 1002 | Button Event | Yes | Yes | Button-event mouse tracking |
| 1003 | Any Event | Yes | Yes | Any-event mouse tracking (motion events) |
| 1004 | Focus Events | Yes | No | Focus in/out reporting |
| 1005 | UTF-8 Mouse | Yes | Yes | UTF-8 mouse encoding |
| 1006 | SGR Mouse | Yes | Yes | SGR mouse encoding (recommended) |
| 1015 | URXVT Mouse | Yes | Yes | URXVT mouse encoding |
| 1047 | Alternate Screen | Yes | Yes | Alternate buffer (no clear) |
| 1048 | Save Cursor | Yes | Yes | Save cursor for alternate |
| 1049 | Alternate Screen | Yes | Yes | Alternate buffer with save/restore |
| 2004 | Bracketed Paste | Yes | Yes | Bracketed paste mode |

### Standard Modes (CSI n h/l)

| Mode | Name | Implemented | Tests | Notes |
|------|------|-------------|-------|-------|
| 4 | IRM | Yes | Yes | Insert Mode |
| 20 | LNM | Yes | Yes | Line Feed/New Line Mode |

## OSC Sequences

| Command | Name | Implemented | Tests | Notes |
|---------|------|-------------|-------|-------|
| OSC 0 | Set Title | Yes | Yes | Set window title and icon |
| OSC 1 | Set Icon | Yes | No | Set icon name (ignored) |
| OSC 2 | Set Title | Yes | Yes | Set window title |
| OSC 4 | Set Color | Partial | No | Set palette color |
| OSC 8 | Hyperlink | Yes | No | Set hyperlink |
| OSC 10 | Set FG | Partial | No | Set foreground color |
| OSC 11 | Set BG | Partial | No | Set background color |
| OSC 52 | Clipboard | Yes | No | Clipboard access (disabled by default for security) |
| OSC 104 | Reset Color | Partial | No | Reset palette color |
| OSC 110 | Reset FG | Partial | No | Reset foreground color |
| OSC 111 | Reset BG | Partial | No | Reset background color |

## DCS/APC/PM/SOS

| Sequence | Implemented | Notes |
|----------|-------------|-------|
| DCS | Partial | Consumed but not interpreted |
| APC | Partial | Consumed but not interpreted |
| PM | Partial | Consumed but not interpreted |
| SOS | Partial | Consumed but not interpreted |

## Known Differences from xterm

1. **DEC Special Graphics**: Line drawing characters not yet implemented
2. **Sixel Graphics**: Not supported
3. **ReGIS Graphics**: Not supported
4. **Tektronix Mode**: Not supported
5. **OSC 52**: Disabled by default for security
6. **Blink**: Rendered as static (no animation)

## References

- [Xterm Control Sequences](https://www.x.org/docs/xterm/ctlseqs.pdf)
- [ECMA-48](https://ecma-international.org/wp-content/uploads/ECMA-48_5th_edition_june_1991.pdf)
- [VT220 Programmer Reference](https://vt100.net/dec/ek-vt220-rm-001.pdf)
