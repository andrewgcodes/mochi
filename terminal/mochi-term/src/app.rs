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

use terminal_core::selection::{Point, SelectionType};

use crate::config::{CliArgs, Config, KeyAction};
use crate::input::{encode_bracketed_paste, encode_focus, encode_key, encode_mouse, MouseEvent};
use crate::renderer::Renderer;
use crate::terminal::Terminal;

/// Double/triple click detection threshold (in milliseconds)
const CLICK_THRESHOLD_MS: u128 = 500;

/// Application state
pub struct App {
    /// Configuration
    config: Config,
    /// CLI arguments (for config reload)
    cli_args: CliArgs,
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
    /// Last click time for double/triple click detection
    last_click_time: Instant,
    /// Last click position for double/triple click detection
    last_click_pos: (u16, u16),
    /// Click count (1=single, 2=double, 3=triple)
    click_count: u8,
    /// Whether we're currently dragging a selection
    selecting: bool,
    /// Search state
    search_state: SearchState,
}

/// Search state for the find bar
#[derive(Default)]
struct SearchState {
    /// Whether the search bar is visible
    active: bool,
    /// Current search query
    query: String,
    /// Search matches (row, start_col, end_col)
    matches: Vec<SearchMatch>,
    /// Current match index
    current_match: usize,
}

/// A search match location
#[derive(Clone, Debug)]
struct SearchMatch {
    /// Row in the terminal (can be negative for scrollback)
    row: isize,
    /// Start column
    start_col: usize,
    /// End column (exclusive)
    end_col: usize,
}

impl App {
    /// Create a new application
    pub fn new(config: Config, cli_args: CliArgs) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            config,
            cli_args,
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
            selecting: false,
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

        // Handle search input if search bar is active
        if self.search_state.active {
            // Handle Shift+Enter for previous match
            if self.modifiers.shift_key() {
                if let Key::Named(NamedKey::Enter) = &event.logical_key {
                    self.search_prev();
                    return;
                }
            }
            if self.handle_search_input(&event.logical_key) {
                return;
            }
        }

        // Get key name for keybinding matching
        let key_name = match &event.logical_key {
            Key::Character(c) => c.to_string(),
            Key::Named(named) => format!("{:?}", named).to_lowercase(),
            _ => return,
        };

        // Check for configurable keybindings
        let ctrl = self.modifiers.control_key();
        let alt = self.modifiers.alt_key();
        let shift = self.modifiers.shift_key();
        let super_key = self.modifiers.super_key();

