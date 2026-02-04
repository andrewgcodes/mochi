//! Event loop for the terminal application
//!
//! This module handles the main event loop using winit for window management.
//! It coordinates:
//! - Window events (resize, close, focus)
//! - Keyboard and mouse input
//! - PTY I/O
//! - Rendering

use std::io;
use std::time::{Duration, Instant};

use log::{error, info, warn};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{
    ElementState, Event, ModifiersState, MouseButton as WinitMouseButton,
    MouseScrollDelta, VirtualKeyCode, WindowEvent,
};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use crate::input::{self, Key, Modifiers, MouseButton, MouseEvent};
use crate::renderer::SoftwareRenderer;
use crate::App;

/// Run the main event loop
pub fn run_event_loop(mut app: App) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new();

    let cell_width = 9.0f32;
    let cell_height = 18.0f32;
    let width = app.config.cols as f32 * cell_width;
    let height = app.config.rows as f32 * cell_height;

    let window = WindowBuilder::new()
        .with_title(&app.title)
        .with_inner_size(LogicalSize::new(width, height))
        .with_min_inner_size(LogicalSize::new(200.0, 100.0))
        .build(&event_loop)?;

    let renderer_result = SoftwareRenderer::new(&app.config);
    let mut renderer = match renderer_result {
        Ok(r) => Some(r),
        Err(e) => {
            warn!("Failed to create renderer: {}. Running in headless mode.", e);
            None
        }
    };

    if let Some(ref child) = app.child {
        child.set_nonblocking(true)?;
    }

    let mut modifiers = ModifiersState::empty();
    let mut last_render = Instant::now();
    let render_interval = Duration::from_millis(16);

    let mut pty_buffer = [0u8; 65536];

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        info!("Window close requested");
                        *control_flow = ControlFlow::Exit;
                    }

                    WindowEvent::Resized(size) => {
                        handle_resize(&mut app, &mut renderer, size);
                        window.request_redraw();
                    }

                    WindowEvent::ModifiersChanged(mods) => {
                        modifiers = mods;
                    }

                    WindowEvent::KeyboardInput { input, .. } => {
                        if input.state == ElementState::Pressed {
                            handle_keyboard(&mut app, input.virtual_keycode, modifiers);
                        }
                    }

                    WindowEvent::ReceivedCharacter(c) => {
                        if !c.is_control() {
                            let bytes = input::encode_key(
                                Key::Char(c),
                                Modifiers {
                                    shift: modifiers.shift(),
                                    ctrl: modifiers.ctrl(),
                                    alt: modifiers.alt(),
                                },
                                app.term.mode.app_cursor,
                                app.term.mode.app_keypad,
                            );
                            if let Err(e) = app.send_input(&bytes) {
                                error!("Failed to send input: {}", e);
                            }
                        }
                    }

                    WindowEvent::MouseInput { state, button, .. } => {
                        handle_mouse_button(&mut app, state, button, modifiers);
                    }

                    WindowEvent::MouseWheel { delta, .. } => {
                        handle_mouse_wheel(&mut app, delta, modifiers);
                    }

                    WindowEvent::CursorMoved { .. } => {
                    }

                    WindowEvent::Focused(focused) => {
                        if app.term.screen_mode().focus_reporting {
                            let bytes = input::encode_focus(focused);
                            let _ = app.send_input(&bytes);
                        }
                    }

                    _ => {}
                }
            }

            Event::RedrawRequested(_) => {
                if let Some(ref mut renderer) = renderer {
                    renderer.render(&app.term);
                }
                app.dirty = false;
            }

            Event::MainEventsCleared => {
                if let Some(ref mut child) = app.child {
                    match child.read(&mut pty_buffer) {
                        Ok(0) => {
                            info!("Child process exited");
                            *control_flow = ControlFlow::Exit;
                        }
                        Ok(n) => {
                            app.process_pty_input(&pty_buffer[..n]);
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        }
                        Err(e) => {
                            error!("PTY read error: {}", e);
                        }
                    }
                }

                if let Some(code) = app.check_child() {
                    info!("Child exited with code {}", code);
                    *control_flow = ControlFlow::Exit;
                }

                let now = Instant::now();
                if app.dirty && now.duration_since(last_render) >= render_interval {
                    window.request_redraw();
                    last_render = now;
                }

                if app.title != app.term.title && !app.term.title.is_empty() {
                    app.title = app.term.title.clone();
                    window.set_title(&app.title);
                }
            }

            _ => {}
        }
    });
}

