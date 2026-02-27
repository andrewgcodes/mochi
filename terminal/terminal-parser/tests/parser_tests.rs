//! Comprehensive tests for the terminal escape sequence parser

use terminal_parser::{Action, CsiAction, EscAction, OscAction, Parser, ParserState};

// ============================================================================
// Parser Creation and State
// ============================================================================

#[test]
fn test_parser_new_ground_state() {
    let parser = Parser::new();
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_parser_default_ground_state() {
    let parser = Parser::default();
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_parser_reset_to_ground() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b[10");
    assert_ne!(parser.state(), ParserState::Ground);
    parser.reset();
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_parser_reset_allows_normal_parsing() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b[");
    parser.reset();
    let actions = parser.parse_collect(b"A");
    assert_eq!(actions[0], Action::Print('A'));
}

// ============================================================================
// Print actions
// ============================================================================

#[test]
fn test_parser_print_single_char() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"A");
    assert_eq!(actions, vec![Action::Print('A')]);
}

#[test]
fn test_parser_print_string() {
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
fn test_parser_print_digits() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"123");
    assert_eq!(actions.len(), 3);
    assert_eq!(actions[0], Action::Print('1'));
    assert_eq!(actions[1], Action::Print('2'));
    assert_eq!(actions[2], Action::Print('3'));
}

#[test]
fn test_parser_print_special_chars() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"@#$%^&*");
    assert_eq!(actions.len(), 7);
    assert_eq!(actions[0], Action::Print('@'));
}

#[test]
fn test_parser_print_space() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b" ");
    assert_eq!(actions, vec![Action::Print(' ')]);
}

#[test]
fn test_parser_print_tilde() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"~");
    assert_eq!(actions, vec![Action::Print('~')]);
}

// ============================================================================
// Control characters
// ============================================================================

#[test]
fn test_parser_control_bel() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x07");
    assert_eq!(actions, vec![Action::Control(0x07)]);
}

#[test]
fn test_parser_control_bs() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x08");
    assert_eq!(actions, vec![Action::Control(0x08)]);
}

#[test]
fn test_parser_control_ht() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x09");
    assert_eq!(actions, vec![Action::Control(0x09)]);
}

#[test]
fn test_parser_control_lf() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x0A");
    assert_eq!(actions, vec![Action::Control(0x0A)]);
}

#[test]
fn test_parser_control_vt() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x0B");
    assert_eq!(actions, vec![Action::Control(0x0B)]);
}

#[test]
fn test_parser_control_ff() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x0C");
    assert_eq!(actions, vec![Action::Control(0x0C)]);
}

#[test]
fn test_parser_control_cr() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x0D");
    assert_eq!(actions, vec![Action::Control(0x0D)]);
}

#[test]
fn test_parser_control_so_ignored() {
    // SO (0x0E) is not in the BEL-CR range, parser ignores it
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x0E");
    assert!(actions.is_empty());
}

#[test]
fn test_parser_control_si_ignored() {
    // SI (0x0F) is not in the BEL-CR range, parser ignores it
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x0F");
    assert!(actions.is_empty());
}

#[test]
fn test_parser_multiple_controls() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x07\x08\x09\x0A\x0D");
    assert_eq!(actions.len(), 5);
    assert_eq!(actions[0], Action::Control(0x07));
    assert_eq!(actions[1], Action::Control(0x08));
    assert_eq!(actions[2], Action::Control(0x09));
    assert_eq!(actions[3], Action::Control(0x0A));
    assert_eq!(actions[4], Action::Control(0x0D));
}

#[test]
fn test_parser_mixed_print_and_control() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"A\x08B");
    assert_eq!(actions.len(), 3);
    assert_eq!(actions[0], Action::Print('A'));
    assert_eq!(actions[1], Action::Control(0x08));
    assert_eq!(actions[2], Action::Print('B'));
}

// ============================================================================
// ESC sequences
// ============================================================================

