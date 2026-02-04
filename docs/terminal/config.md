# Mochi Terminal Configuration

This document describes the configuration system for Mochi Terminal.

## Configuration File Location

Mochi Terminal follows XDG Base Directory conventions. The default configuration file location is:

```
~/.config/mochi/config.toml
```

You can override this location using the `--config` CLI flag:

```bash
mochi --config /path/to/custom/config.toml
```

## Configuration Precedence

Configuration values are resolved in the following order (highest priority first):

1. **CLI flags** - Command-line arguments override all other settings
2. **Environment variables** - `MOCHI_*` environment variables
3. **Config file** - Values from the TOML configuration file
4. **Built-in defaults** - Hardcoded default values

## CLI Arguments

```
mochi [OPTIONS]

Options:
  -c, --config <FILE>       Path to config file
      --font-family <FAMILY> Font family name
      --font-size <SIZE>    Font size in points
      --theme <THEME>       Theme name (dark, light, solarized-dark, solarized-light, dracula, nord, gruvbox)
      --cols <COLS>         Initial columns
      --rows <ROWS>         Initial rows
      --shell <SHELL>       Shell command to run
      --enable-osc52        Enable OSC 52 clipboard support (security risk)
  -h, --help                Print help
  -V, --version             Print version
```

## Environment Variables

The following environment variables are supported:

| Variable | Description | Example |
|----------|-------------|---------|
| `MOCHI_FONT_FAMILY` | Font family name | `MOCHI_FONT_FAMILY="JetBrains Mono"` |
| `MOCHI_FONT_SIZE` | Font size in points | `MOCHI_FONT_SIZE=16` |
| `MOCHI_THEME` | Theme name | `MOCHI_THEME=nord` |
| `MOCHI_SHELL` | Shell command | `MOCHI_SHELL=/bin/zsh` |
| `MOCHI_OSC52_CLIPBOARD` | Enable OSC 52 clipboard | `MOCHI_OSC52_CLIPBOARD=true` |

## Configuration Options

### Font Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `font_family` | string | `"monospace"` | Font family name |
| `font_size` | float | `14.0` | Font size in points (4.0 - 144.0) |

### Terminal Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `dimensions` | [u16, u16] | `[80, 24]` | Initial terminal size (columns, rows) |
| `scrollback_lines` | integer | `10000` | Number of lines to keep in scrollback (max 1,000,000) |
| `shell` | string | `null` | Shell command (defaults to `$SHELL`) |

### Appearance

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `theme` | string | `"dark"` | Theme name (see below) |
| `cursor_style` | string | `"block"` | Cursor style: `block`, `underline`, `bar` |
| `cursor_blink` | boolean | `true` | Enable cursor blinking |

### Security

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `osc52_clipboard` | boolean | `false` | Enable OSC 52 clipboard sequences |
| `osc52_max_size` | integer | `100000` | Maximum OSC 52 payload size in bytes |

## Built-in Themes

Mochi Terminal includes the following built-in themes:

| Theme | Description |
|-------|-------------|
| `dark` | VS Code-inspired dark theme (default) |
| `light` | Light theme with high contrast |
| `solarized-dark` | Solarized dark color scheme |
| `solarized-light` | Solarized light color scheme |
| `dracula` | Dracula color scheme |
| `nord` | Nord color scheme |
| `gruvbox` | Gruvbox dark color scheme |

## Custom Colors

To use custom colors, set `theme = "custom"` and define your colors in the `[colors]` section:

```toml
theme = "custom"

[colors]
foreground = "#d4d4d4"
background = "#1e1e1e"
cursor = "#ffffff"
selection = "#264f78"

# ANSI 16-color palette
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

All colors must be specified as 6-digit hex values with a `#` prefix.

## Validation

The configuration is validated on load. Invalid configurations will cause the terminal to exit with an error message. Validation rules:

- `font_size` must be between 4.0 and 144.0
- `dimensions[0]` (columns) must be between 10 and 1000
- `dimensions[1]` (rows) must be between 5 and 500
- `scrollback_lines` must be at most 1,000,000
- All color values must be valid 6-digit hex colors

## Example Configuration

See [config.example.toml](config.example.toml) for a complete example configuration file.
