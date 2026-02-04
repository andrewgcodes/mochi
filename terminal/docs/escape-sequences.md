# Escape Sequences Reference

This document lists all escape sequences supported by Mochi Terminal.

## Legend

- **Implemented**: Yes = fully working, Partial = basic support, No = not yet
- **Tests**: Unit = unit tests, Golden = golden snapshot tests

## C0 Control Characters

| Code | Name | Meaning | Implemented | Tests |
|------|------|---------|-------------|-------|
| 0x00 | NUL | Null (ignored) | Yes | Unit |
| 0x07 | BEL | Bell | Yes | Unit |
| 0x08 | BS | Backspace | Yes | Unit |
| 0x09 | HT | Horizontal Tab | Yes | Unit |
| 0x0A | LF | Line Feed | Yes | Unit |
| 0x0B | VT | Vertical Tab (treated as LF) | Yes | Unit |
| 0x0C | FF | Form Feed (treated as LF) | Yes | Unit |
| 0x0D | CR | Carriage Return | Yes | Unit |
| 0x0E | SO | Shift Out (G1 charset) | Yes | Unit |
| 0x0F | SI | Shift In (G0 charset) | Yes | Unit |
| 0x1B | ESC | Escape | Yes | Unit |
| 0x7F | DEL | Delete (ignored) | Yes | Unit |

## ESC Sequences

