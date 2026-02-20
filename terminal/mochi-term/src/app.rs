//! Main application logic
//!
//! Ties together the terminal, PTY, and renderer.

use std::io;
use std::rc::Rc;
use std::time::Instant;

use arboard::Clipboard;
use regex::Regex;
use terminal_pty::{Child, WindowSize};
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::window::{Window, WindowBuilder};

use terminal_core::{Point, SelectionType};

use crate::config::Config;
use crate::input::{encode_bracketed_paste, encode_focus, encode_key, encode_mouse, MouseEvent};
use crate::renderer::{Renderer, SearchBarInfo, SearchMatch, TabInfo};
use crate::terminal::Terminal;

/// Padding added to cell height to compute tab bar height
const TAB_BAR_PADDING: u32 = 8;
/// Maximum width of a single tab in pixels
const TAB_MAX_WIDTH: u32 = 200;
/// Width of the close button area in each tab
const CLOSE_BTN_WIDTH: u32 = 20;
/// Width of the new tab (+) button
const NEW_TAB_BTN_WIDTH: u32 = 32;

/// Compute tab bar height from the current cell size so it scales with HiDPI / font size.
fn compute_tab_bar_height(cell_size: &crate::renderer::CellSize) -> u32 {
    cell_size.height as u32 + TAB_BAR_PADDING
}

/// A single search match position
#[derive(Debug, Clone)]
struct MatchPosition {
    /// Line index in unified space: 0..scrollback_len are scrollback lines,
    /// scrollback_len..scrollback_len+rows are screen lines
    line_idx: usize,
    /// Start column of the match
    start_col: usize,
    /// End column of the match (exclusive)
    end_col: usize,
}

/// Search state for a tab
#[derive(Debug, Clone)]
struct SearchState {
    /// Whether the search bar is active
    active: bool,
    /// Current search query
    query: String,
    /// Whether to use regex mode
    regex_mode: bool,
    /// All matches found
    matches: Vec<MatchPosition>,
    /// Index of the current (focused) match
    current_match: usize,
    /// Compiled regex (cached)
    compiled_regex: Option<Regex>,
    /// Whether the regex is invalid
    regex_error: bool,
}

impl SearchState {
    fn new() -> Self {
        Self {
            active: false,
            query: String::new(),
            regex_mode: false,
            matches: Vec::new(),
            current_match: 0,
            compiled_regex: None,
            regex_error: false,
        }
    }

    fn clear(&mut self) {
        self.active = false;
        self.query.clear();
        self.matches.clear();
        self.current_match = 0;
        self.compiled_regex = None;
        self.regex_error = false;
    }

    fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = (self.current_match + 1) % self.matches.len();
        }
    }

    fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            if self.current_match == 0 {
                self.current_match = self.matches.len() - 1;
            } else {
                self.current_match -= 1;
            }
        }
    }
}

/// A single terminal tab
struct Tab {
    terminal: Terminal,
    child: Child,
    title: String,
    scroll_offset: usize,
    search: SearchState,
}

impl Tab {
    fn new(terminal: Terminal, child: Child) -> Self {
        Self {
            terminal,
            child,
            title: String::from("Terminal"),
            scroll_offset: 0,
            search: SearchState::new(),
        }
    }
}

/// Application state
pub struct App {
    /// Configuration
    config: Config,
    /// Window (created on resume)
    window: Option<Rc<Window>>,
    /// Renderer
    renderer: Option<Renderer>,
    /// Tabs (each tab has its own terminal and child process)
    tabs: Vec<Tab>,
    /// Active tab index
    active_tab: usize,
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
    /// Current tab bar height in physical pixels (scales with font / HiDPI)
    tab_bar_height: u32,
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
            tabs: Vec::new(),
            active_tab: 0,
            clipboard: Clipboard::new().ok(),
            modifiers: ModifiersState::empty(),
            mouse_cell: (0, 0),
            mouse_pixel: (0.0, 0.0),
            mouse_buttons: [false; 3],
            last_render: Instant::now(),
            needs_redraw: true,
            focused: true,
            tab_bar_height: 0,
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

