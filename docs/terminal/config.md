# Mochi Terminal Configuration

This document describes the configuration system for Mochi Terminal.

## Configuration File Location

Mochi follows the XDG Base Directory Specification. The default configuration file location is:

```
~/.config/mochi/config.toml
```

You can override this location using the `--config` CLI flag:

```bash
mochi --config /path/to/custom/config.toml
```

## Configuration Precedence

Configuration values are loaded with the following precedence (highest to lowest):

1. **CLI arguments** - Command-line flags override all other settings
2. **Environment variables** - `MOCHI_*` environment variables
3. **Configuration file** - Values from `config.toml`
4. **Built-in defaults** - Sensible defaults for all settings

## CLI Arguments

```
USAGE:
    mochi [OPTIONS]

OPTIONS:
    -c, --config <FILE>       Path to configuration file
        --font-family <FONT>  Font family name
        --font-size <SIZE>    Font size in points
    -t, --theme <THEME>       Theme name (dark, light, solarized-dark, solarized-light, dracula, nord)
    -s, --shell <SHELL>       Shell command to run
        --columns <COLS>      Initial columns
        --rows <ROWS>         Initial rows
        --scrollback <LINES>  Number of scrollback lines
        --osc52-clipboard     Enable OSC 52 clipboard support
    -h, --help                Print help
    -V, --version             Print version
```

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `MOCHI_FONT_FAMILY` | Font family name | `"JetBrains Mono"` |
| `MOCHI_FONT_SIZE` | Font size in points | `16` |
| `MOCHI_THEME` | Theme name | `dark`, `light`, `dracula` |
| `MOCHI_SHELL` | Shell command | `/bin/zsh` |
| `MOCHI_SCROLLBACK` | Scrollback lines | `50000` |
| `MOCHI_COLUMNS` | Initial columns | `120` |
| `MOCHI_ROWS` | Initial rows | `40` |
| `MOCHI_OSC52_CLIPBOARD` | Enable OSC 52 | `true` or `1` |

## Configuration Options

### Font Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `font_family` | string | `"monospace"` | Font family name |
| `font_size` | float | `14.0` | Font size in points (4.0 - 128.0) |

### Terminal Dimensions

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `dimensions` | [u16, u16] | `[80, 24]` | Initial [columns, rows] |
| `scrollback_lines` | usize | `10000` | Lines in scrollback buffer (max 1,000,000) |

### Theme

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `theme` | string | `"dark"` | Theme name (see below) |

Available themes:
- `dark` - VS Code inspired dark theme
- `light` - Light theme with dark text
- `solarized-dark` - Solarized Dark color scheme
- `solarized-light` - Solarized Light color scheme
- `dracula` - Dracula color scheme
- `nord` - Nord color scheme
- `custom` - Use colors from `[colors]` section

### Shell

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `shell` | string | `$SHELL` | Shell command to run |

### Cursor

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `cursor_style` | string | `"block"` | Cursor style: `block`, `underline`, `bar` |
| `cursor_blink` | bool | `true` | Enable cursor blinking |

### Security

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `osc52_clipboard` | bool | `false` | Enable OSC 52 clipboard (disabled for security) |
| `osc52_max_size` | usize | `100000` | Maximum OSC 52 payload size in bytes |

### Custom Colors

When `theme = "custom"`, the `[colors]` section defines the color scheme:

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

## Validation

The configuration system validates all values and provides clear error messages:

- Font size must be between 4.0 and 128.0
- Columns must be between 10 and 1000
- Rows must be between 5 and 500
- Scrollback lines must not exceed 1,000,000
- Theme name must be valid
- Color values must be valid hex strings

If validation fails, Mochi will display an error message and exit.

## Example Configuration

See [config.example.toml](config.example.toml) for a complete example with all options documented.

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

4. Run Mochi:
   ```bash
   mochi
   ```
