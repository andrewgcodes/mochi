//! UTF-8 decoding for the terminal parser
//!
//! Handles streaming UTF-8 decoding with proper error handling.

/// UTF-8 decoder state
#[derive(Debug, Clone, Default)]
pub struct Utf8Decoder {
    /// Bytes accumulated for current character
    buffer: [u8; 4],
    /// Number of bytes in buffer
    len: usize,
    /// Expected total bytes for current character
    expected: usize,
}

/// Result of feeding a byte to the decoder
#[derive(Debug, Clone, PartialEq)]
pub enum Utf8Result {
    /// Need more bytes
    Pending,
    /// Successfully decoded a character
    Char(char),
    /// Invalid sequence, returns replacement character
    Invalid,
}

impl Utf8Decoder {
    /// Create a new decoder
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset the decoder state
    pub fn reset(&mut self) {
        self.len = 0;
        self.expected = 0;
    }

    /// Check if decoder is in the middle of a sequence
    pub fn is_pending(&self) -> bool {
        self.len > 0
    }

    /// Feed a byte to the decoder
    pub fn feed(&mut self, byte: u8) -> Utf8Result {
        // ASCII fast path
        if self.len == 0 && byte < 0x80 {
            return Utf8Result::Char(byte as char);
        }

        // Start of new sequence
        if self.len == 0 {
            if byte & 0b1110_0000 == 0b1100_0000 {
                // 2-byte sequence
                self.buffer[0] = byte;
                self.len = 1;
                self.expected = 2;
                return Utf8Result::Pending;
            } else if byte & 0b1111_0000 == 0b1110_0000 {
                // 3-byte sequence
                self.buffer[0] = byte;
                self.len = 1;
                self.expected = 3;
                return Utf8Result::Pending;
            } else if byte & 0b1111_1000 == 0b1111_0000 {
                // 4-byte sequence
                self.buffer[0] = byte;
                self.len = 1;
                self.expected = 4;
                return Utf8Result::Pending;
            } else {
                // Invalid start byte
                return Utf8Result::Invalid;
            }
        }

        // Continuation byte
        if byte & 0b1100_0000 != 0b1000_0000 {
            // Invalid continuation byte
            self.reset();
            return Utf8Result::Invalid;
        }

        self.buffer[self.len] = byte;
        self.len += 1;

        if self.len < self.expected {
            return Utf8Result::Pending;
        }

        // Complete sequence - decode
        let result = match self.expected {
            2 => {
                let cp = ((self.buffer[0] & 0x1F) as u32) << 6 | (self.buffer[1] & 0x3F) as u32;
                // Check for overlong encoding
                if cp < 0x80 {
                    Utf8Result::Invalid
                } else {
                    char::from_u32(cp)
                        .map(Utf8Result::Char)
                        .unwrap_or(Utf8Result::Invalid)
                }
            }
            3 => {
                let cp = ((self.buffer[0] & 0x0F) as u32) << 12
                    | ((self.buffer[1] & 0x3F) as u32) << 6
                    | (self.buffer[2] & 0x3F) as u32;
                // Check for overlong encoding and surrogates
                if cp < 0x800 || (0xD800..=0xDFFF).contains(&cp) {
                    Utf8Result::Invalid
                } else {
                    char::from_u32(cp)
                        .map(Utf8Result::Char)
                        .unwrap_or(Utf8Result::Invalid)
                }
            }
            4 => {
                let cp = ((self.buffer[0] & 0x07) as u32) << 18
                    | ((self.buffer[1] & 0x3F) as u32) << 12
                    | ((self.buffer[2] & 0x3F) as u32) << 6
                    | (self.buffer[3] & 0x3F) as u32;
                // Check for overlong encoding and valid range
                if cp < 0x10000 || cp > 0x10FFFF {
                    Utf8Result::Invalid
                } else {
                    char::from_u32(cp)
                        .map(Utf8Result::Char)
                        .unwrap_or(Utf8Result::Invalid)
                }
            }
            _ => Utf8Result::Invalid,
        };

        self.reset();
        result
    }

    /// Get the replacement character for invalid sequences
    pub fn replacement_char() -> char {
        '\u{FFFD}'
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii() {
        let mut decoder = Utf8Decoder::new();
        assert_eq!(decoder.feed(b'A'), Utf8Result::Char('A'));
        assert_eq!(decoder.feed(b'z'), Utf8Result::Char('z'));
        assert_eq!(decoder.feed(b'0'), Utf8Result::Char('0'));
    }

    #[test]
    fn test_two_byte() {
        let mut decoder = Utf8Decoder::new();
        // 'é' = U+00E9 = 0xC3 0xA9
        assert_eq!(decoder.feed(0xC3), Utf8Result::Pending);
        assert_eq!(decoder.feed(0xA9), Utf8Result::Char('é'));
    }

    #[test]
    fn test_three_byte() {
        let mut decoder = Utf8Decoder::new();
        // '中' = U+4E2D = 0xE4 0xB8 0xAD
        assert_eq!(decoder.feed(0xE4), Utf8Result::Pending);
        assert_eq!(decoder.feed(0xB8), Utf8Result::Pending);
        assert_eq!(decoder.feed(0xAD), Utf8Result::Char('中'));
    }

    #[test]
    fn test_four_byte() {
        let mut decoder = Utf8Decoder::new();
        // '😀' = U+1F600 = 0xF0 0x9F 0x98 0x80
        assert_eq!(decoder.feed(0xF0), Utf8Result::Pending);
        assert_eq!(decoder.feed(0x9F), Utf8Result::Pending);
        assert_eq!(decoder.feed(0x98), Utf8Result::Pending);
        assert_eq!(decoder.feed(0x80), Utf8Result::Char('😀'));
    }

    #[test]
    fn test_invalid_start() {
        let mut decoder = Utf8Decoder::new();
        // 0xFF is never valid in UTF-8
        assert_eq!(decoder.feed(0xFF), Utf8Result::Invalid);
    }

    #[test]
    fn test_invalid_continuation() {
        let mut decoder = Utf8Decoder::new();
        // Start a 2-byte sequence but give invalid continuation
        assert_eq!(decoder.feed(0xC3), Utf8Result::Pending);
        assert_eq!(decoder.feed(0x00), Utf8Result::Invalid);
    }

    #[test]
    fn test_overlong_encoding() {
        let mut decoder = Utf8Decoder::new();
        // Overlong encoding of 'A' (should be 0x41, not 0xC1 0x81)
        assert_eq!(decoder.feed(0xC1), Utf8Result::Pending);
        assert_eq!(decoder.feed(0x81), Utf8Result::Invalid);
    }

    #[test]
    fn test_reset() {
        let mut decoder = Utf8Decoder::new();
        assert_eq!(decoder.feed(0xC3), Utf8Result::Pending);
        assert!(decoder.is_pending());
        decoder.reset();
        assert!(!decoder.is_pending());
    }
}
