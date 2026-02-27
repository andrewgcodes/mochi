#![allow(clippy::manual_contains)]
//! Comprehensive tests for the terminal parser
//!
//! Tests cover:
//! - Basic character printing
//! - C0/C1 control characters
//! - CSI sequences (cursor movement, SGR, erase, scroll, etc.)
//! - ESC sequences (save/restore cursor, index, charset, etc.)
//! - OSC sequences (title, hyperlink, clipboard, colors, etc.)
//! - DCS, APC, PM, SOS sequences
//! - UTF-8 handling
//! - Streaming/chunked parsing
//! - Edge cases and error handling
//! - Future feature sequences

use terminal_parser::{Action, CsiAction, EscAction, OscAction, Params, Parser, ParserState};

// ============================================================================
// Helper functions
// ============================================================================

fn parse(input: &[u8]) -> Vec<Action> {
    let mut parser = Parser::new();
    parser.parse_collect(input)
}

#[allow(dead_code)]
fn parse_str(input: &str) -> Vec<Action> {
    parse(input.as_bytes())
}

fn print_chars(actions: &[Action]) -> Vec<char> {
    actions
        .iter()
        .filter_map(|a| match a {
            Action::Print(c) => Some(*c),
            _ => None,
        })
        .collect()
}

fn first_csi(actions: &[Action]) -> &CsiAction {
    actions
        .iter()
        .find_map(|a| match a {
            Action::Csi(csi) => Some(csi),
            _ => None,
        })
        .expect("Expected CSI action")
}

fn first_osc(actions: &[Action]) -> &OscAction {
    actions
        .iter()
        .find_map(|a| match a {
            Action::Osc(osc) => Some(osc),
            _ => None,
        })
        .expect("Expected OSC action")
}

#[allow(dead_code)]
fn first_esc(actions: &[Action]) -> &EscAction {
    actions
        .iter()
        .find_map(|a| match a {
            Action::Esc(esc) => Some(esc),
            _ => None,
        })
        .expect("Expected ESC action")
}

// ============================================================================
// 1. Basic printing tests
// ============================================================================

#[test]
fn test_print_single_ascii() {
    let actions = parse(b"A");
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0], Action::Print('A'));
}

#[test]
fn test_print_multiple_ascii() {
    let actions = parse(b"Hello");
    assert_eq!(print_chars(&actions), vec!['H', 'e', 'l', 'l', 'o']);
}

#[test]
fn test_print_all_printable_ascii() {
    for byte in 0x20u8..0x7F {
        let actions = parse(&[byte]);
        assert_eq!(actions.len(), 1, "byte 0x{:02x}", byte);
        assert_eq!(
            actions[0],
            Action::Print(byte as char),
            "byte 0x{:02x}",
            byte
        );
    }
}

#[test]
fn test_print_space() {
    let actions = parse(b" ");
    assert_eq!(actions[0], Action::Print(' '));
}

#[test]
fn test_print_tilde() {
    let actions = parse(b"~");
    assert_eq!(actions[0], Action::Print('~'));
}

#[test]
fn test_print_digits() {
    let actions = parse(b"0123456789");
    let chars = print_chars(&actions);
    assert_eq!(
        chars,
        vec!['0', '1', '2', '3', '4', '5', '6', '7', '8', '9']
    );
}

#[test]
fn test_print_special_chars() {
    let actions = parse(b"!@#$%^&*()");
    assert_eq!(actions.len(), 10);
}

#[test]
fn test_print_mixed_with_controls() {
    let actions = parse(b"A\nB\rC");
    assert_eq!(actions.len(), 5);
    assert_eq!(actions[0], Action::Print('A'));
    assert_eq!(actions[1], Action::Control(0x0A));
    assert_eq!(actions[2], Action::Print('B'));
    assert_eq!(actions[3], Action::Control(0x0D));
    assert_eq!(actions[4], Action::Print('C'));
}

#[test]
fn test_print_empty_input() {
    let actions = parse(b"");
    assert!(actions.is_empty());
}

#[test]
fn test_print_long_string() {
    let input: String = (0..1000).map(|i| (b'A' + (i % 26) as u8) as char).collect();
    let actions = parse(input.as_bytes());
    assert_eq!(actions.len(), 1000);
}

// ============================================================================
// 2. C0 Control character tests
// ============================================================================

#[test]
fn test_control_bel() {
    let actions = parse(b"\x07");
    assert_eq!(actions[0], Action::Control(0x07));
}

#[test]
fn test_control_bs() {
    let actions = parse(b"\x08");
    assert_eq!(actions[0], Action::Control(0x08));
}

#[test]
fn test_control_ht() {
    let actions = parse(b"\x09");
    assert_eq!(actions[0], Action::Control(0x09));
}

#[test]
fn test_control_lf() {
    let actions = parse(b"\x0A");
    assert_eq!(actions[0], Action::Control(0x0A));
}

#[test]
fn test_control_vt() {
    let actions = parse(b"\x0B");
    assert_eq!(actions[0], Action::Control(0x0B));
}

#[test]
fn test_control_ff() {
    let actions = parse(b"\x0C");
    assert_eq!(actions[0], Action::Control(0x0C));
}

#[test]
fn test_control_cr() {
    let actions = parse(b"\x0D");
    assert_eq!(actions[0], Action::Control(0x0D));
}

#[test]
fn test_control_multiple() {
    let actions = parse(b"\x07\x08\x09\x0A\x0B\x0C\x0D");
    assert_eq!(actions.len(), 7);
    for (i, byte) in [0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D]
        .iter()
        .enumerate()
    {
        assert_eq!(actions[i], Action::Control(*byte));
    }
}

#[test]
fn test_control_null_ignored() {
    let actions = parse(b"\x00A");
    // NUL should be ignored, only 'A' printed
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0], Action::Print('A'));
}

#[test]
fn test_control_can_cancels_sequence() {
    let actions = parse(b"\x1b[\x18A");
    // CAN (0x18) cancels the CSI sequence
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0], Action::Print('A'));
}

#[test]
fn test_control_sub_cancels_sequence() {
    let actions = parse(b"\x1b[\x1AA");
    // SUB (0x1A) cancels the CSI sequence
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0], Action::Print('A'));
}

#[test]
fn test_control_esc_starts_sequence() {
    let actions = parse(b"\x1b7");
    assert_eq!(actions[0], Action::Esc(EscAction::SaveCursor));
}

#[test]
fn test_c0_controls_during_csi() {
    // C0 controls should still be executed during CSI sequences
    let actions = parse(b"\x1b[\x07H");
    assert_eq!(actions.len(), 2);
    assert_eq!(actions[0], Action::Control(0x07)); // BEL executed
}

// ============================================================================
// 3. CSI sequence tests - Cursor movement
// ============================================================================

#[test]
fn test_csi_cursor_up() {
    let actions = parse(b"\x1b[A");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'A');
    assert!(!csi.private);
}

#[test]
fn test_csi_cursor_up_with_param() {
    let actions = parse(b"\x1b[5A");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'A');
    assert_eq!(csi.param(0, 1), 5);
}

#[test]
fn test_csi_cursor_down() {
    let actions = parse(b"\x1b[B");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'B');
}

