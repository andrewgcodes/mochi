//! Mochi Terminal - Main GUI Application
//!
//! A real Linux terminal emulator with GPU-accelerated rendering.

use std::process::ExitCode;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[cfg(feature = "gui")]
mod gui {
    use std::sync::Arc;

    use mochi_terminal::core::{MouseEncoding as CoreMouseEncoding, MouseMode, Screen};
    use mochi_terminal::frontend::{
        encode_key, KeyCode, Modifiers, MouseButton, MouseEncoding, MouseEventType, Renderer,
        RendererConfig,
    };
    use mochi_terminal::parser::Parser;
    use mochi_terminal::pty::{Pty, WindowSize};

    use winit::{
        event::{ElementState, Event, MouseButton as WinitMouseButton, WindowEvent},
        event_loop::EventLoop,
        keyboard::{Key, NamedKey},
        window::WindowBuilder,
    };

    pub struct TerminalApp {
        window: Arc<winit::window::Window>,
        renderer: Renderer,
        screen: Screen,
        parser: Parser,
        pty: Option<Pty>,
        modifiers: Modifiers,
        mouse_position: (f64, f64),
    }

    impl TerminalApp {
        pub fn new(event_loop: &EventLoop<()>) -> Result<Self, String> {
            let window = Arc::new(
                WindowBuilder::new()
                    .with_title("Mochi Terminal")
                    .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
                    .build(event_loop)
                    .map_err(|e| format!("Failed to create window: {}", e))?,
            );

            let config = RendererConfig::default();
            let renderer = pollster::block_on(Renderer::new(window.clone(), config))
                .map_err(|e| format!("Failed to create renderer: {}", e))?;

            let (cols, rows) = renderer.terminal_size();
            let screen = Screen::new(cols as usize, rows as usize);

            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
            let pty = match Pty::spawn(&shell, &[], WindowSize::new(cols, rows)) {
                Ok(pty) => {
                    tracing::info!("Spawned shell: {}", shell);
                    Some(pty)
                },
                Err(e) => {
                    tracing::error!("Failed to spawn shell: {}", e);
                    None
                },
            };

            Ok(Self {
                window,
                renderer,
                screen,
                parser: Parser::new(),
                pty,
                modifiers: Modifiers::default(),
                mouse_position: (0.0, 0.0),
            })
        }

