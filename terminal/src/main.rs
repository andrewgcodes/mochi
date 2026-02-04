//! Mochi Terminal Emulator
//!
//! A real Linux terminal emulator built from scratch.

use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, ModifiersState, MouseScrollDelta, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use mochi_term::input::{self, MouseButton, MouseEventType};
use mochi_term::pty::{Pty, PtyError, WindowSize};
use mochi_term::renderer::{FontRenderer, WindowConfig};
use mochi_term::Terminal;

/// Messages from the PTY reader thread
enum PtyMessage {
    Data(Vec<u8>),
    Error(String),
    Closed,
}

/// The main terminal application state
struct TerminalApp {
    /// Terminal state
    terminal: Arc<Mutex<Terminal>>,
    /// PTY handle
    pty: Option<Pty>,
    /// Window configuration
    config: WindowConfig,
    /// Font renderer
    font: Option<FontRenderer>,
    /// PTY data receiver
    pty_rx: Option<Receiver<PtyMessage>>,
    /// Current modifiers state
    modifiers: ModifiersState,
    /// Last mouse position (for motion tracking)
    last_mouse_pos: (f64, f64),
    /// Mouse button state
    mouse_buttons: [bool; 3],
    /// Needs redraw
    dirty: bool,
    /// Last render time
    last_render: Instant,
}

impl TerminalApp {
    fn new() -> Self {
        let config = WindowConfig::default();

        // Create terminal with default size (will be resized when window opens)
        let terminal = Terminal::new(80, 24, 10000);

        Self {
            terminal: Arc::new(Mutex::new(terminal)),
            pty: None,
            config,
            font: None,
            pty_rx: None,
            modifiers: ModifiersState::empty(),
            last_mouse_pos: (0.0, 0.0),
            mouse_buttons: [false; 3],
            dirty: true,
            last_render: Instant::now(),
        }
    }

    fn spawn_pty(&mut self, cols: usize, rows: usize) -> Result<(), PtyError> {
        let size = WindowSize {
            cols: cols as u16,
            rows: rows as u16,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pty = Pty::spawn(None, size, &[])?;

        // Create channel for PTY data
        let (tx, rx) = mpsc::channel();
        self.pty_rx = Some(rx);

        // Spawn reader thread
        let master_fd = pty.master_fd();
        thread::spawn(move || {
            pty_reader_thread(master_fd, tx);
        });

        self.pty = Some(pty);
        Ok(())
    }

    fn process_pty_data(&mut self) {
        if let Some(rx) = &self.pty_rx {
            // Process all available messages
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    PtyMessage::Data(data) => {
                        if let Ok(mut term) = self.terminal.lock() {
                            term.process(&data);
                            self.dirty = true;
                        }
                    }
                    PtyMessage::Error(e) => {
                        log::error!("PTY error: {}", e);
                    }
                    PtyMessage::Closed => {
                        log::info!("PTY closed");
                    }
                }
            }
        }
    }

    fn write_to_pty(&mut self, data: &[u8]) {
        if let Some(pty) = &mut self.pty {
            if let Err(e) = pty.write_all(data) {
                log::error!("Failed to write to PTY: {}", e);
            }
        }
    }

    fn handle_resize(&mut self, width: u32, height: u32) {
        if let Some(font) = &self.font {
            let (cols, rows) = font.calculate_grid_size(width, height);

            if cols > 0 && rows > 0 {
                // Resize terminal
                if let Ok(mut term) = self.terminal.lock() {
                    term.resize(cols, rows);
                }

                // Resize PTY
                if let Some(pty) = &mut self.pty {
                    let ws = WindowSize {
                        cols: cols as u16,
                        rows: rows as u16,
                        pixel_width: width as u16,
                        pixel_height: height as u16,
                    };
                    if let Err(e) = pty.resize(ws) {
                        log::error!("Failed to resize PTY: {}", e);
                    }
                }

                self.dirty = true;
            }
        }
    }
}

/// PTY reader thread function
fn pty_reader_thread(fd: i32, tx: Sender<PtyMessage>) {
    use std::os::unix::io::FromRawFd;

    // Create a File from the fd for reading
    // Note: We need to be careful not to close the fd when this thread exits
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };

    let mut buf = [0u8; 4096];

    loop {
        match file.read(&mut buf) {
            Ok(0) => {
                // EOF
                let _ = tx.send(PtyMessage::Closed);
                break;
            }
            Ok(n) => {
                if tx.send(PtyMessage::Data(buf[..n].to_vec())).is_err() {
                    break;
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    // No data available, sleep briefly
                    thread::sleep(Duration::from_millis(1));
                    continue;
                }
                let _ = tx.send(PtyMessage::Error(e.to_string()));
                break;
            }
        }
    }

    // Don't close the fd - it's owned by the Pty struct
    std::mem::forget(file);
}

