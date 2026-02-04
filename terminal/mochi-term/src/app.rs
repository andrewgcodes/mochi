//! Main application logic
//!
//! Ties together the terminal, PTY, and renderer.

use std::io;
use std::rc::Rc;
use std::time::{Duration, Instant};

use arboard::Clipboard;
use terminal_core::{Point, SelectionType};
use terminal_pty::{Child, WindowSize};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowBuilder};

use crate::config::{Action, Config, ThemeName};
use crate::input::{encode_bracketed_paste, encode_focus, encode_key, encode_mouse, MouseEvent};
use crate::renderer::Renderer;
use crate::terminal::Terminal;

/// Maximum time between clicks to count as multi-click (in milliseconds)
const MULTI_CLICK_THRESHOLD_MS: u64 = 500;

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
    /// Current theme (for runtime switching)
    current_theme: ThemeName,
    /// Last click time (for double/triple click detection)
    last_click_time: Instant,
    /// Click count (1 = single, 2 = double, 3 = triple)
    click_count: u8,
    /// Last click position (for multi-click detection)
    last_click_pos: (u16, u16),
    /// Whether we're currently dragging a selection
    selecting: bool,
}

impl App {
    /// Create a new application
    pub fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let current_theme = config.theme;
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
            current_theme,
            last_click_time: Instant::now(),
            click_count: 0,
            last_click_pos: (0, 0),
            selecting: false,
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

        // Extract key string for keybinding lookup
        let key_str = match &event.logical_key {
            Key::Character(c) => c.to_string(),
            Key::Named(NamedKey::ArrowUp) => "up".to_string(),
            Key::Named(NamedKey::ArrowDown) => "down".to_string(),
            Key::Named(NamedKey::ArrowLeft) => "left".to_string(),
            Key::Named(NamedKey::ArrowRight) => "right".to_string(),
            Key::Named(NamedKey::PageUp) => "pageup".to_string(),
            Key::Named(NamedKey::PageDown) => "pagedown".to_string(),
            Key::Named(NamedKey::Home) => "home".to_string(),
            Key::Named(NamedKey::End) => "end".to_string(),
            Key::Named(NamedKey::Escape) => "escape".to_string(),
            _ => String::new(),
        };

        // Check for configured keybindings
        if !key_str.is_empty() {
            let ctrl = self.modifiers.control_key();
            let shift = self.modifiers.shift_key();
            let alt = self.modifiers.alt_key();

            if let Some(action) = self.config.keybindings.find_action(&key_str, ctrl, shift, alt) {
                match action {
                    Action::Copy => {
                        self.handle_copy();
                        return;
                    }
                    Action::Paste => {
                        self.handle_paste();
                        return;
                    }
                    Action::ToggleTheme => {
                        self.toggle_theme();
                        return;
                    }
                    Action::ReloadConfig => {
                        self.reload_config();
                        return;
                    }
                    Action::Find => {
                        log::info!("Find action triggered (not yet implemented)");
                        return;
                    }
                    Action::FontSizeIncrease => {
                        self.change_font_size(2.0);
                        return;
                    }
                    Action::FontSizeDecrease => {
                        self.change_font_size(-2.0);
                        return;
                    }
                    Action::FontSizeReset => {
                        self.reset_font_size();
                        return;
                    }
                    Action::ScrollPageUp => {
                        self.scroll_page(-1);
                        return;
                    }
                    Action::ScrollPageDown => {
                        self.scroll_page(1);
                        return;
                    }
                    Action::ScrollToTop => {
                        self.scroll_to_top();
                        return;
                    }
                    Action::ScrollToBottom => {
                        self.scroll_to_bottom();
                        return;
                    }
                }
            }
        }

        // Check for font zoom shortcuts with arrow keys (Cmd on macOS, Ctrl on Linux)
        #[cfg(target_os = "macos")]
        let zoom_modifier = self.modifiers.super_key();
        #[cfg(not(target_os = "macos"))]
        let zoom_modifier = self.modifiers.control_key();

