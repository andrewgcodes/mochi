//! Comprehensive tests for character set handling

use terminal_core::{parse_charset_designation, Charset, CharsetState};

// ============================================================
// CharsetState Creation Tests
// ============================================================

#[test]
fn test_charset_state_new_defaults() {
    let state = CharsetState::new();
    assert_eq!(state.g0, Charset::Ascii);
    assert_eq!(state.g1, Charset::Ascii);
    assert_eq!(state.g2, Charset::Ascii);
    assert_eq!(state.g3, Charset::Ascii);
    assert_eq!(state.active, 0);
    assert_eq!(state.single_shift, None);
}

#[test]
fn test_charset_state_default_equals_new() {
    let state1 = CharsetState::new();
    let state2 = CharsetState::default();
    assert_eq!(state1, state2);
}

#[test]
fn test_charset_state_current_is_ascii_by_default() {
    let state = CharsetState::new();
    assert_eq!(state.current(), Charset::Ascii);
}

// ============================================================
// CharsetState Slot Tests
// ============================================================

#[test]
fn test_charset_set_slot_g0() {
    let mut state = CharsetState::new();
    state.set_slot(0, Charset::DecSpecialGraphics);
    assert_eq!(state.g0, Charset::DecSpecialGraphics);
}

#[test]
fn test_charset_set_slot_g1() {
    let mut state = CharsetState::new();
    state.set_slot(1, Charset::DecSpecialGraphics);
    assert_eq!(state.g1, Charset::DecSpecialGraphics);
}

#[test]
fn test_charset_set_slot_g2() {
    let mut state = CharsetState::new();
    state.set_slot(2, Charset::Uk);
    assert_eq!(state.g2, Charset::Uk);
}

#[test]
fn test_charset_set_slot_g3() {
    let mut state = CharsetState::new();
    state.set_slot(3, Charset::Uk);
    assert_eq!(state.g3, Charset::Uk);
}

#[test]
fn test_charset_set_slot_invalid_does_nothing() {
    let mut state = CharsetState::new();
    state.set_slot(4, Charset::DecSpecialGraphics);
    // All slots remain ASCII
    assert_eq!(state.g0, Charset::Ascii);
    assert_eq!(state.g1, Charset::Ascii);
    assert_eq!(state.g2, Charset::Ascii);
    assert_eq!(state.g3, Charset::Ascii);
}

#[test]
fn test_charset_set_slot_255_does_nothing() {
    let mut state = CharsetState::new();
    state.set_slot(255, Charset::DecSpecialGraphics);
    assert_eq!(state.g0, Charset::Ascii);
}

// ============================================================
// Shift In / Shift Out Tests
// ============================================================

#[test]
fn test_charset_shift_out_selects_g1() {
    let mut state = CharsetState::new();
    state.g1 = Charset::DecSpecialGraphics;
    state.shift_out();
    assert_eq!(state.active, 1);
    assert_eq!(state.current(), Charset::DecSpecialGraphics);
}

#[test]
fn test_charset_shift_in_selects_g0() {
    let mut state = CharsetState::new();
    state.g1 = Charset::DecSpecialGraphics;
    state.shift_out();
    state.shift_in();
    assert_eq!(state.active, 0);
    assert_eq!(state.current(), Charset::Ascii);
}

#[test]
fn test_charset_shift_in_clears_single_shift() {
    let mut state = CharsetState::new();
    state.single_shift_2();
    state.shift_in();
    assert_eq!(state.single_shift, None);
}

#[test]
fn test_charset_shift_out_clears_single_shift() {
    let mut state = CharsetState::new();
    state.single_shift_3();
    state.shift_out();
    assert_eq!(state.single_shift, None);
}

// ============================================================
// Single Shift Tests
// ============================================================

#[test]
fn test_charset_single_shift_2() {
    let mut state = CharsetState::new();
    state.g2 = Charset::DecSpecialGraphics;
    state.single_shift_2();
    assert_eq!(state.single_shift, Some(2));
    assert_eq!(state.current(), Charset::DecSpecialGraphics);
}

#[test]
fn test_charset_single_shift_3() {
    let mut state = CharsetState::new();
    state.g3 = Charset::Uk;
    state.single_shift_3();
    assert_eq!(state.single_shift, Some(3));
    assert_eq!(state.current(), Charset::Uk);
}

#[test]
fn test_charset_clear_single_shift() {
    let mut state = CharsetState::new();
    state.single_shift_2();
    state.clear_single_shift();
    assert_eq!(state.single_shift, None);
}

#[test]
fn test_charset_single_shift_overrides_active() {
    let mut state = CharsetState::new();
    state.g0 = Charset::Ascii;
    state.g2 = Charset::DecSpecialGraphics;
    state.single_shift_2();
    // Current should use G2, not G0
    assert_eq!(state.current(), Charset::DecSpecialGraphics);
}

#[test]
fn test_charset_after_clear_single_shift_uses_active() {
    let mut state = CharsetState::new();
    state.g0 = Charset::Uk;
    state.g2 = Charset::DecSpecialGraphics;
    state.single_shift_2();
    state.clear_single_shift();
    assert_eq!(state.current(), Charset::Uk);
}

// ============================================================
// Translate Tests
// ============================================================

