# Escape Sequence Coverage Matrix

This document tracks all escape sequences supported by Mochi terminal.

## Legend

- **Status**: `Yes` = Implemented and tested, `Partial` = Partially implemented, `No` = Not yet implemented, `N/A` = Will not implement
- **Tests**: Links to test files covering this sequence
- **Notes**: Implementation details or differences from xterm

## C0 Control Characters (0x00-0x1F)

| Code | Name | Meaning | Status | Tests | Notes |
|------|------|---------|--------|-------|-------|
| 0x00 | NUL | Null | No | | Ignored |
| 0x07 | BEL | Bell | No | | Visual bell or system bell |
| 0x08 | BS | Backspace | No | | Move cursor left |
| 0x09 | HT | Horizontal Tab | No | | Move to next tab stop |
| 0x0A | LF | Line Feed | No | | New line (may include CR) |
| 0x0B | VT | Vertical Tab | No | | Treated as LF |
| 0x0C | FF | Form Feed | No | | Treated as LF |
| 0x0D | CR | Carriage Return | No | | Move cursor to column 0 |
| 0x0E | SO | Shift Out | No | | Switch to G1 charset |
| 0x0F | SI | Shift In | No | | Switch to G0 charset |
| 0x1B | ESC | Escape | No | | Start escape sequence |

## ESC Sequences (Non-CSI)

| Sequence | Name | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| ESC 7 | DECSC | Save Cursor | No | | Save cursor position and attributes |
| ESC 8 | DECRC | Restore Cursor | No | | Restore saved cursor |
| ESC D | IND | Index | No | | Move cursor down, scroll if at bottom |
| ESC E | NEL | Next Line | No | | Move to start of next line |
| ESC H | HTS | Horizontal Tab Set | No | | Set tab stop at current column |
| ESC M | RI | Reverse Index | No | | Move cursor up, scroll if at top |
| ESC ( B | | Select ASCII charset (G0) | No | | |
| ESC ( 0 | | Select DEC Special Graphics (G0) | No | | Line drawing characters |
| ESC ) B | | Select ASCII charset (G1) | No | | |
| ESC ) 0 | | Select DEC Special Graphics (G1) | No | | |
| ESC = | DECKPAM | Keypad Application Mode | No | | |
| ESC > | DECKPNM | Keypad Numeric Mode | No | | |
| ESC c | RIS | Reset to Initial State | No | | Full terminal reset |

## CSI Sequences (ESC [ ...)

### Cursor Movement

| Sequence | Name | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| CSI Ps A | CUU | Cursor Up | No | | Move cursor up Ps rows |
| CSI Ps B | CUD | Cursor Down | No | | Move cursor down Ps rows |
| CSI Ps C | CUF | Cursor Forward | No | | Move cursor right Ps columns |
| CSI Ps D | CUB | Cursor Back | No | | Move cursor left Ps columns |
| CSI Ps E | CNL | Cursor Next Line | No | | Move to start of Ps-th next line |
| CSI Ps F | CPL | Cursor Previous Line | No | | Move to start of Ps-th previous line |
| CSI Ps G | CHA | Cursor Horizontal Absolute | No | | Move to column Ps |
| CSI Ps ; Ps H | CUP | Cursor Position | No | | Move to row;column |
| CSI Ps d | VPA | Vertical Position Absolute | No | | Move to row Ps |
| CSI Ps ; Ps f | HVP | Horizontal and Vertical Position | No | | Same as CUP |

### Erase Operations

| Sequence | Name | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| CSI Ps J | ED | Erase in Display | No | | 0=below, 1=above, 2=all, 3=scrollback |
| CSI Ps K | EL | Erase in Line | No | | 0=right, 1=left, 2=all |
| CSI Ps X | ECH | Erase Characters | No | | Erase Ps characters |

### Insert/Delete Operations

| Sequence | Name | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| CSI Ps @ | ICH | Insert Characters | No | | Insert Ps blank characters |
| CSI Ps P | DCH | Delete Characters | No | | Delete Ps characters |
| CSI Ps L | IL | Insert Lines | No | | Insert Ps blank lines |
| CSI Ps M | DL | Delete Lines | No | | Delete Ps lines |

### Scroll Region

