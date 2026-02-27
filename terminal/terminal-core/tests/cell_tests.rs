//! Comprehensive tests for terminal cell representation
//!
//! Tests cover: Cell creation, character storage, attributes, wide chars,
//! continuation cells, clearing, resetting, and future features.

use terminal_core::{Cell, CellAttributes, Color};

// ============================================================================
// Cell Creation & Defaults
// ============================================================================

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
fn test_cell_new_content_is_empty_string() {
    let cell = Cell::new();
    assert_eq!(cell.content(), "");
}

#[test]
fn test_cell_new_display_char_is_space() {
    let cell = Cell::new();
    assert_eq!(cell.display_char(), ' ');
}

#[test]
fn test_cell_new_is_not_continuation() {
    let cell = Cell::new();
    assert!(!cell.is_continuation());
}

#[test]
fn test_cell_new_hyperlink_id_is_zero() {
    let cell = Cell::new();
    assert_eq!(cell.hyperlink_id, 0);
}

#[test]
fn test_cell_new_attrs_are_default() {
    let cell = Cell::new();
    assert_eq!(cell.attrs, CellAttributes::default());
}

#[test]
fn test_cell_default_trait() {
    let cell = Cell::default();
    assert!(cell.is_empty());
    assert_eq!(cell.width(), 1);
}

// ============================================================================
// Cell::with_char
// ============================================================================

#[test]
fn test_cell_with_char_ascii() {
    let cell = Cell::with_char('A');
    assert_eq!(cell.display_char(), 'A');
    assert_eq!(cell.width(), 1);
    assert!(!cell.is_empty());
}

