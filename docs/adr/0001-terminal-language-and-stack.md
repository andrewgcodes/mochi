# ADR 0001: Terminal Emulator Language and Technology Stack

## Status
Accepted

## Date
2026-02-04

## Context
We need to build a real Linux terminal emulator from scratch that:
- Runs a real shell via PTY
- Correctly parses VT/xterm escape sequences
- Maintains a screen model with scrollback
- Renders a GUI with font support
- Supports UTF-8, colors (16/256/truecolor), mouse, clipboard, etc.

The implementation must NOT use any terminal emulation libraries (libvte, termwiz, vt100 crates, etc.).

## Decision
We will use **Rust** as the primary language with the following stack:

### Core Dependencies (Allowed)
- **winit**: Cross-platform window creation and event handling
- **wgpu**: Modern GPU rendering API (WebGPU implementation)
- **fontdue**: Simple, fast font rasterization (no shaping, but sufficient for monospace)
- **unicode-width**: Unicode character width tables (we implement terminal rules ourselves)
- **libc**: Low-level PTY and system calls on Linux
- **nix**: Safe Rust bindings for Unix/POSIX APIs (PTY, signals, etc.)
- **arboard**: Clipboard integration
- **serde/serde_json**: Configuration and snapshot serialization
- **log/env_logger**: Logging infrastructure

### Explicitly NOT Using (Forbidden)
- libvte, VTE, or any GTK terminal widget
- termwiz, vt100, or any Rust terminal emulation crate
- libtsm, kmscon, or any other terminal emulation library
- Any code copied from xterm, alacritty, kitty, wezterm, etc.

## Rationale

### Why Rust?
1. **Memory Safety**: Terminal emulators handle untrusted input (escape sequences from remote servers). Rust's ownership model prevents buffer overflows and use-after-free bugs.

2. **Testing & Fuzzing**: Rust has excellent testing infrastructure (cargo test) and fuzzing support (cargo-fuzz, libfuzzer). Critical for parser correctness.

3. **Performance**: Zero-cost abstractions allow high-throughput parsing and rendering without GC pauses.

4. **Type System**: Strong typing helps model complex state machines (parser states, terminal modes) correctly.

5. **Ecosystem**: Good libraries for windowing (winit), rendering (wgpu), and system calls (nix) without terminal emulation.

### Alternatives Considered

**C++**
- Pro: Maximum low-level control, familiar to many
- Con: Manual memory management increases security risk for untrusted input parsing
- Con: Less ergonomic testing and fuzzing setup

**C**
- Pro: Simplest FFI with system calls
- Con: Even higher security risk than C++
- Con: No built-in testing framework

**Go**
- Pro: Good standard library, easy concurrency
- Con: GC pauses could affect rendering smoothness
- Con: Less suitable for low-level PTY manipulation

## Consequences

### Positive
- Memory safety for parsing untrusted escape sequences
- Excellent test coverage with cargo test
- Easy fuzzing with cargo-fuzz
- Strong type system catches state machine bugs at compile time
- Good performance for real-time rendering

### Negative
- Steeper learning curve for contributors unfamiliar with Rust
- Longer compile times than C
- Some system calls require unsafe blocks (PTY operations)

## Project Structure
```
mochi/
├── terminal/           # Main terminal emulator crate
│   ├── src/
│   │   ├── lib.rs      # Library root
│   │   ├── main.rs     # GUI application entry
│   │   ├── core/       # Platform-independent terminal core
│   │   │   ├── mod.rs
│   │   │   ├── screen.rs    # Screen model
│   │   │   ├── cell.rs      # Cell representation
│   │   │   ├── cursor.rs    # Cursor state
│   │   │   ├── scrollback.rs # Scrollback buffer
│   │   │   └── snapshot.rs  # Deterministic snapshots
│   │   ├── parser/     # Escape sequence parser
│   │   │   ├── mod.rs
│   │   │   ├── state.rs     # Parser state machine
│   │   │   └── actions.rs   # Parsed terminal actions
│   │   ├── pty/        # PTY and child process management
│   │   │   ├── mod.rs
│   │   │   └── linux.rs     # Linux-specific PTY code
│   │   ├── input/      # Keyboard/mouse input encoding
│   │   │   └── mod.rs
│   │   └── renderer/   # GUI rendering
│   │       ├── mod.rs
│   │       ├── window.rs
│   │       └── font.rs
│   ├── tests/          # Integration tests
│   └── Cargo.toml
├── docs/
│   ├── adr/            # Architecture Decision Records
│   ├── terminal/       # Terminal-specific docs
│   └── research/       # Research notes with citations
├── assets/
│   └── terminfo/       # Terminfo source files
└── scripts/
    └── tests/
        └── manual/     # Manual test scripts
```

## References
- Rust Book: https://doc.rust-lang.org/book/
- winit: https://github.com/rust-windowing/winit
- wgpu: https://wgpu.rs/
- nix crate: https://docs.rs/nix/
