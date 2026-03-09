//! Comprehensive tests for the terminal parser

use terminal_parser::{Action, CsiAction, EscAction, OscAction, Params, Parser, ParserState};

// ============================================================
// Parser Creation Tests
// ============================================================

#[test]
fn test_parser_new() {
    let parser = Parser::new();
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_parser_default() {
    let parser = Parser::default();
    assert_eq!(parser.state(), ParserState::Ground);
}

// ============================================================
// Print Tests
// ============================================================

#[test]
fn test_parser_print_ascii() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"Hello");
    assert_eq!(actions.len(), 5);
    assert_eq!(actions[0], Action::Print('H'));
    assert_eq!(actions[1], Action::Print('e'));
    assert_eq!(actions[2], Action::Print('l'));
    assert_eq!(actions[3], Action::Print('l'));
    assert_eq!(actions[4], Action::Print('o'));
}

#[test]
fn test_parser_print_space() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b" ");
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0], Action::Print(' '));
}

#[test]
fn test_parser_print_tilde() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"~");
    assert_eq!(actions[0], Action::Print('~'));
}

#[test]
fn test_parser_print_at_sign() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"@");
    assert_eq!(actions[0], Action::Print('@'));
}

#[test]
fn test_parser_print_all_printable() {
    let mut parser = Parser::new();
    for byte in 0x20u8..0x7F {
        let actions = parser.parse_collect(&[byte]);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], Action::Print(byte as char));
    }
}

// ============================================================
// UTF-8 Print Tests
// ============================================================

#[test]
fn test_parser_print_utf8_2byte() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect("é".as_bytes());
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0], Action::Print('é'));
}

#[test]
fn test_parser_print_utf8_3byte() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect("中".as_bytes());
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0], Action::Print('中'));
}

#[test]
fn test_parser_print_utf8_4byte() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect("😀".as_bytes());
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0], Action::Print('😀'));
}

#[test]
fn test_parser_print_utf8_mixed() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect("Hello 世界 🎉".as_bytes());
    let chars: Vec<char> = actions
        .iter()
        .filter_map(|a| match a {
            Action::Print(c) => Some(*c),
            _ => None,
        })
        .collect();
    assert_eq!(
        chars,
        vec!['H', 'e', 'l', 'l', 'o', ' ', '世', '界', ' ', '🎉']
    );
}

#[test]
fn test_parser_print_utf8_streaming_2byte() {
    let mut parser = Parser::new();
    // 'é' = 0xC3 0xA9
    let a1 = parser.parse_collect(&[0xC3]);
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(&[0xA9]);
    assert_eq!(a2.len(), 1);
    assert_eq!(a2[0], Action::Print('é'));
}

#[test]
fn test_parser_print_utf8_streaming_3byte() {
    let mut parser = Parser::new();
    // '中' = 0xE4 0xB8 0xAD
    let a1 = parser.parse_collect(&[0xE4]);
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(&[0xB8]);
    assert!(a2.is_empty());
    let a3 = parser.parse_collect(&[0xAD]);
    assert_eq!(a3[0], Action::Print('中'));
}

#[test]
fn test_parser_print_utf8_streaming_4byte() {
    let mut parser = Parser::new();
    // '😀' = 0xF0 0x9F 0x98 0x80
    assert!(parser.parse_collect(&[0xF0]).is_empty());
    assert!(parser.parse_collect(&[0x9F]).is_empty());
    assert!(parser.parse_collect(&[0x98]).is_empty());
    let a = parser.parse_collect(&[0x80]);
    assert_eq!(a[0], Action::Print('😀'));
}

// ============================================================
// Control Character Tests
// ============================================================

#[test]
fn test_parser_control_bel() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(&[0x07]);
    assert_eq!(actions[0], Action::Control(0x07));
}

#[test]
fn test_parser_control_bs() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(&[0x08]);
    assert_eq!(actions[0], Action::Control(0x08));
}

