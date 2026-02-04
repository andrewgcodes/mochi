//! Headless Runner
//!
//! A tool for running terminal emulation without a GUI.
//! Reads input bytes and produces deterministic screen snapshots.
//! Used for testing and golden tests.

use std::io::{self, Read, Write};

use mochi_term::core::{EraseMode, Screen, TabClearMode};
use mochi_term::parser::{Action, ControlCode, CsiAction, EscAction, OscAction, Parser};

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let (cols, rows) = if args.len() >= 3 {
        let cols: usize = args[1].parse().unwrap_or(80);
        let rows: usize = args[2].parse().unwrap_or(24);
        (cols, rows)
    } else {
        (80, 24)
    };

    eprintln!("Headless runner: {}x{}", cols, rows);

    // Create screen and parser
    let mut screen = Screen::new(cols, rows);
    let mut parser = Parser::new();

    // Read input from stdin
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input)?;

    eprintln!("Read {} bytes", input.len());

    // Parse and apply actions
    let actions = parser.parse(&input);
    eprintln!("Parsed {} actions", actions.len());

    for action in actions {
        apply_action(&mut screen, action);
    }

    // Output snapshot as JSON
    let snapshot = screen.snapshot();
    let json = serde_json::to_string_pretty(&snapshot)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    io::stdout().write_all(json.as_bytes())?;
    io::stdout().write_all(b"\n")?;

    Ok(())
}

fn apply_action(screen: &mut Screen, action: Action) {
    match action {
        Action::Print(c) => {
            screen.print(c);
        }
        Action::Control(ctrl) => {
            apply_control(screen, ctrl);
        }
        Action::Csi(csi) => {
            apply_csi(screen, csi);
        }
        Action::Osc(osc) => {
            apply_osc(screen, osc);
        }
        Action::Esc(esc) => {
            apply_esc(screen, esc);
        }
        _ => {}
    }
}

fn apply_control(screen: &mut Screen, ctrl: ControlCode) {
    match ctrl {
        ControlCode::Backspace => screen.backspace(),
        ControlCode::Tab => screen.tab(),
        ControlCode::LineFeed | ControlCode::VerticalTab | ControlCode::FormFeed => {
            screen.linefeed()
        }
        ControlCode::CarriageReturn => screen.carriage_return(),
        _ => {}
    }
}

fn apply_csi(screen: &mut Screen, csi: CsiAction) {
    match (csi.private_marker, csi.final_char) {
        // Cursor movement
        (None, 'A') => screen.move_cursor_up(csi.param_or_default(0, 1) as usize),
        (None, 'B') => screen.move_cursor_down(csi.param_or_default(0, 1) as usize),
        (None, 'C') => screen.move_cursor_forward(csi.param_or_default(0, 1) as usize),
        (None, 'D') => screen.move_cursor_backward(csi.param_or_default(0, 1) as usize),
        (None, 'E') => {
            screen.move_cursor_down(csi.param_or_default(0, 1) as usize);
            screen.carriage_return();
        }
        (None, 'F') => {
            screen.move_cursor_up(csi.param_or_default(0, 1) as usize);
            screen.carriage_return();
        }
        (None, 'G') => screen.move_cursor_to_column(csi.param_or_default(0, 1) as usize),
        (None, 'H') | (None, 'f') => {
            let row = csi.param_or_default(0, 1) as usize;
            let col = csi.param_or_default(1, 1) as usize;
            screen.move_cursor_to(row, col);
        }
        (None, 'd') => screen.move_cursor_to_row(csi.param_or_default(0, 1) as usize),

        // Erase
        (None, 'J') => {
            let mode = match csi.param(0, 0) {
                0 => EraseMode::ToEnd,
                1 => EraseMode::ToBeginning,
                2 => EraseMode::All,
                3 => EraseMode::Scrollback,
                _ => return,
            };
            screen.erase_in_display(mode);
        }
        (None, 'K') => {
            let mode = match csi.param(0, 0) {
                0 => EraseMode::ToEnd,
                1 => EraseMode::ToBeginning,
                _ => EraseMode::All,
            };
            screen.erase_in_line(mode);
        }
        (None, 'X') => screen.erase_chars(csi.param_or_default(0, 1) as usize),

        // Insert/Delete
        (None, '@') => screen.insert_chars(csi.param_or_default(0, 1) as usize),
        (None, 'P') => screen.delete_chars(csi.param_or_default(0, 1) as usize),
        (None, 'L') => screen.insert_lines(csi.param_or_default(0, 1) as usize),
        (None, 'M') => screen.delete_lines(csi.param_or_default(0, 1) as usize),

        // Scroll
        (None, 'S') => screen.scroll_up(csi.param_or_default(0, 1) as usize),
        (None, 'T') => screen.scroll_down(csi.param_or_default(0, 1) as usize),

        // Scroll region
        (None, 'r') => {
            let top = csi.param_or_default(0, 1) as usize;
            let bottom = csi.param_or_default(1, screen.rows() as u16) as usize;
            screen.set_scroll_region(top.saturating_sub(1), bottom.saturating_sub(1));
            screen.move_cursor_to(1, 1);
        }

        // SGR
        (None, 'm') => apply_sgr(screen, &csi.params),

        // Cursor save/restore
        (None, 's') => screen.save_cursor(),
        (None, 'u') => screen.restore_cursor(),

        // Tab clear
        (None, 'g') => {
            let mode = match csi.param(0, 0) {
                0 => TabClearMode::Current,
                3 => TabClearMode::All,
                _ => return,
            };
            screen.clear_tab_stop(mode);
        }

        // DEC Private modes
        (Some('?'), 'h') => {
            for &param in &csi.params {
                set_dec_mode(screen, param, true);
            }
        }
        (Some('?'), 'l') => {
            for &param in &csi.params {
                set_dec_mode(screen, param, false);
            }
        }

        // Standard modes
        (None, 'h') => {
            for &param in &csi.params {
                set_mode(screen, param, true);
            }
        }
        (None, 'l') => {
            for &param in &csi.params {
                set_mode(screen, param, false);
            }
        }

        _ => {}
    }
}

