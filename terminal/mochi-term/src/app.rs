//! Main application logic
//!
//! Ties together the terminal, PTY, and renderer.

use std::io;
use std::rc::Rc;
use std::time::{Duration, Instant};

use arboard::Clipboard;
use terminal_pty::{Child, WindowSize};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowBuilder};

use crate::config::{Config, ParsedKeybinding, ThemeName};
use crate::input::{encode_bracketed_paste, encode_focus, encode_key, encode_mouse, MouseEvent};
use crate::renderer::Renderer;
use crate::terminal::Terminal;

/// Application state
pub struct App {
    /// Configuration
    config: Config,
    /// Current theme
    current_theme: ThemeName,
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
    /// Last click time for double/triple click detection
    last_click_time: Instant,
    /// Last click position for double/triple click detection
    last_click_pos: (u16, u16),
    /// Click count (1 = single, 2 = double, 3 = triple)
    click_count: u8,
    /// Whether we're currently dragging a selection
    is_selecting: bool,
    /// Search state
    search_state: SearchState,
}

/// Search state for find bar
#[derive(Default)]
pub struct SearchState {
    /// Whether the find bar is visible
    pub active: bool,
    /// Current search query
    pub query: String,
    /// Positions of matches (row, col) - row can be negative for scrollback
    pub matches: Vec<(isize, usize)>,
    /// Current match index
    pub current_match: usize,
}