#[test]
fn test_cell_with_char_digit() {
    let cell = Cell::with_char('9');
    assert_eq!(cell.display_char(), '9');
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_with_char_space() {
    let cell = Cell::with_char(' ');
    assert_eq!(cell.display_char(), ' ');
    assert!(cell.is_empty()); // Space counts as empty
}

#[test]
fn test_cell_with_char_special_symbol() {
    let cell = Cell::with_char('!');
    assert_eq!(cell.display_char(), '!');
    assert!(!cell.is_empty());
}

#[test]
fn test_cell_with_char_tilde() {
    let cell = Cell::with_char('~');
    assert_eq!(cell.display_char(), '~');
    assert_eq!(cell.width(), 1);
}

// ============================================================================
// Wide Characters (CJK)
// ============================================================================

#[test]
fn test_cell_cjk_chinese() {
    let cell = Cell::with_char('中');
    assert_eq!(cell.display_char(), '中');
    assert_eq!(cell.width(), 2);
}

#[test]
fn test_cell_cjk_japanese_hiragana() {
    let cell = Cell::with_char('あ');
    assert_eq!(cell.display_char(), 'あ');
    assert_eq!(cell.width(), 2);
}

#[test]
fn test_cell_cjk_katakana() {
    let cell = Cell::with_char('ア');
    assert_eq!(cell.display_char(), 'ア');
    assert_eq!(cell.width(), 2);
}

#[test]
fn test_cell_cjk_korean() {
    let cell = Cell::with_char('한');
    assert_eq!(cell.display_char(), '한');
    assert_eq!(cell.width(), 2);
}

#[test]
fn test_cell_fullwidth_latin() {
    let cell = Cell::with_char('Ａ'); // Fullwidth A
    assert_eq!(cell.display_char(), 'Ａ');
    assert_eq!(cell.width(), 2);
}

// ============================================================================
// Unicode Characters
// ============================================================================

#[test]
fn test_cell_accented_char() {
    let cell = Cell::with_char('é');
    assert_eq!(cell.display_char(), 'é');
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_greek_letter() {
    let cell = Cell::with_char('α');
    assert_eq!(cell.display_char(), 'α');
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_cyrillic_letter() {
    let cell = Cell::with_char('Д');
    assert_eq!(cell.display_char(), 'Д');
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_emoji_basic() {
    let cell = Cell::with_char('😀');
    assert_eq!(cell.display_char(), '😀');
    assert_eq!(cell.width(), 2);
}

#[test]
fn test_cell_box_drawing() {
    let cell = Cell::with_char('┌');
    assert_eq!(cell.display_char(), '┌');
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_block_element() {
    let cell = Cell::with_char('█');
    assert_eq!(cell.display_char(), '█');
    assert_eq!(cell.width(), 1);
}

// ============================================================================
// Cell::with_char_and_attrs
// ============================================================================

#[test]
fn test_cell_with_char_and_attrs_bold() {
    let attrs = CellAttributes {
        bold: true,
        ..Default::default()
    };
    let cell = Cell::with_char_and_attrs('X', attrs);
    assert_eq!(cell.display_char(), 'X');
    assert!(cell.attrs.bold);
}

#[test]
fn test_cell_with_char_and_attrs_fg_color() {
    let attrs = CellAttributes {
        fg: Color::Indexed(1),
        ..Default::default()
    };
    let cell = Cell::with_char_and_attrs('R', attrs);
    assert_eq!(cell.attrs.fg, Color::Indexed(1));
}

#[test]
fn test_cell_with_char_and_attrs_multiple() {
    let attrs = CellAttributes {
        bold: true,
        italic: true,
        underline: true,
        ..Default::default()
    };
    let cell = Cell::with_char_and_attrs('M', attrs);
    assert!(cell.attrs.bold);
    assert!(cell.attrs.italic);
    assert!(cell.attrs.underline);
}

// ============================================================================
// Cell::set_char
// ============================================================================

#[test]
fn test_cell_set_char_changes_content() {
    let mut cell = Cell::new();
    cell.set_char('Z');
    assert_eq!(cell.display_char(), 'Z');
    assert!(!cell.is_empty());
}

#[test]
fn test_cell_set_char_updates_width_for_wide() {
    let mut cell = Cell::new();
    cell.set_char('中');
    assert_eq!(cell.width(), 2);
}

#[test]
fn test_cell_set_char_overwrites() {
    let mut cell = Cell::with_char('A');
    cell.set_char('B');
    assert_eq!(cell.display_char(), 'B');
}

#[test]
fn test_cell_set_char_narrow_after_wide() {
    let mut cell = Cell::new();
    cell.set_char('中');
    assert_eq!(cell.width(), 2);
    cell.set_char('A');
    assert_eq!(cell.width(), 1);
}

// ============================================================================
// Cell::set_content (grapheme clusters)
// ============================================================================

#[test]
fn test_cell_set_content_single_char() {
    let mut cell = Cell::new();
    cell.set_content("A");
    assert_eq!(cell.content(), "A");
}

#[test]
fn test_cell_set_content_empty_string() {
    let mut cell = Cell::with_char('A');
    cell.set_content("");
    assert_eq!(cell.content(), "");
    assert_eq!(cell.width(), 1); // Width defaults to 1 for empty
}

#[test]
fn test_cell_set_content_multi_codepoint() {
    let mut cell = Cell::new();
    cell.set_content("e\u{0301}"); // e + combining acute
    assert_eq!(cell.content(), "e\u{0301}");
}

// ============================================================================
// Cell::set_continuation
// ============================================================================

#[test]
fn test_cell_set_continuation_width() {
    let mut cell = Cell::with_char('X');
    cell.set_continuation();
    assert_eq!(cell.width(), 0);
}

#[test]
fn test_cell_set_continuation_clears_content() {
    let mut cell = Cell::with_char('X');
    cell.set_continuation();
    assert_eq!(cell.content(), "");
}

#[test]
fn test_cell_is_continuation_after_set() {
    let mut cell = Cell::new();
    cell.set_continuation();
    assert!(cell.is_continuation());
}

// ============================================================================
// Cell::clear
// ============================================================================

#[test]
fn test_cell_clear_empties_content() {
    let mut cell = Cell::with_char('A');
    cell.clear(CellAttributes::default());
    assert!(cell.is_empty());
}

#[test]
fn test_cell_clear_resets_width() {
    let mut cell = Cell::new();
    cell.set_continuation();
    cell.clear(CellAttributes::default());
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_clear_applies_attrs() {
    let attrs = CellAttributes {
        bold: true,
        ..Default::default()
    };
    let mut cell = Cell::with_char('X');
    cell.clear(attrs);
    assert!(cell.attrs.bold);
}

#[test]
fn test_cell_clear_resets_hyperlink() {
    let mut cell = Cell::with_char('X');
    cell.hyperlink_id = 5;
    cell.clear(CellAttributes::default());
    assert_eq!(cell.hyperlink_id, 0);
}

// ============================================================================
// Cell::reset
// ============================================================================

#[test]
fn test_cell_reset_empties_content() {
    let mut cell = Cell::with_char('A');
    cell.reset();
    assert!(cell.is_empty());
}

#[test]
fn test_cell_reset_clears_attrs() {
    let mut cell = Cell::with_char('A');
    cell.attrs.bold = true;
    cell.attrs.fg = Color::Indexed(1);
    cell.reset();
    assert_eq!(cell.attrs, CellAttributes::default());
}

#[test]
fn test_cell_reset_restores_width() {
    let mut cell = Cell::new();
    cell.set_continuation();
    cell.reset();
    assert_eq!(cell.width(), 1);
}

#[test]
fn test_cell_reset_clears_hyperlink() {
    let mut cell = Cell::new();
    cell.hyperlink_id = 42;
    cell.reset();
    assert_eq!(cell.hyperlink_id, 0);
}

// ============================================================================
// CellAttributes
// ============================================================================

#[test]
fn test_cell_attrs_new_all_false() {
    let attrs = CellAttributes::new();
    assert!(!attrs.bold);
    assert!(!attrs.faint);
    assert!(!attrs.italic);
    assert!(!attrs.underline);
    assert!(!attrs.blink);
    assert!(!attrs.inverse);
    assert!(!attrs.hidden);
    assert!(!attrs.strikethrough);
}

#[test]
fn test_cell_attrs_new_default_colors() {
    let attrs = CellAttributes::new();
    assert_eq!(attrs.fg, Color::Default);
    assert_eq!(attrs.bg, Color::Default);
}

#[test]
fn test_cell_attrs_reset_clears_all() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    attrs.italic = true;
    attrs.underline = true;
    attrs.fg = Color::Indexed(3);
    attrs.bg = Color::rgb(255, 0, 0);
    attrs.reset();
    assert_eq!(attrs, CellAttributes::default());
}

#[test]
fn test_cell_attrs_effective_fg_normal() {
    let mut attrs = CellAttributes::new();
    attrs.fg = Color::Indexed(1);
    attrs.bg = Color::Indexed(0);
    assert_eq!(attrs.effective_fg(), Color::Indexed(1));
}

#[test]
fn test_cell_attrs_effective_bg_normal() {
    let mut attrs = CellAttributes::new();
    attrs.fg = Color::Indexed(1);
    attrs.bg = Color::Indexed(0);
    assert_eq!(attrs.effective_bg(), Color::Indexed(0));
}

#[test]
fn test_cell_attrs_effective_fg_inverse() {
    let mut attrs = CellAttributes::new();
    attrs.fg = Color::Indexed(1);
    attrs.bg = Color::Indexed(0);
    attrs.inverse = true;
    assert_eq!(attrs.effective_fg(), Color::Indexed(0));
}

#[test]
fn test_cell_attrs_effective_bg_inverse() {
    let mut attrs = CellAttributes::new();
    attrs.fg = Color::Indexed(1);
    attrs.bg = Color::Indexed(0);
    attrs.inverse = true;
    assert_eq!(attrs.effective_bg(), Color::Indexed(1));
}

#[test]
fn test_cell_attrs_effective_fg_inverse_with_rgb() {
    let mut attrs = CellAttributes::new();
    attrs.fg = Color::rgb(255, 0, 0);
    attrs.bg = Color::rgb(0, 0, 255);
    attrs.inverse = true;
    assert_eq!(attrs.effective_fg(), Color::rgb(0, 0, 255));
}

#[test]
fn test_cell_attrs_effective_bg_inverse_with_rgb() {
    let mut attrs = CellAttributes::new();
    attrs.fg = Color::rgb(255, 0, 0);
    attrs.bg = Color::rgb(0, 0, 255);
    attrs.inverse = true;
    assert_eq!(attrs.effective_bg(), Color::rgb(255, 0, 0));
}

#[test]
fn test_cell_attrs_clone_equality() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    attrs.fg = Color::Indexed(5);
    let cloned = attrs;
    assert_eq!(attrs, cloned);
}

#[test]
fn test_cell_attrs_each_flag_independent() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    assert!(attrs.bold);
    assert!(!attrs.italic);
    assert!(!attrs.underline);
    assert!(!attrs.faint);
    assert!(!attrs.blink);
    assert!(!attrs.inverse);
    assert!(!attrs.hidden);
    assert!(!attrs.strikethrough);
}

// ============================================================================
// Hyperlink support
// ============================================================================

#[test]
fn test_cell_hyperlink_assignment() {
    let mut cell = Cell::new();
    cell.hyperlink_id = 1;
    assert_eq!(cell.hyperlink_id, 1);
}

#[test]
fn test_cell_hyperlink_preserved_on_set_char() {
    let mut cell = Cell::new();
    cell.hyperlink_id = 5;
    cell.set_char('A');
    assert_eq!(cell.hyperlink_id, 5);
}

// ============================================================================
// Future Feature: Cell styling combinations
// ============================================================================

#[test]
fn test_cell_bold_italic_combination() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    attrs.italic = true;
    let cell = Cell::with_char_and_attrs('B', attrs);
    assert!(cell.attrs.bold);
    assert!(cell.attrs.italic);
}

#[test]
fn test_cell_underline_strikethrough_combination() {
    let mut attrs = CellAttributes::new();
    attrs.underline = true;
    attrs.strikethrough = true;
    let cell = Cell::with_char_and_attrs('U', attrs);
    assert!(cell.attrs.underline);
    assert!(cell.attrs.strikethrough);
}

#[test]
fn test_cell_all_styling_flags_at_once() {
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    attrs.faint = true;
    attrs.italic = true;
    attrs.underline = true;
    attrs.blink = true;
    attrs.inverse = true;
    attrs.hidden = true;
    attrs.strikethrough = true;
    let cell = Cell::with_char_and_attrs('X', attrs);
    assert!(cell.attrs.bold);
    assert!(cell.attrs.faint);
    assert!(cell.attrs.italic);
    assert!(cell.attrs.underline);
    assert!(cell.attrs.blink);
    assert!(cell.attrs.inverse);
    assert!(cell.attrs.hidden);
    assert!(cell.attrs.strikethrough);
}

#[test]
fn test_cell_faint_and_bold_simultaneous() {
    // SGR 1 (bold) and SGR 2 (faint) can coexist per spec
    let mut attrs = CellAttributes::new();
    attrs.bold = true;
    attrs.faint = true;
    assert!(attrs.bold);
    assert!(attrs.faint);
}

#[test]
fn test_cell_hidden_text_attrs() {
    let mut attrs = CellAttributes::new();
    attrs.hidden = true;
    let cell = Cell::with_char_and_attrs('S', attrs);
    assert!(cell.attrs.hidden);
    // Hidden text should still have content
    assert_eq!(cell.display_char(), 'S');
}

#[test]
fn test_cell_blink_attr() {
    let mut attrs = CellAttributes::new();
    attrs.blink = true;
    assert!(attrs.blink);
}