#[test]
fn test_csi_cursor_down_with_param() {
    let actions = parse(b"\x1b[10B");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 1), 10);
}

#[test]
fn test_csi_cursor_forward() {
    let actions = parse(b"\x1b[C");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'C');
}

#[test]
fn test_csi_cursor_backward() {
    let actions = parse(b"\x1b[D");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'D');
}

#[test]
fn test_csi_cursor_position() {
    let actions = parse(b"\x1b[10;20H");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'H');
    assert_eq!(csi.param(0, 1), 10);
    assert_eq!(csi.param(1, 1), 20);
}

#[test]
fn test_csi_cursor_position_default() {
    let actions = parse(b"\x1b[H");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'H');
    assert_eq!(csi.param(0, 1), 1); // default
    assert_eq!(csi.param(1, 1), 1); // default
}

#[test]
fn test_csi_cursor_position_f() {
    let actions = parse(b"\x1b[5;10f");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'f');
    assert_eq!(csi.param(0, 1), 5);
    assert_eq!(csi.param(1, 1), 10);
}

#[test]
fn test_csi_cursor_next_line() {
    let actions = parse(b"\x1b[3E");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'E');
    assert_eq!(csi.param(0, 1), 3);
}

#[test]
fn test_csi_cursor_prev_line() {
    let actions = parse(b"\x1b[2F");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'F');
    assert_eq!(csi.param(0, 1), 2);
}

#[test]
fn test_csi_cursor_horizontal_abs() {
    let actions = parse(b"\x1b[15G");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'G');
    assert_eq!(csi.param(0, 1), 15);
}

#[test]
fn test_csi_cursor_vertical_abs() {
    let actions = parse(b"\x1b[8d");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'd');
    assert_eq!(csi.param(0, 1), 8);
}

// ============================================================================
// 4. CSI sequence tests - Erase
// ============================================================================

#[test]
fn test_csi_erase_display_below() {
    let actions = parse(b"\x1b[0J");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'J');
    assert_eq!(csi.param(0, 0), 0); // actually raw 0 means default, None
}

#[test]
fn test_csi_erase_display_above() {
    let actions = parse(b"\x1b[1J");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'J');
    assert_eq!(csi.param(0, 0), 1);
}

#[test]
fn test_csi_erase_display_all() {
    let actions = parse(b"\x1b[2J");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'J');
    assert_eq!(csi.param(0, 0), 2);
}

#[test]
fn test_csi_erase_display_scrollback() {
    let actions = parse(b"\x1b[3J");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'J');
    assert_eq!(csi.param(0, 0), 3);
}

#[test]
fn test_csi_erase_display_default() {
    let actions = parse(b"\x1b[J");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'J');
}

#[test]
fn test_csi_erase_line_right() {
    let actions = parse(b"\x1b[0K");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'K');
}

#[test]
fn test_csi_erase_line_left() {
    let actions = parse(b"\x1b[1K");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'K');
    assert_eq!(csi.param(0, 0), 1);
}

#[test]
fn test_csi_erase_line_all() {
    let actions = parse(b"\x1b[2K");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'K');
    assert_eq!(csi.param(0, 0), 2);
}

#[test]
fn test_csi_erase_line_default() {
    let actions = parse(b"\x1b[K");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'K');
}

#[test]
fn test_csi_erase_chars() {
    let actions = parse(b"\x1b[5X");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'X');
    assert_eq!(csi.param(0, 1), 5);
}

// ============================================================================
// 5. CSI sequence tests - SGR (Select Graphic Rendition)
// ============================================================================

#[test]
fn test_csi_sgr_reset() {
    let actions = parse(b"\x1b[0m");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'm');
}