| Sequence | Name | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| CSI Ps ; Ps r | DECSTBM | Set Top and Bottom Margins | No | | Set scroll region |
| CSI Ps S | SU | Scroll Up | No | | Scroll up Ps lines |
| CSI Ps T | SD | Scroll Down | No | | Scroll down Ps lines |

### Tab Operations

| Sequence | Name | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| CSI Ps g | TBC | Tab Clear | No | | 0=current, 3=all |

### Cursor Save/Restore

| Sequence | Name | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| CSI s | SCOSC | Save Cursor Position | No | | SCO variant |
| CSI u | SCORC | Restore Cursor Position | No | | SCO variant |

### SGR (Select Graphic Rendition)

| Sequence | Meaning | Status | Tests | Notes |
|----------|---------|--------|-------|-------|
| CSI 0 m | Reset all attributes | No | | |
| CSI 1 m | Bold | No | | |
| CSI 2 m | Faint/Dim | No | | |
| CSI 3 m | Italic | No | | |
| CSI 4 m | Underline | No | | |
| CSI 5 m | Slow Blink | No | | |
| CSI 7 m | Inverse/Reverse | No | | |
| CSI 8 m | Hidden/Invisible | No | | |
| CSI 9 m | Strikethrough | No | | |
| CSI 22 m | Normal intensity | No | | Reset bold/faint |
| CSI 23 m | Not italic | No | | |
| CSI 24 m | Not underlined | No | | |
| CSI 25 m | Not blinking | No | | |
| CSI 27 m | Not inverse | No | | |
| CSI 28 m | Not hidden | No | | |
| CSI 29 m | Not strikethrough | No | | |
| CSI 30-37 m | Set foreground color (8 colors) | No | | |
| CSI 38;5;Ps m | Set foreground color (256) | No | | |
| CSI 38;2;R;G;B m | Set foreground color (RGB) | No | | |
| CSI 39 m | Default foreground | No | | |
| CSI 40-47 m | Set background color (8 colors) | No | | |
| CSI 48;5;Ps m | Set background color (256) | No | | |
| CSI 48;2;R;G;B m | Set background color (RGB) | No | | |
| CSI 49 m | Default background | No | | |
| CSI 90-97 m | Set bright foreground (8 colors) | No | | |
| CSI 100-107 m | Set bright background (8 colors) | No | | |

### Mode Set/Reset

| Sequence | Mode | Meaning | Status | Tests | Notes |
|----------|------|---------|--------|-------|-------|
| CSI ? 1 h/l | DECCKM | Cursor Keys Mode | No | | Application/Normal |
| CSI ? 6 h/l | DECOM | Origin Mode | No | | Relative/Absolute |
| CSI ? 7 h/l | DECAWM | Auto Wrap Mode | No | | |
| CSI ? 12 h/l | | Cursor Blink | No | | |
| CSI ? 25 h/l | DECTCEM | Cursor Visible | No | | |
| CSI ? 47 h/l | | Alternate Screen (old) | No | | |
| CSI ? 1000 h/l | | Mouse X10 Mode | No | | |
| CSI ? 1002 h/l | | Mouse Button Event | No | | |
| CSI ? 1003 h/l | | Mouse Any Event | No | | |
| CSI ? 1004 h/l | | Focus Reporting | No | | |
| CSI ? 1006 h/l | | Mouse SGR Mode | No | | |
| CSI ? 1049 h/l | | Alternate Screen + Save Cursor | No | | |
| CSI ? 2004 h/l | | Bracketed Paste Mode | No | | |

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
| OSC 0 ; Pt ST | Set Icon Name and Window Title | No | | |
| OSC 1 ; Pt ST | Set Icon Name | No | | |
| OSC 2 ; Pt ST | Set Window Title | No | | |
| OSC 4 ; c ; spec ST | Set Color | No | | |
| OSC 8 ; params ; uri ST | Hyperlink | No | | |
| OSC 10 ; Pt ST | Set Foreground Color | No | | |
| OSC 11 ; Pt ST | Set Background Color | No | | |
| OSC 12 ; Pt ST | Set Cursor Color | No | | |
| OSC 52 ; c ; data ST | Clipboard | No | | Security controlled |
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
