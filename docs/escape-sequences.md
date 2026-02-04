# Escape Sequence Coverage Matrix

This document tracks all escape sequences supported by Mochi terminal.

## Legend

- **Status**: `Yes` = Implemented and tested, `Partial` = Partially implemented, `No` = Not yet implemented, `N/A` = Will not implement
- **Tests**: Links to test files covering this sequence
- **Notes**: Implementation details or differences from xterm

## C0 Control Characters (0x00-0x1F)

| Code | Name | Meaning | Status | Tests | Notes |
|------|------|---------|--------|-------|-------|
| 0x00 | NUL | Null | Yes | | Ignored |
| 0x07 | BEL | Bell | Yes | | Logged (visual bell TBD) |
| 0x08 | BS | Backspace | Yes | golden_tests | Move cursor left |
| 0x09 | HT | Horizontal Tab | Yes | golden_tests | Move to next tab stop |
| 0x0A | LF | Line Feed | Yes | golden_tests | New line (may include CR) |
| 0x0B | VT | Vertical Tab | Yes | | Treated as LF |
| 0x0C | FF | Form Feed | Yes | | Treated as LF |
| 0x0D | CR | Carriage Return | Yes | golden_tests | Move cursor to column 0 |
| 0x0E | SO | Shift Out | Partial | | Logged, charset switching TBD |
| 0x0F | SI | Shift In | Partial | | Logged, charset switching TBD |
| 0x1B | ESC | Escape | Yes | golden_tests | Start escape sequence |

## ESC Sequences (Non-CSI)

| Sequence | Name | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| ESC 7 | DECSC | Save Cursor | Yes | golden_tests | Save cursor position and attributes |
| ESC 8 | DECRC | Restore Cursor | Yes | golden_tests | Restore saved cursor |
| ESC D | IND | Index | Yes | | Move cursor down, scroll if at bottom |
| ESC E | NEL | Next Line | Yes | | Move to start of next line |
| ESC H | HTS | Horizontal Tab Set | Yes | | Set tab stop at current column |
| ESC M | RI | Reverse Index | Yes | golden_tests | Move cursor up, scroll if at top |
| ESC ( B | | Select ASCII charset (G0) | Partial | | Logged |
| ESC ( 0 | | Select DEC Special Graphics (G0) | Partial | | Logged |
| ESC ) B | | Select ASCII charset (G1) | Partial | | Logged |
| ESC ) 0 | | Select DEC Special Graphics (G1) | Partial | | Logged |
| ESC = | DECKPAM | Keypad Application Mode | Yes | | |
| ESC > | DECKPNM | Keypad Numeric Mode | Yes | | |
| ESC c | RIS | Reset to Initial State | Yes | | Full terminal reset |

## CSI Sequences (ESC [ ...)

### Cursor Movement

| Sequence | Name | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| CSI Ps A | CUU | Cursor Up | Yes | golden_tests | Move cursor up Ps rows |
| CSI Ps B | CUD | Cursor Down | Yes | golden_tests | Move cursor down Ps rows |
| CSI Ps C | CUF | Cursor Forward | Yes | golden_tests | Move cursor right Ps columns |
| CSI Ps D | CUB | Cursor Back | Yes | golden_tests | Move cursor left Ps columns |
| CSI Ps E | CNL | Cursor Next Line | Yes | | Move to start of Ps-th next line |
| CSI Ps F | CPL | Cursor Previous Line | Yes | | Move to start of Ps-th previous line |
| CSI Ps G | CHA | Cursor Horizontal Absolute | Yes | golden_tests | Move to column Ps |
| CSI Ps ; Ps H | CUP | Cursor Position | Yes | golden_tests | Move to row;column |
| CSI Ps d | VPA | Vertical Position Absolute | Yes | golden_tests | Move to row Ps |
| CSI Ps ; Ps f | HVP | Horizontal and Vertical Position | Yes | | Same as CUP |

### Erase Operations

| Sequence | Name | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| CSI Ps J | ED | Erase in Display | Yes | golden_tests | 0=below, 1=above, 2=all, 3=scrollback |
| CSI Ps K | EL | Erase in Line | Yes | golden_tests | 0=right, 1=left, 2=all |
| CSI Ps X | ECH | Erase Characters | Yes | | Erase Ps characters |

### Insert/Delete Operations

| Sequence | Name | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| CSI Ps @ | ICH | Insert Characters | Yes | golden_tests | Insert Ps blank characters |
| CSI Ps P | DCH | Delete Characters | Yes | golden_tests | Delete Ps characters |
| CSI Ps L | IL | Insert Lines | Yes | golden_tests | Insert Ps blank lines |
| CSI Ps M | DL | Delete Lines | Yes | golden_tests | Delete Ps lines |

### Scroll Region

| Sequence | Name | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| CSI Ps ; Ps r | DECSTBM | Set Top and Bottom Margins | Yes | golden_tests | Set scroll region |
| CSI Ps S | SU | Scroll Up | Yes | | Scroll up Ps lines |
| CSI Ps T | SD | Scroll Down | Yes | | Scroll down Ps lines |

### Tab Operations

| Sequence | Name | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| CSI Ps g | TBC | Tab Clear | Yes | | 0=current, 3=all |

### Cursor Save/Restore

| Sequence | Name | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| CSI s | SCOSC | Save Cursor Position | Yes | golden_tests | SCO variant |
| CSI u | SCORC | Restore Cursor Position | Yes | golden_tests | SCO variant |

### SGR (Select Graphic Rendition)

