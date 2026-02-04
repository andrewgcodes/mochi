# Escape Sequences Coverage Matrix

This document tracks the implementation status of VT/xterm escape sequences in the Mochi terminal emulator.

## Legend

| Status | Meaning |
|--------|---------|
| Yes | Fully implemented and tested |
| Partial | Implemented but may have edge cases |
| No | Not yet implemented |
| N/A | Not applicable or intentionally unsupported |

## C0 Control Characters (0x00-0x1F)

| Code | Name | Implemented | Tests | Notes |
|------|------|-------------|-------|-------|
| 0x00 | NUL | Yes | Unit | Ignored |
| 0x07 | BEL | Yes | Unit | Triggers bell callback |
| 0x08 | BS | Yes | Unit | Moves cursor left |
| 0x09 | HT | Yes | Unit | Moves to next tab stop |
| 0x0A | LF | Yes | Unit | Line feed, scrolls if at bottom |
| 0x0B | VT | Yes | Unit | Treated as LF |
| 0x0C | FF | Yes | Unit | Treated as LF |
| 0x0D | CR | Yes | Unit | Moves cursor to column 0 |
| 0x1B | ESC | Yes | Unit | Escape sequence introducer |

## ESC Sequences

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| ESC 7 | DECSC | Yes | Unit | Save cursor position and attributes |
| ESC 8 | DECRC | Yes | Unit | Restore cursor position and attributes |
| ESC D | IND | Yes | Unit | Index (move down, scroll if needed) |
| ESC E | NEL | Yes | Unit | Next line (CR + LF) |
| ESC H | HTS | Yes | Unit | Set tab stop at current column |
| ESC M | RI | Yes | Unit | Reverse index (move up, scroll if needed) |
| ESC ( B | SCS | Partial | - | Select ASCII character set |
| ESC ( 0 | SCS | No | - | Select DEC Special Graphics |
| ESC c | RIS | No | - | Full reset |
| ESC = | DECKPAM | No | - | Application keypad mode |
| ESC > | DECKPNM | No | - | Normal keypad mode |

## CSI Sequences (ESC [ ...)

### Cursor Movement

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| CSI n A | CUU | Yes | Unit | Cursor up n rows |
| CSI n B | CUD | Yes | Unit | Cursor down n rows |
| CSI n C | CUF | Yes | Unit | Cursor forward n columns |
| CSI n D | CUB | Yes | Unit | Cursor backward n columns |
| CSI n G | CHA | Yes | Unit | Cursor to column n |
| CSI n d | VPA | Yes | Unit | Cursor to row n |
| CSI n;m H | CUP | Yes | Unit | Cursor to row n, column m |
| CSI n;m f | HVP | Yes | Unit | Same as CUP |
| CSI s | SCP | Yes | Unit | Save cursor position |
| CSI u | RCP | Yes | Unit | Restore cursor position |

### Erase Operations

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| CSI n J | ED | Yes | Unit | Erase in display (0=below, 1=above, 2=all, 3=scrollback) |
| CSI n K | EL | Yes | Unit | Erase in line (0=right, 1=left, 2=all) |
| CSI n X | ECH | Yes | Unit | Erase n characters |

### Insert/Delete Operations

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| CSI n @ | ICH | Yes | Unit | Insert n blank characters |
| CSI n P | DCH | Yes | Unit | Delete n characters |
| CSI n L | IL | Yes | Unit | Insert n blank lines |
| CSI n M | DL | Yes | Unit | Delete n lines |

### Scroll Region

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| CSI t;b r | DECSTBM | Yes | Unit | Set scroll region (top;bottom) |

### Tab Operations

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| CSI n g | TBC | Yes | Unit | Tab clear (0=current, 3=all) |

### SGR (Select Graphic Rendition)

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| CSI 0 m | Reset | Yes | Unit | Reset all attributes |
| CSI 1 m | Bold | Yes | Unit | Bold/bright |
| CSI 2 m | Faint | Yes | Unit | Dim/faint |
| CSI 3 m | Italic | Yes | Unit | Italic |
| CSI 4 m | Underline | Yes | Unit | Underline |
| CSI 7 m | Inverse | Yes | Unit | Reverse video |
| CSI 8 m | Hidden | Yes | Unit | Hidden/invisible |
| CSI 9 m | Strike | Yes | Unit | Strikethrough |
| CSI 22 m | Normal | Yes | Unit | Normal intensity |
| CSI 23 m | No Italic | Yes | Unit | Not italic |
| CSI 24 m | No Underline | Yes | Unit | Not underlined |
| CSI 27 m | No Inverse | Yes | Unit | Not inverse |
| CSI 28 m | No Hidden | Yes | Unit | Not hidden |
| CSI 29 m | No Strike | Yes | Unit | Not strikethrough |
| CSI 30-37 m | FG Color | Yes | Unit | ANSI foreground colors |
| CSI 38;5;n m | FG 256 | Yes | Unit | 256-color foreground |
| CSI 38;2;r;g;b m | FG RGB | Yes | Unit | Truecolor foreground |
| CSI 39 m | FG Default | Yes | Unit | Default foreground |
| CSI 40-47 m | BG Color | Yes | Unit | ANSI background colors |
| CSI 48;5;n m | BG 256 | Yes | Unit | 256-color background |
| CSI 48;2;r;g;b m | BG RGB | Yes | Unit | Truecolor background |
| CSI 49 m | BG Default | Yes | Unit | Default background |
| CSI 90-97 m | Bright FG | Yes | Unit | Bright foreground colors |
| CSI 100-107 m | Bright BG | Yes | Unit | Bright background colors |

### DEC Private Modes (CSI ? n h/l)

| Mode | Name | Implemented | Tests | Notes |
|------|------|-------------|-------|-------|
| ?1 | DECCKM | Partial | - | Application cursor keys |
| ?6 | DECOM | Yes | Unit | Origin mode |
| ?7 | DECAWM | Yes | Unit | Autowrap mode |
| ?12 | Cursor Blink | No | - | Cursor blinking |
| ?25 | DECTCEM | Yes | Unit | Cursor visibility |
| ?1000 | X10 Mouse | Yes | Unit | X10 mouse tracking |
| ?1002 | Button Event | Yes | Unit | Button event mouse tracking |
| ?1003 | Any Event | Yes | Unit | Any event mouse tracking |
| ?1006 | SGR Mouse | Yes | Unit | SGR extended mouse encoding |
| ?1049 | Alt Screen | Yes | Unit | Alternate screen buffer |
| ?2004 | Bracketed Paste | Yes | Unit | Bracketed paste mode |

### Other CSI Sequences

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| CSI n c | DA | No | - | Device attributes |
| CSI n n | DSR | No | - | Device status report |
| CSI n t | Window | No | - | Window manipulation |

## OSC Sequences (ESC ] ...)

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| OSC 0 ; text BEL | Title | Yes | Unit | Set icon name and window title |
| OSC 2 ; text BEL | Title | Yes | Unit | Set window title |
| OSC 8 ; params ; uri ST | Hyperlink | Yes | Unit | Hyperlinks |
| OSC 52 ; c ; data BEL | Clipboard | Partial | - | Clipboard operations (security controls needed) |

## DCS Sequences (ESC P ...)

| Sequence | Name | Implemented | Tests | Notes |
|----------|------|-------------|-------|-------|
| DCS + q ... ST | XTGETTCAP | No | - | Request terminfo capability |
| DCS $ q ... ST | DECRQSS | No | - | Request selection or setting |

## Known Differences from xterm

1. **Character Sets**: Only ASCII character set is fully supported. DEC Special Graphics (line drawing) is not yet implemented.

2. **Soft Reset**: DECSTR (CSI ! p) is not implemented.

3. **Device Attributes**: DA1/DA2/DA3 responses are not implemented.

4. **Window Operations**: CSI t window manipulation sequences are not implemented.

5. **Sixel Graphics**: Not supported.

6. **ReGIS Graphics**: Not supported.

## Testing Strategy

### Unit Tests
Each escape sequence has unit tests that verify:
- Basic functionality
- Edge cases (zero parameters, out-of-bounds values)
- Interaction with other terminal state

### Golden Tests
Planned: Store input byte streams and expected screen snapshots for regression testing.

### Integration Tests
Planned: Spawn real applications and verify screen state.

### Fuzzing
Planned: Fuzz the parser to ensure no crashes or hangs on malformed input.

## References

- [Xterm Control Sequences](https://www.x.org/docs/xterm/ctlseqs.pdf) - Primary reference
- [ECMA-48](https://ecma-international.org/wp-content/uploads/ECMA-48_5th_edition_june_1991.pdf) - Formal standard
- [VT220 Programmer Reference](https://vt100.net/dec/ek-vt220-rm-001.pdf) - DEC terminal behavior
- [vttest](https://invisible-island.net/vttest/) - Terminal test suite
