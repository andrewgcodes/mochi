//! Comprehensive tests for scrollback buffer

use terminal_core::{Line, Scrollback};

fn make_line(text: &str) -> Line {
    let mut line = Line::new(text.len().max(10));
    for (i, c) in text.chars().enumerate() {
        line.cell_mut(i).set_char(c);
    }
    line
}

// ============================================================
// Creation Tests
// ============================================================

#[test]
fn test_scrollback_new() {
    let sb = Scrollback::new(100);
    assert_eq!(sb.max_lines(), 100);
    assert_eq!(sb.len(), 0);
    assert!(sb.is_empty());
}

#[test]
fn test_scrollback_default() {
    let sb = Scrollback::default();
    assert_eq!(sb.max_lines(), 10000);
    assert!(sb.is_empty());
}

#[test]
fn test_scrollback_zero_capacity() {
    let sb = Scrollback::new(0);
    assert_eq!(sb.max_lines(), 0);
    assert!(sb.is_empty());
}

#[test]
fn test_scrollback_one_capacity() {
    let sb = Scrollback::new(1);
    assert_eq!(sb.max_lines(), 1);
}

// ============================================================
// Push Tests
// ============================================================

#[test]
fn test_scrollback_push_one() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("hello"));
    assert_eq!(sb.len(), 1);
    assert!(!sb.is_empty());
}

#[test]
fn test_scrollback_push_multiple() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("line1"));
    sb.push(make_line("line2"));
    sb.push(make_line("line3"));
    assert_eq!(sb.len(), 3);
}

#[test]
fn test_scrollback_push_to_zero_capacity() {
    let mut sb = Scrollback::new(0);
    sb.push(make_line("hello"));
    assert_eq!(sb.len(), 0);
    assert!(sb.is_empty());
}

#[test]
fn test_scrollback_push_lines() {
    let mut sb = Scrollback::new(100);
    let lines = vec![make_line("a"), make_line("b"), make_line("c")];
    sb.push_lines(lines);
    assert_eq!(sb.len(), 3);
}

// ============================================================
// Ring Buffer Tests
// ============================================================

#[test]
fn test_scrollback_ring_buffer_overwrites_oldest() {
    let mut sb = Scrollback::new(3);
    sb.push(make_line("line1"));
    sb.push(make_line("line2"));
    sb.push(make_line("line3"));
    sb.push(make_line("line4")); // Overwrites line1
    assert_eq!(sb.len(), 3);
    assert_eq!(sb.get(0).unwrap().text(), "line2");
    assert_eq!(sb.get(1).unwrap().text(), "line3");
    assert_eq!(sb.get(2).unwrap().text(), "line4");
}

#[test]
fn test_scrollback_ring_buffer_multiple_overwrites() {
    let mut sb = Scrollback::new(3);
    for i in 0..10 {
        sb.push(make_line(&format!("line{}", i)));
    }
    assert_eq!(sb.len(), 3);
    assert_eq!(sb.get(0).unwrap().text(), "line7");
    assert_eq!(sb.get(1).unwrap().text(), "line8");
    assert_eq!(sb.get(2).unwrap().text(), "line9");
}

#[test]
fn test_scrollback_ring_buffer_single_capacity() {
    let mut sb = Scrollback::new(1);
    sb.push(make_line("line1"));
    sb.push(make_line("line2"));
    assert_eq!(sb.len(), 1);
    assert_eq!(sb.get(0).unwrap().text(), "line2");
}

// ============================================================
// Get Tests
// ============================================================

#[test]
fn test_scrollback_get_first() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("hello"));
    assert_eq!(sb.get(0).unwrap().text(), "hello");
}

#[test]
fn test_scrollback_get_last() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("a"));
    sb.push(make_line("b"));
    sb.push(make_line("c"));
    assert_eq!(sb.get(2).unwrap().text(), "c");
}

#[test]
fn test_scrollback_get_out_of_bounds() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("hello"));
    assert!(sb.get(1).is_none());
    assert!(sb.get(100).is_none());
}

#[test]
fn test_scrollback_get_empty() {
    let sb = Scrollback::new(100);
    assert!(sb.get(0).is_none());
}

// ============================================================
// Get From End Tests
// ============================================================

#[test]
fn test_scrollback_get_from_end_newest() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("a"));
    sb.push(make_line("b"));
    sb.push(make_line("c"));
    assert_eq!(sb.get_from_end(0).unwrap().text(), "c");
}

#[test]
fn test_scrollback_get_from_end_oldest() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("a"));
    sb.push(make_line("b"));
    sb.push(make_line("c"));
    assert_eq!(sb.get_from_end(2).unwrap().text(), "a");
}

#[test]
fn test_scrollback_get_from_end_out_of_bounds() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("hello"));
    assert!(sb.get_from_end(1).is_none());
}