impl App {
    /// Create a new application
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let current_theme = config.theme;
        Ok(Self {
            config,
            current_theme,
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
            last_click_pos: (0, 0),
            click_count: 0,
            is_selecting: false,
            search_state: SearchState::default(),
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

        // Get the key character for matching
        let key_char = match &event.logical_key {
            Key::Character(c) => Some(c.to_lowercase()),
            Key::Named(named) => Some(format!("{:?}", named).to_lowercase()),
            _ => None,
        };

        // Check for configured keybindings
        if let Some(ref key) = key_char {
            let ctrl = self.modifiers.control_key();
            let shift = self.modifiers.shift_key();
            let alt = self.modifiers.alt_key();
            let super_key = self.modifiers.super_key();

            // Toggle theme keybinding
            if let Some(kb) = ParsedKeybinding::parse(&self.config.keybindings.toggle_theme) {
                if kb.matches(ctrl, shift, alt, super_key, key) {
                    self.toggle_theme();
                    return;
                }
            }

            // Copy keybinding
            if let Some(kb) = ParsedKeybinding::parse(&self.config.keybindings.copy) {
                if kb.matches(ctrl, shift, alt, super_key, key) {
                    self.copy_selection();
                    return;
                }
            }

            // Paste keybinding
            if let Some(kb) = ParsedKeybinding::parse(&self.config.keybindings.paste) {
                if kb.matches(ctrl, shift, alt, super_key, key) {
                    self.handle_paste();
                    return;
                }
            }

            // Reload config keybinding
            if let Some(kb) = ParsedKeybinding::parse(&self.config.keybindings.reload_config) {
                if kb.matches(ctrl, shift, alt, super_key, key) {
                    self.reload_config();
                    return;
                }
            }

            // Find keybinding
            if let Some(kb) = ParsedKeybinding::parse(&self.config.keybindings.find) {
                if kb.matches(ctrl, shift, alt, super_key, key) {
                    self.toggle_find_bar();
                    return;
                }
            }
        }

        // Handle find bar input when active
        if self.search_state.active {
            self.handle_find_bar_input(event);
            return;
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

    /// Reload configuration from file
    fn reload_config(&mut self) {
        let config_path = Config::default_config_path();
        match Config::load_with_overrides(config_path, crate::config::CliOverrides::default()) {
            Ok(new_config) => {
                // Apply theme change if different
                if new_config.theme != self.config.theme {
                    self.current_theme = new_config.theme;
                    if let Some(renderer) = &mut self.renderer {
                        let colors = self.current_theme.to_color_scheme();
                        renderer.set_colors(colors);
                    }
                }

                // Apply font size change if different
                if (new_config.font.size - self.config.font.size).abs() > 0.1 {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.set_font_size(new_config.font.size);
                        // Recalculate terminal dimensions
                        if let (Some(window), Some(terminal), Some(child)) =
                            (&self.window, &mut self.terminal, &self.child)
                        {
                            let size = window.inner_size();
                            let cell_size = renderer.cell_size();
                            let cols = (size.width as f32 / cell_size.width) as usize;
                            let rows = (size.height as f32 / cell_size.height) as usize;
                            if cols > 0 && rows > 0 {
                                terminal.resize(cols, rows);
                                let _ = child.resize(WindowSize::new(cols as u16, rows as u16));
                            }
                        }
                    }
                }

                self.config = new_config;
                log::info!("Configuration reloaded successfully");
                self.needs_redraw = true;
            }
            Err(e) => {
                log::error!("Failed to reload configuration: {}", e);
                // Keep the old config active
            }
        }
    }

    /// Toggle to the next theme
    fn toggle_theme(&mut self) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let Some(window) = &self.window else { return };

        // Cycle to next theme
        self.current_theme = self.current_theme.next();

        // Get the color scheme for the new theme
        let colors = self.current_theme.to_color_scheme();

        // Update renderer colors
        renderer.set_colors(colors);

        // Update window title to show current theme
        let title = format!("Mochi Terminal - {}", self.current_theme.display_name());
        window.set_title(&title);

        log::info!("Switched to theme: {}", self.current_theme.display_name());

        self.needs_redraw = true;
    }

    /// Copy selection to clipboard
    fn copy_selection(&mut self) {
        let Some(terminal) = &self.terminal else {
            return;
        };

        let screen = terminal.screen();
        let selection = screen.selection();

        if selection.is_empty() {
            return;
        }

        // Extract selected text from screen
        let text = Self::extract_selected_text_static(screen, selection);
        if text.is_empty() {
            return;
        }

        // Now borrow clipboard mutably
        let Some(clipboard) = &mut self.clipboard else {
            return;
        };

        if let Err(e) = clipboard.set_text(text.clone()) {
            log::warn!("Failed to copy to clipboard: {}", e);
        } else {
            log::debug!("Copied {} characters to clipboard", text.len());
        }
    }

    /// Extract selected text from the screen (static version to avoid borrow issues)
    fn extract_selected_text_static(
        screen: &terminal_core::Screen,
        selection: &terminal_core::Selection,
    ) -> String {
        if selection.is_empty() {
            return String::new();
        }

        let (start, end) = selection.bounds();
        let mut result = String::new();

        for row in start.row..=end.row {
            if row < 0 {
                // TODO: Handle scrollback selection in M5
                continue;
            }

            let row_usize = row as usize;
            if row_usize >= screen.rows() {
                continue;
            }

            let line = screen.line(row_usize);
            let start_col = if row == start.row { start.col } else { 0 };
            let end_col = if row == end.row {
                end.col
            } else {
                line.cols().saturating_sub(1)
            };

            for col in start_col..=end_col {
                if col < line.cols() {
                    let cell = line.cell(col);
                    if !cell.is_continuation() {
                        result.push(cell.display_char());
                    }
                }
            }

            // Add newline between lines (but not after the last line)
            if row < end.row {
                result.push('\n');
            }
        }

        // Trim trailing whitespace from each line
        result
            .lines()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Handle mouse input
    fn handle_mouse_input(&mut self, button: MouseButton, state: ElementState) {
        // Track button state first
        let idx = match button {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
            _ => return,
        };
        self.mouse_buttons[idx] = state == ElementState::Pressed;

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
            return;
        }

        // Handle selection when mouse tracking is NOT enabled
        if button == MouseButton::Left {
            if state == ElementState::Pressed {
                self.handle_selection_start();
            } else {
                self.handle_selection_end();
            }
        }
    }

    /// Handle selection start (left mouse button pressed)
    fn handle_selection_start(&mut self) {
        use terminal_core::{Point, SelectionType};

        let now = Instant::now();
        let double_click_threshold = Duration::from_millis(500);
        let same_position = self.mouse_cell == self.last_click_pos;

        // Determine click count based on timing and position
        if same_position && now.duration_since(self.last_click_time) < double_click_threshold {
            self.click_count = (self.click_count % 3) + 1;
        } else {
            self.click_count = 1;
        }

        self.last_click_time = now;
        self.last_click_pos = self.mouse_cell;
        self.is_selecting = true;

        let Some(terminal) = &mut self.terminal else {
            return;
        };

        let col = self.mouse_cell.0 as usize;
        let row = (self.mouse_cell.1 as isize) - (self.scroll_offset as isize);
        let point = Point::new(col, row);

        let selection_type = match self.click_count {
            1 => SelectionType::Normal,
            2 => SelectionType::Word,
            3 => SelectionType::Line,
            _ => SelectionType::Normal,
        };

        // Start selection
        let selection = terminal.screen_mut().selection_mut();
        selection.start(point, selection_type);

        // For word/line selection, expand to word/line boundaries
        if self.click_count == 2 {
            self.expand_selection_to_word();
        } else if self.click_count == 3 {
            self.expand_selection_to_line();
        }

        self.needs_redraw = true;
    }

    /// Handle selection end (left mouse button released)
    fn handle_selection_end(&mut self) {
        self.is_selecting = false;
        // Selection remains active for copying
    }

    /// Expand selection to word boundaries
    fn expand_selection_to_word(&mut self) {
        use terminal_core::Point;

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
        let cols = line.cols();

        // Find word boundaries
        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';

        let mut start_col = col;
        let mut end_col = col;

        // Expand left
        while start_col > 0 {
            let cell = line.cell(start_col - 1);
            if !is_word_char(cell.display_char()) {
                break;
            }
            start_col -= 1;
        }

        // Expand right
        while end_col < cols.saturating_sub(1) {
            let cell = line.cell(end_col + 1);
            if !is_word_char(cell.display_char()) {
                break;
            }
            end_col += 1;
        }

        let row_isize = (row as isize) - (self.scroll_offset as isize);
        let selection = terminal.screen_mut().selection_mut();
        selection.start(
            Point::new(start_col, row_isize),
            terminal_core::SelectionType::Word,
        );
        selection.update(Point::new(end_col, row_isize));
    }

    /// Expand selection to line boundaries
    fn expand_selection_to_line(&mut self) {
        use terminal_core::Point;

        let Some(terminal) = &mut self.terminal else {
            return;
        };

        let screen = terminal.screen();
        let row = self.mouse_cell.1 as usize;

        if row >= screen.rows() {
            return;
        }

        let line = screen.line(row);
        let cols = line.cols();

        let row_isize = (row as isize) - (self.scroll_offset as isize);
        let selection = terminal.screen_mut().selection_mut();
        selection.start(Point::new(0, row_isize), terminal_core::SelectionType::Line);
        selection.update(Point::new(cols.saturating_sub(1), row_isize));
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

        // Update selection if we're dragging
        if self.is_selecting && self.mouse_buttons[0] {
            use terminal_core::Point;

            let col = self.mouse_cell.0 as usize;
            let row = (self.mouse_cell.1 as isize) - (self.scroll_offset as isize);
            let point = Point::new(col, row);

            let selection = terminal.screen_mut().selection_mut();
            selection.update(point);
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

    /// Handle paste
    #[allow(dead_code)]
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

        // Render find bar overlay if active
        if self.search_state.active {
            if let Err(e) = renderer.render_find_bar(
                &self.search_state.query,
                self.search_state.matches.len(),
                self.search_state.current_match,
            ) {
                log::warn!("Find bar render error: {:?}", e);
            }
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

    /// Toggle the find bar
    fn toggle_find_bar(&mut self) {
        self.search_state.active = !self.search_state.active;
        if !self.search_state.active {
            // Clear search state when closing
            self.search_state.query.clear();
            self.search_state.matches.clear();
            self.search_state.current_match = 0;
        }
        self.needs_redraw = true;
        log::info!(
            "Find bar: {}",
            if self.search_state.active {
                "opened"
            } else {
                "closed"
            }
        );
    }

    /// Handle input when find bar is active
    fn handle_find_bar_input(&mut self, event: &winit::event::KeyEvent) {
        match &event.logical_key {
            Key::Named(NamedKey::Escape) => {
                // Close find bar
                self.search_state.active = false;
                self.search_state.query.clear();
                self.search_state.matches.clear();
                self.needs_redraw = true;
            }
            Key::Named(NamedKey::Enter) => {
                // Navigate to next/prev match
                if self.modifiers.shift_key() {
                    self.search_prev();
                } else {
                    self.search_next();
                }
            }
            Key::Named(NamedKey::Backspace) => {
                // Delete last character
                self.search_state.query.pop();
                self.perform_search();
                self.needs_redraw = true;
            }
            Key::Character(c) => {
                // Add character to search query
                self.search_state.query.push_str(c);
                self.perform_search();
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    /// Perform search in terminal content
    fn perform_search(&mut self) {
        self.search_state.matches.clear();
        self.search_state.current_match = 0;

        if self.search_state.query.is_empty() {
            return;
        }

        let Some(terminal) = &self.terminal else {
            return;
        };

        let screen = terminal.screen();
        let query = &self.search_state.query;

        // Search in visible screen
        for row in 0..screen.rows() {
            let line = screen.line(row);
            let line_text: String = (0..line.cols())
                .map(|col| line.cell(col).display_char())
                .collect();

            // Find all occurrences in this line
            let mut start = 0;
            while let Some(pos) = line_text[start..].find(query) {
                let col = start + pos;
                self.search_state.matches.push((row as isize, col));
                start = col + 1;
            }
        }

        // Search in scrollback
        let scrollback = screen.scrollback();
        for i in 0..scrollback.len() {
            if let Some(line) = scrollback.get_from_end(i) {
                let line_text: String = (0..line.cols())
                    .map(|col| line.cell(col).display_char())
                    .collect();

                let mut start = 0;
                while let Some(pos) = line_text[start..].find(query) {
                    let col = start + pos;
                    // Scrollback rows are negative
                    let row = -((i + 1) as isize);
                    self.search_state.matches.push((row, col));
                    start = col + 1;
                }
            }
        }

        log::info!(
            "Search '{}': {} matches found",
            query,
            self.search_state.matches.len()
        );
    }

    /// Navigate to next search match
    fn search_next(&mut self) {
        if self.search_state.matches.is_empty() {
            return;
        }

        self.search_state.current_match =
            (self.search_state.current_match + 1) % self.search_state.matches.len();
        self.scroll_to_current_match();
    }

    /// Navigate to previous search match
    fn search_prev(&mut self) {
        if self.search_state.matches.is_empty() {
            return;
        }

        if self.search_state.current_match == 0 {
            self.search_state.current_match = self.search_state.matches.len() - 1;
        } else {
            self.search_state.current_match -= 1;
        }
        self.scroll_to_current_match();
    }

    /// Scroll to show the current match
    fn scroll_to_current_match(&mut self) {
        if self.search_state.matches.is_empty() {
            return;
        }

        let (row, _col) = self.search_state.matches[self.search_state.current_match];

        let Some(terminal) = &self.terminal else {
            return;
        };

        let screen = terminal.screen();

        if row < 0 {
            // Match is in scrollback
            let scrollback_row = (-row) as usize;
            self.scroll_offset = scrollback_row;
        } else {
            // Match is in visible area
            let visible_rows = screen.rows();
            if (row as usize) < visible_rows {
                self.scroll_offset = 0;
            }
        }

        self.needs_redraw = true;
        log::info!(
            "Scrolled to match {} of {}",
            self.search_state.current_match + 1,
            self.search_state.matches.len()
        );
    }

    /// Get search state for rendering
    #[allow(dead_code)]
    pub fn search_state(&self) -> &SearchState {
        &self.search_state
    }
}
