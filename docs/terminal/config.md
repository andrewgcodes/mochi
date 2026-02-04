# Mochi Terminal Configuration

This document describes the configuration system for Mochi Terminal.

## Configuration File Location

Mochi Terminal follows the XDG Base Directory Specification. The configuration file is located at:

```
~/.config/mochi/config.toml
```

If this file does not exist, Mochi Terminal will use built-in defaults.

## Configuration Precedence

Configuration values are loaded with the following precedence (highest to lowest):

1. **CLI arguments** - Command-line flags override all other settings
2. **Environment variables** - `MOCHI_*` environment variables
3. **Config file** - Values from `~/.config/mochi/config.toml`
4. **Built-in defaults** - Sensible defaults for all settings

## CLI Arguments

```
mochi [OPTIONS]

Options:
  -c, --config <FILE>      Path to configuration file
      --font-size <SIZE>   Font size in points
  -t, --theme <THEME>      Theme name
      --shell <SHELL>      Shell command to run
      --scrollback <LINES> Number of scrollback lines
      --osc52-clipboard    Enable OSC 52 clipboard support
      --cols <COLS>        Initial window columns
      --rows <ROWS>        Initial window rows
  -h, --help               Print help
  -V, --version            Print version
```

## Environment Variables

The following environment variables are supported:

| Variable | Description | Example |
|----------|-------------|---------|
| `MOCHI_FONT_SIZE` | Font size in points | `MOCHI_FONT_SIZE=16` |
| `MOCHI_THEME` | Theme name | `MOCHI_THEME=dracula` |
| `MOCHI_SHELL` | Shell command | `MOCHI_SHELL=/bin/zsh` |
| `MOCHI_SCROLLBACK` | Scrollback lines | `MOCHI_SCROLLBACK=50000` |
| `MOCHI_OSC52_CLIPBOARD` | Enable OSC 52 clipboard | `MOCHI_OSC52_CLIPBOARD=true` |
| `MOCHI_FONT_FAMILY` | Font family name | `MOCHI_FONT_FAMILY=JetBrains Mono` |

## Configuration Options

### Font Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `font_family` | string | `"monospace"` | Font family name |
| `font_size` | float | `14.0` | Font size in points (6.0 - 128.0) |

### Terminal Dimensions

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `dimensions` | [u16, u16] | `[80, 24]` | Initial [columns, rows] |
| `scrollback_lines` | usize | `10000` | Scrollback buffer size (0 - 1,000,000) |

### Theme Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `theme` | string | `"dark"` | Theme name (see available themes below) |

#### Available Themes

- `dark` - VS Code inspired dark theme (default)
- `light` - Clean light theme
- `solarized-dark` - Solarized dark palette
- `solarized-light` - Solarized light palette
- `dracula` - Dracula theme
- `nord` - Nord theme
- `monokai` - Monokai theme
- `gruvbox-dark` - Gruvbox dark theme
- `custom` - Use custom colors from the `[colors]` section

### Custom Colors

When `theme = "custom"`, you can define your own color scheme:

```toml
[colors]
foreground = "#d4d4d4"
background = "#1e1e1e"
cursor = "#ffffff"
selection = "#264f78"
ansi = [
    "#000000",  # 0: Black
    "#cd3131",  # 1: Red
    "#0dbc79",  # 2: Green
    "#e5e510",  # 3: Yellow
    "#2472c8",  # 4: Blue
    "#bc3fbc",  # 5: Magenta
    "#11a8cd",  # 6: Cyan
    "#e5e5e5",  # 7: White
    "#666666",  # 8: Bright Black
    "#f14c4c",  # 9: Bright Red
    "#23d18b",  # 10: Bright Green
    "#f5f543",  # 11: Bright Yellow
    "#3b8eea",  # 12: Bright Blue
    "#d670d6",  # 13: Bright Magenta
    "#29b8db",  # 14: Bright Cyan
    "#ffffff",  # 15: Bright White
]
```

All colors must be specified in hex format (`#RRGGBB`).

### Cursor Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `cursor_style` | string | `"block"` | Cursor style: `"block"`, `"underline"`, or `"bar"` |
| `cursor_blink` | bool | `true` | Enable cursor blinking |

### Shell Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `shell` | string | (none) | Shell command to run. If not set, uses `$SHELL` |

### Security Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `osc52_clipboard` | bool | `false` | Enable OSC 52 clipboard support |
| `osc52_max_size` | usize | `100000` | Maximum OSC 52 payload size in bytes |

**Security Note:** OSC 52 clipboard support is disabled by default because it allows applications running in the terminal to read and write your system clipboard via escape sequences. Only enable this if you trust the applications you run.

## Example Configuration

See [config.example.toml](config.example.toml) for a fully commented example configuration file.

## Validation

Mochi Terminal validates configuration values on load:

- Font size must be between 6.0 and 128.0
- Scrollback lines must be at most 1,000,000
- Columns must be between 10 and 1000
- Rows must be between 5 and 500
- OSC 52 max size must be at most 10,000,000
- All color values must be valid hex format (#RRGGBB)

If validation fails, Mochi Terminal will log an error and fall back to default values.

## Error Handling

When configuration loading fails:

1. If the config file cannot be read, a warning is logged and defaults are used
2. If the config file has invalid TOML syntax, an error is logged and defaults are used
3. If validation fails, an error is logged and defaults are used
4. CLI arguments that fail validation will cause the application to exit with an error

The terminal will always start, even if configuration loading fails, by falling back to safe defaults.
