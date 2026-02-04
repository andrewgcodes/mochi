# Mochi Terminal Themes

Mochi Terminal includes a comprehensive theming system that allows you to customize the appearance of your terminal.

## Built-in Themes

Mochi comes with 6 built-in themes:

### Dark Themes
- **dark** (default) - VS Code inspired dark theme with comfortable contrast
- **solarized-dark** - Ethan Schoonover's Solarized dark palette
- **dracula** - Popular dark theme with vibrant colors
- **nord** - Arctic, north-bluish color palette

### Light Themes
- **light** - Clean light theme for bright environments
- **solarized-light** - Ethan Schoonover's Solarized light palette

## Selecting a Theme

### Via Configuration File

Add to your `~/.config/mochi/config.toml`:

```toml
theme = "dracula"
```

### Via Command Line

```bash
mochi --theme nord
```

### Via Environment Variable

```bash
export MOCHI_THEME=solarized-dark
mochi
```

### Runtime Theme Switching

Press `Ctrl+Shift+T` to cycle through available themes without restarting the terminal.

## Theme Format

Each theme defines the following colors (in hex format):

| Property | Description |
|----------|-------------|
| `foreground` | Default text color |
| `background` | Terminal background color |
| `cursor` | Cursor color |
| `selection` | Selection highlight color |
| `ansi[0-15]` | ANSI color palette (16 colors) |

### ANSI Color Indices

| Index | Normal | Index | Bright |
|-------|--------|-------|--------|
| 0 | Black | 8 | Bright Black |
| 1 | Red | 9 | Bright Red |
| 2 | Green | 10 | Bright Green |
| 3 | Yellow | 11 | Bright Yellow |
| 4 | Blue | 12 | Bright Blue |
| 5 | Magenta | 13 | Bright Magenta |
| 6 | Cyan | 14 | Bright Cyan |
| 7 | White | 15 | Bright White |

## Custom Themes

To create a custom theme, set `theme = "custom"` and define your colors:

```toml
theme = "custom"

[colors]
foreground = "#c0c0c0"
background = "#1a1a2e"
cursor = "#ffffff"
selection = "#3d3d5c"
ansi = [
    "#1a1a2e",  # Black
    "#ff6b6b",  # Red
    "#4ecdc4",  # Green
    "#ffe66d",  # Yellow
    "#4a90d9",  # Blue
    "#c44fc4",  # Magenta
    "#72d6c9",  # Cyan
    "#f0f0f0",  # White
    "#4a4a6a",  # Bright Black
    "#ff8585",  # Bright Red
    "#6ee6dd",  # Bright Green
    "#fff085",  # Bright Yellow
    "#6aa8f0",  # Bright Blue
    "#dc6bdc",  # Bright Magenta
    "#8aeee1",  # Bright Cyan
    "#ffffff",  # Bright White
]
```

## Theme Precedence

Theme selection follows the standard configuration precedence:

1. CLI flag (`--theme`)
2. Environment variable (`MOCHI_THEME`)
3. Config file (`theme = "..."`)
4. Default (`dark`)

## Color Validation

All colors must be valid 6-digit hex codes (with or without `#` prefix). Invalid colors will cause a configuration error with a helpful message.

## Screenshots

See the `docs/terminal/phase2/screenshots/` directory for visual examples of each theme.
