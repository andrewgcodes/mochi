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
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::window::{Window, WindowBuilder};

use terminal_core::{Point, SelectionType};

use crate::config::Config;
use crate::input::{encode_bracketed_paste, encode_focus, encode_key, encode_mouse, MouseEvent};
use crate::pane::{
    divider_at_position, find_pane_in_direction, NavigationDirection, PaneId, PaneNode, PaneRect,
    SplitDirection, DIVIDER_WIDTH,
};
use crate::renderer::{PaneRenderInfo, Renderer, TabInfo};
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

/// A single terminal tab containing a tree of panes
struct Tab {
    /// Root of the pane tree
    pane_root: PaneNode,
    /// ID of the currently active (focused) pane
    active_pane_id: PaneId,
    /// Title shown in the tab bar
    title: String,
}

impl Tab {
    fn new(pane_root: PaneNode, active_pane_id: PaneId) -> Self {
        Self {
            pane_root,
            active_pane_id,
            title: String::from("Terminal"),
        }
    }
}

/// State for divider dragging
struct DividerDragState {
    /// Index into the dividers list
    divider_index: usize,
    /// Direction of the divider
    direction: SplitDirection,
    /// Parent rect of the split being resized
    parent_rect: PaneRect,
}

/// Application state
pub struct App {
    /// Configuration
    config: Config,
    /// Window (created on resume)
    window: Option<Rc<Window>>,
    /// Renderer
    renderer: Option<Renderer>,
    /// Tabs (each tab has its own pane tree)
    tabs: Vec<Tab>,
    /// Active tab index
    active_tab: usize,
    /// Clipboard
    #[allow(dead_code)]
    clipboard: Option<Clipboard>,
    /// Current modifiers state
    modifiers: ModifiersState,
    /// Mouse position (in cells, relative to active pane)
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
    /// Next pane ID counter
    next_pane_id: PaneId,
    /// Divider drag state
    divider_drag: Option<DividerDragState>,
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
            next_pane_id: 1,
            divider_drag: None,
        })
    }

    /// Allocate a new unique pane ID
    fn alloc_pane_id(&mut self) -> PaneId {
        let id = self.next_pane_id;
        self.next_pane_id += 1;
        id
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

                    // Render directly if needed
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

        let renderer = Renderer::new(
            window.clone(),
            self.config.font_size(),
            self.config.effective_colors(),
        )?;

        let cell_size = renderer.cell_size();
        self.tab_bar_height = compute_tab_bar_height(&cell_size);
        let cols = (size.width as f32 / cell_size.width) as usize;
        let terminal_height = size.height.saturating_sub(self.tab_bar_height);
        let rows = (terminal_height as f32 / cell_size.height) as usize;

        let pane_id = self.alloc_pane_id();
        let terminal = Terminal::new(cols.max(1), rows.max(1));
        let child = Child::spawn_shell(WindowSize::new(cols as u16, rows as u16))?;
        child.set_nonblocking(true)?;

        let pane_root = PaneNode::leaf(pane_id, terminal, child);
        let tab = Tab::new(pane_root, pane_id);
        self.tabs.push(tab);
        self.active_tab = 0;

        self.window = Some(window);
        self.renderer = Some(renderer);

        Ok(())
    }

    /// Get the available rect for panes (window minus tab bar)
    fn pane_available_rect(&self) -> PaneRect {
        let Some(window) = &self.window else {
            return PaneRect::new(0, 0, 800, 600);
        };
        let size = window.inner_size();
        let terminal_height = size.height.saturating_sub(self.tab_bar_height);
        PaneRect::new(0, self.tab_bar_height, size.width, terminal_height)
    }

    /// Resize all panes in the current tab to match their calculated rects
    fn resize_all_panes(&mut self) {
        let Some(renderer) = &self.renderer else {
            return;
        };

        let available = self.pane_available_rect();
        let cell_size = renderer.cell_size();

        if self.tabs.is_empty() {
            return;
        }

        let tab = &mut self.tabs[self.active_tab];
        let rects = tab.pane_root.calculate_rects(available);

        for (pane_id, rect) in rects {
            let cols = (rect.w as f32 / cell_size.width) as usize;
            let rows = (rect.h as f32 / cell_size.height) as usize;
            if cols > 0 && rows > 0 {
                if let Some(pane) = tab.pane_root.find_pane_mut(pane_id) {
                    pane.terminal.resize(cols, rows);
                    let _ = pane.child.resize(WindowSize::new(cols as u16, rows as u16));
                }
            }
        }
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

        let pane_id = self.alloc_pane_id();
        let terminal = Terminal::new(cols.max(1), rows.max(1));
        match Child::spawn_shell(WindowSize::new(cols as u16, rows as u16)) {
            Ok(child) => {
                let _ = child.set_nonblocking(true);
                let pane_root = PaneNode::leaf(pane_id, terminal, child);
                let tab = Tab::new(pane_root, pane_id);
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
    #[allow(dead_code)]
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

    /// Switch to a specific tab
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

        renderer.resize(size.width, size.height);

        let cell_size = renderer.cell_size();
        let available = PaneRect::new(
            0,
            self.tab_bar_height,
            size.width,
            size.height.saturating_sub(self.tab_bar_height),
        );

        // Resize all panes in all tabs
        for tab in &mut self.tabs {
            let rects = tab.pane_root.calculate_rects(available);
            for (pane_id, rect) in rects {
                let cols = (rect.w as f32 / cell_size.width) as usize;
                let rows = (rect.h as f32 / cell_size.height) as usize;
                if cols > 0 && rows > 0 {
                    if let Some(pane) = tab.pane_root.find_pane_mut(pane_id) {
                        pane.terminal.resize(cols, rows);
                        let _ = pane.child.resize(WindowSize::new(cols as u16, rows as u16));
                    }
                }
            }
        }

        self.needs_redraw = true;
    }

    /// Split the active pane in the given direction
    fn split_active_pane(&mut self, direction: SplitDirection) {
        let Some(renderer) = &self.renderer else {
            return;
        };

        if self.tabs.is_empty() {
            return;
        }

        let cell_size = renderer.cell_size();
        let available = self.pane_available_rect();
        let tab = &self.tabs[self.active_tab];
        let active_pane_id = tab.active_pane_id;

        let rects = tab.pane_root.calculate_rects(available);
        let active_rect = rects
            .iter()
            .find(|(id, _)| *id == active_pane_id)
            .map(|(_, r)| *r);
        let Some(active_rect) = active_rect else {
            return;
        };

        // Calculate the size for the new pane (half of active pane minus divider)
        let (new_cols, new_rows) = match direction {
            SplitDirection::Vertical => {
                let half_w = (active_rect.w.saturating_sub(DIVIDER_WIDTH)) / 2;
                let cols = (half_w as f32 / cell_size.width) as usize;
                let rows = (active_rect.h as f32 / cell_size.height) as usize;
                (cols, rows)
            }
            SplitDirection::Horizontal => {
                let cols = (active_rect.w as f32 / cell_size.width) as usize;
                let half_h = (active_rect.h.saturating_sub(DIVIDER_WIDTH)) / 2;
                let rows = (half_h as f32 / cell_size.height) as usize;
                (cols, rows)
            }
        };

        if new_cols == 0 || new_rows == 0 {
            log::warn!("Pane too small to split");
            return;
        }

        let new_pane_id = self.alloc_pane_id();
        let terminal = Terminal::new(new_cols.max(1), new_rows.max(1));
        match Child::spawn_shell(WindowSize::new(new_cols as u16, new_rows as u16)) {
            Ok(child) => {
                let _ = child.set_nonblocking(true);
                let new_pane = crate::pane::Pane::new(new_pane_id, terminal, child);

                let tab = &mut self.tabs[self.active_tab];
                if tab
                    .pane_root
                    .split_pane(active_pane_id, direction, new_pane)
                {
                    tab.active_pane_id = new_pane_id;
                    log::info!(
                        "Split pane {} {:?}, new pane {}",
                        active_pane_id,
                        direction,
                        new_pane_id
                    );

                    self.resize_all_panes();
                    self.needs_redraw = true;
                }
            }
            Err(e) => {
                log::error!("Failed to spawn shell for new pane: {}", e);
            }
        }
    }

    /// Close the active pane. If it's the last pane, close the tab.
    /// Returns false if the entire application should exit.
    fn close_active_pane(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }

        let tab = &mut self.tabs[self.active_tab];
        let active_id = tab.active_pane_id;

        // If only one pane, close the tab
        if tab.pane_root.pane_count() <= 1 {
            if self.tabs.len() <= 1 {
                self.tabs.clear();
                return false;
            }
            self.tabs.remove(self.active_tab);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
            self.needs_redraw = true;
            return true;
        }

        // Get remaining pane IDs before removing
        let remaining_ids: Vec<PaneId> = tab
            .pane_root
            .all_pane_ids()
            .into_iter()
            .filter(|id| *id != active_id)
            .collect();

        if tab.pane_root.remove_pane(active_id) {
            if let Some(&new_active) = remaining_ids.first() {
                tab.active_pane_id = new_active;
            }
            self.resize_all_panes();
            self.needs_redraw = true;
            log::info!("Closed pane {}", active_id);
        }
        true
    }

    /// Navigate to a pane in the given direction
    fn navigate_pane(&mut self, direction: NavigationDirection) {
        if self.tabs.is_empty() {
            return;
        }

        let tab = &self.tabs[self.active_tab];
        let available = self.pane_available_rect();
        let rects = tab.pane_root.calculate_rects(available);

        if let Some(target_id) = find_pane_in_direction(&rects, tab.active_pane_id, direction) {
            let tab = &mut self.tabs[self.active_tab];
            tab.active_pane_id = target_id;
            self.needs_redraw = true;
            log::info!("Navigated to pane {}", target_id);
        }
    }

    /// Find which pane contains the given pixel position
    fn pane_at_pixel(&self, px: f64, py: f64) -> Option<PaneId> {
        if self.tabs.is_empty() {
            return None;
        }

        let tab = &self.tabs[self.active_tab];
        let available = self.pane_available_rect();
        let rects = tab.pane_root.calculate_rects(available);

        rects
            .iter()
            .find(|(_, rect)| rect.contains(px as u32, py as u32))
            .map(|(id, _)| *id)
    }

    /// Get the pixel rect for the active pane
    fn active_pane_rect(&self) -> Option<PaneRect> {
        if self.tabs.is_empty() {
            return None;
        }

        let tab = &self.tabs[self.active_tab];
        let available = self.pane_available_rect();
        let rects = tab.pane_root.calculate_rects(available);

        rects
            .iter()
            .find(|(id, _)| *id == tab.active_pane_id)
            .map(|(_, rect)| *rect)
    }

    /// Handle keyboard input
    fn handle_key_input(&mut self, event: &winit::event::KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }

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
                // Toggle theme: Ctrl+Shift+T (macOS only)
                #[cfg(target_os = "macos")]
                Key::Character(c) if c.to_lowercase() == "t" => {
                    self.handle_toggle_theme();
                    return;
                }
                // Split pane vertically (left/right): Ctrl+Shift+D
                Key::Character(c) if c.to_lowercase() == "d" => {
                    self.split_active_pane(SplitDirection::Vertical);
                    return;
                }
                // Split pane horizontally (top/bottom): Ctrl+Shift+E
                Key::Character(c) if c.to_lowercase() == "e" => {
                    self.split_active_pane(SplitDirection::Horizontal);
                    return;
                }
                // Navigate between panes: Ctrl+Shift+Alt+Arrow
                // (uses Alt modifier to avoid shadowing Ctrl+Shift+Arrow font zoom on Linux)
                Key::Named(NamedKey::ArrowLeft) if self.modifiers.alt_key() => {
                    self.navigate_pane(NavigationDirection::Left);
                    return;
                }
                Key::Named(NamedKey::ArrowRight) if self.modifiers.alt_key() => {
                    self.navigate_pane(NavigationDirection::Right);
                    return;
                }
                Key::Named(NamedKey::ArrowUp) if self.modifiers.alt_key() => {
                    self.navigate_pane(NavigationDirection::Up);
                    return;
                }
                Key::Named(NamedKey::ArrowDown) if self.modifiers.alt_key() => {
                    self.navigate_pane(NavigationDirection::Down);
                    return;
                }
                _ => {}
            }
        }

        // macOS: Cmd+V for paste, Cmd+C for copy, Cmd+N for new window, Cmd+T for new tab,
        // Cmd+W to close pane/tab, Cmd+D for vertical split, Cmd+1-9 to switch tabs
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
                Key::Character(c) if c.to_lowercase() == "d" => {
                    if self.modifiers.shift_key() {
                        self.split_active_pane(SplitDirection::Horizontal);
                    } else {
                        self.split_active_pane(SplitDirection::Vertical);
                    }
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "w" => {
                    if !self.close_active_pane() {
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

        // Linux: Ctrl+Shift+T for new tab, Ctrl+Shift+W to close pane/tab
        #[cfg(not(target_os = "macos"))]
        if ctrl_shift {
            match &event.logical_key {
                Key::Character(c) if c.to_lowercase() == "t" && !self.modifiers.super_key() => {
                    self.create_new_tab();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "w" => {
                    if !self.close_active_pane() {
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
        let active_pane_id = tab.active_pane_id;
        let pane = match tab.pane_root.find_pane_mut(active_pane_id) {
            Some(p) => p,
            None => return,
        };

        // IMPORTANT: Handle control characters FIRST, before any other shortcut processing
        if let Some(text) = event.text_with_all_modifiers() {
            if !text.is_empty() {
                let first_char = text.chars().next().unwrap();
                let char_code = first_char as u32;
                if (1..=26).contains(&char_code) || char_code == 0x7F {
                    log::debug!(
                        "Sending control character from text_with_all_modifiers: {:?} (0x{:02x})",
                        first_char,
                        first_char as u8
                    );
                    let _ = pane.child.write_all(&[first_char as u8]);
                    return;
                }
            }
        }

        // Fallback: Check if logical_key is a control character directly
        if let Key::Character(c) = &event.logical_key {
            if let Some(ch) = c.chars().next() {
                let char_code = ch as u32;
                if (1..=26).contains(&char_code) || char_code == 0x7F {
                    log::debug!(
                        "Sending control character from logical_key: {:?} (0x{:02x})",
                        ch,
                        ch as u8
                    );
                    let _ = pane.child.write_all(&[ch as u8]);
                    return;
                }
            }
        }

        // Check for font zoom shortcuts
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

        let application_cursor_keys = pane.terminal.screen().modes().cursor_keys_application;

        if let Some(data) = encode_key(&event.logical_key, self.modifiers, application_cursor_keys)
        {
            log::debug!("Sending key data: {:?}", data);
            let _ = pane.child.write_all(&data);
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

        let cell_size = renderer.cell_size();
        self.tab_bar_height = compute_tab_bar_height(&cell_size);

        let size = window.inner_size();
        let available = PaneRect::new(
            0,
            self.tab_bar_height,
            size.width,
            size.height.saturating_sub(self.tab_bar_height),
        );

        for tab in &mut self.tabs {
            let rects = tab.pane_root.calculate_rects(available);
            for (pane_id, rect) in rects {
                let cols = (rect.w as f32 / cell_size.width) as usize;
                let rows = (rect.h as f32 / cell_size.height) as usize;
                if cols > 0 && rows > 0 {
                    if let Some(pane) = tab.pane_root.find_pane_mut(pane_id) {
                        pane.terminal.resize(cols, rows);
                        let _ = pane.child.resize(WindowSize::new(cols as u16, rows as u16));
                    }
                }
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

        let cell_size = renderer.cell_size();
        self.tab_bar_height = compute_tab_bar_height(&cell_size);

        let size = window.inner_size();
        let available = PaneRect::new(
            0,
            self.tab_bar_height,
            size.width,
            size.height.saturating_sub(self.tab_bar_height),
        );

        for tab in &mut self.tabs {
            let rects = tab.pane_root.calculate_rects(available);
            for (pane_id, rect) in rects {
                let cols = (rect.w as f32 / cell_size.width) as usize;
                let rows = (rect.h as f32 / cell_size.height) as usize;
                if cols > 0 && rows > 0 {
                    if let Some(pane) = tab.pane_root.find_pane_mut(pane_id) {
                        pane.terminal.resize(cols, rows);
                        let _ = pane.child.resize(WindowSize::new(cols as u16, rows as u16));
                    }
                }
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

        // Handle divider dragging
        if button == MouseButton::Left {
            if state == ElementState::Pressed {
                let available = self.pane_available_rect();
                let tab = &self.tabs[self.active_tab];
                let dividers = tab.pane_root.calculate_dividers(available);
                if let Some(div_idx) = divider_at_position(
                    &dividers,
                    self.mouse_pixel.0 as u32,
                    self.mouse_pixel.1 as u32,
                ) {
                    let div = &dividers[div_idx];
                    self.divider_drag = Some(DividerDragState {
                        divider_index: div_idx,
                        direction: div.direction,
                        parent_rect: div.parent_rect,
                    });
                    return;
                }
            } else if self.divider_drag.is_some() {
                self.divider_drag = None;
                return;
            }
        }

        // Handle scrollbar dragging (only within active pane)
        if button == MouseButton::Left {
            if state == ElementState::Pressed {
                if let Some(pane_rect) = self.active_pane_rect() {
                    let scrollbar_width = 12.0;
                    let px = self.mouse_pixel.0;
                    let py = self.mouse_pixel.1;

                    if px >= (pane_rect.x + pane_rect.w) as f64 - scrollbar_width
                        && px < (pane_rect.x + pane_rect.w) as f64
                        && py >= pane_rect.y as f64
                        && py < (pane_rect.y + pane_rect.h) as f64
                    {
                        let tab = &self.tabs[self.active_tab];
                        if let Some(pane) = tab.pane_root.find_pane(tab.active_pane_id) {
                            let scrollback_len = pane.terminal.screen().scrollback().len();
                            if scrollback_len > 0 {
                                self.scrollbar_dragging = true;
                                self.scrollbar_drag_start_y = py;
                                self.scrollbar_drag_start_offset = pane.scroll_offset;
                                return;
                            }
                        }
                    }
                }
            } else if self.scrollbar_dragging {
                self.scrollbar_dragging = false;
                return;
            }
        }

        // Clicking on a pane focuses it
        if button == MouseButton::Left && state == ElementState::Pressed {
            if let Some(pane_id) = self.pane_at_pixel(self.mouse_pixel.0, self.mouse_pixel.1) {
                let tab = &mut self.tabs[self.active_tab];
                if pane_id != tab.active_pane_id {
                    tab.active_pane_id = pane_id;
                    self.needs_redraw = true;
                }
            }
        }

        // Recompute mouse_cell relative to the (possibly new) active pane's rect
        // so that selection start and mouse tracking use correct coordinates.
        if let Some(renderer) = &self.renderer {
            let cell_size = renderer.cell_size();
            let available = self.pane_available_rect();
            let tab = &self.tabs[self.active_tab];
            let rects = tab.pane_root.calculate_rects(available);
            if let Some((_, pane_rect)) = rects.iter().find(|(id, _)| *id == tab.active_pane_id) {
                let col = ((self.mouse_pixel.0 - pane_rect.x as f64).max(0.0)
                    / cell_size.width as f64) as u16;
                let row = ((self.mouse_pixel.1 - pane_rect.y as f64).max(0.0)
                    / cell_size.height as f64) as u16;
                self.mouse_cell = (col, row);
            }
        }

        let tab = &mut self.tabs[self.active_tab];
        let active_pane_id = tab.active_pane_id;
        let pane = match tab.pane_root.find_pane_mut(active_pane_id) {
            Some(p) => p,
            None => return,
        };
        let modes = pane.terminal.screen().modes().clone();

        // Handle text selection when mouse tracking is NOT enabled
        if !modes.mouse_tracking_enabled() {
            if button == MouseButton::Left {
                let col = self.mouse_cell.0 as usize;
                let row = self.mouse_cell.1 as isize - pane.scroll_offset as isize;

                if state == ElementState::Pressed {
                    pane.terminal
                        .screen_mut()
                        .selection_mut()
                        .start(Point::new(col, row), SelectionType::Normal);
                    self.needs_redraw = true;
                } else {
                    pane.terminal.screen_mut().selection_mut().finish();
                }
            }
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
            let _ = pane.child.write_all(&data);
        }

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
        self.mouse_pixel = (position.x, position.y);

        if self.tabs.is_empty() {
            return;
        }

        // Handle divider dragging
        if let Some(drag) = &self.divider_drag {
            let div_idx = drag.divider_index;
            let new_ratio = match drag.direction {
                SplitDirection::Vertical => {
                    let parent_x = drag.parent_rect.x as f64;
                    let usable_w = (drag.parent_rect.w - DIVIDER_WIDTH) as f64;
                    ((position.x - parent_x) / usable_w) as f32
                }
                SplitDirection::Horizontal => {
                    let parent_y = drag.parent_rect.y as f64;
                    let usable_h = (drag.parent_rect.h - DIVIDER_WIDTH) as f64;
                    ((position.y - parent_y) / usable_h) as f32
                }
            };

            let tab = &mut self.tabs[self.active_tab];
            let mut n = div_idx;
            tab.pane_root.update_nth_split_ratio(&mut n, new_ratio);

            self.resize_all_panes();
            self.needs_redraw = true;
            return;
        }

        // Handle scrollbar dragging
        if self.scrollbar_dragging {
            if let Some(pane_rect) = self.active_pane_rect() {
                let tab = &mut self.tabs[self.active_tab];
                let active_pane_id = tab.active_pane_id;
                if let Some(pane) = tab.pane_root.find_pane_mut(active_pane_id) {
                    let window_height = pane_rect.h as f64;
                    let scrollback_len = pane.terminal.screen().scrollback().len();
                    let visible_rows = pane.terminal.screen().rows();

                    if scrollback_len > 0 && window_height > 0.0 {
                        let delta_y = position.y - self.scrollbar_drag_start_y;
                        let total_lines = scrollback_len + visible_rows;
                        let thumb_height =
                            ((visible_rows as f64 / total_lines as f64) * window_height).max(20.0);
                        let scroll_range = window_height - thumb_height;

                        if scroll_range > 0.0 {
                            let scroll_delta =
                                (-delta_y / scroll_range * scrollback_len as f64) as isize;
                            let new_offset = (self.scrollbar_drag_start_offset as isize
                                + scroll_delta)
                                .max(0)
                                .min(scrollback_len as isize)
                                as usize;

                            if new_offset != pane.scroll_offset {
                                pane.scroll_offset = new_offset;
                                self.needs_redraw = true;
                            }
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

        // Compute cell coords relative to the *active* pane's rect so that
        // mouse tracking events sent to the active pane get correct coordinates.
        let tab = &self.tabs[self.active_tab];
        let available = self.pane_available_rect();
        let rects = tab.pane_root.calculate_rects(available);

        // Find the active pane's rect for cell coordinate calculation
        let active_pane_rect = rects
            .iter()
            .find(|(id, _)| *id == tab.active_pane_id)
            .map(|(_, r)| *r);

        if let Some(pane_rect) = active_pane_rect {
            let adjusted_x = (position.x - pane_rect.x as f64).max(0.0);
            let adjusted_y = (position.y - pane_rect.y as f64).max(0.0);
            let col = (adjusted_x / cell_size.width as f64) as u16;
            let row = (adjusted_y / cell_size.height as f64) as u16;

            if col == self.mouse_cell.0 && row == self.mouse_cell.1 {
                return;
            }

            self.mouse_cell = (col, row);
        }

        let tab = &mut self.tabs[self.active_tab];
        let active_pane_id = tab.active_pane_id;
        let pane = match tab.pane_root.find_pane_mut(active_pane_id) {
            Some(p) => p,
            None => return,
        };
        let modes = pane.terminal.screen().modes().clone();

        // Handle text selection dragging when mouse tracking is NOT enabled
        if !modes.mouse_tracking_enabled() && self.mouse_buttons[0] {
            let sel_col = self.mouse_cell.0 as usize;
            let sel_row = self.mouse_cell.1 as isize - pane.scroll_offset as isize;
            pane.terminal
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
            let event = MouseEvent::Move(self.mouse_cell.0, self.mouse_cell.1);
            if let Some(data) = encode_mouse(
                event,
                modes.mouse_sgr,
                modes.mouse_button_event,
                modes.mouse_any_event,
            ) {
                let _ = pane.child.write_all(&data);
            }
        }
    }

    /// Handle mouse scroll
    fn handle_mouse_scroll(&mut self, delta: MouseScrollDelta) {
        if self.tabs.is_empty() {
            return;
        }

        // Route scroll to whichever pane the mouse is over
        let scroll_pane_id = self
            .pane_at_pixel(self.mouse_pixel.0, self.mouse_pixel.1)
            .unwrap_or_else(|| self.tabs[self.active_tab].active_pane_id);

        // Compute cell coordinates relative to the scroll target pane's rect
        // *before* taking a mutable borrow on the pane tree.
        let scroll_cell = if let Some(renderer) = &self.renderer {
            let cell_size = renderer.cell_size();
            let available = self.pane_available_rect();
            let tab = &self.tabs[self.active_tab];
            let rects = tab.pane_root.calculate_rects(available);
            if let Some((_, pane_rect)) = rects.iter().find(|(id, _)| *id == scroll_pane_id) {
                let col = ((self.mouse_pixel.0 - pane_rect.x as f64).max(0.0)
                    / cell_size.width as f64) as u16;
                let row = ((self.mouse_pixel.1 - pane_rect.y as f64).max(0.0)
                    / cell_size.height as f64) as u16;
                (col, row)
            } else {
                self.mouse_cell
            }
        } else {
            self.mouse_cell
        };

        let tab = &mut self.tabs[self.active_tab];
        let pane = match tab.pane_root.find_pane_mut(scroll_pane_id) {
            Some(p) => p,
            None => return,
        };
        let modes = pane.terminal.screen().modes().clone();
        let lines = match delta {
            MouseScrollDelta::LineDelta(_, y) => y as i32,
            MouseScrollDelta::PixelDelta(pos) => (pos.y / 20.0) as i32,
        };

        if lines == 0 {
            return;
        }

        if modes.mouse_tracking_enabled() || modes.alternate_screen {
            let event = MouseEvent::Scroll {
                x: scroll_cell.0,
                y: scroll_cell.1,
                delta: lines as i8,
            };
            if let Some(data) = encode_mouse(
                event,
                modes.mouse_sgr,
                modes.mouse_button_event,
                modes.mouse_any_event,
            ) {
                let _ = pane.child.write_all(&data);
            }
        } else {
            let scrollback_len = pane.terminal.screen().scrollback().len();
            if lines > 0 {
                pane.scroll_offset = (pane.scroll_offset + lines as usize).min(scrollback_len);
            } else {
                pane.scroll_offset = pane.scroll_offset.saturating_sub((-lines) as usize);
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
        let pane = match tab.pane_root.find_pane(tab.active_pane_id) {
            Some(p) => p,
            None => return,
        };

        let screen = pane.terminal.screen();
        let selection = screen.selection();

        if selection.is_empty() {
            return;
        }

        let (start, end) = selection.bounds();
        let mut text = String::new();
        let cols = screen.cols();

        for row in start.row..=end.row {
            let start_col = if row == start.row { start.col } else { 0 };
            let end_col = if row == end.row { end.col } else { cols };

            if row < 0 {
                let scrollback_idx = (-row - 1) as usize;
                if let Some(line) = screen.scrollback().get_from_end(scrollback_idx) {
                    let line_text = line.text();
                    let chars: Vec<char> = line_text.chars().collect();
                    for ch in chars.iter().take(end_col.min(chars.len())).skip(start_col) {
                        text.push(*ch);
                    }
                }
            } else if (row as usize) < screen.grid().rows() {
                let line = screen.line(row as usize);
                let line_text = line.text();
                let chars: Vec<char> = line_text.chars().collect();
                for ch in chars.iter().take(end_col.min(chars.len())).skip(start_col) {
                    text.push(*ch);
                }
            }

            if row < end.row {
                while text.ends_with(' ') {
                    text.pop();
                }
                text.push('\n');
            }
        }

        let text = text.trim_end().to_string();

        if text.is_empty() {
            return;
        }

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
        let active_pane_id = tab.active_pane_id;
        let pane = match tab.pane_root.find_pane_mut(active_pane_id) {
            Some(p) => p,
            None => return,
        };

        match clipboard.get_text() {
            Ok(text) => {
                if text.is_empty() {
                    return;
                }
                let data = if pane.terminal.screen().modes().bracketed_paste {
                    encode_bracketed_paste(&text)
                } else {
                    text.into_bytes()
                };
                if let Err(e) = pane.child.write_all(&data) {
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

    /// Handle find (Ctrl+Shift+F)
    fn handle_find(&mut self) {
        log::info!("Find requested (Ctrl+Shift+F) - search UI not yet implemented");
    }

    /// Handle new window (Cmd+N on macOS)
    #[cfg(target_os = "macos")]
    fn handle_new_window(&mut self) {
        log::info!("Opening new terminal window...");

        if let Ok(exe_path) = std::env::current_exe() {
            match std::process::Command::new(&exe_path).spawn() {
                Ok(child) => {
                    log::info!("New terminal window spawned successfully");
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
                self.config.theme = new_config.theme;
                self.config.font = new_config.font.clone();
                self.config.keybindings = new_config.keybindings.clone();
                self.config.security = new_config.security.clone();

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
        let active_pane_id = tab.active_pane_id;
        let pane = match tab.pane_root.find_pane_mut(active_pane_id) {
            Some(p) => p,
            None => return,
        };

        if pane.terminal.screen().modes().focus_events {
            let data = encode_focus(focused);
            let _ = pane.child.write_all(&data);
        }
    }

    /// Poll PTY for output from all panes in all tabs
    fn poll_pty(&mut self) {
        let mut buf = [0u8; 65536];
        let mut any_output = false;

        for (tab_idx, tab) in self.tabs.iter_mut().enumerate() {
            let is_active_tab = tab_idx == self.active_tab;

            tab.pane_root.for_each_pane_mut(&mut |pane| {
                let mut received_output = false;

                loop {
                    match pane.child.pty_mut().try_read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            pane.terminal.process(&buf[..n]);
                            received_output = true;
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                        Err(_) => break,
                    }
                }

                if received_output && is_active_tab && !pane.terminal.is_synchronized_output() {
                    any_output = true;
                }

                // Reset scroll offset when new output arrives (auto-scroll to bottom)
                if received_output && pane.scroll_offset > 0 {
                    pane.scroll_offset = 0;
                }

                // Check for title change
                if pane.terminal.take_title_changed() {
                    pane.title = pane.terminal.title().to_string();
                }

                // Check for bell
                if pane.terminal.take_bell() {
                    log::debug!("Bell!");
                }

                // Send any pending responses back to the PTY (DSR, DA1, etc.)
                let responses = pane.terminal.take_pending_responses();
                for response in responses {
                    if let Err(e) = pane.child.write_all(&response) {
                        log::warn!("Failed to send response to PTY: {}", e);
                    }
                }
            });

            // Update tab title from the active pane
            if let Some(pane) = tab.pane_root.find_pane(tab.active_pane_id) {
                tab.title = pane.title.clone();
            }

            // Set window title from active tab's active pane
            if is_active_tab {
                if let Some(pane) = tab.pane_root.find_pane(tab.active_pane_id) {
                    if let Some(window) = &self.window {
                        window.set_title(&pane.title);
                    }
                }
            }
        }

        if any_output {
            self.needs_redraw = true;
        }
    }

    /// Render the terminal
    fn render(&mut self) {
        if self.renderer.is_none() || self.tabs.is_empty() {
            return;
        }

        // Compute these before borrowing renderer mutably
        let available = self.pane_available_rect();
        let tab_bar_height = self.tab_bar_height;
        let active_tab = self.active_tab;

        let tab_infos: Vec<TabInfo<'_>> = self
            .tabs
            .iter()
            .map(|t| TabInfo { title: &t.title })
            .collect();

        let tab = &self.tabs[active_tab];
        let rects = tab.pane_root.calculate_rects(available);
        let dividers = tab.pane_root.calculate_dividers(available);

        // Build PaneRenderInfo for each pane
        let mut pane_render_infos: Vec<PaneRenderInfo<'_>> = Vec::new();
        for (pane_id, rect) in &rects {
            if let Some(pane) = tab.pane_root.find_pane(*pane_id) {
                let screen = pane.terminal.screen();
                let selection = screen.selection();
                pane_render_infos.push(PaneRenderInfo {
                    screen,
                    selection,
                    scroll_offset: pane.scroll_offset,
                    rect: *rect,
                    is_active: *pane_id == tab.active_pane_id,
                });
            }
        }

        let renderer = self.renderer.as_mut().unwrap();
        if let Err(e) = renderer.render_panes(
            &pane_render_infos,
            &dividers,
            tab_bar_height,
            &tab_infos,
            active_tab,
        ) {
            log::warn!("Render error: {:?}", e);
        }

        self.needs_redraw = false;
        self.last_render = Instant::now();
    }

    /// Check if active tab's panes are still running
    fn check_child(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }

        // Remove exited panes from all tabs
        let mut panes_changed = false;
        for tab in &mut self.tabs {
            let exited = tab.pane_root.exited_pane_ids();
            for exited_id in exited {
                if tab.pane_root.pane_count() > 1 {
                    let remaining: Vec<PaneId> = tab
                        .pane_root
                        .all_pane_ids()
                        .into_iter()
                        .filter(|id| *id != exited_id)
                        .collect();

                    tab.pane_root.remove_pane(exited_id);
                    panes_changed = true;

                    if tab.active_pane_id == exited_id {
                        if let Some(&new_active) = remaining.first() {
                            tab.active_pane_id = new_active;
                        }
                    }
                }
            }
        }

        // Remove tabs that have no running panes
        let tab_count_before = self.tabs.len();
        self.tabs.retain(|tab| {
            let ids = tab.pane_root.all_pane_ids();
            !ids.is_empty()
                && ids.iter().any(|id| {
                    tab.pane_root
                        .find_pane(*id)
                        .is_some_and(|p| p.child.is_running())
                })
        });
        if self.tabs.len() != tab_count_before {
            panes_changed = true;
        }

        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len().saturating_sub(1);
        }

        // Resize remaining panes and trigger redraw after layout changes
        if panes_changed {
            self.resize_all_panes();
            self.needs_redraw = true;
        }

        !self.tabs.is_empty()
    }
}
