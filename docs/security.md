# Security Considerations

This document describes security considerations for the Mochi terminal emulator.

## Overview

Terminal emulators process untrusted input from remote systems and local applications. This input can include escape sequences that attempt to:

1. Exfiltrate data (clipboard, screen contents)
2. Execute commands or open URLs
3. Cause denial of service
4. Confuse or mislead the user

Mochi implements several security controls to mitigate these risks.

## OSC 52 - Clipboard Access

### Risk

OSC 52 allows applications to read and write the system clipboard. A malicious application or remote server could:

- Read sensitive data from the clipboard (passwords, tokens)
- Write malicious content to the clipboard for later paste

### Mitigations

1. **Disabled by Default**: OSC 52 is disabled by default and must be explicitly enabled in configuration.

2. **Write-Only Mode**: When enabled, OSC 52 can be configured to only allow writes (setting clipboard) but not reads (querying clipboard).

3. **Size Limits**: Clipboard payloads are limited to prevent memory exhaustion:
   - Maximum base64 payload: 100KB
   - Maximum decoded content: 75KB

4. **Selection Restrictions**: Only `c` (clipboard) and `p` (primary) selections are supported. Other selections are ignored.

5. **Visual Indicator**: When clipboard is modified via OSC 52, a visual indicator is shown (configurable).

### Configuration

```toml
[security]
# Enable OSC 52 clipboard access
osc52_enabled = false

# Allow reading clipboard (query)
osc52_read = false

# Allow writing clipboard (set)
osc52_write = true

# Maximum payload size in bytes
osc52_max_size = 102400

# Show indicator when clipboard is modified
osc52_indicator = true
```

## OSC 8 - Hyperlinks

### Risk

OSC 8 allows embedding clickable hyperlinks in terminal output. Risks include:

- Phishing: Links that display one URL but navigate to another
- Malicious URLs: javascript:, file:, or other dangerous schemes
- URL confusion: Using lookalike characters or long URLs

### Mitigations

1. **Click Required**: Links are never automatically opened. User must explicitly click.

2. **URL Display**: On hover, the actual URL is displayed in a tooltip or status bar.

3. **Scheme Whitelist**: Only safe URL schemes are allowed:
   - `http://`
   - `https://`
   - `mailto:`
   - `file://` (local files, with confirmation)

4. **URL Validation**: URLs are validated before opening:
   - Maximum length enforced
   - Invalid characters rejected
   - Punycode domains highlighted

5. **Confirmation Dialog**: Optionally require confirmation before opening links.

### Configuration

```toml
[security]
# Enable OSC 8 hyperlinks
hyperlinks_enabled = true

# Allowed URL schemes
hyperlink_schemes = ["http", "https", "mailto"]

# Require confirmation before opening
hyperlink_confirm = false

# Maximum URL length
hyperlink_max_length = 2048
```

## Resource Limits

### Scrollback Buffer

Unbounded scrollback can lead to memory exhaustion.

**Mitigation**: Scrollback is bounded by configuration (default: 10,000 lines).

```toml
[terminal]
scrollback_lines = 10000
```

### Escape Sequence Length

Malformed or malicious sequences could be extremely long.

**Mitigation**: Maximum sequence lengths are enforced:
- OSC payload: 100KB
- DCS payload: 1MB
- CSI parameters: 16 parameters, 65535 max value

### Screen Size

Extremely large screen dimensions could cause memory issues.

**Mitigation**: Maximum dimensions enforced:
- Maximum columns: 1000
- Maximum rows: 500

### Title Length

Window titles are bounded to prevent UI issues.

**Mitigation**: Maximum title length: 4096 characters.

## Input Sanitization

### Bracketed Paste

When bracketed paste mode is enabled (mode 2004), pasted content is wrapped with escape sequences that allow applications to distinguish paste from typed input.

**Security benefit**: Prevents paste-based attacks where malicious content includes escape sequences or newlines that execute commands.

**Implementation**: Mochi always sends bracketed paste markers when the mode is enabled, even if the pasted content contains the marker sequences (which are escaped).

### Control Character Filtering

Certain control characters in pasted content could be dangerous:

- `\x1b` (ESC): Could inject escape sequences
- `\x03` (Ctrl-C): Could interrupt processes
- `\x04` (Ctrl-D): Could send EOF

**Mitigation**: When bracketed paste is NOT enabled, control characters in pasted content are optionally filtered or escaped (configurable).

```toml
[security]
# Filter control characters in paste when bracketed paste is disabled
paste_filter_controls = true
```

## Title and Icon Name

### Risk

Applications can set the window title via OSC 0/1/2. Malicious titles could:

- Impersonate other applications
- Include misleading information
- Contain extremely long strings

### Mitigations

1. **Length Limit**: Titles are truncated to 4096 characters
2. **Control Character Stripping**: Control characters are removed from titles
3. **Optional Prefix**: Titles can be prefixed with a fixed string to prevent impersonation

## Denial of Service

### Rapid Output

Applications can output data faster than the terminal can render.

**Mitigation**: 
- Rendering is decoupled from parsing
- Frame rate is capped
- Parsing continues even when rendering is behind

### Infinite Loops

Malformed sequences could potentially cause parser loops.

**Mitigation**:
- Parser state machine has no cycles that don't consume input
- Maximum iterations per byte enforced
- Timeout on sequence completion

### Memory Exhaustion

Various attacks could attempt to exhaust memory.

**Mitigations**:
- Bounded scrollback
- Bounded sequence buffers
- Bounded hyperlink storage
- Periodic garbage collection of unused resources

## Logging and Auditing

### Sensitive Data

Terminal output may contain sensitive data (passwords, tokens).

**Policy**:
- Debug logging does not include terminal content by default
- Sequence logging can be enabled but excludes OSC 52 payloads
- Crash dumps do not include screen contents

### Configuration

```toml
[logging]
# Log parsed sequences (for debugging)
log_sequences = false

# Redact sensitive sequences in logs
redact_sensitive = true
```

## Recommendations

### For Users

1. **Don't enable OSC 52 read** unless you trust all applications
2. **Review hyperlinks** before clicking, especially from remote sessions
3. **Use bracketed paste** when available (most modern shells support it)
4. **Limit scrollback** if memory is a concern

### For Developers

1. **Validate all input** from the PTY
2. **Bound all buffers** and data structures
3. **Test with malicious input** (fuzzing)
4. **Document security decisions** clearly

## Reporting Security Issues

If you discover a security vulnerability in Mochi Terminal, please report it responsibly:

1. Do not open a public issue
2. Email security concerns to the maintainers
3. Allow time for a fix before public disclosure

## References

- [Terminal Security](https://invisible-island.net/xterm/xterm.faq.html#security)
- [OSC 52 Security](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Operating-System-Commands)
- [Bracketed Paste Mode](https://cirw.in/blog/bracketed-paste)
