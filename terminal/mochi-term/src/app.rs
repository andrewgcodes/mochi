//! Main application logic
//!
//! Ties together the terminal, PTY, and renderer.

use std::io;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

use arboard::Clipboard;
use terminal_pty::{Child, WindowSize};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowBuilder};

use crate::config::{CliArgs, Config};
use crate::input::{encode_bracketed_paste, encode_focus, encode_key, encode_mouse, MouseEvent};
use crate::renderer::Renderer;
use crate::terminal::Terminal;

/// Application state
pub struct App {
    /// Configuration
    config: Config,
    /// Path to config file (for reload)
    config_path: Option<PathBuf>,
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
    /// Search mode active
    search_active: bool,
    /// Search query
    search_query: String,
    /// Search matches (line, start_col, end_col)
    search_matches: Vec<(usize, usize, usize)>,
    /// Current search match index
    search_match_index: usize,
    /// Last click time (for double/triple click detection)
    last_click_time: Instant,
    /// Click count (1 = single, 2 = double, 3 = triple)
    click_count: u8,
    /// Last click position
    last_click_pos: (u16, u16),
    /// Is currently selecting (dragging)
    is_selecting: bool,
}

impl App {
    /// Create a new application
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_config_path(config, None)
    }

    /// Create a new application with a specific config path for reload
    pub fn new_with_config_path(
        config: Config,
        config_path: Option<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            config,
            config_path,
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
            search_active: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_match_index: 0,
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
            self.config.line_height,
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

        // Handle search mode input first
        if self.search_active {
            match &event.logical_key {
                Key::Named(NamedKey::Escape) => {
                    self.close_search();
                    return;
                }
                Key::Named(NamedKey::Enter) => {
                    if self.modifiers.shift_key() {
                        self.search_prev();
                    } else {
                        self.search_next();
                    }
                    return;
                }
                Key::Named(NamedKey::Backspace) => {
                    self.search_query.pop();
                    self.update_search_matches();
                    self.needs_redraw = true;
                    return;
                }
                Key::Character(c) => {
                    self.search_query.push_str(c);
                    self.update_search_matches();
                    self.needs_redraw = true;
                    return;
                }
                _ => return,
            }
        }

        // Check for Ctrl+Shift shortcuts (app-level keybindings) - works on all platforms
        let ctrl_shift = self.modifiers.control_key() && self.modifiers.shift_key();
        if ctrl_shift {
            match &event.logical_key {
                // Copy: Ctrl+Shift+C
                Key::Character(c) if c.eq_ignore_ascii_case("c") => {
                    self.handle_copy();
                    return;
                }
                // Paste: Ctrl+Shift+V
                Key::Character(c) if c.eq_ignore_ascii_case("v") => {
                    self.handle_paste();
                    return;
                }
                // Find/Search: Ctrl+Shift+F
                Key::Character(c) if c.eq_ignore_ascii_case("f") => {
                    self.open_search();
                    return;
                }
                // Reload config: Ctrl+Shift+R
                Key::Character(c) if c.eq_ignore_ascii_case("r") => {
                    self.reload_config();
                    return;
                }
                // Toggle theme: Ctrl+Shift+T
                Key::Character(c) if c.eq_ignore_ascii_case("t") => {
                    self.toggle_theme();
                    return;
                }
                _ => {}
            }
        }

        // macOS-specific: Cmd+C/V for copy/paste (standard macOS shortcuts)
        // This is in addition to Ctrl+Shift+C/V which also works
        #[cfg(target_os = "macos")]
        if self.modifiers.super_key() && !self.modifiers.shift_key() && !self.modifiers.control_key()
        {
            match &event.logical_key {
                // Copy: Cmd+C
                Key::Character(c) if c.eq_ignore_ascii_case("c") => {
                    self.handle_copy();
                    return;
                }
                // Paste: Cmd+V
                Key::Character(c) if c.eq_ignore_ascii_case("v") => {
                    self.handle_paste();
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

        if zoom_modifier && !self.modifiers.shift_key() {
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

        if let Some(data) = encode_key(&event.logical_key, self.modifiers, application_cursor_keys)
        {
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
        let Some(terminal) = &mut self.terminal else {
            return;
        };

        let modes = terminal.screen().modes();

        // If mouse tracking is enabled, send to PTY
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

            // Track button state
            let idx = match button {
                MouseButton::Left => 0,
                MouseButton::Middle => 1,
                MouseButton::Right => 2,
                _ => return,
            };
            self.mouse_buttons[idx] = state == ElementState::Pressed;
            return;
        }

        // Handle selection when mouse tracking is disabled
        if button == MouseButton::Left {
            if state == ElementState::Pressed {
                self.handle_left_click_pressed();
            } else {
                self.handle_left_click_released();
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

    /// Handle left mouse button press (selection start)
    fn handle_left_click_pressed(&mut self) {
        use terminal_core::{Point, SelectionType};

        let now = Instant::now();
        let click_threshold = std::time::Duration::from_millis(400);
        let same_position = self.mouse_cell == self.last_click_pos;

        // Detect double/triple click
        if now.duration_since(self.last_click_time) < click_threshold && same_position {
            self.click_count = (self.click_count % 3) + 1;
        } else {
            self.click_count = 1;
        }

        self.last_click_time = now;
        self.last_click_pos = self.mouse_cell;
        self.is_selecting = true;

        let col = self.mouse_cell.0 as usize;
        let row = self.scroll_offset as isize;

        // Calculate word boundaries before borrowing terminal mutably
        let word_bounds = if self.click_count == 2 {
            Some(self.find_word_boundaries(col, -(row) + self.mouse_cell.1 as isize))
        } else {
            None
        };

        let Some(terminal) = &mut self.terminal else {
            return;
        };

        let sel_row = self.mouse_cell.1 as isize - row;
        let point = Point::new(col, sel_row);

        match self.click_count {
            1 => {
                // Single click: start normal selection
                terminal
                    .screen_mut()
                    .selection_mut()
                    .start(point, SelectionType::Normal);
            }
            2 => {
                // Double click: select word
                if let Some((word_start, word_end)) = word_bounds {
                    terminal
                        .screen_mut()
                        .selection_mut()
                        .start(Point::new(word_start, sel_row), SelectionType::Word);
                    terminal
                        .screen_mut()
                        .selection_mut()
                        .update(Point::new(word_end, sel_row));
                }
            }
            3 => {
                // Triple click: select line
                terminal
                    .screen_mut()
                    .selection_mut()
                    .start(Point::new(0, sel_row), SelectionType::Line);
                let cols = terminal.screen().cols();
                terminal
                    .screen_mut()
                    .selection_mut()
                    .update(Point::new(cols.saturating_sub(1), sel_row));
            }
            _ => {}
        }

        self.needs_redraw = true;
    }

    /// Handle left mouse button release (selection end)
    fn handle_left_click_released(&mut self) {
        self.is_selecting = false;

        let Some(terminal) = &mut self.terminal else {
            return;
        };

        terminal.screen_mut().selection_mut().finish();
        self.needs_redraw = true;
    }

    /// Find word boundaries at the given position
    fn find_word_boundaries(&self, col: usize, row: isize) -> (usize, usize) {
        let Some(terminal) = &self.terminal else {
            return (col, col);
        };

        let screen = terminal.screen();
        let cols = screen.cols();

        // Get the line text
        let line_text: String = if row < 0 {
            // In scrollback
            let scrollback = screen.scrollback();
            let scrollback_idx = (scrollback.len() as isize + row) as usize;
            if let Some(line) = scrollback.get(scrollback_idx) {
                line.iter().map(|cell| cell.display_char()).collect()
            } else {
                return (col, col);
            }
        } else if (row as usize) < screen.rows() {
            // In visible buffer
            (0..cols)
                .map(|c| screen.line(row as usize).cell(c).display_char())
                .collect()
        } else {
            return (col, col);
        };

        let chars: Vec<char> = line_text.chars().collect();
        if col >= chars.len() {
            return (col, col);
        }

        // Find word boundaries (alphanumeric + underscore)
        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';

        let mut start = col;
        let mut end = col;

        // If we're on a word character, expand to word boundaries
        if is_word_char(chars[col]) {
            // Find start of word
            while start > 0 && is_word_char(chars[start - 1]) {
                start -= 1;
            }
            // Find end of word
            while end < chars.len() - 1 && is_word_char(chars[end + 1]) {
                end += 1;
            }
        }

        (start, end)
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

        let Some(terminal) = &mut self.terminal else {
            return;
        };

        let modes = terminal.screen().modes();

        // If mouse tracking is enabled, send to PTY
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
            return;
        }

        // Update selection while dragging
        if self.is_selecting {
            use terminal_core::Point;

            let sel_col = col as usize;
            let sel_row = row as isize - self.scroll_offset as isize;

            terminal
                .screen_mut()
                .selection_mut()
                .update(Point::new(sel_col, sel_row));
            self.needs_redraw = true;
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
        let Some(clipboard) = &mut self.clipboard else {
            return;
        };
        let Some(terminal) = &self.terminal else {
            return;
        };

        // Get selected text from terminal
        if let Some(text) = terminal.screen().selection_text() {
            if !text.is_empty() {
                if let Err(e) = clipboard.set_text(&text) {
                    log::warn!("Failed to copy to clipboard: {}", e);
                } else {
                    log::debug!("Copied {} bytes to clipboard", text.len());
                }
            }
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

    /// Open search bar (Ctrl+Shift+F)
    fn open_search(&mut self) {
        self.search_active = true;
        self.search_query.clear();
        self.search_matches.clear();
        self.search_match_index = 0;
        self.needs_redraw = true;
        log::debug!("Search mode activated");
    }

    /// Close search bar (Escape)
    fn close_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.search_matches.clear();
        self.search_match_index = 0;
        self.needs_redraw = true;
        log::debug!("Search mode deactivated");
    }

    /// Navigate to next search match (Enter)
    fn search_next(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_match_index = (self.search_match_index + 1) % self.search_matches.len();
        self.scroll_to_match();
        self.needs_redraw = true;
    }

    /// Navigate to previous search match (Shift+Enter)
    fn search_prev(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        if self.search_match_index == 0 {
            self.search_match_index = self.search_matches.len() - 1;
        } else {
            self.search_match_index -= 1;
        }
        self.scroll_to_match();
        self.needs_redraw = true;
    }

    /// Update search matches based on current query
    fn update_search_matches(&mut self) {
        self.search_matches.clear();
        self.search_match_index = 0;

        if self.search_query.is_empty() {
            return;
        }

        let Some(terminal) = &self.terminal else {
            return;
        };

        let screen = terminal.screen();
        let query = self.search_query.to_lowercase();

        // Search in scrollback
        for (line_idx, line) in screen.scrollback().iter().enumerate() {
            let line_text: String = line.iter().map(|cell| cell.display_char()).collect();
            let line_lower = line_text.to_lowercase();
            let mut start = 0;
            while let Some(pos) = line_lower[start..].find(&query) {
                let abs_pos = start + pos;
                self.search_matches
                    .push((line_idx, abs_pos, abs_pos + query.len()));
                start = abs_pos + 1;
            }
        }

        // Search in visible buffer
        let scrollback_len = screen.scrollback().len();
        for row in 0..screen.rows() {
            let line_text: String = (0..screen.cols())
                .map(|col| screen.line(row).cell(col).display_char())
                .collect();
            let line_lower = line_text.to_lowercase();
            let mut start = 0;
            while let Some(pos) = line_lower[start..].find(&query) {
                let abs_pos = start + pos;
                self.search_matches
                    .push((scrollback_len + row, abs_pos, abs_pos + query.len()));
                start = abs_pos + 1;
            }
        }

        log::debug!(
            "Found {} matches for '{}'",
            self.search_matches.len(),
            self.search_query
        );
    }

    /// Scroll viewport to show current match
    fn scroll_to_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }

        let Some(terminal) = &self.terminal else {
            return;
        };

        let (match_line, _, _) = self.search_matches[self.search_match_index];
        let screen = terminal.screen();
        let scrollback_len = screen.scrollback().len();
        let visible_rows = screen.rows();

        // Calculate scroll offset to show the match
        if match_line < scrollback_len {
            // Match is in scrollback
            self.scroll_offset = scrollback_len - match_line;
        } else {
            // Match is in visible area
            let visible_line = match_line - scrollback_len;
            if visible_line < visible_rows {
                self.scroll_offset = 0;
            }
        }
    }

    /// Reload configuration (Ctrl+Shift+R)
    fn reload_config(&mut self) {
        log::info!("Reloading configuration...");

        // Try to reload from the same path or default
        let new_config = if let Some(path) = &self.config_path {
            Config::load_from_path(path)
        } else {
            Config::load_with_args(&CliArgs::default())
        };

        match new_config {
            Ok(config) => {
                log::info!("Configuration reloaded successfully");
                self.config = config;
                self.apply_config_changes();
            }
            Err(e) => {
                log::error!("Failed to reload configuration: {}", e);
                // Keep the old config - don't crash
            }
        }
    }

    /// Apply configuration changes after reload
    fn apply_config_changes(&mut self) {
        // Apply theme changes
        if let Some(renderer) = &mut self.renderer {
            renderer.set_theme(self.config.theme);
        }

        // Apply font size changes
        if let Some(renderer) = &mut self.renderer {
            if let Some(window) = &self.window {
                let scale_factor = window.scale_factor() as f32;
                let new_size = self.config.font_size * scale_factor;
                renderer.set_font_size(new_size);

                // Recalculate terminal dimensions
                let size = window.inner_size();
                let cell_size = renderer.cell_size();
                let cols = (size.width as f32 / cell_size.width) as usize;
                let rows = (size.height as f32 / cell_size.height) as usize;

                if cols > 0 && rows > 0 {
                    if let Some(terminal) = &mut self.terminal {
                        terminal.resize(cols, rows);
                    }
                    if let Some(child) = &self.child {
                        let _ = child.resize(WindowSize::new(cols as u16, rows as u16));
                    }
                }
            }
        }

        self.needs_redraw = true;
    }

    /// Toggle theme (Ctrl+Shift+T)
    fn toggle_theme(&mut self) {
        let new_theme = self.config.theme.next();
        log::info!(
            "Toggling theme from {:?} to {:?}",
            self.config.theme,
            new_theme
        );
        self.config.theme = new_theme;

        if let Some(renderer) = &mut self.renderer {
            renderer.set_theme(new_theme);
        }

        self.needs_redraw = true;
    }

    /// Check if search is active
    #[allow(dead_code)]
    pub fn is_search_active(&self) -> bool {
        self.search_active
    }

    /// Get search query
    #[allow(dead_code)]
    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    /// Get search matches
    #[allow(dead_code)]
    pub fn search_matches(&self) -> &[(usize, usize, usize)] {
        &self.search_matches
    }

    /// Get current search match index
    #[allow(dead_code)]
    pub fn search_match_index(&self) -> usize {
        self.search_match_index
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

        // Check for OSC 52 clipboard request
        if let Some(clipboard_data) = terminal.take_pending_clipboard() {
            self.handle_osc52_clipboard(&clipboard_data);
        }
    }

    /// Handle OSC 52 clipboard request with security checks
    fn handle_osc52_clipboard(&mut self, data: &str) {
        // Check if OSC 52 clipboard is enabled
        if !self.config.osc52_clipboard {
            log::info!("OSC 52 clipboard request blocked (disabled in config)");
            return;
        }

        // Check payload size limit
        let max_size = self.config.osc52_max_size;
        if data.len() > max_size {
            log::warn!(
                "OSC 52 clipboard request blocked: payload size {} exceeds limit {}",
                data.len(),
                max_size
            );
            return;
        }

        // Decode base64 data
        use base64::{engine::general_purpose::STANDARD, Engine};
        match STANDARD.decode(data) {
            Ok(decoded) => {
                match String::from_utf8(decoded) {
                    Ok(text) => {
                        // Set clipboard content
                        if let Some(clipboard) = &mut self.clipboard {
                            match clipboard.set_text(&text) {
                                Ok(()) => {
                                    log::info!(
                                        "OSC 52: clipboard set ({} bytes from {} base64 bytes)",
                                        text.len(),
                                        data.len()
                                    );
                                }
                                Err(e) => {
                                    log::warn!("OSC 52: failed to set clipboard: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("OSC 52: invalid UTF-8 in clipboard data: {}", e);
                    }
                }
            }
            Err(e) => {
                log::warn!("OSC 52: invalid base64 data: {}", e);
            }
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

        // Pass search matches to renderer if search is active
        let search_matches = if self.search_active && !self.search_matches.is_empty() {
            Some(self.search_matches.as_slice())
        } else {
            None
        };
        let current_match_idx = if self.search_active && !self.search_matches.is_empty() {
            Some(self.search_match_index)
        } else {
            None
        };

        if let Err(e) = renderer.render(
            screen,
            selection,
            self.scroll_offset,
            search_matches,
            current_match_idx,
        ) {
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
