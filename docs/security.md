# Security Documentation

This document describes the security considerations and controls implemented in the Mochi terminal emulator.

## Overview

Terminal emulators process untrusted input from remote systems and local applications. Mochi implements several security controls to protect users from malicious escape sequences and other attacks.

## Threat Model

### Trusted Components
- The Mochi terminal emulator binary
- User configuration files
- System fonts

### Untrusted Input
- All data received from the PTY (shell output, remote SSH sessions, etc.)
- Pasted text from clipboard
- Files displayed with `cat` or similar commands

### Attack Vectors
1. **Escape sequence injection**: Malicious sequences in filenames, environment variables, or command output
2. **Clipboard exfiltration**: OSC 52 sequences that read or write clipboard contents
3. **Hyperlink spoofing**: OSC 8 hyperlinks with misleading display text
4. **Resource exhaustion**: Extremely long sequences or unbounded scrollback
5. **Title bar spoofing**: Misleading window titles via OSC 0/2

## Security Controls

### OSC 52 Clipboard Operations

OSC 52 allows applications to read and write the system clipboard. This is a powerful feature that can be abused for data exfiltration.

**Controls implemented:**
1. **Disabled by default**: OSC 52 clipboard operations are disabled unless explicitly enabled by the user
2. **Size limits**: Maximum payload size of 100KB to prevent memory exhaustion
3. **Query operations blocked**: Reading clipboard contents via OSC 52 is not supported
4. **Base64 validation**: Payloads must be valid base64-encoded UTF-8 text

**Configuration:**
```rust
// In application code
performer.set_osc52_enabled(true);  // Enable OSC 52 (default: false)
```

**Recommendation:** Only enable OSC 52 in trusted environments where you control all applications that may send escape sequences.

### OSC 8 Hyperlinks

OSC 8 allows applications to embed clickable hyperlinks in terminal output.

**Controls implemented:**
1. **Click-to-open**: Hyperlinks are not automatically opened; user must explicitly click
2. **URL storage**: URLs are stored internally and can be inspected before opening
3. **No auto-execution**: Links are opened via the system's default URL handler, not executed directly

**Recommendations:**
- Hover over links to verify the URL before clicking
- Be cautious of links in untrusted output (e.g., from remote SSH sessions)
- Consider the source of the link before clicking

### Window Title (OSC 0/2)

OSC 0 and OSC 2 allow applications to set the window title.

**Controls implemented:**
1. **Length limits**: Titles are truncated to prevent extremely long strings
2. **Character filtering**: Control characters are stripped from titles

**Recommendations:**
- Be aware that window titles can be set by any application
- Do not rely on window titles for security-critical information

### Bracketed Paste Mode

Bracketed paste mode (CSI ?2004h) wraps pasted text with escape sequences so applications can distinguish pasted text from typed input.

**Controls implemented:**
1. **Proper bracketing**: Pasted text is wrapped with `\x1b[200~` and `\x1b[201~` when enabled
2. **Mode tracking**: The terminal correctly tracks when bracketed paste is enabled/disabled

**Recommendations:**
- Use applications that support bracketed paste mode for sensitive operations
- Be cautious when pasting into applications that don't support bracketed paste

### Resource Limits

**Controls implemented:**
1. **Scrollback limit**: Default 10,000 lines maximum to prevent unbounded memory growth
2. **Sequence length limits**: Parser limits on parameter counts and payload sizes
3. **UTF-8 validation**: Invalid UTF-8 sequences are handled with replacement characters

### Parser Robustness

The escape sequence parser is designed to be robust against malformed input:

1. **No panics**: The parser never panics on any input
2. **Bounded state**: Parser state is bounded and cannot grow unboundedly
3. **Chunk boundary handling**: Parser correctly handles sequences split across multiple reads
4. **Invalid sequence recovery**: Parser recovers gracefully from invalid sequences

**Fuzzing:**
The parser is fuzzed with random input to ensure robustness. See `terminal/mochi-parser/fuzz/` for fuzzing targets.

## Best Practices for Users

1. **Review untrusted files**: Use `less` or `cat -v` to safely view files that may contain escape sequences
2. **Disable OSC 52**: Keep OSC 52 disabled unless specifically needed
3. **Verify hyperlinks**: Always verify URLs before clicking
4. **Use trusted shells**: Run untrusted commands in sandboxed environments
5. **Monitor window titles**: Be aware that titles can be changed by applications

## Reporting Security Issues

If you discover a security vulnerability in Mochi, please report it responsibly by:

1. Opening a private security advisory on GitHub
2. Emailing the maintainers directly
3. Not disclosing publicly until a fix is available

## References

- [Terminal Security Best Practices](https://invisible-island.net/xterm/xterm.faq.html#security)
- [OSC 52 Security Considerations](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Operating-System-Commands)
- [ECMA-48 Control Functions](https://ecma-international.org/wp-content/uploads/ECMA-48_5th_edition_june_1991.pdf)
