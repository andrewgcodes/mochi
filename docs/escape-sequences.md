# Escape Sequence Coverage Matrix

This document lists all escape sequences supported by Mochi Terminal.

## Legend

- **Yes**: Fully implemented and tested
- **Partial**: Implemented but with limitations
- **No**: Not implemented
- **N/A**: Not applicable or intentionally unsupported

## C0 Control Characters (0x00-0x1F)

| Code | Name | Hex | Implemented | Notes |
|------|------|-----|-------------|-------|
| NUL | Null | 0x00 | Yes | Ignored |
| BEL | Bell | 0x07 | Yes | Triggers bell (visual/audio) |
| BS | Backspace | 0x08 | Yes | Moves cursor left, does not delete |
| HT | Horizontal Tab | 0x09 | Yes | Advances to next tab stop |
| LF | Line Feed | 0x0A | Yes | Moves cursor down, scrolls if needed |
| VT | Vertical Tab | 0x0B | Yes | Treated as LF |
| FF | Form Feed | 0x0C | Yes | Treated as LF |
| CR | Carriage Return | 0x0D | Yes | Moves cursor to column 0 |
| SO | Shift Out | 0x0E | Yes | Switches to G1 charset |
| SI | Shift In | 0x0F | Yes | Switches to G0 charset |
| ESC | Escape | 0x1B | Yes | Starts escape sequence |

## ESC Sequences

| Sequence | Name | Implemented | Notes |
|----------|------|-------------|-------|
| ESC 7 | DECSC - Save Cursor | Yes | Saves cursor position and attributes |
| ESC 8 | DECRC - Restore Cursor | Yes | Restores saved cursor |
| ESC D | IND - Index | Yes | Move cursor down, scroll if at bottom |
| ESC E | NEL - Next Line | Yes | Move to beginning of next line |
| ESC H | HTS - Horizontal Tab Set | Yes | Set tab stop at current column |
| ESC M | RI - Reverse Index | Yes | Move cursor up, scroll if at top |
| ESC c | RIS - Full Reset | Yes | Reset terminal to initial state |
| ESC = | DECKPAM - Keypad Application | Yes | Enable application keypad mode |
| ESC > | DECKPNM - Keypad Numeric | Yes | Enable numeric keypad mode |
| ESC ( B | SCS - Select ASCII | Yes | Select ASCII charset for G0 |
| ESC ( 0 | SCS - Select Graphics | Partial | Line drawing characters |
| ESC ) B | SCS - Select ASCII G1 | Yes | Select ASCII charset for G1 |
| ESC ) 0 | SCS - Select Graphics G1 | Partial | Line drawing characters |
| ESC # 8 | DECALN - Alignment Test | Yes | Fill screen with 'E' |

## CSI Sequences (ESC [ ...)

### Cursor Movement

| Sequence | Name | Implemented | Notes |
|----------|------|-------------|-------|
| CSI n A | CUU - Cursor Up | Yes | Move cursor up n rows |
| CSI n B | CUD - Cursor Down | Yes | Move cursor down n rows |
| CSI n C | CUF - Cursor Forward | Yes | Move cursor right n columns |
| CSI n D | CUB - Cursor Back | Yes | Move cursor left n columns |
| CSI n E | CNL - Cursor Next Line | Yes | Move to beginning of line n down |
| CSI n F | CPL - Cursor Previous Line | Yes | Move to beginning of line n up |
| CSI n G | CHA - Cursor Horizontal Absolute | Yes | Move to column n |
| CSI n ; m H | CUP - Cursor Position | Yes | Move to row n, column m |
| CSI n ; m f | HVP - Horizontal Vertical Position | Yes | Same as CUP |
| CSI n d | VPA - Vertical Position Absolute | Yes | Move to row n |
| CSI s | SCP - Save Cursor Position | Yes | Save cursor (ANSI.SYS style) |
| CSI u | RCP - Restore Cursor Position | Yes | Restore cursor (ANSI.SYS style) |

### Erase Operations

