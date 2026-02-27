//! Comprehensive tests for CSI parameter parsing

use terminal_parser::Params;

// ============================================================================
// Params Creation
// ============================================================================

#[test]
fn test_params_new_empty() {
    let params = Params::new();
    assert!(params.is_empty());
    assert_eq!(params.len(), 0);
}

#[test]
fn test_params_default_empty() {
    let params = Params::default();
    assert!(params.is_empty());
}

#[test]
fn test_params_from_slice_empty() {
    let params = Params::from_slice(&[]);
    assert!(params.is_empty());
}

#[test]
fn test_params_from_slice_single() {
    let params = Params::from_slice(&[42]);
    assert_eq!(params.len(), 1);
    assert_eq!(params.get(0), Some(42));
}

#[test]
fn test_params_from_slice_multiple() {
    let params = Params::from_slice(&[1, 2, 3]);
    assert_eq!(params.len(), 3);
    assert_eq!(params.get(0), Some(1));
    assert_eq!(params.get(1), Some(2));
    assert_eq!(params.get(2), Some(3));
}

#[test]
fn test_params_from_slice_with_zero() {
    let params = Params::from_slice(&[0, 5, 0]);
    assert_eq!(params.len(), 3);
    assert_eq!(params.get(0), None); // 0 = default
    assert_eq!(params.get(1), Some(5));
    assert_eq!(params.get(2), None); // 0 = default
}

// ============================================================================
// Params::parse - Basic
// ============================================================================

#[test]
fn test_parse_empty() {
    let params = Params::parse(b"");
    assert!(params.is_empty());
}

#[test]
fn test_parse_single_digit() {
    let params = Params::parse(b"5");
    assert_eq!(params.len(), 1);
    assert_eq!(params.get(0), Some(5));
}

#[test]
fn test_parse_multi_digit() {
    let params = Params::parse(b"42");
    assert_eq!(params.get(0), Some(42));
}

#[test]
fn test_parse_three_digit() {
    let params = Params::parse(b"123");
    assert_eq!(params.get(0), Some(123));
}

#[test]
fn test_parse_four_digit() {
    let params = Params::parse(b"1234");
    assert_eq!(params.get(0), Some(1234));
}

#[test]
fn test_parse_five_digit() {
    let params = Params::parse(b"12345");
    assert_eq!(params.get(0), Some(12345));
}

#[test]
fn test_parse_max_u16() {
    let params = Params::parse(b"65535");
    assert_eq!(params.get(0), Some(65535));
}

#[test]
fn test_parse_overflow_saturates() {
    let params = Params::parse(b"99999");
    assert_eq!(params.get(0), Some(65535));
}

#[test]
fn test_parse_large_overflow() {
    let params = Params::parse(b"1000000");
    assert_eq!(params.get(0), Some(65535));
}

// ============================================================================
// Params::parse - Multiple Parameters
// ============================================================================

#[test]
fn test_parse_two_params() {
    let params = Params::parse(b"10;20");
    assert_eq!(params.len(), 2);
    assert_eq!(params.get(0), Some(10));
    assert_eq!(params.get(1), Some(20));
}

#[test]
fn test_parse_three_params() {
    let params = Params::parse(b"1;2;3");
    assert_eq!(params.len(), 3);
}

#[test]
fn test_parse_sgr_bold_red_green_bg() {
    let params = Params::parse(b"1;31;42");
    assert_eq!(params.len(), 3);
    assert_eq!(params.get(0), Some(1));
    assert_eq!(params.get(1), Some(31));
    assert_eq!(params.get(2), Some(42));
}

#[test]
fn test_parse_cursor_position() {
    let params = Params::parse(b"10;20");
    assert_eq!(params.get(0), Some(10));
    assert_eq!(params.get(1), Some(20));
}

#[test]
fn test_parse_many_params() {
    let params = Params::parse(b"1;2;3;4;5;6;7;8;9;10");
    assert_eq!(params.len(), 10);
    for i in 0..10 {
        assert_eq!(params.get(i), Some((i + 1) as u16));
    }
}

// ============================================================================
// Params::parse - Default/Missing Parameters
// ============================================================================

#[test]
fn test_parse_leading_semicolon() {
    let params = Params::parse(b";5");
    assert_eq!(params.len(), 2);
    assert_eq!(params.get(0), None); // Default
    assert_eq!(params.get(1), Some(5));
}