/// Encode a virtual key code to terminal bytes
fn encode_key(
    keycode: VirtualKeyCode,
    modifiers: input::Modifiers,
    app_cursor: bool,
    app_keypad: bool,
) -> Option<Vec<u8>> {
    let key = match keycode {
        VirtualKeyCode::Up => Some(input::Key::Up),
        VirtualKeyCode::Down => Some(input::Key::Down),
        VirtualKeyCode::Left => Some(input::Key::Left),
        VirtualKeyCode::Right => Some(input::Key::Right),
        VirtualKeyCode::Home => Some(input::Key::Home),
        VirtualKeyCode::End => Some(input::Key::End),
        VirtualKeyCode::PageUp => Some(input::Key::PageUp),
        VirtualKeyCode::PageDown => Some(input::Key::PageDown),
        VirtualKeyCode::Insert => Some(input::Key::Insert),
        VirtualKeyCode::Delete => Some(input::Key::Delete),
        VirtualKeyCode::Back => Some(input::Key::Backspace),
        VirtualKeyCode::Tab => Some(input::Key::Tab),
        VirtualKeyCode::Return => Some(input::Key::Enter),
        VirtualKeyCode::Escape => Some(input::Key::Escape),
        VirtualKeyCode::F1 => Some(input::Key::F1),
        VirtualKeyCode::F2 => Some(input::Key::F2),
        VirtualKeyCode::F3 => Some(input::Key::F3),
        VirtualKeyCode::F4 => Some(input::Key::F4),
        VirtualKeyCode::F5 => Some(input::Key::F5),
        VirtualKeyCode::F6 => Some(input::Key::F6),
        VirtualKeyCode::F7 => Some(input::Key::F7),
        VirtualKeyCode::F8 => Some(input::Key::F8),
        VirtualKeyCode::F9 => Some(input::Key::F9),
        VirtualKeyCode::F10 => Some(input::Key::F10),
        VirtualKeyCode::F11 => Some(input::Key::F11),
        VirtualKeyCode::F12 => Some(input::Key::F12),
        _ => None,
    };

    key.map(|k| input::encode_key(k, modifiers, app_cursor, app_keypad))
}

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    log::info!("Starting Mochi Terminal");

    // Create event loop
    let event_loop = EventLoop::new();

    // Create application state
    let mut app = TerminalApp::new();

    // Create window
    let window = WindowBuilder::new()
        .with_title(&app.config.title)
        .with_inner_size(LogicalSize::new(
            app.config.width as f64,
            app.config.height as f64,
        ))
        .build(&event_loop)
        .expect("Failed to create window");

    // Initialize font renderer
    match FontRenderer::with_default_font(app.config.font_size) {
        Ok(font) => {
            let size = window.inner_size();
            let (cols, rows) = font.calculate_grid_size(size.width, size.height);

            app.font = Some(font);

            // Resize terminal to match window
            if let Ok(mut term) = app.terminal.lock() {
                term.resize(cols, rows);
            }

            // Spawn PTY
            if let Err(e) = app.spawn_pty(cols, rows) {
                log::error!("Failed to spawn PTY: {}", e);
            }
        }
        Err(e) => {
            log::error!("Failed to load font: {}", e);
        }
    }

    // Run the event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }

                WindowEvent::Resized(size) => {
                    app.handle_resize(size.width, size.height);
                }

                WindowEvent::ModifiersChanged(mods) => {
                    app.modifiers = mods;
                }

                WindowEvent::KeyboardInput { input, .. } => {
                    if input.state == ElementState::Pressed {
                        let (app_cursor, app_keypad) = {
                            let term = app.terminal.lock().unwrap();
                            (
                                term.screen().modes.application_cursor,
                                term.screen().modes.application_keypad,
                            )
                        };

                        let modifiers = input::Modifiers {
                            shift: app.modifiers.shift(),
                            ctrl: app.modifiers.ctrl(),
                            alt: app.modifiers.alt(),
                        };

                        // Try to encode the key
                        if let Some(keycode) = input.virtual_keycode {
                            if let Some(data) = encode_key(keycode, modifiers, app_cursor, app_keypad) {
                                app.write_to_pty(&data);
                            }
                        }
                    }
                }

                WindowEvent::ReceivedCharacter(c) => {
                    // Handle character input (for regular typing)
                    if !c.is_control() || c == '\r' || c == '\t' {
                        let modifiers = input::Modifiers {
                            shift: app.modifiers.shift(),
                            ctrl: app.modifiers.ctrl(),
                            alt: app.modifiers.alt(),
                        };

                        // Don't double-send if we already handled it as a special key
                        if !modifiers.ctrl {
                            let data = input::encode_char(c, modifiers);
                            app.write_to_pty(&data);
                        }
                    }
                }

                WindowEvent::MouseInput { state, button, .. } => {
                    let btn_idx = match button {
                        winit::event::MouseButton::Left => 0,
                        winit::event::MouseButton::Middle => 1,
                        winit::event::MouseButton::Right => 2,
                        _ => return,
                    };

                    app.mouse_buttons[btn_idx] = state == ElementState::Pressed;

                    let mouse_btn = match button {
                        winit::event::MouseButton::Left => MouseButton::Left,
                        winit::event::MouseButton::Middle => MouseButton::Middle,
                        winit::event::MouseButton::Right => MouseButton::Right,
                        _ => return,
                    };

                    let event_type = if state == ElementState::Pressed {
                        MouseEventType::Press
                    } else {
                        MouseEventType::Release
                    };

                    let (col, row, mode, encoding) = {
                        let term = app.terminal.lock().unwrap();
                        let font = app.font.as_ref().unwrap();
                        let (col, row) = font.pixel_to_cell(
                            app.last_mouse_pos.0 as f32,
                            app.last_mouse_pos.1 as f32,
                        );
                        (
                            col,
                            row,
                            term.screen().modes.mouse_tracking,
                            term.screen().modes.mouse_encoding,
                        )
                    };

                    let modifiers = input::Modifiers {
                        shift: app.modifiers.shift(),
                        ctrl: app.modifiers.ctrl(),
                        alt: app.modifiers.alt(),
                    };

                    if let Some(data) = input::encode_mouse(
                        mouse_btn,
                        event_type,
                        col as u16,
                        row as u16,
                        modifiers,
                        mode,
                        encoding,
                    ) {
                        app.write_to_pty(&data);
                    }
                }

                WindowEvent::CursorMoved { position, .. } => {
                    app.last_mouse_pos = (position.x, position.y);
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    let (button, lines) = match delta {
                        MouseScrollDelta::LineDelta(_, y) => {
                            if y > 0.0 {
                                (MouseButton::WheelUp, y.abs() as i32)
                            } else {
                                (MouseButton::WheelDown, y.abs() as i32)
                            }
                        }
                        MouseScrollDelta::PixelDelta(pos) => {
                            if pos.y > 0.0 {
                                (MouseButton::WheelUp, (pos.y / 20.0).max(1.0) as i32)
                            } else {
                                (MouseButton::WheelDown, (pos.y.abs() / 20.0).max(1.0) as i32)
                            }
                        }
                    };

                    let (col, row, mode, encoding) = {
                        let term = app.terminal.lock().unwrap();
                        let font = app.font.as_ref().unwrap();
                        let (col, row) = font.pixel_to_cell(
                            app.last_mouse_pos.0 as f32,
                            app.last_mouse_pos.1 as f32,
                        );
                        (
                            col,
                            row,
                            term.screen().modes.mouse_tracking,
                            term.screen().modes.mouse_encoding,
                        )
                    };

                    let modifiers = input::Modifiers {
                        shift: app.modifiers.shift(),
                        ctrl: app.modifiers.ctrl(),
                        alt: app.modifiers.alt(),
                    };

                    // Send multiple scroll events for multiple lines
                    for _ in 0..lines {
                        if let Some(data) = input::encode_mouse(
                            button,
                            MouseEventType::Press,
                            col as u16,
                            row as u16,
                            modifiers,
                            mode,
                            encoding,
                        ) {
                            app.write_to_pty(&data);
                        }
                    }
                }

                WindowEvent::Focused(focused) => {
                    let focus_reporting = {
                        let term = app.terminal.lock().unwrap();
                        term.screen().modes.focus_reporting
                    };

                    if focus_reporting {
                        let data = input::encode_focus(focused);
                        app.write_to_pty(&data);
                    }
                }

                _ => {}
            },

            Event::MainEventsCleared => {
                // Process PTY data
                app.process_pty_data();

                // Request redraw if dirty or periodically
                if app.dirty || app.last_render.elapsed() > Duration::from_millis(16) {
                    window.request_redraw();
                }
            }

            Event::RedrawRequested(_) => {
                // Render would happen here
                // For now, just mark as clean
                app.dirty = false;
                app.last_render = Instant::now();
            }

            _ => {}
        }
    });
}
