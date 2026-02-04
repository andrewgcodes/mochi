# VT100/ANSI Parser Research

This document summarizes research on implementing a correct VT/ANSI escape sequence parser.

## Primary Reference

The authoritative reference for parser implementation is:
**"A parser for DEC's ANSI-compatible video terminals"** by Paul Flo Williams
https://vt100.net/emu/dec_ansi_parser

This document provides a complete state machine that handles all input bytes in all states.

## Parser State Machine

### States

The parser has the following states:

1. **Ground** - Normal character processing
2. **Escape** - After receiving ESC (0x1B)
3. **Escape Intermediate** - ESC followed by intermediate byte (0x20-0x2F)
4. **CSI Entry** - After CSI (ESC [ or 0x9B)
5. **CSI Param** - Collecting CSI numeric parameters
6. **CSI Intermediate** - CSI with intermediate bytes
7. **CSI Ignore** - Invalid CSI, consuming until final byte
8. **DCS Entry** - After DCS (ESC P or 0x90)
9. **DCS Param** - Collecting DCS parameters
10. **DCS Intermediate** - DCS with intermediate bytes
11. **DCS Passthrough** - Receiving DCS data
12. **DCS Ignore** - Invalid DCS
13. **OSC String** - Collecting OSC payload
14. **SOS/PM/APC String** - Consuming SOS/PM/APC (ignored)

### Character Classifications

- **C0 controls**: 0x00-0x1F (except ESC at 0x1B)
- **C1 controls**: 0x80-0x9F (8-bit mode only)
- **Intermediate bytes**: 0x20-0x2F (space through /)
- **Parameter bytes**: 0x30-0x3F (0-9, :, ;, <, =, >, ?)
- **Uppercase letters**: 0x40-0x5F (final bytes for some sequences)
- **Lowercase letters**: 0x60-0x7E (final bytes, printable)
- **DEL**: 0x7F (ignored in most states)

### Key Transitions

```
Ground:
  0x00-0x17, 0x19, 0x1C-0x1F → execute (C0 control)
  0x1B → Escape
  0x20-0x7F → print (graphic character)

Escape:
  0x00-0x17, 0x19, 0x1C-0x1F → execute
  0x20-0x2F → collect, → Escape Intermediate
  0x30-0x4F, 0x51-0x57, 0x59, 0x5A, 0x5C, 0x60-0x7E → esc_dispatch, → Ground
  0x5B → → CSI Entry
  0x5D → → OSC String
  0x50 → → DCS Entry
  0x58, 0x5E, 0x5F → → SOS/PM/APC String

CSI Entry:
  0x00-0x17, 0x19, 0x1C-0x1F → execute
  0x20-0x2F → collect, → CSI Intermediate
  0x30-0x39, 0x3B → param, → CSI Param
  0x3A → → CSI Ignore
  0x3C-0x3F → collect, → CSI Param
  0x40-0x7E → csi_dispatch, → Ground

CSI Param:
  0x00-0x17, 0x19, 0x1C-0x1F → execute
  0x20-0x2F → collect, → CSI Intermediate
  0x30-0x39, 0x3B → param
  0x3A, 0x3C-0x3F → → CSI Ignore
  0x40-0x7E → csi_dispatch, → Ground

CSI Intermediate:
  0x00-0x17, 0x19, 0x1C-0x1F → execute
  0x20-0x2F → collect
  0x30-0x3F → → CSI Ignore
  0x40-0x7E → csi_dispatch, → Ground

OSC String:
  0x00-0x06, 0x08-0x17, 0x19, 0x1C-0x1F → ignore
  0x07 → osc_dispatch, → Ground (BEL terminates)
  0x1B → → Escape (for ST = ESC \)
  0x20-0x7F → osc_put
```

## CSI Parameter Parsing

CSI sequences have the form: `CSI [private] params [intermediate] final`

- **Private marker**: `?`, `>`, `<`, `=` (0x3C-0x3F) at start
- **Parameters**: Decimal numbers separated by `;`
- **Subparameters**: Some sequences use `:` as separator (e.g., SGR)
- **Intermediate**: 0x20-0x2F bytes before final
- **Final byte**: 0x40-0x7E determines the command

Example: `CSI ? 1 ; 2 h`
- Private: `?`
- Params: [1, 2]
- Final: `h` (SM - Set Mode)

### Parameter Defaults

Most parameters default to 0 or 1 depending on the command:
- Cursor movement (CUU, CUD, etc.): default 1
- Cursor position (CUP): default 1;1
- Erase (ED, EL): default 0

Empty parameters should be treated as default, not 0:
- `CSI H` = `CSI 1;1 H` (move to 1,1)
- `CSI ;5 H` = `CSI 1;5 H` (move to row 1, col 5)

## OSC Parsing

OSC sequences: `OSC Ps ; Pt ST` or `OSC Ps ; Pt BEL`

- **Ps**: Numeric command
- **Pt**: Text payload
- **ST**: String Terminator (ESC \)
- **BEL**: Alternative terminator (0x07)

The payload is typically split on `;` for multi-part commands.

## UTF-8 Considerations

In UTF-8 mode:
- C1 controls (0x80-0x9F) are NOT recognized as controls
- They are treated as invalid UTF-8 lead bytes
- Only 7-bit escape sequences work
- UTF-8 continuation bytes (0x80-0xBF) must be handled correctly

Invalid UTF-8 handling:
- Replace invalid sequences with U+FFFD
- Don't process decoded C1 controls

## Implementation Notes

### Streaming

The parser must handle arbitrary chunk boundaries:
- A sequence may be split across multiple `feed()` calls
- State must be preserved between calls
- No assumptions about input alignment

### Actions

The parser produces actions, not side effects:
- `Print(char)` - Display a character
- `Execute(byte)` - Handle C0 control
- `CsiDispatch{...}` - Handle CSI sequence
- `OscDispatch{...}` - Handle OSC sequence
- `EscDispatch{...}` - Handle ESC sequence

### Error Handling

Invalid sequences should be:
- Consumed (don't leave partial state)
- Ignored (don't crash or corrupt state)
- Optionally logged for debugging

## References

1. VT100.net Parser: https://vt100.net/emu/dec_ansi_parser
2. ECMA-48: https://ecma-international.org/publications-and-standards/standards/ecma-48/
3. XTerm Control Sequences: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
4. VT220 Reference Manual: https://vt100.net/docs/vt220-rm/