#[test]
fn test_parse_trailing_semicolon() {
    let params = Params::parse(b"5;");
    assert_eq!(params.len(), 2);
    assert_eq!(params.get(0), Some(5));
    assert_eq!(params.get(1), None); // Default
}

#[test]
fn test_parse_middle_default() {
    let params = Params::parse(b"1;;3");
    assert_eq!(params.len(), 3);
    assert_eq!(params.get(0), Some(1));
    assert_eq!(params.get(1), None); // Default
    assert_eq!(params.get(2), Some(3));
}

#[test]
fn test_parse_all_defaults() {
    let params = Params::parse(b";;");
    assert_eq!(params.len(), 3);
    assert_eq!(params.get(0), None);
    assert_eq!(params.get(1), None);
    assert_eq!(params.get(2), None);
}

#[test]
fn test_parse_single_semicolon() {
    let params = Params::parse(b";");
    assert_eq!(params.len(), 2);
    assert_eq!(params.get(0), None);
    assert_eq!(params.get(1), None);
}

// ============================================================================
// Params::get / get_or / raw
// ============================================================================

#[test]
fn test_get_valid_index() {
    let params = Params::from_slice(&[42]);
    assert_eq!(params.get(0), Some(42));
}

#[test]
fn test_get_out_of_bounds() {
    let params = Params::from_slice(&[42]);
    assert_eq!(params.get(1), None);
    assert_eq!(params.get(100), None);
}

#[test]
fn test_get_zero_is_none() {
    let params = Params::from_slice(&[0]);
    assert_eq!(params.get(0), None);
}

#[test]
fn test_get_or_present() {
    let params = Params::from_slice(&[42]);
    assert_eq!(params.get_or(0, 1), 42);
}

#[test]
fn test_get_or_missing() {
    let params = Params::from_slice(&[]);
    assert_eq!(params.get_or(0, 1), 1);
}

#[test]
fn test_get_or_zero_default() {
    let params = Params::from_slice(&[0]);
    assert_eq!(params.get_or(0, 99), 99);
}

#[test]
fn test_raw_present() {
    let params = Params::from_slice(&[42]);
    assert_eq!(params.raw(0), 42);
}

#[test]
fn test_raw_missing() {
    let params = Params::from_slice(&[]);
    assert_eq!(params.raw(0), 0);
}

#[test]
fn test_raw_zero() {
    let params = Params::from_slice(&[0]);
    assert_eq!(params.raw(0), 0);
}

// ============================================================================
// Params::len / is_empty
// ============================================================================

#[test]
fn test_len_empty() {
    let params = Params::new();
    assert_eq!(params.len(), 0);
}

#[test]
fn test_len_one() {
    let params = Params::from_slice(&[1]);
    assert_eq!(params.len(), 1);
}

#[test]
fn test_len_many() {
    let params = Params::from_slice(&[1, 2, 3, 4, 5]);
    assert_eq!(params.len(), 5);
}

#[test]
fn test_is_empty_true() {
    assert!(Params::new().is_empty());
}

#[test]
fn test_is_empty_false() {
    assert!(!Params::from_slice(&[1]).is_empty());
}

// ============================================================================
// Params::iter
// ============================================================================

#[test]
fn test_iter_empty() {
    let params = Params::new();
    let values: Vec<u16> = params.iter().collect();
    assert!(values.is_empty());
}

#[test]
fn test_iter_single() {
    let params = Params::from_slice(&[42]);
    let values: Vec<u16> = params.iter().collect();
    assert_eq!(values, vec![42]);
}

#[test]
fn test_iter_multiple() {
    let params = Params::from_slice(&[1, 2, 3]);
    let values: Vec<u16> = params.iter().collect();
    assert_eq!(values, vec![1, 2, 3]);
}

#[test]
fn test_iter_count() {
    let params = Params::from_slice(&[10, 20, 30, 40]);
    assert_eq!(params.iter().count(), 4);
}

// ============================================================================
// Params::subparams
// ============================================================================

#[test]
fn test_subparams_none() {
    let params = Params::from_slice(&[38]);
    // from_slice doesn't create subparams
    assert_eq!(params.subparams(0), None);
}

#[test]
fn test_subparams_colon_separator() {
    let params = Params::parse(b"38:2:255:128:64");
    let sub = params.subparams(0);
    assert!(sub.is_some());
}

#[test]
fn test_subparams_mixed() {
    // SGR with both semicolon and colon: 38:2:255:0:0;1
    let params = Params::parse(b"38:2:255:0:0;1");
    assert_eq!(params.len(), 2);
    assert!(params.subparams(0).is_some());
}

