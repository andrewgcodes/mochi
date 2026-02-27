//! Comprehensive tests for terminal mode flags

use terminal_core::Modes;

// ============================================================================
// Modes Creation & Defaults
// ============================================================================

#[test]
fn test_modes_new_insert_mode_off() {
    let modes = Modes::new();
    assert!(!modes.insert_mode);
}

#[test]
fn test_modes_new_linefeed_mode_off() {
    let modes = Modes::new();
    assert!(!modes.linefeed_mode);
}

#[test]
fn test_modes_new_cursor_keys_normal() {
    let modes = Modes::new();
    assert!(!modes.cursor_keys_application);
}

#[test]
fn test_modes_new_ansi_mode_on() {
    let modes = Modes::new();
    assert!(modes.ansi_mode);
}

#[test]
fn test_modes_new_column_132_off() {
    let modes = Modes::new();
    assert!(!modes.column_132);
}

#[test]
fn test_modes_new_smooth_scroll_off() {
    let modes = Modes::new();
    assert!(!modes.smooth_scroll);
}

#[test]
fn test_modes_new_reverse_video_off() {
    let modes = Modes::new();
    assert!(!modes.reverse_video);
}

#[test]
fn test_modes_new_origin_mode_off() {
    let modes = Modes::new();
    assert!(!modes.origin_mode);
}

#[test]
fn test_modes_new_auto_wrap_on() {
    let modes = Modes::new();
    assert!(modes.auto_wrap);
}

#[test]
fn test_modes_new_auto_repeat_on() {
    let modes = Modes::new();
    assert!(modes.auto_repeat);
}

#[test]
fn test_modes_new_cursor_visible_on() {
    let modes = Modes::new();
    assert!(modes.cursor_visible);
}

#[test]
fn test_modes_new_mouse_x10_off() {
    let modes = Modes::new();
    assert!(!modes.mouse_x10);
}

#[test]
fn test_modes_new_mouse_vt200_off() {
    let modes = Modes::new();
    assert!(!modes.mouse_vt200);
}

#[test]
fn test_modes_new_mouse_button_event_off() {
    let modes = Modes::new();
    assert!(!modes.mouse_button_event);
}

#[test]
fn test_modes_new_mouse_any_event_off() {
    let modes = Modes::new();
    assert!(!modes.mouse_any_event);
}

#[test]
fn test_modes_new_mouse_sgr_off() {
    let modes = Modes::new();
    assert!(!modes.mouse_sgr);
}

#[test]
fn test_modes_new_focus_events_off() {
    let modes = Modes::new();
    assert!(!modes.focus_events);
}

#[test]
fn test_modes_new_alternate_screen_off() {
    let modes = Modes::new();
    assert!(!modes.alternate_screen);
}

#[test]
fn test_modes_new_bracketed_paste_off() {
    let modes = Modes::new();
    assert!(!modes.bracketed_paste);
}

#[test]
fn test_modes_new_synchronized_output_off() {
    let modes = Modes::new();
    assert!(!modes.synchronized_output);
}

#[test]
fn test_modes_default_trait() {
    let modes = Modes::default();
    assert!(modes.auto_wrap);
    assert!(modes.cursor_visible);
}

// ============================================================================
// Modes::set_dec_mode
// ============================================================================

#[test]
fn test_set_dec_mode_1_cursor_keys() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1, true);
    assert!(modes.cursor_keys_application);
    modes.set_dec_mode(1, false);
    assert!(!modes.cursor_keys_application);
}

#[test]
fn test_set_dec_mode_2_ansi() {
    let mut modes = Modes::new();
    modes.set_dec_mode(2, false);
    assert!(!modes.ansi_mode);
}

#[test]
fn test_set_dec_mode_3_column_132() {
    let mut modes = Modes::new();
    modes.set_dec_mode(3, true);
    assert!(modes.column_132);
}

#[test]
fn test_set_dec_mode_4_smooth_scroll() {
    let mut modes = Modes::new();
    modes.set_dec_mode(4, true);
    assert!(modes.smooth_scroll);
}

#[test]
fn test_set_dec_mode_5_reverse_video() {
    let mut modes = Modes::new();
    modes.set_dec_mode(5, true);
    assert!(modes.reverse_video);
}

#[test]
fn test_set_dec_mode_6_origin() {
    let mut modes = Modes::new();
    modes.set_dec_mode(6, true);
    assert!(modes.origin_mode);
}

#[test]
fn test_set_dec_mode_7_auto_wrap() {
    let mut modes = Modes::new();
    modes.set_dec_mode(7, false);
    assert!(!modes.auto_wrap);
}

#[test]
fn test_set_dec_mode_8_auto_repeat() {
    let mut modes = Modes::new();
    modes.set_dec_mode(8, false);
    assert!(!modes.auto_repeat);
}

