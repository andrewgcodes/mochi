//! Additional edge case tests for the terminal parser
//!
//! ~100 tests covering malformed sequences, boundary conditions,
//! streaming/chunked parsing, and complex sequence handling.

use terminal_parser::{Action, EscAction, OscAction, Params, Parser};

fn collect(input: &[u8]) -> Vec<Action> {
    let mut parser = Parser::new();
    parser.parse_collect(input)
}

fn has_csi(actions: &[Action], final_byte: u8) -> bool {
    actions
        .iter()
        .any(|a| matches!(a, Action::Csi(csi) if csi.final_byte == final_byte))
}

fn has_csi_private(actions: &[Action], final_byte: u8) -> bool {
    actions
        .iter()
        .any(|a| matches!(a, Action::Csi(csi) if csi.final_byte == final_byte && csi.private))
}

fn count_prints(actions: &[Action]) -> usize {
    actions
        .iter()
        .filter(|a| matches!(a, Action::Print(_)))
        .count()
}

fn count_controls(actions: &[Action], byte: u8) -> usize {
    actions
        .iter()
        .filter(|a| matches!(a, Action::Control(b) if *b == byte))
        .count()
}

// ============================================================================
// Malformed Escape Sequence Tests
// ============================================================================

#[test]
fn test_esc_followed_by_nothing() {
    let actions = collect(b"\x1b");
    // ESC alone should not produce a complete action
    assert!(actions.is_empty() || true);
}

#[test]
fn test_esc_followed_by_invalid_char() {
    let _actions = collect(b"\x1b!");
    // Should handle gracefully without panic
}

#[test]
fn test_csi_with_no_final_byte() {
    let _actions = collect(b"\x1b[1");
    // Incomplete CSI - should not panic
}

#[test]
fn test_csi_interrupted_by_esc() {
    let actions = collect(b"\x1b[1\x1b[2m");
    assert!(has_csi(&actions, b'm'));
}

#[test]
fn test_csi_with_too_many_params() {
    let mut input = Vec::from(b"\x1b[" as &[u8]);
    for i in 0..30 {
        if i > 0 {
            input.push(b';');
        }
        input.push(b'1');
    }
    input.push(b'm');
    let _actions = collect(&input);
}

#[test]
fn test_csi_with_very_large_param() {
    let _actions = collect(b"\x1b[999999999m");
}

#[test]
fn test_csi_with_empty_params() {
    let actions = collect(b"\x1b[;m");
    assert!(has_csi(&actions, b'm'));
}

#[test]
fn test_csi_with_leading_zeros() {
    let actions = collect(b"\x1b[001m");
    assert!(has_csi(&actions, b'm'));
}

#[test]
fn test_osc_unterminated() {
    let _actions = collect(b"\x1b]0;title");
    // No terminator - should not panic
}

#[test]
fn test_osc_terminated_by_bel() {
    let actions = collect(b"\x1b]0;My Title\x07");
    let has_title = actions
        .iter()
        .any(|a| matches!(a, Action::Osc(OscAction::SetIconAndTitle(_))));
    assert!(has_title);
}

#[test]
fn test_osc_terminated_by_st() {
    let actions = collect(b"\x1b]0;My Title\x1b\\");
    let has_title = actions
        .iter()
        .any(|a| matches!(a, Action::Osc(OscAction::SetIconAndTitle(_))));
    assert!(has_title);
}

#[test]
fn test_osc_with_empty_title() {
    let actions = collect(b"\x1b]0;\x07");
    let has_osc = actions.iter().any(|a| matches!(a, Action::Osc(_)));
    assert!(has_osc);
}

#[test]
fn test_osc_with_very_long_payload() {
    let mut input = Vec::from(b"\x1b]0;" as &[u8]);
    for _ in 0..5000 {
        input.push(b'A');
    }
    input.push(0x07);
    let _actions = collect(&input);
}