#[test]
fn test_scrollback_get_from_end_with_ring() {
    let mut sb = Scrollback::new(3);
    for i in 0..10 {
        sb.push(make_line(&format!("line{}", i)));
    }
    assert_eq!(sb.get_from_end(0).unwrap().text(), "line9");
    assert_eq!(sb.get_from_end(1).unwrap().text(), "line8");
    assert_eq!(sb.get_from_end(2).unwrap().text(), "line7");
}

// ============================================================
// Clear Tests
// ============================================================

#[test]
fn test_scrollback_clear() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("a"));
    sb.push(make_line("b"));
    sb.clear();
    assert!(sb.is_empty());
    assert_eq!(sb.len(), 0);
}

#[test]
fn test_scrollback_clear_then_push() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("a"));
    sb.clear();
    sb.push(make_line("b"));
    assert_eq!(sb.len(), 1);
    assert_eq!(sb.get(0).unwrap().text(), "b");
}

// ============================================================
// Resize Tests
// ============================================================

#[test]
fn test_scrollback_resize_smaller() {
    let mut sb = Scrollback::new(100);
    for i in 0..10 {
        sb.push(make_line(&format!("line{}", i)));
    }
    sb.resize(5);
    assert_eq!(sb.len(), 5);
    assert_eq!(sb.max_lines(), 5);
    assert_eq!(sb.get(0).unwrap().text(), "line5");
}

#[test]
fn test_scrollback_resize_larger() {
    let mut sb = Scrollback::new(5);
    for i in 0..5 {
        sb.push(make_line(&format!("line{}", i)));
    }
    sb.resize(10);
    assert_eq!(sb.max_lines(), 10);
    assert_eq!(sb.len(), 5);
    assert_eq!(sb.get(0).unwrap().text(), "line0");
}

#[test]
fn test_scrollback_resize_to_zero() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("hello"));
    sb.resize(0);
    assert_eq!(sb.max_lines(), 0);
    assert!(sb.is_empty());
}

#[test]
fn test_scrollback_resize_same_size() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("hello"));
    sb.resize(100);
    assert_eq!(sb.max_lines(), 100);
    assert_eq!(sb.len(), 1);
}

#[test]
fn test_scrollback_resize_with_ring() {
    let mut sb = Scrollback::new(3);
    for i in 0..10 {
        sb.push(make_line(&format!("line{}", i)));
    }
    // Ring is: line7, line8, line9 with start != 0
    sb.resize(5);
    assert_eq!(sb.max_lines(), 5);
    assert_eq!(sb.len(), 3);
    assert_eq!(sb.get(0).unwrap().text(), "line7");
    assert_eq!(sb.get(2).unwrap().text(), "line9");
}

// ============================================================
// Iterator Tests
// ============================================================

#[test]
fn test_scrollback_iter() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("a"));
    sb.push(make_line("b"));
    sb.push(make_line("c"));
    let texts: Vec<_> = sb.iter().map(|l| l.text()).collect();
    assert_eq!(texts, vec!["a", "b", "c"]);
}

#[test]
fn test_scrollback_iter_empty() {
    let sb = Scrollback::new(100);
    let texts: Vec<_> = sb.iter().collect::<Vec<_>>();
    assert!(texts.is_empty());
}

#[test]
fn test_scrollback_iter_with_ring() {
    let mut sb = Scrollback::new(3);
    for i in 0..5 {
        sb.push(make_line(&format!("line{}", i)));
    }
    let texts: Vec<_> = sb.iter().map(|l| l.text()).collect();
    assert_eq!(texts, vec!["line2", "line3", "line4"]);
}

#[test]
fn test_scrollback_iter_rev() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("a"));
    sb.push(make_line("b"));
    sb.push(make_line("c"));
    let texts: Vec<_> = sb.iter_rev().map(|l| l.text()).collect();
    assert_eq!(texts, vec!["c", "b", "a"]);
}

#[test]
fn test_scrollback_iter_rev_empty() {
    let sb = Scrollback::new(100);
    let texts: Vec<_> = sb.iter_rev().collect::<Vec<_>>();
    assert!(texts.is_empty());
}

#[test]
fn test_scrollback_iter_rev_with_ring() {
    let mut sb = Scrollback::new(3);
    for i in 0..5 {
        sb.push(make_line(&format!("line{}", i)));
    }
    let texts: Vec<_> = sb.iter_rev().map(|l| l.text()).collect();
    assert_eq!(texts, vec!["line4", "line3", "line2"]);
}

// ============================================================
// Stress Tests
// ============================================================

#[test]
fn test_scrollback_many_pushes() {
    let mut sb = Scrollback::new(100);
    for i in 0..1000 {
        sb.push(make_line(&format!("line{}", i)));
    }
    assert_eq!(sb.len(), 100);
    assert_eq!(sb.get(0).unwrap().text(), "line900");
    assert_eq!(sb.get(99).unwrap().text(), "line999");
}
