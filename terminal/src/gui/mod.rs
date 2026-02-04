//! GUI Module
//!
//! Provides the graphical user interface for the terminal emulator.
//! Uses winit for window management and softbuffer for rendering.

mod font;
mod input;
mod renderer;
mod selection;

use std::sync::Arc;
use std::time::{Duration, Instant};

use arboard::Clipboard;
use log::{debug, error, info, warn};
use winit::dpi::LogicalSize;
use winit::event::{
    ElementState, Event, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent,
};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use crate::core::{EraseMode, Screen, TabClearMode};
use crate::parser::{Action, ControlCode, CsiAction, EscAction, OscAction, Parser};
use crate::pty::Pty;

pub use input::{InputEncoder, MouseButton as InputMouseButton, MouseEventType};
pub use renderer::Renderer;
pub use selection::Selection;

#[derive(Debug, Clone)]
pub struct Config {
    pub width: u32,
    pub height: u32,
    pub font_size: f32,
    pub font_family: String,
    pub scrollback_size: usize,
    pub osc52_enabled: bool,
    pub osc52_max_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            font_size: 14.0,
            font_family: "monospace".to_string(),
            scrollback_size: 10000,
            osc52_enabled: false,
            osc52_max_size: 65536,
        }
    }
}

impl Config {
    pub fn osc52_enabled(&self) -> bool {
        self.osc52_enabled
    }
}

