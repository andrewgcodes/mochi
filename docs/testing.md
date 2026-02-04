# Testing Strategy

This document describes the testing strategy for the Mochi terminal emulator.

## Testing Layers

### T1: Unit Tests (Fast, Numerous)

Unit tests cover individual functions and modules in isolation.

**Coverage areas:**
- `terminal-core`: Cell operations, line manipulation, screen state
- `terminal-parser`: State machine transitions, parameter parsing
- Selection logic: Coordinate mapping, range calculations

**Location:** `terminal/src/**/tests.rs` (inline) and `terminal/tests/unit/`

**Running:**
```bash
cargo test --lib
```

**Guidelines:**
- Each public function should have tests
- Test edge cases: empty input, maximum values, boundary conditions
- Tests must be deterministic (no random, no time-dependent)

### T2: Golden Snapshot Tests (Deterministic)

Golden tests verify that specific byte sequences produce expected screen states.

**Format:**
```json
{
  "name": "cursor_movement_basic",
  "input": "\u001b[5;10H",
  "expected": {
    "cursor": {"row": 4, "col": 9},
    "screen": [...]
  }
}
```

**Location:** `terminal/tests/golden/`

**Running:**
```bash
cargo test --test golden
```

**Guidelines:**
- One test file per feature area (cursor.json, sgr.json, etc.)
- Include chunk boundary tests (same input split at different points)
- Document expected behavior in test comments
- Update golden files only when behavior intentionally changes

### T3: Integration Tests (PTY-Driven)

Integration tests spawn real processes via PTY and verify terminal behavior.

**Location:** `terminal/tests/integration/`

**Running:**
```bash
cargo test --test integration
```

**Example:**
```rust
#[test]
fn test_shell_echo() {
    let mut term = TestTerminal::new();
    term.spawn("/bin/sh");
    term.send("echo hello\n");
    term.wait_for_output();
    assert!(term.screen_contains("hello"));
}
```

**Guidelines:**
- Use timeouts to prevent hanging tests
- Clean up child processes properly
- Test resize behavior with SIGWINCH
- Verify cursor position after commands

### T4: Manual Test Scripts

Scripts for testing features that are difficult to automate.

**Location:** `scripts/tests/manual/`

**Contents:**
- `test_vim.md`: Steps to verify vim works correctly
- `test_tmux.md`: Steps to verify tmux works correctly
- `test_htop.md`: Steps to verify htop works correctly
- `test_ssh.md`: Steps to verify SSH sessions work
- `test_vttest.md`: How to run vttest and interpret results

**Format:**
```markdown
# Testing vim

## Prerequisites
- vim installed
- Terminal running

## Steps
1. Open terminal
2. Run `vim test.txt`
3. Press `i` to enter insert mode
4. Type some text
5. Press `Esc`
6. Type `:wq` and press Enter

## Expected Results
- Insert mode indicator shows at bottom
- Text appears as typed
- File is saved and vim exits
```

### T5: Fuzzing

Fuzz testing ensures the parser handles arbitrary input safely.

**Location:** `terminal/fuzz/`

**Running:**
```bash
cargo +nightly fuzz run parser -- -max_len=10000
```

**Targets:**
- `parser`: Feed random bytes to parser
- `utf8`: Test UTF-8 decoder with random bytes
- `screen`: Apply random actions to screen

**Invariants checked:**
- No panics
- No infinite loops (timeout enforced)
- Memory usage bounded
- Screen state remains valid

### T6: Performance Tests

Benchmarks for critical paths.

**Location:** `terminal/benches/`

**Running:**
```bash
cargo bench
```

**Benchmarks:**
- `parse_throughput`: Bytes/second through parser
- `screen_update`: Time to apply actions to screen
- `render_frame`: Time to render full screen

**Targets:**
- Parser: > 100 MB/s
- Screen update: < 1ms for typical operations
- Render: < 16ms for 60 FPS

## Continuous Integration

CI runs on every PR:

```yaml
jobs:
  test:
    steps:
      - cargo fmt --check
      - cargo clippy -- -D warnings
      - cargo test --all
      - cargo test --test golden
      - cargo test --test integration
```

Nightly CI:
```yaml
jobs:
  fuzz:
    steps:
      - cargo +nightly fuzz run parser -- -max_total_time=300
```

## Test Data

### Golden Test Input Generation

Use `printf` or `echo -e` to generate test inputs:

```bash
# Cursor movement
printf '\e[5;10H' > cursor_move.bin

# SGR colors
printf '\e[31mRed\e[0m' > sgr_red.bin

# Complex sequence
printf '\e[?1049h\e[2J\e[H' > alt_screen.bin
```

### Snapshot Format

Screen snapshots are JSON for readability:

```json
{
  "rows": 24,
  "cols": 80,
  "cursor": {
    "row": 0,
    "col": 0,
    "visible": true,
    "style": "block"
  },
  "lines": [
    {
      "cells": [
        {"char": "H", "fg": "default", "bg": "default", "attrs": []},
        {"char": "i", "fg": "default", "bg": "default", "attrs": []}
      ],
      "wrapped": false
    }
  ],
  "modes": {
    "autowrap": true,
    "origin": false
  }
}
```

## Coverage

Track test coverage with:

```bash
cargo tarpaulin --out Html
```

Coverage goals:
- Parser state machine: 100% state coverage
- CSI handlers: 100% of implemented sequences
- Screen operations: > 90%
- Overall: > 80%

## Regression Testing

When fixing bugs:
1. Write a failing test that reproduces the bug
2. Fix the bug
3. Verify the test passes
4. Keep the test to prevent regression

## vttest Compatibility

Run vttest to check VT100/VT220 compatibility:

```bash
vttest
```

Document results in `docs/vttest-results.md`:
- Which tests pass
- Which tests fail and why
- Known differences from xterm

## Test Utilities

### TestTerminal

Helper for integration tests:

```rust
struct TestTerminal {
    core: TerminalCore,
    parser: Parser,
}

impl TestTerminal {
    fn new() -> Self;
    fn feed(&mut self, bytes: &[u8]);
    fn snapshot(&self) -> Snapshot;
    fn assert_cursor(&self, row: usize, col: usize);
    fn assert_cell(&self, row: usize, col: usize, ch: char);
}
```

### Snapshot Comparison

```rust
fn compare_snapshots(actual: &Snapshot, expected: &Snapshot) -> Result<(), Diff>;
```

## Writing Good Tests

1. **Test one thing**: Each test should verify one behavior
2. **Descriptive names**: `test_cursor_moves_down_on_lf` not `test1`
3. **Document expectations**: Comment why the expected result is correct
4. **Reference specs**: Link to xterm docs or ECMA-48 for expected behavior
5. **Edge cases**: Test boundaries, empty input, maximum values
