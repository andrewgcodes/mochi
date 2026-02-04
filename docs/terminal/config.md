# Mochi Terminal Configuration

This document describes the configuration system for Mochi Terminal.

## Configuration File Location

Mochi Terminal follows the XDG Base Directory specification for configuration file location:

1. If `XDG_CONFIG_HOME` is set: `$XDG_CONFIG_HOME/mochi/config.toml`
2. Otherwise: `~/.config/mochi/config.toml`

You can override the config file location using the `--config` CLI flag.

## Configuration Precedence

Configuration values are loaded with the following precedence (highest to lowest):

1. **CLI arguments** - Command line flags like `--theme`, `--font-size`
2. **Environment variables** - Variables like `MOCHI_THEME`, `MOCHI_FONT_SIZE`
3. **Config file** - The TOML configuration file
4. **Built-in defaults** - Sensible defaults for all settings

## CLI Arguments

```
mochi [OPTIONS]

Options:
  -c, --config <FILE>      Path to configuration file
  -t, --theme <THEME>      Theme to use (dark, light, solarized-dark, etc.)
      --font-size <SIZE>   Font size in points
      --font-family <NAME> Font family name
  -s, --shell <COMMAND>    Shell command to run
      --scrollback <LINES> Number of scrollback lines
  -h, --help               Print help
  -V, --version            Print version
```

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `MOCHI_THEME` | Theme name | `MOCHI_THEME=dracula` |
| `MOCHI_FONT_SIZE` | Font size in points | `MOCHI_FONT_SIZE=16` |
| `MOCHI_FONT_FAMILY` | Font family name | `MOCHI_FONT_FAMILY="JetBrains Mono"` |
| `MOCHI_SCROLLBACK` | Scrollback lines | `MOCHI_SCROLLBACK=50000` |
| `MOCHI_SHELL` | Shell command | `MOCHI_SHELL=/bin/zsh` |
| `MOCHI_OSC52_CLIPBOARD` | Enable OSC 52 clipboard | `MOCHI_OSC52_CLIPBOARD=true` |

## Configuration Schema

### Top-Level Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `theme` | string | `"dark"` | Color theme name |
| `scrollback_lines` | integer | `10000` | Lines of scrollback history |
| `dimensions` | [u16, u16] | `[80, 24]` | Initial window size (cols, rows) |
| `shell` | string? | `null` | Shell command (null = use $SHELL) |
| `cursor_style` | string | `"block"` | Cursor style: block, underline, bar |
| `cursor_blink` | boolean | `true` | Enable cursor blinking |
| `auto_reload` | boolean | `false` | Auto-reload config on file change |

### Font Configuration (`[font]`)

| Key | Type | Default | Range | Description |
|-----|------|---------|-------|-------------|
| `family` | string | `"monospace"` | - | Font family name |
| `size` | float | `14.0` | 4.0-144.0 | Font size in points |
| `fallback` | [string] | `[]` | - | Fallback font families |
| `line_height` | float | `1.2` | 0.5-3.0 | Line height multiplier |
| `cell_padding` | [f32, f32] | `[0.0, 0.0]` | - | Cell padding (h, v) |

### Keybindings (`[keybindings]`)

| Key | Default | Description |
|-----|---------|-------------|
| `copy` | `"ctrl+shift+c"` | Copy selection to clipboard |
| `paste` | `"ctrl+shift+v"` | Paste from clipboard |
| `find` | `"ctrl+shift+f"` | Open find/search bar |
| `reload_config` | `"ctrl+shift+r"` | Reload configuration |
| `toggle_theme` | `"ctrl+shift+t"` | Toggle between themes |
| `increase_font_size` | `"ctrl+plus"` | Increase font size |
| `decrease_font_size` | `"ctrl+minus"` | Decrease font size |
| `reset_font_size` | `"ctrl+0"` | Reset font size to default |

Keybinding format: `"modifier+modifier+key"` where modifiers can be `ctrl`, `shift`, `alt`, `super`.

### Security Settings (`[security]`)

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `osc52_clipboard` | boolean | `false` | Enable OSC 52 clipboard operations |
| `osc52_max_size` | integer | `100000` | Max OSC 52 payload size (bytes) |
| `clipboard_notification` | boolean | `true` | Show clipboard modification notification |
| `max_title_length` | integer | `4096` | Maximum window title length |
| `title_throttle_ms` | integer | `100` | Title update throttle (ms) |

### Custom Colors (`[colors]`)

Only used when `theme = "custom"`. All colors are hex strings in `#RRGGBB` format.

| Key | Description |
|-----|-------------|
| `foreground` | Default text color |
| `background` | Background color |
| `cursor` | Cursor color |
| `selection` | Selection highlight color |
| `ansi` | Array of 16 ANSI colors (0-7 normal, 8-15 bright) |

## Available Themes

| Theme | Description |
|-------|-------------|
| `dark` | VS Code-inspired dark theme (default) |
| `light` | Light theme with dark text |
| `solarized-dark` | Solarized dark color scheme |
| `solarized-light` | Solarized light color scheme |
| `dracula` | Dracula color scheme |
| `nord` | Nord color scheme |
| `gruvbox` | Gruvbox dark color scheme |
| `onedark` | Atom One Dark color scheme |
| `custom` | Use colors from `[colors]` section |

## Validation

The configuration is validated on load. Invalid configurations will produce clear error messages:

- Font size must be between 4.0 and 144.0
- Line height must be between 0.5 and 3.0
- Scrollback lines must not exceed 1,000,000
- Column count must be between 10 and 1000
- Row count must be between 5 and 500
- Cursor style must be one of: block, underline, bar
- All color values must be valid hex colors (#RRGGBB)

## Example Configuration

See [config.example.toml](config.example.toml) for a fully commented example configuration file.