#[test]
fn test_parser_control_ht() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(&[0x09]);
    assert_eq!(actions[0], Action::Control(0x09));
}

#[test]
fn test_parser_control_lf() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(&[0x0A]);
    assert_eq!(actions[0], Action::Control(0x0A));
}

#[test]
fn test_parser_control_vt() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(&[0x0B]);
    assert_eq!(actions[0], Action::Control(0x0B));
}

#[test]
fn test_parser_control_ff() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(&[0x0C]);
    assert_eq!(actions[0], Action::Control(0x0C));
}

#[test]
fn test_parser_control_cr() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(&[0x0D]);
    assert_eq!(actions[0], Action::Control(0x0D));
}

#[test]
fn test_parser_control_all_in_sequence() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(&[0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D]);
    assert_eq!(actions.len(), 7);
    for (i, &byte) in [0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D]
        .iter()
        .enumerate()
    {
        assert_eq!(actions[i], Action::Control(byte));
    }
}

#[test]
fn test_parser_control_mixed_with_print() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"A\x08B\x0AC");
    assert_eq!(actions.len(), 5);
    assert_eq!(actions[0], Action::Print('A'));
    assert_eq!(actions[1], Action::Control(0x08));
    assert_eq!(actions[2], Action::Print('B'));
    assert_eq!(actions[3], Action::Control(0x0A));
    assert_eq!(actions[4], Action::Print('C'));
}

// ============================================================
// CAN/SUB Tests
// ============================================================

#[test]
fn test_parser_can_cancels_sequence() {
    let mut parser = Parser::new();
    // Start CSI, then CAN
    let actions = parser.parse_collect(b"\x1b[10\x18A");
    // CAN should cancel the CSI sequence, then A is printed
    assert_eq!(actions.last().unwrap(), &Action::Print('A'));
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_parser_sub_cancels_sequence() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[10\x1AA");
    assert_eq!(actions.last().unwrap(), &Action::Print('A'));
    assert_eq!(parser.state(), ParserState::Ground);
}

// ============================================================
// ESC Sequence Tests
// ============================================================

#[test]
fn test_parser_esc_save_cursor() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b7");
    assert_eq!(actions[0], Action::Esc(EscAction::SaveCursor));
}

#[test]
fn test_parser_esc_restore_cursor() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b8");
    assert_eq!(actions[0], Action::Esc(EscAction::RestoreCursor));
}

#[test]
fn test_parser_esc_index() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1bD");
    assert_eq!(actions[0], Action::Esc(EscAction::Index));
}

#[test]
fn test_parser_esc_reverse_index() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1bM");
    assert_eq!(actions[0], Action::Esc(EscAction::ReverseIndex));
}

#[test]
fn test_parser_esc_next_line() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1bE");
    assert_eq!(actions[0], Action::Esc(EscAction::NextLine));
}

#[test]
fn test_parser_esc_horizontal_tab_set() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1bH");
    assert_eq!(actions[0], Action::Esc(EscAction::HorizontalTabSet));
}

#[test]
fn test_parser_esc_full_reset() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1bc");
    assert_eq!(actions[0], Action::Esc(EscAction::FullReset));
}

#[test]
fn test_parser_esc_application_keypad() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b=");
    assert_eq!(actions[0], Action::Esc(EscAction::ApplicationKeypad));
}

#[test]
fn test_parser_esc_normal_keypad() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b>");
    assert_eq!(actions[0], Action::Esc(EscAction::NormalKeypad));
}

#[test]
fn test_parser_esc_dec_alignment_test() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b#8");
    assert_eq!(actions[0], Action::Esc(EscAction::DecAlignmentTest));
}

#[test]
fn test_parser_esc_designate_g0() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b(B");
    assert_eq!(actions[0], Action::Esc(EscAction::DesignateG0('B')));
}