| Sequence | Name | Meaning | Implemented | Tests |
|----------|------|---------|-------------|-------|
| ESC 7 | DECSC | Save Cursor | Yes | Unit |
| ESC 8 | DECRC | Restore Cursor | Yes | Unit |
| ESC D | IND | Index (move down, scroll if needed) | Yes | Unit |
| ESC E | NEL | Next Line | Yes | Unit |
| ESC H | HTS | Horizontal Tab Set | Yes | Unit |
| ESC M | RI | Reverse Index (move up, scroll if needed) | Yes | Unit |
| ESC c | RIS | Reset to Initial State | Yes | Unit |
| ESC = | DECKPAM | Application Keypad Mode | Yes | Unit |
| ESC > | DECKPNM | Normal Keypad Mode | Yes | Unit |
| ESC ( B | SCS | Designate G0 as ASCII | Yes | Unit |
| ESC ( 0 | SCS | Designate G0 as DEC Special Graphics | Yes | Unit |
| ESC ( A | SCS | Designate G0 as UK | Yes | Unit |
| ESC ) B | SCS | Designate G1 as ASCII | Yes | Unit |
| ESC ) 0 | SCS | Designate G1 as DEC Special Graphics | Yes | Unit |
| ESC N | SS2 | Single Shift G2 | Partial | - |
| ESC O | SS3 | Single Shift G3 | Partial | - |

## CSI Sequences (Cursor Movement)

| Sequence | Name | Meaning | Implemented | Tests |
|----------|------|---------|-------------|-------|
| CSI n A | CUU | Cursor Up n (default 1) | Yes | Unit, Golden |
| CSI n B | CUD | Cursor Down n | Yes | Unit, Golden |
| CSI n C | CUF | Cursor Forward n | Yes | Unit, Golden |
| CSI n D | CUB | Cursor Back n | Yes | Unit, Golden |
| CSI n E | CNL | Cursor Next Line n | Yes | Unit |
| CSI n F | CPL | Cursor Previous Line n | Yes | Unit |
| CSI n G | CHA | Cursor Horizontal Absolute | Yes | Unit |
| CSI n ; m H | CUP | Cursor Position (row; col) | Yes | Unit, Golden |
| CSI n ; m f | HVP | Horizontal Vertical Position | Yes | Unit |
| CSI n d | VPA | Vertical Position Absolute | Yes | Unit |
| CSI n ` | HPA | Horizontal Position Absolute | Yes | Unit |
| CSI n a | HPR | Horizontal Position Relative | Yes | Unit |
| CSI n e | VPR | Vertical Position Relative | Yes | Unit |

## CSI Sequences (Erase)

| Sequence | Name | Meaning | Implemented | Tests |
|----------|------|---------|-------------|-------|
| CSI 0 J | ED | Erase Below (cursor to end) | Yes | Unit, Golden |
| CSI 1 J | ED | Erase Above (start to cursor) | Yes | Unit, Golden |
| CSI 2 J | ED | Erase All | Yes | Unit, Golden |
| CSI 3 J | ED | Erase Scrollback | Yes | Unit |
| CSI 0 K | EL | Erase to End of Line | Yes | Unit, Golden |
| CSI 1 K | EL | Erase to Beginning of Line | Yes | Unit, Golden |
| CSI 2 K | EL | Erase Entire Line | Yes | Unit, Golden |
| CSI n X | ECH | Erase n Characters | Yes | Unit |

## CSI Sequences (Insert/Delete)

| Sequence | Name | Meaning | Implemented | Tests |
|----------|------|---------|-------------|-------|
| CSI n @ | ICH | Insert n Characters | Yes | Unit |
| CSI n P | DCH | Delete n Characters | Yes | Unit |
| CSI n L | IL | Insert n Lines | Yes | Unit |
| CSI n M | DL | Delete n Lines | Yes | Unit |

## CSI Sequences (Scroll)

| Sequence | Name | Meaning | Implemented | Tests |
|----------|------|---------|-------------|-------|
| CSI n S | SU | Scroll Up n lines | Yes | Unit |
| CSI n T | SD | Scroll Down n lines | Yes | Unit |
| CSI t ; b r | DECSTBM | Set Scroll Region (top; bottom) | Yes | Unit, Golden |

## CSI Sequences (SGR - Select Graphic Rendition)

| Sequence | Meaning | Implemented | Tests |
|----------|---------|-------------|-------|
| CSI 0 m | Reset all attributes | Yes | Unit |
| CSI 1 m | Bold | Yes | Unit |
| CSI 2 m | Faint/Dim | Yes | Unit |
| CSI 3 m | Italic | Yes | Unit |
| CSI 4 m | Underline | Yes | Unit |
| CSI 5 m | Slow Blink | Yes | Unit |
| CSI 6 m | Rapid Blink | Yes | Unit |
| CSI 7 m | Inverse/Reverse | Yes | Unit |
| CSI 8 m | Hidden/Invisible | Yes | Unit |
| CSI 9 m | Strikethrough | Yes | Unit |
| CSI 21 m | Double Underline / Bold Off | Yes | Unit |
| CSI 22 m | Normal Intensity | Yes | Unit |
| CSI 23 m | Not Italic | Yes | Unit |
| CSI 24 m | Not Underlined | Yes | Unit |
| CSI 25 m | Not Blinking | Yes | Unit |
| CSI 27 m | Not Inverse | Yes | Unit |
| CSI 28 m | Not Hidden | Yes | Unit |
| CSI 29 m | Not Strikethrough | Yes | Unit |
| CSI 30-37 m | Foreground Color (8 colors) | Yes | Unit |
| CSI 38;5;n m | Foreground 256 Color | Yes | Unit |
| CSI 38;2;r;g;b m | Foreground True Color | Yes | Unit |
| CSI 39 m | Default Foreground | Yes | Unit |
| CSI 40-47 m | Background Color (8 colors) | Yes | Unit |
| CSI 48;5;n m | Background 256 Color | Yes | Unit |
| CSI 48;2;r;g;b m | Background True Color | Yes | Unit |
| CSI 49 m | Default Background | Yes | Unit |
| CSI 90-97 m | Bright Foreground (8 colors) | Yes | Unit |
| CSI 100-107 m | Bright Background (8 colors) | Yes | Unit |

## CSI Sequences (Modes)

| Sequence | Name | Meaning | Implemented | Tests |
|----------|------|---------|-------------|-------|
| CSI 4 h | IRM | Insert Mode On | Yes | Unit |
| CSI 4 l | IRM | Insert Mode Off | Yes | Unit |
| CSI 20 h | LNM | Linefeed Mode On | Yes | Unit |
| CSI 20 l | LNM | Linefeed Mode Off | Yes | Unit |

## CSI Private Sequences (DEC Modes)

| Sequence | Name | Meaning | Implemented | Tests |
|----------|------|---------|-------------|-------|
| CSI ? 1 h/l | DECCKM | Application Cursor Keys | Yes | Unit |
| CSI ? 6 h/l | DECOM | Origin Mode | Yes | Unit |
| CSI ? 7 h/l | DECAWM | Auto-wrap Mode | Yes | Unit |
| CSI ? 12 h/l | att610 | Cursor Blinking | Yes | Unit |
| CSI ? 25 h/l | DECTCEM | Cursor Visible | Yes | Unit, Golden |
| CSI ? 47 h/l | - | Alternate Screen (old) | Yes | Unit |
| CSI ? 1000 h/l | - | X10 Mouse Reporting | Yes | Unit |
| CSI ? 1002 h/l | - | Button Event Mouse | Yes | Unit |
| CSI ? 1003 h/l | - | Any Event Mouse | Yes | Unit |
| CSI ? 1004 h/l | - | Focus Reporting | Yes | Unit |
| CSI ? 1005 h/l | - | UTF-8 Mouse Encoding | Yes | Unit |
| CSI ? 1006 h/l | - | SGR Mouse Encoding | Yes | Unit |
| CSI ? 1015 h/l | - | URXVT Mouse Encoding | Yes | Unit |
| CSI ? 1047 h/l | - | Alternate Screen | Yes | Unit |
| CSI ? 1048 h/l | - | Save/Restore Cursor | Yes | Unit |
| CSI ? 1049 h/l | - | Alt Screen + Cursor | Yes | Unit, Golden |
| CSI ? 2004 h/l | - | Bracketed Paste Mode | Yes | Unit |

## CSI Sequences (Other)

| Sequence | Name | Meaning | Implemented | Tests |
|----------|------|---------|-------------|-------|
| CSI s | SCP | Save Cursor Position | Yes | Unit |
| CSI u | RCP | Restore Cursor Position | Yes | Unit |
| CSI 0 g | TBC | Clear Tab Stop at Cursor | Yes | Unit |
| CSI 3 g | TBC | Clear All Tab Stops | Yes | Unit |
| CSI n SP q | DECSCUSR | Set Cursor Style | Yes | Unit |
| CSI ! p | DECSTR | Soft Terminal Reset | Yes | Unit |
| CSI n | DSR | Device Status Report | Partial | - |

### Cursor Styles (DECSCUSR)

| Value | Style |
|-------|-------|
| 0, 1 | Blinking Block |
| 2 | Steady Block |
| 3 | Blinking Underline |
| 4 | Steady Underline |
| 5 | Blinking Bar |
| 6 | Steady Bar |

## OSC Sequences

| Sequence | Meaning | Implemented | Tests |
|----------|---------|-------------|-------|
| OSC 0 ; text BEL/ST | Set Icon Name and Window Title | Yes | Unit |
| OSC 1 ; text BEL/ST | Set Icon Name | Yes | Unit |
| OSC 2 ; text BEL/ST | Set Window Title | Yes | Unit |
| OSC 4 ; ... BEL/ST | Change Color Palette | No | - |
| OSC 7 ; uri BEL/ST | Set Working Directory | Partial | - |
| OSC 8 ; params ; uri BEL/ST | Hyperlink | Yes | Unit |
| OSC 10 ; ? BEL/ST | Query Foreground Color | No | - |
| OSC 11 ; ? BEL/ST | Query Background Color | No | - |
| OSC 12 ; ? BEL/ST | Query Cursor Color | No | - |
| OSC 52 ; c ; data BEL/ST | Clipboard Access | No (security) | - |
| OSC 104 BEL/ST | Reset Color | No | - |
| OSC 112 BEL/ST | Reset Cursor Color | No | - |

## DCS Sequences

DCS sequences are parsed and consumed but not fully implemented:

| Sequence | Meaning | Implemented |
|----------|---------|-------------|
| DCS ... ST | Device Control String | Consumed, not acted upon |

## Differences from xterm

### Intentional Differences

1. **OSC 52 Clipboard**: Disabled by default for security. Can be enabled in config.

2. **Sixel Graphics**: Not implemented. May be added in future.

3. **ReGIS Graphics**: Not implemented.

### Known Limitations

1. **Combining Characters**: Basic support. Complex sequences may not render correctly.

2. **Bidirectional Text**: Not supported.

3. **Double-width/Double-height Lines**: Not supported.

## References

- [Xterm Control Sequences](https://www.x.org/docs/xterm/ctlseqs.pdf)
- [ECMA-48](https://ecma-international.org/wp-content/uploads/ECMA-48_5th_edition_june_1991.pdf)
- [VT220 Programmer Reference](https://vt100.net/dec/ek-vt220-rm-001.pdf)
