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
/// Width of the divider between split panes in pixels
const PANE_DIVIDER_WIDTH: u32 = 3;

/// Compute tab bar height from the current cell size so it scales with HiDPI / font size.
fn compute_tab_bar_height(cell_size: &crate::renderer::CellSize) -> u32 {
    cell_size.height as u32 + TAB_BAR_PADDING
}

/// Direction of a split
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SplitDirection {
    /// Side by side (left | right)
    Vertical,
    /// Top and bottom (top / bottom)
    Horizontal,
}

/// Unique identifier for a pane
type PaneId = u64;

/// A leaf pane containing a terminal and PTY child
struct PaneLeaf {
    id: PaneId,
    terminal: Terminal,
    child: Child,
    title: String,
    scroll_offset: usize,
}

impl PaneLeaf {
    fn new(id: PaneId, terminal: Terminal, child: Child) -> Self {
        Self {
            id,
            terminal,
            child,
            title: String::from("Terminal"),
            scroll_offset: 0,
        }
    }
}

/// A node in the pane tree - either a leaf (terminal) or a branch (split)
enum PaneNode {
    Leaf(Box<PaneLeaf>),
    Split {
        direction: SplitDirection,
        /// Fraction of space given to the first child (0.0 - 1.0)
        ratio: f32,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

impl PaneNode {
    /// Get all leaf panes mutably
    fn leaves_mut(&mut self) -> Vec<&mut PaneLeaf> {
        match self {
            PaneNode::Leaf(ref mut leaf) => vec![leaf],

            PaneNode::Split { first, second, .. } => {
                let mut leaves = first.leaves_mut();
                leaves.extend(second.leaves_mut());
                leaves
            }
        }
    }

    /// Find a leaf by ID
    fn find_leaf(&self, id: PaneId) -> Option<&PaneLeaf> {
        match self {
            PaneNode::Leaf(ref leaf) => {
                if leaf.id == id {
                    Some(leaf)
                } else {
                    None
                }
            }
            PaneNode::Split { first, second, .. } => {
                first.find_leaf(id).or_else(|| second.find_leaf(id))
            }
        }
    }

    /// Find a leaf by ID mutably
    fn find_leaf_mut(&mut self, id: PaneId) -> Option<&mut PaneLeaf> {
        match self {
            PaneNode::Leaf(ref mut leaf) => {
                if leaf.id == id {
                    Some(leaf)
                } else {
                    None
                }
            }
            PaneNode::Split { first, second, .. } => {
                if let Some(leaf) = first.find_leaf_mut(id) {
                    Some(leaf)
                } else {
                    second.find_leaf_mut(id)
                }
            }
        }
    }

    /// Get the first leaf's ID (used as fallback)
    fn first_leaf_id(&self) -> PaneId {
        match self {
            PaneNode::Leaf(ref leaf) => leaf.id,
            PaneNode::Split { first, .. } => first.first_leaf_id(),
        }
    }

    /// Count total leaf panes
    fn leaf_count(&self) -> usize {
        match self {
            PaneNode::Leaf(..) => 1,
            PaneNode::Split { first, second, .. } => first.leaf_count() + second.leaf_count(),
        }
    }

    /// Check if any child is still running
    fn any_running(&self) -> bool {
        match self {
            PaneNode::Leaf(ref leaf) => leaf.child.is_running(),
            PaneNode::Split { first, second, .. } => first.any_running() || second.any_running(),
        }
    }

    /// Split the pane with the given ID
    fn split_pane(
        self,
        target_id: PaneId,
        direction: SplitDirection,
        new_leaf: Box<PaneLeaf>,
    ) -> PaneNode {
        match self {
            PaneNode::Leaf(leaf) if leaf.id == target_id => PaneNode::Split {
                direction,
                ratio: 0.5,
                first: Box::new(PaneNode::Leaf(leaf)),
                second: Box::new(PaneNode::Leaf(new_leaf)),
            },
            PaneNode::Leaf(leaf) => PaneNode::Leaf(leaf),
            PaneNode::Split {
                direction: d,
                ratio,
                first,
                second,
            } => {
                let first_has = first.find_leaf(target_id).is_some();
                if first_has {
                    PaneNode::Split {
                        direction: d,
                        ratio,
                        first: Box::new(first.split_pane(target_id, direction, new_leaf)),
                        second,
                    }
                } else {
                    PaneNode::Split {
                        direction: d,
                        ratio,
                        first,
                        second: Box::new(second.split_pane(target_id, direction, new_leaf)),
                    }
                }
            }
        }
    }

    /// Remove a pane by ID, returning the remaining tree (or None if this was the last pane)
    fn remove_pane(self, target_id: PaneId) -> Option<PaneNode> {
        match self {
            PaneNode::Leaf(ref leaf) if leaf.id == target_id => None,
            PaneNode::Leaf(_) => Some(self),
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let first_has = first.find_leaf(target_id).is_some();
                let second_has = second.find_leaf(target_id).is_some();

                if first_has {
                    match first.remove_pane(target_id) {
                        Some(remaining) => Some(PaneNode::Split {
                            direction,
                            ratio,
                            first: Box::new(remaining),
                            second,
                        }),
                        None => Some(*second),
                    }
                } else if second_has {
                    match second.remove_pane(target_id) {
                        Some(remaining) => Some(PaneNode::Split {
                            direction,
                            ratio,
                            first,
                            second: Box::new(remaining),
                        }),
                        None => Some(*first),
                    }
                } else {
                    Some(PaneNode::Split {
                        direction,
                        ratio,
                        first,
                        second,
                    })
                }
            }
        }
    }

    /// Collect pane render info with computed pixel regions
    #[allow(clippy::too_many_arguments)]
    fn collect_pane_rects(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        focused_id: PaneId,
        cell_width: f32,
        cell_height: f32,
        rects: &mut Vec<PaneRenderInfo>,
    ) {
        match self {
            PaneNode::Leaf(ref leaf) => {
                rects.push(PaneRenderInfo {
                    x,
                    y,
                    width,
                    height,
                    is_focused: leaf.id == focused_id,
                    pane_id: leaf.id,
                });
            }
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let divider = PANE_DIVIDER_WIDTH;
                match direction {
                    SplitDirection::Vertical => {
                        let first_width = ((width as f32 - divider as f32) * ratio).round() as u32;
                        let snapped_first = snap_to_cells(first_width, cell_width);
                        let snapped_second = snap_to_cells(
                            width.saturating_sub(snapped_first + divider),
                            cell_width,
                        );
                        first.collect_pane_rects(
                            x,
                            y,
                            snapped_first,
                            height,
                            focused_id,
                            cell_width,
                            cell_height,
                            rects,
                        );
                        second.collect_pane_rects(
                            x + snapped_first + divider,
                            y,
                            snapped_second,
                            height,
                            focused_id,
                            cell_width,
                            cell_height,
                            rects,
                        );
                    }
                    SplitDirection::Horizontal => {
                        let first_height =
                            ((height as f32 - divider as f32) * ratio).round() as u32;
                        let snapped_first = snap_to_cells(first_height, cell_height);
                        let snapped_second = snap_to_cells(
                            height.saturating_sub(snapped_first + divider),
                            cell_height,
                        );
                        first.collect_pane_rects(
                            x,
                            y,
                            width,
                            snapped_first,
                            focused_id,
                            cell_width,
                            cell_height,
                            rects,
                        );
                        second.collect_pane_rects(
                            x,
                            y + snapped_first + divider,
                            width,
                            snapped_second,
                            focused_id,
                            cell_width,
                            cell_height,
                            rects,
                        );
                    }
                }
            }
        }
    }

