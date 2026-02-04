# Mochi Terminal Configuration Guide

This document describes the configuration system for Mochi Terminal, including all available options, their defaults, and how to customize them.

## Configuration File Location

Mochi Terminal follows the XDG Base Directory specification for configuration file location:

- **Default location**: `~/.config/mochi/config.toml`
- **Override via CLI**: `mochi --config /path/to/config.toml`
- **Override via environment**: `MOCHI_CONFIG=/path/to/config.toml`

If no configuration file exists, Mochi Terminal uses sensible defaults.

## Configuration Precedence

Configuration values are resolved in the following order (highest priority first):

1. **CLI arguments** - Command-line flags override everything
2. **Environment variables** - `MOCHI_*` prefixed variables
3. **Config file** - Values from `config.toml`
4. **Built-in defaults** - Hardcoded fallback values

## CLI Arguments

```
mochi [OPTIONS]

Options:
  -c, --config <FILE>      Path to configuration file
  -t, --theme <THEME>      Theme name (dark, light, solarized-dark, etc.)
      --font-size <SIZE>   Font size in points
      --font-family <NAME> Font family name
  -s, --shell <COMMAND>    Shell command to run
      --columns <N>        Initial window columns
      --rows <N>           Initial window rows
  -d, --debug              Enable debug logging
  -h, --help               Print help
  -V, --version            Print version
```

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `MOCHI_CONFIG` | Path to config file | `/home/user/mochi.toml` |
| `MOCHI_THEME` | Theme name | `dark`, `light` |
| `MOCHI_FONT_SIZE` | Font size in points | `14.0` |
| `MOCHI_FONT_FAMILY` | Font family name | `JetBrains Mono` |

## Configuration Sections

### Theme

The `theme` setting controls the overall color scheme of the terminal.

```toml
theme = "dark"
```

**Available built-in themes:**

| Theme | Description |
|-------|-------------|
| `dark` | Dark theme with gray background (default) |
| `light` | Light theme with white background |
| `solarized-dark` | Solarized dark color scheme |
| `solarized-light` | Solarized light color scheme |
| `dracula` | Dracula color scheme |
| `nord` | Nord color scheme |
| `custom` | Use custom colors defined in `[colors]` section |

### Font Configuration

```toml
[font]
family = "JetBrains Mono"
size = 14.0
weight = 400
line_height = 1.0
fallback = ["Noto Color Emoji"]
ligatures = false
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `family` | string | `"monospace"` | Font family name |
| `size` | float | `14.0` | Font size in points |
| `weight` | integer | `400` | Font weight (100-900) |
| `line_height` | float | `1.0` | Line height multiplier |
| `fallback` | array | `[]` | Fallback fonts for missing glyphs |
| `ligatures` | boolean | `false` | Enable font ligatures |

**Font Discovery:**

Mochi Terminal uses fontconfig to find fonts. You can list available fonts with:

```bash
fc-list :spacing=mono family
```

If the specified font is not found, Mochi falls back to the system's default monospace font.

### Keybindings

```toml
[keybindings]
copy = "ctrl+shift+c"
paste = "ctrl+shift+v"
find = "ctrl+shift+f"
reload_config = "ctrl+shift+r"
toggle_theme = "ctrl+shift+t"
```

**Keybinding Format:**

Keybindings are specified as `modifier+modifier+key` strings:

- **Modifiers**: `ctrl`, `alt`, `shift`, `super`
- **Keys**: Single characters (`a`-`z`, `0`-`9`) or named keys

**Named Keys:**

| Key | Description |
|-----|-------------|
| `f1`-`f12` | Function keys |
| `escape` | Escape key |
| `tab` | Tab key |
| `backspace` | Backspace key |
| `enter` | Enter/Return key |
| `insert` | Insert key |
| `delete` | Delete key |
| `home` | Home key |
| `end` | End key |
| `pageup` | Page Up key |
| `pagedown` | Page Down key |
| `up`, `down`, `left`, `right` | Arrow keys |
| `plus`, `minus` | Plus/Minus keys |

**Available Actions:**

| Action | Default Binding | Description |
|--------|-----------------|-------------|
| `copy` | `ctrl+shift+c` | Copy selection to clipboard |
| `paste` | `ctrl+shift+v` | Paste from clipboard |
| `find` | `ctrl+shift+f` | Open search bar |
| `reload_config` | `ctrl+shift+r` | Reload configuration |
| `toggle_theme` | `ctrl+shift+t` | Cycle through themes |
| `font_size_increase` | - | Increase font size |
| `font_size_decrease` | - | Decrease font size |
| `font_size_reset` | - | Reset font size to default |
| `scroll_up` | - | Scroll up in history |
| `scroll_down` | - | Scroll down in history |
| `scroll_page_up` | - | Scroll up one page |
| `scroll_page_down` | - | Scroll down one page |
| `scroll_to_top` | - | Scroll to top of history |
| `scroll_to_bottom` | - | Scroll to bottom |
| `clear_scrollback` | - | Clear scrollback buffer |

### Security Settings

```toml
[security]
osc52_clipboard = false
osc52_max_size = 100000
osc52_show_notification = true
title_rate_limit_ms = 100
title_max_length = 256
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `osc52_clipboard` | boolean | `false` | Allow OSC 52 clipboard access |
| `osc52_max_size` | integer | `100000` | Max clipboard payload size (bytes) |
| `osc52_show_notification` | boolean | `true` | Show notification on clipboard change |
| `title_rate_limit_ms` | integer | `100` | Min time between title updates (ms) |
| `title_max_length` | integer | `256` | Max window title length |

