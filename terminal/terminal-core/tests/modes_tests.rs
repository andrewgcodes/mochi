//! Comprehensive tests for terminal modes

use terminal_core::Modes;

// ============================================================
// Modes Creation Tests
// ============================================================

#[test]
fn test_modes_new_defaults() {
    let modes = Modes::new();
    assert!(!modes.insert_mode);
    assert!(!modes.linefeed_mode);
    assert!(!modes.cursor_keys_application);
    assert!(modes.ansi_mode);
    assert!(!modes.column_132);
    assert!(!modes.smooth_scroll);
    assert!(!modes.reverse_video);
    assert!(!modes.origin_mode);
    assert!(modes.auto_wrap);
    assert!(modes.auto_repeat);
    assert!(modes.cursor_visible);
    assert!(!modes.mouse_x10);
    assert!(!modes.mouse_vt200);
    assert!(!modes.mouse_button_event);
    assert!(!modes.mouse_any_event);
    assert!(!modes.mouse_sgr);
    assert!(!modes.focus_events);
    assert!(!modes.alternate_screen);
    assert!(!modes.bracketed_paste);
    assert!(!modes.synchronized_output);
}

#[test]
fn test_modes_default_equals_new() {
    let m1 = Modes::new();
    let m2 = Modes::default();
    assert_eq!(m1, m2);
}

// ============================================================
// set_dec_mode Tests
// ============================================================

#[test]
fn test_set_dec_mode_cursor_keys() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1, true);
    assert!(modes.cursor_keys_application);
    modes.set_dec_mode(1, false);
    assert!(!modes.cursor_keys_application);
}

#[test]
fn test_set_dec_mode_ansi_mode() {
    let mut modes = Modes::new();
    modes.set_dec_mode(2, false);
    assert!(!modes.ansi_mode);
}

#[test]
fn test_set_dec_mode_column_132() {
    let mut modes = Modes::new();
    modes.set_dec_mode(3, true);
    assert!(modes.column_132);
}

#[test]
fn test_set_dec_mode_smooth_scroll() {
    let mut modes = Modes::new();
    modes.set_dec_mode(4, true);
    assert!(modes.smooth_scroll);
}

#[test]
fn test_set_dec_mode_reverse_video() {
    let mut modes = Modes::new();
    modes.set_dec_mode(5, true);
    assert!(modes.reverse_video);
}

#[test]
fn test_set_dec_mode_origin() {
    let mut modes = Modes::new();
    modes.set_dec_mode(6, true);
    assert!(modes.origin_mode);
}

#[test]
fn test_set_dec_mode_auto_wrap() {
    let mut modes = Modes::new();
    modes.set_dec_mode(7, false);
    assert!(!modes.auto_wrap);
}

#[test]
fn test_set_dec_mode_auto_repeat() {
    let mut modes = Modes::new();
    modes.set_dec_mode(8, false);
    assert!(!modes.auto_repeat);
}

#[test]
fn test_set_dec_mode_mouse_x10() {
    let mut modes = Modes::new();
    modes.set_dec_mode(9, true);
    assert!(modes.mouse_x10);
}

#[test]
fn test_set_dec_mode_cursor_visible() {
    let mut modes = Modes::new();
    modes.set_dec_mode(25, false);
    assert!(!modes.cursor_visible);
}

#[test]
fn test_set_dec_mode_mouse_vt200() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1000, true);
    assert!(modes.mouse_vt200);
}

#[test]
fn test_set_dec_mode_mouse_button_event() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1002, true);
    assert!(modes.mouse_button_event);
}

#[test]
fn test_set_dec_mode_mouse_any_event() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1003, true);
    assert!(modes.mouse_any_event);
}

#[test]
fn test_set_dec_mode_focus_events() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1004, true);
    assert!(modes.focus_events);
}

#[test]
fn test_set_dec_mode_mouse_sgr() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1006, true);
    assert!(modes.mouse_sgr);
}

#[test]
fn test_set_dec_mode_alternate_screen() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1049, true);
    assert!(modes.alternate_screen);
}

#[test]
fn test_set_dec_mode_bracketed_paste() {
    let mut modes = Modes::new();
    modes.set_dec_mode(2004, true);
    assert!(modes.bracketed_paste);
}

#[test]
fn test_set_dec_mode_synchronized_output() {
    let mut modes = Modes::new();
    modes.set_dec_mode(2026, true);
    assert!(modes.synchronized_output);
}

#[test]
fn test_set_dec_mode_unknown() {
    let mut modes = Modes::new();
    let before = modes.clone();
    modes.set_dec_mode(9999, true);
    // Unknown modes should not change anything
    assert_eq!(modes.insert_mode, before.insert_mode);
}