#[test]
fn test_parser_esc_designate_g0_dec() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b(0");
    assert_eq!(actions[0], Action::Esc(EscAction::DesignateG0('0')));
}

#[test]
fn test_parser_esc_designate_g1() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b)0");
    assert_eq!(actions[0], Action::Esc(EscAction::DesignateG1('0')));
}

#[test]
fn test_parser_esc_designate_g2() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b*0");
    assert_eq!(actions[0], Action::Esc(EscAction::DesignateG2('0')));
}

#[test]
fn test_parser_esc_designate_g3() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b+0");
    assert_eq!(actions[0], Action::Esc(EscAction::DesignateG3('0')));
}

#[test]
fn test_parser_esc_string_terminator() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b\\");
    // ST when not in string state just returns to ground
    assert!(actions.is_empty());
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_parser_esc_unknown() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1bZ");
    if let Action::Esc(EscAction::Unknown(data)) = &actions[0] {
        assert_eq!(data, &vec![b'Z']);
    } else {
        panic!("Expected unknown ESC action");
    }
}

#[test]
fn test_parser_esc_returns_to_ground() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b7");
    assert_eq!(parser.state(), ParserState::Ground);
}

// ============================================================
// CSI Sequence Tests
// ============================================================

#[test]
fn test_parser_csi_cursor_home() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[H");
    assert_eq!(actions.len(), 1);
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'H');
        assert!(!csi.private);
        assert_eq!(csi.param(0, 1), 1);
        assert_eq!(csi.param(1, 1), 1);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_cursor_position() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[10;20H");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 1), 10);
        assert_eq!(csi.param(1, 1), 20);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_cursor_up() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[5A");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'A');
        assert_eq!(csi.param(0, 1), 5);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_cursor_down() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[3B");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'B');
        assert_eq!(csi.param(0, 1), 3);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_cursor_forward() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[7C");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'C');
        assert_eq!(csi.param(0, 1), 7);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_cursor_backward() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[2D");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'D');
        assert_eq!(csi.param(0, 1), 2);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_erase_display() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[2J");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'J');
        assert_eq!(csi.param(0, 0), 2);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_erase_line() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[K");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'K');
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_insert_lines() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[3L");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'L');
        assert_eq!(csi.param(0, 1), 3);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_delete_lines() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[2M");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'M');
        assert_eq!(csi.param(0, 1), 2);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_scroll_up() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[3S");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'S');
        assert_eq!(csi.param(0, 1), 3);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_scroll_down() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[2T");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'T');
        assert_eq!(csi.param(0, 1), 2);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_set_scroll_region() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[5;20r");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'r');
        assert_eq!(csi.param(0, 1), 5);
        assert_eq!(csi.param(1, 24), 20);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_insert_chars() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[5@");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'@');
        assert_eq!(csi.param(0, 1), 5);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_delete_chars() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[3P");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'P');
        assert_eq!(csi.param(0, 1), 3);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_erase_chars() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[4X");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'X');
        assert_eq!(csi.param(0, 1), 4);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_tab_clear() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[3g");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'g');
        assert_eq!(csi.param(0, 0), 3);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_set_column() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[10G");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'G');
        assert_eq!(csi.param(0, 1), 10);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_set_row() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[5d");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'd');
        assert_eq!(csi.param(0, 1), 5);
    } else {
        panic!("Expected CSI");
    }
}

// ============================================================
// CSI Private Mode Tests
// ============================================================

#[test]
fn test_parser_csi_private_mode_set() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[?25h");
    if let Action::Csi(csi) = &actions[0] {
        assert!(csi.private);
        assert_eq!(csi.final_byte, b'h');
        assert_eq!(csi.param(0, 0), 25);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_private_mode_reset() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[?25l");
    if let Action::Csi(csi) = &actions[0] {
        assert!(csi.private);
        assert_eq!(csi.final_byte, b'l');
        assert_eq!(csi.param(0, 0), 25);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_private_alternate_screen() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[?1049h");
    if let Action::Csi(csi) = &actions[0] {
        assert!(csi.private);
        assert_eq!(csi.param(0, 0), 1049);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_private_bracketed_paste() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[?2004h");
    if let Action::Csi(csi) = &actions[0] {
        assert!(csi.private);
        assert_eq!(csi.param(0, 0), 2004);
    } else {
        panic!("Expected CSI");
    }
}

// ============================================================
// CSI SGR Tests
// ============================================================

#[test]
fn test_parser_csi_sgr_reset() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[0m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'm');
        assert_eq!(csi.params.len(), 1);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_sgr_bold() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[1m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 0), 1);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_sgr_multiple() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[1;31;42m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.params.len(), 3);
        assert_eq!(csi.param(0, 0), 1);
        assert_eq!(csi.param(1, 0), 31);
        assert_eq!(csi.param(2, 0), 42);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_sgr_256_color() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[38;5;196m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 0), 38);
        assert_eq!(csi.param(1, 0), 5);
        assert_eq!(csi.param(2, 0), 196);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_sgr_rgb_color() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[38;2;255;128;64m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 0), 38);
        assert_eq!(csi.param(1, 0), 2);
        assert_eq!(csi.param(2, 0), 255);
        assert_eq!(csi.param(3, 0), 128);
        assert_eq!(csi.param(4, 0), 64);
    } else {
        panic!("Expected CSI");
    }
}

// ============================================================
// CSI is / is_private Tests
// ============================================================

#[test]
fn test_csi_action_is() {
    let csi = CsiAction {
        params: Params::new(),
        intermediates: vec![],
        final_byte: b'H',
        private: false,
        prefix: None,
    };
    assert!(csi.is(b'H'));
    assert!(!csi.is(b'J'));
}

#[test]
fn test_csi_action_is_with_intermediates() {
    let csi = CsiAction {
        params: Params::new(),
        intermediates: vec![b' '],
        final_byte: b'q',
        private: false,
        prefix: None,
    };
    assert!(!csi.is(b'q')); // has intermediates, so is() returns false
}

#[test]
fn test_csi_action_is_private() {
    let csi = CsiAction {
        params: Params::new(),
        intermediates: vec![],
        final_byte: b'h',
        private: true,
        prefix: Some(b'?'),
    };
    assert!(csi.is_private(b'h'));
    assert!(!csi.is(b'h'));
}

#[test]
fn test_csi_action_param_defaults() {
    let csi = CsiAction {
        params: Params::from_slice(&[10, 20, 30]),
        intermediates: vec![],
        final_byte: b'H',
        private: false,
        prefix: None,
    };
    assert_eq!(csi.param(0, 1), 10);
    assert_eq!(csi.param(1, 1), 20);
    assert_eq!(csi.param(5, 99), 99); // Out of range
}

// ============================================================
// OSC Sequence Tests
// ============================================================

#[test]
fn test_parser_osc_set_icon_and_title() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]0;My Title\x07");
    if let Action::Osc(OscAction::SetIconAndTitle(title)) = &actions[0] {
        assert_eq!(title, "My Title");
    } else {
        panic!("Expected OSC SetIconAndTitle");
    }
}

#[test]
fn test_parser_osc_set_icon_name() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]1;My Icon\x07");
    if let Action::Osc(OscAction::SetIconName(name)) = &actions[0] {
        assert_eq!(name, "My Icon");
    } else {
        panic!("Expected OSC SetIconName");
    }
}

#[test]
fn test_parser_osc_set_title() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]2;Window Title\x07");
    if let Action::Osc(OscAction::SetTitle(title)) = &actions[0] {
        assert_eq!(title, "Window Title");
    } else {
        panic!("Expected OSC SetTitle");
    }
}

#[test]
fn test_parser_osc_set_color() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]4;1;#ff0000\x07");
    if let Action::Osc(OscAction::SetColor { index, color }) = &actions[0] {
        assert_eq!(*index, 1);
        assert_eq!(color, "#ff0000");
    } else {
        panic!("Expected OSC SetColor");
    }
}

#[test]
fn test_parser_osc_set_current_directory() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]7;file:///home/user\x07");
    if let Action::Osc(OscAction::SetCurrentDirectory(dir)) = &actions[0] {
        assert_eq!(dir, "file:///home/user");
    } else {
        panic!("Expected OSC SetCurrentDirectory");
    }
}

#[test]
fn test_parser_osc_hyperlink() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]8;;https://example.com\x07");
    if let Action::Osc(OscAction::Hyperlink { params, uri }) = &actions[0] {
        assert_eq!(params, "");
        assert_eq!(uri, "https://example.com");
    } else {
        panic!("Expected OSC Hyperlink");
    }
}

#[test]
fn test_parser_osc_hyperlink_with_params() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]8;id=link1;https://example.com\x07");
    if let Action::Osc(OscAction::Hyperlink { params, uri }) = &actions[0] {
        assert_eq!(params, "id=link1");
        assert_eq!(uri, "https://example.com");
    } else {
        panic!("Expected OSC Hyperlink");
    }
}

#[test]
fn test_parser_osc_set_foreground_color() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]10;#ffffff\x07");
    if let Action::Osc(OscAction::SetForegroundColor(color)) = &actions[0] {
        assert_eq!(color, "#ffffff");
    } else {
        panic!("Expected OSC SetForegroundColor");
    }
}

#[test]
fn test_parser_osc_set_background_color() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]11;#000000\x07");
    if let Action::Osc(OscAction::SetBackgroundColor(color)) = &actions[0] {
        assert_eq!(color, "#000000");
    } else {
        panic!("Expected OSC SetBackgroundColor");
    }
}

