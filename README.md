# Mochi Terminal

A modern, customizable terminal emulator written in Rust with a focus on performance, compatibility, and developer experience.

> **Disclaimer**: This software is provided as-is for educational and experimental purposes. Use at your own risk. As with any terminal emulator, there may be undiscovered security vulnerabilities. Do not use this software in security-critical environments without thorough review.

## Features

Mochi provides a complete terminal emulation experience with VT/xterm-compatible escape sequence parsing, full 256-color and true color (24-bit RGB) support, Unicode and wide character handling, and a configurable scrollback buffer supporting up to 10 million lines of history. Mouse support includes click, drag, scroll, and SGR encoding, while security features include bracketed paste mode and OSC 8 hyperlink support.

The terminal offers extensive theming and customization options with 6 built-in themes (Dark, Light, Solarized Dark, Solarized Light, Dracula, and Nord), runtime theme switching via `Ctrl+Shift+T`, and fully customizable color schemes through TOML configuration. Font settings include configurable family, size, and fallback support, with zoom controls (`Ctrl++`, `Ctrl+-`, `Ctrl+0`).

Built with a modern architecture using CPU-based rendering via softbuffer (no GPU driver dependencies required), efficient glyph caching for fast text rendering, and a modular crate structure for maintainability. Cross-platform support covers Linux and macOS.

Mochi works seamlessly with TUI applications including Claude Code, Gemini CLI, GitHub Copilot CLI, and other Ink-based apps through proper handling of DEC mode 2026 (synchronized output). Alternate screen buffer support ensures compatibility with vim, htop, less, tmux, and similar applications.

## Installation

### Prerequisites

Mochi requires Rust 1.70 or later. On Linux, you'll need X11 development libraries:

```bash
# Ubuntu/Debian
sudo apt install libx11-dev libxcursor-dev libxrandr-dev libxi-dev

# Fedora
sudo dnf install libX11-devel libXcursor-devel libXrandr-devel libXi-devel

# Arch
sudo pacman -S libx11 libxcursor libxrandr libxi
```

### Building from Source

```bash
git clone https://github.com/andrewgcodes/mochi.git
cd mochi/terminal
cargo build --release
```

The binary will be at `target/release/mochi`.

### Running

```bash
./target/release/mochi
```

Or with options:

```bash
./target/release/mochi --theme dracula --font-size 16
```

## Configuration

Mochi uses a TOML configuration file located at `~/.config/mochi/config.toml` following XDG conventions.

### Quick Start

Create a minimal configuration:

```bash
mkdir -p ~/.config/mochi
cat > ~/.config/mochi/config.toml << 'EOF'
theme = "dark"

[font]
size = 14.0
EOF
```

### Configuration Precedence

Settings are applied in the following order (highest priority first): command-line arguments, environment variables, configuration file, and built-in defaults.

### Example Configuration

```toml
# Theme: dark, light, solarized-dark, solarized-light, dracula, nord
theme = "dracula"

# Scrollback history
scrollback_lines = 50000

[font]
family = "JetBrains Mono"
size = 14.0

[keybindings]
copy = "ctrl+shift+c"
paste = "ctrl+shift+v"
toggle_theme = "ctrl+shift+t"

[security]
osc52_clipboard = false  # Disabled by default for security
```

See [docs/terminal/config.md](docs/terminal/config.md) for the complete configuration reference and [docs/terminal/config.example.toml](docs/terminal/config.example.toml) for a fully commented example.

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Shift+C` | Copy selection |
| `Ctrl+Shift+V` | Paste from clipboard |
| `Ctrl+Shift+T` | Cycle through themes |
| `Ctrl+Shift+R` | Reload configuration |
| `Ctrl++` or `Ctrl+=` | Zoom in |
| `Ctrl+-` | Zoom out |
| `Ctrl+0` | Reset zoom |

On macOS, `Cmd+C` and `Cmd+V` also work for copy and paste.

## Command-Line Options

```
Usage: mochi [OPTIONS]

Options:
  -c, --config <FILE>      Path to config file
      --font-family <FONT> Font family name
      --font-size <SIZE>   Font size in points
  -t, --theme <THEME>      Theme (dark, light, dracula, etc.)
  -s, --shell <SHELL>      Shell command to run
      --scrollback <LINES> Scrollback buffer size
      --columns <COLS>     Initial columns
      --rows <ROWS>        Initial rows
      --enable-osc52       Enable OSC 52 clipboard (security risk)
  -h, --help               Print help
  -V, --version            Print version
```

## Project Structure

```
terminal/
├── mochi-term/       # Main application crate
│   └── src/
│       ├── main.rs       # Entry point
│       ├── app.rs        # Application state and event loop
│       ├── renderer.rs   # CPU rendering
│       ├── terminal.rs   # Terminal state management
│       ├── config.rs     # Configuration system
│       └── input.rs      # Keyboard/mouse input encoding
├── terminal-core/    # Core terminal state (screen, grid, cells)
├── terminal-parser/  # VT/xterm escape sequence parser
└── terminal-pty/     # PTY management and child process handling
```

## Security

Mochi takes security seriously with OSC 52 clipboard access disabled by default (programs cannot access your clipboard unless explicitly enabled), bracketed paste support to prevent paste injection attacks, no auto-opening of links (hyperlinks require explicit user action), and graceful handling of malformed escape sequences.

See [docs/terminal/security.md](docs/terminal/security.md) for detailed security documentation.

## Compatibility

Mochi aims for compatibility with xterm and other modern terminal emulators. Tested applications include shells (bash, zsh, fish), editors (vim, neovim, nano, emacs), TUI applications (htop, btop, tmux, Claude Code, Gemini CLI), and development tools (git, cargo, npm).

## Development

### Running Tests

```bash
cd terminal
cargo test --all
```

### Linting

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Formatting

```bash
cargo fmt --all
```

## Contributing

Contributions are welcome! Please fork the repository, create a feature branch, make your changes with tests, run `cargo fmt` and `cargo clippy`, and submit a pull request.

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Built with [winit](https://github.com/rust-windowing/winit) for cross-platform window management, [softbuffer](https://github.com/rust-windowing/softbuffer) for CPU-based rendering, [fontdue](https://github.com/mooman219/fontdue) for font rasterization, and [arboard](https://github.com/1Password/arboard) for clipboard access.
