# Mochi Terminal Font Configuration

This document describes font configuration for Mochi Terminal.

## Default Font

Mochi Terminal uses **DejaVu Sans Mono** as its default font. This font is bundled with the application to ensure consistent rendering across all platforms without requiring system font installation.

## Font Size

### Configuration

Set the font size in your `~/.config/mochi/config.toml`:

```toml
font_size = 14.0  # Size in points (default: 14.0)
```

### CLI Override

```bash
mochi --font-size 16
```

### Environment Variable

```bash
MOCHI_FONT_SIZE=18 mochi
```

### Valid Range

Font size must be between **4.0** and **128.0** points. Values outside this range will cause a configuration error.

## Runtime Font Size Changes

You can change the font size while the terminal is running using keyboard shortcuts:

| Shortcut | Action |
|----------|--------|
| `Ctrl+=` or `Ctrl++` | Increase font size |
| `Ctrl+-` | Decrease font size |
| `Ctrl+0` | Reset to default font size |
| `Ctrl+Up` | Increase font size |
| `Ctrl+Down` | Decrease font size |

On macOS, use `Cmd` instead of `Ctrl`.

### Behavior

When font size changes:
1. Cell dimensions are recalculated based on the new font metrics
2. Terminal rows and columns are recalculated based on window size
3. The PTY is notified of the new dimensions (TIOCSWINSZ)
4. Applications receive SIGWINCH and can query the new size

## Cell Dimensions

Cell dimensions are calculated automatically based on the font:
- **Width**: Based on the advance width of the 'M' character
- **Height**: Font size multiplied by 1.4 (line height factor)
- **Baseline**: Equal to the font size

## HiDPI Support

Mochi Terminal automatically scales fonts for HiDPI displays:
- The configured font size is multiplied by the display scale factor
- This ensures text appears at the correct physical size on all displays

## Bold Text

Bold text is rendered using **DejaVu Sans Mono Bold**, which is also bundled with the application. This ensures consistent bold rendering without relying on synthetic bolding.

## Known Limitations

### Custom Font Families

The `font_family` configuration option exists but is **not yet implemented**. Currently, only the bundled DejaVu Sans Mono font is used regardless of the `font_family` setting.

```toml
# This setting is currently ignored
font_family = "JetBrains Mono"  # NOT YET SUPPORTED
```

Future versions may support:
- System font discovery via fontconfig
- Loading fonts from file paths
- Font fallback chains

### Ligatures

Font ligatures are **not supported**. The fontdue library used for font rendering does not support OpenType ligature features.

### Emoji

Emoji rendering is **not yet supported**. Characters outside the font's coverage will display as replacement characters or blank cells.

### Cell Padding

Cell padding and custom line height multipliers are **not yet configurable**. The line height is fixed at 1.4x the font size.

## Troubleshooting

### Text appears too small/large

1. Check your display scale factor in your system settings
2. Adjust the `font_size` in your config
3. Use `Ctrl+0` to reset to the configured default

### Characters appear cut off

This may occur with certain Unicode characters that exceed the cell width. Wide characters (CJK, emoji) should occupy two cells, but rendering issues may occur.

### Font looks blurry

Ensure your display scale factor is correctly detected. Mochi uses the scale factor reported by the windowing system.

## Example Configuration

```toml
# Font settings
font_family = "monospace"  # Currently ignored, uses bundled font
font_size = 14.0           # Font size in points

# Terminal dimensions (calculated from font size and window size)
dimensions = [80, 24]      # Initial columns x rows
```

## Technical Details

### Font Rendering

Mochi uses the following libraries for font rendering:
- **fontdue**: Font parsing and glyph rasterization
- **softbuffer**: CPU-based pixel buffer rendering

### Glyph Caching

Glyphs are cached after first render to improve performance. The cache is cleared when:
- Font size changes
- (Future) Font family changes

### Metrics Calculation

```
cell_width = font.metrics('M', font_size).advance_width
cell_height = font_size * 1.4
baseline = font_size
```
