# Mochi Terminal Keybindings

This document describes the keybinding system in Mochi Terminal, including default shortcuts and how to customize them.

## Default Keybindings

Mochi Terminal comes with sensible default keybindings that follow common terminal emulator conventions.

### Clipboard Operations

| Shortcut | Action | Description |
|----------|--------|-------------|
| Ctrl+Shift+C | Copy | Copy selected text to clipboard |
| Ctrl+Shift+V | Paste | Paste from clipboard |

### Theme and Configuration

| Shortcut | Action | Description |
|----------|--------|-------------|
| Ctrl+Shift+T | Toggle Theme | Cycle through built-in themes |
| Ctrl+Shift+R | Reload Config | Reload configuration from file |

### Font Size

| Shortcut | Action | Description |
|----------|--------|-------------|
| Ctrl+= or Ctrl++ | Increase Font Size | Increase font size by 2 points |
| Ctrl+- | Decrease Font Size | Decrease font size by 2 points |
| Ctrl+0 | Reset Font Size | Reset to configured default size |
| Ctrl+Up Arrow | Increase Font Size | Alternative shortcut |
| Ctrl+Down Arrow | Decrease Font Size | Alternative shortcut |

### Search (Planned)

| Shortcut | Action | Description |
|----------|--------|-------------|
| Ctrl+Shift+F | Find | Open search bar (not yet implemented) |

## Customizing Keybindings

Keybindings can be customized in the configuration file. The keybindings section uses a list of binding definitions.

### Configuration Format

```toml
[keybindings]
bindings = [
    { key = "c", ctrl = true, shift = true, alt = false, action = "copy" },
    { key = "v", ctrl = true, shift = true, alt = false, action = "paste" },
    { key = "t", ctrl = true, shift = true, alt = false, action = "toggle_theme" },
    { key = "r", ctrl = true, shift = true, alt = false, action = "reload_config" },
    { key = "f", ctrl = true, shift = true, alt = false, action = "find" },
    { key = "=", ctrl = true, shift = false, alt = false, action = "font_size_increase" },
    { key = "+", ctrl = true, shift = false, alt = false, action = "font_size_increase" },
    { key = "-", ctrl = true, shift = false, alt = false, action = "font_size_decrease" },
    { key = "0", ctrl = true, shift = false, alt = false, action = "font_size_reset" },
]
```

### Keybinding Fields

Each keybinding has the following fields:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| key | string | Yes | The key to bind (e.g., "c", "v", "=", "-") |
| ctrl | boolean | No | Whether Ctrl modifier is required (default: false) |
| shift | boolean | No | Whether Shift modifier is required (default: false) |
| alt | boolean | No | Whether Alt modifier is required (default: false) |
| action | string | Yes | The action to perform |

### Available Actions

| Action | Description |
|--------|-------------|
| `copy` | Copy selected text to clipboard |
| `paste` | Paste from clipboard |
| `toggle_theme` | Cycle through built-in themes |
| `reload_config` | Reload configuration from file |
| `find` | Open search bar |
| `font_size_increase` | Increase font size |
| `font_size_decrease` | Decrease font size |
| `font_size_reset` | Reset font size to default |
| `scroll_page_up` | Scroll up one page |
| `scroll_page_down` | Scroll down one page |
| `scroll_to_top` | Scroll to top of scrollback |
| `scroll_to_bottom` | Scroll to bottom (current output) |

### Key Names

For regular keys, use the lowercase character (e.g., "c", "v", "t"). For special keys, use these names:

| Key Name | Description |
|----------|-------------|
| `up` | Up arrow |
| `down` | Down arrow |
| `left` | Left arrow |
| `right` | Right arrow |
| `pageup` | Page Up |
| `pagedown` | Page Down |
| `home` | Home |
| `end` | End |
| `escape` | Escape |

## Examples

### Vim-style Scrolling

Add Page Up/Down bindings with Ctrl+U and Ctrl+D:

```toml
[keybindings]
bindings = [
    # Default bindings...
    { key = "c", ctrl = true, shift = true, action = "copy" },
    { key = "v", ctrl = true, shift = true, action = "paste" },
    # Vim-style scrolling
    { key = "u", ctrl = true, action = "scroll_page_up" },
    { key = "d", ctrl = true, action = "scroll_page_down" },
]
```

### Alternative Copy/Paste

Use Ctrl+C and Ctrl+V (note: this may interfere with terminal applications):

```toml
[keybindings]
bindings = [
    { key = "c", ctrl = true, action = "copy" },
    { key = "v", ctrl = true, action = "paste" },
]
```

## Keybinding Precedence

When a key combination is pressed, Mochi Terminal checks keybindings in the following order:

1. User-configured keybindings (first match wins)
2. If no keybinding matches, the key is sent to the terminal application

This means that configured keybindings take precedence over terminal input. Be careful not to bind keys that applications need (like Ctrl+C for interrupt).

## Design Decisions

### Why Ctrl+Shift for Clipboard?

Mochi Terminal uses Ctrl+Shift+C and Ctrl+Shift+V for copy/paste by default because:

1. Ctrl+C is the standard interrupt signal (SIGINT) in Unix terminals
2. Ctrl+V is often used for literal character input in some shells
3. Ctrl+Shift combinations are less likely to conflict with terminal applications
4. This matches the convention used by GNOME Terminal, Konsole, and other modern terminal emulators

### Modifier Support

Currently supported modifiers:
- Ctrl (Control)
- Shift
- Alt

Not yet supported:
- Super/Meta (Windows/Command key)
- Hyper

## Known Limitations

1. Custom keybindings require a restart to take effect (hot reload planned for M6)
2. Super/Meta modifier is not yet supported
3. Multi-key sequences (like Vim's leader key) are not supported
4. Some key combinations may not work on all keyboard layouts

## Troubleshooting

### Keybinding Not Working

1. Check that the keybinding is correctly formatted in your config file
2. Verify the action name is spelled correctly (use snake_case)
3. Ensure the key name is lowercase
4. Check for conflicting keybindings (first match wins)

### Key Sent to Application Instead

If a key is being sent to the terminal application instead of triggering your keybinding:

1. Verify the modifier flags are correct
2. Check that the key name matches exactly
3. Ensure your config file is being loaded (check with `--config` flag)
