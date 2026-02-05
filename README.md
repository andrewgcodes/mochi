# Mochi Terminal

A modern, GPU-accelerated terminal emulator written in Rust with a focus on performance, customization, and compatibility.

## Features

**Core Terminal Emulation**
- VT/xterm-compatible escape sequence parsing
- Full 256-color and true color (24-bit RGB) support
- Unicode and wide character support
- Scrollback buffer with configurable history (up to 10 million lines)
- Mouse support (click, drag, scroll, SGR encoding)
- Bracketed paste mode for secure pasting
- OSC 8 hyperlink support

**Theming and Customization**
- 6 built-in themes: Dark, Light, Solarized Dark, Solarized Light, Dracula, Nord
- Runtime theme switching with `Ctrl+Shift+T`
- Fully customizable color schemes via TOML configuration
- Configurable fonts with fallback support
- Adjustable font size with zoom controls (`Ctrl++`, `Ctrl+-`, `Ctrl+0`)

**Modern Architecture**
- CPU-based rendering via softbuffer (no GPU driver dependencies)
- Efficient glyph caching for fast text rendering
- Modular crate structure for maintainability
- Cross-platform support (Linux, macOS)

**TUI Application Support**
- Works with Claude Code, Gemini CLI, GitHub Copilot CLI, and other Ink-based apps
- Proper handling of DEC mode 2026 (synchronized output)
- Alternate screen buffer support for vim, htop, less, etc.

## Installation

### Prerequisites

- Rust 1.70 or later
- On Linux: X11 development libraries

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

Mochi uses a TOML configuration file located at `~/.config/mochi/config.toml`.

### Quick Start

Create a minimal config:

```bash
mkdir -p ~/.config/mochi
cat > ~/.config/mochi/config.toml << 'EOF'
theme = "dark"

[font]
size = 14.0
EOF
```

### Configuration Precedence

1. Command-line arguments (highest priority)
2. Environment variables
3. Configuration file
4. Built-in defaults

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

See [docs/terminal/config.md](docs/terminal/config.md) for the complete configuration reference.

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

On macOS, `Cmd+C` and `Cmd+V` also work for copy/paste.

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

Mochi takes security seriously:

- **OSC 52 clipboard disabled by default**: Programs cannot access your clipboard unless explicitly enabled
- **Bracketed paste support**: Prevents paste injection attacks
- **No auto-opening links**: Hyperlinks require explicit user action
- **Input validation**: Malformed escape sequences are handled gracefully

See [docs/terminal/security.md](docs/terminal/security.md) for details.

## Compatibility

Mochi aims for compatibility with xterm and other modern terminal emulators. Tested applications include:

- Shells: bash, zsh, fish
- Editors: vim, neovim, nano, emacs
- TUI apps: htop, btop, tmux, Claude Code, Gemini CLI
- Development tools: git, cargo, npm

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

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Run `cargo fmt` and `cargo clippy`
5. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Built with:
- [winit](https://github.com/rust-windowing/winit) - Cross-platform window management
- [softbuffer](https://github.com/rust-windowing/softbuffer) - CPU-based rendering
- [fontdue](https://github.com/mooman219/fontdue) - Font rasterization
- [arboard](https://github.com/1Password/arboard) - Clipboard access
