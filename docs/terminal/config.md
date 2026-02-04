# Mochi Terminal Configuration

This document describes the configuration system for Mochi Terminal.

## Configuration File Location

Mochi Terminal follows XDG Base Directory conventions. The configuration file is loaded from:

```
~/.config/mochi/config.toml
```

You can override this location using the `--config` CLI flag or the `MOCHI_CONFIG` environment variable.

## Configuration Precedence

Configuration values are resolved with the following precedence (highest to lowest):

1. **CLI flags** - Command-line arguments like `--font-size 16`
2. **Environment variables** - Variables like `MOCHI_FONT_SIZE=16`
3. **Config file** - Values from `~/.config/mochi/config.toml`
4. **Built-in defaults** - Hardcoded default values

This means CLI flags always override environment variables, which override config file values, which override defaults.

## CLI Arguments

| Flag | Description | Example |
|------|-------------|---------|
| `-c, --config <FILE>` | Path to configuration file | `--config ~/my-config.toml` |
| `--font-size <SIZE>` | Font size in points | `--font-size 16` |
| `--font-family <FAMILY>` | Font family name | `--font-family "JetBrains Mono"` |
| `--theme <THEME>` | Theme name | `--theme dracula` |
| `--shell <SHELL>` | Shell command to run | `--shell /bin/zsh` |
| `--scrollback <LINES>` | Scrollback buffer size | `--scrollback 50000` |
| `--cols <COLS>` | Initial columns | `--cols 120` |
| `--rows <ROWS>` | Initial rows | `--rows 40` |

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `MOCHI_CONFIG` | Path to configuration file | `MOCHI_CONFIG=~/my-config.toml` |
| `MOCHI_FONT_SIZE` | Font size in points | `MOCHI_FONT_SIZE=16` |
| `MOCHI_FONT_FAMILY` | Font family name | `MOCHI_FONT_FAMILY="Fira Code"` |
| `MOCHI_THEME` | Theme name | `MOCHI_THEME=nord` |
| `MOCHI_SHELL` | Shell command | `MOCHI_SHELL=/bin/zsh` |
| `MOCHI_SCROLLBACK` | Scrollback lines | `MOCHI_SCROLLBACK=50000` |

## Configuration Options

### Font Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `font_family` | string | `"DejaVu Sans Mono"` | Font family name (must be monospace) |
| `font_size` | float | `14.0` | Font size in points (6.0 - 128.0) |

### Terminal Dimensions

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `dimensions` | [u16, u16] | `[80, 24]` | Initial [columns, rows] |

Constraints:
- Columns: 1-500
- Rows: 1-200

### Scrollback

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `scrollback_lines` | integer | `10000` | Lines in scrollback buffer (0 - 1,000,000) |

### Theme

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `theme` | string | `"dark"` | Theme name (see Built-in Themes) |

### Cursor

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `cursor_style` | string | `"block"` | Cursor style: `"block"`, `"underline"`, or `"bar"` |
| `cursor_blink` | boolean | `true` | Whether cursor should blink |

### Shell

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `shell` | string | (none) | Shell command to run. If not set, uses `$SHELL` or `/bin/sh` |

### Security

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `osc52_clipboard` | boolean | `false` | Enable OSC 52 clipboard sequences |
| `osc52_max_size` | integer | `100000` | Max bytes for OSC 52 payloads |

**Security Warning:** Enabling `osc52_clipboard` allows programs running in the terminal to read and write your system clipboard via escape sequences. Only enable this if you trust all programs you run.

### Custom Colors

When `theme = "custom"`, you can define your own color scheme:

```toml
[colors]
foreground = "#d4d4d4"
background = "#1e1e1e"
cursor = "#d4d4d4"
selection = "#264f78"

# ANSI 16-color palette (in order):
# black, red, green, yellow, blue, magenta, cyan, white,
# bright_black, bright_red, bright_green, bright_yellow,
# bright_blue, bright_magenta, bright_cyan, bright_white
ansi = [
    "#1e1e1e", "#f44747", "#6a9955", "#dcdcaa",
    "#569cd6", "#c586c0", "#4ec9b0", "#d4d4d4",
    "#808080", "#f44747", "#6a9955", "#dcdcaa",
    "#569cd6", "#c586c0", "#4ec9b0", "#ffffff",
]
```

All colors must be in `#RRGGBB` hex format.

## Built-in Themes

Mochi Terminal includes the following built-in themes:

| Theme | Description |
|-------|-------------|
| `dark` | VS Code inspired dark theme (default) |
| `light` | Clean light theme with high contrast |
| `solarized-dark` | Solarized dark variant |
| `solarized-light` | Solarized light variant |
| `dracula` | Popular Dracula color scheme |
| `nord` | Arctic, north-bluish color palette |
| `custom` | Use colors defined in `[colors]` section |

Theme names are case-insensitive. You can also use `solarizeddark` instead of `solarized-dark`.

## Example Configuration

See [config.example.toml](config.example.toml) for a fully commented example configuration file.

## Validation

Mochi Terminal validates configuration on load and provides clear error messages for invalid values:

- Font size must be between 6.0 and 128.0 points
- Scrollback lines must be between 0 and 1,000,000
- Dimensions must be 1-500 columns and 1-200 rows
- Colors must be valid `#RRGGBB` hex format

If validation fails, Mochi will display an error message and fall back to default values.

## Troubleshooting

### Config file not loading

1. Check the file exists at `~/.config/mochi/config.toml`
2. Verify the file has valid TOML syntax
3. Run with `RUST_LOG=debug mochi` to see config loading logs

### Invalid configuration

If you see validation errors:

1. Check the error message for the specific invalid value
2. Refer to the constraints in this document
3. Fix the value and restart Mochi

### Environment variables not working

Environment variables must be set before launching Mochi:

```bash
export MOCHI_FONT_SIZE=16
mochi
```

Or inline:

```bash
MOCHI_FONT_SIZE=16 mochi
```
