//! Comprehensive tests for terminal color handling

use terminal_core::Color;

// ============================================================
// Color Construction Tests
// ============================================================

#[test]
fn test_color_default() {
    assert_eq!(Color::default(), Color::Default);
}

#[test]
fn test_color_indexed() {
    let color = Color::indexed(1);
    assert_eq!(color, Color::Indexed(1));
}

#[test]
fn test_color_indexed_zero() {
    let color = Color::indexed(0);
    assert_eq!(color, Color::Indexed(0));
}

#[test]
fn test_color_indexed_max() {
    let color = Color::indexed(255);
    assert_eq!(color, Color::Indexed(255));
}

#[test]
fn test_color_rgb() {
    let color = Color::rgb(255, 128, 64);
    assert_eq!(
        color,
        Color::Rgb {
            r: 255,
            g: 128,
            b: 64
        }
    );
}

#[test]
fn test_color_rgb_black() {
    let color = Color::rgb(0, 0, 0);
    assert_eq!(color, Color::Rgb { r: 0, g: 0, b: 0 });
}

#[test]
fn test_color_rgb_white() {
    let color = Color::rgb(255, 255, 255);
    assert_eq!(
        color,
        Color::Rgb {
            r: 255,
            g: 255,
            b: 255
        }
    );
}

#[test]
fn test_color_rgb_red() {
    let color = Color::rgb(255, 0, 0);
    assert_eq!(color, Color::Rgb { r: 255, g: 0, b: 0 });
}

#[test]
fn test_color_rgb_green() {
    let color = Color::rgb(0, 255, 0);
    assert_eq!(color, Color::Rgb { r: 0, g: 255, b: 0 });
}

#[test]
fn test_color_rgb_blue() {
    let color = Color::rgb(0, 0, 255);
    assert_eq!(color, Color::Rgb { r: 0, g: 0, b: 255 });
}

// ============================================================
// Color Constants Tests
// ============================================================

#[test]
fn test_color_constant_black() {
    assert_eq!(Color::BLACK, 0);
}

#[test]
fn test_color_constant_red() {
    assert_eq!(Color::RED, 1);
}

#[test]
fn test_color_constant_green() {
    assert_eq!(Color::GREEN, 2);
}

#[test]
fn test_color_constant_yellow() {
    assert_eq!(Color::YELLOW, 3);
}

#[test]
fn test_color_constant_blue() {
    assert_eq!(Color::BLUE, 4);
}

#[test]
fn test_color_constant_magenta() {
    assert_eq!(Color::MAGENTA, 5);
}

#[test]
fn test_color_constant_cyan() {
    assert_eq!(Color::CYAN, 6);
}

#[test]
fn test_color_constant_white() {
    assert_eq!(Color::WHITE, 7);
}

#[test]
fn test_color_constant_bright_black() {
    assert_eq!(Color::BRIGHT_BLACK, 8);
}

#[test]
fn test_color_constant_bright_red() {
    assert_eq!(Color::BRIGHT_RED, 9);
}

#[test]
fn test_color_constant_bright_green() {
    assert_eq!(Color::BRIGHT_GREEN, 10);
}

#[test]
fn test_color_constant_bright_yellow() {
    assert_eq!(Color::BRIGHT_YELLOW, 11);
}

#[test]
fn test_color_constant_bright_blue() {
    assert_eq!(Color::BRIGHT_BLUE, 12);
}

#[test]
fn test_color_constant_bright_magenta() {
    assert_eq!(Color::BRIGHT_MAGENTA, 13);
}

#[test]
fn test_color_constant_bright_cyan() {
    assert_eq!(Color::BRIGHT_CYAN, 14);
}

#[test]
fn test_color_constant_bright_white() {
    assert_eq!(Color::BRIGHT_WHITE, 15);
}

// ============================================================
// Color to_rgb Tests (Standard colors)
// ============================================================

#[test]
fn test_standard_color_black_to_rgb() {
    assert_eq!(Color::Indexed(0).to_rgb(), (0, 0, 0));
}

#[test]
fn test_standard_color_red_to_rgb() {
    assert_eq!(Color::Indexed(1).to_rgb(), (205, 0, 0));
}

#[test]
fn test_standard_color_green_to_rgb() {
    assert_eq!(Color::Indexed(2).to_rgb(), (0, 205, 0));
}

#[test]
fn test_standard_color_yellow_to_rgb() {
    assert_eq!(Color::Indexed(3).to_rgb(), (205, 205, 0));
}

#[test]
fn test_standard_color_blue_to_rgb() {
    assert_eq!(Color::Indexed(4).to_rgb(), (0, 0, 238));
}

#[test]
fn test_standard_color_magenta_to_rgb() {
    assert_eq!(Color::Indexed(5).to_rgb(), (205, 0, 205));
}

#[test]
fn test_standard_color_cyan_to_rgb() {
    assert_eq!(Color::Indexed(6).to_rgb(), (0, 205, 205));
}

#[test]
fn test_standard_color_white_to_rgb() {
    assert_eq!(Color::Indexed(7).to_rgb(), (229, 229, 229));
}

// ============================================================
// Color to_rgb Tests (Bright colors)
// ============================================================

#[test]
fn test_bright_color_black_to_rgb() {
    assert_eq!(Color::Indexed(8).to_rgb(), (127, 127, 127));
}

#[test]
fn test_bright_color_red_to_rgb() {
    assert_eq!(Color::Indexed(9).to_rgb(), (255, 0, 0));
}

