use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum KittyAction {
    TransmitData {
        image_id: u32,
        image_number: u32,
        format: KittyFormat,
        width: u32,
        height: u32,
        compression: KittyCompression,
        more_chunks: bool,
        data: Vec<u8>,
    },
    TransmitAndDisplay {
        image_id: u32,
        image_number: u32,
        format: KittyFormat,
        width: u32,
        height: u32,
        compression: KittyCompression,
        placement: KittyPlacement,
        more_chunks: bool,
        data: Vec<u8>,
    },
    Display {
        image_id: u32,
        image_number: u32,
        placement: KittyPlacement,
    },
    TransmitMoreData {
        image_id: u32,
        more_chunks: bool,
        data: Vec<u8>,
    },
    Delete(KittyDelete),
    Query {
        image_id: u32,
        image_number: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KittyFormat {
    Rgba,
    Rgb,
    Png,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KittyCompression {
    None,
    Zlib,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KittyPlacement {
    pub placement_id: u32,
    pub cols: u32,
    pub rows: u32,
    pub x_offset: u32,
    pub y_offset: u32,
    pub source_x: u32,
    pub source_y: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub z_index: i32,
    pub cursor_movement: KittyCursorMovement,
}

impl Default for KittyPlacement {
    fn default() -> Self {
        Self {
            placement_id: 0,
            cols: 0,
            rows: 0,
            x_offset: 0,
            y_offset: 0,
            source_x: 0,
            source_y: 0,
            source_width: 0,
            source_height: 0,
            z_index: 0,
            cursor_movement: KittyCursorMovement::After,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KittyCursorMovement {
    After,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KittyDelete {
    All,
    ById { image_id: u32 },
    ByNumber { image_number: u32 },
    AtCursor,
    AtPosition { col: u32, row: u32 },
    ByPlacement { image_id: u32, placement_id: u32 },
    Column { col: u32 },
    Row { row: u32 },
    ZIndex { z_index: i32 },
}

pub fn parse_kitty_graphics(data: &[u8]) -> Option<KittyAction> {
    let data_str = std::str::from_utf8(data).ok()?;

    if !data_str.starts_with('G') {
        return None;
    }

    let rest = &data_str[1..];

    let (control_str, payload) = if let Some(sep) = rest.find(';') {
        (&rest[..sep], &rest[sep + 1..])
    } else {
        (rest, "")
    };

    let params = parse_control_data(control_str);

    let action_char = params
        .get("a")
        .and_then(|v| v.chars().next())
        .unwrap_or('t');
    let quiet = params
        .get("q")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);

    let image_id = params
        .get("i")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);
    let image_number = params
        .get("I")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);

    let _ = quiet;

    match action_char {
        't' | 'T' => {
            let format = match params
                .get("f")
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(32)
            {
                24 => KittyFormat::Rgb,
                32 => KittyFormat::Rgba,
                100 => KittyFormat::Png,
                _ => KittyFormat::Rgba,
            };

            let width = params
                .get("s")
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);
            let height = params
                .get("v")
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);

            let compression = match params.get("o").map(|v| v.as_str()).unwrap_or("") {
                "z" => KittyCompression::Zlib,
                _ => KittyCompression::None,
            };

            let more_chunks = params
                .get("m")
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0)
                == 1;

            let decoded_data = decode_payload(payload);

            if action_char == 'T' {
                let placement = parse_placement(&params);
                Some(KittyAction::TransmitAndDisplay {
                    image_id,
                    image_number,
                    format,
                    width,
                    height,
                    compression,
                    placement,
                    more_chunks,
                    data: decoded_data,
                })
            } else {
                Some(KittyAction::TransmitData {
                    image_id,
                    image_number,
                    format,
                    width,
                    height,
                    compression,
                    more_chunks,
                    data: decoded_data,
                })
            }
        }
        'p' => {
            let placement = parse_placement(&params);
            Some(KittyAction::Display {
                image_id,
                image_number,
                placement,
            })
        }
        'q' => Some(KittyAction::Query {
            image_id,
            image_number,
        }),
        'd' => {
            let delete = parse_delete(&params);
            Some(KittyAction::Delete(delete))
        }
        _ => {
            if !payload.is_empty() {
                let more_chunks = params
                    .get("m")
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(0)
                    == 1;
                let decoded_data = decode_payload(payload);
                Some(KittyAction::TransmitMoreData {
                    image_id,
                    more_chunks,
                    data: decoded_data,
                })
            } else {
                None
            }
        }
    }
}

fn parse_control_data(data: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in data.split(',') {
        if let Some(eq) = pair.find('=') {
            let key = &pair[..eq];
            let value = &pair[eq + 1..];
            map.insert(key.to_string(), value.to_string());
        }
    }
    map
}

fn parse_placement(params: &HashMap<String, String>) -> KittyPlacement {
    KittyPlacement {
        placement_id: params.get("p").and_then(|v| v.parse().ok()).unwrap_or(0),
        cols: params.get("c").and_then(|v| v.parse().ok()).unwrap_or(0),
        rows: params.get("r").and_then(|v| v.parse().ok()).unwrap_or(0),
        x_offset: params.get("X").and_then(|v| v.parse().ok()).unwrap_or(0),
        y_offset: params.get("Y").and_then(|v| v.parse().ok()).unwrap_or(0),
        source_x: params.get("x").and_then(|v| v.parse().ok()).unwrap_or(0),
        source_y: params.get("y").and_then(|v| v.parse().ok()).unwrap_or(0),
        source_width: params.get("w").and_then(|v| v.parse().ok()).unwrap_or(0),
        source_height: params.get("h").and_then(|v| v.parse().ok()).unwrap_or(0),
        z_index: params.get("z").and_then(|v| v.parse().ok()).unwrap_or(0),
        cursor_movement: match params
            .get("C")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0)
        {
            1 => KittyCursorMovement::None,
            _ => KittyCursorMovement::After,
        },
    }
}

fn parse_delete(params: &HashMap<String, String>) -> KittyDelete {
    let delete_type = params.get("d").map(|v| v.as_str()).unwrap_or("a");

    match delete_type {
        "a" | "A" => KittyDelete::All,
        "i" | "I" => {
            let image_id = params.get("i").and_then(|v| v.parse().ok()).unwrap_or(0);
            KittyDelete::ById { image_id }
        }
        "n" | "N" => {
            let image_number = params.get("I").and_then(|v| v.parse().ok()).unwrap_or(0);
            KittyDelete::ByNumber { image_number }
        }
        "c" | "C" => KittyDelete::AtCursor,
        "p" | "P" => {
            let col = params.get("x").and_then(|v| v.parse().ok()).unwrap_or(0);
            let row = params.get("y").and_then(|v| v.parse().ok()).unwrap_or(0);
            KittyDelete::AtPosition { col, row }
        }
        "x" | "X" => {
            let col = params.get("x").and_then(|v| v.parse().ok()).unwrap_or(0);
            KittyDelete::Column { col }
        }
        "y" | "Y" => {
            let row = params.get("y").and_then(|v| v.parse().ok()).unwrap_or(0);
            KittyDelete::Row { row }
        }
        "z" | "Z" => {
            let z_index = params.get("z").and_then(|v| v.parse().ok()).unwrap_or(0);
            KittyDelete::ZIndex { z_index }
        }
        _ => KittyDelete::All,
    }
}

fn decode_payload(payload: &str) -> Vec<u8> {
    if payload.is_empty() {
        return Vec::new();
    }
    base64_decode(payload)
}

fn base64_decode(input: &str) -> Vec<u8> {
    const TABLE: [u8; 128] = {
        let mut t = [255u8; 128];
        let mut i = 0u8;
        while i < 26 {
            t[(b'A' + i) as usize] = i;
            i += 1;
        }
        i = 0;
        while i < 26 {
            t[(b'a' + i) as usize] = 26 + i;
            i += 1;
        }
        i = 0;
        while i < 10 {
            t[(b'0' + i) as usize] = 52 + i;
            i += 1;
        }
        t[b'+' as usize] = 62;
        t[b'/' as usize] = 63;
        t
    };

    let bytes: Vec<u8> = input
        .bytes()
        .filter(|&b| b != b'=' && b != b'\n' && b != b'\r' && b != b' ')
        .collect();

    let mut output = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut i = 0;

    while i + 3 < bytes.len() {
        let a = *TABLE.get(bytes[i] as usize).unwrap_or(&255);
        let b = *TABLE.get(bytes[i + 1] as usize).unwrap_or(&255);
        let c = *TABLE.get(bytes[i + 2] as usize).unwrap_or(&255);
        let d = *TABLE.get(bytes[i + 3] as usize).unwrap_or(&255);

        if a == 255 || b == 255 {
            i += 4;
            continue;
        }

        output.push((a << 2) | (b >> 4));
        if c != 255 {
            output.push((b << 4) | (c >> 2));
            if d != 255 {
                output.push((c << 6) | d);
            }
        }
        i += 4;
    }

    if i < bytes.len() {
        let remaining = bytes.len() - i;
        if remaining >= 2 {
            let a = *TABLE.get(bytes[i] as usize).unwrap_or(&255);
            let b = *TABLE.get(bytes[i + 1] as usize).unwrap_or(&255);
            if a != 255 && b != 255 {
                output.push((a << 2) | (b >> 4));
                if remaining >= 3 {
                    let c = *TABLE.get(bytes[i + 2] as usize).unwrap_or(&255);
                    if c != 255 {
                        output.push((b << 4) | (c >> 2));
                    }
                }
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_control_data() {
        let params = parse_control_data("a=t,f=32,s=100,v=50,i=1");
        assert_eq!(params.get("a").map(|s| s.as_str()), Some("t"));
        assert_eq!(params.get("f").map(|s| s.as_str()), Some("32"));
        assert_eq!(params.get("s").map(|s| s.as_str()), Some("100"));
        assert_eq!(params.get("v").map(|s| s.as_str()), Some("50"));
        assert_eq!(params.get("i").map(|s| s.as_str()), Some("1"));
    }

    #[test]
    fn test_base64_decode() {
        let encoded = "SGVsbG8=";
        let decoded = base64_decode(encoded);
        assert_eq!(decoded, b"Hello");
    }

    #[test]
    fn test_base64_decode_no_padding() {
        let encoded = "SGVsbG8";
        let decoded = base64_decode(encoded);
        assert_eq!(decoded, b"Hello");
    }

    #[test]
    fn test_parse_kitty_transmit() {
        let data = b"Ga=t,f=32,s=2,v=2,i=1;AAAAAAAAAAAAAAAAAAAAAAA=";
        let action = parse_kitty_graphics(data);
        assert!(action.is_some());
        if let Some(KittyAction::TransmitData {
            image_id,
            format,
            width,
            height,
            ..
        }) = action
        {
            assert_eq!(image_id, 1);
            assert_eq!(format, KittyFormat::Rgba);
            assert_eq!(width, 2);
            assert_eq!(height, 2);
        } else {
            panic!("Expected TransmitData");
        }
    }

    #[test]
    fn test_parse_kitty_display() {
        let data = b"Ga=p,i=1,c=10,r=5";
        let action = parse_kitty_graphics(data);
        assert!(action.is_some());
        if let Some(KittyAction::Display {
            image_id,
            placement,
            ..
        }) = action
        {
            assert_eq!(image_id, 1);
            assert_eq!(placement.cols, 10);
            assert_eq!(placement.rows, 5);
        } else {
            panic!("Expected Display");
        }
    }

    #[test]
    fn test_parse_kitty_delete() {
        let data = b"Ga=d,d=i,i=42";
        let action = parse_kitty_graphics(data);
        assert!(action.is_some());
        if let Some(KittyAction::Delete(KittyDelete::ById { image_id })) = action {
            assert_eq!(image_id, 42);
        } else {
            panic!("Expected Delete ById");
        }
    }

    #[test]
    fn test_parse_kitty_invalid() {
        let data = b"Xinvalid";
        let action = parse_kitty_graphics(data);
        assert!(action.is_none());
    }

    #[test]
    fn test_parse_kitty_transmit_and_display() {
        let data = b"Ga=T,f=100,i=5,c=20,r=10;iVBORw==";
        let action = parse_kitty_graphics(data);
        assert!(action.is_some());
        if let Some(KittyAction::TransmitAndDisplay {
            image_id,
            format,
            placement,
            ..
        }) = action
        {
            assert_eq!(image_id, 5);
            assert_eq!(format, KittyFormat::Png);
            assert_eq!(placement.cols, 20);
            assert_eq!(placement.rows, 10);
        } else {
            panic!("Expected TransmitAndDisplay");
        }
    }
}
