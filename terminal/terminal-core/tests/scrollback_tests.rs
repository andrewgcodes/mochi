//! Comprehensive tests for scrollback buffer

use terminal_core::{Line, Scrollback};

fn make_line(text: &str) -> Line {
    let mut line = Line::new(text.len().max(10));
    for (i, c) in text.chars().enumerate() {
        line.cell_mut(i).set_char(c);
    }
    line
}

// ============================================================================
// Scrollback Creation
// ============================================================================

#[test]
fn test_scrollback_new() {
    let sb = Scrollback::new(100);
    assert_eq!(sb.max_lines(), 100);
    assert_eq!(sb.len(), 0);
    assert!(sb.is_empty());
}

#[test]
fn test_scrollback_new_zero_max() {
    let sb = Scrollback::new(0);
    assert_eq!(sb.max_lines(), 0);
    assert!(sb.is_empty());
}

#[test]
fn test_scrollback_default() {
    let sb = Scrollback::default();
    assert_eq!(sb.max_lines(), 10000);
    assert!(sb.is_empty());
}

// ============================================================================
// Scrollback::push
// ============================================================================

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
fn test_scrollback_push_to_zero_max() {
    let mut sb = Scrollback::new(0);
    sb.push(make_line("hello"));
    assert_eq!(sb.len(), 0); // Should be discarded
}

#[test]
fn test_scrollback_push_content_preserved() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("hello"));
    assert_eq!(sb.get(0).unwrap().text(), "hello");
}

// ============================================================================
// Scrollback ring buffer behavior
// ============================================================================

#[test]
fn test_scrollback_ring_buffer_overflow() {
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
fn test_scrollback_ring_buffer_double_overflow() {
    let mut sb = Scrollback::new(2);
    for i in 0..10 {
        sb.push(make_line(&format!("line{}", i)));
    }
    assert_eq!(sb.len(), 2);
    assert_eq!(sb.get(0).unwrap().text(), "line8");
    assert_eq!(sb.get(1).unwrap().text(), "line9");
}

#[test]
fn test_scrollback_ring_buffer_exact_capacity() {
    let mut sb = Scrollback::new(5);
    for i in 0..5 {
        sb.push(make_line(&format!("line{}", i)));
    }
    assert_eq!(sb.len(), 5);
    assert_eq!(sb.get(0).unwrap().text(), "line0");
    assert_eq!(sb.get(4).unwrap().text(), "line4");
}

#[test]
fn test_scrollback_ring_one_more_than_capacity() {
    let mut sb = Scrollback::new(3);
    sb.push(make_line("A"));
    sb.push(make_line("B"));
    sb.push(make_line("C"));
    sb.push(make_line("D"));
    assert_eq!(sb.get(0).unwrap().text(), "B");
    assert_eq!(sb.get(2).unwrap().text(), "D");
}

// ============================================================================
// Scrollback::push_lines
// ============================================================================

#[test]
fn test_scrollback_push_lines() {
    let mut sb = Scrollback::new(100);
    let lines = vec![make_line("a"), make_line("b"), make_line("c")];
    sb.push_lines(lines);
    assert_eq!(sb.len(), 3);
    assert_eq!(sb.get(0).unwrap().text(), "a");
    assert_eq!(sb.get(2).unwrap().text(), "c");
}

#[test]
fn test_scrollback_push_lines_empty() {
    let mut sb = Scrollback::new(100);
    sb.push_lines(vec![]);
    assert_eq!(sb.len(), 0);
}

// ============================================================================
// Scrollback::get
// ============================================================================

#[test]
fn test_scrollback_get_valid() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("hello"));
    assert!(sb.get(0).is_some());
}

#[test]
fn test_scrollback_get_invalid() {
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

#[test]
fn test_scrollback_get_order() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("first"));
    sb.push(make_line("second"));
    sb.push(make_line("third"));
    assert_eq!(sb.get(0).unwrap().text(), "first");
    assert_eq!(sb.get(1).unwrap().text(), "second");
    assert_eq!(sb.get(2).unwrap().text(), "third");
}

// ============================================================================
// Scrollback::get_from_end
// ============================================================================

#[test]
fn test_scrollback_get_from_end_newest() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("first"));
    sb.push(make_line("last"));
    assert_eq!(sb.get_from_end(0).unwrap().text(), "last");
}

#[test]
fn test_scrollback_get_from_end_oldest() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("first"));
    sb.push(make_line("last"));
    assert_eq!(sb.get_from_end(1).unwrap().text(), "first");
}

#[test]
fn test_scrollback_get_from_end_invalid() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("hello"));
    assert!(sb.get_from_end(1).is_none());
}

#[test]
fn test_scrollback_get_from_end_after_overflow() {
    let mut sb = Scrollback::new(3);
    sb.push(make_line("a"));
    sb.push(make_line("b"));
    sb.push(make_line("c"));
    sb.push(make_line("d"));
    assert_eq!(sb.get_from_end(0).unwrap().text(), "d");
    assert_eq!(sb.get_from_end(1).unwrap().text(), "c");
    assert_eq!(sb.get_from_end(2).unwrap().text(), "b");
}

