# Security Considerations

This document describes security considerations for Mochi Terminal.

## Overview

Terminal emulators process untrusted input from:
- Child processes (shells, applications)
- Remote systems (via SSH)
- Pasted content

This input can contain escape sequences that, if not handled carefully, could:
- Exfiltrate data (clipboard, screen contents)
- Execute commands
- Cause denial of service
- Mislead users

## Threat Model

### Trusted
- The terminal emulator code itself
- Configuration files (user-controlled)
- Font files (system fonts)

### Untrusted
- All data from the PTY (child process output)
- Pasted content
- Any data that could be influenced by remote systems

## Security Controls

### OSC 52 Clipboard Access

**Risk**: A malicious program could read or write the system clipboard without user consent.

**Mitigation**:
- OSC 52 is **disabled by default**
- When enabled, payload size is limited (default 100KB)
- Clear logging when clipboard is accessed
- Future: Visual indicator when clipboard is modified

**Configuration**:
```json
{
  "osc52_enabled": false,
  "osc52_max_size": 100000
}
```

**Recommendation**: Only enable OSC 52 if you specifically need it (e.g., for remote clipboard sync over SSH).

### OSC 8 Hyperlinks

**Risk**: Malicious hyperlinks could:
- Phish users with misleading display text
- Link to dangerous URLs

**Mitigation**:
- Hyperlinks are not auto-opened
- User must explicitly click/activate
- URL is shown on hover (future)
- No JavaScript or data: URLs

### Window Title (OSC 0/1/2)

**Risk**: Malicious title could:
- Mislead users about what's running
- Contain escape sequences that affect the terminal emulator's own terminal

**Mitigation**:
- Title is sanitized (control characters removed)
- Title length is bounded
- Title is displayed in window chrome, not terminal area

### Parser Robustness

**Risk**: Malformed escape sequences could:
- Crash the terminal
- Cause infinite loops
- Exhaust memory

**Mitigation**:
- Parser has bounded buffers (64KB for OSC/DCS)
- Parser handles all invalid sequences gracefully
- No panics on malformed input
- Fuzzing tests verify robustness

### Denial of Service

**Risk**: Rapid output could:
- Overwhelm the renderer
- Exhaust memory (scrollback)
- Freeze the UI

**Mitigation**:
- Scrollback has configurable maximum (default 10,000 lines)
- Rendering is throttled (~60 FPS)
- Large outputs are processed in chunks

### Paste Attacks

**Risk**: Pasted content could contain:
- Hidden control characters
- Commands that execute immediately

**Mitigation**:
- Bracketed paste mode support
- When enabled, pasted content is wrapped in escape sequences
- Applications can detect and handle pasted content safely

**Recommendation**: Use shells/applications that support bracketed paste (bash 4.4+, zsh, fish).

## Secure Defaults

Mochi Terminal ships with secure defaults:

| Feature | Default | Reason |
|---------|---------|--------|
| OSC 52 | Disabled | Clipboard access is sensitive |
| Hyperlink auto-open | Disabled | Prevent accidental navigation |
| Max scrollback | 10,000 | Prevent memory exhaustion |
| Max OSC payload | 64KB | Prevent memory exhaustion |

## Recommendations for Users

1. **Don't enable OSC 52** unless you specifically need it
2. **Be cautious with hyperlinks** - verify URLs before clicking
3. **Use bracketed paste** - ensure your shell supports it
4. **Review window titles** - be suspicious of unexpected titles
5. **Keep terminal updated** - security fixes will be released as needed

## Reporting Security Issues

If you discover a security vulnerability:

1. **Do not** open a public issue
2. Email security concerns to the maintainers
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

## Security Testing

### Fuzzing

The parser is fuzzed with random input:
```bash
cargo fuzz run parser_fuzz
```

Invariants tested:
- No panics
- No infinite loops
- Bounded memory usage

### Manual Testing

Test cases for security-sensitive features:
- Large OSC payloads
- Deeply nested escape sequences
- Invalid UTF-8
- Rapid output
- Long lines

## References

- [Terminal Security](https://invisible-island.net/xterm/xterm.faq.html#security)
- [OSC 52 Security](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Operating-System-Commands)
- [Bracketed Paste Mode](https://cirw.in/blog/bracketed-paste)