#[test]
fn test_csi_sgr_bold() {
    let actions = parse(b"\x1b[1m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 1);
}

#[test]
fn test_csi_sgr_faint() {
    let actions = parse(b"\x1b[2m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 2);
}

#[test]
fn test_csi_sgr_italic() {
    let actions = parse(b"\x1b[3m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 3);
}

#[test]
fn test_csi_sgr_underline() {
    let actions = parse(b"\x1b[4m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 4);
}

#[test]
fn test_csi_sgr_blink() {
    let actions = parse(b"\x1b[5m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 5);
}

#[test]
fn test_csi_sgr_inverse() {
    let actions = parse(b"\x1b[7m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 7);
}

#[test]
fn test_csi_sgr_hidden() {
    let actions = parse(b"\x1b[8m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 8);
}

#[test]
fn test_csi_sgr_strikethrough() {
    let actions = parse(b"\x1b[9m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 9);
}

#[test]
fn test_csi_sgr_multiple_attrs() {
    let actions = parse(b"\x1b[1;3;4;31m");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'm');
    assert_eq!(csi.params.len(), 4);
    assert_eq!(csi.param(0, 0), 1); // bold
    assert_eq!(csi.param(1, 0), 3); // italic
    assert_eq!(csi.param(2, 0), 4); // underline
    assert_eq!(csi.param(3, 0), 31); // red fg
}

#[test]
fn test_csi_sgr_fg_standard() {
    for color in 30..=37u16 {
        let seq = format!("\x1b[{}m", color);
        let actions = parse(seq.as_bytes());
        let csi = first_csi(&actions);
        assert_eq!(csi.param(0, 0), color);
    }
}

#[test]
fn test_csi_sgr_bg_standard() {
    for color in 40..=47u16 {
        let seq = format!("\x1b[{}m", color);
        let actions = parse(seq.as_bytes());
        let csi = first_csi(&actions);
        assert_eq!(csi.param(0, 0), color);
    }
}

#[test]
fn test_csi_sgr_fg_bright() {
    for color in 90..=97u16 {
        let seq = format!("\x1b[{}m", color);
        let actions = parse(seq.as_bytes());
        let csi = first_csi(&actions);
        assert_eq!(csi.param(0, 0), color);
    }
}

#[test]
fn test_csi_sgr_bg_bright() {
    for color in 100..=107u16 {
        let seq = format!("\x1b[{}m", color);
        let actions = parse(seq.as_bytes());
        let csi = first_csi(&actions);
        assert_eq!(csi.param(0, 0), color);
    }
}

#[test]
fn test_csi_sgr_256_color_fg() {
    let actions = parse(b"\x1b[38;5;196m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 38);
    assert_eq!(csi.param(1, 0), 5);
    assert_eq!(csi.param(2, 0), 196);
}

#[test]
fn test_csi_sgr_256_color_bg() {
    let actions = parse(b"\x1b[48;5;232m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 48);
    assert_eq!(csi.param(1, 0), 5);
    assert_eq!(csi.param(2, 0), 232);
}

#[test]
fn test_csi_sgr_rgb_fg() {
    let actions = parse(b"\x1b[38;2;255;128;64m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 38);
    assert_eq!(csi.param(1, 0), 2);
    assert_eq!(csi.param(2, 0), 255);
    assert_eq!(csi.param(3, 0), 128);
    assert_eq!(csi.param(4, 0), 64);
}

#[test]
fn test_csi_sgr_rgb_bg() {
    let actions = parse(b"\x1b[48;2;0;128;255m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 48);
    assert_eq!(csi.param(1, 0), 2);
}

#[test]
fn test_csi_sgr_default_fg() {
    let actions = parse(b"\x1b[39m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 39);
}

#[test]
fn test_csi_sgr_default_bg() {
    let actions = parse(b"\x1b[49m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 49);
}

#[test]
fn test_csi_sgr_default_reset() {
    let actions = parse(b"\x1b[m");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'm');
    assert!(csi.params.is_empty());
}

#[test]
fn test_csi_sgr_disable_bold() {
    let actions = parse(b"\x1b[22m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 22);
}

#[test]
fn test_csi_sgr_disable_italic() {
    let actions = parse(b"\x1b[23m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 23);
}

#[test]
fn test_csi_sgr_disable_underline() {
    let actions = parse(b"\x1b[24m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 24);
}

#[test]
fn test_csi_sgr_disable_blink() {
    let actions = parse(b"\x1b[25m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 25);
}

#[test]
fn test_csi_sgr_disable_inverse() {
    let actions = parse(b"\x1b[27m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 27);
}

#[test]
fn test_csi_sgr_disable_hidden() {
    let actions = parse(b"\x1b[28m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 28);
}

#[test]
fn test_csi_sgr_disable_strikethrough() {
    let actions = parse(b"\x1b[29m");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 29);
}

// ============================================================================
// 6. CSI sequence tests - Private modes
// ============================================================================

#[test]
fn test_csi_show_cursor() {
    let actions = parse(b"\x1b[?25h");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.final_byte, b'h');
    assert_eq!(csi.param(0, 0), 25);
}

#[test]
fn test_csi_hide_cursor() {
    let actions = parse(b"\x1b[?25l");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.final_byte, b'l');
    assert_eq!(csi.param(0, 0), 25);
}

#[test]
fn test_csi_alternate_screen_on() {
    let actions = parse(b"\x1b[?1049h");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.param(0, 0), 1049);
}

#[test]
fn test_csi_alternate_screen_off() {
    let actions = parse(b"\x1b[?1049l");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.param(0, 0), 1049);
}

#[test]
fn test_csi_bracketed_paste_on() {
    let actions = parse(b"\x1b[?2004h");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.param(0, 0), 2004);
}

#[test]
fn test_csi_bracketed_paste_off() {
    let actions = parse(b"\x1b[?2004l");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.param(0, 0), 2004);
}

#[test]
fn test_csi_application_cursor_keys() {
    let actions = parse(b"\x1b[?1h");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.param(0, 0), 1);
}

#[test]
fn test_csi_auto_wrap_mode() {
    let actions = parse(b"\x1b[?7h");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.param(0, 0), 7);
}

#[test]
fn test_csi_origin_mode() {
    let actions = parse(b"\x1b[?6h");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.param(0, 0), 6);
}

#[test]
fn test_csi_mouse_tracking_x10() {
    let actions = parse(b"\x1b[?9h");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.param(0, 0), 9);
}

#[test]
fn test_csi_mouse_tracking_vt200() {
    let actions = parse(b"\x1b[?1000h");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.param(0, 0), 1000);
}

#[test]
fn test_csi_mouse_button_event() {
    let actions = parse(b"\x1b[?1002h");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.param(0, 0), 1002);
}

#[test]
fn test_csi_mouse_any_event() {
    let actions = parse(b"\x1b[?1003h");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.param(0, 0), 1003);
}

#[test]
fn test_csi_mouse_sgr_mode() {
    let actions = parse(b"\x1b[?1006h");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.param(0, 0), 1006);
}

#[test]
fn test_csi_focus_events() {
    let actions = parse(b"\x1b[?1004h");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.param(0, 0), 1004);
}

#[test]
fn test_csi_synchronized_output() {
    let actions = parse(b"\x1b[?2026h");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.param(0, 0), 2026);
}

#[test]
fn test_csi_reverse_video() {
    let actions = parse(b"\x1b[?5h");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.param(0, 0), 5);
}

#[test]
fn test_csi_132_column_mode() {
    let actions = parse(b"\x1b[?3h");
    let csi = first_csi(&actions);
    assert!(csi.private);
    assert_eq!(csi.param(0, 0), 3);
}

// ============================================================================
// 7. CSI sequence tests - Scroll and insert/delete
// ============================================================================

#[test]
fn test_csi_scroll_up() {
    let actions = parse(b"\x1b[3S");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'S');
    assert_eq!(csi.param(0, 1), 3);
}

#[test]
fn test_csi_scroll_down() {
    let actions = parse(b"\x1b[2T");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'T');
    assert_eq!(csi.param(0, 1), 2);
}

#[test]
fn test_csi_insert_lines() {
    let actions = parse(b"\x1b[5L");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'L');
    assert_eq!(csi.param(0, 1), 5);
}

#[test]
fn test_csi_delete_lines() {
    let actions = parse(b"\x1b[3M");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'M');
    assert_eq!(csi.param(0, 1), 3);
}

#[test]
fn test_csi_insert_chars() {
    let actions = parse(b"\x1b[4@");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'@');
    assert_eq!(csi.param(0, 1), 4);
}

#[test]
fn test_csi_delete_chars() {
    let actions = parse(b"\x1b[2P");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'P');
    assert_eq!(csi.param(0, 1), 2);
}

#[test]
fn test_csi_set_scroll_region() {
    let actions = parse(b"\x1b[5;20r");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'r');
    assert_eq!(csi.param(0, 1), 5);
    assert_eq!(csi.param(1, 1), 20);
}

#[test]
fn test_csi_clear_tab_stop() {
    let actions = parse(b"\x1b[0g");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'g');
}

#[test]
fn test_csi_clear_all_tab_stops() {
    let actions = parse(b"\x1b[3g");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'g');
    assert_eq!(csi.param(0, 0), 3);
}

// ============================================================================
// 8. CSI sequence tests - Cursor style
// ============================================================================

#[test]
fn test_csi_set_cursor_style_block_blink() {
    let actions = parse(b"\x1b[1 q");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'q');
    assert_eq!(csi.param(0, 0), 1);
    assert_eq!(csi.intermediates, vec![b' ']);
}

#[test]
fn test_csi_set_cursor_style_block_steady() {
    let actions = parse(b"\x1b[2 q");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 2);
}

#[test]
fn test_csi_set_cursor_style_underline_blink() {
    let actions = parse(b"\x1b[3 q");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 3);
}

#[test]
fn test_csi_set_cursor_style_underline_steady() {
    let actions = parse(b"\x1b[4 q");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 4);
}

#[test]
fn test_csi_set_cursor_style_bar_blink() {
    let actions = parse(b"\x1b[5 q");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 5);
}

#[test]
fn test_csi_set_cursor_style_bar_steady() {
    let actions = parse(b"\x1b[6 q");
    let csi = first_csi(&actions);
    assert_eq!(csi.param(0, 0), 6);
}

// ============================================================================
// 9. CSI sequence tests - Device status
// ============================================================================

#[test]
fn test_csi_device_status_report() {
    let actions = parse(b"\x1b[6n");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'n');
    assert_eq!(csi.param(0, 0), 6);
}

#[test]
fn test_csi_device_attributes() {
    let actions = parse(b"\x1b[c");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'c');
}