// ============================================================
// get_dec_mode Tests
// ============================================================

#[test]
fn test_get_dec_mode_cursor_keys() {
    let modes = Modes::new();
    assert!(!modes.get_dec_mode(1));
}

#[test]
fn test_get_dec_mode_ansi() {
    let modes = Modes::new();
    assert!(modes.get_dec_mode(2));
}

#[test]
fn test_get_dec_mode_auto_wrap() {
    let modes = Modes::new();
    assert!(modes.get_dec_mode(7));
}

#[test]
fn test_get_dec_mode_cursor_visible() {
    let modes = Modes::new();
    assert!(modes.get_dec_mode(25));
}

#[test]
fn test_get_dec_mode_alternate_screen() {
    let modes = Modes::new();
    assert!(!modes.get_dec_mode(1049));
}

#[test]
fn test_get_dec_mode_unknown() {
    let modes = Modes::new();
    assert!(!modes.get_dec_mode(9999));
}

#[test]
fn test_get_dec_mode_after_set() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1049, true);
    assert!(modes.get_dec_mode(1049));
    modes.set_dec_mode(1049, false);
    assert!(!modes.get_dec_mode(1049));
}

// ============================================================
// set_mode Tests (standard modes)
// ============================================================

#[test]
fn test_set_mode_insert() {
    let mut modes = Modes::new();
    modes.set_mode(4, true);
    assert!(modes.insert_mode);
    modes.set_mode(4, false);
    assert!(!modes.insert_mode);
}

#[test]
fn test_set_mode_linefeed() {
    let mut modes = Modes::new();
    modes.set_mode(20, true);
    assert!(modes.linefeed_mode);
    modes.set_mode(20, false);
    assert!(!modes.linefeed_mode);
}

#[test]
fn test_set_mode_unknown() {
    let mut modes = Modes::new();
    modes.set_mode(9999, true);
    // Should not crash, no visible change
    assert!(!modes.insert_mode);
}

// ============================================================
// mouse_tracking_enabled Tests
// ============================================================

#[test]
fn test_mouse_tracking_none() {
    let modes = Modes::new();
    assert!(!modes.mouse_tracking_enabled());
}

#[test]
fn test_mouse_tracking_x10() {
    let mut modes = Modes::new();
    modes.mouse_x10 = true;
    assert!(modes.mouse_tracking_enabled());
}

#[test]
fn test_mouse_tracking_vt200() {
    let mut modes = Modes::new();
    modes.mouse_vt200 = true;
    assert!(modes.mouse_tracking_enabled());
}

#[test]
fn test_mouse_tracking_button_event() {
    let mut modes = Modes::new();
    modes.mouse_button_event = true;
    assert!(modes.mouse_tracking_enabled());
}

#[test]
fn test_mouse_tracking_any_event() {
    let mut modes = Modes::new();
    modes.mouse_any_event = true;
    assert!(modes.mouse_tracking_enabled());
}

#[test]
fn test_mouse_tracking_sgr_alone_not_sufficient() {
    let mut modes = Modes::new();
    modes.mouse_sgr = true;
    assert!(!modes.mouse_tracking_enabled());
}

// ============================================================
// reset Tests
// ============================================================

#[test]
fn test_modes_reset() {
    let mut modes = Modes::new();
    modes.cursor_visible = false;
    modes.alternate_screen = true;
    modes.bracketed_paste = true;
    modes.insert_mode = true;
    modes.mouse_vt200 = true;
    modes.auto_wrap = false;

    modes.reset();

    assert_eq!(modes, Modes::new());
}

// ============================================================
// Clone/Eq Tests
// ============================================================

#[test]
fn test_modes_clone() {
    let mut modes = Modes::new();
    modes.alternate_screen = true;
    let clone = modes.clone();
    assert_eq!(modes, clone);
}

#[test]
fn test_modes_inequality() {
    let mut m1 = Modes::new();
    let m2 = Modes::new();
    m1.alternate_screen = true;
    assert_ne!(m1, m2);
}

// ============================================================
// All DEC modes roundtrip Tests
// ============================================================

#[test]
fn test_all_known_dec_modes_set_get() {
    let known_modes: &[u16] = &[
        1, 2, 3, 4, 5, 6, 7, 8, 9, 25, 1000, 1002, 1003, 1004, 1006, 1049, 2004, 2026,
    ];
    for &mode in known_modes {
        let mut modes = Modes::new();
        let initial = modes.get_dec_mode(mode);
        modes.set_dec_mode(mode, !initial);
        assert_eq!(
            modes.get_dec_mode(mode),
            !initial,
            "Failed for mode {}",
            mode
        );
    }
}