#[test]
fn test_parser_osc_set_cursor_color() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]12;#00ff00\x07");
    if let Action::Osc(OscAction::SetCursorColor(color)) = &actions[0] {
        assert_eq!(color, "#00ff00");
    } else {
        panic!("Expected OSC SetCursorColor");
    }
}

#[test]
fn test_parser_osc_clipboard() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]52;c;SGVsbG8=\x07");
    if let Action::Osc(OscAction::Clipboard { clipboard, data }) = &actions[0] {
        assert_eq!(clipboard, "c");
        assert_eq!(data, "SGVsbG8=");
    } else {
        panic!("Expected OSC Clipboard");
    }
}

#[test]
fn test_parser_osc_reset_color() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]104;5\x07");
    if let Action::Osc(OscAction::ResetColor(index)) = &actions[0] {
        assert_eq!(*index, Some(5));
    } else {
        panic!("Expected OSC ResetColor");
    }
}

#[test]
fn test_parser_osc_reset_color_all() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]104;\x07");
    if let Action::Osc(OscAction::ResetColor(index)) = &actions[0] {
        assert_eq!(*index, None);
    } else {
        panic!("Expected OSC ResetColor");
    }
}

#[test]
fn test_parser_osc_reset_fg_color() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]110;\x07");
    assert_eq!(actions[0], Action::Osc(OscAction::ResetForegroundColor));
}

