# Mochi Terminal Configuration Guide

This document describes the configuration system for Mochi Terminal, including all available options, their defaults, and how to customize them.

## Configuration File Location

Mochi Terminal follows the XDG Base Directory Specification. The default configuration file location is:

```
~/.config/mochi/config.toml
```

You can override this with the `--config` command-line flag:

```bash
mochi --config /path/to/custom/config.toml
```

## Configuration Precedence

Configuration values are resolved in the following order (highest priority first):

1. **Command-line arguments** (`--font-size 16`, `--theme dark`, etc.)
2. **Environment variables** (`MOCHI_FONT_SIZE=16`, `MOCHI_THEME=dark`, etc.)
3. **Configuration file** (`~/.config/mochi/config.toml`)
4. **Built-in defaults**

## Command-Line Arguments

| Argument | Description | Example |
|----------|-------------|---------|
| `-c, --config <FILE>` | Path to config file | `--config ~/.config/mochi/custom.toml` |
| `--font-family <FONT>` | Font family name | `--font-family "JetBrains Mono"` |
| `--font-size <SIZE>` | Font size in points | `--font-size 16` |
| `-t, --theme <THEME>` | Theme name | `--theme dracula` |
| `-s, --shell <SHELL>` | Shell command | `--shell /bin/zsh` |
| `--scrollback <LINES>` | Scrollback lines | `--scrollback 50000` |
| `--columns <COLS>` | Initial columns | `--columns 120` |
| `--rows <ROWS>` | Initial rows | `--rows 40` |
| `--enable-osc52` | Enable OSC 52 clipboard | `--enable-osc52` |

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `MOCHI_FONT_FAMILY` | Font family name | `export MOCHI_FONT_FAMILY="Fira Code"` |
| `MOCHI_FONT_SIZE` | Font size in points | `export MOCHI_FONT_SIZE=14` |
| `MOCHI_THEME` | Theme name | `export MOCHI_THEME=light` |
| `MOCHI_SHELL` | Shell command | `export MOCHI_SHELL=/bin/fish` |
| `MOCHI_SCROLLBACK` | Scrollback lines | `export MOCHI_SCROLLBACK=20000` |
| `MOCHI_OSC52_CLIPBOARD` | Enable OSC 52 | `export MOCHI_OSC52_CLIPBOARD=true` |

## Configuration File Schema

### General Settings

```toml
# Number of lines to keep in scrollback history
# Default: 10000, Range: 0 - 10,000,000
scrollback_lines = 10000

# Initial terminal dimensions [columns, rows]
# Default: [80, 24]
dimensions = [80, 24]

# Shell command (optional, defaults to $SHELL)
shell = "/bin/bash"

# Cursor style: "block", "underline", or "bar"
# Default: "block"
cursor_style = "block"

# Enable cursor blinking
# Default: true
cursor_blink = true
```

### Theme Settings

```toml
# Theme name
# Options: "mochi", "dark", "light", "solarized-dark", "solarized-light", "dracula", "nord", "custom"
# Default: "mochi"
theme = "mochi"
```

#### Built-in Themes

| Theme | Description |
|-------|-------------|
| `mochi` | Cute pink kawaii theme (default) |
| `dark` | VS Code-inspired dark theme |
| `light` | Clean light theme |
| `solarized-dark` | Solarized dark color scheme |
| `solarized-light` | Solarized light color scheme |
| `dracula` | Popular Dracula theme |
| `nord` | Arctic, north-bluish color palette |
| `custom` | Use colors defined in `[colors]` section |

#### Custom Colors

When `theme = "custom"`, you can define your own color scheme:

```toml
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
    "#ffffff"   # 15: Bright White
]
```

### Font Settings

```toml
[font]
# Font family name
# Default: "monospace"
family = "monospace"

# Font size in points
# Default: 14.0, Range: 4.0 - 200.0
size = 14.0

# Fallback fonts (tried in order)
fallbacks = [
    "DejaVu Sans Mono",
    "Liberation Mono",
    "Courier New"
]

# Cell padding in pixels
cell_padding_x = 0
cell_padding_y = 0

# Line height multiplier
# Default: 1.0, Range: 0.5 - 3.0
line_height = 1.0
```

### Keybindings

```toml
[keybindings]
# Format: "modifier+modifier+key"
# Modifiers: ctrl, shift, alt, super

copy = "ctrl+shift+c"
paste = "ctrl+shift+v"
find = "ctrl+shift+f"
reload_config = "ctrl+shift+r"
toggle_theme = "ctrl+shift+t"
zoom_in = "ctrl+plus"
zoom_out = "ctrl+minus"
zoom_reset = "ctrl+0"
```

### Security Settings

```toml
[security]
# Enable OSC 52 clipboard sequences
# WARNING: Security risk - programs can access clipboard
# Default: false
osc52_clipboard = false

# Maximum OSC 52 payload size in bytes
# Default: 100000
osc52_max_size = 100000

# Show notification when clipboard is modified
# Default: true
osc52_notify = true

# Maximum title updates per second
# Default: 10
title_update_rate = 10
```

## Runtime Controls

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Shift+C` | Copy selection to clipboard |
| `Ctrl+Shift+V` | Paste from clipboard |
| `Ctrl+Shift+F` | Open search bar |
| `Ctrl+Shift+R` | Reload configuration |
| `Ctrl+Shift+T` | Toggle/cycle theme |
| `Ctrl++` / `Ctrl+=` | Zoom in (increase font size) |
| `Ctrl+-` | Zoom out (decrease font size) |
| `Ctrl+0` | Reset zoom to default |

### Theme Cycling

Press `Ctrl+Shift+T` to cycle through themes in this order:
1. Mochi
2. Dark
3. Light
4. Solarized Dark
5. Solarized Light
6. Dracula
7. Nord
8. (back to Mochi)

### Configuration Reload

Press `Ctrl+Shift+R` to reload the configuration file without restarting the terminal. This applies changes to:
- Theme/colors
- Font settings
- Keybindings
- Security settings

Note: Some settings (like initial dimensions) only take effect on startup.

## Validation

The configuration system validates all values and provides helpful error messages:

- Font size must be between 4.0 and 200.0
- Dimensions must be at least 10 columns and 3 rows
- Scrollback must be at most 10,000,000 lines
- Line height must be between 0.5 and 3.0
- All color values must be valid hex format (#RRGGBB)

If validation fails, Mochi will display an error message and exit. Fix the configuration file and try again.

## Example Configurations

### Minimal Dark Setup

```toml
theme = "dark"

[font]
size = 14.0
```

### Developer Setup with Large Font

```toml
theme = "dracula"
scrollback_lines = 50000

[font]
family = "JetBrains Mono"
size = 16.0
line_height = 1.2
```

### Light Theme for Presentations

```toml
theme = "light"

[font]
family = "Fira Code"
size = 24.0
```

### Security-Conscious Setup

```toml
theme = "nord"

[security]
osc52_clipboard = false
osc52_notify = true
title_update_rate = 5
```