fn handle_resize(app: &mut App, renderer: &mut Option<SoftwareRenderer>, size: PhysicalSize<u32>) {
    if size.width == 0 || size.height == 0 {
        return;
    }

    if let Some(ref renderer) = renderer {
        let (rows, cols) = renderer.calc_dimensions(size.width, size.height);
        if rows != app.term.rows() || cols != app.term.cols() {
            if let Err(e) = app.resize(rows, cols) {
                error!("Failed to resize terminal: {}", e);
            }
        }
    }

    if let Some(ref mut renderer) = renderer {
        renderer.resize(size.width, size.height);
    }
}

fn handle_keyboard(app: &mut App, keycode: Option<VirtualKeyCode>, modifiers: ModifiersState) {
    let keycode = match keycode {
        Some(k) => k,
        None => return,
    };

    let mods = Modifiers {
        shift: modifiers.shift(),
        ctrl: modifiers.ctrl(),
        alt: modifiers.alt(),
    };

    let key = match keycode {
        VirtualKeyCode::Return => Key::Enter,
        VirtualKeyCode::Tab => Key::Tab,
        VirtualKeyCode::Back => Key::Backspace,
        VirtualKeyCode::Escape => Key::Escape,
        VirtualKeyCode::Up => Key::Up,
        VirtualKeyCode::Down => Key::Down,
        VirtualKeyCode::Left => Key::Left,
        VirtualKeyCode::Right => Key::Right,
        VirtualKeyCode::Home => Key::Home,
        VirtualKeyCode::End => Key::End,
        VirtualKeyCode::PageUp => Key::PageUp,
        VirtualKeyCode::PageDown => Key::PageDown,
        VirtualKeyCode::Insert => Key::Insert,
        VirtualKeyCode::Delete => Key::Delete,
        VirtualKeyCode::F1 => Key::F(1),
        VirtualKeyCode::F2 => Key::F(2),
        VirtualKeyCode::F3 => Key::F(3),
        VirtualKeyCode::F4 => Key::F(4),
        VirtualKeyCode::F5 => Key::F(5),
        VirtualKeyCode::F6 => Key::F(6),
        VirtualKeyCode::F7 => Key::F(7),
        VirtualKeyCode::F8 => Key::F(8),
        VirtualKeyCode::F9 => Key::F(9),
        VirtualKeyCode::F10 => Key::F(10),
        VirtualKeyCode::F11 => Key::F(11),
        VirtualKeyCode::F12 => Key::F(12),
        VirtualKeyCode::A if mods.ctrl => Key::Char('\x01'),
        VirtualKeyCode::B if mods.ctrl => Key::Char('\x02'),
        VirtualKeyCode::C if mods.ctrl => Key::Char('\x03'),
        VirtualKeyCode::D if mods.ctrl => Key::Char('\x04'),
        VirtualKeyCode::E if mods.ctrl => Key::Char('\x05'),
        VirtualKeyCode::F if mods.ctrl => Key::Char('\x06'),
        VirtualKeyCode::G if mods.ctrl => Key::Char('\x07'),
        VirtualKeyCode::H if mods.ctrl => Key::Char('\x08'),
        VirtualKeyCode::I if mods.ctrl => Key::Char('\x09'),
        VirtualKeyCode::J if mods.ctrl => Key::Char('\x0a'),
        VirtualKeyCode::K if mods.ctrl => Key::Char('\x0b'),
        VirtualKeyCode::L if mods.ctrl => Key::Char('\x0c'),
        VirtualKeyCode::M if mods.ctrl => Key::Char('\x0d'),
        VirtualKeyCode::N if mods.ctrl => Key::Char('\x0e'),
        VirtualKeyCode::O if mods.ctrl => Key::Char('\x0f'),
        VirtualKeyCode::P if mods.ctrl => Key::Char('\x10'),
        VirtualKeyCode::Q if mods.ctrl => Key::Char('\x11'),
        VirtualKeyCode::R if mods.ctrl => Key::Char('\x12'),
        VirtualKeyCode::S if mods.ctrl => Key::Char('\x13'),
        VirtualKeyCode::T if mods.ctrl => Key::Char('\x14'),
        VirtualKeyCode::U if mods.ctrl => Key::Char('\x15'),
        VirtualKeyCode::V if mods.ctrl => Key::Char('\x16'),
        VirtualKeyCode::W if mods.ctrl => Key::Char('\x17'),
        VirtualKeyCode::X if mods.ctrl => Key::Char('\x18'),
        VirtualKeyCode::Y if mods.ctrl => Key::Char('\x19'),
        VirtualKeyCode::Z if mods.ctrl => Key::Char('\x1a'),
        _ => return,
    };

    let bytes = input::encode_key(
        key,
        mods,
        app.term.mode.app_cursor,
        app.term.mode.app_keypad,
    );

    if let Err(e) = app.send_input(&bytes) {
        error!("Failed to send input: {}", e);
    }
}

