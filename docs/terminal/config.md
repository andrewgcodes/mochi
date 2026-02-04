# Mochi Terminal Configuration

This document describes the configuration system for Mochi Terminal.

## Configuration File Location

Mochi Terminal follows XDG Base Directory conventions. The configuration file is located at:

- **Default**: `~/.config/mochi/config.toml`
- **XDG Override**: `$XDG_CONFIG_HOME/mochi/config.toml`

## Configuration Precedence

Configuration values are resolved in the following order (highest to lowest priority):

1. **CLI flags** - Command line arguments override all other settings
2. **Environment variables** - `MOCHI_*` environment variables
3. **Config file** - Values from the TOML configuration file
4. **Built-in defaults** - Hardcoded default values

## Command Line Options

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

| Variable | Description |
|----------|-------------|
| `MOCHI_CONFIG` | Path to configuration file |
| `MOCHI_FONT_SIZE` | Font size override |
| `MOCHI_THEME` | Theme name override |

## Configuration Options

### General Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `font_family` | string | "DejaVu Sans Mono" | Font family name (must be monospace) |
| `font_size` | float | 14.0 | Font size in points (range: 6.0-72.0) |
| `scrollback_lines` | integer | 10000 | Number of scrollback lines |
| `dimensions` | [cols, rows] | [80, 24] | Initial window dimensions |
| `theme` | string | "dark" | Theme name |
| `shell` | string | $SHELL | Shell command to run |
| `cursor_style` | string | "block" | Cursor style: "block", "underline", "beam" |
| `cursor_blink` | boolean | true | Enable cursor blinking |

### Security Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `osc52_clipboard` | boolean | false | Enable OSC 52 clipboard access |
| `osc52_max_size` | integer | 100000 | Maximum OSC 52 payload size in bytes |

### Color Scheme

When `theme = "custom"`, the `[colors]` section is used:

| Option | Type | Description |
|--------|------|-------------|
| `foreground` | hex color | Default text color |
| `background` | hex color | Background color |
| `cursor` | hex color | Cursor color |
| `selection` | hex color | Selection highlight color |
| `ansi` | array of 16 hex colors | ANSI color palette |

## Built-in Themes

Mochi Terminal includes the following built-in themes:

- **dark** - Default dark theme with VS Code-inspired colors
- **light** - Light theme for bright environments
- **solarizeddark** - Solarized Dark color scheme
- **solarizedlight** - Solarized Light color scheme
- **dracula** - Dracula color scheme
- **nord** - Nord color scheme

## Keybindings

The following keybindings are available:

| Keybinding | Action |
|------------|--------|
| `Ctrl+Shift+C` | Copy selection to clipboard |
| `Ctrl+Shift+V` | Paste from clipboard |
| `Ctrl+Shift+T` | Toggle theme (cycle through themes) |
| `Ctrl+Shift+R` | Reload configuration |
| `Ctrl+=` or `Ctrl++` | Increase font size |
| `Ctrl+-` | Decrease font size |
| `Ctrl+0` | Reset font size to default |

## Mouse Actions

| Action | Effect |
|--------|--------|
| Click + drag | Select text |
| Double-click | Select word |
| Triple-click | Select line |
| Middle-click | Paste from clipboard |
| Scroll wheel | Scroll through scrollback history |

## Example Configuration

See `config.example.toml` for a complete example configuration file with all available options documented.

## Validation

The configuration is validated on load. Invalid configurations will produce clear error messages. Common validation errors include:

- Font size out of range (must be 6.0-72.0)
- Invalid color format (must be #rrggbb or rrggbb)
- Invalid dimensions (must be at least 10x5)
- Zero scrollback lines (must be at least 1)

If a configuration file specified via `--config` does not exist, an error is reported. If the default configuration file does not exist, built-in defaults are used.