// ============================================================================
// Scrollback::clear
// ============================================================================

#[test]
fn test_scrollback_clear() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("hello"));
    sb.push(make_line("world"));
    sb.clear();
    assert!(sb.is_empty());
    assert_eq!(sb.len(), 0);
}

#[test]
fn test_scrollback_clear_already_empty() {
    let mut sb = Scrollback::new(100);
    sb.clear();
    assert!(sb.is_empty());
}

#[test]
fn test_scrollback_push_after_clear() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("before"));
    sb.clear();
    sb.push(make_line("after"));
    assert_eq!(sb.len(), 1);
    assert_eq!(sb.get(0).unwrap().text(), "after");
}

// ============================================================================
// Scrollback::resize
// ============================================================================

#[test]
fn test_scrollback_resize_smaller() {
    let mut sb = Scrollback::new(100);
    for i in 0..10 {
        sb.push(make_line(&format!("line{}", i)));
    }
    sb.resize(5);
    assert_eq!(sb.len(), 5);
    assert_eq!(sb.max_lines(), 5);
    // Keeps newest lines
    assert_eq!(sb.get(0).unwrap().text(), "line5");
}

#[test]
fn test_scrollback_resize_larger() {
    let mut sb = Scrollback::new(5);
    sb.push(make_line("hello"));
    sb.resize(100);
    assert_eq!(sb.max_lines(), 100);
    assert_eq!(sb.len(), 1);
    assert_eq!(sb.get(0).unwrap().text(), "hello");
}

#[test]
fn test_scrollback_resize_same() {
    let mut sb = Scrollback::new(10);
    sb.push(make_line("hello"));
    sb.resize(10);
    assert_eq!(sb.max_lines(), 10);
    assert_eq!(sb.len(), 1);
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
fn test_scrollback_resize_after_overflow() {
    let mut sb = Scrollback::new(3);
    sb.push(make_line("a"));
    sb.push(make_line("b"));
    sb.push(make_line("c"));
    sb.push(make_line("d"));
    // Ring buffer: b, c, d
    sb.resize(2);
    assert_eq!(sb.len(), 2);
    // Keeps newest
    assert_eq!(sb.get(0).unwrap().text(), "c");
    assert_eq!(sb.get(1).unwrap().text(), "d");
}

// ============================================================================
// Scrollback iteration
// ============================================================================

#[test]
fn test_scrollback_iter_empty() {
    let sb = Scrollback::new(100);
    assert_eq!(sb.iter().count(), 0);
}

#[test]
fn test_scrollback_iter_order() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("first"));
    sb.push(make_line("second"));
    sb.push(make_line("third"));
    let texts: Vec<_> = sb.iter().map(|l| l.text()).collect();
    assert_eq!(texts, vec!["first", "second", "third"]);
}

#[test]
fn test_scrollback_iter_after_overflow() {
    let mut sb = Scrollback::new(3);
    sb.push(make_line("a"));
    sb.push(make_line("b"));
    sb.push(make_line("c"));
    sb.push(make_line("d"));
    let texts: Vec<_> = sb.iter().map(|l| l.text()).collect();
    assert_eq!(texts, vec!["b", "c", "d"]);
}

#[test]
fn test_scrollback_iter_rev_order() {
    let mut sb = Scrollback::new(100);
    sb.push(make_line("first"));
    sb.push(make_line("second"));
    sb.push(make_line("third"));
    let texts: Vec<_> = sb.iter_rev().map(|l| l.text()).collect();
    assert_eq!(texts, vec!["third", "second", "first"]);
}

#[test]
fn test_scrollback_iter_rev_after_overflow() {
    let mut sb = Scrollback::new(3);
    sb.push(make_line("a"));
    sb.push(make_line("b"));
    sb.push(make_line("c"));
    sb.push(make_line("d"));
    let texts: Vec<_> = sb.iter_rev().map(|l| l.text()).collect();
    assert_eq!(texts, vec!["d", "c", "b"]);
}

#[test]
fn test_scrollback_iter_rev_empty() {
    let sb = Scrollback::new(100);
    assert_eq!(sb.iter_rev().count(), 0);
}

// ============================================================================
// Scrollback capacity=1
// ============================================================================

#[test]
fn test_scrollback_capacity_one() {
    let mut sb = Scrollback::new(1);
    sb.push(make_line("first"));
    assert_eq!(sb.len(), 1);
    assert_eq!(sb.get(0).unwrap().text(), "first");

    sb.push(make_line("second"));
    assert_eq!(sb.len(), 1);
    assert_eq!(sb.get(0).unwrap().text(), "second");
}

// ============================================================================
// Scrollback stress
// ============================================================================

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
