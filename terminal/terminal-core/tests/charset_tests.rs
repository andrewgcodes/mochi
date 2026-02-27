//! Comprehensive tests for character set handling

use terminal_core::{parse_charset_designation, Charset, CharsetState};

// ============================================================================
// Charset enum
// ============================================================================

#[test]
fn test_charset_default_is_ascii() {
    assert_eq!(Charset::default(), Charset::Ascii);
}

#[test]
fn test_charset_variants_distinct() {
    assert_ne!(Charset::Ascii, Charset::DecSpecialGraphics);
    assert_ne!(Charset::Ascii, Charset::Uk);
    assert_ne!(Charset::DecSpecialGraphics, Charset::Uk);
}

// ============================================================================
// CharsetState Creation
// ============================================================================

#[test]
fn test_charset_state_new_all_ascii() {
    let state = CharsetState::new();
    assert_eq!(state.g0, Charset::Ascii);
    assert_eq!(state.g1, Charset::Ascii);
    assert_eq!(state.g2, Charset::Ascii);
    assert_eq!(state.g3, Charset::Ascii);
}

#[test]
fn test_charset_state_new_g0_active() {
    let state = CharsetState::new();
    assert_eq!(state.active, 0);
}

#[test]
fn test_charset_state_new_no_single_shift() {
    let state = CharsetState::new();
    assert_eq!(state.single_shift, None);
}

#[test]
fn test_charset_state_default_trait() {
    let state = CharsetState::default();
    assert_eq!(state.g0, Charset::Ascii);
    assert_eq!(state.active, 0);
}

// ============================================================================
// CharsetState::current
// ============================================================================

#[test]
fn test_charset_state_current_default() {
    let state = CharsetState::new();
    assert_eq!(state.current(), Charset::Ascii);
}

#[test]
fn test_charset_state_current_g0() {
    let mut state = CharsetState::new();
    state.g0 = Charset::DecSpecialGraphics;
    assert_eq!(state.current(), Charset::DecSpecialGraphics);
}

#[test]
fn test_charset_state_current_g1_active() {
    let mut state = CharsetState::new();
    state.g1 = Charset::Uk;
    state.active = 1;
    assert_eq!(state.current(), Charset::Uk);
}

#[test]
fn test_charset_state_current_single_shift_2() {
    let mut state = CharsetState::new();
    state.g2 = Charset::DecSpecialGraphics;
    state.single_shift = Some(2);
    assert_eq!(state.current(), Charset::DecSpecialGraphics);
}

#[test]
fn test_charset_state_current_single_shift_3() {
    let mut state = CharsetState::new();
    state.g3 = Charset::Uk;
    state.single_shift = Some(3);
    assert_eq!(state.current(), Charset::Uk);
}

#[test]
fn test_charset_state_single_shift_overrides_active() {
    let mut state = CharsetState::new();
    state.g0 = Charset::Ascii;
    state.g2 = Charset::DecSpecialGraphics;
    state.active = 0;
    state.single_shift = Some(2);
    assert_eq!(state.current(), Charset::DecSpecialGraphics);
}

// ============================================================================
// CharsetState::set_slot
// ============================================================================

#[test]
fn test_charset_state_set_slot_0() {
    let mut state = CharsetState::new();
    state.set_slot(0, Charset::DecSpecialGraphics);
    assert_eq!(state.g0, Charset::DecSpecialGraphics);
}

#[test]
fn test_charset_state_set_slot_1() {
    let mut state = CharsetState::new();
    state.set_slot(1, Charset::Uk);
    assert_eq!(state.g1, Charset::Uk);
}

#[test]
fn test_charset_state_set_slot_2() {
    let mut state = CharsetState::new();
    state.set_slot(2, Charset::DecSpecialGraphics);
    assert_eq!(state.g2, Charset::DecSpecialGraphics);
}

#[test]
fn test_charset_state_set_slot_3() {
    let mut state = CharsetState::new();
    state.set_slot(3, Charset::Uk);
    assert_eq!(state.g3, Charset::Uk);
}

#[test]
fn test_charset_state_set_slot_invalid() {
    let mut state = CharsetState::new();
    state.set_slot(4, Charset::DecSpecialGraphics); // Should be no-op
    assert_eq!(state.g0, Charset::Ascii);
}

// ============================================================================
// CharsetState::shift_in / shift_out
// ============================================================================

#[test]
fn test_charset_state_shift_out() {
    let mut state = CharsetState::new();
    state.shift_out();
    assert_eq!(state.active, 1);
}

#[test]
fn test_charset_state_shift_in() {
    let mut state = CharsetState::new();
    state.shift_out();
    state.shift_in();
    assert_eq!(state.active, 0);
}

#[test]
fn test_charset_state_shift_out_clears_single_shift() {
    let mut state = CharsetState::new();
    state.single_shift = Some(2);
    state.shift_out();
    assert_eq!(state.single_shift, None);
}

#[test]
fn test_charset_state_shift_in_clears_single_shift() {
    let mut state = CharsetState::new();
    state.single_shift = Some(3);
    state.shift_in();
    assert_eq!(state.single_shift, None);
}

#[test]
fn test_charset_shift_in_out_cycle() {
    let mut state = CharsetState::new();
    state.g1 = Charset::DecSpecialGraphics;

    assert_eq!(state.current(), Charset::Ascii);
    state.shift_out();
    assert_eq!(state.current(), Charset::DecSpecialGraphics);
    state.shift_in();
    assert_eq!(state.current(), Charset::Ascii);
}

// ============================================================================
// CharsetState::single_shift_2 / single_shift_3
// ============================================================================

#[test]
fn test_charset_state_single_shift_2() {
    let mut state = CharsetState::new();
    state.single_shift_2();
    assert_eq!(state.single_shift, Some(2));
}