#[test]
fn test_charset_translate_ascii_passthrough() {
    let state = CharsetState::new();
    assert_eq!(state.translate('A'), 'A');
    assert_eq!(state.translate('z'), 'z');
    assert_eq!(state.translate('5'), '5');
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
fn test_charset_translate_dec_all_special_chars() {
    let mut state = CharsetState::new();
    state.g0 = Charset::DecSpecialGraphics;
    assert_eq!(state.translate('`'), '◆');
    assert_eq!(state.translate('a'), '▒');
    assert_eq!(state.translate('f'), '°');
    assert_eq!(state.translate('g'), '±');
    assert_eq!(state.translate('n'), '┼');
    assert_eq!(state.translate('t'), '├');
    assert_eq!(state.translate('u'), '┤');
    assert_eq!(state.translate('v'), '┴');
    assert_eq!(state.translate('w'), '┬');
    assert_eq!(state.translate('y'), '≤');
    assert_eq!(state.translate('z'), '≥');
    assert_eq!(state.translate('{'), 'π');
    assert_eq!(state.translate('|'), '≠');
    assert_eq!(state.translate('}'), '£');
    assert_eq!(state.translate('~'), '·');
}

#[test]
fn test_charset_translate_dec_non_special_passthrough() {
    let mut state = CharsetState::new();
    state.g0 = Charset::DecSpecialGraphics;
    // Characters outside the mapping range should pass through
    assert_eq!(state.translate('A'), 'A');
    assert_eq!(state.translate('0'), '0');
}

#[test]
fn test_charset_translate_uk() {
    let mut state = CharsetState::new();
    state.g0 = Charset::Uk;
    assert_eq!(state.translate('#'), '£');
    assert_eq!(state.translate('A'), 'A'); // Other chars pass through
}

#[test]
fn test_charset_translate_uk_non_hash_passthrough() {
    let mut state = CharsetState::new();
    state.g0 = Charset::Uk;
    for c in ['A', 'Z', '0', '9', ' ', '!', '@'] {
        assert_eq!(state.translate(c), c);
    }
}

// ============================================================
// Reset Tests
// ============================================================

#[test]
fn test_charset_reset() {
    let mut state = CharsetState::new();
    state.g0 = Charset::DecSpecialGraphics;
    state.g1 = Charset::Uk;
    state.active = 1;
    state.single_shift = Some(2);
    state.reset();
    assert_eq!(state, CharsetState::new());
}

// ============================================================
// parse_charset_designation Tests
// ============================================================

#[test]
fn test_parse_charset_ascii_b() {
    assert_eq!(parse_charset_designation('B'), Charset::Ascii);
}

#[test]
fn test_parse_charset_ascii_at() {
    assert_eq!(parse_charset_designation('@'), Charset::Ascii);
}

#[test]
fn test_parse_charset_dec_0() {
    assert_eq!(parse_charset_designation('0'), Charset::DecSpecialGraphics);
}

#[test]
fn test_parse_charset_dec_2() {
    assert_eq!(parse_charset_designation('2'), Charset::DecSpecialGraphics);
}

#[test]
fn test_parse_charset_uk() {
    assert_eq!(parse_charset_designation('A'), Charset::Uk);
}

#[test]
fn test_parse_charset_unknown_defaults_to_ascii() {
    assert_eq!(parse_charset_designation('X'), Charset::Ascii);
    assert_eq!(parse_charset_designation('Z'), Charset::Ascii);
    assert_eq!(parse_charset_designation('1'), Charset::Ascii);
}

// ============================================================
// Charset Enum Tests
// ============================================================

#[test]
fn test_charset_default_is_ascii() {
    assert_eq!(Charset::default(), Charset::Ascii);
}

#[test]
fn test_charset_equality() {
    assert_eq!(Charset::Ascii, Charset::Ascii);
    assert_eq!(Charset::DecSpecialGraphics, Charset::DecSpecialGraphics);
    assert_eq!(Charset::Uk, Charset::Uk);
}

#[test]
fn test_charset_inequality() {
    assert_ne!(Charset::Ascii, Charset::DecSpecialGraphics);
    assert_ne!(Charset::Ascii, Charset::Uk);
    assert_ne!(Charset::DecSpecialGraphics, Charset::Uk);
}

#[test]
fn test_charset_clone() {
    let cs = Charset::DecSpecialGraphics;
    let clone = cs;
    assert_eq!(cs, clone);
}

// ============================================================
// Complex Scenario Tests
// ============================================================

#[test]
fn test_charset_shift_out_translate_shift_in() {
    let mut state = CharsetState::new();
    state.g1 = Charset::DecSpecialGraphics;

    // Initially ASCII
    assert_eq!(state.translate('q'), 'q');

    // After shift out, use DEC Special Graphics
    state.shift_out();
    assert_eq!(state.translate('q'), '─');

    // After shift in, back to ASCII
    state.shift_in();
    assert_eq!(state.translate('q'), 'q');
}

#[test]
fn test_charset_single_shift_then_clear() {
    let mut state = CharsetState::new();
    state.g2 = Charset::DecSpecialGraphics;

    // Single shift to G2
    state.single_shift_2();
    assert_eq!(state.translate('q'), '─');

    // Clear single shift, back to G0
    state.clear_single_shift();
    assert_eq!(state.translate('q'), 'q');
}