#[test]
fn test_parser_esc_save_cursor() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b7");
    assert_eq!(actions, vec![Action::Esc(EscAction::SaveCursor)]);
}

#[test]
fn test_parser_esc_restore_cursor() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b8");
    assert_eq!(actions, vec![Action::Esc(EscAction::RestoreCursor)]);
}

#[test]
fn test_parser_esc_index() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1bD");
    assert_eq!(actions, vec![Action::Esc(EscAction::Index)]);
}

#[test]
fn test_parser_esc_reverse_index() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1bM");
    assert_eq!(actions, vec![Action::Esc(EscAction::ReverseIndex)]);
}

#[test]
fn test_parser_esc_next_line() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1bE");
    assert_eq!(actions, vec![Action::Esc(EscAction::NextLine)]);
}

#[test]
fn test_parser_esc_horizontal_tab_set() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1bH");
    assert_eq!(actions, vec![Action::Esc(EscAction::HorizontalTabSet)]);
}

#[test]
fn test_parser_esc_full_reset() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1bc");
    assert_eq!(actions, vec![Action::Esc(EscAction::FullReset)]);
}

#[test]
fn test_parser_esc_application_keypad() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b=");
    assert_eq!(actions, vec![Action::Esc(EscAction::ApplicationKeypad)]);
}

#[test]
fn test_parser_esc_normal_keypad() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b>");
    assert_eq!(actions, vec![Action::Esc(EscAction::NormalKeypad)]);
}

#[test]
fn test_parser_esc_designate_g0_ascii() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b(B");
    assert_eq!(actions, vec![Action::Esc(EscAction::DesignateG0('B'))]);
}

#[test]
fn test_parser_esc_designate_g0_dec() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b(0");
    assert_eq!(actions, vec![Action::Esc(EscAction::DesignateG0('0'))]);
}

#[test]
fn test_parser_esc_designate_g1_ascii() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b)B");
    assert_eq!(actions, vec![Action::Esc(EscAction::DesignateG1('B'))]);
}

#[test]
fn test_parser_esc_designate_g1_dec() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b)0");
    assert_eq!(actions, vec![Action::Esc(EscAction::DesignateG1('0'))]);
}

#[test]
fn test_parser_esc_designate_g2() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b*B");
    assert_eq!(actions, vec![Action::Esc(EscAction::DesignateG2('B'))]);
}

#[test]
fn test_parser_esc_designate_g3() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b+B");
    assert_eq!(actions, vec![Action::Esc(EscAction::DesignateG3('B'))]);
}

#[test]
fn test_parser_esc_dec_alignment_test() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b#8");
    assert_eq!(actions, vec![Action::Esc(EscAction::DecAlignmentTest)]);
}

#[test]
fn test_parser_multiple_esc_sequences() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b7\x1b8\x1bD\x1bM\x1bE");
    assert_eq!(actions.len(), 5);
    assert_eq!(actions[0], Action::Esc(EscAction::SaveCursor));
    assert_eq!(actions[1], Action::Esc(EscAction::RestoreCursor));
    assert_eq!(actions[2], Action::Esc(EscAction::Index));
    assert_eq!(actions[3], Action::Esc(EscAction::ReverseIndex));
    assert_eq!(actions[4], Action::Esc(EscAction::NextLine));
}

// ============================================================================
// CSI sequences
// ============================================================================