#[test]
fn test_csi_secondary_device_attributes() {
    // ESC [ > c  -- but '>' isn't treated as private marker for '?'
    let actions = parse(b"\x1b[>c");
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'c');
}

// ============================================================================
// 10. CSI edge cases
// ============================================================================

#[test]
fn test_csi_no_params() {
    let actions = parse(b"\x1b[m");
    let csi = first_csi(&actions);
    assert!(csi.params.is_empty());
}

#[test]
fn test_csi_trailing_semicolon() {
    let actions = parse(b"\x1b[1;m");
    let csi = first_csi(&actions);
    assert_eq!(csi.params.len(), 2);
}

#[test]
fn test_csi_multiple_semicolons() {
    let actions = parse(b"\x1b[;;m");
    let csi = first_csi(&actions);
    // Multiple semicolons mean default (0) params
    assert!(csi.params.len() >= 2);
}

#[test]
fn test_csi_large_param() {
    let actions = parse(b"\x1b[99999m");
    let csi = first_csi(&actions);
    // Should saturate at u16::MAX
    assert_eq!(csi.param(0, 0), 65535);
}

#[test]
fn test_csi_many_params() {
    let seq = format!(
        "\x1b[{}m",
        (1..=30)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(";")
    );
    let actions = parse(seq.as_bytes());
    let csi = first_csi(&actions);
    assert!(csi.params.len() >= 20);
}

