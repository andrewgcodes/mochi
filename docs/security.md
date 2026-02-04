# Security Considerations

This document describes security considerations for the Mochi terminal emulator.

## Threat Model

Terminal emulators process untrusted input from:
1. Remote servers (via SSH)
2. Downloaded scripts
3. Malicious files (e.g., `cat malicious.txt`)
4. Compromised applications

Attackers may attempt to:
- Exfiltrate data via clipboard (OSC 52)
- Execute commands via paste injection
- Cause denial of service
- Exploit memory safety bugs

## Security Controls

### OSC 52 Clipboard Access

OSC 52 allows applications to read and write the system clipboard. This is a significant security risk as malicious content could:
- Read sensitive data from clipboard (passwords, tokens)
- Write malicious commands to clipboard for paste injection

**Controls:**
- **Disabled by default**: OSC 52 is disabled unless explicitly enabled in config
- **Separate read/write controls**: Can enable write-only (safer) or read+write
- **Size limits**: Maximum payload size enforced (default: 100KB)
- **Visual indicator**: UI shows when clipboard was modified via OSC 52
- **Logging**: All OSC 52 operations are logged

Configuration:
```toml
[security]
osc52_read = false   # Allow reading clipboard
osc52_write = false  # Allow writing clipboard
osc52_max_bytes = 102400  # Maximum payload size
```

### OSC 8 Hyperlinks

Hyperlinks can be used for phishing by displaying misleading text.

**Controls:**
- Links are not auto-opened; require explicit click
- URL is shown in tooltip/status bar before clicking
- Only http/https/mailto schemes allowed by default
- File:// URLs require explicit opt-in

### Paste Injection

Malicious content may include hidden characters that execute commands when pasted.

**Controls:**
- **Bracketed paste mode**: When enabled by applications, pasted content is wrapped in escape sequences that applications can detect
- **Paste sanitization option**: Strip control characters from pasted content
- **Paste confirmation**: Optional confirmation dialog for large pastes

### Title/Icon Setting

Window titles can be set via escape sequences, potentially for social engineering.

**Controls:**
- Title length limited (default: 256 characters)
- Control characters stripped from titles
- Optional: Prefix titles with indicator that they were set by application

### Denial of Service

Malicious input may attempt to:
- Consume unlimited memory (scrollback, long lines)
- Cause CPU exhaustion (complex escape sequences)
- Hang the terminal (infinite loops in parsing)

**Controls:**
- **Scrollback limit**: Configurable maximum (default: 10,000 lines)
- **Line length limit**: Maximum characters per line (default: 10,000)
- **Escape sequence timeout**: Incomplete sequences timeout after 1 second
- **Parameter limits**: CSI parameters limited to reasonable values
- **Fuzzing**: Parser is fuzz-tested to ensure no hangs or crashes

### Memory Safety

As a Rust application, Mochi benefits from:
- No buffer overflows in safe code
- No use-after-free bugs
- No null pointer dereferences
- Bounds checking on all array access

**Unsafe code policy:**
- Minimize use of `unsafe`
- All `unsafe` blocks must have safety comments
- `unsafe` code is reviewed and audited
- Fuzzing covers code paths through `unsafe`

## Input Validation

### UTF-8 Handling

- Invalid UTF-8 sequences are replaced with U+FFFD
- Overlong encodings are rejected
- Surrogate pairs are rejected
- No processing of decoded C1 controls in UTF-8 mode

### Escape Sequence Parsing

- Unknown sequences are consumed and ignored
- Malformed sequences do not cause crashes
- Parameter overflow is handled gracefully
- Intermediate bytes are validated

## Logging and Auditing

Security-relevant events are logged:
- OSC 52 clipboard operations
- Hyperlink activations
- Large paste operations
- Unusual escape sequences

Log levels:
- `WARN`: Potentially dangerous operations
- `INFO`: Normal security-relevant events
- `DEBUG`: Detailed parsing information

## Recommendations for Users

1. **Don't enable OSC 52** unless you trust all applications you run
2. **Use bracketed paste** when available (most modern shells support it)
3. **Review URLs** before clicking hyperlinks
4. **Limit scrollback** if running untrusted applications
5. **Keep terminal updated** for security fixes

## Vulnerability Reporting

Please report security vulnerabilities to [security contact TBD].

## References

- [Terminal Security Issues](https://invisible-island.net/xterm/xterm.faq.html#security)
- [OSC 52 Security](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Operating-System-Commands)
- [Paste Injection Attacks](https://thejh.net/misc/website-hierarchical-clipboard-copy)