#[test]
fn test_parser_csi_cursor_up() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[5A");
    assert_eq!(actions.len(), 1);
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'A');
        assert_eq!(csi.param(0, 1), 5);
        assert!(!csi.private);
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
    let actions = parser.parse_collect(b"\x1b[10C");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'C');
        assert_eq!(csi.param(0, 1), 10);
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
fn test_parser_csi_cursor_position() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[10;20H");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'H');
        assert_eq!(csi.param(0, 1), 10);
        assert_eq!(csi.param(1, 1), 20);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_cursor_position_default() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[H");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'H');
        assert_eq!(csi.param(0, 1), 1);
        assert_eq!(csi.param(1, 1), 1);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_erase_display_below() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[J");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'J');
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_erase_display_above() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[1J");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'J');
        assert_eq!(csi.param(0, 0), 1);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_erase_display_all() {
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
fn test_parser_csi_erase_line_to_end() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[K");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'K');
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_erase_line_to_start() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[1K");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'K');
        assert_eq!(csi.param(0, 0), 1);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_erase_entire_line() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[2K");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'K');
        assert_eq!(csi.param(0, 0), 2);
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
fn test_parser_csi_insert_chars() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[4@");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'@');
        assert_eq!(csi.param(0, 1), 4);
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
    let actions = parser.parse_collect(b"\x1b[5X");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'X');
        assert_eq!(csi.param(0, 1), 5);
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
        assert_eq!(csi.param(1, 1), 20);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_cursor_next_line() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[3E");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'E');
        assert_eq!(csi.param(0, 1), 3);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_cursor_prev_line() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[2F");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'F');
        assert_eq!(csi.param(0, 1), 2);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_csi_cursor_horizontal_absolute() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[15G");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'G');
        assert_eq!(csi.param(0, 1), 15);
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

// ============================================================================
// CSI SGR (Select Graphic Rendition)
// ============================================================================

#[test]
fn test_parser_sgr_reset() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[0m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'm');
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_sgr_bold() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[1m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 0), 1);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_sgr_dim() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[2m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 0), 2);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_sgr_italic() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[3m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 0), 3);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_sgr_underline() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[4m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 0), 4);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_sgr_blink() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[5m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 0), 5);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_sgr_inverse() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[7m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 0), 7);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_sgr_invisible() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[8m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 0), 8);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_sgr_strikethrough() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[9m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 0), 9);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_sgr_fg_black() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[30m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 0), 30);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_sgr_fg_red() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[31m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 0), 31);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_sgr_fg_green() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[32m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 0), 32);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_sgr_bg_blue() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[44m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 0), 44);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_sgr_256_fg() {
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
fn test_parser_sgr_256_bg() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[48;5;100m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.param(0, 0), 48);
        assert_eq!(csi.param(1, 0), 5);
        assert_eq!(csi.param(2, 0), 100);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_sgr_rgb_fg() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[38;2;255;128;64m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.params.len(), 5);
        assert_eq!(csi.param(0, 0), 38);
        assert_eq!(csi.param(1, 0), 2);
        assert_eq!(csi.param(2, 0), 255);
        assert_eq!(csi.param(3, 0), 128);
        assert_eq!(csi.param(4, 0), 64);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_sgr_combined() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[1;31;42m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.params.len(), 3);
        assert_eq!(csi.param(0, 0), 1); // bold
        assert_eq!(csi.param(1, 0), 31); // fg red
        assert_eq!(csi.param(2, 0), 42); // bg green
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_sgr_no_params_is_reset() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[m");
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'm');
    } else {
        panic!("Expected CSI");
    }
}

// ============================================================================
// CSI Private modes (DEC)
// ============================================================================