#[test]
fn test_set_dec_mode_9_mouse_x10() {
    let mut modes = Modes::new();
    modes.set_dec_mode(9, true);
    assert!(modes.mouse_x10);
}

#[test]
fn test_set_dec_mode_25_cursor_visible() {
    let mut modes = Modes::new();
    modes.set_dec_mode(25, false);
    assert!(!modes.cursor_visible);
    modes.set_dec_mode(25, true);
    assert!(modes.cursor_visible);
}

#[test]
fn test_set_dec_mode_1000_mouse_vt200() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1000, true);
    assert!(modes.mouse_vt200);
}

#[test]
fn test_set_dec_mode_1002_mouse_button() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1002, true);
    assert!(modes.mouse_button_event);
}

#[test]
fn test_set_dec_mode_1003_mouse_any() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1003, true);
    assert!(modes.mouse_any_event);
}

#[test]
fn test_set_dec_mode_1004_focus() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1004, true);
    assert!(modes.focus_events);
}

#[test]
fn test_set_dec_mode_1006_mouse_sgr() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1006, true);
    assert!(modes.mouse_sgr);
}

#[test]
fn test_set_dec_mode_1049_alternate_screen() {
    let mut modes = Modes::new();
    modes.set_dec_mode(1049, true);
    assert!(modes.alternate_screen);
}

#[test]
fn test_set_dec_mode_2004_bracketed_paste() {
    let mut modes = Modes::new();
    modes.set_dec_mode(2004, true);
    assert!(modes.bracketed_paste);
}

#[test]
fn test_set_dec_mode_2026_synchronized_output() {
    let mut modes = Modes::new();
    modes.set_dec_mode(2026, true);
    assert!(modes.synchronized_output);
}

#[test]
fn test_set_dec_mode_unknown() {
    let mut modes = Modes::new();
    modes.set_dec_mode(9999, true); // Unknown mode - should not panic
                                    // Just verify it doesn't crash
}

// ============================================================================
// Modes::get_dec_mode
// ============================================================================

#[test]
fn test_get_dec_mode_1() {
    let modes = Modes::new();
    assert!(!modes.get_dec_mode(1));
}

#[test]
fn test_get_dec_mode_25() {
    let modes = Modes::new();
    assert!(modes.get_dec_mode(25));
}

#[test]
fn test_get_dec_mode_1049() {
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
    modes.set_dec_mode(2004, true);
    assert!(modes.get_dec_mode(2004));
    modes.set_dec_mode(2004, false);
    assert!(!modes.get_dec_mode(2004));
}

// ============================================================================
// Modes::set_mode (standard/ANSI modes)
// ============================================================================

#[test]
fn test_set_mode_4_insert() {
    let mut modes = Modes::new();
    modes.set_mode(4, true);
    assert!(modes.insert_mode);
    modes.set_mode(4, false);
    assert!(!modes.insert_mode);
}

#[test]
fn test_set_mode_20_linefeed() {
    let mut modes = Modes::new();
    modes.set_mode(20, true);
    assert!(modes.linefeed_mode);
}

#[test]
fn test_set_mode_unknown() {
    let mut modes = Modes::new();
    modes.set_mode(999, true); // Should not panic
}

// ============================================================================
// Modes::mouse_tracking_enabled
// ============================================================================

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
fn test_mouse_tracking_sgr_alone_not_tracking() {
    let mut modes = Modes::new();
    modes.mouse_sgr = true;
    assert!(!modes.mouse_tracking_enabled()); // SGR is format, not tracking
}

#[test]
fn test_mouse_tracking_multiple() {
    let mut modes = Modes::new();
    modes.mouse_x10 = true;
    modes.mouse_vt200 = true;
    assert!(modes.mouse_tracking_enabled());
}

// ============================================================================
// Modes::reset
// ============================================================================

#[test]
fn test_modes_reset_restores_defaults() {
    let mut modes = Modes::new();
    modes.insert_mode = true;
    modes.cursor_visible = false;
    modes.alternate_screen = true;
    modes.bracketed_paste = true;
    modes.mouse_vt200 = true;
    modes.auto_wrap = false;

    modes.reset();

    assert!(!modes.insert_mode);
    assert!(modes.cursor_visible);
    assert!(!modes.alternate_screen);
    assert!(!modes.bracketed_paste);
    assert!(!modes.mouse_vt200);
    assert!(modes.auto_wrap);
}

#[test]
fn test_modes_reset_all_mouse_modes() {
    let mut modes = Modes::new();
    modes.mouse_x10 = true;
    modes.mouse_vt200 = true;
    modes.mouse_button_event = true;
    modes.mouse_any_event = true;
    modes.mouse_sgr = true;

    modes.reset();

    assert!(!modes.mouse_x10);
    assert!(!modes.mouse_vt200);
    assert!(!modes.mouse_button_event);
    assert!(!modes.mouse_any_event);
    assert!(!modes.mouse_sgr);
}
