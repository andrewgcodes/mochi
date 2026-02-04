//! Parser benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use mochi_terminal::parser::Parser;

fn bench_parse_plain_text(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser");

    // Plain ASCII text
    let plain_text = "Hello, World! ".repeat(1000);
    group.throughput(Throughput::Bytes(plain_text.len() as u64));

    group.bench_function("plain_text", |b| {
        b.iter(|| {
            let mut parser = Parser::new();
            let actions = parser.feed(black_box(plain_text.as_bytes()));
            black_box(actions)
        })
    });

    group.finish();
}

fn bench_parse_csi_sequences(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser");

    // CSI sequences (cursor movement, SGR)
    let csi_heavy = "\x1b[1;31mRed\x1b[0m \x1b[5;10H\x1b[2J".repeat(100);
    group.throughput(Throughput::Bytes(csi_heavy.len() as u64));

    group.bench_function("csi_sequences", |b| {
        b.iter(|| {
            let mut parser = Parser::new();
            let actions = parser.feed(black_box(csi_heavy.as_bytes()));
            black_box(actions)
        })
    });

    group.finish();
}

fn bench_parse_mixed(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser");

    // Mixed content (typical terminal output)
    let mixed = "Line 1: \x1b[32mOK\x1b[0m\r\nLine 2: \x1b[31mERROR\x1b[0m\r\n".repeat(500);
    group.throughput(Throughput::Bytes(mixed.len() as u64));

    group.bench_function("mixed_content", |b| {
        b.iter(|| {
            let mut parser = Parser::new();
            let actions = parser.feed(black_box(mixed.as_bytes()));
            black_box(actions)
        })
    });

    group.finish();
}

fn bench_parse_utf8(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser");

    // UTF-8 content
    let utf8 = "Hello, 世界! 🎉 ".repeat(500);
    group.throughput(Throughput::Bytes(utf8.len() as u64));

    group.bench_function("utf8_content", |b| {
        b.iter(|| {
            let mut parser = Parser::new();
            let actions = parser.feed(black_box(utf8.as_bytes()));
            black_box(actions)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_plain_text,
    bench_parse_csi_sequences,
    bench_parse_mixed,
    bench_parse_utf8
);

criterion_main!(benches);
