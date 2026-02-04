# ADR 0001: Terminal Emulator Language and Stack Choice

## Status

Accepted

## Context

We need to build a real Linux terminal emulator from scratch that:
- Runs a real shell/apps via a PTY
- Correctly parses VT/xterm escape sequences
- Maintains a screen model
- Renders a GUI
- Does NOT wrap any existing terminal widget/library/implementation

The choice of language and technology stack is critical for:
- Safety and correctness (terminal emulators handle untrusted input)
- Testing and fuzzing capabilities
- Performance (rendering, parsing throughput)
- Cross-platform potential (Linux first, but future portability)
- Developer productivity

## Decision

We chose **Rust** as the implementation language with the following stack:

### Core Dependencies

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | Rust 2021 Edition | Memory safety, strong typing, excellent testing/fuzzing support |
| PTY handling | nix crate | Safe Rust bindings to POSIX PTY APIs |
| GUI windowing | winit | Cross-platform, well-maintained, Wayland/X11 support |
| GPU rendering | wgpu | Modern, safe GPU abstraction (Vulkan/Metal/DX12/WebGPU) |
| Font rendering | fontdue | Pure Rust, no system dependencies |
| Unicode | unicode-width, unicode-segmentation | Standard Unicode handling |
| Serialization | serde, serde_json | Snapshot testing, config files |

### Crate Structure

```
terminal/
├── mochi-core/     # Platform-independent terminal logic
├── mochi-parser/   # VT/xterm escape sequence parser
├── mochi-pty/      # Linux PTY handling
└── mochi-term/     # GUI application
```

## Alternatives Considered

### C++

**Pros:**
- Low-level control
- Mature ecosystem for terminal emulators
- Direct system API access

**Cons:**
- Memory safety concerns (buffer overflows, use-after-free)
- Manual memory management
- Less ergonomic testing/fuzzing
- Build system complexity

### Go

**Pros:**
- Simple concurrency model
- Fast compilation
- Good standard library

**Cons:**
- Garbage collection pauses (problematic for smooth rendering)
- Less control over memory layout
- Weaker type system for modeling terminal state

### Python (with C extensions)

**Pros:**
- Rapid prototyping
- Easy to test

**Cons:**
- Performance concerns for parsing/rendering
- Would need C extensions for critical paths
- Not suitable for a production terminal emulator

## Consequences

### Positive

1. **Memory Safety**: Rust's ownership system prevents common bugs like buffer overflows, which are critical when parsing untrusted escape sequences.

2. **Testing**: Rust's built-in testing framework and cargo make it easy to write and run unit tests, integration tests, and property-based tests.

3. **Fuzzing**: Rust has excellent fuzzing support via cargo-fuzz and libFuzzer, essential for parser correctness.

4. **Performance**: Zero-cost abstractions allow high-level code without runtime overhead.

5. **Concurrency**: Fearless concurrency for handling PTY I/O and rendering in parallel.

6. **Ecosystem**: Strong crates for GUI (winit), GPU (wgpu), and system APIs (nix).

### Negative

1. **Learning Curve**: Rust's ownership system requires adjustment for developers unfamiliar with it.

2. **Compilation Time**: Rust compilation is slower than C/Go, though incremental builds help.

3. **Binary Size**: Rust binaries are larger than C equivalents (mitigated by LTO and stripping).

### Neutral

1. **Linux-First**: The PTY implementation is Linux-specific, but the core terminal logic is platform-independent.

2. **No libvte/termwiz**: We explicitly avoid terminal emulator libraries per the spec requirements.

## References

- [Rust Programming Language](https://www.rust-lang.org/)
- [winit - Cross-platform window creation](https://github.com/rust-windowing/winit)
- [wgpu - Safe GPU abstraction](https://wgpu.rs/)
- [nix - Rust friendly bindings to *nix APIs](https://github.com/nix-rust/nix)
