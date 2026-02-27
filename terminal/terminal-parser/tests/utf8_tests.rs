//! Comprehensive tests for UTF-8 streaming decoder

use terminal_parser::Parser;

// Helper to parse bytes and collect printed characters
fn parse_chars(bytes: &[u8]) -> Vec<char> {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(bytes);
    actions
        .iter()
        .filter_map(|a| match a {
            terminal_parser::Action::Print(c) => Some(*c),
            _ => None,
        })
        .collect()
}

// ============================================================================
// ASCII characters (1-byte)
// ============================================================================

#[test]
fn test_utf8_ascii_a() {
    assert_eq!(parse_chars(b"A"), vec!['A']);
}

#[test]
fn test_utf8_ascii_z() {
    assert_eq!(parse_chars(b"z"), vec!['z']);
}

#[test]
fn test_utf8_ascii_digit() {
    assert_eq!(parse_chars(b"0"), vec!['0']);
}

#[test]
fn test_utf8_ascii_space() {
    assert_eq!(parse_chars(b" "), vec![' ']);
}

#[test]
fn test_utf8_ascii_exclamation() {
    assert_eq!(parse_chars(b"!"), vec!['!']);
}

#[test]
fn test_utf8_ascii_tilde() {
    assert_eq!(parse_chars(b"~"), vec!['~']);
}

#[test]
fn test_utf8_ascii_at() {
    assert_eq!(parse_chars(b"@"), vec!['@']);
}

#[test]
fn test_utf8_ascii_hash() {
    assert_eq!(parse_chars(b"#"), vec!['#']);
}

#[test]
fn test_utf8_ascii_string() {
    assert_eq!(parse_chars(b"Hello"), vec!['H', 'e', 'l', 'l', 'o']);
}

#[test]
fn test_utf8_ascii_all_printable() {
    // ASCII printable range: 0x20-0x7E
    for byte in 0x20u8..=0x7E {
        let chars = parse_chars(&[byte]);
        assert_eq!(chars, vec![byte as char], "Failed for byte 0x{:02X}", byte);
    }
}

// ============================================================================
// 2-byte UTF-8 sequences (U+0080 to U+07FF)
// ============================================================================

#[test]
fn test_utf8_latin_e_acute() {
    // é = U+00E9 = 0xC3 0xA9
    assert_eq!(parse_chars(&[0xC3, 0xA9]), vec!['é']);
}

#[test]
fn test_utf8_latin_n_tilde() {
    // ñ = U+00F1 = 0xC3 0xB1
    assert_eq!(parse_chars(&[0xC3, 0xB1]), vec!['ñ']);
}

#[test]
fn test_utf8_latin_u_umlaut() {
    // ü = U+00FC = 0xC3 0xBC
    assert_eq!(parse_chars(&[0xC3, 0xBC]), vec!['ü']);
}

#[test]
fn test_utf8_copyright() {
    // © = U+00A9 = 0xC2 0xA9
    assert_eq!(parse_chars(&[0xC2, 0xA9]), vec!['©']);
}

#[test]
fn test_utf8_registered() {
    // ® = U+00AE = 0xC2 0xAE
    assert_eq!(parse_chars(&[0xC2, 0xAE]), vec!['®']);
}

#[test]
fn test_utf8_degree() {
    // ° = U+00B0 = 0xC2 0xB0
    assert_eq!(parse_chars(&[0xC2, 0xB0]), vec!['°']);
}

#[test]
fn test_utf8_pound() {
    // £ = U+00A3 = 0xC2 0xA3
    assert_eq!(parse_chars(&[0xC2, 0xA3]), vec!['£']);
}

#[test]
fn test_utf8_yen() {
    // ¥ = U+00A5 = 0xC2 0xA5
    assert_eq!(parse_chars(&[0xC2, 0xA5]), vec!['¥']);
}

#[test]
fn test_utf8_micro() {
    // µ = U+00B5 = 0xC2 0xB5
    assert_eq!(parse_chars(&[0xC2, 0xB5]), vec!['µ']);
}

#[test]
fn test_utf8_pi_lower() {
    // π = U+03C0 = 0xCF 0x80
    assert_eq!(parse_chars(&[0xCF, 0x80]), vec!['π']);
}

#[test]
fn test_utf8_sigma() {
    // Σ = U+03A3 = 0xCE 0xA3
    assert_eq!(parse_chars(&[0xCE, 0xA3]), vec!['Σ']);
}

