# Mochi Terminal Configuration Guide

This document describes the configuration system for Mochi Terminal.

## Configuration File Location

Mochi Terminal looks for configuration in the following locations (in order):

1. Path specified by `--config` CLI flag
2. Path specified by `MOCHI_CONFIG` environment variable
3. `$XDG_CONFIG_HOME/mochi/config.toml` (if XDG_CONFIG_HOME is set)
4. `~/.config/mochi/config.toml` (default)

If no configuration file is found, built-in defaults are used.

## Configuration Precedence

Configuration values are applied with the following precedence (highest to lowest):

1. **CLI flags** - Command line arguments override everything
2. **Environment variables** - Override config file and defaults
3. **Config file** - Override built-in defaults
4. **Built-in defaults** - Used when no other value is specified

## CLI Arguments

```
mochi [OPTIONS]

OPTIONS:
    -c, --config <PATH>     Use custom config file path
    --font-size <SIZE>      Override font size (e.g., 14.0)
    -t, --theme <THEME>     Override theme (dark, light, solarized-dark,
                            solarized-light, dracula, nord)
    -e, --shell <COMMAND>   Override shell command
    -h, --help              Print help information
    -v, --version           Print version information
```

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `MOCHI_CONFIG` | Path to config file | `/path/to/config.toml` |
| `MOCHI_FONT_SIZE` | Font size override | `16.0` |
| `MOCHI_THEME` | Theme override | `dracula` |
| `MOCHI_SHELL` | Shell command override | `/bin/zsh` |

## Configuration Schema

### Font Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `font_family` | string | `"monospace"` | Font family name |
| `font_size` | float | `14.0` | Font size in points (6.0-72.0) |

### Window Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `dimensions` | [u16, u16] | `[80, 24]` | Initial window size [cols, rows] |
| `scrollback_lines` | usize | `10000` | Lines to keep in scrollback |

### Theme Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `theme` | string | `"dark"` | Theme name (see below) |

Available themes:
- `dark` - VS Code inspired dark theme
- `light` - VS Code inspired light theme
- `solarizeddark` - Solarized Dark
- `solarizedlight` - Solarized Light
- `dracula` - Dracula theme
- `nord` - Nord theme
- `custom` - Use colors from `[colors]` section

### Cursor Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `cursor_style` | string | `"block"` | Cursor style: block, underline, beam |
| `cursor_blink` | bool | `true` | Enable cursor blinking |

### Shell Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `shell` | string | `null` | Shell command (uses $SHELL if not set) |

### Security Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `osc52_clipboard` | bool | `false` | Enable OSC 52 clipboard sequences |
| `osc52_max_size` | usize | `100000` | Max OSC 52 payload size in bytes |

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

All colors are specified as hex RGB values (with or without `#` prefix).

The `ansi` array contains the 16 ANSI colors in order:
- 0-7: Normal colors (black, red, green, yellow, blue, magenta, cyan, white)
- 8-15: Bright colors

## Validation

The configuration is validated when loaded. Invalid configurations will produce clear error messages:

- Font size must be between 6.0 and 72.0
- All color values must be valid hex RGB (e.g., `#ff0000` or `ff0000`)
- Dimensions must be at least 10x5
- Scrollback lines must be greater than 0

## Example Configuration

See [config.example.toml](config.example.toml) for a fully commented example configuration file.

## Quick Start

1. Create the config directory:
   ```bash
   mkdir -p ~/.config/mochi
   ```

2. Copy the example config:
   ```bash
   cp docs/terminal/config.example.toml ~/.config/mochi/config.toml
   ```

3. Edit to your preferences:
   ```bash
   $EDITOR ~/.config/mochi/config.toml
   ```

4. Run Mochi Terminal:
   ```bash
   mochi
   ```

Or override settings via CLI:
```bash
mochi --theme dracula --font-size 16
```