#[test]
fn test_parser_osc_reset_bg_color() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]111;\x07");
    assert_eq!(actions[0], Action::Osc(OscAction::ResetBackgroundColor));
}

#[test]
fn test_parser_osc_reset_cursor_color() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]112;\x07");
    assert_eq!(actions[0], Action::Osc(OscAction::ResetCursorColor));
}

#[test]
fn test_parser_osc_unknown() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]999;data\x07");
    if let Action::Osc(OscAction::Unknown { command, data }) = &actions[0] {
        assert_eq!(*command, 999);
        assert_eq!(data, "data");
    } else {
        panic!("Expected OSC Unknown");
    }
}

#[test]
fn test_parser_osc_terminated_by_st() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]0;Title\x1b\\");
    assert_eq!(actions.len(), 1);
    if let Action::Osc(OscAction::SetIconAndTitle(title)) = &actions[0] {
        assert_eq!(title, "Title");
    } else {
        panic!("Expected OSC SetIconAndTitle");
    }
}

// ============================================================
// Streaming / Chunk Boundary Tests
// ============================================================

#[test]
fn test_parser_streaming_csi() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"\x1b[10");
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(b";20H");
    assert_eq!(a2.len(), 1);
    if let Action::Csi(csi) = &a2[0] {
        assert_eq!(csi.param(0, 1), 10);
        assert_eq!(csi.param(1, 1), 20);
    }
}

