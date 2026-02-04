# Mochi Terminal Font Configuration

Mochi Terminal provides configurable font settings to customize the appearance and readability of your terminal.

## Font Settings

### Font Size

The font size is specified in points. Valid range is 6.0 to 128.0.

```toml
font_size = 14.0
```

You can also change font size at runtime using keyboard shortcuts:
- `Ctrl+=` or `Ctrl++` - Increase font size
- `Ctrl+-` - Decrease font size
- `Ctrl+0` - Reset to default font size

### Line Height

The line height multiplier controls the vertical spacing between lines. A value of 1.0 means no extra spacing, while 1.4 (the default) adds 40% extra space for better readability.

```toml
line_height = 1.4
```

Valid range is 1.0 to 3.0.

### Font Family

Currently, Mochi uses the bundled DejaVu Sans Mono font for consistent cross-platform rendering. The `font_family` setting is reserved for future use when system font support is added.

```toml
font_family = "monospace"
```

## Bundled Fonts

Mochi includes the following fonts bundled in the application:

- **DejaVu Sans Mono** - Primary monospace font with excellent Unicode coverage
- **DejaVu Sans Mono Bold** - Bold variant for bold text rendering

These fonts are embedded in the binary, ensuring consistent rendering across all platforms without requiring font installation.

## Cell Size Calculation

The terminal cell size is calculated based on:
1. Font size (scaled for HiDPI displays)
2. Line height multiplier

The cell width is determined by the advance width of the 'M' character, and the cell height is `font_size * line_height`.

When font settings change, Mochi automatically:
1. Recalculates cell dimensions
2. Recomputes terminal rows and columns
3. Resizes the PTY to inform running applications of the new dimensions

## Configuration Precedence

Font settings follow the standard configuration precedence:

1. CLI flag (`--font-size`)
2. Environment variable (`MOCHI_FONT_SIZE`)
3. Config file
4. Default values

## Example Configuration

```toml
# Font settings
font_family = "monospace"
font_size = 14.0
line_height = 1.4
```

## HiDPI Support

Mochi automatically scales fonts for HiDPI displays. The configured font size is multiplied by the display's scale factor to ensure crisp rendering on high-resolution screens.

## Future Enhancements

Planned font features for future releases:
- System font support via fontconfig
- Custom font file loading
- Fallback font chains for missing glyphs
- Emoji font support
- Ligature support (optional)