/// Main entry point for the GUI terminal
pub fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting terminal with config: {:?}", config);

    // Create event loop and window
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Mochi Terminal")
        .with_inner_size(LogicalSize::new(config.width, config.height))
        .build(&event_loop)?;

    let window = Arc::new(window);

    // Create renderer
    let mut renderer = Renderer::new(window.clone(), config.font_size)?;
    let (cols, rows) = renderer.grid_size();

    info!("Terminal size: {}x{}", cols, rows);

    // Create screen and parser
    let mut screen = Screen::new(cols, rows);
    let mut parser = Parser::new();

    // Spawn PTY with shell
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    info!("Spawning shell: {}", shell);
    let mut pty = Pty::spawn(Some(&shell), cols as u16, rows as u16)?;

    // Input encoder
    let input_encoder = InputEncoder::new();

    // Selection state
    let mut selection = Selection::new();
    let mut scroll_offset: usize = 0;

    // Clipboard
    let mut clipboard = Clipboard::new().ok();

    // Mouse state
    let mut mouse_pressed = false;
    let mut last_mouse_pos: Option<(usize, usize)> = None;
    let mut modifiers = winit::event::ModifiersState::empty();

    // Read buffer for PTY
    let mut read_buf = [0u8; 65536];

    // Timing for rendering
    let mut last_render = Instant::now();
    let render_interval = Duration::from_millis(16); // ~60 FPS

    // Track if we need to redraw
    let mut needs_redraw = true;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    info!("Window close requested");
                    *control_flow = ControlFlow::Exit;
                }

                WindowEvent::Resized(size) => {
                    debug!("Window resized to {:?}", size);
                    renderer.resize(size.width, size.height);
                    let (new_cols, new_rows) = renderer.grid_size();
                    if new_cols != screen.cols() || new_rows != screen.rows() {
                        screen.resize(new_cols, new_rows);
                        if let Err(e) = pty.resize(new_cols as u16, new_rows as u16) {
                            warn!("Failed to resize PTY: {}", e);
                        }
                    }
                    needs_redraw = true;
                }

                WindowEvent::ModifiersChanged(mods) => {
                    modifiers = mods;
                }

                WindowEvent::ReceivedCharacter(c) => {
                    // Handle text input
                    if !c.is_control() || c == '\r' || c == '\t' {
                        let mut buf = [0u8; 4];
                        let s = c.encode_utf8(&mut buf);

                        // Handle bracketed paste mode
                        if let Err(e) = pty.write(s.as_bytes()) {
                            error!("Failed to write to PTY: {}", e);
                        }
                    }
                }

                WindowEvent::KeyboardInput { input, .. } => {
                    if input.state == ElementState::Pressed {
                        if let Some(keycode) = input.virtual_keycode {
                            // Handle special key combinations
                            if modifiers.ctrl() {
                                match keycode {
                                    VirtualKeyCode::C => {
                                        // Ctrl+Shift+C = Copy
                                        if modifiers.shift() {
                                            if let Some(text) =
                                                selection.get_text(&screen, scroll_offset)
                                            {
                                                if let Some(ref mut cb) = clipboard {
                                                    let _ = cb.set_text(text);
                                                }
                                            }
                                            return;
                                        }
                                        // Ctrl+C = send interrupt
                                        let _ = pty.write(&[0x03]);
                                        return;
                                    }
                                    VirtualKeyCode::V => {
                                        // Ctrl+Shift+V = Paste
                                        if modifiers.shift() {
                                            if let Some(ref mut cb) = clipboard {
                                                if let Ok(text) = cb.get_text() {
                                                    // Bracketed paste mode
                                                    if screen.modes.bracketed_paste {
                                                        let _ = pty.write(b"\x1b[200~");
                                                        let _ = pty.write(text.as_bytes());
                                                        let _ = pty.write(b"\x1b[201~");
                                                    } else {
                                                        let _ = pty.write(text.as_bytes());
                                                    }
                                                }
                                            }
                                            return;
                                        }
                                        // Ctrl+V = send literal
                                        let _ = pty.write(&[0x16]);
                                        return;
                                    }
                                    VirtualKeyCode::L => {
                                        // Ctrl+L = clear screen
                                        let _ = pty.write(&[0x0c]);
                                        return;
                                    }
                                    VirtualKeyCode::D => {
                                        // Ctrl+D = EOF
                                        let _ = pty.write(&[0x04]);
                                        return;
                                    }
                                    VirtualKeyCode::Z => {
                                        // Ctrl+Z = suspend
                                        let _ = pty.write(&[0x1a]);
                                        return;
                                    }
                                    _ => {}
                                }

                                // Send Ctrl+letter
                                if let Some(c) = keycode_to_char(keycode) {
                                    if c.is_ascii_alphabetic() {
                                        let ctrl_char = (c.to_ascii_uppercase() as u8) - b'A' + 1;
                                        let _ = pty.write(&[ctrl_char]);
                                        return;
                                    }
                                }
                            }

                            // Handle special keys
                            if let Some(seq) =
                                input_encoder.encode_keycode(keycode, modifiers, &screen.modes)
                            {
                                let _ = pty.write(&seq);
                            }
                        }
                    }
                }

                WindowEvent::MouseInput { state, button, .. } => {
                    // Convert winit button to our input button type
                    let input_button = match button {
                        MouseButton::Left => Some(InputMouseButton::Left),
                        MouseButton::Middle => Some(InputMouseButton::Middle),
                        MouseButton::Right => Some(InputMouseButton::Right),
                        MouseButton::Other(_) => None,
                    };

                    let event_type = if state == ElementState::Pressed {
                        MouseEventType::Press
                    } else {
                        MouseEventType::Release
                    };

                    // Try to send mouse event to PTY if mouse tracking is enabled
                    if let (Some(btn), Some((col, row))) = (input_button, last_mouse_pos) {
                        if let Some(seq) = input_encoder.encode_mouse(
                            btn,
                            event_type,
                            col,
                            row,
                            modifiers,
                            &screen.modes,
                        ) {
                            let _ = pty.write(&seq);
                            // Don't do selection when mouse tracking is enabled
                            return;
                        }
                    }

                    // Fall back to selection behavior when mouse tracking is disabled
                    match button {
                        MouseButton::Left => {
                            mouse_pressed = state == ElementState::Pressed;
                            if mouse_pressed {
                                // Start selection
                                if let Some((col, row)) = last_mouse_pos {
                                    selection.start(col, row + scroll_offset);
                                    needs_redraw = true;
                                }
                            } else {
                                // End selection - copy to primary clipboard on X11
                                if let Some(text) = selection.get_text(&screen, scroll_offset) {
                                    if !text.is_empty() {
                                        if let Some(ref mut cb) = clipboard {
                                            let _ = cb.set_text(text);
                                        }
                                    }
                                }
                            }
                        }
                        MouseButton::Middle => {
                            // Middle click = paste primary selection
                            if state == ElementState::Pressed {
                                if let Some(ref mut cb) = clipboard {
                                    if let Ok(text) = cb.get_text() {
                                        if screen.modes.bracketed_paste {
                                            let _ = pty.write(b"\x1b[200~");
                                            let _ = pty.write(text.as_bytes());
                                            let _ = pty.write(b"\x1b[201~");
                                        } else {
                                            let _ = pty.write(text.as_bytes());
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                WindowEvent::CursorMoved { position, .. } => {
                    let (col, row) = renderer.pixel_to_cell(position.x, position.y);
                    last_mouse_pos = Some((col, row));

                    // Send motion events if mouse tracking is enabled and button is pressed
                    if mouse_pressed {
                        if let Some(seq) = input_encoder.encode_mouse(
                            InputMouseButton::Left,
                            MouseEventType::Motion,
                            col,
                            row,
                            modifiers,
                            &screen.modes,
                        ) {
                            let _ = pty.write(&seq);
                            // Don't do selection when mouse tracking is enabled
                            return;
                        }

                        // Fall back to selection behavior
                        selection.update(col, row + scroll_offset);
                        needs_redraw = true;
                    }
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    let lines = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y as i32,
                        MouseScrollDelta::PixelDelta(pos) => (pos.y / 20.0) as i32,
                    };

                    // Send scroll events to PTY if mouse tracking is enabled
                    if let Some((col, row)) = last_mouse_pos {
                        let button = if lines > 0 {
                            InputMouseButton::WheelUp
                        } else {
                            InputMouseButton::WheelDown
                        };
                        if let Some(seq) = input_encoder.encode_mouse(
                            button,
                            MouseEventType::Press,
                            col,
                            row,
                            modifiers,
                            &screen.modes,
                        ) {
                            // Send multiple scroll events for larger deltas
                            for _ in 0..lines.unsigned_abs() {
                                let _ = pty.write(&seq);
                            }
                            return;
                        }
                    }

                    // Fall back to scrollback behavior when mouse tracking is disabled
                    use std::cmp::Ordering;
                    match lines.cmp(&0) {
                        Ordering::Greater => {
                            // Scroll up (into history)
                            scroll_offset =
                                scroll_offset.saturating_add(lines.unsigned_abs() as usize);
                            let max_scroll = screen.scrollback().len();
                            scroll_offset = scroll_offset.min(max_scroll);
                        }
                        Ordering::Less => {
                            // Scroll down (toward present)
                            scroll_offset =
                                scroll_offset.saturating_sub(lines.unsigned_abs() as usize);
                        }
                        Ordering::Equal => {}
                    }
                    needs_redraw = true;
                }

                _ => {}
            },

            Event::MainEventsCleared => {
                // Check if child process has exited first
                match pty.try_wait() {
                    Ok(Some(code)) => {
                        info!("Child process exited with code: {}", code);
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                    Ok(None) => {
                        // Still running, continue
                    }
                    Err(e) => {
                        error!("Error checking child process: {}", e);
                    }
                }

                // Read from PTY (non-blocking)
                match pty.read(&mut read_buf) {
                    Ok(0) => {
                        // No data available (non-blocking read returns 0)
                        // This is normal, not an error
                    }
                    Ok(n) => {
                        // Parse and apply actions
                        let actions = parser.parse(&read_buf[..n]);
                        for action in actions {
                            apply_action(&mut screen, action, &config, &mut clipboard);
                        }
                        // Reset scroll to bottom when new output arrives
                        scroll_offset = 0;
                        needs_redraw = true;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No data available, that's fine
                    }
                    Err(e) => {
                        error!("PTY read error: {}", e);
                        *control_flow = ControlFlow::Exit;
                    }
                }

                // Render at fixed interval
                let now = Instant::now();
                if needs_redraw && now.duration_since(last_render) >= render_interval {
                    renderer.render(&screen, &selection, scroll_offset);
                    last_render = now;
                    needs_redraw = false;
                }
            }

            Event::RedrawRequested(_) => {
                renderer.render(&screen, &selection, scroll_offset);
                last_render = Instant::now();
                needs_redraw = false;
            }

            _ => {}
        }
    });
}

/// Convert a virtual keycode to a character
fn keycode_to_char(keycode: VirtualKeyCode) -> Option<char> {
    match keycode {
        VirtualKeyCode::A => Some('a'),
        VirtualKeyCode::B => Some('b'),
        VirtualKeyCode::C => Some('c'),
        VirtualKeyCode::D => Some('d'),
        VirtualKeyCode::E => Some('e'),
        VirtualKeyCode::F => Some('f'),
        VirtualKeyCode::G => Some('g'),
        VirtualKeyCode::H => Some('h'),
        VirtualKeyCode::I => Some('i'),
        VirtualKeyCode::J => Some('j'),
        VirtualKeyCode::K => Some('k'),
        VirtualKeyCode::L => Some('l'),
        VirtualKeyCode::M => Some('m'),
        VirtualKeyCode::N => Some('n'),
        VirtualKeyCode::O => Some('o'),
        VirtualKeyCode::P => Some('p'),
        VirtualKeyCode::Q => Some('q'),
        VirtualKeyCode::R => Some('r'),
        VirtualKeyCode::S => Some('s'),
        VirtualKeyCode::T => Some('t'),
        VirtualKeyCode::U => Some('u'),
        VirtualKeyCode::V => Some('v'),
        VirtualKeyCode::W => Some('w'),
        VirtualKeyCode::X => Some('x'),
        VirtualKeyCode::Y => Some('y'),
        VirtualKeyCode::Z => Some('z'),
        _ => None,
    }
}

/// Apply a parsed action to the screen
fn apply_action(
    screen: &mut Screen,
    action: Action,
    config: &Config,
    clipboard: &mut Option<Clipboard>,
) {
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
            apply_osc(screen, osc, config, clipboard);
        }
        Action::Esc(esc) => {
            apply_esc(screen, esc);
        }
        _ => {}
    }
}