#[test]
fn test_utf8_delta() {
    // Δ = U+0394 = 0xCE 0x94
    assert_eq!(parse_chars(&[0xCE, 0x94]), vec!['Δ']);
}

#[test]
fn test_utf8_cyrillic_a() {
    // А = U+0410 = 0xD0 0x90
    assert_eq!(parse_chars(&[0xD0, 0x90]), vec!['А']);
}

#[test]
fn test_utf8_cyrillic_ya() {
    // Я = U+042F = 0xD0 0xAF
    assert_eq!(parse_chars(&[0xD0, 0xAF]), vec!['Я']);
}

#[test]
fn test_utf8_arabic_alef() {
    // ا = U+0627 = 0xD8 0xA7
    assert_eq!(parse_chars(&[0xD8, 0xA7]), vec!['ا']);
}

// ============================================================================
// 3-byte UTF-8 sequences (U+0800 to U+FFFF)
// ============================================================================

#[test]
fn test_utf8_cjk_zhong() {
    // 中 = U+4E2D = 0xE4 0xB8 0xAD
    assert_eq!(parse_chars(&[0xE4, 0xB8, 0xAD]), vec!['中']);
}

#[test]
fn test_utf8_cjk_wen() {
    // 文 = U+6587 = 0xE6 0x96 0x87
    assert_eq!(parse_chars(&[0xE6, 0x96, 0x87]), vec!['文']);
}

#[test]
fn test_utf8_hiragana_a() {
    // あ = U+3042 = 0xE3 0x81 0x82
    assert_eq!(parse_chars(&[0xE3, 0x81, 0x82]), vec!['あ']);
}

#[test]
fn test_utf8_katakana_a() {
    // ア = U+30A2 = 0xE3 0x82 0xA2
    assert_eq!(parse_chars(&[0xE3, 0x82, 0xA2]), vec!['ア']);
}

#[test]
fn test_utf8_korean_ga() {
    // 가 = U+AC00 = 0xEA 0xB0 0x80
    assert_eq!(parse_chars(&[0xEA, 0xB0, 0x80]), vec!['가']);
}

#[test]
fn test_utf8_euro_sign() {
    // € = U+20AC = 0xE2 0x82 0xAC
    assert_eq!(parse_chars(&[0xE2, 0x82, 0xAC]), vec!['€']);
}

#[test]
fn test_utf8_snowman() {
    // ☃ = U+2603 = 0xE2 0x98 0x83
    assert_eq!(parse_chars(&[0xE2, 0x98, 0x83]), vec!['☃']);
}

#[test]
fn test_utf8_check_mark() {
    // ✓ = U+2713 = 0xE2 0x9C 0x93
    assert_eq!(parse_chars(&[0xE2, 0x9C, 0x93]), vec!['✓']);
}

#[test]
fn test_utf8_infinity() {
    // ∞ = U+221E = 0xE2 0x88 0x9E
    assert_eq!(parse_chars(&[0xE2, 0x88, 0x9E]), vec!['∞']);
}

#[test]
fn test_utf8_box_drawing_horizontal() {
    // ─ = U+2500 = 0xE2 0x94 0x80
    assert_eq!(parse_chars(&[0xE2, 0x94, 0x80]), vec!['─']);
}

#[test]
fn test_utf8_box_drawing_vertical() {
    // │ = U+2502 = 0xE2 0x94 0x82
    assert_eq!(parse_chars(&[0xE2, 0x94, 0x82]), vec!['│']);
}

#[test]
fn test_utf8_box_drawing_corner_tl() {
    // ┌ = U+250C = 0xE2 0x94 0x8C
    assert_eq!(parse_chars(&[0xE2, 0x94, 0x8C]), vec!['┌']);
}

#[test]
fn test_utf8_block_full() {
    // █ = U+2588 = 0xE2 0x96 0x88
    assert_eq!(parse_chars(&[0xE2, 0x96, 0x88]), vec!['█']);
}

#[test]
fn test_utf8_replacement_char() {
    // U+FFFD = 0xEF 0xBF 0xBD
    assert_eq!(parse_chars(&[0xEF, 0xBF, 0xBD]), vec!['\u{FFFD}']);
}

#[test]
fn test_utf8_bom() {
    // BOM U+FEFF = 0xEF 0xBB 0xBF
    assert_eq!(parse_chars(&[0xEF, 0xBB, 0xBF]), vec!['\u{FEFF}']);
}