#[test]
fn test_charset_state_single_shift_3() {
    let mut state = CharsetState::new();
    state.single_shift_3();
    assert_eq!(state.single_shift, Some(3));
}

#[test]
fn test_charset_state_clear_single_shift() {
    let mut state = CharsetState::new();
    state.single_shift_2();
    state.clear_single_shift();
    assert_eq!(state.single_shift, None);
}

// ============================================================================
// CharsetState::translate
// ============================================================================

#[test]
fn test_charset_translate_ascii_passthrough() {
    let state = CharsetState::new();
    assert_eq!(state.translate('A'), 'A');
    assert_eq!(state.translate('z'), 'z');
    assert_eq!(state.translate('0'), '0');
}

#[test]
fn test_charset_translate_dec_special_graphics() {
    let mut state = CharsetState::new();
    state.g0 = Charset::DecSpecialGraphics;
    assert_eq!(state.translate('q'), '─');
    assert_eq!(state.translate('x'), '│');
    assert_eq!(state.translate('l'), '┌');
    assert_eq!(state.translate('k'), '┐');
    assert_eq!(state.translate('m'), '└');
    assert_eq!(state.translate('j'), '┘');
}

#[test]
fn test_charset_translate_dec_special_tees() {
    let mut state = CharsetState::new();
    state.g0 = Charset::DecSpecialGraphics;
    assert_eq!(state.translate('t'), '├');
    assert_eq!(state.translate('u'), '┤');
    assert_eq!(state.translate('v'), '┴');
    assert_eq!(state.translate('w'), '┬');
    assert_eq!(state.translate('n'), '┼');
}

#[test]
fn test_charset_translate_dec_special_symbols() {
    let mut state = CharsetState::new();
    state.g0 = Charset::DecSpecialGraphics;
    assert_eq!(state.translate('`'), '◆');
    assert_eq!(state.translate('a'), '▒');
    assert_eq!(state.translate('f'), '°');
    assert_eq!(state.translate('g'), '±');
    assert_eq!(state.translate('y'), '≤');
    assert_eq!(state.translate('z'), '≥');
    assert_eq!(state.translate('{'), 'π');
    assert_eq!(state.translate('|'), '≠');
    assert_eq!(state.translate('}'), '£');
    assert_eq!(state.translate('~'), '·');
}

#[test]
fn test_charset_translate_dec_special_control_symbols() {
    let mut state = CharsetState::new();
    state.g0 = Charset::DecSpecialGraphics;
    assert_eq!(state.translate('b'), '␉');
    assert_eq!(state.translate('c'), '␌');
    assert_eq!(state.translate('d'), '␍');
    assert_eq!(state.translate('e'), '␊');
    assert_eq!(state.translate('h'), '␤');
    assert_eq!(state.translate('i'), '␋');
}

#[test]
fn test_charset_translate_dec_special_scan_lines() {
    let mut state = CharsetState::new();
    state.g0 = Charset::DecSpecialGraphics;
    assert_eq!(state.translate('o'), '⎺');
    assert_eq!(state.translate('p'), '⎻');
    assert_eq!(state.translate('r'), '⎼');
    assert_eq!(state.translate('s'), '⎽');
}

#[test]
fn test_charset_translate_dec_passthrough_non_special() {
    let mut state = CharsetState::new();
    state.g0 = Charset::DecSpecialGraphics;
    // Characters outside the special range pass through
    assert_eq!(state.translate('A'), 'A');
    assert_eq!(state.translate('Z'), 'Z');
    assert_eq!(state.translate('0'), '0');
}

#[test]
fn test_charset_translate_uk() {
    let mut state = CharsetState::new();
    state.g0 = Charset::Uk;
    assert_eq!(state.translate('#'), '£');
}

#[test]
fn test_charset_translate_uk_passthrough() {
    let mut state = CharsetState::new();
    state.g0 = Charset::Uk;
    assert_eq!(state.translate('A'), 'A');
    assert_eq!(state.translate('$'), '$');
}

// ============================================================================
// CharsetState::reset
// ============================================================================

#[test]
fn test_charset_state_reset() {
    let mut state = CharsetState::new();
    state.g0 = Charset::DecSpecialGraphics;
    state.g1 = Charset::Uk;
    state.active = 1;
    state.single_shift = Some(2);

    state.reset();

    assert_eq!(state.g0, Charset::Ascii);
    assert_eq!(state.g1, Charset::Ascii);
    assert_eq!(state.g2, Charset::Ascii);
    assert_eq!(state.g3, Charset::Ascii);
    assert_eq!(state.active, 0);
    assert_eq!(state.single_shift, None);
}

// ============================================================================
// parse_charset_designation
// ============================================================================

#[test]
fn test_parse_charset_b_ascii() {
    assert_eq!(parse_charset_designation('B'), Charset::Ascii);
}

#[test]
fn test_parse_charset_at_ascii() {
    assert_eq!(parse_charset_designation('@'), Charset::Ascii);
}

#[test]
fn test_parse_charset_0_dec() {
    assert_eq!(parse_charset_designation('0'), Charset::DecSpecialGraphics);
}

#[test]
fn test_parse_charset_2_dec() {
    assert_eq!(parse_charset_designation('2'), Charset::DecSpecialGraphics);
}

#[test]
fn test_parse_charset_a_uk() {
    assert_eq!(parse_charset_designation('A'), Charset::Uk);
}

#[test]
fn test_parse_charset_unknown_defaults_ascii() {
    assert_eq!(parse_charset_designation('X'), Charset::Ascii);
    assert_eq!(parse_charset_designation('Z'), Charset::Ascii);
    assert_eq!(parse_charset_designation('1'), Charset::Ascii);
}