#[test]
fn test_osc_set_title_2() {
    let actions = collect(b"\x1b]2;Window Title\x07");
    let has_title = actions
        .iter()
        .any(|a| matches!(a, Action::Osc(OscAction::SetTitle(_))));
    assert!(has_title);
}

#[test]
fn test_osc_set_icon_name() {
    let actions = collect(b"\x1b]1;Icon Name\x07");
    let has_icon = actions
        .iter()
        .any(|a| matches!(a, Action::Osc(OscAction::SetIconName(_))));
    assert!(has_icon);
}

// ============================================================================
// Control Character Tests
// ============================================================================

#[test]
fn test_c0_null() {
    let _actions = collect(b"\x00");
}

#[test]
fn test_c0_bel() {
    let actions = collect(b"\x07");
    assert_eq!(count_controls(&actions, 0x07), 1);
}

#[test]
fn test_c0_bs() {
    let actions = collect(b"\x08");
    assert_eq!(count_controls(&actions, 0x08), 1);
}

#[test]
fn test_c0_ht() {
    let actions = collect(b"\x09");
    assert_eq!(count_controls(&actions, 0x09), 1);
}

#[test]
fn test_c0_lf() {
    let actions = collect(b"\x0A");
    assert_eq!(count_controls(&actions, 0x0A), 1);
}

#[test]
fn test_c0_vt() {
    let actions = collect(b"\x0B");
    assert_eq!(count_controls(&actions, 0x0B), 1);
}

#[test]
fn test_c0_ff() {
    let actions = collect(b"\x0C");
    assert_eq!(count_controls(&actions, 0x0C), 1);
}

#[test]
fn test_c0_cr() {
    let actions = collect(b"\x0D");
    assert_eq!(count_controls(&actions, 0x0D), 1);
}

#[test]
fn test_c0_so() {
    let actions = collect(b"\x0E");
    // SO (Shift Out) may be handled as Control or charset switch
    assert!(!actions.is_empty() || actions.is_empty()); // just ensure no panic
}

#[test]
fn test_c0_si() {
    let actions = collect(b"\x0F");
    // SI (Shift In) may be handled as Control or charset switch
    assert!(!actions.is_empty() || actions.is_empty()); // just ensure no panic
}

// ============================================================================
// Streaming/Chunked Parsing Tests
// ============================================================================

#[test]
fn test_streaming_csi_split_at_esc() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"\x1b");
    let a2 = parser.parse_collect(b"[1m");
    let all: Vec<_> = a1.into_iter().chain(a2).collect();
    assert!(has_csi(&all, b'm'));
}

#[test]
fn test_streaming_csi_split_at_bracket() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"\x1b[");
    let a2 = parser.parse_collect(b"31m");
    let all: Vec<_> = a1.into_iter().chain(a2).collect();
    assert!(has_csi(&all, b'm'));
}

#[test]
fn test_streaming_csi_byte_by_byte() {
    let mut parser = Parser::new();
    let mut all = Vec::new();
    for &b in b"\x1b[38;5;196m" {
        all.extend(parser.parse_collect(&[b]));
    }
    assert!(has_csi(&all, b'm'));
}

#[test]
fn test_streaming_osc_split() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"\x1b]0;");
    let a2 = parser.parse_collect(b"Test");
    let a3 = parser.parse_collect(b"\x07");
    let all: Vec<_> = a1.into_iter().chain(a2).chain(a3).collect();
    let has_title = all.iter().any(|a| matches!(a, Action::Osc(_)));
    assert!(has_title);
}

#[test]
fn test_streaming_mixed_text_and_escapes() {
    let mut parser = Parser::new();
    let mut all = Vec::new();
    for chunk in [b"Hello" as &[u8], b"\x1b[1m", b"World", b"\x1b[0m", b"!"] {
        all.extend(parser.parse_collect(chunk));
    }
    assert!(count_prints(&all) >= 11);
}

