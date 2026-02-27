//! Comprehensive tests for terminal parser action types

use terminal_parser::{Action, CsiAction, EscAction, OscAction, Params};

// ============================================================================
// Action variants
// ============================================================================

#[test]
fn test_action_print() {
    let action = Action::Print('A');
    assert_eq!(action, Action::Print('A'));
}

#[test]
fn test_action_print_unicode() {
    let action = Action::Print('中');
    assert_eq!(action, Action::Print('中'));
}

#[test]
fn test_action_print_emoji() {
    let action = Action::Print('🎉');
    assert_eq!(action, Action::Print('🎉'));
}

#[test]
fn test_action_control_bel() {
    let action = Action::Control(0x07);
    assert_eq!(action, Action::Control(0x07));
}

#[test]
fn test_action_control_bs() {
    let action = Action::Control(0x08);
    assert_eq!(action, Action::Control(0x08));
}

#[test]
fn test_action_control_ht() {
    let action = Action::Control(0x09);
    assert_eq!(action, Action::Control(0x09));
}

#[test]
fn test_action_control_lf() {
    let action = Action::Control(0x0A);
    assert_eq!(action, Action::Control(0x0A));
}

#[test]
fn test_action_control_vt() {
    let action = Action::Control(0x0B);
    assert_eq!(action, Action::Control(0x0B));
}

#[test]
fn test_action_control_ff() {
    let action = Action::Control(0x0C);
    assert_eq!(action, Action::Control(0x0C));
}

#[test]
fn test_action_control_cr() {
    let action = Action::Control(0x0D);
    assert_eq!(action, Action::Control(0x0D));
}

#[test]
fn test_action_control_so() {
    let action = Action::Control(0x0E);
    assert_eq!(action, Action::Control(0x0E));
}

#[test]
fn test_action_control_si() {
    let action = Action::Control(0x0F);
    assert_eq!(action, Action::Control(0x0F));
}

#[test]
fn test_action_esc() {
    let action = Action::Esc(EscAction::SaveCursor);
    assert_eq!(action, Action::Esc(EscAction::SaveCursor));
}

#[test]
fn test_action_csi() {
    let csi = CsiAction {
        params: Params::new(),
        intermediates: vec![],
        final_byte: b'H',
        private: false,
    };
    let action = Action::Csi(csi.clone());
    assert_eq!(action, Action::Csi(csi));
}

#[test]
fn test_action_osc() {
    let action = Action::Osc(OscAction::SetTitle("test".to_string()));
    assert_eq!(action, Action::Osc(OscAction::SetTitle("test".to_string())));
}

#[test]
fn test_action_dcs() {
    let action = Action::Dcs {
        params: Params::new(),
        data: vec![1, 2, 3],
    };
    if let Action::Dcs { params, data } = &action {
        assert!(params.is_empty());
        assert_eq!(data, &vec![1, 2, 3]);
    } else {
        panic!("Expected Dcs");
    }
}

#[test]
fn test_action_apc() {
    let action = Action::Apc(vec![0x41, 0x42]);
    assert_eq!(action, Action::Apc(vec![0x41, 0x42]));
}

#[test]
fn test_action_pm() {
    let action = Action::Pm(vec![0x43]);
    assert_eq!(action, Action::Pm(vec![0x43]));
}

#[test]
fn test_action_sos() {
    let action = Action::Sos(vec![0x44, 0x45]);
    assert_eq!(action, Action::Sos(vec![0x44, 0x45]));
}

#[test]
fn test_action_invalid() {
    let action = Action::Invalid(vec![0xFF]);
    assert_eq!(action, Action::Invalid(vec![0xFF]));
}

// ============================================================================
// Action clone
// ============================================================================

#[test]
fn test_action_print_clone() {
    let action = Action::Print('X');
    let cloned = action.clone();
    assert_eq!(action, cloned);
}

#[test]
fn test_action_control_clone() {
    let action = Action::Control(0x07);
    let cloned = action.clone();
    assert_eq!(action, cloned);
}

#[test]
fn test_action_esc_clone() {
    let action = Action::Esc(EscAction::Index);
    let cloned = action.clone();
    assert_eq!(action, cloned);
}

// ============================================================================
// Action inequality
// ============================================================================