        if zoom_modifier {
            match &event.logical_key {
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
        let Some(window) = &self.window else { return };

        // Cycle to next theme
        self.current_theme = self.current_theme.next();
        let colors = self.current_theme.color_scheme();

        // Update renderer colors
        renderer.set_colors(colors);

        // Update window title to show current theme
        let base_title = window.title();
        let title_without_theme = base_title
            .split(" [Theme:")
            .next()
            .unwrap_or("Mochi Terminal");
        window.set_title(&format!(
            "{} [Theme: {}]",
            title_without_theme,
            self.current_theme.display_name()
        ));

        log::info!("Theme switched to: {}", self.current_theme.display_name());
        self.needs_redraw = true;
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
            log::debug!("No selection to copy");
            return;
        }

        // Extract selected text from screen
        let (start, end) = selection.bounds();
        let mut text = String::new();

        for row in start.row..=end.row {
            if row < 0 {
                // Scrollback - skip for now (would need scrollback access)
                continue;
            }
            let row_idx = row as usize;
            if row_idx >= screen.rows() {
                continue;
            }

            let line = screen.line(row_idx);
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
                        text.push(cell.display_char());
                    }
                }
            }

            // Add newline between rows (but not after the last row)
            if row < end.row {
                text.push('\n');
            }
        }

        // Trim trailing whitespace from each line
        let text: String = text
            .lines()
            .map(|l| l.trim_end())
            .collect::<Vec<_>>()
            .join("\n");

        if !text.is_empty() {
            if let Err(e) = clipboard.set_text(text.clone()) {
                log::warn!("Failed to copy to clipboard: {}", e);
            } else {
                log::debug!("Copied {} characters to clipboard", text.len());
            }
        }
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
        } else {
            // Handle selection when mouse tracking is not enabled
            if button == MouseButton::Left {
                if state == ElementState::Pressed {
                    self.handle_selection_start();
                } else {
                    self.handle_selection_end();
                }
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

    /// Handle selection start (mouse button pressed)
    fn handle_selection_start(&mut self) {
        let now = Instant::now();
        let (col, row) = self.mouse_cell;

        // Check for multi-click (same position, within time threshold)
        let same_position = col == self.last_click_pos.0 && row == self.last_click_pos.1;
        let within_threshold =
            now.duration_since(self.last_click_time) < Duration::from_millis(MULTI_CLICK_THRESHOLD_MS);

        if same_position && within_threshold {
            self.click_count = (self.click_count % 3) + 1;
        } else {
            self.click_count = 1;
        }

        self.last_click_time = now;
        self.last_click_pos = (col, row);
        self.selecting = true;

        // Determine selection type based on click count
        let selection_type = match self.click_count {
            1 => SelectionType::Normal,
            2 => SelectionType::Word,
            3 => SelectionType::Line,
            _ => SelectionType::Normal,
        };

        // Calculate the point in terminal coordinates (accounting for scroll offset)
        let point = Point::new(col as usize, row as isize - self.scroll_offset as isize);

        // For word selection, find boundaries first (before borrowing terminal mutably)
        let word_bounds = if selection_type == SelectionType::Word {
            self.find_word_boundaries(col as usize, row as usize)
        } else {
            (col as usize, col as usize)
        };

        let Some(terminal) = &mut self.terminal else {
            return;
        };

        // Start selection
        let screen = terminal.screen_mut();

        match selection_type {
            SelectionType::Word => {
                // For word selection, expand to word boundaries
                let (start, end) = word_bounds;
                let start_point = Point::new(start, row as isize - self.scroll_offset as isize);
                let end_point = Point::new(end, row as isize - self.scroll_offset as isize);
                screen.selection_mut().start(start_point, SelectionType::Word);
                screen.selection_mut().update(end_point);
            }
            SelectionType::Line => {
                // For line selection, select entire line
                let start_point = Point::new(0, row as isize - self.scroll_offset as isize);
                screen.selection_mut().start(start_point, SelectionType::Line);
                // End point will be updated on mouse motion or release
            }
            SelectionType::Normal => {
                screen.selection_mut().start(point, SelectionType::Normal);
            }
            _ => {
                screen.selection_mut().start(point, SelectionType::Normal);
            }
        }

        self.needs_redraw = true;
    }

    /// Handle selection end (mouse button released)
    fn handle_selection_end(&mut self) {
        self.selecting = false;

        let Some(terminal) = &mut self.terminal else {
            return;
        };

        terminal.screen_mut().selection_mut().finish();
        self.needs_redraw = true;
    }

    /// Find word boundaries at the given position
    fn find_word_boundaries(&self, col: usize, row: usize) -> (usize, usize) {
        let Some(terminal) = &self.terminal else {
            return (col, col);
        };

        let screen = terminal.screen();
        if row >= screen.rows() {
            return (col, col);
        }

        let line = screen.line(row);
        let cols = line.cols();

        if col >= cols {
            return (col, col);
        }

        // Get the character at the click position
        let cell = line.cell(col);
        let ch = cell.display_char();

        // Define word characters (alphanumeric and underscore)
        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';

        // If clicked on a non-word character, just select that character
        if !is_word_char(ch) {
            return (col, col);
        }

        // Find start of word
        let mut start = col;
        while start > 0 {
            let prev_cell = line.cell(start - 1);
            if !is_word_char(prev_cell.display_char()) {
                break;
            }
            start -= 1;
        }

        // Find end of word
        let mut end = col;
        while end + 1 < cols {
            let next_cell = line.cell(end + 1);
            if !is_word_char(next_cell.display_char()) {
                break;
            }
            end += 1;
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
        } else if self.selecting {
            // Update selection while dragging
            let point = Point::new(col as usize, row as isize - self.scroll_offset as isize);
            terminal.screen_mut().selection_mut().update(point);
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

    /// Reload configuration from file
    fn reload_config(&mut self) {
        log::info!("Reloading configuration...");

        // For now, just log that reload was requested
        // Full implementation would re-read config file and apply changes
        // This is a placeholder for M6 implementation
        log::info!("Config reload not yet fully implemented");
    }

    /// Scroll by pages (negative = up, positive = down)
    fn scroll_page(&mut self, direction: i32) {
        let Some(terminal) = &self.terminal else {
            return;
        };

        let rows = terminal.screen().rows();
        let scrollback_len = terminal.screen().scrollback().len();

        if direction < 0 {
            // Scroll up (show older content)
            self.scroll_offset = (self.scroll_offset + rows).min(scrollback_len);
        } else {
            // Scroll down (show newer content)
            self.scroll_offset = self.scroll_offset.saturating_sub(rows);
        }

        self.needs_redraw = true;
    }

    /// Scroll to the top of scrollback
    fn scroll_to_top(&mut self) {
        let Some(terminal) = &self.terminal else {
            return;
        };

        self.scroll_offset = terminal.screen().scrollback().len();
        self.needs_redraw = true;
    }

    /// Scroll to the bottom (current output)
    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.needs_redraw = true;
    }
}
