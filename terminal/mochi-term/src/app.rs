//! Main application logic
//!
//! Ties together the terminal, PTY, and renderer.

use std::io;
use std::rc::Rc;
use std::time::Instant;

use arboard::Clipboard;
use terminal_pty::{Child, WindowSize};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowBuilder};

use terminal_core::{Point, SelectionType};

use crate::config::{Config, ThemeName};
use crate::input::{encode_bracketed_paste, encode_focus, encode_key, encode_mouse, MouseEvent};
use crate::renderer::Renderer;
use crate::terminal::Terminal;

/// Actions that can be triggered by keybindings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Copy,
    Paste,
    ToggleTheme,
    ReloadConfig,
    ZoomIn,
    ZoomOut,
    ZoomReset,
}

/// Application state
pub struct App {
    /// Configuration
    config: Config,
    /// Window (created on resume)
    window: Option<Rc<Window>>,
    /// Renderer
    renderer: Option<Renderer>,
    /// Terminal state
    terminal: Option<Terminal>,
    /// Child process
    child: Option<Child>,
    /// Clipboard
    clipboard: Option<Clipboard>,
    /// Current modifiers state
    modifiers: ModifiersState,
    /// Mouse position (in cells)
    mouse_cell: (u16, u16),
    /// Mouse button state
    mouse_buttons: [bool; 3],
    /// Last render time
    last_render: Instant,
    /// Needs redraw
    needs_redraw: bool,
    /// Is focused
    focused: bool,
    /// Scroll offset (number of lines scrolled back into history)
    scroll_offset: usize,
    /// Last click time (for double/triple click detection)
    last_click_time: Instant,
    /// Click count (1=single, 2=double, 3=triple)
    click_count: u8,
    /// Last click position (for detecting clicks in same location)
    last_click_pos: (u16, u16),
    /// Whether we're currently dragging a selection
    is_selecting: bool,
}

