# Mochi Terminal Configuration Reference

This document describes all configuration options available in Mochi Terminal.

## Configuration File Location

Mochi Terminal follows the XDG Base Directory specification for configuration files:

1. **Default location**: `~/.config/mochi/config.toml`
2. **XDG override**: If `XDG_CONFIG_HOME` is set, uses `$XDG_CONFIG_HOME/mochi/config.toml`
3. **CLI override**: Use `--config /path/to/config.toml` to specify a custom path

## Configuration Precedence

Configuration values are resolved in the following order (highest priority first):

1. **CLI arguments** - Command-line flags like `--theme dark`
2. **Environment variables** - Variables like `MOCHI_THEME=dark`
3. **Config file** - Values from `config.toml`
4. **Built-in defaults** - Hardcoded fallback values

## CLI Arguments

```
mochi [OPTIONS]

Options:
  -c, --config <FILE>      Path to configuration file
  -t, --theme <THEME>      Theme to use (dark, light, solarized-dark, solarized-light, dracula, nord)
      --font-size <SIZE>   Font size in points
      --font-family <NAME> Font family name
  -s, --shell <COMMAND>    Shell command to run
      --scrollback <LINES> Number of scrollback lines
      --cols <COLS>        Initial columns
      --rows <ROWS>        Initial rows
  -h, --help               Print help
  -V, --version            Print version
```

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `MOCHI_CONFIG` | Path to config file | `/home/user/mochi.toml` |
| `MOCHI_THEME` | Theme name | `dracula` |
| `MOCHI_FONT_SIZE` | Font size in points | `16.0` |
| `MOCHI_SHELL` | Shell command | `/bin/zsh` |

## Configuration Options

### Font Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `font_family` | string | `"DejaVu Sans Mono"` | Font family name |
| `font_size` | float | `14.0` | Font size in points |

The terminal ships with DejaVu Sans Mono bundled. You can specify any monospace font installed on your system, but non-monospace fonts may cause rendering issues.

### Theme Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `theme` | string | `"dark"` | Theme name or "custom" |

#### Available Themes

| Theme Name | Aliases | Description |
|------------|---------|-------------|
| `dark` | `mochi-dark` | Dark theme with comfortable contrast |
| `light` | `mochi-light` | Light theme for bright environments |
| `solarized-dark` | - | Solarized dark color scheme |
| `solarized-light` | - | Solarized light color scheme |
| `dracula` | - | Popular dark theme with vibrant colors |
| `nord` | - | Arctic, north-bluish color palette |
| `custom` | - | Use colors from `[colors]` section |

### Terminal Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `scrollback_lines` | integer | `10000` | Number of scrollback lines (0 to disable) |
| `dimensions` | [cols, rows] | `[80, 24]` | Initial terminal dimensions |
| `shell` | string | `$SHELL` | Shell command to run |
| `cursor_style` | string | `"block"` | Cursor style: "block", "underline", or "bar" |
| `cursor_blink` | boolean | `true` | Enable cursor blinking |

### Security Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `osc52_clipboard` | boolean | `false` | Enable OSC 52 clipboard access |
| `osc52_max_size` | integer | `100000` | Maximum OSC 52 payload size in bytes |

**Security Warning**: OSC 52 clipboard access allows applications to read and write your system clipboard via escape sequences. Only enable this if you trust all applications you run in the terminal.

### Keybindings

Keybindings are configured in the `[keybindings]` section:

```toml
[keybindings]
bindings = [
    { key = "ctrl+shift+c", action = "Copy" },
    { key = "ctrl+shift+v", action = "Paste" },
]
```

#### Key Format

Keys are specified as `modifier+key` where:
- **Modifiers**: `ctrl`, `shift`, `alt`, `super`
- **Keys**: Single characters (`a`, `v`), special keys (`plus`, `minus`, `0`)

Multiple modifiers can be combined: `ctrl+shift+c`

#### Available Actions

| Action | Default Binding | Description |
|--------|-----------------|-------------|
| `Copy` | `ctrl+shift+c` | Copy selected text to clipboard |
| `Paste` | `ctrl+shift+v` | Paste from clipboard |
| `Find` | `ctrl+shift+f` | Open search/find bar |
| `ReloadConfig` | `ctrl+shift+r` | Reload configuration file |
| `ToggleTheme` | `ctrl+shift+t` | Cycle through available themes |
| `ZoomIn` | `ctrl+plus` | Increase font size |
| `ZoomOut` | `ctrl+minus` | Decrease font size |
| `ResetZoom` | `ctrl+0` | Reset font size to default |

### Custom Colors

When `theme = "custom"`, colors are read from the `[colors]` section:

```toml
[colors]
foreground = "#d4d4d4"
background = "#1e1e1e"
cursor = "#ffffff"
selection = "#264f78"
ansi = [
    "#1e1e1e",  # 0: Black
    "#f44747",  # 1: Red
    "#6a9955",  # 2: Green
    "#dcdcaa",  # 3: Yellow
    "#569cd6",  # 4: Blue
    "#c586c0",  # 5: Magenta
    "#4ec9b0",  # 6: Cyan
    "#d4d4d4",  # 7: White
    "#808080",  # 8: Bright Black
    "#f44747",  # 9: Bright Red
    "#6a9955",  # 10: Bright Green
    "#dcdcaa",  # 11: Bright Yellow
    "#569cd6",  # 12: Bright Blue
    "#c586c0",  # 13: Bright Magenta
    "#4ec9b0",  # 14: Bright Cyan
    "#ffffff",  # 15: Bright White
]
```

All colors are specified as hex strings in `#RRGGBB` format.

## Example Configuration

See [config.example.toml](config.example.toml) for a complete example configuration file with all options documented.

## Runtime Configuration

### Reloading Configuration

Press `Ctrl+Shift+R` (or your configured keybinding) to reload the configuration file without restarting the terminal. If the reload fails (e.g., invalid TOML), the previous configuration is preserved and an error is logged.

### Switching Themes

Press `Ctrl+Shift+T` (or your configured keybinding) to cycle through available themes at runtime. The theme order is: dark -> light -> solarized-dark -> solarized-light -> dracula -> nord -> dark.

## Troubleshooting

### Configuration Not Loading

1. Check the config file path: `mochi --help` shows the default location
2. Verify TOML syntax: Use a TOML validator
3. Check logs: Run with `RUST_LOG=debug mochi` for detailed logging

### Font Not Found

If your specified font is not found, the terminal falls back to the bundled DejaVu Sans Mono. Check that:
1. The font is installed on your system
2. The font name is spelled correctly (case-sensitive on some systems)
3. The font is a monospace font

### Theme Colors Look Wrong

1. Ensure your terminal is using true color (24-bit) mode
2. Check that your `$TERM` environment variable is set correctly
3. Try a different theme to isolate the issue