    /// Collect divider rects for rendering
    #[allow(clippy::too_many_arguments)]
    fn collect_dividers(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        cell_width: f32,
        cell_height: f32,
        dividers: &mut Vec<(u32, u32, u32, u32)>,
    ) {
        if let PaneNode::Split {
            direction,
            ratio,
            first,
            second,
        } = self
        {
            let divider = PANE_DIVIDER_WIDTH;
            match direction {
                SplitDirection::Vertical => {
                    let first_width = ((width as f32 - divider as f32) * ratio).round() as u32;
                    let snapped_first = snap_to_cells(first_width, cell_width);
                    let snapped_second =
                        snap_to_cells(width.saturating_sub(snapped_first + divider), cell_width);
                    dividers.push((x + snapped_first, y, divider, height));
                    first.collect_dividers(
                        x,
                        y,
                        snapped_first,
                        height,
                        cell_width,
                        cell_height,
                        dividers,
                    );
                    second.collect_dividers(
                        x + snapped_first + divider,
                        y,
                        snapped_second,
                        height,
                        cell_width,
                        cell_height,
                        dividers,
                    );
                }
                SplitDirection::Horizontal => {
                    let first_height = ((height as f32 - divider as f32) * ratio).round() as u32;
                    let snapped_first = snap_to_cells(first_height, cell_height);
                    let snapped_second =
                        snap_to_cells(height.saturating_sub(snapped_first + divider), cell_height);
                    dividers.push((x, y + snapped_first, width, divider));
                    first.collect_dividers(
                        x,
                        y,
                        width,
                        snapped_first,
                        cell_width,
                        cell_height,
                        dividers,
                    );
                    second.collect_dividers(
                        x,
                        y + snapped_first + divider,
                        width,
                        snapped_second,
                        cell_width,
                        cell_height,
                        dividers,
                    );
                }
            }
        }
    }