| Sequence | Meaning | Status | Tests | Notes |
|----------|---------|--------|-------|-------|
| CSI 0 m | Reset all attributes | Yes | golden_tests | |
| CSI 1 m | Bold | Yes | golden_tests | |
| CSI 2 m | Faint/Dim | Yes | golden_tests | |
| CSI 3 m | Italic | Yes | golden_tests | |
| CSI 4 m | Underline | Yes | golden_tests | |
| CSI 5 m | Slow Blink | Yes | | |
| CSI 7 m | Inverse/Reverse | Yes | golden_tests | |
| CSI 8 m | Hidden/Invisible | Yes | | |
| CSI 9 m | Strikethrough | Yes | golden_tests | |
| CSI 22 m | Normal intensity | Yes | golden_tests | Reset bold/faint |
| CSI 23 m | Not italic | Yes | golden_tests | |
| CSI 24 m | Not underlined | Yes | golden_tests | |
| CSI 25 m | Not blinking | Yes | | |
| CSI 27 m | Not inverse | Yes | golden_tests | |
| CSI 28 m | Not hidden | Yes | | |
| CSI 29 m | Not strikethrough | Yes | golden_tests | |
| CSI 30-37 m | Set foreground color (8 colors) | Yes | golden_tests | |
| CSI 38;5;Ps m | Set foreground color (256) | Yes | golden_tests | |
| CSI 38;2;R;G;B m | Set foreground color (RGB) | Yes | golden_tests | |
| CSI 39 m | Default foreground | Yes | golden_tests | |
| CSI 40-47 m | Set background color (8 colors) | Yes | golden_tests | |
| CSI 48;5;Ps m | Set background color (256) | Yes | golden_tests | |
| CSI 48;2;R;G;B m | Set background color (RGB) | Yes | golden_tests | |
| CSI 49 m | Default background | Yes | golden_tests | |
| CSI 90-97 m | Set bright foreground (8 colors) | Yes | golden_tests | |
| CSI 100-107 m | Set bright background (8 colors) | Yes | golden_tests | |

### Mode Set/Reset

| Sequence | Mode | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| CSI ? 1 h/l | DECCKM | Cursor Keys Mode | Yes | | Application/Normal |
| CSI ? 6 h/l | DECOM | Origin Mode | Yes | | Relative/Absolute |
| CSI ? 7 h/l | DECAWM | Auto Wrap Mode | Yes | | |
| CSI ? 12 h/l | | Cursor Blink | Yes | | |
| CSI ? 25 h/l | DECTCEM | Cursor Visible | Yes | golden_tests | |
| CSI ? 47 h/l | | Alternate Screen (old) | Yes | | |
| CSI ? 1000 h/l | | Mouse X10 Mode | Yes | | |
| CSI ? 1002 h/l | | Mouse Button Event | Yes | | |
| CSI ? 1003 h/l | | Mouse Any Event | Yes | | |
| CSI ? 1004 h/l | | Focus Reporting | Yes | | |
| CSI ? 1006 h/l | | Mouse SGR Mode | Yes | | |
| CSI ? 1049 h/l | | Alternate Screen + Save Cursor | Yes | golden_tests | |
| CSI ? 2004 h/l | | Bracketed Paste Mode | Yes | | |

### Device Status

| Sequence | Name | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| CSI 5 n | DSR | Device Status Report | No | | Reply: CSI 0 n |
| CSI 6 n | CPR | Cursor Position Report | No | | Reply: CSI row;col R |
| CSI ? 6 n | DECXCPR | Extended Cursor Position | No | | |
| CSI c | DA | Device Attributes | No | | Primary DA |
| CSI > c | DA2 | Secondary Device Attributes | No | | |

## OSC Sequences (ESC ] ...)

| Sequence | Meaning | Status | Tests | Notes |
|----------|---------|--------|-------|-------|
| OSC 0 ; Pt ST | Set Icon Name and Window Title | Yes | golden_tests | |
| OSC 1 ; Pt ST | Set Icon Name | Yes | | Treated same as title |
| OSC 2 ; Pt ST | Set Window Title | Yes | golden_tests | |
| OSC 4 ; c ; spec ST | Set Color | Partial | | Logged, not applied |
| OSC 8 ; params ; uri ST | Hyperlink | Yes | | With URI validation |
| OSC 10 ; Pt ST | Set Foreground Color | Partial | | Logged, not applied |
| OSC 11 ; Pt ST | Set Background Color | Partial | | Logged, not applied |
| OSC 12 ; Pt ST | Set Cursor Color | Partial | | Logged, not applied |
| OSC 52 ; c ; data ST | Clipboard | Partial | | Disabled by default for security |
| OSC 104 ; c ST | Reset Color | No | | |
| OSC 110 ST | Reset Foreground | No | | |
| OSC 111 ST | Reset Background | No | | |
| OSC 112 ST | Reset Cursor Color | No | | |

## DCS Sequences (ESC P ...)

| Sequence | Meaning | Status | Tests | Notes |
|----------|---------|--------|-------|-------|
| DCS $ q " p ST | DECRQSS | No | | Request setting |
| DCS + q Pt ST | XTGETTCAP | No | | Request termcap |

## Notes on Implementation

### Differences from xterm

1. **OSC 52 Security**: Disabled by default, requires explicit opt-in
2. **Mouse Modes**: Only SGR 1006 mode fully supported initially
3. **Character Sets**: Only ASCII and DEC Special Graphics initially

### Known Limitations

1. Sixel graphics not supported
2. ReGIS graphics not supported
3. Tektronix 4014 mode not supported
4. Some obscure DEC private modes not implemented

## References

- [XTerm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)
- [ECMA-48](https://ecma-international.org/publications-and-standards/standards/ecma-48/)
- [VT100.net Parser](https://vt100.net/emu/dec_ansi_parser)