| Sequence | Name | Implemented | Notes |
|----------|------|-------------|-------|
| CSI n J | ED - Erase in Display | Yes | 0=below, 1=above, 2=all, 3=scrollback |
| CSI n K | EL - Erase in Line | Yes | 0=right, 1=left, 2=all |
| CSI n X | ECH - Erase Characters | Yes | Erase n characters |

### Insert/Delete Operations

| Sequence | Name | Implemented | Notes |
|----------|------|-------------|-------|
| CSI n @ | ICH - Insert Characters | Yes | Insert n blank characters |
| CSI n P | DCH - Delete Characters | Yes | Delete n characters |
| CSI n L | IL - Insert Lines | Yes | Insert n blank lines |
| CSI n M | DL - Delete Lines | Yes | Delete n lines |

### Scroll Operations

| Sequence | Name | Implemented | Notes |
|----------|------|-------------|-------|
| CSI n S | SU - Scroll Up | Yes | Scroll up n lines |
| CSI n T | SD - Scroll Down | Yes | Scroll down n lines |
| CSI t ; b r | DECSTBM - Set Scroll Region | Yes | Set top and bottom margins |

### Tab Operations

| Sequence | Name | Implemented | Notes |
|----------|------|-------------|-------|
| CSI n g | TBC - Tab Clear | Yes | 0=current, 3=all |
| CSI n I | CHT - Cursor Forward Tab | Yes | Move forward n tab stops |
| CSI n Z | CBT - Cursor Backward Tab | Yes | Move backward n tab stops |

### Mode Operations

| Sequence | Name | Implemented | Notes |
|----------|------|-------------|-------|
| CSI n h | SM - Set Mode | Yes | Set ANSI mode |
| CSI n l | RM - Reset Mode | Yes | Reset ANSI mode |
| CSI ? n h | DECSET - DEC Private Set | Yes | Set DEC private mode |
| CSI ? n l | DECRST - DEC Private Reset | Yes | Reset DEC private mode |

### Device Status

| Sequence | Name | Implemented | Notes |
|----------|------|-------------|-------|
| CSI n n | DSR - Device Status Report | Yes | 5=status, 6=cursor position |
| CSI ? n n | DECDSR - DEC Device Status | Partial | Limited support |
| CSI c | DA - Device Attributes | Yes | Reports VT100 |
| CSI > c | DA2 - Secondary DA | Yes | Reports VT220 |

### SGR (Select Graphic Rendition)

| Code | Name | Implemented | Notes |
|------|------|-------------|-------|
| 0 | Reset | Yes | Reset all attributes |
| 1 | Bold | Yes | Bold/bright |
| 2 | Faint | Yes | Dim/faint |
| 3 | Italic | Yes | Italic |
| 4 | Underline | Yes | Underline |
| 5 | Slow Blink | Yes | Blinking (treated same as 6) |
| 6 | Rapid Blink | Yes | Blinking |
| 7 | Inverse | Yes | Reverse video |
| 8 | Hidden | Yes | Invisible text |
| 9 | Strikethrough | Yes | Crossed out |
| 21 | Double Underline | Partial | Treated as underline |
| 22 | Normal Intensity | Yes | Not bold, not faint |
| 23 | Not Italic | Yes | Disable italic |
| 24 | Not Underlined | Yes | Disable underline |
| 25 | Not Blinking | Yes | Disable blink |
| 27 | Not Inverse | Yes | Disable inverse |
| 28 | Not Hidden | Yes | Disable hidden |
| 29 | Not Strikethrough | Yes | Disable strikethrough |
| 30-37 | Foreground Color | Yes | Standard colors |
| 38;5;n | Foreground 256 | Yes | 256-color palette |
| 38;2;r;g;b | Foreground RGB | Yes | True color |
| 39 | Default Foreground | Yes | Reset to default |
| 40-47 | Background Color | Yes | Standard colors |
| 48;5;n | Background 256 | Yes | 256-color palette |
| 48;2;r;g;b | Background RGB | Yes | True color |
| 49 | Default Background | Yes | Reset to default |
| 90-97 | Bright Foreground | Yes | Bright colors |
| 100-107 | Bright Background | Yes | Bright colors |

## DEC Private Modes (CSI ? n h/l)