#[test]
fn test_parser_dec_cursor_visible_set() {
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
fn test_parser_dec_cursor_visible_reset() {
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
fn test_parser_dec_alt_screen_set() {
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
fn test_parser_dec_alt_screen_reset() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[?1049l");
    if let Action::Csi(csi) = &actions[0] {
        assert!(csi.private);
        assert_eq!(csi.param(0, 0), 1049);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_dec_bracketed_paste_set() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[?2004h");
    if let Action::Csi(csi) = &actions[0] {
        assert!(csi.private);
        assert_eq!(csi.param(0, 0), 2004);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_dec_mouse_vt200() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[?1000h");
    if let Action::Csi(csi) = &actions[0] {
        assert!(csi.private);
        assert_eq!(csi.param(0, 0), 1000);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_dec_sgr_mouse() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[?1006h");
    if let Action::Csi(csi) = &actions[0] {
        assert!(csi.private);
        assert_eq!(csi.param(0, 0), 1006);
    } else {
        panic!("Expected CSI");
    }
}

#[test]
fn test_parser_dec_focus_events() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[?1004h");
    if let Action::Csi(csi) = &actions[0] {
        assert!(csi.private);
        assert_eq!(csi.param(0, 0), 1004);
    } else {
        panic!("Expected CSI");
    }
}

// ============================================================================
// OSC sequences
// ============================================================================

#[test]
fn test_parser_osc_set_title_bel() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]0;My Title\x07");
    if let Action::Osc(OscAction::SetIconAndTitle(title)) = &actions[0] {
        assert_eq!(title, "My Title");
    } else {
        panic!("Expected OSC SetIconAndTitle, got {:?}", actions);
    }
}

#[test]
fn test_parser_osc_set_title_st() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]0;My Title\x1b\\");
    // Should produce OSC action
    let osc_actions: Vec<_> = actions
        .iter()
        .filter(|a| matches!(a, Action::Osc(_)))
        .collect();
    assert!(!osc_actions.is_empty());
}

#[test]
fn test_parser_osc_set_icon_name() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]1;icon\x07");
    if let Action::Osc(OscAction::SetIconName(name)) = &actions[0] {
        assert_eq!(name, "icon");
    } else {
        panic!("Expected OSC SetIconName");
    }
}

#[test]
fn test_parser_osc_set_title_only() {
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
    let actions = parser.parse_collect(b"\x1b]4;0;#000000\x07");
    if let Action::Osc(OscAction::SetColor { index, color }) = &actions[0] {
        assert_eq!(*index, 0);
        assert_eq!(color, "#000000");
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
fn test_parser_osc_hyperlink_with_id() {
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
    let actions = parser.parse_collect(b"\x1b]10;rgb:ff/ff/ff\x07");
    if let Action::Osc(OscAction::SetForegroundColor(color)) = &actions[0] {
        assert_eq!(color, "rgb:ff/ff/ff");
    } else {
        panic!("Expected OSC SetForegroundColor");
    }
}

#[test]
fn test_parser_osc_set_background_color() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]11;rgb:00/00/00\x07");
    if let Action::Osc(OscAction::SetBackgroundColor(color)) = &actions[0] {
        assert_eq!(color, "rgb:00/00/00");
    } else {
        panic!("Expected OSC SetBackgroundColor");
    }
}

#[test]
fn test_parser_osc_set_cursor_color() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]12;rgb:ff/00/00\x07");
    if let Action::Osc(OscAction::SetCursorColor(color)) = &actions[0] {
        assert_eq!(color, "rgb:ff/00/00");
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
fn test_parser_osc_reset_foreground() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]110;\x07");
    if let Action::Osc(OscAction::ResetForegroundColor) = &actions[0] {
        // ok
    } else {
        panic!("Expected OSC ResetForegroundColor");
    }
}

#[test]
fn test_parser_osc_reset_background() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]111;\x07");
    if let Action::Osc(OscAction::ResetBackgroundColor) = &actions[0] {
        // ok
    } else {
        panic!("Expected OSC ResetBackgroundColor");
    }
}

#[test]
fn test_parser_osc_reset_cursor() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]112;\x07");
    if let Action::Osc(OscAction::ResetCursorColor) = &actions[0] {
        // ok
    } else {
        panic!("Expected OSC ResetCursorColor");
    }
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
fn test_parser_osc_empty_title() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]0;\x07");
    if let Action::Osc(OscAction::SetIconAndTitle(title)) = &actions[0] {
        assert_eq!(title, "");
    } else {
        panic!("Expected OSC SetIconAndTitle");
    }
}