// ============================================================================
// 4-byte UTF-8 sequences (U+10000 to U+10FFFF)
// ============================================================================

#[test]
fn test_utf8_grinning_face() {
    // 😀 = U+1F600 = 0xF0 0x9F 0x98 0x80
    assert_eq!(parse_chars(&[0xF0, 0x9F, 0x98, 0x80]), vec!['😀']);
}

#[test]
fn test_utf8_party_popper() {
    // 🎉 = U+1F389 = 0xF0 0x9F 0x8E 0x89
    assert_eq!(parse_chars(&[0xF0, 0x9F, 0x8E, 0x89]), vec!['🎉']);
}

#[test]
fn test_utf8_rocket() {
    // 🚀 = U+1F680 = 0xF0 0x9F 0x9A 0x80
    assert_eq!(parse_chars(&[0xF0, 0x9F, 0x9A, 0x80]), vec!['🚀']);
}

#[test]
fn test_utf8_heart() {
    // ❤ = U+2764 = 0xE2 0x9D 0xA4 (3-byte, not 4)
    assert_eq!(parse_chars(&[0xE2, 0x9D, 0xA4]), vec!['❤']);
}

#[test]
fn test_utf8_thumbs_up() {
    // 👍 = U+1F44D = 0xF0 0x9F 0x91 0x8D
    assert_eq!(parse_chars(&[0xF0, 0x9F, 0x91, 0x8D]), vec!['👍']);
}

#[test]
fn test_utf8_fire() {
    // 🔥 = U+1F525 = 0xF0 0x9F 0x94 0xA5
    assert_eq!(parse_chars(&[0xF0, 0x9F, 0x94, 0xA5]), vec!['🔥']);
}

#[test]
fn test_utf8_musical_symbol() {
    // 𝄞 = U+1D11E = 0xF0 0x9D 0x84 0x9E
    assert_eq!(parse_chars(&[0xF0, 0x9D, 0x84, 0x9E]), vec!['𝄞']);
}

// ============================================================================
// Streaming UTF-8 (split across chunks)
// ============================================================================

#[test]
fn test_utf8_streaming_2byte_split() {
    let mut parser = Parser::new();
    // é = 0xC3 0xA9
    let a1 = parser.parse_collect(&[0xC3]);
    assert!(a1.iter().all(|a| !matches!(a, terminal_parser::Action::Print(_))));
    let a2 = parser.parse_collect(&[0xA9]);
    let chars: Vec<char> = a2.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    assert_eq!(chars, vec!['é']);
}

#[test]
fn test_utf8_streaming_3byte_split_1_2() {
    let mut parser = Parser::new();
    // 中 = 0xE4 0xB8 0xAD
    let a1 = parser.parse_collect(&[0xE4]);
    assert!(a1.iter().all(|a| !matches!(a, terminal_parser::Action::Print(_))));
    let a2 = parser.parse_collect(&[0xB8, 0xAD]);
    let chars: Vec<char> = a2.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    assert_eq!(chars, vec!['中']);
}

#[test]
fn test_utf8_streaming_3byte_split_2_1() {
    let mut parser = Parser::new();
    // 中 = 0xE4 0xB8 0xAD
    let a1 = parser.parse_collect(&[0xE4, 0xB8]);
    assert!(a1.iter().all(|a| !matches!(a, terminal_parser::Action::Print(_))));
    let a2 = parser.parse_collect(&[0xAD]);
    let chars: Vec<char> = a2.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    assert_eq!(chars, vec!['中']);
}

#[test]
fn test_utf8_streaming_3byte_split_1_1_1() {
    let mut parser = Parser::new();
    // 中 = 0xE4 0xB8 0xAD
    let a1 = parser.parse_collect(&[0xE4]);
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(&[0xB8]);
    assert!(a2.is_empty());
    let a3 = parser.parse_collect(&[0xAD]);
    let chars: Vec<char> = a3.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    assert_eq!(chars, vec!['中']);
}

#[test]
fn test_utf8_streaming_4byte_split_1_3() {
    let mut parser = Parser::new();
    // 😀 = 0xF0 0x9F 0x98 0x80
    let a1 = parser.parse_collect(&[0xF0]);
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(&[0x9F, 0x98, 0x80]);
    let chars: Vec<char> = a2.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    assert_eq!(chars, vec!['😀']);
}

