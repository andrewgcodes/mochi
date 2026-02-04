# Mochi Terminal Themes

This document describes the theming system for Mochi Terminal.

## Built-in Themes

Mochi Terminal includes 6 built-in themes:

| Theme Name | Description |
|------------|-------------|
| `dark` | VS Code inspired dark theme (default) |
| `light` | VS Code inspired light theme |
| `solarized-dark` | Solarized dark color scheme |
| `solarized-light` | Solarized light color scheme |
| `dracula` | Dracula color scheme |
| `nord` | Nord color scheme |

## Selecting a Theme

### Via Configuration File

Set the `theme` option in your config file (`~/.config/mochi/config.toml`):

```toml
theme = "dracula"
```

### Via CLI

Use the `--theme` flag:

```bash
mochi --theme light
```

### Via Environment Variable

Set the `MOCHI_THEME` environment variable:

```bash
MOCHI_THEME=nord mochi
```

### Runtime Theme Switching

Press `Ctrl+Shift+T` to cycle through themes at runtime. The cycle order is:
dark -> light -> solarized-dark -> solarized-light -> dracula -> nord -> dark

## Custom Themes

### Inline Custom Colors

Set `theme = "custom"` and define colors in the `[colors]` section:

```toml
theme = "custom"

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

### External Theme Files

You can also load themes from external TOML files using the `theme_file` option:

```toml
theme_file = "my-theme.toml"
```

Theme files are searched in the following order:
1. Absolute path (if the path starts with `/`)
2. `~/.config/mochi/themes/` directory
3. Current directory

### Theme File Format

A theme file should contain the same structure as the `[colors]` section:

```toml
# my-theme.toml
foreground = "#f8f8f2"
background = "#282a36"
cursor = "#f8f8f2"
selection = "#44475a"
ansi = [
    "#21222c",
    "#ff5555",
    "#50fa7b",
    "#f1fa8c",
    "#bd93f9",
    "#ff79c6",
    "#8be9fd",
    "#f8f8f2",
    "#6272a4",
    "#ff6e6e",
    "#69ff94",
    "#ffffa5",
    "#d6acff",
    "#ff92df",
    "#a4ffff",
    "#ffffff",
]
```

## Color Format

All colors are specified as 6-digit hex strings, with or without the `#` prefix:

- `#ff0000` - Red
- `ff0000` - Also valid (without #)
- `#FF0000` - Case insensitive

## ANSI Color Indices

The `ansi` array contains 16 colors in the following order:

| Index | Color | Description |
|-------|-------|-------------|
| 0 | Black | Normal black |
| 1 | Red | Normal red |
| 2 | Green | Normal green |
| 3 | Yellow | Normal yellow |
| 4 | Blue | Normal blue |
| 5 | Magenta | Normal magenta |
| 6 | Cyan | Normal cyan |
| 7 | White | Normal white |
| 8 | Bright Black | Bright/bold black (gray) |
| 9 | Bright Red | Bright/bold red |
| 10 | Bright Green | Bright/bold green |
| 11 | Bright Yellow | Bright/bold yellow |
| 12 | Bright Blue | Bright/bold blue |
| 13 | Bright Magenta | Bright/bold magenta |
| 14 | Bright Cyan | Bright/bold cyan |
| 15 | Bright White | Bright/bold white |

## Theme Precedence

When determining which colors to use, the following precedence applies:

1. `theme_file` - If specified and valid, loads colors from the file
2. `theme = "custom"` - Uses colors from the `[colors]` section
3. Built-in theme - Uses the specified built-in theme's colors

If a theme file fails to load (file not found, parse error, invalid colors), Mochi falls back to the built-in theme specified by `theme`.

## Creating a Theme

To create a new theme:

1. Create a new file in `~/.config/mochi/themes/` (e.g., `my-theme.toml`)
2. Define all required colors (foreground, background, cursor, selection, ansi)
3. Reference it in your config: `theme_file = "my-theme.toml"`

### Tips for Theme Creation

- Use a color contrast checker to ensure readability
- Test with various applications (vim, htop, etc.) to verify ANSI colors
- Consider both light and dark backgrounds for selection visibility
- The cursor color should contrast well with both foreground and background

## Keybindings

| Keybinding | Action |
|------------|--------|
| `Ctrl+Shift+T` | Cycle to next theme |
| `Ctrl+Shift+R` | Reload configuration (applies theme changes) |