#[test]
fn test_action_print_ne_control() {
    assert_ne!(Action::Print('A'), Action::Control(0x41));
}

#[test]
fn test_action_different_chars() {
    assert_ne!(Action::Print('A'), Action::Print('B'));
}

#[test]
fn test_action_different_controls() {
    assert_ne!(Action::Control(0x07), Action::Control(0x08));
}

// ============================================================================
// EscAction variants
// ============================================================================

#[test]
fn test_esc_save_cursor() {
    let action = EscAction::SaveCursor;
    assert_eq!(action, EscAction::SaveCursor);
}

#[test]
fn test_esc_restore_cursor() {
    let action = EscAction::RestoreCursor;
    assert_eq!(action, EscAction::RestoreCursor);
}

#[test]
fn test_esc_index() {
    let action = EscAction::Index;
    assert_eq!(action, EscAction::Index);
}

#[test]
fn test_esc_reverse_index() {
    let action = EscAction::ReverseIndex;
    assert_eq!(action, EscAction::ReverseIndex);
}

#[test]
fn test_esc_next_line() {
    let action = EscAction::NextLine;
    assert_eq!(action, EscAction::NextLine);
}

#[test]
fn test_esc_horizontal_tab_set() {
    let action = EscAction::HorizontalTabSet;
    assert_eq!(action, EscAction::HorizontalTabSet);
}

#[test]
fn test_esc_full_reset() {
    let action = EscAction::FullReset;
    assert_eq!(action, EscAction::FullReset);
}

#[test]
fn test_esc_application_keypad() {
    let action = EscAction::ApplicationKeypad;
    assert_eq!(action, EscAction::ApplicationKeypad);
}

#[test]
fn test_esc_normal_keypad() {
    let action = EscAction::NormalKeypad;
    assert_eq!(action, EscAction::NormalKeypad);
}

#[test]
fn test_esc_designate_g0() {
    let action = EscAction::DesignateG0('B');
    assert_eq!(action, EscAction::DesignateG0('B'));
}

#[test]
fn test_esc_designate_g0_dec() {
    let action = EscAction::DesignateG0('0');
    assert_eq!(action, EscAction::DesignateG0('0'));
}

#[test]
fn test_esc_designate_g1() {
    let action = EscAction::DesignateG1('B');
    assert_eq!(action, EscAction::DesignateG1('B'));
}

#[test]
fn test_esc_designate_g2() {
    let action = EscAction::DesignateG2('B');
    assert_eq!(action, EscAction::DesignateG2('B'));
}

#[test]
fn test_esc_designate_g3() {
    let action = EscAction::DesignateG3('B');
    assert_eq!(action, EscAction::DesignateG3('B'));
}

#[test]
fn test_esc_dec_alignment_test() {
    let action = EscAction::DecAlignmentTest;
    assert_eq!(action, EscAction::DecAlignmentTest);
}

#[test]
fn test_esc_unknown() {
    let action = EscAction::Unknown(vec![0x5B]);
    assert_eq!(action, EscAction::Unknown(vec![0x5B]));
}

// ============================================================================
// EscAction clone/equality
// ============================================================================

#[test]
fn test_esc_action_clone() {
    let action = EscAction::SaveCursor;
    assert_eq!(action.clone(), EscAction::SaveCursor);
}

#[test]
fn test_esc_action_ne() {
    assert_ne!(EscAction::SaveCursor, EscAction::RestoreCursor);
}

#[test]
fn test_esc_action_designate_ne() {
    assert_ne!(EscAction::DesignateG0('B'), EscAction::DesignateG0('0'));
}

#[test]
fn test_esc_action_designate_slots_ne() {
    assert_ne!(EscAction::DesignateG0('B'), EscAction::DesignateG1('B'));
}

// ============================================================================
// CsiAction
// ============================================================================

#[test]
fn test_csi_action_creation() {
    let csi = CsiAction {
        params: Params::from_slice(&[10, 20]),
        intermediates: vec![],
        final_byte: b'H',
        private: false,
    };
    assert_eq!(csi.final_byte, b'H');
    assert!(!csi.private);
}

