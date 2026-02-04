# Mochi Terminal Themes

This document describes the theming system for Mochi Terminal.

## Built-in Themes

Mochi Terminal includes 6 built-in themes:

### Dark (default)
A VS Code-inspired dark theme with comfortable contrast for extended use.
- Background: `#1e1e1e`
- Foreground: `#d4d4d4`

### Light
A clean light theme with dark text on a light background.
- Background: `#ffffff`
- Foreground: `#000000`

### Solarized Dark
The popular Solarized color scheme in dark mode.
- Background: `#002b36`
- Foreground: `#839496`

### Solarized Light
The popular Solarized color scheme in light mode.
- Background: `#fdf6e3`
- Foreground: `#657b83`

### Dracula
The popular Dracula theme with vibrant colors.
- Background: `#282a36`
- Foreground: `#f8f8f2`

### Nord
A calm, arctic-inspired color palette.
- Background: `#2e3440`
- Foreground: `#d8dee9`

## Switching Themes

### Via Configuration File

Set the theme in your `~/.config/mochi/config.toml`:

```toml
theme = "dark"  # or "light", "solarized-dark", "solarized-light", "dracula", "nord"
```

### Via CLI Argument

```bash
mochi --theme light
```

### Via Environment Variable

```bash
MOCHI_THEME=dracula mochi
```

### At Runtime (Keybinding)

Press `Ctrl+Shift+T` to cycle through themes while the terminal is running. The current theme name is displayed in the window title.

Theme cycle order: Dark -> Light -> Solarized Dark -> Solarized Light -> Dracula -> Nord -> Dark

## Custom Themes

You can define a custom theme by setting `theme = "custom"` and providing colors in the `[colors]` section:

```toml
theme = "custom"

[colors]
foreground = "#c0c0c0"
background = "#1a1a2e"
cursor = "#ffffff"
selection = "#3d3d5c"
ansi = [
    "#000000",  # 0: Black
    "#ff5555",  # 1: Red
    "#50fa7b",  # 2: Green
    "#f1fa8c",  # 3: Yellow
    "#6272a4",  # 4: Blue
    "#ff79c6",  # 5: Magenta
    "#8be9fd",  # 6: Cyan
    "#bfbfbf",  # 7: White
    "#4d4d4d",  # 8: Bright Black
    "#ff6e6e",  # 9: Bright Red
    "#69ff94",  # 10: Bright Green
    "#ffffa5",  # 11: Bright Yellow
    "#d6acff",  # 12: Bright Blue
    "#ff92df",  # 13: Bright Magenta
    "#a4ffff",  # 14: Bright Cyan
    "#ffffff",  # 15: Bright White
]
```

## Color Format

All colors must be specified as 6-digit hexadecimal values with a `#` prefix:
- Valid: `#ff5555`, `#FF5555`, `#1e1e1e`
- Invalid: `ff5555`, `#f55`, `rgb(255, 85, 85)`

## Theme Components

Each theme defines the following colors:

| Component | Description |
|-----------|-------------|
| `foreground` | Default text color |
| `background` | Terminal background color |
| `cursor` | Cursor color |
| `selection` | Selection highlight background |
| `ansi[0-7]` | Standard ANSI colors (black, red, green, yellow, blue, magenta, cyan, white) |
| `ansi[8-15]` | Bright ANSI colors |

## ANSI Color Palette

The ANSI color palette consists of 16 colors:

| Index | Name | Typical Use |
|-------|------|-------------|
| 0 | Black | Background, dark elements |
| 1 | Red | Errors, deletions |
| 2 | Green | Success, additions |
| 3 | Yellow | Warnings, modifications |
| 4 | Blue | Information, directories |
| 5 | Magenta | Special items |
| 6 | Cyan | Strings, links |
| 7 | White | Normal text |
| 8-15 | Bright variants | Bold/bright versions of 0-7 |

## Testing Your Theme

To verify your theme colors are correct, you can run this command in the terminal:

```bash
for i in {0..15}; do printf "\e[48;5;${i}m  \e[0m"; done; echo
```

This displays all 16 ANSI colors as background blocks.

For a more comprehensive test:

```bash
for i in {0..15}; do printf "\e[38;5;${i}m Color $i \e[0m\n"; done
```

This displays text in each of the 16 ANSI colors.

## Known Limitations

- Custom themes cannot be hot-reloaded yet (requires restart)
- Theme files cannot be loaded from external paths (only inline in config.toml)
- The `Ctrl+Shift+T` keybinding cycles through built-in themes only; custom themes are skipped in the cycle
