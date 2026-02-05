# Contributing to Mochi Terminal

Thank you for your interest in contributing to Mochi Terminal! This document provides guidelines and information for contributors.

## Getting Started

Fork the repository on GitHub, then clone your fork locally:

```bash
git clone https://github.com/YOUR_USERNAME/mochi.git
cd mochi/terminal
```

Build and run the tests to ensure everything works:

```bash
cargo build --all
cargo test --all
```

## Development Workflow

Create a new branch for your feature or bug fix:

```bash
git checkout -b feature/your-feature-name
```

Make your changes, ensuring that the code compiles without warnings, all tests pass, code is formatted with `cargo fmt`, and clippy reports no warnings.

Before submitting, run the full check suite:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

## Code Style

Mochi follows standard Rust conventions. Run `cargo fmt` before committing to ensure consistent formatting. Use meaningful variable and function names, and keep functions focused and reasonably sized.

When adding new features, include tests that cover the new functionality. For bug fixes, add a test that would have caught the bug.

## Project Structure

The codebase is organized into several crates within the `terminal/` directory:

The `mochi-term` crate contains the main application including the entry point, event loop, rendering, and configuration system. The `terminal-core` crate handles core terminal state management including the screen model, grid, cells, cursor, and selection. The `terminal-parser` crate implements VT/xterm escape sequence parsing. The `terminal-pty` crate manages PTY creation and child process handling.

## Pull Request Process

Ensure your PR includes a clear description of the changes and why they're needed. Reference any related issues using GitHub's issue linking syntax. Make sure all CI checks pass before requesting review.

For significant changes, consider opening an issue first to discuss the approach. This helps ensure your contribution aligns with the project's direction and avoids wasted effort.

## Reporting Issues

When reporting bugs, include the operating system and version, Rust version (`rustc --version`), steps to reproduce the issue, expected behavior, and actual behavior.

For feature requests, describe the use case and why the feature would be valuable to users.

## Testing

Run the full test suite with `cargo test --all`. For specific crates, use `cargo test -p terminal-core` or similar.

When adding new escape sequence handling, consider adding golden tests that verify the terminal state after processing specific input sequences.

## Documentation

Update documentation when adding new features or changing behavior. This includes inline documentation (doc comments), the README if user-facing behavior changes, and configuration documentation in `docs/terminal/config.md` for new config options.

## License

By contributing to Mochi Terminal, you agree that your contributions will be licensed under the MIT License.
