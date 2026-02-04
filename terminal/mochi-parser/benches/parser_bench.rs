use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use mochi_parser::Parser;

fn generate_plain_text(size: usize) -> Vec<u8> {
    let text = "Hello, World! This is a test of plain text parsing. ";
    text.as_bytes().iter().cycle().take(size).copied().collect()
}

fn generate_colored_text(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    let colors = [
        "\x1b[31m", "\x1b[32m", "\x1b[33m", "\x1b[34m", "\x1b[35m", "\x1b[36m", "\x1b[0m",
    ];
    let text = "Colored text ";

    let mut i = 0;
    while data.len() < size {
        data.extend_from_slice(colors[i % colors.len()].as_bytes());
        data.extend_from_slice(text.as_bytes());
        i += 1;
    }
    data.truncate(size);
    data
}

fn generate_cursor_movement(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    let sequences = [
        "\x1b[A",      // cursor up
        "\x1b[B",      // cursor down
        "\x1b[C",      // cursor forward
        "\x1b[D",      // cursor back
        "\x1b[10;20H", // cursor position
        "\x1b[2J",     // clear screen
        "\x1b[K",      // clear line
    ];

    let mut i = 0;
    while data.len() < size {
        data.extend_from_slice(sequences[i % sequences.len()].as_bytes());
        i += 1;
    }
    data.truncate(size);
    data
}

fn generate_sgr_sequences(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    let sequences = [
        "\x1b[0m",              // reset
        "\x1b[1m",              // bold
        "\x1b[4m",              // underline
        "\x1b[38;5;196m",       // 256-color fg
        "\x1b[48;5;21m",        // 256-color bg
        "\x1b[38;2;255;128;0m", // truecolor fg
        "\x1b[48;2;0;128;255m", // truecolor bg
    ];

    let mut i = 0;
    while data.len() < size {
        data.extend_from_slice(sequences[i % sequences.len()].as_bytes());
        data.extend_from_slice(b"X"); // single char between sequences
        i += 1;
    }
    data.truncate(size);
    data
}

fn generate_mixed_content(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    let content = [
        "Hello ",
        "\x1b[31m",
        "World",
        "\x1b[0m",
        "! ",
        "\x1b[10;20H",
        "Cursor moved ",
        "\x1b[38;2;255;0;0m",
        "Red text",
        "\x1b[0m",
        "\n",
    ];

    let mut i = 0;
    while data.len() < size {
        data.extend_from_slice(content[i % content.len()].as_bytes());
        i += 1;
    }
    data.truncate(size);
    data
}

fn bench_parser_throughput(c: &mut Criterion) {
    let sizes = [1024, 10 * 1024, 100 * 1024]; // 1KB, 10KB, 100KB

    let mut group = c.benchmark_group("parser_throughput");

    for size in sizes {
        let plain_text = generate_plain_text(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_function(format!("plain_text_{size}"), |b| {
            b.iter(|| {
                let mut parser = Parser::new();
                parser.advance(black_box(&plain_text), |_action| {});
            });
        });

        let colored_text = generate_colored_text(size);
        group.bench_function(format!("colored_text_{size}"), |b| {
            b.iter(|| {
                let mut parser = Parser::new();
                parser.advance(black_box(&colored_text), |_action| {});
            });
        });

        let cursor_movement = generate_cursor_movement(size);
        group.bench_function(format!("cursor_movement_{size}"), |b| {
            b.iter(|| {
                let mut parser = Parser::new();
                parser.advance(black_box(&cursor_movement), |_action| {});
            });
        });

        let sgr_sequences = generate_sgr_sequences(size);
        group.bench_function(format!("sgr_sequences_{size}"), |b| {
            b.iter(|| {
                let mut parser = Parser::new();
                parser.advance(black_box(&sgr_sequences), |_action| {});
            });
        });

        let mixed_content = generate_mixed_content(size);
        group.bench_function(format!("mixed_content_{size}"), |b| {
            b.iter(|| {
                let mut parser = Parser::new();
                parser.advance(black_box(&mixed_content), |_action| {});
            });
        });
    }

    group.finish();
}

fn bench_chunk_boundaries(c: &mut Criterion) {
    let data = generate_mixed_content(10 * 1024);
    let chunk_sizes = [1, 8, 64, 512, 1024];

    let mut group = c.benchmark_group("chunk_boundaries");

    for chunk_size in chunk_sizes {
        group.bench_function(format!("chunk_{chunk_size}"), |b| {
            b.iter(|| {
                let mut parser = Parser::new();
                for chunk in data.chunks(chunk_size) {
                    parser.advance(black_box(chunk), |_action| {});
                }
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_parser_throughput, bench_chunk_boundaries);
criterion_main!(benches);