fn apply_control(screen: &mut Screen, ctrl: ControlCode) {
    match ctrl {
        ControlCode::Bell => screen.bell(),
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

        // Device status report
        (None, 'n') => {
            // We don't respond to DSR in this context
            // Would need PTY write access
        }

        // Cursor style (DECSCUSR)
        (None, 'q') if !csi.intermediates.is_empty() && csi.intermediates[0] == ' ' => {
            // Cursor style - we could implement this
        }

        _ => {
            debug!("Unhandled CSI: {:?} {}", csi.private_marker, csi.final_char);
        }
    }
}

fn apply_sgr(screen: &mut Screen, params: &[u16]) {
    use crate::core::Color;

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
            30..=37 => screen.cursor.attrs.fg = Color::Indexed((params[i] - 30) as u8),
            38 => {
                if let Some(color) = parse_extended_color(params, &mut i) {
                    screen.cursor.attrs.fg = color;
                }
            }
            39 => screen.cursor.attrs.fg = Color::Default,
            40..=47 => screen.cursor.attrs.bg = Color::Indexed((params[i] - 40) as u8),
            48 => {
                if let Some(color) = parse_extended_color(params, &mut i) {
                    screen.cursor.attrs.bg = color;
                }
            }
            49 => screen.cursor.attrs.bg = Color::Default,
            90..=97 => screen.cursor.attrs.fg = Color::Indexed((params[i] - 90 + 8) as u8),
            100..=107 => screen.cursor.attrs.bg = Color::Indexed((params[i] - 100 + 8) as u8),
            _ => {}
        }
        i += 1;
    }

    if params.is_empty() {
        screen.cursor.attrs.reset();
    }
}