#[test]
fn test_parser_streaming_esc() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"\x1b");
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(b"7");
    assert_eq!(a2[0], Action::Esc(EscAction::SaveCursor));
}

#[test]
fn test_parser_streaming_osc() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"\x1b]0;My ");
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(b"Title\x07");
    if let Action::Osc(OscAction::SetIconAndTitle(title)) = &a2[0] {
        assert_eq!(title, "My Title");
    }
}

#[test]
fn test_parser_byte_at_a_time() {
    let mut parser = Parser::new();
    let input = b"\x1b[10;20H";
    let mut all_actions = Vec::new();
    for &byte in input {
        let actions = parser.parse_collect(&[byte]);
        all_actions.extend(actions);
    }
    assert_eq!(all_actions.len(), 1);
    if let Action::Csi(csi) = &all_actions[0] {
        assert_eq!(csi.param(0, 1), 10);
        assert_eq!(csi.param(1, 1), 20);
    }
}

// ============================================================
// Parser State Tests
// ============================================================

#[test]
fn test_parser_state_ground_initial() {
    let parser = Parser::new();
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_parser_state_escape_after_esc() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b");
    assert_eq!(parser.state(), ParserState::Escape);
}

#[test]
fn test_parser_state_csi_param() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b[10");
    assert_eq!(parser.state(), ParserState::CsiParam);
}

#[test]
fn test_parser_state_osc_string() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b]0;test");
    assert_eq!(parser.state(), ParserState::OscString);
}

#[test]
fn test_parser_state_returns_ground_after_sequence() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b[H");
    assert_eq!(parser.state(), ParserState::Ground);
}

// ============================================================
// Parser Reset Tests
// ============================================================

#[test]
fn test_parser_reset_from_csi() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b[10");
    assert_eq!(parser.state(), ParserState::CsiParam);
    parser.reset();
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_parser_reset_then_parse() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b[10");
    parser.reset();
    let actions = parser.parse_collect(b"A");
    assert_eq!(actions[0], Action::Print('A'));
}

// ============================================================
// DCS Sequence Tests
// ============================================================

#[test]
fn test_parser_dcs_basic() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1bPq\x1b\\");
    // Should produce a DCS action
    let has_dcs = actions.iter().any(|a| matches!(a, Action::Dcs { .. }));
    assert!(has_dcs);
}

// ============================================================
// Multiple Sequences Tests
// ============================================================

#[test]
fn test_parser_multiple_csi_sequences() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[1m\x1b[31m\x1b[42m");
    assert_eq!(actions.len(), 3);
    for action in &actions {
        assert!(matches!(action, Action::Csi(_)));
    }
}

#[test]
fn test_parser_mixed_text_and_sequences() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"Hello\x1b[1mWorld\x1b[0m!");
    let prints: Vec<char> = actions
        .iter()
        .filter_map(|a| match a {
            Action::Print(c) => Some(*c),
            _ => None,
        })
        .collect();
    assert_eq!(
        prints,
        vec!['H', 'e', 'l', 'l', 'o', 'W', 'o', 'r', 'l', 'd', '!']
    );
}