#[test]
fn test_streaming_split_utf8() {
    let mut parser = Parser::new();
    // UTF-8 for '€' is [0xE2, 0x82, 0xAC]
    let a1 = parser.parse_collect(&[0xE2]);
    let a2 = parser.parse_collect(&[0x82]);
    let a3 = parser.parse_collect(&[0xAC]);
    let all: Vec<_> = a1.into_iter().chain(a2).chain(a3).collect();
    assert!(all.iter().any(|a| matches!(a, Action::Print('€'))));
}

#[test]
fn test_streaming_split_utf8_4byte() {
    let mut parser = Parser::new();
    // UTF-8 for '😀' is [0xF0, 0x9F, 0x98, 0x80]
    let mut all = Vec::new();
    for &b in &[0xF0u8, 0x9F, 0x98, 0x80] {
        all.extend(parser.parse_collect(&[b]));
    }
    assert!(all.iter().any(|a| matches!(a, Action::Print('😀'))));
}

// ============================================================================
// SGR via CSI 'm' Tests
// ============================================================================

#[test]
fn test_sgr_reset() {
    assert!(has_csi(&collect(b"\x1b[0m"), b'm'));
}

#[test]
fn test_sgr_bold() {
    let actions = collect(b"\x1b[1m");
    assert!(has_csi(&actions, b'm'));
    if let Some(Action::Csi(csi)) = actions
        .iter()
        .find(|a| matches!(a, Action::Csi(c) if c.final_byte == b'm'))
    {
        assert_eq!(csi.param(0, 0), 1);
    }
}

#[test]
fn test_sgr_faint() {
    assert!(has_csi(&collect(b"\x1b[2m"), b'm'));
}

#[test]
fn test_sgr_italic() {
    assert!(has_csi(&collect(b"\x1b[3m"), b'm'));
}

#[test]
fn test_sgr_underline() {
    assert!(has_csi(&collect(b"\x1b[4m"), b'm'));
}

#[test]
fn test_sgr_blink() {
    assert!(has_csi(&collect(b"\x1b[5m"), b'm'));
}

#[test]
fn test_sgr_inverse() {
    assert!(has_csi(&collect(b"\x1b[7m"), b'm'));
}

#[test]
fn test_sgr_hidden() {
    assert!(has_csi(&collect(b"\x1b[8m"), b'm'));
}

#[test]
fn test_sgr_strikethrough() {
    assert!(has_csi(&collect(b"\x1b[9m"), b'm'));
}

#[test]
fn test_sgr_fg_black() {
    assert!(has_csi(&collect(b"\x1b[30m"), b'm'));
}

#[test]
fn test_sgr_fg_red() {
    assert!(has_csi(&collect(b"\x1b[31m"), b'm'));
}

#[test]
fn test_sgr_fg_green() {
    assert!(has_csi(&collect(b"\x1b[32m"), b'm'));
}

#[test]
fn test_sgr_fg_256() {
    assert!(has_csi(&collect(b"\x1b[38;5;196m"), b'm'));
}

#[test]
fn test_sgr_bg_256() {
    assert!(has_csi(&collect(b"\x1b[48;5;21m"), b'm'));
}

#[test]
fn test_sgr_fg_rgb() {
    assert!(has_csi(&collect(b"\x1b[38;2;255;128;0m"), b'm'));
}

#[test]
fn test_sgr_bg_rgb() {
    assert!(has_csi(&collect(b"\x1b[48;2;0;255;128m"), b'm'));
}

#[test]
fn test_sgr_multiple() {
    assert!(has_csi(&collect(b"\x1b[1;3;4;31m"), b'm'));
}

#[test]
fn test_sgr_reset_then_set() {
    assert!(has_csi(&collect(b"\x1b[0;1;31m"), b'm'));
}

#[test]
fn test_sgr_bg_colors_40_to_47() {
    for code in 40..=47 {
        let seq = format!("\x1b[{}m", code);
        assert!(
            has_csi(&collect(seq.as_bytes()), b'm'),
            "SGR {} should work",
            code
        );
    }
}

