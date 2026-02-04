# Security Considerations

This document describes security considerations for the Mochi terminal emulator.

## Overview

Terminal emulators process untrusted input from remote servers and local applications. Malicious escape sequences could potentially:
- Exfiltrate data via clipboard
- Execute commands via paste
- Cause denial of service
- Confuse users with misleading output

## OSC 52 Clipboard

OSC 52 allows applications to set the system clipboard via escape sequences. This is useful for copying text from remote SSH sessions, but can be abused for data exfiltration.

### Security Controls

1. **Disabled by default**: OSC 52 clipboard write is disabled by default. Users must explicitly enable it in configuration.

2. **Size limits**: Maximum payload size is limited to prevent memory exhaustion.

3. **Base64 validation**: Payload must be valid base64.

### Configuration

```toml
[security]
osc52_clipboard = false  # Enable with caution
osc52_max_size = 100000  # Maximum bytes
```

### Recommendations

- Only enable OSC 52 if you need it for remote clipboard access
- Be aware that any application can set your clipboard when enabled
- Consider using a dedicated clipboard manager with history

## OSC 8 Hyperlinks

OSC 8 allows applications to create clickable hyperlinks in terminal output.

### Security Controls

1. **No auto-open**: Links are never automatically opened. User must explicitly click.

2. **URL validation**: Only http, https, and file URLs are allowed.

3. **Visual indication**: Hyperlinks are visually distinct (underlined).

### Recommendations

- Hover over links to see the actual URL before clicking
- Be cautious of links from untrusted sources

## Bracketed Paste

Bracketed paste mode wraps pasted text in escape sequences so applications can distinguish pasted text from typed text. This prevents "paste injection" attacks where malicious text includes newlines to execute commands.

### How It Works

When bracketed paste is enabled (CSI ? 2004 h):
- Pasted text is wrapped: `\x1b[200~` ... text ... `\x1b[201~`
- Applications can detect and handle pasted text specially

### Recommendations

- Use shells and editors that support bracketed paste (bash 4.4+, zsh, vim, etc.)
- Be cautious when pasting into applications that don't support it

## Title Setting

Applications can set the window title via OSC 0/2. Malicious titles could:
- Impersonate other applications
- Display misleading information
- Cause visual disruption through rapid updates

### Security Controls

1. **Length limits**: Title length is bounded.
2. **Character filtering**: Control characters are stripped.
3. **Update throttling**: Title updates are throttled to a minimum interval of 100ms to prevent rapid title change attacks that could cause visual flickering or resource exhaustion.

## Denial of Service

### Memory Exhaustion

- Scrollback buffer is bounded (default 10,000 lines)
- Parser buffers have fixed maximum sizes
- OSC string length is limited

### CPU Exhaustion

- Parser is designed to make progress on every byte
- No unbounded loops in sequence handling
- Malformed sequences are handled gracefully

### Rendering

- Glyph cache could grow unbounded with many unique characters
- Consider adding LRU eviction for production use

## Input Validation

### UTF-8

- Invalid UTF-8 sequences are replaced with U+FFFD (replacement character)
- Overlong encodings are rejected
- Surrogate pairs are rejected

### Escape Sequences

- Unknown sequences are ignored (not executed)
- Malformed sequences are handled gracefully
- Parser state machine prevents infinite loops

## Recommendations for Users

1. **Don't run untrusted code**: Terminal emulators execute whatever the shell runs.

2. **Be cautious with SSH**: Remote servers can send arbitrary escape sequences.

3. **Review before pasting**: Especially from untrusted sources.

4. **Keep software updated**: Security fixes are released periodically.

5. **Use bracketed paste**: Enable in your shell for paste protection.

## Reporting Security Issues

Please report security issues to the maintainers privately before public disclosure.
