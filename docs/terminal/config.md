# Mochi Terminal Configuration

This document describes the configuration system for Mochi Terminal.

## Configuration File Location

Mochi Terminal follows XDG Base Directory conventions. The default configuration file location is:

```
~/.config/mochi/config.toml
```

You can override this with the `--config` CLI flag:

```bash
mochi --config /path/to/custom/config.toml
```

## Configuration Precedence

Configuration values are loaded with the following precedence (highest to lowest):

1. **CLI arguments** - Command-line flags override all other settings
2. **Environment variables** - `MOCHI_*` environment variables
3. **Config file** - Values from the TOML config file
4. **Built-in defaults** - Hardcoded default values

## CLI Arguments

```
mochi [OPTIONS]

Options:
  -c, --config <FILE>      Path to configuration file
      --font-size <SIZE>   Font size in points
  -t, --theme <THEME>      Theme name (dark, light, solarized-dark, solarized-light, dracula, nord)
      --shell <SHELL>      Shell command to run
      --scrollback <LINES> Number of scrollback lines
      --cols <COLS>        Initial window columns
      --rows <ROWS>        Initial window rows
  -h, --help               Print help
  -V, --version            Print version
```

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `MOCHI_FONT_SIZE` | Font size in points | `MOCHI_FONT_SIZE=16` |
| `MOCHI_THEME` | Theme name | `MOCHI_THEME=light` |
| `MOCHI_SHELL` | Shell command | `MOCHI_SHELL=/bin/zsh` |
| `MOCHI_SCROLLBACK` | Scrollback lines | `MOCHI_SCROLLBACK=50000` |
| `MOCHI_OSC52_CLIPBOARD` | Enable OSC 52 clipboard | `MOCHI_OSC52_CLIPBOARD=true` |

## Configuration Schema

### Font Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `font_family` | string | `"monospace"` | Font family name |
| `font_size` | float | `14.0` | Font size in points (6.0-128.0) |

### Window Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `dimensions` | [u16, u16] | `[80, 24]` | Initial window size [columns, rows] |
| `scrollback_lines` | usize | `10000` | Lines to keep in scrollback (0-1000000) |

### Theme Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `theme` | string | `"dark"` | Theme name (see below) |

Available themes:
- `dark` - VS Code inspired dark theme (default)
- `light` - VS Code inspired light theme
- `solarized-dark` - Solarized dark color scheme
- `solarized-light` - Solarized light color scheme
- `dracula` - Dracula color scheme
- `nord` - Nord color scheme
- `custom` - Use custom colors from `[colors]` section

### Custom Colors

When `theme = "custom"`, the `[colors]` section is used:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `foreground` | string | `"#d4d4d4"` | Default text color |
| `background` | string | `"#1e1e1e"` | Background color |
| `cursor` | string | `"#ffffff"` | Cursor color |
| `selection` | string | `"#264f78"` | Selection highlight color |
| `ansi` | [string; 16] | (see below) | ANSI 16-color palette |

ANSI color indices:
- 0-7: Normal colors (black, red, green, yellow, blue, magenta, cyan, white)
- 8-15: Bright colors (bright black through bright white)

### Cursor Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `cursor_style` | string | `"block"` | Cursor style: "block", "underline", "bar" |
| `cursor_blink` | bool | `true` | Enable cursor blinking |

### Shell Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `shell` | string | (none) | Shell command (uses $SHELL if not set) |

### Security Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `osc52_clipboard` | bool | `false` | Enable OSC 52 clipboard sequences |
| `osc52_max_size` | usize | `100000` | Max OSC 52 payload size in bytes |

## Validation

The configuration system validates all values and provides clear error messages:

- `font_size` must be between 6.0 and 128.0
- `dimensions[0]` (columns) must be between 10 and 1000
- `dimensions[1]` (rows) must be between 5 and 500
- `scrollback_lines` must be at most 1,000,000
- `osc52_max_size` must be at most 10,000,000
- All color values must be valid 6-digit hex colors (with or without `#` prefix)

If validation fails, Mochi will display an error message and exit.

## Example Configuration

See [config.example.toml](config.example.toml) for a fully commented example configuration file.

## Quick Start

1. Copy the example config:
   ```bash
   mkdir -p ~/.config/mochi
   cp docs/terminal/config.example.toml ~/.config/mochi/config.toml
   ```

2. Edit to your preferences:
   ```bash
   $EDITOR ~/.config/mochi/config.toml
   ```

3. Run Mochi:
   ```bash
   mochi
   ```

Or use CLI flags for quick testing:
```bash
mochi --theme light --font-size 16
```