fn parse_extended_color(params: &[u16], i: &mut usize) -> Option<crate::core::Color> {
    use crate::core::Color;

    if *i + 1 >= params.len() {
        return None;
    }

    match params[*i + 1] {
        5 => {
            // 256 color mode
            if *i + 2 < params.len() {
                *i += 2;
                Some(Color::Indexed(params[*i] as u8))
            } else {
                None
            }
        }
        2 => {
            // True color mode
            if *i + 4 < params.len() {
                let r = params[*i + 2] as u8;
                let g = params[*i + 3] as u8;
                let b = params[*i + 4] as u8;
                *i += 4;
                Some(Color::Rgb(r, g, b))
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
        12 => {
            // Cursor blink - we don't implement blinking yet
        }
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
        1000 => {
            screen.modes.mouse_tracking = if enable {
                crate::core::MouseMode::X10
            } else {
                crate::core::MouseMode::None
            }
        }
        1002 => {
            screen.modes.mouse_tracking = if enable {
                crate::core::MouseMode::ButtonEvent
            } else {
                crate::core::MouseMode::None
            }
        }
        1003 => {
            screen.modes.mouse_tracking = if enable {
                crate::core::MouseMode::AnyEvent
            } else {
                crate::core::MouseMode::None
            }
        }
        1006 => {
            screen.modes.mouse_encoding = if enable {
                crate::core::MouseEncoding::Sgr
            } else {
                crate::core::MouseEncoding::X10
            }
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
        _ => {
            debug!("Unhandled DEC mode: {} = {}", mode, enable);
        }
    }
}

fn set_mode(screen: &mut Screen, mode: u16, enable: bool) {
    match mode {
        4 => screen.modes.insert_mode = enable,
        20 => screen.modes.linefeed_mode = enable,
        _ => {
            debug!("Unhandled mode: {} = {}", mode, enable);
        }
    }
}

fn apply_osc(
    screen: &mut Screen,
    osc: OscAction,
    config: &Config,
    clipboard: &mut Option<Clipboard>,
) {
    match osc {
        OscAction::SetTitle(title) => {
            screen.title = title;
            // Note: We'd need window access to actually set the title
        }
        OscAction::SetIconName(_) => {
            // Icon name - not commonly used
        }
        OscAction::Hyperlink { params, url } => {
            if url.is_empty() {
                // End hyperlink
                screen.current_hyperlink = None;
                screen.cursor.attrs.hyperlink_id = 0;
            } else {
                // Start hyperlink
                let id = screen.register_hyperlink(url, params);
                screen.cursor.attrs.hyperlink_id = id;
            }
        }
        OscAction::Clipboard {
            clipboard: cb_type,
            data,
        } => {
            if !config.osc52_enabled {
                warn!("OSC 52 clipboard access disabled");
                return;
            }

            if data == "?" {
                // Query clipboard - we don't support this for security
                return;
            }

            if data.len() > config.osc52_max_size {
                warn!(
                    "OSC 52 data too large: {} > {}",
                    data.len(),
                    config.osc52_max_size
                );
                return;
            }

            // Decode base64 and set clipboard
            if cb_type.contains('c') || cb_type.contains('s') {
                if let Ok(decoded) = base64_decode(&data) {
                    if let Ok(text) = String::from_utf8(decoded) {
                        if let Some(ref mut cb) = clipboard {
                            let _ = cb.set_text(text);
                        }
                    }
                }
            }
        }
        OscAction::SetColor { .. } | OscAction::ResetColor { .. } => {
            // Color customization - not implemented yet
        }
        OscAction::Unknown { command, .. } => {
            debug!("Unknown OSC command: {}", command);
        }
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
        EscAction::ApplicationKeypad => {
            // Application keypad mode - affects numpad key encoding
            // We handle this in the input encoder
        }
        EscAction::NormalKeypad => {
            // Normal keypad mode
        }
        EscAction::SelectG0Ascii | EscAction::SelectG1Ascii => {
            // Character set selection - basic ASCII is default
        }
        EscAction::SelectG0DecGraphics | EscAction::SelectG1DecGraphics => {
            // DEC graphics character set - would need special handling
        }
        EscAction::DecAlignmentTest => {
            // Fill screen with 'E' characters
            for row in 0..screen.rows() {
                for col in 0..screen.cols() {
                    screen.move_cursor_to(row + 1, col + 1);
                    screen.print('E');
                }
            }
            screen.move_cursor_to(1, 1);
        }
        _ => {}
    }
}

/// Simple base64 decoder
fn base64_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut output = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;

    for c in input.bytes() {
        if c == b'=' {
            break;
        }
        if c == b'\n' || c == b'\r' || c == b' ' {
            continue;
        }

        let value = ALPHABET
            .iter()
            .position(|&x| x == c)
            .ok_or("Invalid base64 character")? as u32;

        buffer = (buffer << 6) | value;
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }

    Ok(output)
}