        pub fn process_pty_output(&mut self) {
            if let Some(ref pty) = self.pty {
                let mut buf = [0u8; 4096];
                while pty.poll_read(0).unwrap_or(false) {
                    match pty.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let actions = self.parser.feed(&buf[..n]);
                            for action in actions {
                                self.screen.apply(action);
                            }
                        },
                        Err(_) => break,
                    }
                }
            }
        }

        pub fn send_to_pty(&self, data: &[u8]) {
            if let Some(ref pty) = self.pty {
                let _ = pty.write_all(data);
            }
        }

        pub fn handle_key(&mut self, key: Key, state: ElementState) {
            if state != ElementState::Pressed {
                return;
            }

            let key_code = match key {
                Key::Named(NamedKey::ArrowUp) => Some(KeyCode::Up),
                Key::Named(NamedKey::ArrowDown) => Some(KeyCode::Down),
                Key::Named(NamedKey::ArrowLeft) => Some(KeyCode::Left),
                Key::Named(NamedKey::ArrowRight) => Some(KeyCode::Right),
                Key::Named(NamedKey::Home) => Some(KeyCode::Home),
                Key::Named(NamedKey::End) => Some(KeyCode::End),
                Key::Named(NamedKey::PageUp) => Some(KeyCode::PageUp),
                Key::Named(NamedKey::PageDown) => Some(KeyCode::PageDown),
                Key::Named(NamedKey::Insert) => Some(KeyCode::Insert),
                Key::Named(NamedKey::Delete) => Some(KeyCode::Delete),
                Key::Named(NamedKey::Escape) => Some(KeyCode::Escape),
                Key::Named(NamedKey::Tab) => Some(KeyCode::Tab),
                Key::Named(NamedKey::Backspace) => Some(KeyCode::Backspace),
                Key::Named(NamedKey::Enter) => Some(KeyCode::Enter),
                Key::Named(NamedKey::F1) => Some(KeyCode::F1),
                Key::Named(NamedKey::F2) => Some(KeyCode::F2),
                Key::Named(NamedKey::F3) => Some(KeyCode::F3),
                Key::Named(NamedKey::F4) => Some(KeyCode::F4),
                Key::Named(NamedKey::F5) => Some(KeyCode::F5),
                Key::Named(NamedKey::F6) => Some(KeyCode::F6),
                Key::Named(NamedKey::F7) => Some(KeyCode::F7),
                Key::Named(NamedKey::F8) => Some(KeyCode::F8),
                Key::Named(NamedKey::F9) => Some(KeyCode::F9),
                Key::Named(NamedKey::F10) => Some(KeyCode::F10),
                Key::Named(NamedKey::F11) => Some(KeyCode::F11),
                Key::Named(NamedKey::F12) => Some(KeyCode::F12),
                Key::Character(ref s) => s.chars().next().map(KeyCode::Char),
                _ => None,
            };

            if let Some(key) = key_code {
                let modes = self.screen.modes();
                if let Some(seq) = encode_key(
                    key,
                    self.modifiers,
                    modes.cursor_keys_application,
                    modes.keypad_application,
                ) {
                    self.send_to_pty(seq.as_bytes());
                }
            }
        }

        pub fn handle_mouse_button(&mut self, button: WinitMouseButton, state: ElementState) {
            let modes = self.screen.modes();
            if modes.mouse_mode == MouseMode::None {
                return;
            }

            let mouse_button = match button {
                WinitMouseButton::Left => MouseButton::Left,
                WinitMouseButton::Right => MouseButton::Right,
                WinitMouseButton::Middle => MouseButton::Middle,
                _ => return,
            };

            let event_type = match state {
                ElementState::Pressed => MouseEventType::Press,
                ElementState::Released => MouseEventType::Release,
            };

            let (cell_width, cell_height) = self.renderer.cell_size();
            let col = (self.mouse_position.0 / cell_width as f64) as u16 + 1;
            let row = (self.mouse_position.1 / cell_height as f64) as u16 + 1;

            let encoding = match modes.mouse_encoding {
                CoreMouseEncoding::Sgr => MouseEncoding::Sgr,
                _ => MouseEncoding::X10,
            };

            let seq = mochi_terminal::frontend::encode_mouse(
                event_type,
                mouse_button,
                col,
                row,
                self.modifiers,
                encoding,
            );
            self.send_to_pty(seq.as_bytes());
        }

        pub fn handle_scroll(&mut self, delta_y: f32) {
            let modes = self.screen.modes();
            if modes.mouse_mode == MouseMode::None {
                // No mouse tracking - could scroll viewport here if we had scrollback
                // For now, just ignore scroll when not in mouse mode
                return;
            }

            let button = if delta_y > 0.0 {
                MouseButton::WheelUp
            } else {
                MouseButton::WheelDown
            };

            let (cell_width, cell_height) = self.renderer.cell_size();
            let col = (self.mouse_position.0 / cell_width as f64) as u16 + 1;
            let row = (self.mouse_position.1 / cell_height as f64) as u16 + 1;

            let encoding = match modes.mouse_encoding {
                CoreMouseEncoding::Sgr => MouseEncoding::Sgr,
                _ => MouseEncoding::X10,
            };

            let seq = mochi_terminal::frontend::encode_mouse(
                MouseEventType::Press,
                button,
                col,
                row,
                self.modifiers,
                encoding,
            );
            self.send_to_pty(seq.as_bytes());
        }

        pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
            self.renderer.resize((size.width, size.height));
            let (cols, rows) = self.renderer.terminal_size();
            self.screen.resize(cols as usize, rows as usize);

            if let Some(ref pty) = self.pty {
                let _ = pty.resize(WindowSize::new(cols, rows));
            }
        }

        pub fn render(&mut self) -> Result<(), String> {
            self.renderer.render(&self.screen)
        }

        pub fn request_redraw(&self) {
            self.window.request_redraw();
        }

        pub fn is_pty_alive(&mut self) -> bool {
            self.pty.as_mut().map(|p| p.is_alive()).unwrap_or(false)
        }

        pub fn set_modifiers(&mut self, mods: winit::event::Modifiers) {
            self.modifiers = Modifiers {
                shift: mods.state().shift_key(),
                ctrl: mods.state().control_key(),
                alt: mods.state().alt_key(),
                logo: mods.state().super_key(),
            };
        }

        pub fn set_mouse_position(&mut self, x: f64, y: f64) {
            self.mouse_position = (x, y);
        }
    }

    pub fn run() -> Result<(), String> {
        let event_loop =
            EventLoop::new().map_err(|e| format!("Failed to create event loop: {}", e))?;
        let mut app = TerminalApp::new(&event_loop)?;

        event_loop
            .run(move |event, elwt| match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        tracing::info!("Window close requested");
                        elwt.exit();
                    },

                    WindowEvent::Resized(size) => {
                        app.resize(size);
                    },

                    WindowEvent::ModifiersChanged(mods) => {
                        app.set_modifiers(mods);
                    },

                    WindowEvent::KeyboardInput { event, .. } => {
                        app.handle_key(event.logical_key, event.state);
                    },

                    WindowEvent::MouseInput { state, button, .. } => {
                        app.handle_mouse_button(button, state);
                    },

                    WindowEvent::CursorMoved { position, .. } => {
                        app.set_mouse_position(position.x, position.y);
                    },

                    WindowEvent::MouseWheel { delta, .. } => {
                        let delta_y = match delta {
                            winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                            winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 20.0,
                        };
                        app.handle_scroll(delta_y);
                    },

                    WindowEvent::RedrawRequested => {
                        app.process_pty_output();

                        if !app.is_pty_alive() {
                            tracing::info!("Shell exited");
                            elwt.exit();
                            return;
                        }

                        if let Err(e) = app.render() {
                            tracing::error!("Render error: {}", e);
                        }
                    },

                    _ => {},
                },

                Event::AboutToWait => {
                    app.process_pty_output();
                    app.request_redraw();
                },

                _ => {},
            })
            .map_err(|e| format!("Event loop error: {}", e))
    }
}

fn main() -> ExitCode {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Mochi Terminal starting...");

    #[cfg(feature = "gui")]
    {
        if let Err(e) = gui::run() {
            tracing::error!("Error: {}", e);
            return ExitCode::FAILURE;
        }
        ExitCode::SUCCESS
    }

    #[cfg(not(feature = "gui"))]
    {
        println!("Mochi Terminal v{}", env!("CARGO_PKG_VERSION"));
        println!();
        println!("GUI terminal emulator requires the 'gui' feature.");
        println!("Build with: cargo build --features gui");
        println!();
        println!("Use mochi-headless for testing the terminal core.");
        println!("Use mochi-pty-test for testing PTY functionality.");

        ExitCode::SUCCESS
    }
}
