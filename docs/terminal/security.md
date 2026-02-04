# Mochi Terminal Security

This document describes the security considerations and safeguards implemented in the Mochi terminal emulator.

## Clipboard Security (OSC 52)

OSC 52 is an escape sequence that allows applications running in the terminal to read from and write to the system clipboard. While useful for remote clipboard access, it poses security risks if not properly controlled.

### Current Implementation

The Mochi terminal implements the following security measures for clipboard operations:

**OSC 52 Clipboard Access:**
- **Disabled by default**: The `osc52_clipboard` configuration option is `false` by default
- **Explicit opt-in required**: Users must explicitly enable clipboard access in their config file
- **Maximum payload size**: Clipboard data is limited to `osc52_max_size` bytes (default: 100,000 bytes)
- **Base64 validation**: All clipboard data must be valid base64-encoded content

### Configuration

```toml
# In ~/.config/mochi/config.toml

# Enable OSC 52 clipboard access (default: false)
osc52_clipboard = false

# Maximum size for clipboard data in bytes (default: 100000)
osc52_max_size = 100000
```

### Recommendations

1. **Keep OSC 52 disabled** unless you specifically need remote clipboard access
2. If enabled, use the smallest `osc52_max_size` that meets your needs
3. Be cautious when running untrusted scripts or connecting to untrusted remote hosts

## Window Title Security

Terminal escape sequences can change the window title. Malicious scripts could abuse this to:
- Display misleading information
- Create phishing-like scenarios
- Cause visual confusion

### Current Implementation

**Title Update Throttling:**
- Title updates are processed but not rate-limited in the current implementation
- Future versions may implement throttling to prevent rapid title changes

### Recommendations

1. Be aware that terminal titles can be changed by running programs
2. Do not rely solely on window titles for security-critical information

## Hyperlink Security (OSC 8)

OSC 8 allows terminals to display clickable hyperlinks. This feature requires careful handling to prevent:
- Automatic navigation to malicious URLs
- Misleading link text that differs from the actual URL

### Current Implementation

**Hyperlink Safety:**
- Links are **never auto-opened** - explicit user action (Ctrl+Click) is required
- Link URLs are stored and can be inspected before clicking
- Visual indication (underline) shows which text is a hyperlink

### Recommendations

1. Always verify the URL before clicking on terminal hyperlinks
2. Be cautious with links from untrusted sources

## Input Handling

### Bracketed Paste Mode

When bracketed paste mode is enabled (by applications like vim, zsh, etc.):
- Pasted text is wrapped in special escape sequences
- This prevents pasted text from being interpreted as commands
- Protects against "paste-jacking" attacks

### Keyboard Input

- Keyboard shortcuts (Ctrl+Shift+C/V/T/R) are handled by the terminal, not sent to the shell
- This prevents applications from intercepting these security-critical shortcuts

## Best Practices

1. **Keep the terminal updated** to receive security fixes
2. **Review your config file** and understand each setting
3. **Be cautious with untrusted content** - scripts, remote connections, etc.
4. **Use SSH keys** instead of passwords when connecting to remote hosts
5. **Verify URLs** before clicking on hyperlinks in the terminal

## Reporting Security Issues

If you discover a security vulnerability in Mochi terminal, please report it responsibly by:
1. Opening a private security advisory on GitHub
2. Emailing the maintainers directly
3. Not disclosing the issue publicly until a fix is available

## Version History

| Version | Security Changes |
|---------|-----------------|
| Phase 2 | Added OSC 52 controls, documented security model |
| Phase 1 | Initial implementation with basic security |