#[test]
fn test_bright_color_green_to_rgb() {
    assert_eq!(Color::Indexed(10).to_rgb(), (0, 255, 0));
}

#[test]
fn test_bright_color_yellow_to_rgb() {
    assert_eq!(Color::Indexed(11).to_rgb(), (255, 255, 0));
}

#[test]
fn test_bright_color_blue_to_rgb() {
    assert_eq!(Color::Indexed(12).to_rgb(), (92, 92, 255));
}

#[test]
fn test_bright_color_magenta_to_rgb() {
    assert_eq!(Color::Indexed(13).to_rgb(), (255, 0, 255));
}

#[test]
fn test_bright_color_cyan_to_rgb() {
    assert_eq!(Color::Indexed(14).to_rgb(), (0, 255, 255));
}

#[test]
fn test_bright_color_white_to_rgb() {
    assert_eq!(Color::Indexed(15).to_rgb(), (255, 255, 255));
}

// ============================================================
// Color Cube Tests (16-231)
// ============================================================

#[test]
fn test_color_cube_first_black() {
    assert_eq!(Color::Indexed(16).to_rgb(), (0, 0, 0));
}

#[test]
fn test_color_cube_pure_red() {
    assert_eq!(Color::Indexed(196).to_rgb(), (255, 0, 0));
}

#[test]
fn test_color_cube_pure_green() {
    assert_eq!(Color::Indexed(46).to_rgb(), (0, 255, 0));
}

#[test]
fn test_color_cube_pure_blue() {
    assert_eq!(Color::Indexed(21).to_rgb(), (0, 0, 255));
}

#[test]
fn test_color_cube_last_white() {
    assert_eq!(Color::Indexed(231).to_rgb(), (255, 255, 255));
}

#[test]
fn test_color_cube_index_17() {
    // Index 17: R=0, G=0, B=1 -> (0, 0, 95)
    assert_eq!(Color::Indexed(17).to_rgb(), (0, 0, 95));
}

#[test]
fn test_color_cube_index_52() {
    // Index 52: R=1, G=0, B=0 -> (95, 0, 0)
    assert_eq!(Color::Indexed(52).to_rgb(), (95, 0, 0));
}

// ============================================================
// Grayscale Tests (232-255)
// ============================================================

#[test]
fn test_grayscale_darkest() {
    assert_eq!(Color::Indexed(232).to_rgb(), (8, 8, 8));
}

#[test]
fn test_grayscale_lightest() {
    assert_eq!(Color::Indexed(255).to_rgb(), (238, 238, 238));
}

#[test]
fn test_grayscale_middle() {
    // Index 244: 8 + (244-232)*10 = 8 + 120 = 128
    assert_eq!(Color::Indexed(244).to_rgb(), (128, 128, 128));
}

#[test]
fn test_grayscale_all_equal_components() {
    for i in 232u8..=255 {
        let (r, g, b) = Color::Indexed(i).to_rgb();
        assert_eq!(r, g);
        assert_eq!(g, b);
    }
}

// ============================================================
// Color Default to_rgb Tests
// ============================================================

#[test]
fn test_color_default_to_rgb() {
    assert_eq!(Color::Default.to_rgb(), (255, 255, 255));
}

#[test]
fn test_color_rgb_passthrough() {
    let color = Color::rgb(42, 128, 200);
    assert_eq!(color.to_rgb(), (42, 128, 200));
}

#[test]
fn test_color_rgb_boundary_values() {
    assert_eq!(Color::rgb(0, 0, 0).to_rgb(), (0, 0, 0));
    assert_eq!(Color::rgb(255, 255, 255).to_rgb(), (255, 255, 255));
}

// ============================================================
// Color Equality Tests
// ============================================================

#[test]
fn test_color_equality_default() {
    assert_eq!(Color::Default, Color::Default);
}

#[test]
fn test_color_equality_indexed() {
    assert_eq!(Color::Indexed(5), Color::Indexed(5));
}

#[test]
fn test_color_inequality_indexed() {
    assert_ne!(Color::Indexed(5), Color::Indexed(6));
}

#[test]
fn test_color_equality_rgb() {
    assert_eq!(Color::rgb(10, 20, 30), Color::rgb(10, 20, 30));
}

#[test]
fn test_color_inequality_rgb() {
    assert_ne!(Color::rgb(10, 20, 30), Color::rgb(10, 20, 31));
}

#[test]
fn test_color_inequality_types() {
    assert_ne!(Color::Default, Color::Indexed(0));
    assert_ne!(Color::Indexed(0), Color::rgb(0, 0, 0));
}

// ============================================================
// Color Clone/Copy Tests
// ============================================================

#[test]
fn test_color_clone() {
    let color = Color::rgb(100, 200, 50);
    let clone = color;
    assert_eq!(color, clone);
}

#[test]
fn test_color_copy() {
    let color = Color::Indexed(42);
    let copy = color;
    assert_eq!(color, copy);
}

// ============================================================
// All 256 indexed colors to_rgb roundtrip
// ============================================================

#[test]
fn test_all_256_indexed_colors_produce_valid_rgb() {
    for i in 0u8..=255 {
        let (r, g, b) = Color::Indexed(i).to_rgb();
        // All values should be valid u8 (0-255)
        // Verify we get valid RGB tuple (u8 values are always 0-255)
        let _ = (r, g, b);
    }
}