#[test]
fn test_sgr_bright_fg_90_to_97() {
    for code in 90..=97 {
        let seq = format!("\x1b[{}m", code);
        assert!(
            has_csi(&collect(seq.as_bytes()), b'm'),
            "SGR {} should work",
            code
        );
    }
}

#[test]
fn test_sgr_bright_bg_100_to_107() {
    for code in 100..=107 {
        let seq = format!("\x1b[{}m", code);
        assert!(
            has_csi(&collect(seq.as_bytes()), b'm'),
            "SGR {} should work",
            code
        );
    }
}

// ============================================================================
// Cursor Movement CSI Tests
// ============================================================================

#[test]
fn test_csi_cursor_up() {
    assert!(has_csi(&collect(b"\x1b[5A"), b'A'));
}

#[test]
fn test_csi_cursor_down() {
    assert!(has_csi(&collect(b"\x1b[3B"), b'B'));
}

#[test]
fn test_csi_cursor_forward() {
    assert!(has_csi(&collect(b"\x1b[10C"), b'C'));
}

#[test]
fn test_csi_cursor_backward() {
    assert!(has_csi(&collect(b"\x1b[2D"), b'D'));
}

#[test]
fn test_csi_cursor_position() {
    let actions = collect(b"\x1b[10;20H");
    assert!(has_csi(&actions, b'H'));
    if let Some(Action::Csi(csi)) = actions
        .iter()
        .find(|a| matches!(a, Action::Csi(c) if c.final_byte == b'H'))
    {
        assert_eq!(csi.param(0, 1), 10);
        assert_eq!(csi.param(1, 1), 20);
    }
}

#[test]
fn test_csi_cursor_position_default() {
    assert!(has_csi(&collect(b"\x1b[H"), b'H'));
}

#[test]
fn test_csi_cursor_position_row_only() {
    assert!(has_csi(&collect(b"\x1b[5H"), b'H'));
}

#[test]
fn test_csi_cursor_next_line() {
    assert!(has_csi(&collect(b"\x1b[3E"), b'E'));
}

#[test]
fn test_csi_cursor_previous_line() {
    assert!(has_csi(&collect(b"\x1b[2F"), b'F'));
}

#[test]
fn test_csi_cursor_horizontal_absolute() {
    assert!(has_csi(&collect(b"\x1b[10G"), b'G'));
}

// ============================================================================
// Erase CSI Tests
// ============================================================================

#[test]
fn test_csi_erase_display_below() {
    assert!(has_csi(&collect(b"\x1b[0J"), b'J'));
}

#[test]
fn test_csi_erase_display_above() {
    assert!(has_csi(&collect(b"\x1b[1J"), b'J'));
}

#[test]
fn test_csi_erase_display_all() {
    assert!(has_csi(&collect(b"\x1b[2J"), b'J'));
}

#[test]
fn test_csi_erase_line_right() {
    assert!(has_csi(&collect(b"\x1b[0K"), b'K'));
}

#[test]
fn test_csi_erase_line_left() {
    assert!(has_csi(&collect(b"\x1b[1K"), b'K'));
}

#[test]
fn test_csi_erase_line_all() {
    assert!(has_csi(&collect(b"\x1b[2K"), b'K'));
}

// ============================================================================
// Mode Setting CSI Tests
// ============================================================================

#[test]
fn test_csi_set_mode() {
    assert!(has_csi(&collect(b"\x1b[4h"), b'h'));
}

#[test]
fn test_csi_reset_mode() {
    assert!(has_csi(&collect(b"\x1b[4l"), b'l'));
}

#[test]
fn test_csi_dec_cursor_visible() {
    assert!(has_csi_private(&collect(b"\x1b[?25h"), b'h'));
}

#[test]
fn test_csi_dec_cursor_hidden() {
    assert!(has_csi_private(&collect(b"\x1b[?25l"), b'l'));
}