#[test]
fn test_csi_action_param_valid() {
    let csi = CsiAction {
        params: Params::from_slice(&[10, 20, 30]),
        intermediates: vec![],
        final_byte: b'H',
        private: false,
    };
    assert_eq!(csi.param(0, 1), 10);
    assert_eq!(csi.param(1, 1), 20);
    assert_eq!(csi.param(2, 1), 30);
}

#[test]
fn test_csi_action_param_default() {
    let csi = CsiAction {
        params: Params::new(),
        intermediates: vec![],
        final_byte: b'H',
        private: false,
    };
    assert_eq!(csi.param(0, 1), 1);
    assert_eq!(csi.param(1, 1), 1);
}

#[test]
fn test_csi_action_param_out_of_range() {
    let csi = CsiAction {
        params: Params::from_slice(&[42]),
        intermediates: vec![],
        final_byte: b'H',
        private: false,
    };
    assert_eq!(csi.param(5, 99), 99);
}

#[test]
fn test_csi_action_is_match() {
    let csi = CsiAction {
        params: Params::new(),
        intermediates: vec![],
        final_byte: b'H',
        private: false,
    };
    assert!(csi.is(b'H'));
}

#[test]
fn test_csi_action_is_no_match() {
    let csi = CsiAction {
        params: Params::new(),
        intermediates: vec![],
        final_byte: b'H',
        private: false,
    };
    assert!(!csi.is(b'J'));
}

#[test]
fn test_csi_action_is_with_intermediates() {
    let csi = CsiAction {
        params: Params::new(),
        intermediates: vec![b' '],
        final_byte: b'q',
        private: false,
    };
    // is() returns false when intermediates present
    assert!(!csi.is(b'q'));
}

#[test]
fn test_csi_action_is_private_false_for_non_private() {
    let csi = CsiAction {
        params: Params::new(),
        intermediates: vec![],
        final_byte: b'H',
        private: false,
    };
    assert!(!csi.is_private(b'H'));
}

#[test]
fn test_csi_action_is_private_true() {
    let csi = CsiAction {
        params: Params::new(),
        intermediates: vec![],
        final_byte: b'h',
        private: true,
    };
    assert!(csi.is_private(b'h'));
}

#[test]
fn test_csi_action_is_returns_false_for_private() {
    let csi = CsiAction {
        params: Params::new(),
        intermediates: vec![],
        final_byte: b'h',
        private: true,
    };
    assert!(!csi.is(b'h'));
}

#[test]
fn test_csi_action_clone() {
    let csi = CsiAction {
        params: Params::from_slice(&[1, 2]),
        intermediates: vec![b' '],
        final_byte: b'q',
        private: false,
    };
    let cloned = csi.clone();
    assert_eq!(csi, cloned);
}

// ============================================================================
// CsiAction - Common terminal sequences
// ============================================================================

#[test]
fn test_csi_cursor_up() {
    let csi = CsiAction {
        params: Params::from_slice(&[5]),
        intermediates: vec![],
        final_byte: b'A',
        private: false,
    };
    assert!(csi.is(b'A'));
    assert_eq!(csi.param(0, 1), 5);
}

#[test]
fn test_csi_cursor_down() {
    let csi = CsiAction {
        params: Params::from_slice(&[3]),
        intermediates: vec![],
        final_byte: b'B',
        private: false,
    };
    assert!(csi.is(b'B'));
}

#[test]
fn test_csi_cursor_forward() {
    let csi = CsiAction {
        params: Params::from_slice(&[10]),
        intermediates: vec![],
        final_byte: b'C',
        private: false,
    };
    assert!(csi.is(b'C'));
}

#[test]
fn test_csi_cursor_backward() {
    let csi = CsiAction {
        params: Params::from_slice(&[2]),
        intermediates: vec![],
        final_byte: b'D',
        private: false,
    };
    assert!(csi.is(b'D'));
}

#[test]
fn test_csi_cursor_position() {
    let csi = CsiAction {
        params: Params::from_slice(&[10, 20]),
        intermediates: vec![],
        final_byte: b'H',
        private: false,
    };
    assert!(csi.is(b'H'));
}

#[test]
fn test_csi_erase_display() {
    let csi = CsiAction {
        params: Params::from_slice(&[2]),
        intermediates: vec![],
        final_byte: b'J',
        private: false,
    };
    assert!(csi.is(b'J'));
}