#[test]
fn test_subparams_out_of_bounds() {
    let params = Params::from_slice(&[1]);
    assert_eq!(params.subparams(5), None);
}

// ============================================================================
// Params::iter_with_subparams
// ============================================================================

#[test]
fn test_iter_with_subparams_empty() {
    let params = Params::new();
    assert_eq!(params.iter_with_subparams().count(), 0);
}

#[test]
fn test_iter_with_subparams_no_subparams() {
    let params = Params::from_slice(&[1, 2, 3]);
    let items: Vec<(u16, &[u16])> = params.iter_with_subparams().collect();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].0, 1);
    assert_eq!(items[1].0, 2);
    assert_eq!(items[2].0, 3);
}

// ============================================================================
// Params::parse - Subparameters with colons
// ============================================================================

#[test]
fn test_parse_sgr_256_color() {
    // ESC[38;5;196m - 256 color foreground (semicolons)
    let params = Params::parse(b"38;5;196");
    assert_eq!(params.len(), 3);
    assert_eq!(params.get(0), Some(38));
    assert_eq!(params.get(1), Some(5));
    assert_eq!(params.get(2), Some(196));
}

#[test]
fn test_parse_sgr_rgb_semicolons() {
    // ESC[38;2;255;128;64m - RGB foreground (semicolons)
    let params = Params::parse(b"38;2;255;128;64");
    assert_eq!(params.len(), 5);
    assert_eq!(params.get(0), Some(38));
    assert_eq!(params.get(1), Some(2));
    assert_eq!(params.get(2), Some(255));
    assert_eq!(params.get(3), Some(128));
    assert_eq!(params.get(4), Some(64));
}

#[test]
fn test_parse_sgr_bg_256() {
    let params = Params::parse(b"48;5;100");
    assert_eq!(params.get(0), Some(48));
    assert_eq!(params.get(1), Some(5));
    assert_eq!(params.get(2), Some(100));
}

// ============================================================================
// Params clone / equality
// ============================================================================