#[test]
fn test_parser_esc_sequence_pair() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b7\x1b8");
    assert_eq!(actions.len(), 2);
    assert_eq!(actions[0], Action::Esc(EscAction::SaveCursor));
    assert_eq!(actions[1], Action::Esc(EscAction::RestoreCursor));
}

// ============================================================
// parse() callback API Tests
// ============================================================

#[test]
fn test_parser_parse_callback() {
    let mut parser = Parser::new();
    let mut actions = Vec::new();
    parser.parse(b"AB", |action| actions.push(action));
    assert_eq!(actions.len(), 2);
    assert_eq!(actions[0], Action::Print('A'));
    assert_eq!(actions[1], Action::Print('B'));
}

// ============================================================
// Params Tests
// ============================================================

#[test]
fn test_params_new() {
    let params = Params::new();
    assert!(params.is_empty());
    assert_eq!(params.len(), 0);
}

#[test]
fn test_params_default() {
    let params = Params::default();
    assert!(params.is_empty());
}

#[test]
fn test_params_from_slice() {
    let params = Params::from_slice(&[1, 2, 3]);
    assert_eq!(params.len(), 3);
    assert_eq!(params.get(0), Some(1));
    assert_eq!(params.get(1), Some(2));
    assert_eq!(params.get(2), Some(3));
}

#[test]
fn test_params_parse_single() {
    let params = Params::parse(b"42");
    assert_eq!(params.get(0), Some(42));
}

#[test]
fn test_params_parse_multiple() {
    let params = Params::parse(b"1;2;3");
    assert_eq!(params.len(), 3);
}

#[test]
fn test_params_parse_defaults() {
    let params = Params::parse(b";5;");
    assert_eq!(params.get(0), None); // 0 treated as default
    assert_eq!(params.get(1), Some(5));
    assert_eq!(params.get(2), None);
}

#[test]
fn test_params_get_or() {
    let params = Params::parse(b";5");
    assert_eq!(params.get_or(0, 1), 1); // Default
    assert_eq!(params.get_or(1, 1), 5);
    assert_eq!(params.get_or(5, 99), 99); // Out of range
}

#[test]
fn test_params_raw() {
    let params = Params::parse(b";5");
    assert_eq!(params.raw(0), 0);
    assert_eq!(params.raw(1), 5);
    assert_eq!(params.raw(5), 0); // Out of range
}

#[test]
fn test_params_overflow_saturates() {
    let params = Params::parse(b"99999");
    assert_eq!(params.get(0), Some(65535));
}

#[test]
fn test_params_iter() {
    let params = Params::parse(b"1;2;3");
    let values: Vec<u16> = params.iter().collect();
    assert_eq!(values, vec![1, 2, 3]);
}

#[test]
fn test_params_empty_string() {
    let params = Params::parse(b"");
    assert!(params.is_empty());
}

#[test]
fn test_params_subparams() {
    let params = Params::parse(b"38:2:255:128:64");
    assert_eq!(params.len(), 1);
    assert!(params.subparams(0).is_some());
}

#[test]
fn test_params_iter_with_subparams() {
    let params = Params::parse(b"1;2;3");
    let count = params.iter_with_subparams().count();
    assert_eq!(count, 3);
}

#[test]
fn test_params_equality() {
    let p1 = Params::from_slice(&[1, 2, 3]);
    let p2 = Params::from_slice(&[1, 2, 3]);
    assert_eq!(p1, p2);
}

#[test]
fn test_params_inequality() {
    let p1 = Params::from_slice(&[1, 2, 3]);
    let p2 = Params::from_slice(&[1, 2, 4]);
    assert_ne!(p1, p2);
}

#[test]
fn test_params_clone() {
    let p1 = Params::from_slice(&[1, 2, 3]);
    let p2 = p1.clone();
    assert_eq!(p1, p2);
}