impl App {
    /// Create a new application
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            config,
            window: None,
            renderer: None,
            terminal: None,
            child: None,
            clipboard: Clipboard::new().ok(),
            modifiers: ModifiersState::empty(),
            mouse_cell: (0, 0),
            mouse_buttons: [false; 3],
            last_render: Instant::now(),
            needs_redraw: true,
            focused: true,
            scroll_offset: 0,
            last_click_time: Instant::now(),
            click_count: 0,
            last_click_pos: (0, 0),
            is_selecting: false,
        })
    }

    /// Run the application
    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new()?;

        // Create window
        let window = WindowBuilder::new()
            .with_title("Mochi Terminal")
            .with_inner_size(LogicalSize::new(800, 600))
            .build(&event_loop)?;

        let window = Rc::new(window);

        // Initialize graphics
        self.init_graphics(window.clone())?;

        // Run event loop
        event_loop.run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, .. } => {
                    self.handle_window_event(event, elwt);
                }
                Event::AboutToWait => {
                    // Poll PTY
                    self.poll_pty();

                    // Check if child exited
                    if !self.check_child() {
                        log::info!("Child process exited");
                        elwt.exit();
                        return;
                    }

                    // Request redraw if needed
                    if self.needs_redraw {
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
                _ => {}
            }
        })?;

        Ok(())
    }

    /// Handle window events
    fn handle_window_event(
        &mut self,
        event: WindowEvent,
        elwt: &winit::event_loop::EventLoopWindowTarget<()>,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                elwt.exit();
            }
            WindowEvent::Resized(size) => {
                self.handle_resize(size);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_key_input(&event);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::MouseInput { button, state, .. } => {
                self.handle_mouse_input(button, state);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.handle_mouse_motion(position);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_mouse_scroll(delta);
            }
            WindowEvent::Focused(focused) => {
                self.handle_focus(focused);
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            _ => {}
        }
    }

    /// Initialize graphics
    fn init_graphics(&mut self, window: Rc<Window>) -> Result<(), Box<dyn std::error::Error>> {
        let size = window.inner_size();

        // Create renderer with effective colors based on theme
        let renderer = Renderer::new(
            window.clone(),
            self.config.font_size,
            self.config.effective_colors(),
        )?;

        // Calculate terminal dimensions
        let cell_size = renderer.cell_size();
        let cols = (size.width as f32 / cell_size.width) as usize;
        let rows = (size.height as f32 / cell_size.height) as usize;

        // Create terminal
        let terminal = Terminal::new(cols.max(1), rows.max(1));

        // Spawn shell
        let child = Child::spawn_shell(WindowSize::new(cols as u16, rows as u16))?;
        child.set_nonblocking(true)?;

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.terminal = Some(terminal);
        self.child = Some(child);

        Ok(())
    }

    /// Handle window resize
    fn handle_resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let Some(terminal) = &mut self.terminal else {
            return;
        };
        let Some(child) = &self.child else { return };

        // Update renderer
        renderer.resize(size.width, size.height);

        // Calculate new terminal dimensions
        let cell_size = renderer.cell_size();
        let cols = (size.width as f32 / cell_size.width) as usize;
        let rows = (size.height as f32 / cell_size.height) as usize;

        if cols > 0 && rows > 0 {
            terminal.resize(cols, rows);
            let _ = child.resize(WindowSize::new(cols as u16, rows as u16));
        }

        self.needs_redraw = true;
    }

    /// Handle keyboard input
    fn handle_key_input(&mut self, event: &winit::event::KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }

        if let Some(action) = self.check_keybinding(event) {
            self.execute_action(action);
            return;
        }

        let Some(terminal) = &self.terminal else {
            return;
        };
        let Some(child) = &mut self.child else { return };

        let application_cursor_keys = terminal.screen().modes().cursor_keys_application;

        if let Some(data) = encode_key(&event.logical_key, self.modifiers, application_cursor_keys)
        {
            let _ = child.write_all(&data);
        }
    }

    /// Check if a key event matches a keybinding
    fn check_keybinding(&self, event: &winit::event::KeyEvent) -> Option<KeyAction> {
        let ctrl = self.modifiers.control_key();
        let shift = self.modifiers.shift_key();
        #[cfg(target_os = "macos")]
        let cmd = self.modifiers.super_key();
        #[cfg(not(target_os = "macos"))]
        let cmd = false;

        match &event.logical_key {
            Key::Character(c) => {
                let c = c.to_lowercase();
                if ctrl && shift {
                    match c.as_str() {
                        "c" => return Some(KeyAction::Copy),
                        "v" => return Some(KeyAction::Paste),
                        "t" => return Some(KeyAction::ToggleTheme),
                        "r" => return Some(KeyAction::ReloadConfig),
                        _ => {}
                    }
                }
                #[cfg(target_os = "macos")]
                if cmd {
                    match c.as_str() {
                        "c" => return Some(KeyAction::Copy),
                        "v" => return Some(KeyAction::Paste),
                        "=" | "+" => return Some(KeyAction::ZoomIn),
                        "-" => return Some(KeyAction::ZoomOut),
                        "0" => return Some(KeyAction::ZoomReset),
                        _ => {}
                    }
                }
                #[cfg(not(target_os = "macos"))]
                if ctrl && !shift {
                    match c.as_str() {
                        "=" | "+" => return Some(KeyAction::ZoomIn),
                        "-" => return Some(KeyAction::ZoomOut),
                        "0" => return Some(KeyAction::ZoomReset),
                        _ => {}
                    }
                }
                if cmd || (ctrl && !shift) {
                    match c.as_str() {
                        "=" | "+" => return Some(KeyAction::ZoomIn),
                        "-" => return Some(KeyAction::ZoomOut),
                        "0" => return Some(KeyAction::ZoomReset),
                        _ => {}
                    }
                }
            }
            Key::Named(NamedKey::ArrowUp) if ctrl => {
                return Some(KeyAction::ZoomIn);
            }
            Key::Named(NamedKey::ArrowDown) if ctrl => {
                return Some(KeyAction::ZoomOut);
            }
            _ => {}
        }

        None
    }

    /// Execute a keybinding action
    fn execute_action(&mut self, action: KeyAction) {
        match action {
            KeyAction::Copy => self.handle_copy(),
            KeyAction::Paste => self.handle_paste(),
            KeyAction::ToggleTheme => self.toggle_theme(),
            KeyAction::ReloadConfig => self.reload_config(),
            KeyAction::ZoomIn => self.change_font_size(2.0),
            KeyAction::ZoomOut => self.change_font_size(-2.0),
            KeyAction::ZoomReset => self.reset_font_size(),
        }
    }

    /// Handle copy (Ctrl+Shift+C)
    fn handle_copy(&mut self) {
        let Some(clipboard) = &mut self.clipboard else {
            return;
        };
        let Some(terminal) = &self.terminal else {
            return;
        };

        let screen = terminal.screen();
        let selection = screen.selection();

        if selection.is_empty() {
            return;
        }

        let text = screen.get_selected_text(selection);
        if !text.is_empty() {
            if let Err(e) = clipboard.set_text(&text) {
                log::warn!("Failed to copy to clipboard: {}", e);
            } else {
                log::debug!("Copied {} chars to clipboard", text.len());
            }
        }
    }

    /// Toggle between themes
    fn toggle_theme(&mut self) {
        let next_theme = match self.config.theme {
            ThemeName::Dark => ThemeName::Light,
            ThemeName::Light => ThemeName::SolarizedDark,
            ThemeName::SolarizedDark => ThemeName::SolarizedLight,
            ThemeName::SolarizedLight => ThemeName::Dracula,
            ThemeName::Dracula => ThemeName::Nord,
            ThemeName::Nord => ThemeName::Dark,
            ThemeName::Custom => ThemeName::Dark,
        };

        self.config.theme = next_theme;
        let new_colors = self.config.effective_colors();

        if let Some(renderer) = &mut self.renderer {
            renderer.set_colors(new_colors);
        }

        log::info!("Switched to theme: {:?}", next_theme);
        self.needs_redraw = true;
    }

    /// Reload configuration from file
    fn reload_config(&mut self) {
        match self.config.reload() {
            Ok(()) => {
                let new_colors = self.config.effective_colors();
                if let Some(renderer) = &mut self.renderer {
                    renderer.set_colors(new_colors);
                }
                log::info!("Configuration reloaded successfully");
                self.needs_redraw = true;
            }
            Err(e) => {
                log::error!("Failed to reload configuration: {}", e);
            }
        }
    }

    /// Change font size by delta
    fn change_font_size(&mut self, delta: f32) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let Some(terminal) = &mut self.terminal else {
            return;
        };
        let Some(child) = &self.child else { return };
        let Some(window) = &self.window else { return };

        let current_size = renderer.font_size();
        let new_size = (current_size + delta).clamp(8.0, 72.0);

        if (new_size - current_size).abs() < 0.1 {
            return;
        }

        renderer.set_font_size(new_size);

        // Recalculate terminal dimensions
        let size = window.inner_size();
        let cell_size = renderer.cell_size();
        let cols = (size.width as f32 / cell_size.width) as usize;
        let rows = (size.height as f32 / cell_size.height) as usize;

        if cols > 0 && rows > 0 {
            terminal.resize(cols, rows);
            let _ = child.resize(WindowSize::new(cols as u16, rows as u16));
        }

        self.needs_redraw = true;
    }

    /// Reset font size to default (scaled for HiDPI)
    fn reset_font_size(&mut self) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let Some(terminal) = &mut self.terminal else {
            return;
        };
        let Some(child) = &self.child else { return };
        let Some(window) = &self.window else { return };

        let scale_factor = window.scale_factor() as f32;
        let default_size = self.config.font_size * scale_factor;

        renderer.set_font_size(default_size);

        // Recalculate terminal dimensions
        let size = window.inner_size();
        let cell_size = renderer.cell_size();
        let cols = (size.width as f32 / cell_size.width) as usize;
        let rows = (size.height as f32 / cell_size.height) as usize;

        if cols > 0 && rows > 0 {
            terminal.resize(cols, rows);
            let _ = child.resize(WindowSize::new(cols as u16, rows as u16));
        }

        self.needs_redraw = true;
    }

    /// Handle mouse input
    fn handle_mouse_input(&mut self, button: MouseButton, state: ElementState) {
        // Handle left button for selection
        if button == MouseButton::Left {
            if state == ElementState::Pressed {
                let now = Instant::now();
                let double_click_threshold = std::time::Duration::from_millis(500);

                // Check if this is a multi-click (same position, within time threshold)
                let is_same_pos = self.mouse_cell == self.last_click_pos;
                let is_quick_click = now.duration_since(self.last_click_time) < double_click_threshold;

                if is_same_pos && is_quick_click {
                    self.click_count = (self.click_count % 3) + 1;
                } else {
                    self.click_count = 1;
                }

                self.last_click_time = now;
                self.last_click_pos = self.mouse_cell;

                // Determine selection type based on click count
                let selection_type = match self.click_count {
                    2 => SelectionType::Word,
                    3 => SelectionType::Line,
                    _ => SelectionType::Normal,
                };

                // Start selection
                if let Some(terminal) = &mut self.terminal {
                    let point = Point::new(
                        self.mouse_cell.0 as usize,
                        self.mouse_cell.1 as isize - self.scroll_offset as isize,
                    );
                    terminal.screen_mut().selection_mut().start(point, selection_type);
                }

                // For word/line selection, expand to word/line boundaries
                if selection_type == SelectionType::Word {
                    self.expand_word_selection();
                } else if selection_type == SelectionType::Line {
                    self.expand_line_selection();
                }

                self.is_selecting = true;
                self.needs_redraw = true;
            } else {
                // Mouse released - finish selection
                self.is_selecting = false;
            }
        }

        // Send mouse events to PTY if mouse tracking is enabled
        let Some(terminal) = &self.terminal else {
            return;
        };
        let modes = terminal.screen().modes();

        if modes.mouse_tracking_enabled() {
            let Some(child) = &mut self.child else { return };

            let event = if state == ElementState::Pressed {
                MouseEvent::Press(button, self.mouse_cell.0, self.mouse_cell.1)
            } else {
                MouseEvent::Release(button, self.mouse_cell.0, self.mouse_cell.1)
            };

            if let Some(data) = encode_mouse(
                event,
                modes.mouse_sgr,
                modes.mouse_button_event,
                modes.mouse_any_event,
            ) {
                let _ = child.write_all(&data);
            }
        }

        // Track button state
        let idx = match button {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
            _ => return,
        };
        self.mouse_buttons[idx] = state == ElementState::Pressed;
    }

    /// Expand selection to word boundaries
    fn expand_word_selection(&mut self) {
        let Some(terminal) = &mut self.terminal else {
            return;
        };

        let screen = terminal.screen();
        let col = self.mouse_cell.0 as usize;
        let row = self.mouse_cell.1 as usize;

        if row >= screen.rows() {
            return;
        }

        let line = screen.line(row);
        let line_cols = line.cols();

        // Find word boundaries
        let mut start_col = col;
        let mut end_col = col;

        // Expand left
        while start_col > 0 {
            let c = line.cell(start_col - 1).display_char();
            if !is_word_char(c) {
                break;
            }
            start_col -= 1;
        }

        // Expand right
        while end_col < line_cols.saturating_sub(1) {
            let c = line.cell(end_col + 1).display_char();
            if !is_word_char(c) {
                break;
            }
            end_col += 1;
        }

        // Update selection bounds
        let selection = terminal.screen_mut().selection_mut();
        let row_offset = row as isize - self.scroll_offset as isize;
        selection.start = Point::new(start_col, row_offset);
        selection.end = Point::new(end_col, row_offset);
    }

    /// Expand selection to line boundaries
    fn expand_line_selection(&mut self) {
        let Some(terminal) = &mut self.terminal else {
            return;
        };

        let screen = terminal.screen();
        let row = self.mouse_cell.1 as usize;

        if row >= screen.rows() {
            return;
        }

        let line = screen.line(row);
        let line_cols = line.cols();

        // Select entire line
        let selection = terminal.screen_mut().selection_mut();
        let row_offset = row as isize - self.scroll_offset as isize;
        selection.start = Point::new(0, row_offset);
        selection.end = Point::new(line_cols.saturating_sub(1), row_offset);
    }

    /// Handle mouse motion
    fn handle_mouse_motion(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        let Some(renderer) = &self.renderer else {
            return;
        };

        let cell_size = renderer.cell_size();
        let col = (position.x / cell_size.width as f64) as u16;
        let row = (position.y / cell_size.height as f64) as u16;

        if col == self.mouse_cell.0 && row == self.mouse_cell.1 {
            return;
        }

        self.mouse_cell = (col, row);

        // Update selection if we're dragging
        if self.is_selecting {
            let Some(terminal) = &mut self.terminal else {
                return;
            };

            let point = Point::new(
                col as usize,
                row as isize - self.scroll_offset as isize,
            );
            terminal.screen_mut().selection_mut().update(point);
            self.needs_redraw = true;
        }

        // Send mouse events to PTY if mouse tracking is enabled
        let Some(terminal) = &self.terminal else {
            return;
        };
        let modes = terminal.screen().modes();

        if modes.mouse_any_event
            || (modes.mouse_button_event && self.mouse_buttons.iter().any(|&b| b))
        {
            let Some(child) = &mut self.child else { return };
            let event = MouseEvent::Move(col, row);
            if let Some(data) = encode_mouse(
                event,
                modes.mouse_sgr,
                modes.mouse_button_event,
                modes.mouse_any_event,
            ) {
                let _ = child.write_all(&data);
            }
        }
    }

    /// Handle mouse scroll
    fn handle_mouse_scroll(&mut self, delta: MouseScrollDelta) {
        let Some(terminal) = &self.terminal else {
            return;
        };

        let modes = terminal.screen().modes();
        let lines = match delta {
            MouseScrollDelta::LineDelta(_, y) => y as i32,
            MouseScrollDelta::PixelDelta(pos) => (pos.y / 20.0) as i32,
        };

        if lines == 0 {
            return;
        }

        // If mouse tracking is enabled or in alternate screen, send to PTY
        if modes.mouse_tracking_enabled() || modes.alternate_screen {
            let Some(child) = &mut self.child else { return };
            let event = MouseEvent::Scroll {
                x: self.mouse_cell.0,
                y: self.mouse_cell.1,
                delta: lines as i8,
            };
            if let Some(data) = encode_mouse(
                event,
                modes.mouse_sgr,
                modes.mouse_button_event,
                modes.mouse_any_event,
            ) {
                let _ = child.write_all(&data);
            }
        } else {
            // Scroll the viewport through scrollback history
            let scrollback_len = terminal.screen().scrollback().len();
            if lines > 0 {
                // Scroll up (show older content)
                self.scroll_offset = (self.scroll_offset + lines as usize).min(scrollback_len);
            } else {
                // Scroll down (show newer content)
                self.scroll_offset = self.scroll_offset.saturating_sub((-lines) as usize);
            }
            self.needs_redraw = true;
        }
    }

    /// Handle paste (Ctrl+Shift+V)
    fn handle_paste(&mut self) {
        let Some(clipboard) = &mut self.clipboard else {
            return;
        };
        let Some(terminal) = &self.terminal else {
            return;
        };
        let Some(child) = &mut self.child else { return };

        if let Ok(text) = clipboard.get_text() {
            let data = if terminal.screen().modes().bracketed_paste {
                encode_bracketed_paste(&text)
            } else {
                text.into_bytes()
            };
            let _ = child.write_all(&data);
        }
    }

    /// Handle focus change
    fn handle_focus(&mut self, focused: bool) {
        self.focused = focused;

        let Some(terminal) = &self.terminal else {
            return;
        };
        let Some(child) = &mut self.child else { return };

        if terminal.screen().modes().focus_events {
            let data = encode_focus(focused);
            let _ = child.write_all(&data);
        }
    }

    /// Poll PTY for output
    fn poll_pty(&mut self) {
        let Some(child) = &mut self.child else { return };
        let Some(terminal) = &mut self.terminal else {
            return;
        };

        let mut buf = [0u8; 65536];
        let mut received_output = false;

        loop {
            match child.pty_mut().try_read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    terminal.process(&buf[..n]);
                    self.needs_redraw = true;
                    received_output = true;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        // Reset scroll offset when new output arrives (auto-scroll to bottom)
        if received_output && self.scroll_offset > 0 {
            self.scroll_offset = 0;
        }

        // Check for title change
        if terminal.take_title_changed() {
            if let Some(window) = &self.window {
                window.set_title(terminal.title());
            }
        }

        // Check for bell
        if terminal.take_bell() {
            // Could play a sound or flash the window
            log::debug!("Bell!");
        }
    }

    /// Render the terminal
    fn render(&mut self) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let Some(terminal) = &self.terminal else {
            return;
        };

        let screen = terminal.screen();
        let selection = screen.selection();

        if let Err(e) = renderer.render(screen, selection, self.scroll_offset) {
            log::warn!("Render error: {:?}", e);
        }

        self.needs_redraw = false;
        self.last_render = Instant::now();
    }

    /// Check if child is still running
    fn check_child(&mut self) -> bool {
        if let Some(child) = &self.child {
            child.is_running()
        } else {
            false
        }
    }
}

/// Check if a character is a word character (for word selection)
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}