    /// Find which pane contains the given pixel coordinate
    #[allow(clippy::too_many_arguments)]
    fn pane_at_pixel(
        &self,
        px: f64,
        py: f64,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        cell_width: f32,
        cell_height: f32,
    ) -> Option<PaneId> {
        match self {
            PaneNode::Leaf(ref leaf) => {
                if px >= x as f64
                    && px < (x + width) as f64
                    && py >= y as f64
                    && py < (y + height) as f64
                {
                    Some(leaf.id)
                } else {
                    None
                }
            }
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let divider = PANE_DIVIDER_WIDTH;
                match direction {
                    SplitDirection::Vertical => {
                        let first_width = ((width as f32 - divider as f32) * ratio).round() as u32;
                        let snapped_first = snap_to_cells(first_width, cell_width);
                        let snapped_second = snap_to_cells(
                            width.saturating_sub(snapped_first + divider),
                            cell_width,
                        );
                        if let Some(id) = first.pane_at_pixel(
                            px,
                            py,
                            x,
                            y,
                            snapped_first,
                            height,
                            cell_width,
                            cell_height,
                        ) {
                            return Some(id);
                        }
                        second.pane_at_pixel(
                            px,
                            py,
                            x + snapped_first + divider,
                            y,
                            snapped_second,
                            height,
                            cell_width,
                            cell_height,
                        )
                    }
                    SplitDirection::Horizontal => {
                        let first_height =
                            ((height as f32 - divider as f32) * ratio).round() as u32;
                        let snapped_first = snap_to_cells(first_height, cell_height);
                        let snapped_second = snap_to_cells(
                            height.saturating_sub(snapped_first + divider),
                            cell_height,
                        );
                        if let Some(id) = first.pane_at_pixel(
                            px,
                            py,
                            x,
                            y,
                            width,
                            snapped_first,
                            cell_width,
                            cell_height,
                        ) {
                            return Some(id);
                        }
                        second.pane_at_pixel(
                            px,
                            py,
                            x,
                            y + snapped_first + divider,
                            width,
                            snapped_second,
                            cell_width,
                            cell_height,
                        )
                    }
                }
            }
        }
    }

    /// Get the neighbor pane in a given direction from the focused pane
    #[allow(clippy::too_many_arguments)]
    fn neighbor_in_direction(
        &self,
        target_id: PaneId,
        dir: NavDirection,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        cell_width: f32,
        cell_height: f32,
    ) -> Option<PaneId> {
        let mut rects = Vec::new();
        self.collect_pane_rects(
            x,
            y,
            width,
            height,
            target_id,
            cell_width,
            cell_height,
            &mut rects,
        );

        let focused_rect = rects.iter().find(|r| r.pane_id == target_id)?;
        let fx = focused_rect.x as f64 + focused_rect.width as f64 / 2.0;
        let fy = focused_rect.y as f64 + focused_rect.height as f64 / 2.0;

        let mut best: Option<(PaneId, f64)> = None;

        for r in &rects {
            if r.pane_id == target_id {
                continue;
            }
            let rx = r.x as f64 + r.width as f64 / 2.0;
            let ry = r.y as f64 + r.height as f64 / 2.0;

            let is_valid = match dir {
                NavDirection::Left => rx < fx,
                NavDirection::Right => rx > fx,
                NavDirection::Up => ry < fy,
                NavDirection::Down => ry > fy,
            };

            if is_valid {
                let dist = (rx - fx).powi(2) + (ry - fy).powi(2);
                if best.is_none() || dist < best.unwrap().1 {
                    best = Some((r.pane_id, dist));
                }
            }
        }

        best.map(|(id, _)| id)
    }

    /// Resize all terminal leaves to fit their computed pixel regions
    #[allow(clippy::only_used_in_recursion)]
    fn resize_leaves(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        cell_width: f32,
        cell_height: f32,
    ) {
        match self {
            PaneNode::Leaf(ref mut leaf) => {
                let cols = (width as f32 / cell_width) as usize;
                let rows = (height as f32 / cell_height) as usize;
                if cols > 0 && rows > 0 {
                    leaf.terminal.resize(cols, rows);
                    let _ = leaf.child.resize(WindowSize::new(cols as u16, rows as u16));
                }
            }
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let divider = PANE_DIVIDER_WIDTH;
                match direction {
                    SplitDirection::Vertical => {
                        let first_width = ((width as f32 - divider as f32) * *ratio).round() as u32;
                        let snapped_first = snap_to_cells(first_width, cell_width);
                        let snapped_second = snap_to_cells(
                            width.saturating_sub(snapped_first + divider),
                            cell_width,
                        );
                        first.resize_leaves(x, y, snapped_first, height, cell_width, cell_height);
                        second.resize_leaves(
                            x + snapped_first + divider,
                            y,
                            snapped_second,
                            height,
                            cell_width,
                            cell_height,
                        );
                    }
                    SplitDirection::Horizontal => {
                        let first_height =
                            ((height as f32 - divider as f32) * *ratio).round() as u32;
                        let snapped_first = snap_to_cells(first_height, cell_height);
                        let snapped_second = snap_to_cells(
                            height.saturating_sub(snapped_first + divider),
                            cell_height,
                        );
                        first.resize_leaves(x, y, width, snapped_first, cell_width, cell_height);
                        second.resize_leaves(
                            x,
                            y + snapped_first + divider,
                            width,
                            snapped_second,
                            cell_width,
                            cell_height,
                        );
                    }
                }
            }
        }
    }
}

