//! Comprehensive tests for terminal color representation

use terminal_core::Color;

// ============================================================================
// Color::Default
// ============================================================================

#[test]
fn test_color_default_variant() {
    let c = Color::Default;
    assert_eq!(c, Color::Default);
}

#[test]
fn test_color_default_to_rgb() {
    assert_eq!(Color::Default.to_rgb(), (255, 255, 255));
}

#[test]
fn test_color_default_is_default_trait() {
    assert_eq!(Color::default(), Color::Default);
}

// ============================================================================
// Color::Indexed - Standard ANSI (0-7)
// ============================================================================

#[test]
fn test_color_indexed_black() {
    assert_eq!(Color::Indexed(Color::BLACK).to_rgb(), (0, 0, 0));
}

#[test]
fn test_color_indexed_red() {
    assert_eq!(Color::Indexed(Color::RED).to_rgb(), (205, 0, 0));
}

#[test]
fn test_color_indexed_green() {
    assert_eq!(Color::Indexed(Color::GREEN).to_rgb(), (0, 205, 0));
}

#[test]
fn test_color_indexed_yellow() {
    assert_eq!(Color::Indexed(Color::YELLOW).to_rgb(), (205, 205, 0));
}

#[test]
fn test_color_indexed_blue() {
    assert_eq!(Color::Indexed(Color::BLUE).to_rgb(), (0, 0, 238));
}

#[test]
fn test_color_indexed_magenta() {
    assert_eq!(Color::Indexed(Color::MAGENTA).to_rgb(), (205, 0, 205));
}

#[test]
fn test_color_indexed_cyan() {
    assert_eq!(Color::Indexed(Color::CYAN).to_rgb(), (0, 205, 205));
}

#[test]
fn test_color_indexed_white() {
    assert_eq!(Color::Indexed(Color::WHITE).to_rgb(), (229, 229, 229));
}

// ============================================================================
// Color::Indexed - Bright ANSI (8-15)
// ============================================================================

#[test]
fn test_color_bright_black() {
    assert_eq!(Color::Indexed(Color::BRIGHT_BLACK).to_rgb(), (127, 127, 127));
}

#[test]
fn test_color_bright_red() {
    assert_eq!(Color::Indexed(Color::BRIGHT_RED).to_rgb(), (255, 0, 0));
}

#[test]
fn test_color_bright_green() {
    assert_eq!(Color::Indexed(Color::BRIGHT_GREEN).to_rgb(), (0, 255, 0));
}

#[test]
fn test_color_bright_yellow() {
    assert_eq!(Color::Indexed(Color::BRIGHT_YELLOW).to_rgb(), (255, 255, 0));
}

#[test]
fn test_color_bright_blue() {
    assert_eq!(Color::Indexed(Color::BRIGHT_BLUE).to_rgb(), (92, 92, 255));
}

#[test]
fn test_color_bright_magenta() {
    assert_eq!(Color::Indexed(Color::BRIGHT_MAGENTA).to_rgb(), (255, 0, 255));
}

#[test]
fn test_color_bright_cyan() {
    assert_eq!(Color::Indexed(Color::BRIGHT_CYAN).to_rgb(), (0, 255, 255));
}

#[test]
fn test_color_bright_white() {
    assert_eq!(Color::Indexed(Color::BRIGHT_WHITE).to_rgb(), (255, 255, 255));
}

// ============================================================================
// Color::Indexed - 6x6x6 Color Cube (16-231)
// ============================================================================

#[test]
fn test_color_cube_origin() {
    // Index 16 = (0,0,0) in cube = black
    assert_eq!(Color::Indexed(16).to_rgb(), (0, 0, 0));
}

#[test]
fn test_color_cube_pure_red() {
    // Index 196 = r=5, g=0, b=0
    assert_eq!(Color::Indexed(196).to_rgb(), (255, 0, 0));
}

#[test]
fn test_color_cube_pure_green() {
    // Index 46 = r=0, g=5, b=0
    assert_eq!(Color::Indexed(46).to_rgb(), (0, 255, 0));
}

#[test]
fn test_color_cube_pure_blue() {
    // Index 21 = r=0, g=0, b=5
    assert_eq!(Color::Indexed(21).to_rgb(), (0, 0, 255));
}

#[test]
fn test_color_cube_white() {
    // Index 231 = r=5, g=5, b=5
    assert_eq!(Color::Indexed(231).to_rgb(), (255, 255, 255));
}

#[test]
fn test_color_cube_mid_values() {
    // Index 16 + 36*2 + 6*2 + 2 = 16+72+12+2 = 102
    // r=2, g=2, b=2
    let (r, g, b) = Color::Indexed(102).to_rgb();
    assert_eq!(r, 135); // 55 + 2*40
    assert_eq!(g, 135);
    assert_eq!(b, 135);
}

