# Mochi Terminal Configuration

This document describes the configuration system for Mochi Terminal.

## Configuration File Location

Mochi Terminal follows the XDG Base Directory Specification for configuration file location:

1. **Default location**: `~/.config/mochi/config.toml`
2. **XDG override**: If `XDG_CONFIG_HOME` is set, uses `$XDG_CONFIG_HOME/mochi/config.toml`
3. **CLI override**: Use `--config /path/to/config.toml` to specify a custom location
4. **Environment override**: Set `MOCHI_CONFIG=/path/to/config.toml`

## Configuration Precedence

Configuration values are loaded with the following precedence (highest to lowest):

1. **CLI arguments** - Command line flags like `--font-size 16`
2. **Environment variables** - Variables like `MOCHI_FONT_SIZE=16`
3. **Config file** - Values from `config.toml`
4. **Built-in defaults** - Hardcoded fallback values

This means CLI arguments always win, followed by environment variables, then the config file.

## CLI Arguments

```
mochi [OPTIONS]

Options:
  -c, --config <FILE>       Path to configuration file
      --font-size <SIZE>    Font size in points
      --font-family <FAMILY> Font family name
      --theme <THEME>       Theme name (dark, light, solarized-dark, etc.)
      --cols <COLS>         Initial window columns
      --rows <ROWS>         Initial window rows
      --shell <SHELL>       Shell command to run
      --osc52-clipboard     Enable OSC 52 clipboard support
  -h, --help                Print help
  -V, --version             Print version
```

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `MOCHI_CONFIG` | Path to config file | `/home/user/my-config.toml` |
| `MOCHI_FONT_SIZE` | Font size in points | `16` |
| `MOCHI_FONT_FAMILY` | Font family name | `JetBrains Mono` |
| `MOCHI_THEME` | Theme name | `dracula` |
| `MOCHI_SCROLLBACK_LINES` | Scrollback buffer size | `50000` |
| `MOCHI_SHELL` | Shell command | `/bin/zsh` |
| `MOCHI_OSC52_CLIPBOARD` | Enable OSC 52 clipboard | `true` or `1` |
| `MOCHI_LINE_HEIGHT` | Line height multiplier | `1.2` |

## Configuration Schema

### Font Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `font_family` | string | `"monospace"` | Font family name |
| `font_size` | float | `14.0` | Font size in points (4-200) |
| `font_fallback` | array | `[]` | Fallback fonts for missing glyphs |
| `line_height` | float | `1.4` | Line height multiplier (0.5-3.0) |
| `cell_padding` | [float, float] | `[0.0, 0.0]` | Cell padding [horizontal, vertical] |

### Theme Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `theme` | string | `"mochi-dark"` | Theme name (see below) |
| `colors` | object | - | Custom color scheme (when theme = "custom") |

**Available Themes:**
- `mochi-dark` - VS Code inspired dark theme (default)
- `mochi-light` - Clean light theme
- `solarized-dark` - Solarized dark color scheme
- `solarized-light` - Solarized light color scheme
- `dracula` - Dracula color scheme
- `nord` - Nord color scheme
- `custom` - Use the `colors` section

### Color Scheme (for custom theme)

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

### Terminal Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `scrollback_lines` | integer | `10000` | Lines to keep in scrollback buffer |
| `dimensions` | [u16, u16] | `[80, 24]` | Initial window size [columns, rows] |
| `shell` | string | `null` | Shell command (uses `$SHELL` if not set) |
| `cursor_style` | string | `"block"` | Cursor style: "block", "underline", "bar" |
| `cursor_blink` | boolean | `true` | Enable cursor blinking |

### Security Settings

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `osc52_clipboard` | boolean | `false` | Enable OSC 52 clipboard access |
| `osc52_max_size` | integer | `100000` | Max OSC 52 payload size in bytes |
| `title_update_rate` | integer | `10` | Max title updates per second |

**Security Note:** OSC 52 clipboard is disabled by default because it allows programs running in the terminal to read and write your system clipboard. Only enable this if you trust all programs you run.

### Keybindings

```toml
[keybindings]
bindings = [
    { key = "c", modifiers = ["ctrl", "shift"], action = "copy" },
    { key = "v", modifiers = ["ctrl", "shift"], action = "paste" },
    { key = "f", modifiers = ["ctrl", "shift"], action = "find" },
    { key = "r", modifiers = ["ctrl", "shift"], action = "reload-config" },
    { key = "t", modifiers = ["ctrl", "shift"], action = "toggle-theme" },
    { key = "=", modifiers = ["ctrl"], action = "font-size-increase" },
    { key = "-", modifiers = ["ctrl"], action = "font-size-decrease" },
    { key = "0", modifiers = ["ctrl"], action = "font-size-reset" },
]
```

**Available Modifiers:** `ctrl`, `shift`, `alt`, `super` (or `meta`)

**Available Actions:**
| Action | Description |
|--------|-------------|
| `copy` | Copy selected text to clipboard |
| `paste` | Paste from clipboard |
| `find` | Open search bar |
| `reload-config` | Reload configuration file |
| `toggle-theme` | Cycle through themes |
| `font-size-increase` | Increase font size |
| `font-size-decrease` | Decrease font size |
| `font-size-reset` | Reset font size to default |
| `scroll-up` | Scroll up one line |
| `scroll-down` | Scroll down one line |
| `scroll-page-up` | Scroll up one page |
| `scroll-page-down` | Scroll down one page |
| `scroll-to-top` | Scroll to top of scrollback |
| `scroll-to-bottom` | Scroll to bottom |

## Validation

The configuration is validated on load. Invalid configurations will show an error and fall back to defaults. Validation rules:

- `font_size`: Must be between 4 and 200
- `line_height`: Must be between 0.5 and 3.0
- `dimensions[0]` (columns): Must be between 10 and 1000
- `dimensions[1]` (rows): Must be between 5 and 500
- `osc52_max_size`: Must be at most 10MB (10,000,000 bytes)

## Runtime Reload

Configuration can be reloaded at runtime using the `reload-config` keybinding (default: `Ctrl+Shift+R`). The following settings can be changed without restart:

- Font settings (family, size, fallback, line height, padding)
- Theme and colors
- Keybindings
- OSC 52 settings
- Title update rate

Settings that require restart:
- Shell command
- Initial dimensions

If reload fails (e.g., invalid config), the previous configuration is kept and an error is logged.

## Example Configuration

See [config.example.toml](config.example.toml) for a fully commented example configuration file.