/// Snap a pixel dimension to the nearest cell boundary
fn snap_to_cells(pixels: u32, cell_size: f32) -> u32 {
    let cells = (pixels as f32 / cell_size).floor() as u32;
    (cells as f32 * cell_size) as u32
}

/// Navigation direction for pane focus switching
#[derive(Debug, Clone, Copy)]
enum NavDirection {
    Left,
    Right,
    Up,
    Down,
}

/// A single terminal tab (now containing a pane tree)
struct Tab {
    panes: PaneNode,
    /// The focused pane ID within this tab
    focused_pane: PaneId,
    /// Title (derived from focused pane)
    title: String,
}

impl Tab {
    fn new(pane: Box<PaneLeaf>) -> Self {
        let id = pane.id;
        Self {
            panes: PaneNode::Leaf(pane),
            focused_pane: id,
            title: String::from("Terminal"),
        }
    }

    /// Get the focused pane leaf
    fn focused_leaf(&self) -> Option<&PaneLeaf> {
        self.panes.find_leaf(self.focused_pane)
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
    /// Tabs (each tab has a pane tree)
    tabs: Vec<Tab>,
    /// Active tab index
    active_tab: usize,
    /// Clipboard
    #[allow(dead_code)]
    clipboard: Option<Clipboard>,
    /// Current modifiers state
    modifiers: ModifiersState,
    /// Mouse position (in cells, relative to focused pane)
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
    /// Whether we are currently dragging the scrollbar
    scrollbar_dragging: bool,
    /// Y position where scrollbar drag started (in pixels)
    scrollbar_drag_start_y: f64,
    /// Scroll offset when scrollbar drag started
    scrollbar_drag_start_offset: usize,
    /// Next pane ID counter
    next_pane_id: PaneId,
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
                        log::info!("All child processes exited");
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

        // Create first tab with a single pane
        let pane_id = self.alloc_pane_id();
        let terminal = Terminal::new(cols.max(1), rows.max(1));
        let child = Child::spawn_shell(WindowSize::new(cols as u16, rows as u16))?;
        child.set_nonblocking(true)?;

        let pane = Box::new(PaneLeaf::new(pane_id, terminal, child));
        let tab = Tab::new(pane);
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

        let pane_id = self.alloc_pane_id();
        let terminal = Terminal::new(cols.max(1), rows.max(1));
        match Child::spawn_shell(WindowSize::new(cols as u16, rows as u16)) {
            Ok(child) => {
                let _ = child.set_nonblocking(true);
                let pane = Box::new(PaneLeaf::new(pane_id, terminal, child));
                let tab = Tab::new(pane);
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

    /// Split the focused pane in the active tab
    fn split_focused_pane(&mut self, direction: SplitDirection) {
        if self.renderer.is_none() || self.window.is_none() || self.tabs.is_empty() {
            return;
        }

        // Extract everything we need from renderer/window before mutating self
        let cell_size = self.renderer.as_ref().unwrap().cell_size();
        let size = self.window.as_ref().unwrap().inner_size();
        let terminal_height = size.height.saturating_sub(self.tab_bar_height);

        let tab = &self.tabs[self.active_tab];
        let focused_id = tab.focused_pane;

        // Collect pane rects to find the focused pane dimensions
        let mut rects = Vec::new();
        tab.panes.collect_pane_rects(
            0,
            self.tab_bar_height,
            size.width,
            terminal_height,
            focused_id,
            cell_size.width,
            cell_size.height,
            &mut rects,
        );

        let focused_rect = rects.iter().find(|r| r.pane_id == focused_id);
        let (pane_width, pane_height) = if let Some(r) = focused_rect {
            (r.width, r.height)
        } else {
            (size.width, terminal_height)
        };

        // Calculate new pane dimensions
        let (new_cols, new_rows) = match direction {
            SplitDirection::Vertical => {
                let half_w = pane_width.saturating_sub(PANE_DIVIDER_WIDTH) / 2;
                let cols = (half_w as f32 / cell_size.width) as usize;
                let rows = (pane_height as f32 / cell_size.height) as usize;
                (cols.max(1), rows.max(1))
            }
            SplitDirection::Horizontal => {
                let cols = (pane_width as f32 / cell_size.width) as usize;
                let half_h = pane_height.saturating_sub(PANE_DIVIDER_WIDTH) / 2;
                let rows = (half_h as f32 / cell_size.height) as usize;
                (cols.max(1), rows.max(1))
            }
        };

        let pane_id = self.alloc_pane_id();
        let terminal = Terminal::new(new_cols, new_rows);
        match Child::spawn_shell(WindowSize::new(new_cols as u16, new_rows as u16)) {
            Ok(child) => {
                let _ = child.set_nonblocking(true);
                let new_pane = Box::new(PaneLeaf::new(pane_id, terminal, child));

                // Take ownership of the tab, split, and put it back
                let tab = self.tabs.remove(self.active_tab);
                let new_panes = tab.panes.split_pane(focused_id, direction, new_pane);

                let mut new_tab = Tab {
                    panes: new_panes,
                    focused_pane: pane_id,
                    title: tab.title,
                };

                // Resize all panes to fit properly
                new_tab.panes.resize_leaves(
                    0,
                    self.tab_bar_height,
                    size.width,
                    terminal_height,
                    cell_size.width,
                    cell_size.height,
                );

                self.tabs.insert(self.active_tab, new_tab);
                self.needs_redraw = true;
                log::info!("Split pane {:?}, new pane id={}", direction, pane_id);
            }
            Err(e) => {
                log::error!("Failed to split pane: {}", e);
            }
        }
    }

    /// Close the focused pane in the active tab
    fn close_focused_pane(&mut self) {
        if self.tabs.is_empty() {
            return;
        }

        let tab = &self.tabs[self.active_tab];
        if tab.panes.leaf_count() <= 1 {
            return;
        }

        let focused_id = tab.focused_pane;
        let tab = self.tabs.remove(self.active_tab);
        if let Some(remaining) = tab.panes.remove_pane(focused_id) {
            let new_focused = remaining.first_leaf_id();
            let mut new_tab = Tab {
                panes: remaining,
                focused_pane: new_focused,
                title: tab.title,
            };

            // Resize all panes
            if let (Some(renderer), Some(window)) = (&self.renderer, &self.window) {
                let size = window.inner_size();
                let cell_size = renderer.cell_size();
                let terminal_height = size.height.saturating_sub(self.tab_bar_height);
                new_tab.panes.resize_leaves(
                    0,
                    self.tab_bar_height,
                    size.width,
                    terminal_height,
                    cell_size.width,
                    cell_size.height,
                );
            }

            self.tabs.insert(self.active_tab, new_tab);
        }
        self.needs_redraw = true;
        log::info!("Closed focused pane");
    }

    /// Navigate focus to a neighboring pane
    fn navigate_pane(&mut self, dir: NavDirection) {
        if self.tabs.is_empty() {
            return;
        }
        let Some(renderer) = &self.renderer else {
            return;
        };
        let Some(window) = &self.window else { return };

        let tab = &self.tabs[self.active_tab];
        if tab.panes.leaf_count() <= 1 {
            return;
        }

        let size = window.inner_size();
        let cell_size = renderer.cell_size();
        let terminal_height = size.height.saturating_sub(self.tab_bar_height);

        if let Some(new_id) = tab.panes.neighbor_in_direction(
            tab.focused_pane,
            dir,
            0,
            self.tab_bar_height,
            size.width,
            terminal_height,
            cell_size.width,
            cell_size.height,
        ) {
            self.tabs[self.active_tab].focused_pane = new_id;
            self.needs_redraw = true;
            log::info!("Navigated pane focus to {}", new_id);
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
        let terminal_height = size.height.saturating_sub(self.tab_bar_height);

        for tab in &mut self.tabs {
            tab.panes.resize_leaves(
                0,
                self.tab_bar_height,
                size.width,
                terminal_height,
                cell_size.width,
                cell_size.height,
            );
        }

        self.needs_redraw = true;
    }

    /// Get the pixel rect of the focused pane in the active tab
    fn focused_pane_rect(&self) -> Option<PaneRenderInfo> {
        let tab = self.tabs.get(self.active_tab)?;
        let renderer = self.renderer.as_ref()?;
        let window = self.window.as_ref()?;

        let size = window.inner_size();
        let cell_size = renderer.cell_size();
        let terminal_height = size.height.saturating_sub(self.tab_bar_height);

        let mut rects = Vec::new();
        tab.panes.collect_pane_rects(
            0,
            self.tab_bar_height,
            size.width,
            terminal_height,
            tab.focused_pane,
            cell_size.width,
            cell_size.height,
            &mut rects,
        );

        rects.into_iter().find(|r| r.pane_id == tab.focused_pane)
    }

    /// Compute the mouse cell position relative to a specific pane rect
    fn mouse_cell_in_pane(&self, pane_rect: &PaneRenderInfo) -> (u16, u16) {
        let Some(renderer) = &self.renderer else {
            return (0, 0);
        };
        let cell_size = renderer.cell_size();
        let local_x = (self.mouse_pixel.0 - pane_rect.x as f64).max(0.0);
        let local_y = (self.mouse_pixel.1 - pane_rect.y as f64).max(0.0);
        let col = (local_x / cell_size.width as f64) as u16;
        let row = (local_y / cell_size.height as f64) as u16;
        (col, row)
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
                // Split pane vertically: Ctrl+Shift+D (side by side)
                Key::Character(c) if c.to_lowercase() == "d" => {
                    self.split_focused_pane(SplitDirection::Vertical);
                    return;
                }
                // Split pane horizontally: Ctrl+Shift+E (top/bottom)
                Key::Character(c) if c.to_lowercase() == "e" => {
                    self.split_focused_pane(SplitDirection::Horizontal);
                    return;
                }
                // Close focused pane: Ctrl+Shift+X
                Key::Character(c) if c.to_lowercase() == "x" => {
                    self.close_focused_pane();
                    return;
                }
                // Navigate panes with Ctrl+Shift+Arrow keys
                Key::Named(NamedKey::ArrowLeft) => {
                    self.navigate_pane(NavDirection::Left);
                    return;
                }
                Key::Named(NamedKey::ArrowRight) => {
                    self.navigate_pane(NavDirection::Right);
                    return;
                }
                Key::Named(NamedKey::ArrowUp) => {
                    self.navigate_pane(NavDirection::Up);
                    return;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    self.navigate_pane(NavDirection::Down);
                    return;
                }
                _ => {}
            }
        }

        // macOS shortcuts
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
                    if self
                        .tabs
                        .get(self.active_tab)
                        .is_some_and(|t| t.panes.leaf_count() > 1)
                    {
                        self.close_focused_pane();
                    } else if !self.close_current_tab() {
                        self.tabs.clear();
                    }
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "d" => {
                    if self.modifiers.shift_key() {
                        self.split_focused_pane(SplitDirection::Horizontal);
                    } else {
                        self.split_focused_pane(SplitDirection::Vertical);
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

        // Linux: Ctrl+Shift+T for new tab, Ctrl+Shift+W to close tab/pane
        #[cfg(not(target_os = "macos"))]
        if ctrl_shift {
            match &event.logical_key {
                Key::Character(c) if c.to_lowercase() == "t" && !self.modifiers.super_key() => {
                    self.create_new_tab();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "w" => {
                    if self
                        .tabs
                        .get(self.active_tab)
                        .is_some_and(|t| t.panes.leaf_count() > 1)
                    {
                        self.close_focused_pane();
                    } else if !self.close_current_tab() {
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
        let focused_id = tab.focused_pane;
        let leaf = match tab.panes.find_leaf_mut(focused_id) {
            Some(leaf) => leaf,
            None => return,
        };

        // Handle control characters
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
                    let _ = leaf.child.write_all(&[first_char as u8]);
                    return;
                }
            }
        }

        // Fallback control character check
        if let Key::Character(c) = &event.logical_key {
            if let Some(ch) = c.chars().next() {
                let char_code = ch as u32;
                if (1..=26).contains(&char_code) || char_code == 0x7F {
                    log::debug!(
                        "Sending control character from logical_key: {:?} (0x{:02x})",
                        ch,
                        ch as u8
                    );
                    let _ = leaf.child.write_all(&[ch as u8]);
                    return;
                }
            }
        }

        // Font zoom shortcuts
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
                    #[cfg(target_os = "macos")]
                    {
                        self.change_font_size(2.0);
                        return;
                    }
                }
                Key::Named(NamedKey::ArrowDown) => {
                    #[cfg(target_os = "macos")]
                    {
                        self.change_font_size(-2.0);
                        return;
                    }
                }
                _ => {}
            }
        }

        let application_cursor_keys = leaf.terminal.screen().modes().cursor_keys_application;

        if let Some(data) = encode_key(&event.logical_key, self.modifiers, application_cursor_keys)
        {
            log::debug!("Sending key data: {:?}", data);
            let _ = leaf.child.write_all(&data);
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

        let size = window.inner_size();
        let cell_size = renderer.cell_size();
        self.tab_bar_height = compute_tab_bar_height(&cell_size);
        let terminal_height = size.height.saturating_sub(self.tab_bar_height);

        for tab in &mut self.tabs {
            tab.panes.resize_leaves(
                0,
                self.tab_bar_height,
                size.width,
                terminal_height,
                cell_size.width,
                cell_size.height,
            );
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

        let size = window.inner_size();
        let cell_size = renderer.cell_size();
        self.tab_bar_height = compute_tab_bar_height(&cell_size);
        let terminal_height = size.height.saturating_sub(self.tab_bar_height);

        for tab in &mut self.tabs {
            tab.panes.resize_leaves(
                0,
                self.tab_bar_height,
                size.width,
                terminal_height,
                cell_size.width,
                cell_size.height,
            );
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

        // Handle click-to-focus pane
        if button == MouseButton::Left && state == ElementState::Pressed {
            if let (Some(renderer), Some(window)) = (&self.renderer, &self.window) {
                let size = window.inner_size();
                let cell_size = renderer.cell_size();
                let terminal_height = size.height.saturating_sub(self.tab_bar_height);
                let tab = &self.tabs[self.active_tab];

                if let Some(clicked_pane_id) = tab.panes.pane_at_pixel(
                    self.mouse_pixel.0,
                    self.mouse_pixel.1,
                    0,
                    self.tab_bar_height,
                    size.width,
                    terminal_height,
                    cell_size.width,
                    cell_size.height,
                ) {
                    if clicked_pane_id != tab.focused_pane {
                        self.tabs[self.active_tab].focused_pane = clicked_pane_id;
                        self.needs_redraw = true;
                    }
                }
            }
        }

        // Handle scrollbar dragging (left button only)
        if button == MouseButton::Left {
            if state == ElementState::Pressed {
                if let Some(pane_rect) = self.focused_pane_rect() {
                    let pane_right = (pane_rect.x + pane_rect.width) as f64;
                    let scrollbar_width = 12.0;

                    if self.mouse_pixel.0 >= pane_right - scrollbar_width
                        && self.mouse_pixel.0 < pane_right
                        && self.mouse_pixel.1 >= pane_rect.y as f64
                    {
                        let tab = &self.tabs[self.active_tab];
                        if let Some(leaf) = tab.focused_leaf() {
                            let scrollback_len = leaf.terminal.screen().scrollback().len();
                            if scrollback_len > 0 {
                                self.scrollbar_dragging = true;
                                self.scrollbar_drag_start_y = self.mouse_pixel.1;
                                self.scrollbar_drag_start_offset = leaf.scroll_offset;
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

        // Get mouse cell relative to pane (compute before mutable borrow)
        let pane_rect = self.focused_pane_rect();
        let (col, row) = if let Some(ref pr) = pane_rect {
            self.mouse_cell_in_pane(pr)
        } else {
            self.mouse_cell
        };

        let tab = &mut self.tabs[self.active_tab];
        let focused_id = tab.focused_pane;
        let leaf = match tab.panes.find_leaf_mut(focused_id) {
            Some(leaf) => leaf,
            None => return,
        };
        let modes = leaf.terminal.screen().modes().clone();

        // Handle text selection when mouse tracking is NOT enabled
        if !modes.mouse_tracking_enabled() {
            if button == MouseButton::Left {
                let sel_row = row as isize - leaf.scroll_offset as isize;

                if state == ElementState::Pressed {
                    leaf.terminal
                        .screen_mut()
                        .selection_mut()
                        .start(Point::new(col as usize, sel_row), SelectionType::Normal);
                    self.needs_redraw = true;
                } else {
                    leaf.terminal.screen_mut().selection_mut().finish();
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
            MouseEvent::Press(button, col, row)
        } else {
            MouseEvent::Release(button, col, row)
        };

        if let Some(data) = encode_mouse(
            event,
            modes.mouse_sgr,
            modes.mouse_button_event,
            modes.mouse_any_event,
        ) {
            let _ = leaf.child.write_all(&data);
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

        // Handle scrollbar dragging
        if self.scrollbar_dragging {
            if let Some(pane_rect) = self.focused_pane_rect() {
                let tab = &mut self.tabs[self.active_tab];
                let focused_id = tab.focused_pane;
                if let Some(leaf) = tab.panes.find_leaf_mut(focused_id) {
                    let window_height = pane_rect.height as f64;
                    let scrollback_len = leaf.terminal.screen().scrollback().len();
                    let visible_rows = leaf.terminal.screen().rows();

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

                            if new_offset != leaf.scroll_offset {
                                leaf.scroll_offset = new_offset;
                                self.needs_redraw = true;
                            }
                        }
                    }
                }
            }
            return;
        }

        // Calculate mouse cell relative to focused pane
        if let Some(pane_rect) = self.focused_pane_rect() {
            let (col, row) = self.mouse_cell_in_pane(&pane_rect);

            if col == self.mouse_cell.0 && row == self.mouse_cell.1 {
                return;
            }

            self.mouse_cell = (col, row);

            let tab = &mut self.tabs[self.active_tab];
            let focused_id = tab.focused_pane;
            let leaf = match tab.panes.find_leaf_mut(focused_id) {
                Some(leaf) => leaf,
                None => return,
            };
            let modes = leaf.terminal.screen().modes().clone();

            if !modes.mouse_tracking_enabled() && self.mouse_buttons[0] {
                let sel_col = col as usize;
                let sel_row = row as isize - leaf.scroll_offset as isize;
                leaf.terminal
                    .screen_mut()
                    .selection_mut()
                    .update(Point::new(sel_col, sel_row));
                self.needs_redraw = true;
                return;
            }

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
                    let _ = leaf.child.write_all(&data);
                }
            }
        }
    }

    /// Handle mouse scroll
    fn handle_mouse_scroll(&mut self, delta: MouseScrollDelta) {
        if self.tabs.is_empty() {
            return;
        }

        let tab = &mut self.tabs[self.active_tab];
        let focused_id = tab.focused_pane;
        let leaf = match tab.panes.find_leaf_mut(focused_id) {
            Some(leaf) => leaf,
            None => return,
        };
        let modes = leaf.terminal.screen().modes().clone();
        let lines = match delta {
            MouseScrollDelta::LineDelta(_, y) => y as i32,
            MouseScrollDelta::PixelDelta(pos) => (pos.y / 20.0) as i32,
        };

        if lines == 0 {
            return;
        }

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
                let _ = leaf.child.write_all(&data);
            }
        } else {
            let scrollback_len = leaf.terminal.screen().scrollback().len();
            if lines > 0 {
                leaf.scroll_offset = (leaf.scroll_offset + lines as usize).min(scrollback_len);
            } else {
                leaf.scroll_offset = leaf.scroll_offset.saturating_sub((-lines) as usize);
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
        let leaf = match tab.focused_leaf() {
            Some(leaf) => leaf,
            None => return,
        };

        let screen = leaf.terminal.screen();
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
        let focused_id = tab.focused_pane;
        let leaf = match tab.panes.find_leaf_mut(focused_id) {
            Some(leaf) => leaf,
            None => return,
        };

        match clipboard.get_text() {
            Ok(text) => {
                if text.is_empty() {
                    return;
                }
                let data = if leaf.terminal.screen().modes().bracketed_paste {
                    encode_bracketed_paste(&text)
                } else {
                    text.into_bytes()
                };
                if let Err(e) = leaf.child.write_all(&data) {
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
        let focused_id = tab.focused_pane;
        if let Some(leaf) = tab.panes.find_leaf_mut(focused_id) {
            if leaf.terminal.screen().modes().focus_events {
                let data = encode_focus(focused);
                let _ = leaf.child.write_all(&data);
            }
        }
    }

    /// Poll PTY for output from all tabs (all panes in each tab)
    fn poll_pty(&mut self) {
        let mut buf = [0u8; 65536];

        for (tab_idx, tab) in self.tabs.iter_mut().enumerate() {
            let is_active_tab = tab_idx == self.active_tab;
            let focused_id = tab.focused_pane;

            for leaf in tab.panes.leaves_mut() {
                let mut received_output = false;

                loop {
                    match leaf.child.pty_mut().try_read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            leaf.terminal.process(&buf[..n]);
                            received_output = true;
                            if is_active_tab && !leaf.terminal.is_synchronized_output() {
                                self.needs_redraw = true;
                            }
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                        Err(_) => break,
                    }
                }

                if received_output && leaf.scroll_offset > 0 {
                    leaf.scroll_offset = 0;
                }

                if leaf.terminal.take_title_changed() {
                    leaf.title = leaf.terminal.title().to_string();
                    if is_active_tab && leaf.id == focused_id {
                        if let Some(window) = &self.window {
                            window.set_title(&leaf.title);
                        }
                    }
                }

                if leaf.terminal.take_bell() {
                    log::debug!("Bell!");
                }

                let responses = leaf.terminal.take_pending_responses();
                for response in responses {
                    if let Err(e) = leaf.child.write_all(&response) {
                        log::warn!("Failed to send response to PTY: {}", e);
                    }
                }
            }

            // Update tab title from focused pane
            if let Some(leaf) = tab.panes.find_leaf(focused_id) {
                tab.title = leaf.title.clone();
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
        let focused_id = tab.focused_pane;

        let (window_width, window_height) = if let Some(window) = &self.window {
            let size = window.inner_size();
            (size.width, size.height)
        } else {
            return;
        };
        let terminal_height = window_height.saturating_sub(self.tab_bar_height);

        let cell_size = renderer.cell_size();
        let mut pane_rects = Vec::new();
        tab.panes.collect_pane_rects(
            0,
            self.tab_bar_height,
            window_width,
            terminal_height,
            focused_id,
            cell_size.width,
            cell_size.height,
            &mut pane_rects,
        );

        let mut dividers = Vec::new();
        tab.panes.collect_dividers(
            0,
            self.tab_bar_height,
            window_width,
            terminal_height,
            cell_size.width,
            cell_size.height,
            &mut dividers,
        );

        // Collect pane data for rendering
        let pane_data: Vec<_> = pane_rects
            .iter()
            .filter_map(|rect| {
                let leaf = tab.panes.find_leaf(rect.pane_id)?;
                let screen = leaf.terminal.screen();
                let selection = screen.selection();
                Some((rect.clone(), screen, selection, leaf.scroll_offset))
            })
            .collect();

        if let Err(e) = renderer.render_with_panes(
            &pane_data,
            &dividers,
            self.tab_bar_height,
            &tab_infos,
            self.active_tab,
        ) {
            log::warn!("Render error: {:?}", e);
        }

        self.needs_redraw = false;
        self.last_render = Instant::now();
    }

    /// Check if active tab children are still running, prune dead panes/tabs
    fn check_child(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }

        // Remove fully dead tabs
        self.tabs.retain(|tab| tab.panes.any_running());

        if self.tabs.is_empty() {
            return false;
        }

        // Adjust active tab index if needed
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }

        // For each tab, check if the focused pane is still valid
        for tab in &mut self.tabs {
            if tab.panes.find_leaf(tab.focused_pane).is_none() {
                tab.focused_pane = tab.panes.first_leaf_id();
                self.needs_redraw = true;
            }
        }

        true
    }
}
