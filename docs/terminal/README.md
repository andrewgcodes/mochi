# Mochi Terminal Emulator

A real Linux terminal emulator built from scratch, implementing VT/xterm-style terminal emulation with proper PTY handling, escape sequence parsing, screen state management, and GPU-accelerated rendering.

## Project Goals

1. **Genuine Terminal Emulation**: Implement a real VT/xterm-compatible terminal that can run interactive applications like vim, htop, tmux, and ssh.

2. **From Scratch Implementation**: No use of existing terminal emulation libraries. We implement:
   - Escape sequence parsing (CSI, OSC, DCS, etc.)
   - Screen model with scrollback
   - PTY management
   - GUI rendering

3. **Correctness First**: Prioritize correct behavior over features. Every implemented feature must be tested and documented.

4. **Security Conscious**: Handle untrusted input safely, with explicit security controls for dangerous features like OSC 52 clipboard access.

## Non-Goals

- Windows/macOS support (Linux-only for now)
- Tektronix 4014 graphics mode
- Sixel graphics (may be added later)
- Full VT520 compatibility (targeting practical xterm-256color subset)

## Architecture Overview

The terminal is structured as several independent modules:

```
┌─────────────────────────────────────────────────────────────┐
│                      mochi-term (GUI)                       │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Window    │  │  Renderer   │  │   Input Encoder     │  │
│  │  (winit)    │  │   (wgpu)    │  │  (keys → sequences) │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                     terminal-core                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Screen    │  │  Scrollback │  │   Cursor/Modes      │  │
│  │    Grid     │  │    Buffer   │  │      State          │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                    terminal-parser                          │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  State Machine: bytes → Terminal Actions                ││
│  │  (CSI, OSC, ESC, DCS parsing)                          ││
│  └─────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                         PTY                                 │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  posix_openpt → grantpt → unlockpt → fork/exec         ││
│  │  Non-blocking I/O, SIGWINCH handling                   ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

## Dependency Audit

### Allowed Dependencies (per project rules)

| Crate | Purpose | Why Allowed |
|-------|---------|-------------|
| `nix` | POSIX syscall bindings | Low-level PTY ops, not terminal emulation |
| `winit` | Window creation | GUI framework, not terminal logic |
| `wgpu` | GPU rendering | Rendering only |
| `fontdue` | Font rasterization | Glyph rendering only |
| `unicode-width` | Character width | Data tables only, we implement rules |
| `arboard` | Clipboard | System integration only |
| `tracing` | Logging | Debugging infrastructure |
| `serde` | Serialization | Testing infrastructure |

### Forbidden Dependencies

- Any crate that parses ANSI/VT escape sequences
- Any crate that implements terminal screen logic
- Any crate that wraps libvte or similar
- Any code copied from existing terminal emulators

## Building

```bash
cd terminal
cargo build --release
```

## Running

```bash
# GUI terminal
cargo run --release --bin mochi-term

# Headless mode (for testing)
cargo run --release --bin mochi-headless < input.txt > snapshot.json
```

## Testing

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Golden tests
cargo test --test golden

# Integration tests (requires PTY)
cargo test --test integration

# Fuzzing (requires nightly)
cargo +nightly fuzz run parser
```

## Documentation

- [Architecture Details](../architecture.md)
- [Escape Sequence Coverage](../escape-sequences.md)
- [Security Considerations](../security.md)
- [Testing Strategy](../testing.md)
- [Research Notes](../research/)

## Target Compatibility

We target `xterm-256color` compatibility level, meaning:
- 256-color and truecolor support
- Alternate screen buffer
- Mouse reporting (SGR 1006 mode)
- Bracketed paste
- Common DEC private modes

See [escape-sequences.md](../escape-sequences.md) for the complete coverage matrix.

## License

[To be determined]