#[test]
fn test_csi_erase_line() {
    let csi = CsiAction {
        params: Params::from_slice(&[2]),
        intermediates: vec![],
        final_byte: b'K',
        private: false,
    };
    assert!(csi.is(b'K'));
}

#[test]
fn test_csi_insert_lines() {
    let csi = CsiAction {
        params: Params::from_slice(&[3]),
        intermediates: vec![],
        final_byte: b'L',
        private: false,
    };
    assert!(csi.is(b'L'));
}

#[test]
fn test_csi_delete_lines() {
    let csi = CsiAction {
        params: Params::from_slice(&[2]),
        intermediates: vec![],
        final_byte: b'M',
        private: false,
    };
    assert!(csi.is(b'M'));
}

#[test]
fn test_csi_sgr() {
    let csi = CsiAction {
        params: Params::from_slice(&[1, 31, 42]),
        intermediates: vec![],
        final_byte: b'm',
        private: false,
    };
    assert!(csi.is(b'm'));
    assert_eq!(csi.param(0, 0), 1);
    assert_eq!(csi.param(1, 0), 31);
    assert_eq!(csi.param(2, 0), 42);
}

#[test]
fn test_csi_scroll_up() {
    let csi = CsiAction {
        params: Params::from_slice(&[5]),
        intermediates: vec![],
        final_byte: b'S',
        private: false,
    };
    assert!(csi.is(b'S'));
}

#[test]
fn test_csi_scroll_down() {
    let csi = CsiAction {
        params: Params::from_slice(&[3]),
        intermediates: vec![],
        final_byte: b'T',
        private: false,
    };
    assert!(csi.is(b'T'));
}

#[test]
fn test_csi_set_scroll_region() {
    let csi = CsiAction {
        params: Params::from_slice(&[5, 20]),
        intermediates: vec![],
        final_byte: b'r',
        private: false,
    };
    assert!(csi.is(b'r'));
}

#[test]
fn test_csi_dec_set() {
    let csi = CsiAction {
        params: Params::from_slice(&[25]),
        intermediates: vec![],
        final_byte: b'h',
        private: true,
    };
    assert!(csi.is_private(b'h'));
}

#[test]
fn test_csi_dec_reset() {
    let csi = CsiAction {
        params: Params::from_slice(&[25]),
        intermediates: vec![],
        final_byte: b'l',
        private: true,
    };
    assert!(csi.is_private(b'l'));
}

// ============================================================================
// OscAction variants
// ============================================================================

#[test]
fn test_osc_set_icon_and_title() {
    let action = OscAction::SetIconAndTitle("My Title".to_string());
    assert_eq!(action, OscAction::SetIconAndTitle("My Title".to_string()));
}

#[test]
fn test_osc_set_icon_name() {
    let action = OscAction::SetIconName("icon".to_string());
    assert_eq!(action, OscAction::SetIconName("icon".to_string()));
}

#[test]
fn test_osc_set_title() {
    let action = OscAction::SetTitle("title".to_string());
    assert_eq!(action, OscAction::SetTitle("title".to_string()));
}

#[test]
fn test_osc_set_color() {
    let action = OscAction::SetColor {
        index: 0,
        color: "#000000".to_string(),
    };
    if let OscAction::SetColor { index, color } = &action {
        assert_eq!(*index, 0);
        assert_eq!(color, "#000000");
    }
}

#[test]
fn test_osc_set_current_directory() {
    let action = OscAction::SetCurrentDirectory("/home/user".to_string());
    assert_eq!(
        action,
        OscAction::SetCurrentDirectory("/home/user".to_string())
    );
}

#[test]
fn test_osc_hyperlink() {
    let action = OscAction::Hyperlink {
        params: "".to_string(),
        uri: "https://example.com".to_string(),
    };
    if let OscAction::Hyperlink { params, uri } = &action {
        assert_eq!(params, "");
        assert_eq!(uri, "https://example.com");
    }
}

#[test]
fn test_osc_hyperlink_with_params() {
    let action = OscAction::Hyperlink {
        params: "id=link1".to_string(),
        uri: "https://example.com".to_string(),
    };
    if let OscAction::Hyperlink { params, .. } = &action {
        assert_eq!(params, "id=link1");
    }
}

