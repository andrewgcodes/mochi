# ADR 0001: Terminal Emulator Language and Stack

## Status

Accepted

## Context

We need to build a real terminal emulator from scratch that:
- Runs a real shell/apps via a PTY
- Correctly parses VT/xterm escape sequences
- Maintains a screen model
- Renders a GUI
- Does NOT use any existing terminal emulator libraries

The choice of language and technology stack is critical for:
- Safety (handling untrusted input from PTY)
- Performance (parsing and rendering at high throughput)
- Testing (unit tests, golden tests, fuzzing)
- Maintainability (clear module boundaries)

## Decision

We will use **Rust** as the implementation language with the following stack:

### Core Libraries
- **mochi-core**: Platform-independent terminal state (no external deps for core logic)
- **mochi-parser**: Escape sequence parser (no external deps for parsing logic)
- **mochi-pty**: Linux PTY management using `libc` and `nix` crates
- **mochi-term**: GUI application

### Dependencies
- **winit**: Cross-platform window creation and event handling
- **wgpu**: GPU-accelerated rendering (optional, with software fallback)
- **fontdue**: Font rasterization (pure Rust, no system dependencies)
- **libc/nix**: Low-level POSIX APIs for PTY
- **serde/serde_json**: Configuration and snapshot serialization
- **log/env_logger**: Structured logging
- **unicode-width**: Unicode character width calculation
- **arboard**: Clipboard integration

### Forbidden Dependencies
The following are explicitly NOT used:
- libvte, VTE (GTK terminal widget)
- libtsm, kmscon
- termwiz, vt100 crates
- Any crate that implements terminal emulation or ANSI parsing

## Alternatives Considered

### C++
- Pros: Low-level control, mature ecosystem
- Cons: Memory safety concerns, harder to test/fuzz, no built-in package manager

### Go
- Pros: Good concurrency, simple syntax
- Cons: GC pauses could affect rendering, less control over memory layout

### Zig
- Pros: Low-level control, safety features
- Cons: Less mature ecosystem, fewer libraries available

## Consequences

### Positive
- Memory safety without GC pauses
- Excellent testing infrastructure (cargo test, proptest)
- Built-in fuzzing support (cargo-fuzz)
- Strong type system catches many bugs at compile time
- Good performance characteristics
- Cross-platform potential (though we target Linux first)

### Negative
- Steeper learning curve for contributors unfamiliar with Rust
- Longer compile times
- Some GUI libraries less mature than C++ equivalents

## References

- [Rust Programming Language](https://www.rust-lang.org/)
- [winit](https://github.com/rust-windowing/winit)
- [wgpu](https://wgpu.rs/)
- [fontdue](https://github.com/mooman219/fontdue)