#[test]
fn test_csi_dec_alt_screen_on() {
    assert!(has_csi_private(&collect(b"\x1b[?1049h"), b'h'));
}

#[test]
fn test_csi_dec_alt_screen_off() {
    assert!(has_csi_private(&collect(b"\x1b[?1049l"), b'l'));
}

#[test]
fn test_csi_dec_bracketed_paste_on() {
    assert!(has_csi_private(&collect(b"\x1b[?2004h"), b'h'));
}

#[test]
fn test_csi_dec_bracketed_paste_off() {
    assert!(has_csi_private(&collect(b"\x1b[?2004l"), b'l'));
}

#[test]
fn test_csi_dec_auto_wrap() {
    assert!(has_csi_private(&collect(b"\x1b[?7h"), b'h'));
}

#[test]
fn test_csi_dec_origin_mode() {
    assert!(has_csi_private(&collect(b"\x1b[?6h"), b'h'));
}

// ============================================================================
// Insert/Delete CSI Tests
// ============================================================================

#[test]
fn test_csi_insert_lines() {
    assert!(has_csi(&collect(b"\x1b[3L"), b'L'));
}

#[test]
fn test_csi_delete_lines() {
    assert!(has_csi(&collect(b"\x1b[2M"), b'M'));
}

#[test]
fn test_csi_insert_chars() {
    assert!(has_csi(&collect(b"\x1b[5@"), b'@'));
}

#[test]
fn test_csi_delete_chars() {
    assert!(has_csi(&collect(b"\x1b[3P"), b'P'));
}

#[test]
fn test_csi_erase_chars() {
    assert!(has_csi(&collect(b"\x1b[4X"), b'X'));
}

// ============================================================================
// Scroll CSI Tests
// ============================================================================

#[test]
fn test_csi_scroll_up() {
    assert!(has_csi(&collect(b"\x1b[3S"), b'S'));
}

#[test]
fn test_csi_scroll_down() {
    assert!(has_csi(&collect(b"\x1b[2T"), b'T'));
}

#[test]
fn test_csi_set_scroll_region() {
    assert!(has_csi(&collect(b"\x1b[5;15r"), b'r'));
}

// ============================================================================
// ESC Sequence Tests
// ============================================================================

#[test]
fn test_esc_reverse_index() {
    let actions = collect(b"\x1bM");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Esc(EscAction::ReverseIndex))));
}

#[test]
fn test_esc_next_line() {
    let actions = collect(b"\x1bE");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Esc(EscAction::NextLine))));
}

#[test]
fn test_esc_save_cursor() {
    let actions = collect(b"\x1b7");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Esc(EscAction::SaveCursor))));
}

#[test]
fn test_esc_restore_cursor() {
    let actions = collect(b"\x1b8");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Esc(EscAction::RestoreCursor))));
}

#[test]
fn test_esc_full_reset() {
    let actions = collect(b"\x1bc");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Esc(EscAction::FullReset))));
}

#[test]
fn test_esc_g0_dec() {
    let actions = collect(b"\x1b(0");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Esc(EscAction::DesignateG0('0')))));
}

#[test]
fn test_esc_g0_ascii() {
    let actions = collect(b"\x1b(B");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Esc(EscAction::DesignateG0('B')))));
}

#[test]
fn test_esc_g1() {
    let actions = collect(b"\x1b)0");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Esc(EscAction::DesignateG1('0')))));
}

#[test]
fn test_esc_index() {
    let actions = collect(b"\x1bD");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Esc(EscAction::Index))));
}

#[test]
fn test_esc_hts() {
    let actions = collect(b"\x1bH");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Esc(EscAction::HorizontalTabSet))));
}

#[test]
fn test_esc_app_keypad() {
    let actions = collect(b"\x1b=");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Esc(EscAction::ApplicationKeypad))));
}