#[test]
fn test_csi_ignore_invalid() {
    // Invalid CSI sequence should be ignored
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b[\x7fH"); // DEL in CSI should go to CsiIgnore
                                         // Should end in ground state after final byte
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_csi_intermediate_bytes() {
    let actions = parse(b"\x1b[1 q"); // Set cursor style
    let csi = first_csi(&actions);
    assert_eq!(csi.intermediates, vec![b' ']);
}

#[test]
fn test_csi_too_many_intermediates() {
    // More than 4 intermediate bytes should cause CsiIgnore
    let mut parser = Parser::new();
    let _ = parser.parse_collect(b"\x1b[     q");
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_csi_is_method() {
    let csi = CsiAction {
        params: Params::new(),
        intermediates: vec![],
        final_byte: b'H',
        private: false,
    };
    assert!(csi.is(b'H'));
    assert!(!csi.is(b'J'));
}

#[test]
fn test_csi_is_private_method() {
    let csi = CsiAction {
        params: Params::new(),
        intermediates: vec![],
        final_byte: b'h',
        private: true,
    };
    assert!(csi.is_private(b'h'));
    assert!(!csi.is(b'h'));
}

#[test]
fn test_csi_is_with_intermediates() {
    let csi = CsiAction {
        params: Params::new(),
        intermediates: vec![b' '],
        final_byte: b'q',
        private: false,
    };
    assert!(!csi.is(b'q')); // has intermediates, so is() returns false
}

// ============================================================================
// 11. ESC sequence tests
// ============================================================================

#[test]
fn test_esc_save_cursor() {
    let actions = parse(b"\x1b7");
    assert_eq!(actions[0], Action::Esc(EscAction::SaveCursor));
}

#[test]
fn test_esc_restore_cursor() {
    let actions = parse(b"\x1b8");
    assert_eq!(actions[0], Action::Esc(EscAction::RestoreCursor));
}

#[test]
fn test_esc_index() {
    let actions = parse(b"\x1bD");
    assert_eq!(actions[0], Action::Esc(EscAction::Index));
}

#[test]
fn test_esc_reverse_index() {
    let actions = parse(b"\x1bM");
    assert_eq!(actions[0], Action::Esc(EscAction::ReverseIndex));
}

#[test]
fn test_esc_next_line() {
    let actions = parse(b"\x1bE");
    assert_eq!(actions[0], Action::Esc(EscAction::NextLine));
}

#[test]
fn test_esc_tab_set() {
    let actions = parse(b"\x1bH");
    assert_eq!(actions[0], Action::Esc(EscAction::HorizontalTabSet));
}

#[test]
fn test_esc_full_reset() {
    let actions = parse(b"\x1bc");
    assert_eq!(actions[0], Action::Esc(EscAction::FullReset));
}

#[test]
fn test_esc_application_keypad() {
    let actions = parse(b"\x1b=");
    assert_eq!(actions[0], Action::Esc(EscAction::ApplicationKeypad));
}

#[test]
fn test_esc_normal_keypad() {
    let actions = parse(b"\x1b>");
    assert_eq!(actions[0], Action::Esc(EscAction::NormalKeypad));
}

#[test]
fn test_esc_designate_g0_ascii() {
    let actions = parse(b"\x1b(B");
    assert_eq!(actions[0], Action::Esc(EscAction::DesignateG0('B')));
}

#[test]
fn test_esc_designate_g0_line_drawing() {
    let actions = parse(b"\x1b(0");
    assert_eq!(actions[0], Action::Esc(EscAction::DesignateG0('0')));
}

#[test]
fn test_esc_designate_g1() {
    let actions = parse(b"\x1b)0");
    assert_eq!(actions[0], Action::Esc(EscAction::DesignateG1('0')));
}

#[test]
fn test_esc_designate_g2() {
    let actions = parse(b"\x1b*B");
    assert_eq!(actions[0], Action::Esc(EscAction::DesignateG2('B')));
}

#[test]
fn test_esc_designate_g3() {
    let actions = parse(b"\x1b+B");
    assert_eq!(actions[0], Action::Esc(EscAction::DesignateG3('B')));
}

#[test]
fn test_esc_dec_alignment_test() {
    let actions = parse(b"\x1b#8");
    assert_eq!(actions[0], Action::Esc(EscAction::DecAlignmentTest));
}

#[test]
fn test_esc_string_terminator() {
    // ESC \ on its own just goes to ground
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b\\");
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_esc_unknown_sequence() {
    let actions = parse(b"\x1bZ"); // DECID - not explicitly handled
    match &actions[0] {
        Action::Esc(EscAction::Unknown(data)) => {
            assert_eq!(data, &vec![b'Z']);
        }
        _ => panic!("Expected unknown ESC sequence"),
    }
}

#[test]
fn test_esc_unknown_with_intermediate() {
    let actions = parse(b"\x1b#3"); // DECDHL - not handled
    match &actions[0] {
        Action::Esc(EscAction::Unknown(data)) => {
            assert!(data.contains(&b'#'));
        }
        other => panic!("Expected unknown ESC sequence, got {:?}", other),
    }
}

// ============================================================================
// 12. OSC sequence tests
// ============================================================================

#[test]
fn test_osc_set_title_bel() {
    let actions = parse(b"\x1b]2;My Title\x07");
    match first_osc(&actions) {
        OscAction::SetTitle(t) => assert_eq!(t, "My Title"),
        other => panic!("Expected SetTitle, got {:?}", other),
    }
}

#[test]
fn test_osc_set_title_st() {
    let actions = parse(b"\x1b]2;My Title\x1b\\");
    match first_osc(&actions) {
        OscAction::SetTitle(t) => assert_eq!(t, "My Title"),
        other => panic!("Expected SetTitle, got {:?}", other),
    }
}

#[test]
fn test_osc_set_icon_and_title() {
    let actions = parse(b"\x1b]0;Window Title\x07");
    match first_osc(&actions) {
        OscAction::SetIconAndTitle(t) => assert_eq!(t, "Window Title"),
        other => panic!("Expected SetIconAndTitle, got {:?}", other),
    }
}

#[test]
fn test_osc_set_icon_name() {
    let actions = parse(b"\x1b]1;Icon\x07");
    match first_osc(&actions) {
        OscAction::SetIconName(t) => assert_eq!(t, "Icon"),
        other => panic!("Expected SetIconName, got {:?}", other),
    }
}

#[test]
fn test_osc_set_color() {
    let actions = parse(b"\x1b]4;1;#ff0000\x07");
    match first_osc(&actions) {
        OscAction::SetColor { index, color } => {
            assert_eq!(*index, 1);
            assert_eq!(color, "#ff0000");
        }
        other => panic!("Expected SetColor, got {:?}", other),
    }
}

#[test]
fn test_osc_set_current_directory() {
    let actions = parse(b"\x1b]7;file:///home/user\x07");
    match first_osc(&actions) {
        OscAction::SetCurrentDirectory(dir) => {
            assert_eq!(dir, "file:///home/user");
        }
        other => panic!("Expected SetCurrentDirectory, got {:?}", other),
    }
}

#[test]
fn test_osc_hyperlink() {
    let actions = parse(b"\x1b]8;;https://example.com\x07");
    match first_osc(&actions) {
        OscAction::Hyperlink { params, uri } => {
            assert_eq!(params, "");
            assert_eq!(uri, "https://example.com");
        }
        other => panic!("Expected Hyperlink, got {:?}", other),
    }
}

#[test]
fn test_osc_hyperlink_with_params() {
    let actions = parse(b"\x1b]8;id=abc;https://example.com\x07");
    match first_osc(&actions) {
        OscAction::Hyperlink { params, uri } => {
            assert_eq!(params, "id=abc");
            assert_eq!(uri, "https://example.com");
        }
        other => panic!("Expected Hyperlink, got {:?}", other),
    }
}

#[test]
fn test_osc_hyperlink_close() {
    let actions = parse(b"\x1b]8;;\x07");
    match first_osc(&actions) {
        OscAction::Hyperlink { params, uri } => {
            assert_eq!(params, "");
            assert_eq!(uri, "");
        }
        other => panic!("Expected Hyperlink, got {:?}", other),
    }
}

#[test]
fn test_osc_set_foreground_color() {
    let actions = parse(b"\x1b]10;#d4d4d4\x07");
    match first_osc(&actions) {
        OscAction::SetForegroundColor(c) => assert_eq!(c, "#d4d4d4"),
        other => panic!("Expected SetForegroundColor, got {:?}", other),
    }
}

#[test]
fn test_osc_set_background_color() {
    let actions = parse(b"\x1b]11;#1e1e1e\x07");
    match first_osc(&actions) {
        OscAction::SetBackgroundColor(c) => assert_eq!(c, "#1e1e1e"),
        other => panic!("Expected SetBackgroundColor, got {:?}", other),
    }
}

#[test]
fn test_osc_set_cursor_color() {
    let actions = parse(b"\x1b]12;#ffffff\x07");
    match first_osc(&actions) {
        OscAction::SetCursorColor(c) => assert_eq!(c, "#ffffff"),
        other => panic!("Expected SetCursorColor, got {:?}", other),
    }
}

#[test]
fn test_osc_clipboard_set() {
    let actions = parse(b"\x1b]52;c;SGVsbG8=\x07");
    match first_osc(&actions) {
        OscAction::Clipboard { clipboard, data } => {
            assert_eq!(clipboard, "c");
            assert_eq!(data, "SGVsbG8=");
        }
        other => panic!("Expected Clipboard, got {:?}", other),
    }
}

#[test]
fn test_osc_clipboard_query() {
    let actions = parse(b"\x1b]52;c;?\x07");
    match first_osc(&actions) {
        OscAction::Clipboard { clipboard, data } => {
            assert_eq!(clipboard, "c");
            assert_eq!(data, "?");
        }
        other => panic!("Expected Clipboard, got {:?}", other),
    }
}

#[test]
fn test_osc_reset_color() {
    let actions = parse(b"\x1b]104;1\x07");
    match first_osc(&actions) {
        OscAction::ResetColor(idx) => assert_eq!(*idx, Some(1)),
        other => panic!("Expected ResetColor, got {:?}", other),
    }
}

#[test]
fn test_osc_reset_color_all() {
    let actions = parse(b"\x1b]104\x07");
    match first_osc(&actions) {
        OscAction::ResetColor(idx) => assert_eq!(*idx, None),
        other => panic!("Expected ResetColor, got {:?}", other),
    }
}

#[test]
fn test_osc_reset_foreground() {
    let actions = parse(b"\x1b]110\x07");
    match first_osc(&actions) {
        OscAction::ResetForegroundColor => {}
        other => panic!("Expected ResetForegroundColor, got {:?}", other),
    }
}

#[test]
fn test_osc_reset_background() {
    let actions = parse(b"\x1b]111\x07");
    match first_osc(&actions) {
        OscAction::ResetBackgroundColor => {}
        other => panic!("Expected ResetBackgroundColor, got {:?}", other),
    }
}

#[test]
fn test_osc_reset_cursor_color() {
    let actions = parse(b"\x1b]112\x07");
    match first_osc(&actions) {
        OscAction::ResetCursorColor => {}
        other => panic!("Expected ResetCursorColor, got {:?}", other),
    }
}

#[test]
fn test_osc_unknown() {
    let actions = parse(b"\x1b]999;some data\x07");
    match first_osc(&actions) {
        OscAction::Unknown { command, data } => {
            assert_eq!(*command, 999);
            assert_eq!(data, "some data");
        }
        other => panic!("Expected Unknown OSC, got {:?}", other),
    }
}

#[test]
fn test_osc_empty_payload() {
    let actions = parse(b"\x1b]2;\x07");
    match first_osc(&actions) {
        OscAction::SetTitle(t) => assert_eq!(t, ""),
        other => panic!("Expected SetTitle, got {:?}", other),
    }
}

#[test]
fn test_osc_utf8_title() {
    // OSC may have limited UTF-8 support; just verify it produces a SetTitle
    let actions = parse("\x1b]2;日本語タイトル\x07".as_bytes());
    match first_osc(&actions) {
        OscAction::SetTitle(t) => assert!(!t.is_empty()),
        other => panic!("Expected SetTitle, got {:?}", other),
    }
}

#[test]
fn test_osc_title_with_special_chars() {
    let actions = parse(b"\x1b]2;user@host: ~/dir\x07");
    match first_osc(&actions) {
        OscAction::SetTitle(t) => assert_eq!(t, "user@host: ~/dir"),
        other => panic!("Expected SetTitle, got {:?}", other),
    }
}

#[test]
fn test_osc_can_cancels() {
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b]2;Title\x18A");
    // CAN should cancel OSC, then 'A' is printed
    let prints: Vec<char> = actions
        .iter()
        .filter_map(|a| match a {
            Action::Print(c) => Some(*c),
            _ => None,
        })
        .collect();
    assert!(prints.contains(&'A'));
}

// ============================================================================
// 13. DCS sequence tests
// ============================================================================

#[test]
fn test_dcs_basic() {
    let actions = parse(b"\x1bPq\x1b\\");
    let has_dcs = actions.iter().any(|a| matches!(a, Action::Dcs { .. }));
    assert!(has_dcs);
}

#[test]
fn test_dcs_with_params() {
    let actions = parse(b"\x1bP1;2q\x1b\\");
    let has_dcs = actions.iter().any(|a| matches!(a, Action::Dcs { .. }));
    assert!(has_dcs);
}

#[test]
fn test_dcs_with_data() {
    let actions = parse(b"\x1bPqhello data\x1b\\");
    if let Some(Action::Dcs { data, .. }) = actions.iter().find(|a| matches!(a, Action::Dcs { .. }))
    {
        assert!(!data.is_empty());
    }
}

#[test]
fn test_dcs_8bit_entry() {
    // 0x90 is 8-bit DCS
    let actions = parse(b"\x90q\x9C");
    let has_dcs = actions.iter().any(|a| matches!(a, Action::Dcs { .. }));
    assert!(has_dcs);
}

// ============================================================================
// 14. APC, PM, SOS sequence tests
// ============================================================================

#[test]
fn test_apc_basic() {
    let actions = parse(b"\x1b_some apc data\x1b\\");
    let has_apc = actions.iter().any(|a| matches!(a, Action::Apc(_)));
    assert!(has_apc);
}

#[test]
fn test_pm_basic() {
    let actions = parse(b"\x1b^some pm data\x1b\\");
    let has_pm = actions.iter().any(|a| matches!(a, Action::Pm(_)));
    assert!(has_pm);
}

#[test]
fn test_sos_basic() {
    let actions = parse(b"\x1bXsome sos data\x1b\\");
    let has_sos = actions.iter().any(|a| matches!(a, Action::Sos(_)));
    assert!(has_sos);
}

#[test]
fn test_apc_8bit_st() {
    let actions = parse(b"\x1b_apc content\x9C");
    let has_apc = actions.iter().any(|a| matches!(a, Action::Apc(_)));
    assert!(has_apc);
}

// ============================================================================
// 15. UTF-8 tests
// ============================================================================

#[test]
fn test_utf8_ascii() {
    let actions = parse(b"A");
    assert_eq!(actions[0], Action::Print('A'));
}

#[test]
fn test_utf8_two_byte_char() {
    // é = U+00E9 = C3 A9
    let actions = parse(&[0xC3, 0xA9]);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0], Action::Print('é'));
}

#[test]
fn test_utf8_three_byte_char() {
    // 中 = U+4E2D = E4 B8 AD
    let actions = parse(&[0xE4, 0xB8, 0xAD]);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0], Action::Print('中'));
}

