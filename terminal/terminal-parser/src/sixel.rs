const DEFAULT_COLOR_REGISTERS: usize = 256;
const MAX_SIXEL_WIDTH: u32 = 4096;
const MAX_SIXEL_HEIGHT: u32 = 4096;

#[derive(Debug, Clone, Default)]
struct ColorRegister {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SixelImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub fn decode_sixel(params: &[u16], data: &[u8]) -> Option<SixelImage> {
    let background_mode = if !params.is_empty() { params[0] } else { 0 };

    let mut colors: Vec<ColorRegister> = vec![ColorRegister::default(); DEFAULT_COLOR_REGISTERS];
    colors[0] = ColorRegister {
        r: 255,
        g: 255,
        b: 255,
    };

    let mut pixels: Vec<u8> = Vec::new();
    let mut width: u32 = 0;
    let mut height: u32 = 0;
    let mut cursor_x: u32 = 0;
    let mut cursor_y: u32 = 0;
    let mut current_color: usize = 0;

    let mut i = 0;
    let len = data.len();

    while i < len {
        let byte = data[i];
        match byte {
            b'#' => {
                i += 1;
                let (reg, consumed) = parse_number(&data[i..]);
                i += consumed;

                if i < len && data[i] == b';' {
                    i += 1;
                    let (color_type, consumed) = parse_number(&data[i..]);
                    i += consumed;

                    if i < len && data[i] == b';' {
                        i += 1;
                        let (c1, consumed) = parse_number(&data[i..]);
                        i += consumed;

                        if i < len && data[i] == b';' {
                            i += 1;
                            let (c2, consumed) = parse_number(&data[i..]);
                            i += consumed;

                            if i < len && data[i] == b';' {
                                i += 1;
                                let (c3, consumed) = parse_number(&data[i..]);
                                i += consumed;

                                let reg = reg as usize;
                                if reg < colors.len() {
                                    if color_type == 2 {
                                        colors[reg] = ColorRegister {
                                            r: (c1 * 255 / 100) as u8,
                                            g: (c2 * 255 / 100) as u8,
                                            b: (c3 * 255 / 100) as u8,
                                        };
                                    } else if color_type == 1 {
                                        let (r, g, b) = hls_to_rgb(c1, c2, c3);
                                        colors[reg] = ColorRegister { r, g, b };
                                    }
                                }
                            }
                        }
                    }
                } else {
                    current_color = reg as usize;
                }
            }
            b'"' => {
                i += 1;
                let (_pad1, consumed) = parse_number(&data[i..]);
                i += consumed;
                if i < len && data[i] == b';' {
                    i += 1;
                }
                let (_pad2, consumed) = parse_number(&data[i..]);
                i += consumed;
                if i < len && data[i] == b';' {
                    i += 1;
                }
                let (raster_w, consumed) = parse_number(&data[i..]);
                i += consumed;
                if i < len && data[i] == b';' {
                    i += 1;
                }
                let (raster_h, consumed) = parse_number(&data[i..]);
                i += consumed;

                if raster_w > 0 && raster_h > 0 {
                    let rw = raster_w.min(MAX_SIXEL_WIDTH);
                    let rh = raster_h.min(MAX_SIXEL_HEIGHT);
                    width = rw;
                    height = rh;
                    let total = (rw * rh * 4) as usize;
                    pixels.resize(total, if background_mode == 1 { 0 } else { 255 });
                    if background_mode != 1 {
                        for chunk in pixels.chunks_exact_mut(4) {
                            chunk[0] = 0;
                            chunk[1] = 0;
                            chunk[2] = 0;
                            chunk[3] = 0;
                        }
                    }
                }
            }
            b'!' => {
                i += 1;
                let (repeat, consumed) = parse_number(&data[i..]);
                i += consumed;

                if i < len && data[i] >= 0x3F && data[i] <= 0x7E {
                    let sixel_val = data[i] - 0x3F;
                    i += 1;
                    let color = if current_color < colors.len() {
                        &colors[current_color]
                    } else {
                        &colors[0]
                    };
                    for _ in 0..repeat {
                        draw_sixel(
                            &mut pixels,
                            &mut width,
                            &mut height,
                            cursor_x,
                            cursor_y,
                            sixel_val,
                            color,
                        );
                        cursor_x += 1;
                    }
                }
            }
            b'-' => {
                cursor_x = 0;
                cursor_y += 6;
                i += 1;
            }
            b'$' => {
                cursor_x = 0;
                i += 1;
            }
            0x3F..=0x7E => {
                let sixel_val = byte - 0x3F;
                let color = if current_color < colors.len() {
                    &colors[current_color]
                } else {
                    &colors[0]
                };
                draw_sixel(
                    &mut pixels,
                    &mut width,
                    &mut height,
                    cursor_x,
                    cursor_y,
                    sixel_val,
                    color,
                );
                cursor_x += 1;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    if (width == 0 || height == 0) && (cursor_x > 0 || cursor_y > 0) {
        width = width.max(cursor_x);
        height = height.max(cursor_y + 6);
    }

    if width == 0 || height == 0 || pixels.is_empty() {
        return None;
    }

    pixels.resize((width * height * 4) as usize, 0);

    Some(SixelImage {
        width,
        height,
        rgba: pixels,
    })
}

fn draw_sixel(
    pixels: &mut Vec<u8>,
    width: &mut u32,
    height: &mut u32,
    x: u32,
    y: u32,
    sixel_val: u8,
    color: &ColorRegister,
) {
    if x >= MAX_SIXEL_WIDTH {
        return;
    }

    let needed_w = x + 1;
    let needed_h = y + 6;

    if needed_w > *width || needed_h > *height {
        let new_w = (*width).max(needed_w).min(MAX_SIXEL_WIDTH);
        let new_h = (*height).max(needed_h).min(MAX_SIXEL_HEIGHT);
        let mut new_pixels = vec![0u8; (new_w * new_h * 4) as usize];

        for row in 0..*height {
            let src_start = (row * *width * 4) as usize;
            let src_end = src_start + (*width * 4) as usize;
            let dst_start = (row * new_w * 4) as usize;
            if src_end <= pixels.len() {
                let copy_len = (*width * 4) as usize;
                new_pixels[dst_start..dst_start + copy_len]
                    .copy_from_slice(&pixels[src_start..src_end]);
            }
        }

        *pixels = new_pixels;
        *width = new_w;
        *height = new_h;
    }

    for bit in 0..6u32 {
        if sixel_val & (1 << bit) != 0 {
            let py = y + bit;
            if py < *height {
                let idx = ((py * *width + x) * 4) as usize;
                if idx + 3 < pixels.len() {
                    pixels[idx] = color.r;
                    pixels[idx + 1] = color.g;
                    pixels[idx + 2] = color.b;
                    pixels[idx + 3] = 255;
                }
            }
        }
    }
}

fn parse_number(data: &[u8]) -> (u32, usize) {
    let mut val: u32 = 0;
    let mut consumed = 0;
    for &byte in data {
        if byte.is_ascii_digit() {
            val = val.saturating_mul(10).saturating_add((byte - b'0') as u32);
            consumed += 1;
        } else {
            break;
        }
    }
    (val, consumed)
}

fn hls_to_rgb(h: u32, l: u32, s: u32) -> (u8, u8, u8) {
    let h = (h % 360) as f64;
    let l = (l as f64) / 100.0;
    let s = (s as f64) / 100.0;

    if s == 0.0 {
        let v = (l * 255.0) as u8;
        return (v, v, v);
    }

    let m2 = if l <= 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let m1 = 2.0 * l - m2;

    let r = hue_to_rgb(m1, m2, h + 120.0);
    let g = hue_to_rgb(m1, m2, h);
    let b = hue_to_rgb(m1, m2, h - 120.0);

    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

fn hue_to_rgb(m1: f64, m2: f64, mut h: f64) -> f64 {
    if h < 0.0 {
        h += 360.0;
    }
    if h > 360.0 {
        h -= 360.0;
    }

    if h < 60.0 {
        m1 + (m2 - m1) * h / 60.0
    } else if h < 180.0 {
        m2
    } else if h < 240.0 {
        m1 + (m2 - m1) * (240.0 - h) / 60.0
    } else {
        m1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number() {
        assert_eq!(parse_number(b"123abc"), (123, 3));
        assert_eq!(parse_number(b"abc"), (0, 0));
        assert_eq!(parse_number(b"0"), (0, 1));
    }

    #[test]
    fn test_hls_to_rgb() {
        let (r, g, b) = hls_to_rgb(0, 50, 100);
        assert!(r > 200);
        assert!(g < 50);
        assert!(b < 50);
    }

    #[test]
    fn test_decode_simple_sixel() {
        let data = b"#0;2;100;0;0~-#0;2;100;0;0~";
        let result = decode_sixel(&[0], data);
        assert!(result.is_some());
        let img = result.unwrap();
        assert!(img.width > 0);
        assert!(img.height > 0);
    }

    #[test]
    fn test_decode_empty_sixel() {
        let result = decode_sixel(&[0], b"");
        assert!(result.is_none());
    }

    #[test]
    fn test_sixel_repeat() {
        let data = b"#0;2;0;100;0!5~";
        let result = decode_sixel(&[0], data);
        assert!(result.is_some());
        let img = result.unwrap();
        assert!(img.width >= 5);
    }
}