#[test]
fn test_osc_set_foreground_color() {
    let action = OscAction::SetForegroundColor("rgb:ff/ff/ff".to_string());
    assert_eq!(
        action,
        OscAction::SetForegroundColor("rgb:ff/ff/ff".to_string())
    );
}

#[test]
fn test_osc_set_background_color() {
    let action = OscAction::SetBackgroundColor("rgb:00/00/00".to_string());
    assert_eq!(
        action,
        OscAction::SetBackgroundColor("rgb:00/00/00".to_string())
    );
}

#[test]
fn test_osc_set_cursor_color() {
    let action = OscAction::SetCursorColor("rgb:ff/00/00".to_string());
    assert_eq!(
        action,
        OscAction::SetCursorColor("rgb:ff/00/00".to_string())
    );
}

#[test]
fn test_osc_clipboard() {
    let action = OscAction::Clipboard {
        clipboard: "c".to_string(),
        data: "SGVsbG8=".to_string(),
    };
    if let OscAction::Clipboard { clipboard, data } = &action {
        assert_eq!(clipboard, "c");
        assert_eq!(data, "SGVsbG8=");
    }
}

#[test]
fn test_osc_reset_color() {
    let action = OscAction::ResetColor(Some(5));
    assert_eq!(action, OscAction::ResetColor(Some(5)));
}

#[test]
fn test_osc_reset_color_none() {
    let action = OscAction::ResetColor(None);
    assert_eq!(action, OscAction::ResetColor(None));
}

#[test]
fn test_osc_reset_foreground_color() {
    let action = OscAction::ResetForegroundColor;
    assert_eq!(action, OscAction::ResetForegroundColor);
}

#[test]
fn test_osc_reset_background_color() {
    let action = OscAction::ResetBackgroundColor;
    assert_eq!(action, OscAction::ResetBackgroundColor);
}

#[test]
fn test_osc_reset_cursor_color() {
    let action = OscAction::ResetCursorColor;
    assert_eq!(action, OscAction::ResetCursorColor);
}

#[test]
fn test_osc_unknown() {
    let action = OscAction::Unknown {
        command: 999,
        data: "test".to_string(),
    };
    if let OscAction::Unknown { command, data } = &action {
        assert_eq!(*command, 999);
        assert_eq!(data, "test");
    }
}

// ============================================================================
// OscAction clone/equality
// ============================================================================

#[test]
fn test_osc_action_clone() {
    let action = OscAction::SetTitle("test".to_string());
    assert_eq!(action.clone(), OscAction::SetTitle("test".to_string()));
}

#[test]
fn test_osc_action_ne_different_variants() {
    assert_ne!(
        OscAction::SetTitle("test".to_string()),
        OscAction::SetIconName("test".to_string())
    );
}

#[test]
fn test_osc_action_ne_different_values() {
    assert_ne!(
        OscAction::SetTitle("a".to_string()),
        OscAction::SetTitle("b".to_string())
    );
}

// ============================================================================
// OscAction - Empty strings
// ============================================================================

#[test]
fn test_osc_set_title_empty() {
    let action = OscAction::SetTitle(String::new());
    assert_eq!(action, OscAction::SetTitle(String::new()));
}

#[test]
fn test_osc_hyperlink_empty_uri() {
    let action = OscAction::Hyperlink {
        params: String::new(),
        uri: String::new(),
    };
    if let OscAction::Hyperlink { uri, .. } = &action {
        assert!(uri.is_empty());
    }
}

#[test]
fn test_osc_clipboard_empty() {
    let action = OscAction::Clipboard {
        clipboard: String::new(),
        data: String::new(),
    };
    if let OscAction::Clipboard { clipboard, data } = &action {
        assert!(clipboard.is_empty());
        assert!(data.is_empty());
    }
}

// ============================================================================
// OscAction - Unicode in titles
// ============================================================================

#[test]
fn test_osc_title_unicode() {
    let action = OscAction::SetTitle("日本語タイトル".to_string());
    if let OscAction::SetTitle(title) = &action {
        assert_eq!(title, "日本語タイトル");
    }
}

#[test]
fn test_osc_title_emoji() {
    let action = OscAction::SetTitle("🚀 Terminal".to_string());
    if let OscAction::SetTitle(title) = &action {
        assert_eq!(title, "🚀 Terminal");
    }
}
