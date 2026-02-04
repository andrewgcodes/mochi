# ADR 0001: Terminal Emulator Language and Stack

## Status
Accepted

## Context
We need to build a real Linux terminal emulator from scratch that:
- Runs a real shell/apps via PTY
- Correctly parses VT/xterm escape sequences
- Maintains a screen model
- Renders a GUI
- Supports UTF-8, colors, cursor styles, alternate screen, mouse reporting, etc.

The implementation must not use existing terminal emulation libraries (libvte, termwiz, etc.).

## Decision

### Language: Rust

We chose Rust for the following reasons:

1. **Memory Safety**: Terminal emulators process untrusted input (escape sequences from remote servers). Rust's ownership model prevents buffer overflows and use-after-free bugs that are common in C/C++ terminal implementations.

2. **Performance**: Rust compiles to native code with performance comparable to C/C++, which is important for smooth scrolling and high-throughput terminal output.

3. **Testing Infrastructure**: Rust has excellent built-in testing support with `cargo test`, property-based testing via `proptest`, and fuzzing support via `cargo-fuzz`.

4. **Type System**: Rust's strong type system helps model terminal state machines correctly and catch errors at compile time.

5. **Ecosystem**: Good crates available for windowing (winit), font rendering (fontdue), and system interfaces (nix).

### GUI Stack

- **winit**: Cross-platform window creation and event handling
- **softbuffer**: CPU-based rendering to window surface
- **fontdue**: Pure Rust font rasterization

We chose CPU-based rendering over GPU (wgpu/OpenGL) for simplicity and portability. Terminal rendering is not GPU-intensive, and CPU rendering avoids driver compatibility issues.

### PTY Interface

- **nix crate**: Provides safe Rust bindings to POSIX APIs for PTY management, signals, and process control.

### Dependencies

Core dependencies (all allowed per spec):
- `winit` - Window creation and event handling
- `softbuffer` - CPU rendering to window
- `fontdue` - Font rasterization
- `nix` - POSIX system calls
- `arboard` - Clipboard integration
- `serde` + `toml` - Configuration
- `unicode-width` - Character width calculation

No forbidden dependencies (libvte, termwiz, vt100, etc.) are used.

## Alternatives Considered

### C++
- Pro: Maximum control, existing terminal implementations to reference
- Con: Memory safety concerns, slower iteration, less testing infrastructure

### Go
- Pro: Good standard library, easy concurrency
- Con: GC pauses could affect rendering smoothness, less control over memory layout

### Python
- Pro: Rapid development
- Con: Too slow for terminal emulation, not suitable for real-time rendering

## Consequences

### Positive
- Memory-safe implementation reduces security vulnerabilities
- Strong type system catches many bugs at compile time
- Excellent testing and fuzzing support
- Good performance for terminal workloads

### Negative
- Steeper learning curve for contributors unfamiliar with Rust
- Longer compile times compared to C
- Some platform-specific code needed for PTY handling

## References
- [Rust Book](https://doc.rust-lang.org/book/)
- [winit documentation](https://docs.rs/winit)
- [nix crate documentation](https://docs.rs/nix)
