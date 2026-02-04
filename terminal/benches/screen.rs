//! Screen benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use mochi_terminal::core::Screen;
use mochi_terminal::parser::{Parser, TerminalAction};

fn bench_screen_print(c: &mut Criterion) {
    let mut group = c.benchmark_group("screen");

    // Measure printing characters
    let chars: Vec<TerminalAction> = "Hello, World! "
        .chars()
        .map(TerminalAction::Print)
        .collect();

    group.bench_function("print_chars", |b| {
        b.iter(|| {
            let mut screen = Screen::new(80, 24);
            for action in &chars {
                screen.apply(action.clone());
            }
            black_box(screen)
        })
    });

    group.finish();
}

fn bench_screen_scroll(c: &mut Criterion) {
    let mut group = c.benchmark_group("screen");

    // Fill screen and scroll
    group.bench_function("scroll", |b| {
        b.iter(|| {
            let mut screen = Screen::new(80, 24);
            // Fill screen with lines
            for i in 0..100 {
                for c in format!("Line {}: Some text content here\n", i).chars() {
                    screen.apply(TerminalAction::Print(c));
                }
            }
            black_box(screen)
        })
    });

    group.finish();
}

fn bench_screen_csi(c: &mut Criterion) {
    let mut group = c.benchmark_group("screen");

    // Parse and apply CSI sequences
    let input = "\x1b[H\x1b[2J\x1b[1;31mHello\x1b[0m".repeat(100);

    group.bench_function("csi_apply", |b| {
        b.iter(|| {
            let mut screen = Screen::new(80, 24);
            let mut parser = Parser::new();
            let actions = parser.feed(input.as_bytes());
            for action in actions {
                screen.apply(action);
            }
            black_box(screen)
        })
    });

    group.finish();
}

fn bench_screen_resize(c: &mut Criterion) {
    let mut group = c.benchmark_group("screen");

    group.bench_function("resize", |b| {
        b.iter(|| {
            let mut screen = Screen::new(80, 24);
            // Fill with content
            for c in "Hello, World!\n".repeat(20).chars() {
                screen.apply(TerminalAction::Print(c));
            }
            // Resize multiple times
            screen.resize(120, 40);
            screen.resize(80, 24);
            screen.resize(132, 50);
            black_box(screen)
        })
    });

    group.finish();
}

fn bench_screen_full_redraw(c: &mut Criterion) {
    let mut group = c.benchmark_group("screen");

    // Simulate a full screen redraw (like vim opening)
    let mut setup_input = String::new();
    for row in 1..=24 {
        setup_input.push_str(&format!("\x1b[{};1H", row));
        setup_input.push_str(&"X".repeat(80));
    }

    group.throughput(Throughput::Bytes(setup_input.len() as u64));

    group.bench_function("full_redraw", |b| {
        b.iter(|| {
            let mut screen = Screen::new(80, 24);
            let mut parser = Parser::new();
            let actions = parser.feed(setup_input.as_bytes());
            for action in actions {
                screen.apply(action);
            }
            black_box(screen)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_screen_print,
    bench_screen_scroll,
    bench_screen_csi,
    bench_screen_resize,
    bench_screen_full_redraw
);

criterion_main!(benches);
