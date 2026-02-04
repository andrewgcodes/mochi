# Security Considerations

This document describes security considerations for Mochi Terminal and how potentially dangerous features are handled.

## Threat Model

Terminal emulators process untrusted input from:
1. Remote servers (via SSH)
2. Downloaded files (via `cat`, `less`, etc.)
3. Malicious websites (via copy-paste)
4. Other applications

Attackers may attempt to:
- Exfiltrate data (clipboard, screen contents)
- Execute commands
- Cause denial of service
- Exploit parsing vulnerabilities

## OSC 52 Clipboard Access

### Risk
OSC 52 allows applications to set the system clipboard. A malicious server could:
- Overwrite clipboard with malicious content
- Exfiltrate clipboard contents (if query is supported)

### Mitigation
- **Disabled by default**: OSC 52 is disabled unless explicitly enabled in configuration
- **No query support**: We do not support querying clipboard contents (only setting)
- **Size limit**: Maximum payload size is 64KB
- **Base64 validation**: Payload must be valid base64

### Configuration
```toml
[security]
osc52_enabled = false  # Default: disabled
osc52_max_size = 65536  # Maximum payload size in bytes
```

## OSC 8 Hyperlinks

### Risk
Hyperlinks could:
- Link to malicious URLs
- Use deceptive display text
- Trigger automatic actions

### Mitigation
- **No automatic opening**: URLs are never opened automatically
- **Click required**: User must explicitly click to open
- **Visual indication**: Hyperlinks are visually distinct (underlined)
- **URL validation**: Basic URL validation before opening

## Title Setting (OSC 0/2)

### Risk
- Excessively long titles could cause UI issues
- Special characters could cause rendering problems

### Mitigation
- **Length limit**: Titles are truncated to 4096 characters
- **Character filtering**: Control characters are stripped

## Denial of Service

### Risks
1. **Infinite loops**: Malformed sequences could cause parser hangs
2. **Memory exhaustion**: Unbounded buffers could consume all memory
3. **CPU exhaustion**: Complex rendering could freeze the UI

### Mitigations

#### Parser Bounds
- Maximum CSI parameters: 32
- Maximum OSC string length: 64KB
- Maximum DCS/APC/PM/SOS string length: 64KB
- Parser state machine has no infinite loops

#### Memory Bounds
- Scrollback buffer: Configurable maximum (default 10,000 lines)
- Glyph cache: Could be bounded with LRU (future improvement)

#### Rendering
- Damage tracking limits redraw scope
- Frame rate limiting prevents CPU exhaustion

## UTF-8 Handling

### Risk
Invalid UTF-8 sequences could:
- Cause crashes
- Lead to buffer overflows
- Enable encoding attacks

### Mitigation
- **Replacement character**: Invalid sequences produce U+FFFD
- **No panics**: Parser never panics on invalid input
- **Bounded buffers**: All buffers have maximum sizes

## Escape Sequence Injection

### Risk
If terminal output is logged or displayed elsewhere, escape sequences could:
- Manipulate log viewers
- Hide malicious content
- Inject commands into other terminals

### Mitigation
- **Not our responsibility**: This is a concern for log viewers, not the terminal
- **Bracketed paste**: Prevents paste injection into the shell

## Bracketed Paste Mode

### Purpose
Prevents pasted text from being interpreted as commands by the shell.

### Implementation
When enabled (CSI ? 2004 h):
- Pasted text is wrapped with `\e[200~` ... `\e[201~`
- Shell can distinguish paste from typed input

### Security Note
- Only effective if the shell supports bracketed paste
- Modern shells (bash 4.4+, zsh, fish) support this

## Input Validation

### Keyboard Input
- All keyboard input is encoded according to xterm conventions
- No raw passthrough of potentially dangerous sequences

### Mouse Input
- Mouse coordinates are bounded to screen dimensions
- Mouse encoding follows standard protocols

## Process Isolation

### PTY Security
- Child process runs in its own session
- Controlling terminal is properly set
- No privilege escalation

### Environment
- TERM is set to a safe value (xterm-256color)
- No sensitive environment variables are leaked

## Recommendations for Users

1. **Don't enable OSC 52** unless you trust all remote servers
2. **Be cautious with hyperlinks** from untrusted sources
3. **Use bracketed paste** in your shell
4. **Keep scrollback reasonable** to limit memory usage
5. **Don't cat untrusted files** without filtering

## Reporting Security Issues

If you discover a security vulnerability, please report it to the maintainers privately before public disclosure.

## References

- [Terminal Security](https://invisible-island.net/xterm/xterm.faq.html#security)
- [OSC 52 Security](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Operating-System-Commands)
- [Bracketed Paste](https://cirw.in/blog/bracketed-paste)