fn apply_sgr(screen: &mut Screen, params: &[u16]) {
    let mut i = 0;
    while i < params.len() {
        match params[i] {
            0 => screen.cursor.attrs.reset(),
            1 => screen.cursor.attrs.style.bold = true,
            2 => screen.cursor.attrs.style.faint = true,
            3 => screen.cursor.attrs.style.italic = true,
            4 => screen.cursor.attrs.style.underline = true,
            5 | 6 => screen.cursor.attrs.style.blink = true,
            7 => screen.cursor.attrs.style.inverse = true,
            8 => screen.cursor.attrs.style.hidden = true,
            9 => screen.cursor.attrs.style.strikethrough = true,
            22 => {
                screen.cursor.attrs.style.bold = false;
                screen.cursor.attrs.style.faint = false;
            }
            23 => screen.cursor.attrs.style.italic = false,
            24 => screen.cursor.attrs.style.underline = false,
            25 => screen.cursor.attrs.style.blink = false,
            27 => screen.cursor.attrs.style.inverse = false,
            28 => screen.cursor.attrs.style.hidden = false,
            29 => screen.cursor.attrs.style.strikethrough = false,
            30..=37 => {
                screen.cursor.attrs.fg = mochi_term::core::Color::Indexed((params[i] - 30) as u8)
            }
            38 => {
                if let Some(color) = parse_extended_color(params, &mut i) {
                    screen.cursor.attrs.fg = color;
                }
            }
            39 => screen.cursor.attrs.fg = mochi_term::core::Color::Default,
            40..=47 => {
                screen.cursor.attrs.bg = mochi_term::core::Color::Indexed((params[i] - 40) as u8)
            }
            48 => {
                if let Some(color) = parse_extended_color(params, &mut i) {
                    screen.cursor.attrs.bg = color;
                }
            }
            49 => screen.cursor.attrs.bg = mochi_term::core::Color::Default,
            90..=97 => {
                screen.cursor.attrs.fg =
                    mochi_term::core::Color::Indexed((params[i] - 90 + 8) as u8)
            }
            100..=107 => {
                screen.cursor.attrs.bg =
                    mochi_term::core::Color::Indexed((params[i] - 100 + 8) as u8)
            }
            _ => {}
        }
        i += 1;
    }

    if params.is_empty() {
        screen.cursor.attrs.reset();
    }
}

fn parse_extended_color(params: &[u16], i: &mut usize) -> Option<mochi_term::core::Color> {
    if *i + 1 >= params.len() {
        return None;
    }

    match params[*i + 1] {
        5 => {
            if *i + 2 < params.len() {
                *i += 2;
                Some(mochi_term::core::Color::Indexed(params[*i] as u8))
            } else {
                None
            }
        }
        2 => {
            if *i + 4 < params.len() {
                let r = params[*i + 2] as u8;
                let g = params[*i + 3] as u8;
                let b = params[*i + 4] as u8;
                *i += 4;
                Some(mochi_term::core::Color::Rgb(r, g, b))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn set_dec_mode(screen: &mut Screen, mode: u16, enable: bool) {
    match mode {
        1 => screen.modes.application_cursor_keys = enable,
        6 => {
            screen.modes.origin_mode = enable;
            if enable {
                let (top, _) = screen.scroll_region();
                screen.cursor.row = top;
                screen.cursor.col = 0;
            }
        }
        7 => screen.modes.auto_wrap = enable,
        25 => {
            screen.modes.cursor_visible = enable;
            screen.cursor.visible = enable;
        }
        47 | 1047 => {
            if enable {
                screen.enter_alternate_screen();
            } else {
                screen.exit_alternate_screen();
            }
            screen.modes.alternate_screen = enable;
        }
        1048 => {
            if enable {
                screen.save_cursor();
            } else {
                screen.restore_cursor();
            }
        }
        1049 => {
            if enable {
                screen.save_cursor();
                screen.enter_alternate_screen();
                screen.erase_in_display(EraseMode::All);
            } else {
                screen.exit_alternate_screen();
                screen.restore_cursor();
            }
            screen.modes.alternate_screen = enable;
        }
        2004 => screen.modes.bracketed_paste = enable,
        _ => {}
    }
}

fn set_mode(screen: &mut Screen, mode: u16, enable: bool) {
    match mode {
        4 => screen.modes.insert_mode = enable,
        20 => screen.modes.linefeed_mode = enable,
        _ => {}
    }
}

fn apply_osc(screen: &mut Screen, osc: OscAction) {
    if let OscAction::SetTitle(title) = osc {
        screen.title = title;
    }
}

fn apply_esc(screen: &mut Screen, esc: EscAction) {
    match esc {
        EscAction::SaveCursor => screen.save_cursor(),
        EscAction::RestoreCursor => screen.restore_cursor(),
        EscAction::Index => screen.linefeed(),
        EscAction::ReverseIndex => screen.reverse_index(),
        EscAction::NextLine => {
            screen.carriage_return();
            screen.linefeed();
        }
        EscAction::TabSet => screen.set_tab_stop(),
        EscAction::FullReset => screen.reset(),
        _ => {}
    }
}