#[test]
fn test_color_cube_step_values() {
    // r=1,g=0,b=0: index=16+36=52
    let (r, _, _) = Color::Indexed(52).to_rgb();
    assert_eq!(r, 95); // 55 + 1*40
}

// ============================================================================
// Color::Indexed - Grayscale (232-255)
// ============================================================================

#[test]
fn test_color_grayscale_darkest() {
    assert_eq!(Color::Indexed(232).to_rgb(), (8, 8, 8));
}

#[test]
fn test_color_grayscale_lightest() {
    assert_eq!(Color::Indexed(255).to_rgb(), (238, 238, 238));
}

#[test]
fn test_color_grayscale_mid() {
    // Index 244 = 8 + (244-232)*10 = 8 + 120 = 128
    assert_eq!(Color::Indexed(244).to_rgb(), (128, 128, 128));
}

#[test]
fn test_color_grayscale_all_components_equal() {
    for i in 232..=255 {
        let (r, g, b) = Color::Indexed(i).to_rgb();
        assert_eq!(r, g);
        assert_eq!(g, b);
    }
}

// ============================================================================
// Color::Rgb
// ============================================================================

#[test]
fn test_color_rgb_black() {
    assert_eq!(Color::rgb(0, 0, 0).to_rgb(), (0, 0, 0));
}

#[test]
fn test_color_rgb_white() {
    assert_eq!(Color::rgb(255, 255, 255).to_rgb(), (255, 255, 255));
}

#[test]
fn test_color_rgb_red() {
    assert_eq!(Color::rgb(255, 0, 0).to_rgb(), (255, 0, 0));
}

#[test]
fn test_color_rgb_arbitrary() {
    assert_eq!(Color::rgb(42, 128, 200).to_rgb(), (42, 128, 200));
}

#[test]
fn test_color_rgb_passthrough() {
    // RGB colors should pass through unchanged
    for r in [0, 1, 127, 128, 254, 255] {
        for g in [0, 1, 127, 128, 254, 255] {
            for b in [0, 1, 127, 128, 254, 255] {
                assert_eq!(Color::rgb(r, g, b).to_rgb(), (r, g, b));
            }
        }
    }
}

// ============================================================================
// Color construction helpers
// ============================================================================

#[test]
fn test_color_indexed_helper() {
    assert_eq!(Color::indexed(5), Color::Indexed(5));
}

#[test]
fn test_color_rgb_helper() {
    assert_eq!(Color::rgb(10, 20, 30), Color::Rgb { r: 10, g: 20, b: 30 });
}

// ============================================================================
// Color equality
// ============================================================================

#[test]
fn test_color_equality_indexed() {
    assert_eq!(Color::Indexed(5), Color::Indexed(5));
    assert_ne!(Color::Indexed(5), Color::Indexed(6));
}

#[test]
fn test_color_equality_rgb() {
    assert_eq!(Color::rgb(1, 2, 3), Color::rgb(1, 2, 3));
    assert_ne!(Color::rgb(1, 2, 3), Color::rgb(1, 2, 4));
}

#[test]
fn test_color_equality_different_variants() {
    assert_ne!(Color::Default, Color::Indexed(0));
    assert_ne!(Color::Indexed(0), Color::rgb(0, 0, 0));
    assert_ne!(Color::Default, Color::rgb(255, 255, 255));
}

// ============================================================================
// Color clone/copy
// ============================================================================

#[test]
fn test_color_clone() {
    let c = Color::rgb(10, 20, 30);
    let c2 = c;
    assert_eq!(c, c2);
}

#[test]
fn test_color_constants_values() {
    assert_eq!(Color::BLACK, 0);
    assert_eq!(Color::RED, 1);
    assert_eq!(Color::GREEN, 2);
    assert_eq!(Color::YELLOW, 3);
    assert_eq!(Color::BLUE, 4);
    assert_eq!(Color::MAGENTA, 5);
    assert_eq!(Color::CYAN, 6);
    assert_eq!(Color::WHITE, 7);
    assert_eq!(Color::BRIGHT_BLACK, 8);
    assert_eq!(Color::BRIGHT_RED, 9);
    assert_eq!(Color::BRIGHT_GREEN, 10);
    assert_eq!(Color::BRIGHT_YELLOW, 11);
    assert_eq!(Color::BRIGHT_BLUE, 12);
    assert_eq!(Color::BRIGHT_MAGENTA, 13);
    assert_eq!(Color::BRIGHT_CYAN, 14);
    assert_eq!(Color::BRIGHT_WHITE, 15);
}
