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

use crate::config::{Config, KeyAction};
use crate::input::{encode_bracketed_paste, encode_focus, encode_key, encode_mouse, MouseEvent};
use crate::renderer::Renderer;
use crate::terminal::Terminal;

/// Check if a character is a word boundary
fn is_word_boundary(c: char) -> bool {
    matches!(
        c,
        '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '"' | '\'' | '`' | ',' | '.' | ';' | ':'
    )
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
    /// Search bar state
    search_active: bool,
    /// Search query
    search_query: String,
    /// Search matches (row, col, length)
    search_matches: Vec<(usize, usize, usize)>,
    /// Current search match index
    search_current: usize,
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
            last_click_pos: (0, 0),
            click_count: 0,
            is_selecting: false,
            search_active: false,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_current: 0,
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
            self.config.cell_padding,
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

        // Handle search input first if search bar is active
        if self.handle_search_input(event) {
            return;
        }

        // Extract key string for keybinding lookup
        let key_str = match &event.logical_key {
            Key::Character(c) => c.to_string(),
            Key::Named(named) => format!("{:?}", named).to_lowercase(),
            _ => String::new(),
        };

        // Check for configured keybindings
        let ctrl = self.modifiers.control_key();
        let shift = self.modifiers.shift_key();
        let alt = self.modifiers.alt_key();
        let super_key = self.modifiers.super_key();

        if let Some(action) = self
            .config
            .keybindings
            .find_action(&key_str, ctrl, shift, alt, super_key)
        {
            match action {
                KeyAction::Copy => {
                    self.handle_copy();
                    return;
                }
                KeyAction::Paste => {
                    self.handle_paste();
                    return;
                }
                KeyAction::Find => {
                    self.toggle_search();
                    return;
                }
                KeyAction::ReloadConfig => {
                    self.reload_config();
                    return;
                }
                KeyAction::ToggleTheme => {
                    self.toggle_theme();
                    return;
                }
                KeyAction::FontSizeIncrease => {
                    self.change_font_size(2.0);
                    return;
                }
                KeyAction::FontSizeDecrease => {
                    self.change_font_size(-2.0);
                    return;
                }
                KeyAction::FontSizeReset => {
                    self.reset_font_size();
                    return;
                }
                KeyAction::ScrollUp => {
                    self.scroll_viewport(1);
                    return;
                }
                KeyAction::ScrollDown => {
                    self.scroll_viewport(-1);
                    return;
                }
                KeyAction::ScrollPageUp => {
                    if let Some(terminal) = &self.terminal {
                        let rows = terminal.screen().rows();
                        self.scroll_viewport(rows as i32);
                    }
                    return;
                }
                KeyAction::ScrollPageDown => {
                    if let Some(terminal) = &self.terminal {
                        let rows = terminal.screen().rows();
                        self.scroll_viewport(-(rows as i32));
                    }
                    return;
                }
                KeyAction::ScrollToTop => {
                    if let Some(terminal) = &self.terminal {
                        self.scroll_offset = terminal.screen().scrollback().len();
                        self.needs_redraw = true;
                    }
                    return;
                }
                KeyAction::ScrollToBottom => {
                    self.scroll_offset = 0;
                    self.needs_redraw = true;
                    return;
                }
            }
        }

        // Platform-specific copy: Cmd+C on macOS (only when there's a selection)
        #[cfg(target_os = "macos")]
        let platform_copy = super_key && !ctrl && !shift && !alt && key_str == "c";
        #[cfg(not(target_os = "macos"))]
        let platform_copy = false; // On Linux, Ctrl+C should pass through as interrupt
        if platform_copy {
            if let Some(terminal) = &self.terminal {
                if terminal.screen().selection().active {
                    self.handle_copy();
                    return;
                }
            }
        }

        // Platform-specific paste: Cmd+V on macOS, Ctrl+V on Linux
        #[cfg(target_os = "macos")]
        let platform_paste = super_key && !ctrl && !shift && !alt && key_str == "v";
        #[cfg(not(target_os = "macos"))]
        let platform_paste = ctrl && !shift && !alt && !super_key && key_str == "v";
        if platform_paste {
            self.handle_paste();
            return;
        }

        // Legacy font zoom shortcuts (Cmd on macOS, Ctrl on Linux) for compatibility
        #[cfg(target_os = "macos")]
        let zoom_modifier = self.modifiers.super_key();
        #[cfg(not(target_os = "macos"))]
        let zoom_modifier = self.modifiers.control_key();

        if zoom_modifier && !shift {
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

    /// Toggle to the next theme
    fn toggle_theme(&mut self) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };

        // Cycle to next theme
        self.config.theme = self.config.theme.next();
        let new_colors = self.config.effective_colors();

        log::info!("Switched to theme: {}", self.config.theme.display_name());

        // Update renderer colors
        renderer.set_colors(new_colors);
        self.needs_redraw = true;
    }

    /// Reload configuration from file
    fn reload_config(&mut self) {
        match self.config.reload() {
            Ok(()) => {
                log::info!("Configuration reloaded successfully");

                // Apply new colors to renderer
                if let Some(renderer) = &mut self.renderer {
                    renderer.set_colors(self.config.effective_colors());
                }

                self.needs_redraw = true;
            }
            Err(e) => {
                log::warn!("Failed to reload configuration: {}", e);
            }
        }
    }

    /// Scroll the viewport by a number of lines (positive = up/older, negative = down/newer)
    fn scroll_viewport(&mut self, lines: i32) {
        let Some(terminal) = &self.terminal else {
            return;
        };

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

    /// Toggle search bar visibility
    fn toggle_search(&mut self) {
        self.search_active = !self.search_active;
        if !self.search_active {
            // Clear search state when closing
            self.search_query.clear();
            self.search_matches.clear();
            self.search_current = 0;
        }
        self.needs_redraw = true;
        log::info!("Search bar toggled: {}", self.search_active);
    }

    /// Close search bar
    fn close_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.search_matches.clear();
        self.search_current = 0;
        self.needs_redraw = true;
    }

    /// Update search query and find matches
    fn update_search(&mut self, query: &str) {
        self.search_query = query.to_string();
        self.search_matches.clear();
        self.search_current = 0;

        if query.is_empty() {
            self.needs_redraw = true;
            return;
        }

        let Some(terminal) = &self.terminal else {
            return;
        };

        let screen = terminal.screen();
        let query_lower = query.to_lowercase();

        // Search in visible buffer
        for row in 0..screen.rows() {
            let line = screen.line(row);
            let mut line_text = String::new();
            for col in 0..line.cols() {
                line_text.push(line.cell(col).display_char());
            }

            let line_lower = line_text.to_lowercase();
            let mut start = 0;
            while let Some(pos) = line_lower[start..].find(&query_lower) {
                let col = start + pos;
                self.search_matches.push((row, col, query.len()));
                start = col + 1;
            }
        }

        // Search in scrollback buffer
        let scrollback = screen.scrollback();
        for (sb_idx, line) in scrollback.iter().enumerate() {
            let mut line_text = String::new();
            for col in 0..line.cols() {
                line_text.push(line.cell(col).display_char());
            }

            let line_lower = line_text.to_lowercase();
            let mut start = 0;
            while let Some(pos) = line_lower[start..].find(&query_lower) {
                let col = start + pos;
                // Store scrollback rows as negative indices (offset from visible area)
                let row = -(sb_idx as isize + 1);
                self.search_matches.push((row as usize, col, query.len()));
                start = col + 1;
            }
        }

        log::info!(
            "Search for '{}': found {} matches",
            query,
            self.search_matches.len()
        );
        self.needs_redraw = true;
    }

    /// Navigate to next search match
    fn search_next(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }

        self.search_current = (self.search_current + 1) % self.search_matches.len();
        self.scroll_to_match();
        self.needs_redraw = true;
    }

    /// Navigate to previous search match
    fn search_prev(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }

        if self.search_current == 0 {
            self.search_current = self.search_matches.len() - 1;
        } else {
            self.search_current -= 1;
        }
        self.scroll_to_match();
        self.needs_redraw = true;
    }

    /// Scroll viewport to show current search match
    fn scroll_to_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }

        let Some(terminal) = &self.terminal else {
            return;
        };

        let (row, _, _) = self.search_matches[self.search_current];
        let visible_rows = terminal.screen().rows();

        // If the match is in scrollback (negative row stored as large usize)
        if row > visible_rows {
            // This is a scrollback row encoded as negative
            let scrollback_idx = !(row as isize) as usize;
            self.scroll_offset = scrollback_idx + 1;
        } else {
            // Match is in visible area, scroll to show it
            self.scroll_offset = 0;
        }
    }

    /// Handle search input (called when search bar is active)
    fn handle_search_input(&mut self, event: &winit::event::KeyEvent) -> bool {
        if !self.search_active {
            return false;
        }

        use winit::keyboard::{Key, NamedKey};

        match &event.logical_key {
            Key::Named(NamedKey::Escape) => {
                self.close_search();
                return true;
            }
            Key::Named(NamedKey::Enter) => {
                if self.modifiers.shift_key() {
                    self.search_prev();
                } else {
                    self.search_next();
                }
                return true;
            }
            Key::Named(NamedKey::Backspace) => {
                if !self.search_query.is_empty() {
                    self.search_query.pop();
                    self.update_search(&self.search_query.clone());
                }
                return true;
            }
            Key::Character(c) => {
                let mut query = self.search_query.clone();
                query.push_str(c);
                self.update_search(&query);
                return true;
            }
            _ => {}
        }

        false
    }

    /// Handle copy (copy selection to clipboard)
    fn handle_copy(&mut self) {
        let Some(clipboard) = &mut self.clipboard else {
            log::warn!("Clipboard not available");
            return;
        };
        let Some(terminal) = &self.terminal else {
            return;
        };

        let screen = terminal.screen();
        let selection = screen.selection();

        if !selection.active {
            log::debug!("No selection to copy");
            return;
        }

        // Get selected text from screen
        let mut text = String::new();
        let (start, end) = selection.bounds();
        let start_row = start.row.max(0) as usize;
        let end_row = end.row.max(0) as usize;

        for row in start_row..=end_row {
            if row >= screen.rows() {
                continue;
            }
            let line = screen.line(row);
            let start_col = if row == start_row { start.col } else { 0 };
            let end_col = if row == end_row {
                end.col
            } else {
                line.cols().saturating_sub(1)
            };

            for col in start_col..=end_col {
                if col < line.cols() {
                    let cell = line.cell(col);
                    if !cell.is_continuation() {
                        text.push(cell.display_char());
                    }
                }
            }

            if row < end_row {
                text.push('\n');
            }
        }

        if !text.is_empty() {
            if let Err(e) = clipboard.set_text(&text) {
                log::warn!("Failed to copy to clipboard: {}", e);
            } else {
                log::debug!("Copied {} characters to clipboard", text.len());
            }
        }
    }

    /// Handle mouse input
    fn handle_mouse_input(&mut self, button: MouseButton, state: ElementState) {
        // Handle left button for selection
        if button == MouseButton::Left {
            if state == ElementState::Pressed {
                self.handle_left_click_press();
            } else {
                self.handle_left_click_release();
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

    /// Handle left mouse button press (selection start)
    fn handle_left_click_press(&mut self) {
        use terminal_core::{Point, SelectionType};

        let now = Instant::now();
        let double_click_threshold = std::time::Duration::from_millis(500);

        // Check for double/triple click
        let same_position = self.last_click_pos.0 == self.mouse_cell.0
            && self.last_click_pos.1 == self.mouse_cell.1;

        if same_position && now.duration_since(self.last_click_time) < double_click_threshold {
            self.click_count = (self.click_count % 3) + 1;
        } else {
            self.click_count = 1;
        }

        self.last_click_time = now;
        self.last_click_pos = self.mouse_cell;

        let col = self.mouse_cell.0 as usize;
        let row = self.mouse_cell.1 as isize;
        let point = Point::new(col, row);

        let selection_type = match self.click_count {
            1 => SelectionType::Normal,
            2 => SelectionType::Word,
            3 => SelectionType::Line,
            _ => SelectionType::Normal,
        };

        // Start selection
        let Some(terminal) = &mut self.terminal else {
            return;
        };
        terminal
            .screen_mut()
            .selection_mut()
            .start(point, selection_type);

        // For word/line selection, expand to word/line boundaries
        let click_count = self.click_count;
        if click_count == 2 {
            Self::expand_selection_to_word(terminal);
        } else if click_count == 3 {
            Self::expand_selection_to_line(terminal);
        }

        self.is_selecting = true;
        self.needs_redraw = true;
    }

    /// Handle left mouse button release (selection end)
    fn handle_left_click_release(&mut self) {
        self.is_selecting = false;
    }

    /// Expand selection to word boundaries
    fn expand_selection_to_word(terminal: &mut Terminal) {
        let screen = terminal.screen();
        let selection = screen.selection();

        if !selection.active {
            return;
        }

        let row = selection.start.row;
        if row < 0 || row as usize >= screen.rows() {
            return;
        }

        let line = screen.line(row as usize);
        let col = selection.start.col;

        // Find word boundaries
        let mut start_col = col;
        let mut end_col = col;

        // Expand left
        while start_col > 0 {
            let cell = line.cell(start_col - 1);
            let c = cell.display_char();
            if c.is_whitespace() || is_word_boundary(c) {
                break;
            }
            start_col -= 1;
        }

        // Expand right
        while end_col < line.cols().saturating_sub(1) {
            let cell = line.cell(end_col + 1);
            let c = cell.display_char();
            if c.is_whitespace() || is_word_boundary(c) {
                break;
            }
            end_col += 1;
        }

        // Update selection
        let selection = terminal.screen_mut().selection_mut();
        selection.start.col = start_col;
        selection.end.col = end_col;
    }

    /// Expand selection to line boundaries
    fn expand_selection_to_line(terminal: &mut Terminal) {
        let screen = terminal.screen();
        let row = screen.selection().start.row;

        if row < 0 || row as usize >= screen.rows() {
            return;
        }

        let line = screen.line(row as usize);
        let cols = line.cols();

        // Update selection to cover entire line
        let selection = terminal.screen_mut().selection_mut();
        selection.start.col = 0;
        selection.end.col = cols.saturating_sub(1);
    }

    /// Handle mouse motion
    fn handle_mouse_motion(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        let Some(renderer) = &self.renderer else {
            return;
        };
        let Some(terminal) = &mut self.terminal else {
            return;
        };

        let cell_size = renderer.cell_size();
        let col = (position.x / cell_size.width as f64) as u16;
        let row = (position.y / cell_size.height as f64) as u16;

        if col == self.mouse_cell.0 && row == self.mouse_cell.1 {
            return;
        }

        self.mouse_cell = (col, row);

        // Update selection while dragging
        if self.is_selecting {
            use terminal_core::Point;
            let point = Point::new(col as usize, row as isize);
            terminal.screen_mut().selection_mut().update(point);
            self.needs_redraw = true;
        }

        // Send mouse events to PTY if mouse tracking is enabled
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

        // Check for OSC 52 clipboard request
        if let Some(request) = terminal.take_clipboard_request() {
            self.handle_osc52_clipboard(&request);
        }
    }

    /// Handle OSC 52 clipboard request with security checks
    fn handle_osc52_clipboard(&mut self, request: &crate::terminal::ClipboardRequest) {
        // Security check: OSC 52 must be explicitly enabled
        if !self.config.osc52_clipboard {
            log::warn!(
                "OSC 52 clipboard request blocked (disabled in config). \
                 Enable with osc52_clipboard = true in config or --osc52-clipboard flag"
            );
            return;
        }

        // Security check: payload size limit
        if request.data.len() > self.config.osc52_max_size {
            log::warn!(
                "OSC 52 clipboard request blocked: payload size {} exceeds limit {}",
                request.data.len(),
                self.config.osc52_max_size
            );
            return;
        }

        let Some(clipboard) = &mut self.clipboard else {
            log::warn!("OSC 52 clipboard request: clipboard not available");
            return;
        };

        // Handle clipboard query (data = "?")
        if request.data == "?" {
            log::debug!("OSC 52 clipboard query (not implemented - would require writing to PTY)");
            return;
        }

        // Decode base64 data
        use base64::Engine;
        let decoded = match base64::engine::general_purpose::STANDARD.decode(&request.data) {
            Ok(data) => data,
            Err(e) => {
                log::warn!("OSC 52 clipboard: invalid base64 data: {}", e);
                return;
            }
        };

        // Convert to string
        let text = match String::from_utf8(decoded) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("OSC 52 clipboard: invalid UTF-8 data: {}", e);
                return;
            }
        };

        // Set clipboard
        if let Err(e) = clipboard.set_text(&text) {
            log::warn!("OSC 52 clipboard: failed to set clipboard: {}", e);
        } else {
            log::info!(
                "OSC 52 clipboard: set {} characters (target: {})",
                text.len(),
                request.target
            );
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

        let result = if self.search_active {
            renderer.render_with_search(
                screen,
                selection,
                self.scroll_offset,
                Some(&self.search_query),
                &self.search_matches,
                self.search_current,
            )
        } else {
            renderer.render(screen, selection, self.scroll_offset)
        };

        if let Err(e) = result {
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