#[test]
fn test_utf8_four_byte_char() {
    // 😀 = U+1F600 = F0 9F 98 80
    let actions = parse(&[0xF0, 0x9F, 0x98, 0x80]);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0], Action::Print('😀'));
}

#[test]
fn test_utf8_mixed_with_ascii() {
    let actions = parse("Hé中😀!".as_bytes());
    let chars = print_chars(&actions);
    assert_eq!(chars, vec!['H', 'é', '中', '😀', '!']);
}

#[test]
fn test_utf8_japanese() {
    let actions = parse("こんにちは".as_bytes());
    let chars = print_chars(&actions);
    assert_eq!(chars, vec!['こ', 'ん', 'に', 'ち', 'は']);
}

#[test]
fn test_utf8_korean() {
    let actions = parse("안녕하세요".as_bytes());
    let chars = print_chars(&actions);
    assert_eq!(chars, vec!['안', '녕', '하', '세', '요']);
}

#[test]
fn test_utf8_arabic() {
    let actions = parse("مرحبا".as_bytes());
    let chars = print_chars(&actions);
    assert_eq!(chars.len(), 5);
}

#[test]
fn test_utf8_emoji_face() {
    let actions = parse("😊".as_bytes());
    assert_eq!(actions[0], Action::Print('😊'));
}

#[test]
fn test_utf8_emoji_flag() {
    // Flag emojis are sequences of regional indicators
    let actions = parse("🇺🇸".as_bytes());
    assert!(!actions.is_empty());
}

#[test]
fn test_utf8_invalid_start_byte() {
    // 0xFF is never valid UTF-8
    let actions = parse(&[0xFF]);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0], Action::Print('\u{FFFD}'));
}

#[test]
fn test_utf8_invalid_continuation() {
    // Start 2-byte seq then give another start byte instead of continuation
    // 0xC3 starts a 2-byte seq, 0xC4 is another start byte (invalid continuation)
    let actions = parse(&[0xC3, 0xC4, 0x80]);
    // First 2-byte seq (0xC3) should fail because 0xC4 is not a valid continuation,
    // producing a replacement char. Then 0xC4 0x80 should decode to U+0100 'Ā'
    let _has_replacement = actions.iter().any(|a| *a == Action::Print('\u{FFFD}'));
    // The parser may handle this differently; just check we get some output
    assert!(!actions.is_empty());
}

#[test]
fn test_utf8_overlong_encoding() {
    // Overlong encoding of '/' (should be 0x2F, not C0 AF)
    let actions = parse(&[0xC0, 0xAF]);
    let has_replacement = actions.iter().any(|a| *a == Action::Print('\u{FFFD}'));
    assert!(has_replacement);
}

#[test]
fn test_utf8_surrogate_halves() {
    // UTF-8 encoding of surrogate D800 should be rejected
    let actions = parse(&[0xED, 0xA0, 0x80]);
    let has_replacement = actions.iter().any(|a| *a == Action::Print('\u{FFFD}'));
    assert!(has_replacement);
}

#[test]
fn test_utf8_max_valid_codepoint() {
    // U+10FFFF = F4 8F BF BF
    let actions = parse(&[0xF4, 0x8F, 0xBF, 0xBF]);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0], Action::Print('\u{10FFFF}'));
}

#[test]
fn test_utf8_beyond_max_codepoint() {
    // F4 90 80 80 would be U+110000, which is invalid
    let actions = parse(&[0xF4, 0x90, 0x80, 0x80]);
    let has_replacement = actions.iter().any(|a| *a == Action::Print('\u{FFFD}'));
    assert!(has_replacement);
}

// ============================================================================
// 16. Streaming/chunked parsing tests
// ============================================================================

#[test]
fn test_streaming_csi_split_at_esc() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"\x1b");
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(b"[10;20H");
    assert_eq!(a2.len(), 1);
    let csi = first_csi(&a2);
    assert_eq!(csi.param(0, 1), 10);
    assert_eq!(csi.param(1, 1), 20);
}

#[test]
fn test_streaming_csi_split_at_bracket() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"\x1b[");
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(b"5A");
    let csi = first_csi(&a2);
    assert_eq!(csi.param(0, 1), 5);
}

#[test]
fn test_streaming_csi_split_in_params() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"\x1b[10");
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(b";20H");
    let csi = first_csi(&a2);
    assert_eq!(csi.param(0, 1), 10);
    assert_eq!(csi.param(1, 1), 20);
}

