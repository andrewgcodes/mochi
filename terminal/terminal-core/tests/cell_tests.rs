//! Comprehensive tests for terminal cell representation

use terminal_core::{Cell, CellAttributes, Color};

// ============================================================
// Cell Creation Tests
// ============================================================

#[test]
fn test_cell_new_is_empty() {
    let cell = Cell::new();
    assert!(cell.is_empty());
}

#[test]
fn test_cell_new_width_is_one() {
    let cell = Cell::new();
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_new_display_char_is_space() {
    let cell = Cell::new();
    assert_eq!(cell.display_char(), ' ');
}

#[test]
fn test_cell_new_content_is_empty_string() {
    let cell = Cell::new();
    assert_eq!(cell.content(), "");
}

#[test]
fn test_cell_new_hyperlink_id_is_zero() {
    let cell = Cell::new();
    assert_eq!(cell.hyperlink_id, 0);
}

#[test]
fn test_cell_new_not_continuation() {
    let cell = Cell::new();
    assert!(!cell.is_continuation());
}

#[test]
fn test_cell_default_equals_new() {
    let cell1 = Cell::new();
    let cell2 = Cell::default();
    assert_eq!(cell1, cell2);
}

#[test]
fn test_cell_new_attrs_are_default() {
    let cell = Cell::new();
    assert_eq!(cell.attrs, CellAttributes::default());
}

// ============================================================
// Cell::with_char Tests
// ============================================================

#[test]
fn test_cell_with_ascii_char() {
    let cell = Cell::with_char('A');
    assert_eq!(cell.display_char(), 'A');
    assert_eq!(cell.width(), 1);
    assert!(!cell.is_empty());
}

#[test]
fn test_cell_with_space_char() {
    let cell = Cell::with_char(' ');
    assert_eq!(cell.display_char(), ' ');
    assert!(cell.is_empty());
}

#[test]
fn test_cell_with_digit() {
    let cell = Cell::with_char('9');
    assert_eq!(cell.display_char(), '9');
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_with_special_char() {
    let cell = Cell::with_char('@');
    assert_eq!(cell.display_char(), '@');
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_with_newline() {
    let cell = Cell::with_char('\n');
    assert_eq!(cell.display_char(), '\n');
}

#[test]
fn test_cell_with_tab() {
    let cell = Cell::with_char('\t');
    assert_eq!(cell.display_char(), '\t');
}

#[test]
fn test_cell_with_all_printable_ascii() {
    for c in 0x20u8..=0x7E {
        let cell = Cell::with_char(c as char);
        assert_eq!(cell.display_char(), c as char);
        assert_eq!(cell.width(), 1);
    }
}

#[test]
fn test_cell_with_lowercase_letters() {
    for c in b'a'..=b'z' {
        let cell = Cell::with_char(c as char);
        assert_eq!(cell.display_char(), c as char);
        assert!(!cell.is_empty());
    }
}

#[test]
fn test_cell_with_uppercase_letters() {
    for c in b'A'..=b'Z' {
        let cell = Cell::with_char(c as char);
        assert_eq!(cell.display_char(), c as char);
        assert!(!cell.is_empty());
    }
}

#[test]
fn test_cell_with_digits_all() {
    for c in b'0'..=b'9' {
        let cell = Cell::with_char(c as char);
        assert_eq!(cell.display_char(), c as char);
    }
}

#[test]
fn test_cell_with_punctuation() {
    for c in [
        '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/',
    ] {
        let cell = Cell::with_char(c);
        assert_eq!(cell.display_char(), c);
        assert_eq!(cell.width(), 1);
    }
}

// ============================================================
// Wide Character Tests (CJK, etc.)
// ============================================================

#[test]
fn test_cell_cjk_chinese() {
    let cell = Cell::with_char('中');
    assert_eq!(cell.width(), 2);
    assert_eq!(cell.display_char(), '中');
}

#[test]
fn test_cell_cjk_japanese() {
    let cell = Cell::with_char('日');
    assert_eq!(cell.width(), 2);
}

#[test]
fn test_cell_cjk_korean() {
    let cell = Cell::with_char('한');
    assert_eq!(cell.width(), 2);
}

#[test]
fn test_cell_fullwidth_letter() {
    let cell = Cell::with_char('Ａ');
    assert_eq!(cell.width(), 2);
}

#[test]
fn test_cell_emoji() {
    let cell = Cell::with_char('😀');
    assert_eq!(cell.width(), 2);
}

#[test]
fn test_cell_latin_accented() {
    let cell = Cell::with_char('é');
    assert_eq!(cell.width(), 1);
    assert_eq!(cell.display_char(), 'é');
}

#[test]
fn test_cell_greek_letter() {
    let cell = Cell::with_char('α');
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_cyrillic_letter() {
    let cell = Cell::with_char('Д');
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_arabic_letter() {
    let cell = Cell::with_char('ع');
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_cjk_various() {
    for c in ['龍', '本', '語', '字', '國'] {
        let cell = Cell::with_char(c);
        assert_eq!(cell.width(), 2, "Expected width 2 for '{}'", c);
    }
}

#[test]
fn test_cell_cjk_katakana_fullwidth() {
    let cell = Cell::with_char('カ');
    assert_eq!(cell.width(), 2);
}

// ============================================================
// Cell::with_char_and_attrs Tests
// ============================================================

#[test]
fn test_cell_with_char_and_attrs_bold() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    let cell = Cell::with_char_and_attrs('X', attrs);
    assert_eq!(cell.display_char(), 'X');
    assert!(cell.attrs.bold);
}

#[test]
fn test_cell_with_char_and_attrs_colored() {
    let mut attrs = CellAttributes::new();
    attrs.fg = Color::Indexed(1);
    attrs.bg = Color::Indexed(4);
    let cell = Cell::with_char_and_attrs('Y', attrs);
    assert_eq!(cell.attrs.fg, Color::Indexed(1));
    assert_eq!(cell.attrs.bg, Color::Indexed(4));
}

#[test]
fn test_cell_with_char_and_attrs_all_flags() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    attrs.italic = true;
    attrs.underline = true;
    attrs.blink = true;
    attrs.inverse = true;
    attrs.hidden = true;
    attrs.strikethrough = true;
    let cell = Cell::with_char_and_attrs('Z', attrs);
    assert!(cell.attrs.bold);
    assert!(cell.attrs.italic);
    assert!(cell.attrs.underline);
    assert!(cell.attrs.blink);
    assert!(cell.attrs.inverse);
    assert!(cell.attrs.hidden);
    assert!(cell.attrs.strikethrough);
}

#[test]
fn test_cell_with_char_and_attrs_faint() {
    let mut attrs = CellAttributes::new();
    attrs.faint = true;
    let cell = Cell::with_char_and_attrs('F', attrs);
    assert!(cell.attrs.faint);
}

#[test]
fn test_cell_with_char_and_attrs_rgb_colors() {
    let mut attrs = CellAttributes::new();
    attrs.fg = Color::rgb(255, 0, 0);
    attrs.bg = Color::rgb(0, 255, 0);
    let cell = Cell::with_char_and_attrs('R', attrs);
    assert_eq!(cell.attrs.fg, Color::rgb(255, 0, 0));
    assert_eq!(cell.attrs.bg, Color::rgb(0, 255, 0));
}

#[test]
fn test_cell_with_char_and_attrs_hyperlink_still_zero() {
    let attrs = CellAttributes::new();
    let cell = Cell::with_char_and_attrs('H', attrs);
    assert_eq!(cell.hyperlink_id, 0);
}

// ============================================================
// Cell set_char / set_content Tests
// ============================================================

#[test]
fn test_cell_set_char() {
    let mut cell = Cell::new();
    cell.set_char('B');
    assert_eq!(cell.display_char(), 'B');
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_set_char_overwrites() {
    let mut cell = Cell::with_char('A');
    cell.set_char('B');
    assert_eq!(cell.display_char(), 'B');
}

#[test]
fn test_cell_set_char_wide() {
    let mut cell = Cell::new();
    cell.set_char('中');
    assert_eq!(cell.width(), 2);
}

#[test]
fn test_cell_set_char_narrow_after_wide() {
    let mut cell = Cell::new();
    cell.set_char('中');
    assert_eq!(cell.width(), 2);
    cell.set_char('A');
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_set_content_string() {
    let mut cell = Cell::new();
    cell.set_content("Hello");
    assert_eq!(cell.content(), "Hello");
}

#[test]
fn test_cell_set_content_empty() {
    let mut cell = Cell::with_char('A');
    cell.set_content("");
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_set_content_grapheme() {
    let mut cell = Cell::new();
    cell.set_content("é");
    assert_eq!(cell.content(), "é");
}

#[test]
fn test_cell_set_content_preserves_attrs() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    let mut cell = Cell::with_char_and_attrs('A', attrs);
    cell.set_content("B");
    assert!(cell.attrs.bold);
}

// ============================================================
// Cell continuation Tests
// ============================================================

#[test]
fn test_cell_set_continuation() {
    let mut cell = Cell::with_char('A');
    cell.set_continuation();
    assert!(cell.is_continuation());
    assert_eq!(cell.width(), 0);
    assert_eq!(cell.content(), "");
}

#[test]
fn test_cell_continuation_display_char() {
    let mut cell = Cell::new();
    cell.set_continuation();
    assert_eq!(cell.display_char(), ' ');
}

#[test]
fn test_cell_continuation_is_empty_content() {
    let mut cell = Cell::with_char('X');
    cell.set_continuation();
    assert_eq!(cell.content(), "");
}

// ============================================================
// Cell clear/reset Tests
// ============================================================

#[test]
fn test_cell_clear_resets_content() {
    let mut cell = Cell::with_char('X');
    cell.clear(CellAttributes::default());
    assert!(cell.is_empty());
}

#[test]
fn test_cell_clear_resets_width() {
    let mut cell = Cell::with_char('中');
    cell.clear(CellAttributes::default());
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_clear_sets_attrs() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    let mut cell = Cell::with_char('X');
    cell.clear(attrs);
    assert!(cell.attrs.bold);
}

#[test]
fn test_cell_clear_resets_hyperlink() {
    let mut cell = Cell::with_char('X');
    cell.hyperlink_id = 42;
    cell.clear(CellAttributes::default());
    assert_eq!(cell.hyperlink_id, 0);
}

#[test]
fn test_cell_reset_all() {
    let mut cell = Cell::with_char('X');
    cell.attrs.bold = true;
    cell.hyperlink_id = 42;
    cell.reset();
    assert!(cell.is_empty());
    assert!(!cell.attrs.bold);
    assert_eq!(cell.hyperlink_id, 0);
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_reset_after_continuation() {
    let mut cell = Cell::new();
    cell.set_continuation();
    cell.reset();
    assert_eq!(cell.width(), 1);
    assert!(!cell.is_continuation());
}

#[test]
fn test_cell_clear_after_continuation() {
    let mut cell = Cell::new();
    cell.set_continuation();
    cell.clear(CellAttributes::default());
    assert_eq!(cell.width(), 1);
    assert!(!cell.is_continuation());
}

// ============================================================
// CellAttributes Tests
// ============================================================

#[test]
fn test_cell_attrs_new_defaults() {
    let attrs = CellAttributes::new();
    assert!(!attrs.bold);
    assert!(!attrs.faint);
    assert!(!attrs.italic);
    assert!(!attrs.underline);
    assert!(!attrs.blink);
    assert!(!attrs.inverse);
    assert!(!attrs.hidden);
    assert!(!attrs.strikethrough);
    assert_eq!(attrs.fg, Color::Default);
    assert_eq!(attrs.bg, Color::Default);
}

#[test]
fn test_cell_attrs_reset() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    attrs.italic = true;
    attrs.fg = Color::Indexed(1);
    attrs.reset();
    assert!(!attrs.bold);
    assert!(!attrs.italic);
    assert_eq!(attrs.fg, Color::Default);
}

#[test]
fn test_cell_attrs_effective_fg_normal() {
    let mut attrs = CellAttributes::new();
    attrs.fg = Color::Indexed(1);
    attrs.bg = Color::Indexed(4);
    assert_eq!(attrs.effective_fg(), Color::Indexed(1));
}

#[test]
fn test_cell_attrs_effective_bg_normal() {
    let mut attrs = CellAttributes::new();
    attrs.fg = Color::Indexed(1);
    attrs.bg = Color::Indexed(4);
    assert_eq!(attrs.effective_bg(), Color::Indexed(4));
}

#[test]
fn test_cell_attrs_effective_fg_inverse() {
    let mut attrs = CellAttributes::new();
    attrs.fg = Color::Indexed(1);
    attrs.bg = Color::Indexed(4);
    attrs.inverse = true;
    assert_eq!(attrs.effective_fg(), Color::Indexed(4));
}

#[test]
fn test_cell_attrs_effective_bg_inverse() {
    let mut attrs = CellAttributes::new();
    attrs.fg = Color::Indexed(1);
    attrs.bg = Color::Indexed(4);
    attrs.inverse = true;
    assert_eq!(attrs.effective_bg(), Color::Indexed(1));
}

#[test]
fn test_cell_attrs_inverse_with_defaults() {
    let mut attrs = CellAttributes::new();
    attrs.inverse = true;
    assert_eq!(attrs.effective_fg(), Color::Default);
    assert_eq!(attrs.effective_bg(), Color::Default);
}

#[test]
fn test_cell_attrs_inverse_with_rgb() {
    let mut attrs = CellAttributes::new();
    attrs.fg = Color::rgb(255, 0, 0);
    attrs.bg = Color::rgb(0, 0, 255);
    attrs.inverse = true;
    assert_eq!(attrs.effective_fg(), Color::rgb(0, 0, 255));
    assert_eq!(attrs.effective_bg(), Color::rgb(255, 0, 0));
}

#[test]
fn test_cell_attrs_copy() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    attrs.fg = Color::Indexed(3);
    let copy = attrs;
    assert!(copy.bold);
    assert_eq!(copy.fg, Color::Indexed(3));
}

#[test]
fn test_cell_attrs_default_eq() {
    let attrs1 = CellAttributes::default();
    let attrs2 = CellAttributes::new();
    assert_eq!(attrs1, attrs2);
}

#[test]
fn test_cell_attrs_reset_all_fields() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    attrs.faint = true;
    attrs.italic = true;
    attrs.underline = true;
    attrs.blink = true;
    attrs.inverse = true;
    attrs.hidden = true;
    attrs.strikethrough = true;
    attrs.fg = Color::Indexed(15);
    attrs.bg = Color::rgb(128, 128, 128);
    attrs.reset();
    assert_eq!(attrs, CellAttributes::default());
}

#[test]
fn test_cell_attrs_not_inverse_by_default() {
    let attrs = CellAttributes::new();
    assert!(!attrs.inverse);
    assert_eq!(attrs.effective_fg(), Color::Default);
    assert_eq!(attrs.effective_bg(), Color::Default);
}

// ============================================================
// Cell equality / clone Tests
// ============================================================

#[test]
fn test_cell_equality() {
    let cell1 = Cell::with_char('A');
    let cell2 = Cell::with_char('A');
    assert_eq!(cell1, cell2);
}

#[test]
fn test_cell_inequality_different_char() {
    let cell1 = Cell::with_char('A');
    let cell2 = Cell::with_char('B');
    assert_ne!(cell1, cell2);
}

#[test]
fn test_cell_inequality_different_attrs() {
    let cell1 = Cell::with_char('A');
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    let cell2 = Cell::with_char_and_attrs('A', attrs);
    assert_ne!(cell1, cell2);
}

#[test]
fn test_cell_clone() {
    let cell = Cell::with_char('A');
    let clone = cell.clone();
    assert_eq!(cell, clone);
}

#[test]
fn test_cell_clone_independence() {
    let cell = Cell::with_char('A');
    let mut clone = cell.clone();
    clone.set_char('B');
    assert_ne!(cell.display_char(), clone.display_char());
}

#[test]
fn test_cell_hyperlink_equality() {
    let mut cell1 = Cell::with_char('A');
    let mut cell2 = Cell::with_char('A');
    cell1.hyperlink_id = 1;
    cell2.hyperlink_id = 2;
    assert_ne!(cell1, cell2);
}

#[test]
fn test_cell_hyperlink_same_equality() {
    let mut cell1 = Cell::with_char('A');
    let mut cell2 = Cell::with_char('A');
    cell1.hyperlink_id = 5;
    cell2.hyperlink_id = 5;
    assert_eq!(cell1, cell2);
}