        // Calculate terminal dimensions (account for tab bar height)
        let cell_size = renderer.cell_size();
        self.tab_bar_height = compute_tab_bar_height(&cell_size);
        let cols = (size.width as f32 / cell_size.width) as usize;
        let terminal_height = size.height.saturating_sub(self.tab_bar_height);
        let rows = (terminal_height as f32 / cell_size.height) as usize;

        // Create first tab
        let terminal = Terminal::new(cols.max(1), rows.max(1));
        let child = Child::spawn_shell(WindowSize::new(cols as u16, rows as u16))?;
        child.set_nonblocking(true)?;

        let tab = Tab::new(terminal, child);
        self.tabs.push(tab);
        self.active_tab = 0;

        self.window = Some(window);
        self.renderer = Some(renderer);

        Ok(())
    }

    /// Create a new tab
    fn create_new_tab(&mut self) {
        let Some(renderer) = &self.renderer else {
            return;
        };
        let Some(window) = &self.window else { return };

        let size = window.inner_size();
        let cell_size = renderer.cell_size();
        let cols = (size.width as f32 / cell_size.width) as usize;
        let terminal_height = size.height.saturating_sub(self.tab_bar_height);
        let rows = (terminal_height as f32 / cell_size.height) as usize;

        let terminal = Terminal::new(cols.max(1), rows.max(1));
        match Child::spawn_shell(WindowSize::new(cols as u16, rows as u16)) {
            Ok(child) => {
                let _ = child.set_nonblocking(true);
                let tab = Tab::new(terminal, child);
                self.tabs.push(tab);
                self.active_tab = self.tabs.len() - 1;
                self.needs_redraw = true;
                log::info!("Created new tab {}", self.active_tab + 1);
            }
            Err(e) => {
                log::error!("Failed to create new tab: {}", e);
            }
        }
    }

    /// Close the current tab
    fn close_current_tab(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }

        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        self.needs_redraw = true;
        log::info!("Closed tab, now on tab {}", self.active_tab + 1);
        true
    }

    /// Switch to a specific tab (used by Cmd+1-9 on macOS)
    #[allow(dead_code)]
    fn switch_to_tab(&mut self, index: usize) {
        if index < self.tabs.len() && index != self.active_tab {
            self.active_tab = index;
            self.needs_redraw = true;
            log::info!("Switched to tab {}", index + 1);
        }
    }

    /// Handle a click in the tab bar area
    fn handle_tab_bar_click(&mut self, x: f64) {
        if self.tabs.is_empty() {
            return;
        }
        let Some(window) = &self.window else { return };

        let window_width = window.inner_size().width;
        let num_tabs = self.tabs.len() as u32;
        let available_width = window_width.saturating_sub(NEW_TAB_BTN_WIDTH);
        let tab_width = if num_tabs > 0 {
            (available_width / num_tabs).min(TAB_MAX_WIDTH)
        } else {
            TAB_MAX_WIDTH
        };

        let click_x = x as u32;
        let tabs_end = num_tabs * tab_width;

        if click_x >= tabs_end && click_x < tabs_end + NEW_TAB_BTN_WIDTH {
            self.create_new_tab();
            return;
        }

        if click_x < tabs_end {
            let tab_index = (click_x / tab_width) as usize;
            if tab_index < self.tabs.len() {
                let tab_start = tab_index as u32 * tab_width;
                let close_x_start = tab_start + tab_width.saturating_sub(CLOSE_BTN_WIDTH);

                if click_x >= close_x_start && self.tabs.len() > 1 {
                    self.tabs.remove(tab_index);
                    if self.active_tab >= self.tabs.len() {
                        self.active_tab = self.tabs.len() - 1;
                    } else if self.active_tab > tab_index {
                        self.active_tab -= 1;
                    }
                    self.needs_redraw = true;
                    log::info!("Closed tab via click {}", tab_index + 1);
                } else {
                    self.switch_to_tab(tab_index);
                }
            }
        }
    }

    /// Handle window resize
    fn handle_resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        let Some(renderer) = &mut self.renderer else {
            return;
        };

        // Update renderer
        renderer.resize(size.width, size.height);

        // Calculate new terminal dimensions (account for tab bar)
        let cell_size = renderer.cell_size();
        let cols = (size.width as f32 / cell_size.width) as usize;
        let terminal_height = size.height.saturating_sub(self.tab_bar_height);
        let rows = (terminal_height as f32 / cell_size.height) as usize;

        // Resize all tabs
        if cols > 0 && rows > 0 {
            for tab in &mut self.tabs {
                tab.terminal.resize(cols, rows);
                let _ = tab.child.resize(WindowSize::new(cols as u16, rows as u16));
            }
        }

        self.needs_redraw = true;
    }

    /// Handle keyboard input
    fn handle_key_input(&mut self, event: &winit::event::KeyEvent) {
        // If search bar is active, route input there first
        if self.handle_search_input(event) {
            return;
        }

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
                // Toggle theme: Ctrl+Shift+T (macOS only; on Linux Ctrl+Shift+T is new tab)
                #[cfg(target_os = "macos")]
                Key::Character(c) if c.to_lowercase() == "t" => {
                    self.handle_toggle_theme();
                    return;
                }
                _ => {}
            }
        }

        // macOS: Cmd+V for paste, Cmd+C for copy, Cmd+N for new window, Cmd+T for new tab,
        // Cmd+W to close tab, Cmd+1-9 to switch tabs (standard macOS shortcuts)
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
                Key::Character(c) if c.to_lowercase() == "n" => {
                    self.handle_new_window();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "t" => {
                    self.create_new_tab();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "w" => {
                    if !self.close_current_tab() {
                        // Only one tab left - close the terminal window
                        self.tabs.clear();
                    }
                    return;
                }
                Key::Character(c) if c == "1" => {
                    self.switch_to_tab(0);
                    return;
                }
                Key::Character(c) if c == "2" => {
                    self.switch_to_tab(1);
                    return;
                }
                Key::Character(c) if c == "3" => {
                    self.switch_to_tab(2);
                    return;
                }
                Key::Character(c) if c == "4" => {
                    self.switch_to_tab(3);
                    return;
                }
                Key::Character(c) if c == "5" => {
                    self.switch_to_tab(4);
                    return;
                }
                Key::Character(c) if c == "6" => {
                    self.switch_to_tab(5);
                    return;
                }
                Key::Character(c) if c == "7" => {
                    self.switch_to_tab(6);
                    return;
                }
                Key::Character(c) if c == "8" => {
                    self.switch_to_tab(7);
                    return;
                }
                Key::Character(c) if c == "9" => {
                    self.switch_to_tab(8);
                    return;
                }
                _ => {}
            }
        }

        // Linux: Ctrl+Shift+T for new tab, Ctrl+Shift+W to close tab
        #[cfg(not(target_os = "macos"))]
        if ctrl_shift {
            match &event.logical_key {
                Key::Character(c) if c.to_lowercase() == "t" && !self.modifiers.super_key() => {
                    self.create_new_tab();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "w" => {
                    if !self.close_current_tab() {
                        // Only one tab left - close the terminal window
                        self.tabs.clear();
                    }
                    return;
                }
                _ => {}
            }
        }

        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];

        // IMPORTANT: Handle control characters FIRST, before any other shortcut processing
        // This fixes the modifier state synchronization issue where ModifiersChanged and
        // KeyboardInput events can arrive out of sync.
        //
        // Use text_with_all_modifiers() from KeyEventExtModifierSupplement trait.
        // This is the proper way to get control characters in winit - it returns the text
        // with ALL modifiers applied, including Ctrl. For example:
        // - Ctrl+C produces Some("\x03")
        // - Ctrl+A produces Some("\x01")
        // The regular `text` field does NOT include Ctrl modifier effects.
        if let Some(text) = event.text_with_all_modifiers() {
            if !text.is_empty() {
                let first_char = text.chars().next().unwrap();
                // Check if it's a control character (0x01-0x1A) or DEL (0x7F)
                let char_code = first_char as u32;
                if (1..=26).contains(&char_code) || char_code == 0x7F {
                    log::debug!(
                        "Sending control character from text_with_all_modifiers: {:?} (0x{:02x})",
                        first_char,
                        first_char as u8
                    );
                    let _ = tab.child.write_all(&[first_char as u8]);
                    return;
                }
            }
        }

        // Fallback: Check if logical_key is a control character directly
        // This handles edge cases where text_with_all_modifiers might not be available
        if let Key::Character(c) = &event.logical_key {
            if let Some(ch) = c.chars().next() {
                let char_code = ch as u32;
                if (1..=26).contains(&char_code) || char_code == 0x7F {
                    log::debug!(
                        "Sending control character from logical_key: {:?} (0x{:02x})",
                        ch,
                        ch as u8
                    );
                    let _ = tab.child.write_all(&[ch as u8]);
                    return;
                }
            }
        }

        // Check for font zoom shortcuts
        // On macOS: Cmd+=/- for zoom (standard macOS behavior)
        // On Linux: Ctrl+Shift+=/- for zoom (Ctrl+arrows are used by terminal apps for word navigation)
        #[cfg(target_os = "macos")]
        let zoom_modifier = self.modifiers.super_key();
        #[cfg(not(target_os = "macos"))]
        let zoom_modifier = self.modifiers.control_key() && self.modifiers.shift_key();

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

        let application_cursor_keys = tab.terminal.screen().modes().cursor_keys_application;

        if let Some(data) = encode_key(&event.logical_key, self.modifiers, application_cursor_keys)
        {
            log::debug!("Sending key data: {:?}", data);
            let _ = tab.child.write_all(&data);
        }
    }

    /// Change font size by delta
    fn change_font_size(&mut self, delta: f32) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let Some(window) = &self.window else { return };

        let current_size = renderer.font_size();
        let new_size = (current_size + delta).clamp(8.0, 72.0);

        if (new_size - current_size).abs() < 0.1 {
            return;
        }

        renderer.set_font_size(new_size);

        // Recalculate terminal dimensions (account for tab bar)
        let size = window.inner_size();
        let cell_size = renderer.cell_size();
        self.tab_bar_height = compute_tab_bar_height(&cell_size);
        let cols = (size.width as f32 / cell_size.width) as usize;
        let terminal_height = size.height.saturating_sub(self.tab_bar_height);
        let rows = (terminal_height as f32 / cell_size.height) as usize;

        // Resize all tabs
        if cols > 0 && rows > 0 {
            for tab in &mut self.tabs {
                tab.terminal.resize(cols, rows);
                let _ = tab.child.resize(WindowSize::new(cols as u16, rows as u16));
            }
        }

        self.needs_redraw = true;
    }

    /// Reset font size to default (scaled for HiDPI)
    fn reset_font_size(&mut self) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let Some(window) = &self.window else { return };

        let scale_factor = window.scale_factor() as f32;
        let default_size = self.config.font_size() * scale_factor;

        renderer.set_font_size(default_size);

        // Recalculate terminal dimensions (account for tab bar)
        let size = window.inner_size();
        let cell_size = renderer.cell_size();
        self.tab_bar_height = compute_tab_bar_height(&cell_size);
        let cols = (size.width as f32 / cell_size.width) as usize;
        let terminal_height = size.height.saturating_sub(self.tab_bar_height);
        let rows = (terminal_height as f32 / cell_size.height) as usize;

        // Resize all tabs
        if cols > 0 && rows > 0 {
            for tab in &mut self.tabs {
                tab.terminal.resize(cols, rows);
                let _ = tab.child.resize(WindowSize::new(cols as u16, rows as u16));
            }
        }

        self.needs_redraw = true;
    }

    /// Handle mouse input
    fn handle_mouse_input(&mut self, button: MouseButton, state: ElementState) {
        if self.tabs.is_empty() {
            return;
        }

        // Handle tab bar clicks
        if button == MouseButton::Left
            && state == ElementState::Pressed
            && self.mouse_pixel.1 < self.tab_bar_height as f64
        {
            self.handle_tab_bar_click(self.mouse_pixel.0);
            return;
        }

        // Handle scrollbar dragging first (left button only)
        if button == MouseButton::Left {
            if state == ElementState::Pressed {
                // Check if click is on scrollbar (right 12 pixels of window)
                if let Some(window) = &self.window {
                    let window_width = window.inner_size().width as f64;
                    let scrollbar_width = 12.0;

                    if self.mouse_pixel.0 >= window_width - scrollbar_width
                        && self.mouse_pixel.1 >= self.tab_bar_height as f64
                    {
                        let tab = &self.tabs[self.active_tab];
                        let scrollback_len = tab.terminal.screen().scrollback().len();
                        if scrollback_len > 0 {
                            // Start scrollbar dragging
                            self.scrollbar_dragging = true;
                            self.scrollbar_drag_start_y = self.mouse_pixel.1;
                            self.scrollbar_drag_start_offset = tab.scroll_offset;
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

        let tab = &mut self.tabs[self.active_tab];
        let modes = tab.terminal.screen().modes().clone();

        // Handle text selection when mouse tracking is NOT enabled
        if !modes.mouse_tracking_enabled() {
            if button == MouseButton::Left {
                let col = self.mouse_cell.0 as usize;
                let row = self.mouse_cell.1 as isize - tab.scroll_offset as isize;

                if state == ElementState::Pressed {
                    // Start a new selection
                    tab.terminal
                        .screen_mut()
                        .selection_mut()
                        .start(Point::new(col, row), SelectionType::Normal);
                    self.needs_redraw = true;
                } else {
                    // Finish selection
                    tab.terminal.screen_mut().selection_mut().finish();
                }
            }
            // Track button state for selection dragging
            let idx = match button {
                MouseButton::Left => 0,
                MouseButton::Middle => 1,
                MouseButton::Right => 2,
                _ => return,
            };
            self.mouse_buttons[idx] = state == ElementState::Pressed;
            return;
        }

        // Mouse tracking is enabled - send events to PTY
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
            let _ = tab.child.write_all(&data);
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

        if self.tabs.is_empty() {
            return;
        }

        // Handle scrollbar dragging
        if self.scrollbar_dragging {
            if let Some(window) = &self.window {
                let tab = &mut self.tabs[self.active_tab];
                let window_height =
                    (window.inner_size().height as f64 - self.tab_bar_height as f64).max(1.0);
                let scrollback_len = tab.terminal.screen().scrollback().len();
                let visible_rows = tab.terminal.screen().rows();

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

                        if new_offset != tab.scroll_offset {
                            tab.scroll_offset = new_offset;
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

        let cell_size = renderer.cell_size();
        let col = (position.x / cell_size.width as f64) as u16;
        let adjusted_y = (position.y - self.tab_bar_height as f64).max(0.0);
        let row = (adjusted_y / cell_size.height as f64) as u16;

        if col == self.mouse_cell.0 && row == self.mouse_cell.1 {
            return;
        }

        self.mouse_cell = (col, row);

        let tab = &mut self.tabs[self.active_tab];
        let modes = tab.terminal.screen().modes().clone();

        // Handle text selection dragging when mouse tracking is NOT enabled
        if !modes.mouse_tracking_enabled() && self.mouse_buttons[0] {
            // Left button is held - update selection
            let sel_col = col as usize;
            let sel_row = row as isize - tab.scroll_offset as isize;
            tab.terminal
                .screen_mut()
                .selection_mut()
                .update(Point::new(sel_col, sel_row));
            self.needs_redraw = true;
            return;
        }

        // Mouse tracking is enabled - send events to PTY
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
                let _ = tab.child.write_all(&data);
            }
        }
    }

    /// Handle mouse scroll
    fn handle_mouse_scroll(&mut self, delta: MouseScrollDelta) {
        if self.tabs.is_empty() {
            return;
        }

        let tab = &mut self.tabs[self.active_tab];
        let modes = tab.terminal.screen().modes().clone();
        let lines = match delta {
            MouseScrollDelta::LineDelta(_, y) => y as i32,
            MouseScrollDelta::PixelDelta(pos) => (pos.y / 20.0) as i32,
        };

        if lines == 0 {
            return;
        }

        // If mouse tracking is enabled or in alternate screen, send to PTY
        if modes.mouse_tracking_enabled() || modes.alternate_screen {
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
                let _ = tab.child.write_all(&data);
            }
        } else {
            // Scroll the viewport through scrollback history
            let scrollback_len = tab.terminal.screen().scrollback().len();
            if lines > 0 {
                // Scroll up (show older content)
                tab.scroll_offset = (tab.scroll_offset + lines as usize).min(scrollback_len);
            } else {
                // Scroll down (show newer content)
                tab.scroll_offset = tab.scroll_offset.saturating_sub((-lines) as usize);
            }
            self.needs_redraw = true;
        }
    }

    /// Handle copy (Ctrl+Shift+C)
    fn handle_copy(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let tab = &self.tabs[self.active_tab];

        let screen = tab.terminal.screen();
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
        if self.tabs.is_empty() {
            return;
        }

        let tab = &mut self.tabs[self.active_tab];

        match clipboard.get_text() {
            Ok(text) => {
                if text.is_empty() {
                    return;
                }
                let data = if tab.terminal.screen().modes().bracketed_paste {
                    encode_bracketed_paste(&text)
                } else {
                    text.into_bytes()
                };
                if let Err(e) = tab.child.write_all(&data) {
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

    /// Handle find (Ctrl+Shift+F) - toggle search bar
    fn handle_find(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];
        if tab.search.active {
            tab.search.clear();
        } else {
            tab.search.active = true;
            tab.search.query.clear();
            tab.search.matches.clear();
            tab.search.current_match = 0;
            tab.search.regex_error = false;
        }
        self.needs_redraw = true;
    }

    fn line_text_and_boundaries(line: &terminal_core::Line) -> (String, Vec<(usize, usize)>) {
        let mut text = String::new();
        let mut boundaries = Vec::with_capacity(line.cols() + 1);
        let mut byte_idx = 0usize;
        let mut col = 0usize;

        for i in 0..line.cols() {
            let cell = line.cell(i);
            if cell.is_continuation() {
                continue;
            }

            boundaries.push((byte_idx, col));

            let content = if cell.content().is_empty() {
                " "
            } else {
                cell.content()
            };
            text.push_str(content);

            byte_idx += content.len();
            col += cell.width() as usize;
        }

        boundaries.push((byte_idx, col));
        (text, boundaries)
    }

    fn byte_idx_to_col(boundaries: &[(usize, usize)], byte_idx: usize) -> usize {
        match boundaries.binary_search_by_key(&byte_idx, |(b, _)| *b) {
            Ok(i) => boundaries[i].1,
            Err(i) => {
                if i == 0 {
                    0
                } else {
                    boundaries[i - 1].1
                }
            }
        }
    }

    /// Run incremental search over scrollback + visible screen for the active tab
    fn run_search(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];
        tab.search.matches.clear();
        tab.search.current_match = 0;
        tab.search.regex_error = false;

        if tab.search.query.is_empty() {
            return;
        }

        let screen = tab.terminal.screen();
        let scrollback = screen.scrollback();
        let scrollback_len = scrollback.len();
        let rows = screen.rows();

        let compiled = if tab.search.regex_mode {
            match Regex::new(&tab.search.query) {
                Ok(re) => Some(re),
                Err(_) => {
                    tab.search.regex_error = true;
                    return;
                }
            }
        } else {
            Regex::new(&regex::escape(&tab.search.query)).ok()
        };

        let Some(re) = compiled else { return };

        // Search scrollback lines (oldest to newest)
        for i in 0..scrollback_len {
            if let Some(line) = scrollback.get(i) {
                let (text, boundaries) = Self::line_text_and_boundaries(line);
                for mat in re.find_iter(&text) {
                    let start_col = Self::byte_idx_to_col(&boundaries, mat.start());
                    let end_col = Self::byte_idx_to_col(&boundaries, mat.end());
                    tab.search.matches.push(MatchPosition {
                        line_idx: i,
                        start_col,
                        end_col,
                    });
                }
            }
        }

        // Search visible screen lines
        for row in 0..rows {
            let line = screen.line(row);
            let (text, boundaries) = Self::line_text_and_boundaries(line);
            for mat in re.find_iter(&text) {
                let start_col = Self::byte_idx_to_col(&boundaries, mat.start());
                let end_col = Self::byte_idx_to_col(&boundaries, mat.end());
                tab.search.matches.push(MatchPosition {
                    line_idx: scrollback_len + row,
                    start_col,
                    end_col,
                });
            }
        }

        tab.search.compiled_regex = Some(re);
    }

    /// Scroll viewport so the current search match is visible
    fn scroll_to_current_match(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];
        if tab.search.matches.is_empty() {
            return;
        }

        let current = &tab.search.matches[tab.search.current_match];
        let scrollback_len = tab.terminal.screen().scrollback().len();
        let rows = tab.terminal.screen().rows();

        if current.line_idx < scrollback_len {
            // Match is in scrollback: compute scroll_offset so match line is visible
            let lines_from_end = scrollback_len - current.line_idx;
            // Place the match roughly in the middle of the screen
            let half_screen = rows / 2;
            tab.scroll_offset = if lines_from_end > half_screen {
                lines_from_end - half_screen
            } else {
                0
            };
            tab.scroll_offset = tab.scroll_offset.min(scrollback_len);
        } else {
            // Match is on the visible screen - just scroll to bottom
            tab.scroll_offset = 0;
        }
    }

    /// Handle keyboard input when search bar is active.
    /// Returns true if the event was consumed by the search bar.
    fn handle_search_input(&mut self, event: &winit::event::KeyEvent) -> bool {
        if self.tabs.is_empty() {
            return false;
        }
        if !self.tabs[self.active_tab].search.active {
            return false;
        }
        if event.state != ElementState::Pressed {
            return true; // consume release events too
        }

        match &event.logical_key {
            Key::Named(NamedKey::Escape) => {
                self.tabs[self.active_tab].search.clear();
                self.needs_redraw = true;
            }
            Key::Named(NamedKey::Enter) => {
                if self.modifiers.shift_key() {
                    self.tabs[self.active_tab].search.prev_match();
                } else {
                    self.tabs[self.active_tab].search.next_match();
                }
                self.scroll_to_current_match();
                self.needs_redraw = true;
            }
            Key::Named(NamedKey::Backspace) => {
                self.tabs[self.active_tab].search.query.pop();
                self.run_search();
                if !self.tabs[self.active_tab].search.matches.is_empty() {
                    self.scroll_to_current_match();
                }
                self.needs_redraw = true;
            }
            Key::Named(NamedKey::Tab) => {
                // Toggle regex mode
                let tab = &mut self.tabs[self.active_tab];
                tab.search.regex_mode = !tab.search.regex_mode;
                self.run_search();
                self.needs_redraw = true;
            }
            Key::Character(c) => {
                // Don't consume Ctrl+Shift+F (allow toggling search off)
                if self.modifiers.control_key() && self.modifiers.shift_key() {
                    return false;
                }
                // Ignore other ctrl combos in search
                if self.modifiers.control_key() || self.modifiers.super_key() {
                    return true;
                }
                self.tabs[self.active_tab]
                    .search
                    .query
                    .push_str(c.as_str());
                self.run_search();
                if !self.tabs[self.active_tab].search.matches.is_empty() {
                    self.scroll_to_current_match();
                }
                self.needs_redraw = true;
            }
            _ => {}
        }
        true
    }

    /// Handle new window (Cmd+N on macOS)
    ///
    /// Spawns a new instance of the Mochi terminal.
    #[cfg(target_os = "macos")]
    fn handle_new_window(&mut self) {
        log::info!("Opening new terminal window...");

        // Get the path to the current executable
        if let Ok(exe_path) = std::env::current_exe() {
            match std::process::Command::new(&exe_path).spawn() {
                Ok(child) => {
                    log::info!("New terminal window spawned successfully");
                    // Spawn a thread to wait on the child process to prevent zombie processes.
                    // When the child exits, this thread will reap it by calling wait().
                    std::thread::spawn(move || {
                        let mut child = child;
                        let _ = child.wait();
                    });
                }
                Err(e) => {
                    log::error!("Failed to spawn new terminal window: {}", e);
                }
            }
        } else {
            log::error!("Failed to get current executable path");
        }
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

    /// Handle toggle theme (Ctrl+Shift+T on macOS)
    #[allow(dead_code)]
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

        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];

        if tab.terminal.screen().modes().focus_events {
            let data = encode_focus(focused);
            let _ = tab.child.write_all(&data);
        }
    }

    /// Poll PTY for output from all tabs
    fn poll_pty(&mut self) {
        let mut buf = [0u8; 65536];

        // Poll all tabs for output
        for (i, tab) in self.tabs.iter_mut().enumerate() {
            let mut received_output = false;

            loop {
                match tab.child.pty_mut().try_read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        tab.terminal.process(&buf[..n]);
                        received_output = true;
                        // Only trigger redraw if synchronized output mode is disabled
                        // and this is the active tab
                        if i == self.active_tab && !tab.terminal.is_synchronized_output() {
                            self.needs_redraw = true;
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                    Err(_) => break,
                }
            }

            // Reset scroll offset when new output arrives (auto-scroll to bottom)
            if received_output && tab.scroll_offset > 0 {
                tab.scroll_offset = 0;
            }

            // Check for title change (only update window title for active tab)
            if tab.terminal.take_title_changed() {
                tab.title = tab.terminal.title().to_string();
                if i == self.active_tab {
                    if let Some(window) = &self.window {
                        window.set_title(&tab.title);
                    }
                }
            }

            // Check for bell
            if tab.terminal.take_bell() {
                log::debug!("Bell!");
            }

            // Send any pending responses back to the PTY (DSR, DA1, etc.)
            let responses = tab.terminal.take_pending_responses();
            for response in responses {
                if let Err(e) = tab.child.write_all(&response) {
                    log::warn!("Failed to send response to PTY: {}", e);
                }
            }
        }
    }

    /// Render the terminal
    fn render(&mut self) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };

        if self.tabs.is_empty() {
            return;
        }

        let tab_infos: Vec<TabInfo<'_>> = self
            .tabs
            .iter()
            .map(|t| TabInfo { title: &t.title })
            .collect();
        let tab = &self.tabs[self.active_tab];
        let screen = tab.terminal.screen();
        let selection = screen.selection();
        let scrollback_len = screen.scrollback().len();
        let rows = screen.rows();

        // Build search info for the renderer
        let search_bar = if tab.search.active {
            Some(SearchBarInfo {
                query: &tab.search.query,
                regex_mode: tab.search.regex_mode,
                match_count: tab.search.matches.len(),
                current_match: if tab.search.matches.is_empty() {
                    0
                } else {
                    tab.search.current_match + 1
                },
                regex_error: tab.search.regex_error,
            })
        } else {
            None
        };

        // Build visible search matches for the renderer
        let search_matches: Vec<SearchMatch> = if tab.search.active {
            let scroll_offset = tab.scroll_offset;
            tab.search
                .matches
                .iter()
                .enumerate()
                .filter_map(|(i, m)| {
                    // Convert unified line_idx to a display row
                    let display_row = if scroll_offset > 0 {
                        let view_start = scrollback_len.saturating_sub(scroll_offset);
                        let view_end = view_start + rows;
                        if m.line_idx >= view_start && m.line_idx < view_end {
                            Some((m.line_idx - view_start) as usize)
                        } else {
                            None
                        }
                    } else {
                        // No scroll: only screen lines are visible
                        if m.line_idx >= scrollback_len
                            && m.line_idx < scrollback_len + rows
                        {
                            Some(m.line_idx - scrollback_len)
                        } else {
                            None
                        }
                    };
                    display_row.map(|row| SearchMatch {
                        row,
                        start_col: m.start_col,
                        end_col: m.end_col,
                        is_current: i == tab.search.current_match,
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        if let Err(e) = renderer.render(
            screen,
            selection,
            tab.scroll_offset,
            self.tab_bar_height,
            &tab_infos,
            self.active_tab,
            search_bar.as_ref(),
            &search_matches,
        ) {
            log::warn!("Render error: {:?}", e);
        }

        self.needs_redraw = false;
        self.last_render = Instant::now();
    }

    /// Check if active tab's child is still running
    fn check_child(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }

        // Check if active tab's child is running
        let active_running = self.tabs[self.active_tab].child.is_running();

        // Remove any tabs whose children have exited
        self.tabs.retain(|tab| tab.child.is_running());

        // Adjust active tab index if needed
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len().saturating_sub(1);
        }

        // Return true if there are still tabs with running children
        !self.tabs.is_empty() && (active_running || self.tabs[self.active_tab].child.is_running())
    }
}