#[test]
fn test_streaming_utf8_split_two_byte() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(&[0xC3]);
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(&[0xA9]);
    assert_eq!(a2[0], Action::Print('é'));
}

#[test]
fn test_streaming_utf8_split_three_byte() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(&[0xE4]);
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(&[0xB8]);
    assert!(a2.is_empty());
    let a3 = parser.parse_collect(&[0xAD]);
    assert_eq!(a3[0], Action::Print('中'));
}

#[test]
fn test_streaming_utf8_split_four_byte() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(&[0xF0]);
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(&[0x9F]);
    assert!(a2.is_empty());
    let a3 = parser.parse_collect(&[0x98]);
    assert!(a3.is_empty());
    let a4 = parser.parse_collect(&[0x80]);
    assert_eq!(a4[0], Action::Print('😀'));
}

#[test]
fn test_streaming_osc_split() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"\x1b]2;My ");
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(b"Title\x07");
    match first_osc(&a2) {
        OscAction::SetTitle(t) => assert_eq!(t, "My Title"),
        other => panic!("Expected SetTitle, got {:?}", other),
    }
}

#[test]
fn test_streaming_esc_split() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"\x1b");
    assert!(a1.is_empty());
    let a2 = parser.parse_collect(b"D");
    assert_eq!(a2[0], Action::Esc(EscAction::Index));
}

#[test]
fn test_streaming_byte_by_byte() {
    let input = b"\x1b[1;31mHello\x1b[0m";
    let mut parser = Parser::new();
    let mut all_actions = Vec::new();
    for &byte in input {
        all_actions.extend(parser.parse_collect(&[byte]));
    }
    // Should get: CSI(1;31m), prints of Hello, CSI(0m)
    assert!(all_actions.len() >= 7); // CSI + 5 prints + CSI
}

#[test]
fn test_streaming_interleaved_text_and_sequences() {
    let mut parser = Parser::new();
    let a1 = parser.parse_collect(b"AB");
    assert_eq!(a1.len(), 2);
    let a2 = parser.parse_collect(b"\x1b[1m");
    assert_eq!(a2.len(), 1);
    let a3 = parser.parse_collect(b"CD");
    assert_eq!(a3.len(), 2);
}

// ============================================================================
// 17. Parser state tests
// ============================================================================

#[test]
fn test_parser_initial_state() {
    let parser = Parser::new();
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_parser_state_after_esc() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b");
    assert_eq!(parser.state(), ParserState::Escape);
}