#[test]
fn test_utf8_streaming_4byte_split_2_2() {
    let mut parser = Parser::new();
    // 😀 = 0xF0 0x9F 0x98 0x80
    let a1 = parser.parse_collect(&[0xF0, 0x9F]);
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(&[0x98, 0x80]);
    let chars: Vec<char> = a2.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    assert_eq!(chars, vec!['😀']);
}

#[test]
fn test_utf8_streaming_4byte_split_3_1() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(&[0xF0, 0x9F, 0x98]);
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(&[0x80]);
    let chars: Vec<char> = a2.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    assert_eq!(chars, vec!['😀']);
}

#[test]
fn test_utf8_streaming_4byte_split_1_1_1_1() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(&[0xF0]);
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(&[0x9F]);
    assert!(a2.is_empty());
    let a3 = parser.parse_collect(&[0x98]);
    assert!(a3.is_empty());
    let a4 = parser.parse_collect(&[0x80]);
    let chars: Vec<char> = a4.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    assert_eq!(chars, vec!['😀']);
}

// ============================================================================
// Mixed UTF-8 and ASCII
// ============================================================================

#[test]
fn test_utf8_mixed_ascii_and_2byte() {
    assert_eq!(parse_chars("café".as_bytes()), vec!['c', 'a', 'f', 'é']);
}

#[test]
fn test_utf8_mixed_ascii_and_3byte() {
    let chars = parse_chars("Hello 世界".as_bytes());
    assert_eq!(chars, vec!['H', 'e', 'l', 'l', 'o', ' ', '世', '界']);
}

#[test]
fn test_utf8_mixed_ascii_and_4byte() {
    let chars = parse_chars("Hi 🎉".as_bytes());
    assert_eq!(chars, vec!['H', 'i', ' ', '🎉']);
}

#[test]
fn test_utf8_mixed_all_byte_lengths() {
    // A (1-byte), é (2-byte), € (3-byte), 🎉 (4-byte)
    let chars = parse_chars("Aé€🎉".as_bytes());
    assert_eq!(chars, vec!['A', 'é', '€', '🎉']);
}

#[test]
fn test_utf8_japanese_string() {
    let chars = parse_chars("こんにちは".as_bytes());
    assert_eq!(chars, vec!['こ', 'ん', 'に', 'ち', 'は']);
}

#[test]
fn test_utf8_korean_string() {
    let chars = parse_chars("안녕".as_bytes());
    assert_eq!(chars, vec!['안', '녕']);
}

#[test]
fn test_utf8_arabic_string() {
    let chars = parse_chars("مرحبا".as_bytes());
    assert_eq!(chars, vec!['م', 'ر', 'ح', 'ب', 'ا']);
}

#[test]
fn test_utf8_emoji_sequence() {
    let chars = parse_chars("🔥🚀💯".as_bytes());
    assert_eq!(chars, vec!['🔥', '🚀', '💯']);
}

// ============================================================================
// Invalid UTF-8 sequences
// ============================================================================

#[test]
fn test_utf8_invalid_start_byte_0xff() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(&[0xFF]);
    // Should not produce a Print action for invalid bytes
    let prints: Vec<char> = actions.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    assert!(prints.is_empty() || prints == vec!['\u{FFFD}']);
}

#[test]
fn test_utf8_invalid_start_byte_0xfe() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(&[0xFE]);
    let prints: Vec<char> = actions.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    assert!(prints.is_empty() || prints == vec!['\u{FFFD}']);
}

#[test]
fn test_utf8_invalid_continuation_only() {
    let mut parser = Parser::new();
    // 0x80 is a continuation byte without a start byte
    let actions = parser.parse_collect(&[0x80]);
    let prints: Vec<char> = actions.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    assert!(prints.is_empty() || prints == vec!['\u{FFFD}']);
}

#[test]
fn test_utf8_truncated_2byte() {
    let mut parser = Parser::new();
    // Start 2-byte but never finish, then ASCII
    parser.parse_collect(&[0xC3]); // Start of é
    let actions = parser.parse_collect(b"A"); // ASCII should reset
    let prints: Vec<char> = actions.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    // Should eventually get 'A' after handling the incomplete sequence
    assert!(prints.contains(&'A'));
}

#[test]
fn test_utf8_overlong_2byte() {
    let mut parser = Parser::new();
    // Overlong encoding of 'A' (0x41) as 2-byte: 0xC1 0x81
    let actions = parser.parse_collect(&[0xC1, 0x81]);
    let prints: Vec<char> = actions.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    // Should not produce 'A' from overlong encoding
    assert!(!prints.contains(&'A'));
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn test_utf8_empty_input() {
    assert_eq!(parse_chars(b""), vec![]);
}

#[test]
fn test_utf8_null_byte_is_control() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(&[0x00]);
    // Null byte should be treated as control, not print
    let prints: Vec<char> = actions.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    assert!(prints.is_empty());
}