// ============================================================================
// Streaming / chunk boundary tests
// ============================================================================

#[test]
fn test_parser_streaming_csi_split_at_bracket() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"\x1b");
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(b"[10;20H");
    assert_eq!(a2.len(), 1);
    if let Action::Csi(csi) = &a2[0] {
        assert_eq!(csi.param(0, 1), 10);
        assert_eq!(csi.param(1, 1), 20);
    }
}

#[test]
fn test_parser_streaming_csi_split_at_params() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"\x1b[10");
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(b";20H");
    assert_eq!(a2.len(), 1);
}

#[test]
fn test_parser_streaming_csi_split_at_semicolon() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"\x1b[10;");
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(b"20H");
    assert_eq!(a2.len(), 1);
}

#[test]
fn test_parser_streaming_osc_split() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"\x1b]0;My");
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(b" Title\x07");
    if let Action::Osc(OscAction::SetIconAndTitle(title)) = &a2[0] {
        assert_eq!(title, "My Title");
    }
}

#[test]
fn test_parser_streaming_utf8_split() {
    let mut parser = Parser::new();
    // '中' = 0xE4 0xB8 0xAD
    let a1 = parser.parse_collect(&[0xE4]);
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(&[0xB8]);
    assert!(a2.is_empty());
    let a3 = parser.parse_collect(&[0xAD]);
    assert_eq!(a3, vec![Action::Print('中')]);
}

#[test]
fn test_parser_streaming_byte_at_a_time() {
    let mut parser = Parser::new();
    let input = b"\x1b[1;31mHello\x1b[0m";
    let mut all_actions = vec![];
    for byte in input {
        let actions = parser.parse_collect(&[*byte]);
        all_actions.extend(actions);
    }
    // Should have: CSI (bold+red), H, e, l, l, o, CSI (reset)
    let prints: Vec<char> = all_actions
        .iter()
        .filter_map(|a| match a {
            Action::Print(c) => Some(*c),
            _ => None,
        })
        .collect();
    assert_eq!(prints, vec!['H', 'e', 'l', 'l', 'o']);
    let csi_count = all_actions
        .iter()
        .filter(|a| matches!(a, Action::Csi(_)))
        .count();
    assert_eq!(csi_count, 2);
}

// ============================================================================
// Complex sequences
// ============================================================================

#[test]
fn test_parser_print_between_csi() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[1mHello\x1b[0m");
    let prints: Vec<char> = actions
        .iter()
        .filter_map(|a| match a {
            Action::Print(c) => Some(*c),
            _ => None,
        })
        .collect();
    assert_eq!(prints, vec!['H', 'e', 'l', 'l', 'o']);
}

#[test]
fn test_parser_controls_between_text() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"A\x0D\x0AB");
    assert_eq!(actions.len(), 4);
    assert_eq!(actions[0], Action::Print('A'));
    assert_eq!(actions[1], Action::Control(0x0D));
    assert_eq!(actions[2], Action::Control(0x0A));
    assert_eq!(actions[3], Action::Print('B'));
}

#[test]
fn test_parser_esc_between_text() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"A\x1b7B");
    assert_eq!(actions.len(), 3);
    assert_eq!(actions[0], Action::Print('A'));
    assert_eq!(actions[1], Action::Esc(EscAction::SaveCursor));
    assert_eq!(actions[2], Action::Print('B'));
}

#[test]
fn test_parser_multiple_sgr_sequences() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[1m\x1b[31m\x1b[42m");
    let csi_count = actions
        .iter()
        .filter(|a| matches!(a, Action::Csi(_)))
        .count();
    assert_eq!(csi_count, 3);
}

// ============================================================================
// Parser state transitions
// ============================================================================

#[test]
fn test_parser_state_ground_to_escape() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b");
    assert_eq!(parser.state(), ParserState::Escape);
}

#[test]
fn test_parser_state_escape_to_csi() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b[");
    assert_eq!(parser.state(), ParserState::CsiEntry);
}

#[test]
fn test_parser_state_csi_param() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b[1");
    assert_eq!(parser.state(), ParserState::CsiParam);
}

#[test]
fn test_parser_state_osc_string() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b]");
    assert_eq!(parser.state(), ParserState::OscString);
}

