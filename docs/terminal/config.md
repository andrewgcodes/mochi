# Mochi Terminal Configuration

This document describes the configuration system for Mochi Terminal.

## Configuration File Location

Mochi Terminal follows the XDG Base Directory Specification for configuration files:

- **Default location**: `~/.config/mochi/config.toml`
- **XDG override**: If `XDG_CONFIG_HOME` is set, uses `$XDG_CONFIG_HOME/mochi/config.toml`
- **CLI override**: Use `--config /path/to/config.toml` to specify a custom location

## Configuration Precedence

Configuration values are loaded with the following precedence (highest to lowest):

1. **CLI flags** (`--font-size`, `--theme`, etc.)
2. **Environment variables** (`MOCHI_FONT_SIZE`, `MOCHI_THEME`, etc.)
3. **Config file** (`~/.config/mochi/config.toml`)
4. **Built-in defaults**

This means CLI flags always win, followed by environment variables, then the config file, and finally the built-in defaults for any unspecified values.

## Environment Variables

The following environment variables can be used to override configuration:

| Variable | Description | Example |
|----------|-------------|---------|
| `MOCHI_FONT_SIZE` | Font size in points | `MOCHI_FONT_SIZE=16.0` |
| `MOCHI_THEME` | Theme name | `MOCHI_THEME=light` |
| `MOCHI_SHELL` | Shell command | `MOCHI_SHELL=/bin/zsh` |
| `MOCHI_SCROLLBACK_LINES` | Scrollback buffer size | `MOCHI_SCROLLBACK_LINES=50000` |
| `MOCHI_OSC52_CLIPBOARD` | Enable OSC 52 clipboard | `MOCHI_OSC52_CLIPBOARD=true` |

## CLI Arguments

```
mochi [OPTIONS]

OPTIONS:
    -c, --config <PATH>     Path to config file
    -f, --font-size <SIZE>  Font size in points
    -t, --theme <THEME>     Theme name
    -s, --shell <SHELL>     Shell command to run
    -h, --help              Print help information
    -V, --version           Print version information
```

## Configuration Options

### Font Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `font_family` | string | `"monospace"` | Font family name |
| `font_size` | float | `14.0` | Font size in points (6.0-128.0) |

The terminal uses the bundled DejaVu Sans Mono font as a fallback if the specified font family is not found.

### Terminal Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `scrollback_lines` | integer | `10000` | Lines to keep in scrollback (max 1,000,000) |
| `dimensions` | [cols, rows] | `[80, 24]` | Initial terminal dimensions |
| `shell` | string | `$SHELL` | Shell command to run |
| `cursor_style` | string | `"block"` | Cursor style: "block", "underline", or "bar" |
| `cursor_blink` | boolean | `true` | Enable cursor blinking |

### Theme Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `theme` | string | `"dark"` | Theme name (see below) |

Available built-in themes:
- `dark` - VS Code-inspired dark theme (default)
- `light` - Clean light theme
- `solarized-dark` - Solarized dark color scheme
- `solarized-light` - Solarized light color scheme
- `dracula` - Dracula color scheme
- `nord` - Nord color scheme
- `custom` - Use the `[colors]` section for custom colors

### Custom Colors

When `theme = "custom"`, the `[colors]` section is used:

```toml
[colors]
foreground = "#d4d4d4"
background = "#1e1e1e"
cursor = "#ffffff"
selection = "#264f78"
ansi = [
    "#000000", "#cd3131", "#0dbc79", "#e5e510",
    "#2472c8", "#bc3fbc", "#11a8cd", "#e5e5e5",
    "#666666", "#f14c4c", "#23d18b", "#f5f543",
    "#3b8eea", "#d670d6", "#29b8db", "#ffffff"
]
```

The `ansi` array contains the 16 standard terminal colors in order:
0-7: black, red, green, yellow, blue, magenta, cyan, white
8-15: bright versions of the above

### Security Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `osc52_clipboard` | boolean | `false` | Enable OSC 52 clipboard sequences |
| `osc52_max_size` | integer | `100000` | Maximum OSC 52 payload size in bytes |

**Security Warning**: OSC 52 clipboard sequences allow programs running in the terminal to read and write your system clipboard. This is disabled by default for security. Only enable if you trust all programs you run in the terminal.

## Validation

The configuration is validated on load. Invalid configurations will produce clear error messages:

- Font size must be between 6.0 and 128.0
- Scrollback lines must be at most 1,000,000
- OSC 52 max size must be at most 10,000,000
- All color values must be valid 6-digit hex codes
- Dimensions must be at least 1x1

If validation fails, the terminal will fall back to default configuration and log a warning.

## Example Configuration

See [config.example.toml](config.example.toml) for a fully commented example configuration file.

## Runtime Configuration

Some configuration can be changed at runtime without restarting:

- **Font size**: Use Ctrl+=/- to zoom in/out, Ctrl+0 to reset
- **Theme**: Use Ctrl+Shift+T to cycle through themes
- **Reload config**: Use Ctrl+Shift+R to reload the configuration file

Changes to the configuration file can be applied without restarting by pressing Ctrl+Shift+R.