**Security Considerations:**

- **OSC 52 Clipboard**: When enabled, applications can read and write your clipboard via escape sequences. This is disabled by default because malicious applications could steal clipboard contents.

- **Title Updates**: Rapid title changes can be used for visual spam or social engineering. Rate limiting prevents abuse.

### Terminal Settings

```toml
scrollback_lines = 10000
columns = 80
rows = 24
cursor_style = "block"
```

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `scrollback_lines` | integer | `10000` | Lines to keep in scrollback |
| `columns` | integer | `80` | Initial terminal columns |
| `rows` | integer | `24` | Initial terminal rows |
| `cursor_style` | string | `"block"` | Cursor style |

**Cursor Styles:**

- `block` - Solid block cursor
- `underline` - Underline cursor
- `bar` - Vertical bar cursor

### Custom Color Scheme

When `theme = "custom"`, you can define your own colors:

```toml
theme = "custom"

[colors]
foreground = "#d4d4d4"
background = "#1e1e1e"
cursor = "#ffffff"
selection = "#264f78"

# ANSI colors (0-7)
black = "#000000"
red = "#cd3131"
green = "#0dbc79"
yellow = "#e5e510"
blue = "#2472c8"
magenta = "#bc3fbc"
cyan = "#11a8cd"
white = "#e5e5e5"

# Bright colors (8-15)
bright_black = "#666666"
bright_red = "#f14c4c"
bright_green = "#23d18b"
bright_yellow = "#f5f543"
bright_blue = "#3b8eea"
bright_magenta = "#d670d6"
bright_cyan = "#29b8db"
bright_white = "#ffffff"
```

All colors must be in `#RRGGBB` hexadecimal format.

## Validation

Mochi Terminal validates the configuration file on load and will:

1. **Report errors** for invalid syntax or values
2. **Use defaults** for missing optional values
3. **Refuse to start** if critical errors are found

Common validation errors:

- Invalid color format (must be `#RRGGBB`)
- Invalid keybinding format
- Font size out of range (must be 1.0-200.0)
- Unknown theme name

## Runtime Reload

You can reload the configuration without restarting:

1. **Keybinding**: Press `Ctrl+Shift+R` (default)
2. **File watcher**: Changes are detected automatically (if enabled)

On reload:

- Theme changes take effect immediately
- Font changes update the display
- Keybinding changes take effect immediately
- Invalid config keeps the previous settings and logs an error

## Example Configurations

### Minimal Dark Theme

```toml
theme = "dark"

[font]
size = 16.0
```

### Solarized with Custom Font

```toml
theme = "solarized-dark"

[font]
family = "Fira Code"
size = 13.0
ligatures = true
```

### High Security

```toml
[security]
osc52_clipboard = false
osc52_max_size = 10000
title_rate_limit_ms = 500
title_max_length = 100
```

### Custom Keybindings

```toml
[keybindings]
copy = "ctrl+c"
paste = "ctrl+v"
find = "ctrl+f"
toggle_theme = "ctrl+t"
```

## Troubleshooting

### Config file not found

Ensure the file exists at `~/.config/mochi/config.toml` or specify the path:

```bash
mochi --config /path/to/config.toml
```

### Font not rendering correctly

1. Check if the font is installed: `fc-list | grep "FontName"`
2. Try a different font family
3. Check the font supports the characters you need

### Keybinding not working

1. Ensure the keybinding doesn't conflict with your window manager
2. Check the format is correct: `modifier+modifier+key`
3. Try reloading config with `Ctrl+Shift+R`

### Theme not applying

1. Check the theme name is spelled correctly
2. For custom themes, ensure all required colors are defined
3. Try reloading config or restarting the terminal