#[test]
fn test_parser_state_returns_to_ground_after_csi() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b[1m");
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_parser_state_returns_to_ground_after_osc() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b]0;title\x07");
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_parser_state_returns_to_ground_after_esc() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b7");
    assert_eq!(parser.state(), ParserState::Ground);
}

// ============================================================================
// UTF-8 through parser
// ============================================================================

#[test]
fn test_parser_utf8_hello_world_cjk() {
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
fn test_parser_utf8_mixed_with_escapes() {
    let mut parser = Parser::new();
    let mut input = Vec::new();
    input.extend_from_slice(b"\x1b[1m");
    input.extend_from_slice("日本語".as_bytes());
    input.extend_from_slice(b"\x1b[0m");
    let actions = parser.parse_collect(&input);
    let prints: Vec<char> = actions
        .iter()
        .filter_map(|a| match a {
            Action::Print(c) => Some(*c),
            _ => None,
        })
        .collect();
    assert_eq!(prints, vec!['日', '本', '語']);
}

// ============================================================================
// DCS sequences
// ============================================================================

#[test]
fn test_parser_dcs_sequence() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1bPdata\x1b\\");
    let dcs_count = actions
        .iter()
        .filter(|a| matches!(a, Action::Dcs { .. }))
        .count();
    assert!(dcs_count > 0 || actions.iter().any(|a| matches!(a, Action::Dcs { .. })));
}

// ============================================================================
// APC / PM / SOS sequences
// ============================================================================

#[test]
fn test_parser_apc_sequence() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b_data\x1b\\");
    let apc_count = actions
        .iter()
        .filter(|a| matches!(a, Action::Apc(_)))
        .count();
    assert!(apc_count > 0);
}

#[test]
fn test_parser_pm_sequence() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b^data\x1b\\");
    let pm_count = actions
        .iter()
        .filter(|a| matches!(a, Action::Pm(_)))
        .count();
    assert!(pm_count > 0);
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn test_parser_empty_input() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"");
    assert!(actions.is_empty());
}

#[test]
fn test_parser_only_esc() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b");
    assert!(actions.is_empty());
    assert_eq!(parser.state(), ParserState::Escape);
}

#[test]
fn test_parser_esc_followed_by_esc() {
    let mut parser = Parser::new();
    // Second ESC should cancel the first
    let actions = parser.parse_collect(b"\x1b\x1b7");
    // Should eventually produce SaveCursor
    let has_save = actions
        .iter()
        .any(|a| *a == Action::Esc(EscAction::SaveCursor));
    assert!(has_save);
}

#[test]
fn test_parser_csi_no_params() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b[m");
    assert_eq!(actions.len(), 1);
    if let Action::Csi(csi) = &actions[0] {
        assert_eq!(csi.final_byte, b'm');
    }
}

#[test]
fn test_parser_rapid_mode_changes() {
    let mut parser = Parser::new();
    // Rapidly enable and disable bracketed paste
    let actions = parser.parse_collect(b"\x1b[?2004h\x1b[?2004l");
    let csi_count = actions
        .iter()
        .filter(|a| matches!(a, Action::Csi(_)))
        .count();
    assert_eq!(csi_count, 2);
}

#[test]
fn test_parser_all_printable_ascii() {
    let mut parser = Parser::new();
    for byte in 0x20u8..=0x7E {
        let actions = parser.parse_collect(&[byte]);
        assert_eq!(
            actions.len(),
            1,
            "Byte 0x{:02X} should produce one action",
            byte
        );
        assert_eq!(actions[0], Action::Print(byte as char));
    }
}