| Mode | Name | Implemented | Notes |
|------|------|-------------|-------|
| 1 | DECCKM - Cursor Keys | Yes | Application cursor keys |
| 3 | DECCOLM - 132 Column | Partial | Clears screen, doesn't resize |
| 5 | DECSCNM - Reverse Video | Yes | Screen-wide reverse |
| 6 | DECOM - Origin Mode | Yes | Origin relative to margins |
| 7 | DECAWM - Auto Wrap | Yes | Auto wrap at end of line |
| 12 | Cursor Blink | Yes | Start/stop cursor blinking |
| 25 | DECTCEM - Cursor Visible | Yes | Show/hide cursor |
| 47 | Alternate Screen | Yes | Switch to alternate buffer |
| 66 | DECNKM - Keypad Mode | Yes | Application keypad |
| 1000 | X10 Mouse | Yes | Send mouse press only |
| 1002 | Button Event Mouse | Yes | Send press/release/motion |
| 1003 | Any Event Mouse | Yes | Send all mouse events |
| 1004 | Focus Events | Yes | Send focus in/out |
| 1005 | UTF-8 Mouse | Yes | UTF-8 encoded coordinates |
| 1006 | SGR Mouse | Yes | SGR encoded mouse |
| 1015 | URXVT Mouse | Yes | URXVT encoded mouse |
| 1047 | Alternate Screen | Yes | Alternate buffer (no clear) |
| 1048 | Save Cursor | Yes | Save cursor position |
| 1049 | Alternate Screen + Cursor | Yes | Save cursor, switch, clear |
| 2004 | Bracketed Paste | Yes | Wrap paste with markers |

## ANSI Modes (CSI n h/l)

| Mode | Name | Implemented | Notes |
|------|------|-------------|-------|
| 4 | IRM - Insert Mode | Yes | Insert vs replace |
| 20 | LNM - Line Feed Mode | Yes | LF implies CR |

## OSC Sequences (ESC ] ...)

| Code | Name | Implemented | Notes |
|------|------|-------------|-------|
| 0 | Set Icon Name and Title | Yes | Sets window title |
| 1 | Set Icon Name | Yes | Sets icon name |
| 2 | Set Title | Yes | Sets window title |
| 4 | Set Color | Partial | Set palette color |
| 8 | Hyperlink | Yes | OSC 8 hyperlinks |
| 10 | Set Foreground | Partial | Query/set foreground |
| 11 | Set Background | Partial | Query/set background |
| 12 | Set Cursor Color | Partial | Query/set cursor color |
| 52 | Clipboard | Yes | Get/set clipboard (with security) |
| 104 | Reset Color | Yes | Reset palette color |
| 110 | Reset Foreground | Yes | Reset foreground |
| 111 | Reset Background | Yes | Reset background |
| 112 | Reset Cursor Color | Yes | Reset cursor color |

### OSC 52 Security

OSC 52 clipboard access is controlled by security settings:
- Disabled by default (must be explicitly enabled)
- Maximum payload size enforced
- Only primary and clipboard selections supported
- See [security.md](security.md) for details

## DCS Sequences (ESC P ...)

| Sequence | Name | Implemented | Notes |
|----------|------|-------------|-------|
| DCS + q | Request Termcap | No | Not implemented |
| DCS $ q | DECRQSS | No | Not implemented |
| DCS q (Sixel) | Sixel Graphics | No | Not implemented |

## Not Implemented (Intentional)

The following are intentionally not implemented:

- **Sixel Graphics**: Complex bitmap graphics protocol
- **ReGIS Graphics**: DEC graphics protocol
- **Tektronix 4014**: Vector graphics mode
- **Printer Control**: Hardcopy/printer sequences
- **Soft Fonts**: DRCS (downloadable character sets)

## Testing

Each implemented sequence has:
1. Unit tests in the relevant module
2. Golden tests with expected output
3. Integration tests where applicable

Run tests with:
```bash
cargo test
```

## References

- [Xterm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)
- [ECMA-48](https://ecma-international.org/publications-and-standards/standards/ecma-48/)
- [VT100.net](https://vt100.net/)
- [vttest](https://invisible-island.net/vttest/)
