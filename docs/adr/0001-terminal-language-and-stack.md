# ADR 0001: Terminal Emulator Language and Technology Stack

## Status
Accepted

## Context
We need to build a real Linux terminal emulator from scratch that:
- Spawns a child shell attached to a PTY
- Correctly parses VT/xterm escape sequences
- Maintains a screen model with scrollback
- Renders a GUI with proper font handling
- Handles UTF-8, colors, mouse input, clipboard, and more

The implementation must NOT use any existing terminal emulation libraries (libvte, termwiz, vt100 crates, etc.).

## Decision
We will use **Rust** as the primary implementation language.

### Rationale

1. **Memory Safety**: Terminal emulators process untrusted input from potentially malicious sources. Rust's ownership model prevents buffer overflows, use-after-free, and other memory safety issues without runtime overhead.

2. **Performance**: Rust compiles to native code with performance comparable to C/C++. Terminal emulators need to handle high-throughput output (e.g., `cat` of large files) with minimal latency.

3. **Testing Infrastructure**: Rust has excellent built-in testing support (`cargo test`), property-based testing (proptest), and fuzzing support (cargo-fuzz with libFuzzer). This is critical for validating parser correctness.

4. **Ecosystem for Allowed Dependencies**:
   - `nix` crate: Low-level POSIX bindings for PTY operations (NOT a terminal emulator)
   - `winit`: Cross-platform window creation and event handling
   - `wgpu`: Modern GPU abstraction for rendering
   - `fontdue`: Pure Rust font rasterization
   - `unicode-width`: Unicode character width tables
   - `arboard`: Clipboard integration

5. **Error Handling**: Rust's `Result` type and `?` operator make it easy to handle errors explicitly without exceptions.

6. **Concurrency Safety**: The borrow checker prevents data races, important for handling PTY I/O and GUI rendering on separate threads.

### Alternatives Considered

**C++**
- Pros: Low-level control, mature ecosystem
- Cons: Manual memory management increases security risk, no built-in testing framework, harder to write correct concurrent code

**C**
- Pros: Maximum control, minimal dependencies
- Cons: Even higher security risk than C++, no namespaces/modules, primitive error handling

**Go**
- Pros: Good concurrency, garbage collected
- Cons: GC pauses could affect rendering latency, less control over memory layout

**Zig**
- Pros: Memory safety features, C interop
- Cons: Less mature ecosystem, fewer libraries available

## Technology Stack

### Core Dependencies (Allowed per spec)

| Purpose | Crate | Justification |
|---------|-------|---------------|
| PTY operations | `nix` | Low-level POSIX bindings, not terminal emulation |
| Window/events | `winit` | Cross-platform window creation |
| GPU rendering | `wgpu` | Modern, safe GPU abstraction |
| Font rasterization | `fontdue` | Pure Rust, no system dependencies |
| Unicode width | `unicode-width` | Width tables only, we implement terminal rules |
| Clipboard | `arboard` | System clipboard integration |
| Logging | `tracing` | Structured logging for debugging |
| Serialization | `serde`, `serde_json` | For snapshot tests and config |

### NOT Using (Forbidden per spec)

- `vte` crate (terminal parser)
- `termwiz` (terminal emulation)
- `crossterm` (terminal manipulation)
- Any libvte bindings
- Any existing terminal emulator code

## Project Structure

```
terminal/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Library root
│   ├── core/            # Platform-independent screen model
│   │   ├── mod.rs
│   │   ├── cell.rs      # Cell with char + attributes
│   │   ├── line.rs      # Line of cells
│   │   ├── screen.rs    # Screen grid + cursor + modes
│   │   ├── scrollback.rs # Ring buffer for history
│   │   └── snapshot.rs  # Serializable state for testing
│   ├── parser/          # Escape sequence parser
│   │   ├── mod.rs
│   │   ├── state.rs     # Parser state machine
│   │   └── actions.rs   # Parsed terminal actions
│   ├── pty/             # Linux PTY handling
│   │   ├── mod.rs
│   │   └── unix.rs      # POSIX PTY implementation
│   ├── frontend/        # GUI rendering
│   │   ├── mod.rs
│   │   ├── window.rs    # Window management
│   │   ├── renderer.rs  # GPU rendering
│   │   └── input.rs     # Keyboard/mouse encoding
│   └── app/             # Application glue
│       ├── mod.rs
│       └── config.rs    # Configuration
├── tests/
│   ├── unit/            # Unit tests
│   ├── golden/          # Snapshot tests
│   └── integration/     # PTY integration tests
└── bin/
    ├── mochi-term.rs    # Main GUI application
    └── mochi-headless.rs # Headless test runner
```

## Consequences

### Positive
- Memory safety without runtime overhead
- Excellent testing and fuzzing support
- Strong type system catches bugs at compile time
- Good ecosystem for our allowed dependencies
- Cross-compilation potential for future platforms

### Negative
- Steeper learning curve than some languages
- Longer compile times than C
- Some low-level PTY operations require `unsafe` blocks

### Risks
- Must carefully audit any `unsafe` code
- Need to ensure all dependencies are actively maintained
- GPU rendering complexity may require iteration

## References
- [Rust Book](https://doc.rust-lang.org/book/)
- [nix crate documentation](https://docs.rs/nix/)
- [wgpu documentation](https://wgpu.rs/)
- [XTerm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)
- [ECMA-48 Standard](https://ecma-international.org/publications-and-standards/standards/ecma-48/)