fn handle_mouse_button(
    app: &mut App,
    state: ElementState,
    button: WinitMouseButton,
    _modifiers: ModifiersState,
) {
    let mouse_mode = app.term.mouse_mode();
    if mouse_mode == mochi_core::screen::MouseMode::None {
        return;
    }

    let btn = match button {
        WinitMouseButton::Left => MouseButton::Left,
        WinitMouseButton::Middle => MouseButton::Middle,
        WinitMouseButton::Right => MouseButton::Right,
        _ => return,
    };

    let event = match state {
        ElementState::Pressed => MouseEvent::Press(btn),
        ElementState::Released => MouseEvent::Release(btn),
    };

    let x = 0u16;
    let y = 0u16;

    let bytes = input::encode_mouse(
        event,
        x,
        y,
        mouse_mode,
        app.term.mouse_encoding(),
    );

    if !bytes.is_empty() {
        let _ = app.send_input(&bytes);
    }
}

fn handle_mouse_wheel(app: &mut App, delta: MouseScrollDelta, _modifiers: ModifiersState) {
    let mouse_mode = app.term.mouse_mode();

    let lines = match delta {
        MouseScrollDelta::LineDelta(_, y) => y as i32,
        MouseScrollDelta::PixelDelta(pos) => (pos.y / 20.0) as i32,
    };

    if lines == 0 {
        return;
    }

    if mouse_mode != mochi_core::screen::MouseMode::None {
        let btn = if lines > 0 {
            MouseButton::WheelUp
        } else {
            MouseButton::WheelDown
        };

        let x = 0u16;
        let y = 0u16;

        for _ in 0..lines.abs() {
            let bytes = input::encode_mouse(
                MouseEvent::Press(btn),
                x,
                y,
                mouse_mode,
                app.term.mouse_encoding(),
            );
            if !bytes.is_empty() {
                let _ = app.send_input(&bytes);
            }
        }
    }
}

/// Headless event loop for testing
pub fn run_headless(mut app: App, input: &[u8]) -> io::Result<String> {
    app.process_pty_input(input);
    Ok(app.term.snapshot().text())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_headless() {
        let config = Config::default();
        if let Ok(app) = App::new(config) {
            let result = run_headless(app, b"Hello\r\n");
            assert!(result.is_ok());
        }
    }
}
