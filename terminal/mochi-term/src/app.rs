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

use crate::config::Config;
use crate::input::{encode_bracketed_paste, encode_focus, encode_key, encode_mouse, MouseEvent};
use crate::renderer::Renderer;
use crate::terminal::Terminal;

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
    #[allow(dead_code)]
    clipboard: Option<Clipboard>,
    /// Current modifiers state
    modifiers: ModifiersState,
    /// Mouse position (in cells)
    mouse_cell: (u16, u16),
    /// Mouse position (in pixels)
    mouse_pixel: (f64, f64),
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
    /// Whether we're currently dragging the scrollbar
    scrollbar_dragging: bool,
    /// Y position where scrollbar drag started (in pixels)
    scrollbar_drag_start_y: f64,
    /// Scroll offset when scrollbar drag started
    scrollbar_drag_start_offset: usize,
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
            mouse_pixel: (0.0, 0.0),
            mouse_buttons: [false; 3],
            last_render: Instant::now(),
            needs_redraw: true,
            focused: true,
            scroll_offset: 0,
            scrollbar_dragging: false,
            scrollbar_drag_start_y: 0.0,
            scrollbar_drag_start_offset: 0,
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

                    // Render directly if needed (more reliable than request_redraw on macOS)
                    // This ensures TUI apps like Claude Code render immediately
                    if self.needs_redraw {
                        self.render();
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
            self.config.font_size(),
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

        // Check for app shortcuts (Ctrl+Shift combinations)
        let ctrl_shift = self.modifiers.control_key() && self.modifiers.shift_key();

        if ctrl_shift {
            match &event.logical_key {
                // Copy: Ctrl+Shift+C
                Key::Character(c) if c.to_lowercase() == "c" => {
                    self.handle_copy();
                    return;
                }
                // Paste: Ctrl+Shift+V
                Key::Character(c) if c.to_lowercase() == "v" => {
                    self.handle_paste();
                    return;
                }
                // Find: Ctrl+Shift+F
                Key::Character(c) if c.to_lowercase() == "f" => {
                    self.handle_find();
                    return;
                }
                // Reload config: Ctrl+Shift+R
                Key::Character(c) if c.to_lowercase() == "r" => {
                    self.handle_reload_config();
                    return;
                }
                // Toggle theme: Ctrl+Shift+T
                Key::Character(c) if c.to_lowercase() == "t" => {
                    self.handle_toggle_theme();
                    return;
                }
                _ => {}
            }
        }

        // macOS: Cmd+V for paste, Cmd+C for copy (standard macOS shortcuts)
        #[cfg(target_os = "macos")]
        if self.modifiers.super_key() && !self.modifiers.control_key() && !self.modifiers.alt_key()
        {
            match &event.logical_key {
                Key::Character(c) if c.to_lowercase() == "v" => {
                    self.handle_paste();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "c" => {
                    self.handle_copy();
                    return;
                }
                _ => {}
            }
        }

        // Check for font zoom shortcuts (Cmd on macOS, Ctrl on Linux)
        #[cfg(target_os = "macos")]
        let zoom_modifier = self.modifiers.super_key();
        #[cfg(not(target_os = "macos"))]
        let zoom_modifier = self.modifiers.control_key();

        if zoom_modifier {
            match &event.logical_key {
                Key::Character(c) if c == "=" || c == "+" => {
                    self.change_font_size(2.0);
                    return;
                }
                Key::Character(c) if c == "-" => {
                    self.change_font_size(-2.0);
                    return;
                }
                Key::Character(c) if c == "0" => {
                    self.reset_font_size();
                    return;
                }
                Key::Named(NamedKey::ArrowUp) => {
                    self.change_font_size(2.0);
                    return;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    self.change_font_size(-2.0);
                    return;
                }
                _ => {}
            }
        }

        let Some(terminal) = &self.terminal else {
            return;
        };
        let Some(child) = &mut self.child else { return };

        let application_cursor_keys = terminal.screen().modes().cursor_keys_application;

        // First, try to use the text field from the KeyEvent
        // On macOS, control characters like Ctrl+C might be in the text field directly
        if let Some(text) = &event.text {
            if !text.is_empty() {
                let first_char = text.chars().next().unwrap();
                // Check if it's a control character (0x01-0x1A)
                if (first_char as u32) >= 1 && (first_char as u32) <= 26 {
                    log::debug!(
                        "Sending control character from text field: {:?} (0x{:02x})",
                        first_char,
                        first_char as u8
                    );
                    let _ = child.write_all(&[first_char as u8]);
                    return;
                }
            }
        }

        if let Some(data) = encode_key(&event.logical_key, self.modifiers, application_cursor_keys)
        {
            log::debug!("Sending key data: {:?}", data);
            let _ = child.write_all(&data);
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
        let default_size = self.config.font_size() * scale_factor;

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
        // Handle scrollbar dragging first (left button only)
        if button == MouseButton::Left {
            if state == ElementState::Pressed {
                // Check if click is on scrollbar (right 8 pixels of window)
                if let (Some(window), Some(terminal)) = (&self.window, &self.terminal) {
                    let window_width = window.inner_size().width as f64;
                    let scrollbar_width = 8.0;

                    if self.mouse_pixel.0 >= window_width - scrollbar_width {
                        let scrollback_len = terminal.screen().scrollback().len();
                        if scrollback_len > 0 {
                            // Start scrollbar dragging
                            self.scrollbar_dragging = true;
                            self.scrollbar_drag_start_y = self.mouse_pixel.1;
                            self.scrollbar_drag_start_offset = self.scroll_offset;
                            return;
                        }
                    }
                }
            } else {
                // Mouse released - stop dragging
                if self.scrollbar_dragging {
                    self.scrollbar_dragging = false;
                    return;
                }
            }
        }

        let Some(terminal) = &self.terminal else {
            return;
        };
        let Some(child) = &mut self.child else { return };

        let modes = terminal.screen().modes();
        if !modes.mouse_tracking_enabled() {
            return;
        }

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

        // Track button state
        let idx = match button {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
            _ => return,
        };
        self.mouse_buttons[idx] = state == ElementState::Pressed;
    }

    /// Handle mouse motion
    fn handle_mouse_motion(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        // Update pixel position
        self.mouse_pixel = (position.x, position.y);

        // Handle scrollbar dragging
        if self.scrollbar_dragging {
            if let (Some(window), Some(terminal)) = (&self.window, &self.terminal) {
                let window_height = window.inner_size().height as f64;
                let scrollback_len = terminal.screen().scrollback().len();
                let visible_rows = terminal.screen().rows();

                if scrollback_len > 0 && window_height > 0.0 {
                    // Calculate how much the mouse has moved
                    let delta_y = position.y - self.scrollbar_drag_start_y;

                    // Calculate the scroll range (total scrollable area)
                    let total_lines = scrollback_len + visible_rows;
                    let thumb_height =
                        ((visible_rows as f64 / total_lines as f64) * window_height).max(20.0);
                    let scroll_range = window_height - thumb_height;

                    if scroll_range > 0.0 {
                        // Convert pixel delta to scroll offset delta
                        // Moving down (positive delta) should decrease scroll_offset (show newer content)
                        // Moving up (negative delta) should increase scroll_offset (show older content)
                        let scroll_delta =
                            (-delta_y / scroll_range * scrollback_len as f64) as isize;

                        let new_offset = (self.scrollbar_drag_start_offset as isize + scroll_delta)
                            .max(0)
                            .min(scrollback_len as isize)
                            as usize;

                        if new_offset != self.scroll_offset {
                            self.scroll_offset = new_offset;
                            self.needs_redraw = true;
                        }
                    }
                }
            }
            return;
        }

        let Some(renderer) = &self.renderer else {
            return;
        };
        let Some(terminal) = &self.terminal else {
            return;
        };
        let Some(child) = &mut self.child else { return };

        let cell_size = renderer.cell_size();
        let col = (position.x / cell_size.width as f64) as u16;
        let row = (position.y / cell_size.height as f64) as u16;

        if col == self.mouse_cell.0 && row == self.mouse_cell.1 {
            return;
        }

        self.mouse_cell = (col, row);

        let modes = terminal.screen().modes();
        if modes.mouse_any_event
            || (modes.mouse_button_event && self.mouse_buttons.iter().any(|&b| b))
        {
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

    /// Handle copy (Ctrl+Shift+C)
    fn handle_copy(&mut self) {
        let Some(terminal) = &self.terminal else {
            return;
        };

        let screen = terminal.screen();
        let selection = screen.selection();

        if selection.is_empty() {
            return;
        }

        // Get selected text using the Line::text() method
        let (start, end) = selection.bounds();
        let mut text = String::new();
        let cols = screen.cols();

        for row in start.row..=end.row {
            let start_col = if row == start.row { start.col } else { 0 };
            let end_col = if row == end.row { end.col } else { cols };

            // Get line from screen or scrollback
            if row < 0 {
                // Line is in scrollback
                let scrollback_idx = (-row - 1) as usize;
                if let Some(line) = screen.scrollback().get_from_end(scrollback_idx) {
                    let line_text = line.text();
                    let chars: Vec<char> = line_text.chars().collect();
                    for ch in chars.iter().take(end_col.min(chars.len())).skip(start_col) {
                        text.push(*ch);
                    }
                }
            } else if (row as usize) < screen.grid().rows() {
                // Line is in visible grid
                let line = screen.line(row as usize);
                let line_text = line.text();
                let chars: Vec<char> = line_text.chars().collect();
                for ch in chars.iter().take(end_col.min(chars.len())).skip(start_col) {
                    text.push(*ch);
                }
            }

            // Add newline between lines (but not after the last line)
            if row < end.row {
                // Trim trailing spaces before newline
                while text.ends_with(' ') {
                    text.pop();
                }
                text.push('\n');
            }
        }

        // Trim trailing whitespace
        let text = text.trim_end().to_string();

        if text.is_empty() {
            return;
        }

        // Now copy to clipboard
        let Some(clipboard) = &mut self.clipboard else {
            return;
        };

        if let Err(e) = clipboard.set_text(&text) {
            log::warn!("Failed to copy to clipboard: {}", e);
        } else {
            log::debug!("Copied {} bytes to clipboard", text.len());
        }
    }

    /// Handle paste (Ctrl+Shift+V)
    fn handle_paste(&mut self) {
        let Some(clipboard) = &mut self.clipboard else {
            log::warn!("Clipboard not available");
            return;
        };
        let Some(terminal) = &self.terminal else {
            return;
        };
        let Some(child) = &mut self.child else { return };

        match clipboard.get_text() {
            Ok(text) => {
                if text.is_empty() {
                    return;
                }
                let data = if terminal.screen().modes().bracketed_paste {
                    encode_bracketed_paste(&text)
                } else {
                    text.into_bytes()
                };
                if let Err(e) = child.write_all(&data) {
                    log::warn!("Failed to write paste data to PTY: {}", e);
                } else {
                    log::debug!("Pasted {} bytes", data.len());
                }
            }
            Err(e) => {
                log::warn!("Failed to get clipboard text: {}", e);
            }
        }
    }

    /// Handle find (Ctrl+Shift+F) - placeholder for search UI
    fn handle_find(&mut self) {
        log::info!("Find requested (Ctrl+Shift+F) - search UI not yet implemented");
        // TODO: Implement search UI overlay in M5
    }

    /// Handle reload config (Ctrl+Shift+R)
    fn handle_reload_config(&mut self) {
        log::info!("Reloading configuration...");

        match Config::load() {
            Some(new_config) => {
                // Update theme
                self.config.theme = new_config.theme;
                self.config.font = new_config.font.clone();
                self.config.keybindings = new_config.keybindings.clone();
                self.config.security = new_config.security.clone();

                // Apply theme change
                if let Some(renderer) = &mut self.renderer {
                    renderer.set_colors(self.config.effective_colors());
                }

                log::info!("Configuration reloaded successfully");
                self.needs_redraw = true;
            }
            None => {
                log::warn!("No config file found or failed to parse");
            }
        }
    }

    /// Handle toggle theme (Ctrl+Shift+T)
    fn handle_toggle_theme(&mut self) {
        let new_theme = self.config.theme.next();
        log::info!(
            "Switching theme from {:?} to {:?}",
            self.config.theme,
            new_theme
        );

        self.config.theme = new_theme;

        if let Some(renderer) = &mut self.renderer {
            renderer.set_colors(self.config.effective_colors());
        }

        self.needs_redraw = true;
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