#[test]
fn test_utf8_del_is_not_printed() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(&[0x7F]);
    let prints: Vec<char> = actions.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    assert!(prints.is_empty());
}

#[test]
fn test_utf8_first_two_byte_char() {
    // U+0080 = 0xC2 0x80 (first 2-byte character)
    assert_eq!(parse_chars(&[0xC2, 0x80]), vec!['\u{0080}']);
}

#[test]
fn test_utf8_last_two_byte_char() {
    // U+07FF = 0xDF 0xBF
    assert_eq!(parse_chars(&[0xDF, 0xBF]), vec!['\u{07FF}']);
}

#[test]
fn test_utf8_first_three_byte_char() {
    // U+0800 = 0xE0 0xA0 0x80
    assert_eq!(parse_chars(&[0xE0, 0xA0, 0x80]), vec!['\u{0800}']);
}

#[test]
fn test_utf8_last_three_byte_char() {
    // U+FFFF = 0xEF 0xBF 0xBF
    assert_eq!(parse_chars(&[0xEF, 0xBF, 0xBF]), vec!['\u{FFFF}']);
}

#[test]
fn test_utf8_first_four_byte_char() {
    // U+10000 = 0xF0 0x90 0x80 0x80
    assert_eq!(parse_chars(&[0xF0, 0x90, 0x80, 0x80]), vec!['\u{10000}']);
}

#[test]
fn test_utf8_long_multibyte_string() {
    let input = "日本語テスト文字列テキスト処理";
    let chars: Vec<char> = input.chars().collect();
    assert_eq!(parse_chars(input.as_bytes()), chars);
}

#[test]
fn test_utf8_many_emoji() {
    let input = "🎉🎊🎈🎂🎁🎄🎃🎆🎇✨";
    let chars: Vec<char> = input.chars().collect();
    assert_eq!(parse_chars(input.as_bytes()), chars);
}

#[test]
fn test_utf8_mixed_scripts() {
    let input = "AéΣ中あ🎉";
    let chars: Vec<char> = input.chars().collect();
    assert_eq!(parse_chars(input.as_bytes()), chars);
}

// ============================================================================
// Surrogate pair rejection
// ============================================================================

#[test]
fn test_utf8_surrogate_high_rejected() {
    // U+D800 = 0xED 0xA0 0x80 (high surrogate, invalid in UTF-8)
    let mut parser = Parser::new();
    let actions = parser.parse_collect(&[0xED, 0xA0, 0x80]);
    let prints: Vec<char> = actions.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    // Should not produce a valid char from surrogate range
    for c in &prints {
        let cp = *c as u32;
        assert!(!(0xD800..=0xDFFF).contains(&cp));
    }
}

#[test]
fn test_utf8_surrogate_low_rejected() {
    // U+DC00 = 0xED 0xB0 0x80 (low surrogate, invalid in UTF-8)
    let mut parser = Parser::new();
    let actions = parser.parse_collect(&[0xED, 0xB0, 0x80]);
    let prints: Vec<char> = actions.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    for c in &prints {
        let cp = *c as u32;
        assert!(!(0xD800..=0xDFFF).contains(&cp));
    }
}

// ============================================================================
// UTF-8 in mixed terminal content
// ============================================================================

#[test]
fn test_utf8_between_escape_sequences() {
    let mut parser = Parser::new();
    // Print UTF-8, then CSI, then more UTF-8
    let mut input = Vec::new();
    input.extend_from_slice("日".as_bytes());
    input.extend_from_slice(b"\x1b[1m"); // bold
    input.extend_from_slice("本".as_bytes());
    let actions = parser.parse_collect(&input);
    let prints: Vec<char> = actions.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    assert_eq!(prints, vec!['日', '本']);
}

#[test]
fn test_utf8_after_reset() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b[1m"); // Some CSI
    parser.reset();
    let chars = parser.parse_collect("日本語".as_bytes());
    let prints: Vec<char> = chars.iter().filter_map(|a| match a {
        terminal_parser::Action::Print(c) => Some(*c),
        _ => None,
    }).collect();
    assert_eq!(prints, vec!['日', '本', '語']);
}