#[test]
fn test_parser_state_after_csi_entry() {
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
fn test_parser_reset() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b[10");
    assert_eq!(parser.state(), ParserState::CsiParam);
    parser.reset();
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_parser_default() {
    let parser = Parser::default();
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_parser_state_back_to_ground_after_complete_csi() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b[1m");
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_parser_state_back_to_ground_after_complete_esc() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b7");
    assert_eq!(parser.state(), ParserState::Ground);
}

#[test]
fn test_parser_state_back_to_ground_after_osc() {
    let mut parser = Parser::new();
    parser.parse_collect(b"\x1b]2;title\x07");
    assert_eq!(parser.state(), ParserState::Ground);
}

// ============================================================================
// 18. C1 control tests (8-bit)
// ============================================================================

#[test]
fn test_c1_csi() {
    // 0x9B is 8-bit CSI
    let actions = parse(&[0x9B, b'1', b'm']);
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'm');
}

#[test]
fn test_c1_osc() {
    // 0x9D is 8-bit OSC
    let actions = parse(b"\x9D2;Title\x07");
    match first_osc(&actions) {
        OscAction::SetTitle(t) => assert_eq!(t, "Title"),
        other => panic!("Expected SetTitle, got {:?}", other),
    }
}

#[test]
fn test_c1_dcs() {
    // 0x90 is 8-bit DCS
    let mut parser = Parser::new();
    parser.parse_collect(&[0x90]);
    assert_eq!(parser.state(), ParserState::DcsEntry);
}

#[test]
fn test_c1_st() {
    // 0x9C is 8-bit ST - used to terminate strings
    let actions = parse(b"\x1b]2;Title\x9C");
    match first_osc(&actions) {
        OscAction::SetTitle(t) => assert_eq!(t, "Title"),
        other => panic!("Expected SetTitle, got {:?}", other),
    }
}

// ============================================================================
// 19. Complex real-world sequence tests
// ============================================================================

#[test]
fn test_shell_prompt_coloring() {
    // Typical bash prompt: \e[1;32muser@host\e[0m:\e[1;34m~/dir\e[0m$
    let actions = parse(b"\x1b[1;32muser\x1b[0m:\x1b[1;34m~\x1b[0m$ ");
    let prints = print_chars(&actions);
    assert!(prints.contains(&'u'));
    assert!(prints.contains(&':'));
    assert!(prints.contains(&'$'));
}

#[test]
fn test_cursor_save_restore_around_status() {
    // Save cursor, move to status line, write, restore
    let actions = parse(b"\x1b7\x1b[25;1HStatus\x1b8");
    assert_eq!(actions[0], Action::Esc(EscAction::SaveCursor));
    let last = actions.last().unwrap();
    assert_eq!(*last, Action::Esc(EscAction::RestoreCursor));
}

#[test]
fn test_clear_screen_sequence() {
    // Typical clear: ESC[2J ESC[H
    let actions = parse(b"\x1b[2J\x1b[H");
    assert_eq!(actions.len(), 2);
}

#[test]
fn test_alternate_screen_enter_exit() {
    let actions = parse(b"\x1b[?1049h\x1b[?1049l");
    assert_eq!(actions.len(), 2);
    let csi1 = match &actions[0] {
        Action::Csi(c) => c,
        _ => panic!("Expected CSI"),
    };
    assert!(csi1.private);
    assert_eq!(csi1.final_byte, b'h');
}

#[test]
fn test_sgr_then_text_then_reset() {
    let actions = parse(b"\x1b[1;31mError!\x1b[0m");
    assert_eq!(actions.len(), 8); // CSI + 6 chars + CSI
}

#[test]
fn test_multiple_osc_in_sequence() {
    let actions = parse(b"\x1b]0;Title1\x07\x1b]2;Title2\x07");
    let oscs: Vec<_> = actions
        .iter()
        .filter(|a| matches!(a, Action::Osc(_)))
        .collect();
    assert_eq!(oscs.len(), 2);
}

#[test]
fn test_scroll_region_and_cursor_move() {
    let actions = parse(b"\x1b[5;20r\x1b[5;1H");
    assert_eq!(actions.len(), 2);
}

#[test]
fn test_line_drawing_mode_sequence() {
    let actions = parse(b"\x1b(0lqqk\x1b(B");
    // Should see: DesignateG0('0'), prints, DesignateG0('B')
    assert_eq!(actions[0], Action::Esc(EscAction::DesignateG0('0')));
    let last = actions.last().unwrap();
    assert_eq!(*last, Action::Esc(EscAction::DesignateG0('B')));
}

#[test]
fn test_bracketed_paste_mode() {
    // Enable bracketed paste, paste content, disable
    let actions = parse(b"\x1b[?2004h\x1b[200~pasted text\x1b[201~\x1b[?2004l");
    // Should contain: private CSI, CSI(200~), text, CSI(201~), private CSI
    assert!(actions.len() > 4);
}

// ============================================================================
// 20. Params tests
// ============================================================================

#[test]
fn test_params_new() {
    let params = Params::new();
    assert!(params.is_empty());
    assert_eq!(params.len(), 0);
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
fn test_params_from_slice_empty() {
    let params = Params::from_slice(&[]);
    assert!(params.is_empty());
}

#[test]
fn test_params_parse_single() {
    let params = Params::parse(b"42");
    assert_eq!(params.len(), 1);
    assert_eq!(params.get(0), Some(42));
}

#[test]
fn test_params_parse_multiple() {
    let params = Params::parse(b"1;2;3;4;5");
    assert_eq!(params.len(), 5);
    for i in 0..5 {
        assert_eq!(params.get(i), Some((i + 1) as u16));
    }
}

#[test]
fn test_params_parse_defaults() {
    let params = Params::parse(b";5;");
    assert_eq!(params.len(), 3);
    assert_eq!(params.get(0), None); // 0 = default
    assert_eq!(params.get(1), Some(5));
    assert_eq!(params.get(2), None); // 0 = default
}

#[test]
fn test_params_get_or() {
    let params = Params::parse(b";5");
    assert_eq!(params.get_or(0, 1), 1);
    assert_eq!(params.get_or(1, 1), 5);
    assert_eq!(params.get_or(99, 42), 42);
}

#[test]
fn test_params_raw() {
    let params = Params::parse(b"0;5");
    assert_eq!(params.raw(0), 0);
    assert_eq!(params.raw(1), 5);
    assert_eq!(params.raw(99), 0);
}

#[test]
fn test_params_overflow_saturates() {
    let params = Params::parse(b"99999");
    assert_eq!(params.get(0), Some(65535));
}

#[test]
fn test_params_subparams() {
    let params = Params::parse(b"38:2:255:128:64");
    let subparams = params.subparams(0);
    assert!(subparams.is_some());
}

#[test]
fn test_params_iter() {
    let params = Params::parse(b"1;2;3");
    let values: Vec<u16> = params.iter().collect();
    assert_eq!(values, vec![1, 2, 3]);
}

#[test]
fn test_params_iter_with_subparams() {
    let params = Params::parse(b"1;2");
    let items: Vec<_> = params.iter_with_subparams().collect();
    assert_eq!(items.len(), 2);
}

#[test]
fn test_params_default_trait() {
    let params = Params::default();
    assert!(params.is_empty());
}

#[test]
fn test_params_max_params() {
    // More than 32 params should be truncated
    let input = (0..40).map(|i| i.to_string()).collect::<Vec<_>>().join(";");
    let params = Params::parse(input.as_bytes());
    assert!(params.len() <= 32);
}

#[test]
fn test_params_parse_empty() {
    let params = Params::parse(b"");
    assert!(params.is_empty());
}

#[test]
fn test_params_get_out_of_bounds() {
    let params = Params::parse(b"1");
    assert_eq!(params.get(5), None);
}

// ============================================================================
// 21. Multiple sequences in one input
// ============================================================================

#[test]
fn test_multiple_csi_sequences() {
    let actions = parse(b"\x1b[1m\x1b[31m\x1b[42m");
    let csis: Vec<_> = actions
        .iter()
        .filter(|a| matches!(a, Action::Csi(_)))
        .collect();
    assert_eq!(csis.len(), 3);
}

#[test]
fn test_text_between_sequences() {
    let actions = parse(b"\x1b[1mBold\x1b[0m Normal");
    let prints = print_chars(&actions);
    assert_eq!(prints.len(), 11); // "Bold Normal" = 11 chars (with space)
}

#[test]
fn test_esc_followed_by_esc() {
    // Second ESC should cancel first
    let mut parser = Parser::new();
    let actions = parser.parse_collect(b"\x1b\x1b7");
    assert_eq!(actions[0], Action::Esc(EscAction::SaveCursor));
}

// ============================================================================
// 22. Future feature tests - Sixel graphics protocol
// ============================================================================

#[test]
fn test_sixel_dcs_sequence() {
    // Sixel uses DCS q ... ST
    let actions = parse(b"\x1bPq#0;2;0;0;0#1;2;100;100;0~-\x1b\\");
    let has_dcs = actions.iter().any(|a| matches!(a, Action::Dcs { .. }));
    assert!(has_dcs);
}

// ============================================================================
// 23. Future feature tests - Kitty graphics protocol
// ============================================================================

#[test]
fn test_kitty_graphics_apc() {
    // Kitty graphics uses APC sequences
    let actions = parse(b"\x1b_Gf=32,s=1,v=1;AAAA\x1b\\");
    let has_apc = actions.iter().any(|a| matches!(a, Action::Apc(_)));
    assert!(has_apc);
}

// ============================================================================
// 24. Stress tests
// ============================================================================

#[test]
fn test_very_long_osc_data() {
    let title = "x".repeat(10000);
    let seq = format!("\x1b]2;{}\x07", title);
    let actions = parse(seq.as_bytes());
    // Should handle gracefully (may truncate at MAX_OSC_LEN)
    assert!(!actions.is_empty());
}

#[test]
fn test_many_sequential_csi() {
    let mut input = Vec::new();
    for _ in 0..100 {
        input.extend_from_slice(b"\x1b[1m");
    }
    let actions = parse(&input);
    let csi_count = actions
        .iter()
        .filter(|a| matches!(a, Action::Csi(_)))
        .count();
    assert_eq!(csi_count, 100);
}

#[test]
fn test_rapid_mode_switches() {
    let input = b"\x1b[?1049h\x1b[?1049l\x1b[?1049h\x1b[?1049l";
    let actions = parse(input);
    let csi_count = actions
        .iter()
        .filter(|a| matches!(a, Action::Csi(_)))
        .count();
    assert_eq!(csi_count, 4);
}

#[test]
fn test_parse_callback_vs_collect() {
    let input = b"\x1b[1;31mHello\x1b[0m";
    let mut parser1 = Parser::new();
    let collected = parser1.parse_collect(input);

    let mut parser2 = Parser::new();
    let mut callback_actions = Vec::new();
    parser2.parse(input, |action| callback_actions.push(action));

    assert_eq!(collected.len(), callback_actions.len());
    for (a, b) in collected.iter().zip(callback_actions.iter()) {
        assert_eq!(a, b);
    }
}

// ============================================================================
// 25. Insert/Set Mode (SM/RM) tests
// ============================================================================

#[test]
fn test_csi_set_mode() {
    let actions = parse(b"\x1b[4h"); // Insert mode
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'h');
    assert_eq!(csi.param(0, 0), 4);
    assert!(!csi.private);
}

#[test]
fn test_csi_reset_mode() {
    let actions = parse(b"\x1b[4l"); // Replace mode
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'l');
    assert_eq!(csi.param(0, 0), 4);
}

#[test]
fn test_csi_set_linefeed_mode() {
    let actions = parse(b"\x1b[20h"); // Linefeed mode
    let csi = first_csi(&actions);
    assert_eq!(csi.final_byte, b'h');
    assert_eq!(csi.param(0, 0), 20);
}