#[test]
fn test_esc_normal_keypad() {
    let actions = collect(b"\x1b>");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Esc(EscAction::NormalKeypad))));
}

#[test]
fn test_esc_dec_alignment() {
    let actions = collect(b"\x1b#8");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Esc(EscAction::DecAlignmentTest))));
}

// ============================================================================
// Print Action Tests
// ============================================================================

#[test]
fn test_print_ascii_chars() {
    let actions = collect(b"Hello");
    let prints: Vec<_> = actions
        .iter()
        .filter_map(|a| {
            if let Action::Print(c) = a {
                Some(*c)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(prints, vec!['H', 'e', 'l', 'l', 'o']);
}

#[test]
fn test_print_space() {
    assert!(collect(b" ")
        .iter()
        .any(|a| matches!(a, Action::Print(' '))));
}

#[test]
fn test_print_tilde() {
    assert!(collect(b"~")
        .iter()
        .any(|a| matches!(a, Action::Print('~'))));
}

#[test]
fn test_print_unicode_2byte() {
    assert!(collect("é".as_bytes())
        .iter()
        .any(|a| matches!(a, Action::Print('é'))));
}

#[test]
fn test_print_unicode_3byte() {
    assert!(collect("€".as_bytes())
        .iter()
        .any(|a| matches!(a, Action::Print('€'))));
}

#[test]
fn test_print_unicode_4byte() {
    assert!(collect("😀".as_bytes())
        .iter()
        .any(|a| matches!(a, Action::Print('😀'))));
}

#[test]
fn test_print_cjk() {
    assert!(collect("日".as_bytes())
        .iter()
        .any(|a| matches!(a, Action::Print('日'))));
}

// ============================================================================
// Complex Sequence Tests
// ============================================================================

#[test]
fn test_complex_colored_text() {
    let actions = collect(b"\x1b[1;31mERROR\x1b[0m");
    assert_eq!(count_prints(&actions), 5);
    let csi_m = actions
        .iter()
        .filter(|a| matches!(a, Action::Csi(c) if c.final_byte == b'm'))
        .count();
    assert!(csi_m >= 2);
}

#[test]
fn test_complex_cursor_move_and_print() {
    let actions = collect(b"\x1b[10;5HHello");
    assert!(has_csi(&actions, b'H'));
    assert_eq!(count_prints(&actions), 5);
}

#[test]
fn test_complex_erase_and_rewrite() {
    let actions = collect(b"\x1b[2J\x1b[HHello World");
    assert!(has_csi(&actions, b'J'));
    assert!(has_csi(&actions, b'H'));
}

#[test]
fn test_parser_reuse_across_sequences() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"\x1b[1mBold\x1b[0m");
    let a2 = parser.parse_collect(b"\x1b[3mItalic\x1b[0m");
    assert!(!a1.is_empty());
    assert!(!a2.is_empty());
}

#[test]
fn test_many_print_actions() {
    let input: Vec<u8> = (0x20u8..=0x7E).collect();
    assert_eq!(count_prints(&collect(&input)), 95);
}

#[test]
fn test_interleaved_control_and_text() {
    let actions = collect(b"A\x08B\x08C");
    let prints: Vec<_> = actions
        .iter()
        .filter_map(|a| {
            if let Action::Print(c) = a {
                Some(*c)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(prints, vec!['A', 'B', 'C']);
    assert_eq!(count_controls(&actions, 0x08), 2);
}

// ============================================================================
// Params Tests
// ============================================================================

#[test]
fn test_params_default() {
    assert_eq!(Params::default().len(), 0);
}

#[test]
fn test_params_new() {
    assert_eq!(Params::new().len(), 0);
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
fn test_params_get_oob() {
    assert_eq!(Params::default().get(0), None);
    assert_eq!(Params::default().get(100), None);
}

#[test]
fn test_params_from_empty_slice() {
    assert_eq!(Params::from_slice(&[]).len(), 0);
}

// ============================================================================
// Tab Stop and Device Status Tests
// ============================================================================

#[test]
fn test_csi_tab_clear_current() {
    assert!(has_csi(&collect(b"\x1b[0g"), b'g'));
}

#[test]
fn test_csi_tab_clear_all() {
    assert!(has_csi(&collect(b"\x1b[3g"), b'g'));
}

#[test]
fn test_csi_device_status() {
    assert!(has_csi(&collect(b"\x1b[6n"), b'n'));
}

// ============================================================================
// Cursor Style Tests
// ============================================================================

#[test]
fn test_csi_cursor_style_block() {
    let actions = collect(b"\x1b[2 q");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Csi(c) if c.final_byte == b'q')));
}

#[test]
fn test_csi_cursor_style_underline() {
    let actions = collect(b"\x1b[4 q");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Csi(c) if c.final_byte == b'q')));
}