        if let Some(action) = self
            .config
            .keybindings
            .get_action(ctrl, alt, shift, super_key, &key_name)
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
                _ => {}
            }
        }

        // Legacy font zoom shortcuts (Cmd on macOS, Ctrl on Linux) for compatibility
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

    /// Handle copy to clipboard
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
        if selection.active {
            let text = screen.get_selected_text(selection);
            if !text.is_empty() {
                if let Err(e) = clipboard.set_text(&text) {
                    log::warn!("Failed to copy to clipboard: {}", e);
                } else {
                    log::debug!("Copied {} characters to clipboard", text.len());
                }
            }
        }
    }

    /// Toggle search UI
    fn toggle_search(&mut self) {
        self.search_state.active = !self.search_state.active;
        if self.search_state.active {
            log::info!("Search bar opened");
            self.search_state.query.clear();
            self.search_state.matches.clear();
            self.search_state.current_match = 0;
        } else {
            log::info!("Search bar closed");
            self.search_state.query.clear();
            self.search_state.matches.clear();
        }
        self.needs_redraw = true;
    }

    /// Handle search input when search bar is active
    fn handle_search_input(&mut self, key: &Key) -> bool {
        if !self.search_state.active {
            return false;
        }

        match key {
            Key::Named(NamedKey::Escape) => {
                self.toggle_search();
                true
            }
            Key::Named(NamedKey::Enter) => {
                self.search_next();
                true
            }
            Key::Named(NamedKey::Backspace) => {
                self.search_state.query.pop();
                self.perform_search();
                self.needs_redraw = true;
                true
            }
            Key::Character(c) => {
                self.search_state.query.push_str(c);
                self.perform_search();
                self.needs_redraw = true;
                true
            }
            _ => false,
        }
    }

    /// Perform search in terminal buffer
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
        let rows = screen.rows();

        // Search visible buffer
        for row_idx in 0..rows {
            let line = screen.line(row_idx);
            let line_text: String = (0..line.cols())
                .map(|col| line.cell(col).display_char())
                .collect();

            // Find all occurrences in this line
            let mut start = 0;
            while let Some(pos) = line_text[start..].find(query) {
                let abs_pos = start + pos;
                self.search_state.matches.push(SearchMatch {
                    row: row_idx as isize,
                    start_col: abs_pos,
                    end_col: abs_pos + query.len(),
                });
                start = abs_pos + 1;
            }
        }

        // Search scrollback buffer
        let scrollback = screen.scrollback();
        let scrollback_len = scrollback.len();
        for i in 0..scrollback_len {
            if let Some(line) = scrollback.get(i) {
                let line_text: String = (0..line.cols())
                    .map(|col| line.cell(col).display_char())
                    .collect();

                let mut start = 0;
                while let Some(pos) = line_text[start..].find(query) {
                    let abs_pos = start + pos;
                    self.search_state.matches.push(SearchMatch {
                        row: -((scrollback_len - i) as isize),
                        start_col: abs_pos,
                        end_col: abs_pos + query.len(),
                    });
                    start = abs_pos + 1;
                }
            }
        }

        log::debug!(
            "Found {} matches for '{}'",
            self.search_state.matches.len(),
            query
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
        self.needs_redraw = true;
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
        self.needs_redraw = true;
    }

    /// Scroll to make current match visible
    fn scroll_to_current_match(&mut self) {
        if self.search_state.matches.is_empty() {
            return;
        }

        let Some(terminal) = &self.terminal else {
            return;
        };

        let current = &self.search_state.matches[self.search_state.current_match];
        let rows = terminal.screen().rows() as isize;

        // Calculate scroll offset needed to show this match
        if current.row < 0 {
            // Match is in scrollback
            self.scroll_offset = (-current.row) as usize;
        } else if current.row >= rows - self.scroll_offset as isize {
            // Match is below visible area
            self.scroll_offset = 0;
        } else {
            // Match is visible or above, adjust if needed
            if (current.row as usize) < self.scroll_offset {
                self.scroll_offset = current.row.max(0) as usize;
            }
        }
    }

    /// Reload configuration
    fn reload_config(&mut self) {
        log::info!("Reloading configuration...");
        match self.config.reload(&self.cli_args) {
            Ok(()) => {
                log::info!("Configuration reloaded successfully");
                // Apply new theme colors to renderer
                if let Some(renderer) = &mut self.renderer {
                    renderer.set_colors(self.config.effective_colors());
                }
                self.needs_redraw = true;
            }
            Err(e) => {
                log::error!("Failed to reload configuration: {}", e);
            }
        }
    }

    /// Toggle between themes
    fn toggle_theme(&mut self) {
        let new_theme = self.config.theme.next();
        log::info!("Switching theme to {:?}", new_theme);
        self.config.theme = new_theme;

        // Apply new theme colors to renderer
        if let Some(renderer) = &mut self.renderer {
            renderer.set_colors(self.config.effective_colors());
        }
        self.needs_redraw = true;
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
        let Some(terminal) = &mut self.terminal else {
            return;
        };

        let modes = terminal.screen().modes();

        // If mouse tracking is enabled, send events to PTY
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
            // Handle selection when mouse tracking is disabled
            if button == MouseButton::Left {
                if state == ElementState::Pressed {
                    let now = Instant::now();
                    let same_pos = self.last_click_pos == self.mouse_cell;
                    let time_diff = now.duration_since(self.last_click_time).as_millis();

                    // Detect double/triple click
                    if same_pos && time_diff < CLICK_THRESHOLD_MS {
                        self.click_count = (self.click_count % 3) + 1;
                    } else {
                        self.click_count = 1;
                    }

                    self.last_click_time = now;
                    self.last_click_pos = self.mouse_cell;

                    let point = Point::new(
                        self.mouse_cell.0 as usize,
                        self.mouse_cell.1 as isize - self.scroll_offset as isize,
                    );

                    let selection_type = match self.click_count {
                        1 => SelectionType::Normal,
                        2 => SelectionType::Word,
                        3 => SelectionType::Line,
                        _ => SelectionType::Normal,
                    };

                    // Start selection
                    let screen = terminal.screen_mut();
                    screen.selection_mut().start(point, selection_type);

                    // For word/line selection, expand to word/line boundaries
                    if selection_type == SelectionType::Word {
                        self.expand_word_selection();
                    } else if selection_type == SelectionType::Line {
                        self.expand_line_selection();
                    }

                    self.selecting = true;
                    self.needs_redraw = true;
                } else {
                    // Mouse released - finish selection
                    self.selecting = false;
                    terminal.screen_mut().selection_mut().finish();
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

    /// Expand selection to word boundaries
    fn expand_word_selection(&mut self) {
        let Some(terminal) = &mut self.terminal else {
            return;
        };

        let screen = terminal.screen_mut();
        let selection = screen.selection();
        if !selection.active {
            return;
        }

        let row = selection.start.row;
        let col = selection.start.col;

        // Get the line content
        if row < 0 || row as usize >= screen.rows() {
            return;
        }

        let line = screen.line(row as usize);
        let line_cols = line.cols();

        // Get the character at the click position
        let start_char = if col < line_cols {
            line.cell(col).display_char()
        } else {
            ' '
        };

        // Determine character type for word boundary detection
        #[derive(PartialEq)]
        enum CharType {
            Whitespace,
            Word,
            Other,
        }

        fn classify_char(c: char) -> CharType {
            if c.is_whitespace() {
                CharType::Whitespace
            } else if c.is_alphanumeric() || c == '_' {
                CharType::Word
            } else {
                CharType::Other
            }
        }

        let target_type = classify_char(start_char);

        // Find start of word
        let mut word_start = col;
        while word_start > 0 {
            let c = line.cell(word_start - 1).display_char();
            if classify_char(c) != target_type {
                break;
            }
            word_start -= 1;
        }

        // Find end of word
        let mut word_end = col;
        while word_end < line_cols.saturating_sub(1) {
            let c = line.cell(word_end + 1).display_char();
            if classify_char(c) != target_type {
                break;
            }
            word_end += 1;
        }

        // Update selection
        let selection = screen.selection_mut();
        selection.start.col = word_start;
        selection.end.col = word_end;
    }

    /// Expand selection to line boundaries
    fn expand_line_selection(&mut self) {
        let Some(terminal) = &mut self.terminal else {
            return;
        };

        let screen = terminal.screen_mut();
        let cols = screen.cols();
        let selection = screen.selection_mut();
        if !selection.active {
            return;
        }

        // For line selection, set columns to cover entire line
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

        let modes = terminal.screen().modes();

        // If mouse tracking is enabled, send motion events to PTY
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

    /// Handle paste from clipboard
    fn handle_paste(&mut self) {
        let Some(clipboard) = &mut self.clipboard else {
            log::warn!("Clipboard not available for paste");
            return;
        };
        let Some(terminal) = &self.terminal else {
            return;
        };
        let Some(child) = &mut self.child else { return };

        match clipboard.get_text() {
            Ok(text) => {
                let data = if terminal.screen().modes().bracketed_paste {
                    encode_bracketed_paste(&text)
                } else {
                    text.into_bytes()
                };
                if let Err(e) = child.write_all(&data) {
                    log::warn!("Failed to write paste data to PTY: {}", e);
                }
            }
            Err(e) => {
                log::warn!("Failed to get clipboard text: {}", e);
            }
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

        // Render with search highlights if search is active
        if self.search_state.active && !self.search_state.matches.is_empty() {
            let search_matches: Vec<(isize, usize, usize)> = self
                .search_state
                .matches
                .iter()
                .map(|m| (m.row, m.start_col, m.end_col))
                .collect();

            if let Err(e) = renderer.render_with_search(
                screen,
                selection,
                self.scroll_offset,
                &search_matches,
                self.search_state.current_match,
            ) {
                log::warn!("Render error: {:?}", e);
            }

            // Draw search bar overlay
            if let Err(e) = renderer.draw_search_bar(
                &self.search_state.query,
                self.search_state.matches.len(),
                self.search_state.current_match,
            ) {
                log::warn!("Search bar render error: {:?}", e);
            }
        } else if self.search_state.active {
            // Search bar is active but no matches yet
            if let Err(e) = renderer.render(screen, selection, self.scroll_offset) {
                log::warn!("Render error: {:?}", e);
            }
            if let Err(e) = renderer.draw_search_bar(&self.search_state.query, 0, 0) {
                log::warn!("Search bar render error: {:?}", e);
            }
        } else if let Err(e) = renderer.render(screen, selection, self.scroll_offset) {
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