#[test]
fn test_params_clone() {
    let params = Params::from_slice(&[1, 2, 3]);
    let cloned = params.clone();
    assert_eq!(params, cloned);
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
fn test_params_inequality_length() {
    let p1 = Params::from_slice(&[1, 2]);
    let p2 = Params::from_slice(&[1, 2, 3]);
    assert_ne!(p1, p2);
}

// ============================================================================
// Params edge cases
// ============================================================================

#[test]
fn test_parse_zero() {
    let params = Params::parse(b"0");
    assert_eq!(params.len(), 1);
    assert_eq!(params.raw(0), 0);
    assert_eq!(params.get(0), None); // 0 = default/unspecified
}

#[test]
fn test_parse_non_digit_ignored() {
    let params = Params::parse(b"1x2");
    // x is ignored, so we get "12" as one value
    assert_eq!(params.len(), 1);
}

#[test]
fn test_parse_only_non_digits() {
    let params = Params::parse(b"abc");
    assert!(params.is_empty());
}

#[test]
fn test_parse_spaces_ignored() {
    let params = Params::parse(b"1 2");
    // spaces are ignored, parser sees "12"
    assert_eq!(params.len(), 1);
}

#[test]
fn test_get_or_with_default_1() {
    // Common pattern: cursor position defaults to 1
    let params = Params::parse(b"");
    assert_eq!(params.get_or(0, 1), 1);
    assert_eq!(params.get_or(1, 1), 1);
}

#[test]
fn test_raw_vs_get() {
    let params = Params::from_slice(&[0, 5, 0]);
    // raw returns 0 for unset params
    assert_eq!(params.raw(0), 0);
    assert_eq!(params.raw(1), 5);
    // get returns None for 0 (unset)
    assert_eq!(params.get(0), None);
    assert_eq!(params.get(1), Some(5));
}

// ============================================================================
// Common CSI parameter patterns
// ============================================================================

#[test]
fn test_csi_cup_default() {
    // CSI H - cursor position, defaults to 1;1
    let params = Params::parse(b"");
    assert_eq!(params.get_or(0, 1), 1);
    assert_eq!(params.get_or(1, 1), 1);
}

#[test]
fn test_csi_cup_explicit() {
    let params = Params::parse(b"10;20");
    assert_eq!(params.get_or(0, 1), 10);
    assert_eq!(params.get_or(1, 1), 20);
}

#[test]
fn test_csi_ed_0() {
    // CSI 0 J - erase below
    let params = Params::parse(b"0");
    assert_eq!(params.raw(0), 0);
}

#[test]
fn test_csi_ed_1() {
    // CSI 1 J - erase above
    let params = Params::parse(b"1");
    assert_eq!(params.get(0), Some(1));
}

#[test]
fn test_csi_ed_2() {
    // CSI 2 J - erase all
    let params = Params::parse(b"2");
    assert_eq!(params.get(0), Some(2));
}

#[test]
fn test_csi_el_0() {
    let params = Params::parse(b"0");
    assert_eq!(params.raw(0), 0);
}

#[test]
fn test_csi_el_1() {
    let params = Params::parse(b"1");
    assert_eq!(params.get(0), Some(1));
}

#[test]
fn test_csi_el_2() {
    let params = Params::parse(b"2");
    assert_eq!(params.get(0), Some(2));
}

#[test]
fn test_csi_sgr_reset() {
    let params = Params::parse(b"0");
    assert_eq!(params.raw(0), 0);
}

#[test]
fn test_csi_sgr_bold() {
    let params = Params::parse(b"1");
    assert_eq!(params.get(0), Some(1));
}

#[test]
fn test_csi_sgr_dim() {
    let params = Params::parse(b"2");
    assert_eq!(params.get(0), Some(2));
}

#[test]
fn test_csi_sgr_italic() {
    let params = Params::parse(b"3");
    assert_eq!(params.get(0), Some(3));
}

#[test]
fn test_csi_sgr_underline() {
    let params = Params::parse(b"4");
    assert_eq!(params.get(0), Some(4));
}

#[test]
fn test_csi_sgr_blink() {
    let params = Params::parse(b"5");
    assert_eq!(params.get(0), Some(5));
}

#[test]
fn test_csi_sgr_inverse() {
    let params = Params::parse(b"7");
    assert_eq!(params.get(0), Some(7));
}

#[test]
fn test_csi_sgr_invisible() {
    let params = Params::parse(b"8");
    assert_eq!(params.get(0), Some(8));
}

#[test]
fn test_csi_sgr_strikethrough() {
    let params = Params::parse(b"9");
    assert_eq!(params.get(0), Some(9));
}

#[test]
fn test_csi_dec_mode_cursor_keys() {
    let params = Params::parse(b"1");
    assert_eq!(params.get(0), Some(1));
}

#[test]
fn test_csi_dec_mode_132_col() {
    let params = Params::parse(b"3");
    assert_eq!(params.get(0), Some(3));
}

#[test]
fn test_csi_dec_mode_cursor_visible() {
    let params = Params::parse(b"25");
    assert_eq!(params.get(0), Some(25));
}

#[test]
fn test_csi_dec_mode_mouse_x10() {
    let params = Params::parse(b"9");
    assert_eq!(params.get(0), Some(9));
}

#[test]
fn test_csi_dec_mode_mouse_vt200() {
    let params = Params::parse(b"1000");
    assert_eq!(params.get(0), Some(1000));
}

#[test]
fn test_csi_dec_mode_mouse_button_event() {
    let params = Params::parse(b"1002");
    assert_eq!(params.get(0), Some(1002));
}

#[test]
fn test_csi_dec_mode_mouse_any_event() {
    let params = Params::parse(b"1003");
    assert_eq!(params.get(0), Some(1003));
}

#[test]
fn test_csi_dec_mode_sgr_mouse() {
    let params = Params::parse(b"1006");
    assert_eq!(params.get(0), Some(1006));
}

#[test]
fn test_csi_dec_mode_alt_screen() {
    let params = Params::parse(b"1049");
    assert_eq!(params.get(0), Some(1049));
}

#[test]
fn test_csi_dec_mode_bracketed_paste() {
    let params = Params::parse(b"2004");
    assert_eq!(params.get(0), Some(2004));
}

#[test]
fn test_csi_scroll_region() {
    // DECSTBM: top;bottom
    let params = Params::parse(b"5;20");
    assert_eq!(params.get(0), Some(5));
    assert_eq!(params.get(1), Some(20));
}

#[test]
fn test_parse_consecutive_semicolons_three() {
    let params = Params::parse(b";;;");
    assert_eq!(params.len(), 4);
}

#[test]
fn test_parse_value_1() {
    let params = Params::parse(b"1");
    assert_eq!(params.get(0), Some(1));
}