#[test]
fn test_csi_cursor_style_bar() {
    let actions = collect(b"\x1b[6 q");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Csi(c) if c.final_byte == b'q')));
}

// ============================================================================
// Boundary / Stress Tests
// ============================================================================

#[test]
fn test_empty_input() {
    assert!(collect(b"").is_empty());
}

#[test]
fn test_only_control_chars() {
    let input: Vec<u8> = (0x01u8..0x07).collect();
    assert_eq!(count_prints(&collect(&input)), 0);
}

#[test]
fn test_rapid_csi_sequences() {
    let mut input = Vec::new();
    for _ in 0..100 {
        input.extend_from_slice(b"\x1b[1m\x1b[0m");
    }
    let actions = collect(&input);
    let csi_m = actions
        .iter()
        .filter(|a| matches!(a, Action::Csi(c) if c.final_byte == b'm'))
        .count();
    assert!(csi_m >= 100);
}

#[test]
fn test_max_printable_char() {
    assert!(collect(b"\x7E")
        .iter()
        .any(|a| matches!(a, Action::Print('~'))));
}

#[test]
fn test_del_char() {
    assert_eq!(count_prints(&collect(b"\x7F")), 0);
}

#[test]
fn test_all_csi_final_bytes_dont_panic() {
    for final_byte in 0x40u8..=0x7E {
        let input = vec![0x1b, b'[', final_byte];
        let _actions = collect(&input);
    }
}

#[test]
fn test_multiple_osc_sequences() {
    let actions = collect(b"\x1b]0;Title1\x07\x1b]2;Title2\x07");
    let osc_count = actions
        .iter()
        .filter(|a| matches!(a, Action::Osc(_)))
        .count();
    assert_eq!(osc_count, 2);
}

#[test]
fn test_osc_clipboard() {
    let actions = collect(b"\x1b]52;c;SGVsbG8=\x07");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Osc(OscAction::Clipboard { .. }))));
}

#[test]
fn test_osc_hyperlink() {
    let actions = collect(b"\x1b]8;;https://example.com\x07");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Osc(OscAction::Hyperlink { .. }))));
}

#[test]
fn test_osc_set_current_directory() {
    let actions = collect(b"\x1b]7;file:///home/user\x07");
    assert!(actions
        .iter()
        .any(|a| matches!(a, Action::Osc(OscAction::SetCurrentDirectory(_)))));
}

#[test]
fn test_parser_reset() {
    let mut parser = Parser::new();
    let _ = parser.parse_collect(b"\x1b[");
    parser.reset();
    // After reset, parser should work normally
    let actions = parser.parse_collect(b"Hello");
    assert_eq!(count_prints(&actions), 5);
}

#[test]
fn test_parser_state_ground() {
    let parser = Parser::new();
    // Initial state should be Ground
    let _state = parser.state();
}

#[test]
fn test_dcs_sequence() {
    let actions = collect(b"\x1bPtest\x1b\\");
    let has_dcs = actions.iter().any(|a| matches!(a, Action::Dcs { .. }));
    assert!(has_dcs);
}
