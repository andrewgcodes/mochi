# ADR 0001: Terminal Emulator Language and Stack

## Status
Accepted

## Context
We need to build a terminal emulator from scratch for the mochi project. The terminal emulator must:
- Run on Linux (X11/Wayland)
- Spawn child processes via PTY
- Parse VT/xterm escape sequences
- Render a GUI with font support
- Handle UTF-8 text correctly
- Be testable and fuzzable

## Decision
We will use **Rust** as the implementation language with the following stack:

### Core Dependencies
- **winit** (0.29): Cross-platform window creation and event handling
- **softbuffer** (0.4): Software rendering without GPU requirements
- **fontdue** (0.8): Pure Rust font rasterization
- **nix** (0.28): Safe Unix system call wrappers for PTY operations
- **arboard** (3.4): Cross-platform clipboard integration

### Supporting Libraries
- **unicode-width** (0.1): Unicode character width calculation
- **serde/serde_json** (1.0): Serialization for snapshots and testing
- **log/env_logger** (0.4/0.11): Logging infrastructure
- **polling** (3.4): Non-blocking I/O polling

### Development Dependencies
- **proptest** (1.4): Property-based testing
- **arbitrary** (1.3): Fuzzing support

## Alternatives Considered

### C++
- Pros: Low-level control, mature ecosystem
- Cons: Memory safety concerns, more complex build system, harder to test

### Go
- Pros: Good concurrency, fast compilation
- Cons: Less control over memory layout, GC pauses could affect rendering

### Python
- Pros: Rapid development
- Cons: Too slow for real-time rendering, not suitable for system programming

## Rationale

Rust was chosen because:

1. **Memory Safety**: Terminal emulators handle untrusted input (escape sequences from remote servers). Rust's ownership system prevents buffer overflows and use-after-free bugs.

2. **Performance**: Zero-cost abstractions allow high-performance rendering without sacrificing safety.

3. **Testing**: Rust's built-in testing framework and cargo ecosystem make it easy to write unit tests, integration tests, and fuzz tests.

4. **Cross-platform**: While we target Linux, the architecture allows future portability.

5. **No Runtime**: No garbage collector means predictable latency for rendering.

6. **Ecosystem**: High-quality crates for windowing (winit), fonts (fontdue), and system calls (nix).

## Consequences

### Positive
- Strong compile-time guarantees
- Excellent tooling (cargo, clippy, rustfmt)
- Easy fuzzing with cargo-fuzz
- Deterministic behavior for testing

### Negative
- Steeper learning curve
- Longer compile times
- Some platform-specific code needed for PTY

## Dependencies Audit

All dependencies are well-maintained, widely used crates:

| Crate | Purpose | License | Notes |
|-------|---------|---------|-------|
| winit | Windowing | Apache-2.0 | Maintained by rust-windowing |
| softbuffer | Rendering | MIT/Apache-2.0 | Simple software rendering |
| fontdue | Fonts | MIT | Pure Rust, no system deps |
| nix | Unix syscalls | MIT | Safe wrappers |
| arboard | Clipboard | MIT/Apache-2.0 | Cross-platform |
| unicode-width | Unicode | MIT/Apache-2.0 | Standard crate |
| serde | Serialization | MIT/Apache-2.0 | De facto standard |
| log | Logging | MIT/Apache-2.0 | Standard facade |

**No terminal emulation libraries are used.** All VT/xterm parsing and screen model logic is implemented from scratch.
