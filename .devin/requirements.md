# Mochi Terminal Emulator Requirements

## Overview
Build a real Linux terminal emulator from scratch that:
- Runs a real shell/apps via PTY
- Correctly parses VT/xterm escape sequences
- Maintains a proper screen model
- Renders a GUI

## Non-Negotiable Rules
- NO terminal emulation libraries (libvte, termwiz, vt100 crates, etc.)
- NO placeholders or TODO stubs - every feature must be fully implemented
- Continuous documentation as code is written
- Many incremental commits
- Research before implementing each capability

## Core Deliverables

### D1: mochi-term Application
- Runs on Linux (X11/Wayland)
- Spawns child shell via PTY
- Handles interactive apps: vim, htop, less, man, tmux, ssh
- Supports:
  - UTF-8 text
  - Resizing (SIGWINCH)
  - Scrollback
  - Selection + clipboard
  - Colors: 16 + 256 + truecolor
  - Cursor styles/visibility
  - Alternate screen
  - Bracketed paste
  - Mouse reporting
  - OSC window title, hyperlinks, clipboard

### D2: Headless Test Harness
- Terminal core library without GUI
- Deterministic snapshots
- Unit tests, golden tests, fuzz tests

### D3: Documentation
- docs/architecture.md
- docs/escape-sequences.md (coverage matrix)
- docs/testing.md
- docs/security.md
- docs/research/ notes

## Architecture Requirements

### A) terminal-core (platform-independent)
- Screen model: primary/alternate grids, scrollback, cursor, tab stops, margins, modes, SGR attributes
- Deterministic behavior

### B) terminal-parser (platform-independent)
- Stateful parser: bytes -> terminal actions
- Supports: Print, Control chars, CSI, OSC, ESC, DCS
- Incremental streaming
- Correct UTF-8 decoding

### C) pty (Linux-specific)
- PTY open/spawn/resize
- Non-blocking IO
- Signal handling

### D) frontend (GUI)
- Window + event loop
- Font rendering
- Keyboard/mouse input
- Clipboard integration

## Minimum Escape Sequence Coverage
- C0: BEL, BS, HT, LF, VT/FF, CR, ESC
- ESC: 7/8 (save/restore), D/M/E/H (IND/RI/NEL/HTS)
- CSI: A/B/C/D/G/d/H/f (cursor), J/K/X (erase), r (scroll region), @/P/L/M (insert/delete), m (SGR), h/l (modes)
- OSC: 0/2 (title), 8 (hyperlinks), 52 (clipboard)

## Testing Requirements
- T1: Unit tests (fast, many)
- T2: Golden snapshot tests
- T3: Integration tests (PTY-driven)
- T4: Manual test scripts
- T5: Fuzzing
- T6: Performance tests

## Security Requirements
- OSC 52: default OFF, size limits, user opt-in
- OSC 8: safe URL handling
- DoS protection: bounded sequences, scrollback limits
