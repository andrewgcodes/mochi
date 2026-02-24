//! Main application logic
//!
//! Ties together the terminal, PTY, and renderer.
//! Supports split panes (horizontal/vertical terminal multiplexing).

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
use crate::renderer::{Renderer, TabInfo};
use crate::terminal::Terminal;

/// Padding added to cell height to compute tab bar height
const TAB_BAR_PADDING: u32 = 8;
/// Maximum width of a single tab in pixels
const TAB_MAX_WIDTH: u32 = 200;
/// Width of the close button area in each tab
const CLOSE_BTN_WIDTH: u32 = 20;
/// Width of the new tab (+) button
const NEW_TAB_BTN_WIDTH: u32 = 32;
/// Width of the divider between panes in pixels
const PANE_DIVIDER_WIDTH: u32 = 4;

/// Compute tab bar height from the current cell size so it scales with HiDPI / font size.
fn compute_tab_bar_height(cell_size: &crate::renderer::CellSize) -> u32 {
    cell_size.height as u32 + TAB_BAR_PADDING
}

/// Unique pane identifier
type PaneId = u64;

/// Next pane ID counter
static mut NEXT_PANE_ID: PaneId = 1;

fn next_pane_id() -> PaneId {
    unsafe {
        let id = NEXT_PANE_ID;
        NEXT_PANE_ID += 1;
        id
    }
}

/// Split direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Vertical,
    Horizontal,
}

/// A rectangular region in pixels
#[derive(Debug, Clone, Copy)]
pub struct PaneRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// A leaf pane containing a terminal and child process
struct LeafPane {
    id: PaneId,
    terminal: Terminal,
    child: Child,
    title: String,
    scroll_offset: usize,
}

impl LeafPane {
    fn new(terminal: Terminal, child: Child) -> Self {
        Self {
            id: next_pane_id(),
            terminal,
            child,
            title: String::from("Terminal"),
            scroll_offset: 0,
        }
    }
}

/// A pane tree node
enum PaneNode {
    Leaf(Box<LeafPane>),
    Split {
        direction: SplitDirection,
        ratio: f64,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
    /// Temporary placeholder used during tree surgery to avoid spawning dummy processes.
    /// Must never be observed outside of a `std::mem::replace` sequence.
    Placeholder,
}

impl PaneNode {
    fn new_leaf(terminal: Terminal, child: Child) -> Self {
        PaneNode::Leaf(Box::new(LeafPane::new(terminal, child)))
    }

    fn first_leaf_id(&self) -> PaneId {
        match self {
            PaneNode::Leaf(leaf) => leaf.id,
            PaneNode::Split { first, .. } => first.first_leaf_id(),
            PaneNode::Placeholder => 0,
        }
    }

    fn find_leaf_mut(&mut self, id: PaneId) -> Option<&mut LeafPane> {
        match self {
            PaneNode::Leaf(leaf) => {
                if leaf.id == id {
                    Some(leaf)
                } else {
                    None
                }
            }
            PaneNode::Split { first, second, .. } => {
                first.find_leaf_mut(id).or_else(|| second.find_leaf_mut(id))
            }
            PaneNode::Placeholder => None,
        }
    }

    fn find_leaf(&self, id: PaneId) -> Option<&LeafPane> {
        match self {
            PaneNode::Leaf(leaf) => {
                if leaf.id == id {
                    Some(leaf)
                } else {
                    None
                }
            }
            PaneNode::Split { first, second, .. } => {
                first.find_leaf(id).or_else(|| second.find_leaf(id))
            }
            PaneNode::Placeholder => None,
        }
    }

    fn collect_leaf_ids(&self) -> Vec<PaneId> {
        match self {
            PaneNode::Leaf(leaf) => vec![leaf.id],
            PaneNode::Split { first, second, .. } => {
                let mut ids = first.collect_leaf_ids();
                ids.extend(second.collect_leaf_ids());
                ids
            }
            PaneNode::Placeholder => vec![],
        }
    }

    fn collect_leaves_with_rects(&self, rect: PaneRect) -> Vec<(PaneId, PaneRect)> {
        match self {
            PaneNode::Leaf(leaf) => vec![(leaf.id, rect)],
            PaneNode::Placeholder => vec![],
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (first_rect, second_rect) = split_rect(rect, *direction, *ratio);
                let mut result = first.collect_leaves_with_rects(first_rect);
                result.extend(second.collect_leaves_with_rects(second_rect));
                result
            }
        }
    }

    fn collect_dividers(&self, rect: PaneRect) -> Vec<(PaneRect, SplitDirection)> {
        match self {
            PaneNode::Leaf(_) | PaneNode::Placeholder => vec![],
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let divider = compute_divider_rect(rect, *direction, *ratio);
                let (first_rect, second_rect) = split_rect(rect, *direction, *ratio);
                let mut result = vec![(divider, *direction)];
                result.extend(first.collect_dividers(first_rect));
                result.extend(second.collect_dividers(second_rect));
                result
            }
        }
    }

    fn contains_leaf(&self, id: PaneId) -> bool {
        match self {
            PaneNode::Leaf(leaf) => leaf.id == id,
            PaneNode::Placeholder => false,
            PaneNode::Split { first, second, .. } => {
                first.contains_leaf(id) || second.contains_leaf(id)
            }
        }
    }

    fn split_leaf(
        &mut self,
        id: PaneId,
        direction: SplitDirection,
        new_terminal: Terminal,
        new_child: Child,
    ) -> bool {
        match self {
            PaneNode::Leaf(leaf) if leaf.id == id => {
                // Take ownership of the existing leaf by swapping in a lightweight placeholder
                let old_self = std::mem::replace(self, PaneNode::Placeholder);
                let new_leaf = LeafPane {
                    id: next_pane_id(),
                    terminal: new_terminal,
                    child: new_child,
                    title: String::from("Terminal"),
                    scroll_offset: 0,
                };
                *self = PaneNode::Split {
                    direction,
                    ratio: 0.5,
                    first: Box::new(old_self),
                    second: Box::new(PaneNode::Leaf(Box::new(new_leaf))),
                };
                true
            }
            PaneNode::Leaf(_) | PaneNode::Placeholder => false,
            PaneNode::Split { first, second, .. } => {
                if first.contains_leaf(id) {
                    first.split_leaf(id, direction, new_terminal, new_child)
                } else {
                    second.split_leaf(id, direction, new_terminal, new_child)
                }
            }
        }
    }

    fn remove_leaf(&mut self, id: PaneId) -> RemoveResult {
        match self {
            PaneNode::Leaf(_) | PaneNode::Placeholder => RemoveResult::NotFound,
            PaneNode::Split { first, second, .. } => {
                if let PaneNode::Leaf(leaf) = first.as_ref() {
                    if leaf.id == id {
                        return RemoveResult::PromoteSibling;
                    }
                }
                if let PaneNode::Leaf(leaf) = second.as_ref() {
                    if leaf.id == id {
                        return RemoveResult::PromoteFirst;
                    }
                }
                match first.remove_leaf(id) {
                    RemoveResult::PromoteSibling => {
                        if let PaneNode::Split { second: s2, .. } = first.as_mut() {
                            let promoted = std::mem::replace(s2.as_mut(), PaneNode::Placeholder);
                            **first = promoted;
                        }
                        return RemoveResult::NotFound;
                    }
                    RemoveResult::PromoteFirst => {
                        if let PaneNode::Split { first: f1, .. } = first.as_mut() {
                            let promoted = std::mem::replace(f1.as_mut(), PaneNode::Placeholder);
                            **first = promoted;
                        }
                        return RemoveResult::NotFound;
                    }
                    RemoveResult::NotFound => {}
                }
                match second.remove_leaf(id) {
                    RemoveResult::PromoteSibling => {
                        if let PaneNode::Split { second: s2, .. } = second.as_mut() {
                            let promoted = std::mem::replace(s2.as_mut(), PaneNode::Placeholder);
                            **second = promoted;
                        }
                        return RemoveResult::NotFound;
                    }
                    RemoveResult::PromoteFirst => {
                        if let PaneNode::Split { first: f1, .. } = second.as_mut() {
                            let promoted = std::mem::replace(f1.as_mut(), PaneNode::Placeholder);
                            **second = promoted;
                        }
                        return RemoveResult::NotFound;
                    }
                    RemoveResult::NotFound => {}
                }
                RemoveResult::NotFound
            }
        }
    }

    fn leaf_count(&self) -> usize {
        match self {
            PaneNode::Leaf(_) => 1,
            PaneNode::Placeholder => 0,
            PaneNode::Split { first, second, .. } => first.leaf_count() + second.leaf_count(),
        }
    }

    fn for_each_leaf_mut<F: FnMut(&mut LeafPane)>(&mut self, f: &mut F) {
        match self {
            PaneNode::Leaf(leaf) => f(leaf),
            PaneNode::Placeholder => {}
            PaneNode::Split { first, second, .. } => {
                first.for_each_leaf_mut(f);
                second.for_each_leaf_mut(f);
            }
        }
    }

    fn retain_living(&mut self) -> bool {
        match self {
            PaneNode::Leaf(leaf) => leaf.child.is_running(),
            PaneNode::Placeholder => false,
            PaneNode::Split { first, second, .. } => {
                let first_alive = first.retain_living();
                let second_alive = second.retain_living();
                if !first_alive && !second_alive {
                    return false;
                }
                if !first_alive {
                    let promoted = std::mem::replace(second.as_mut(), PaneNode::Placeholder);
                    *self = promoted;
                    return true;
                }
                if !second_alive {
                    let promoted = std::mem::replace(first.as_mut(), PaneNode::Placeholder);
                    *self = promoted;
                    return true;
                }
                true
            }
        }
    }

    fn pane_at_position(&self, rect: PaneRect, px: f64, py: f64) -> Option<PaneId> {
        match self {
            PaneNode::Placeholder => None,
            PaneNode::Leaf(leaf) => {
                if px >= rect.x as f64
                    && px < (rect.x + rect.width) as f64
                    && py >= rect.y as f64
                    && py < (rect.y + rect.height) as f64
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
                let (first_rect, second_rect) = split_rect(rect, *direction, *ratio);
                first
                    .pane_at_position(first_rect, px, py)
                    .or_else(|| second.pane_at_position(second_rect, px, py))
            }
        }
    }

    fn divider_at_position(
        &self,
        rect: PaneRect,
        px: f64,
        py: f64,
        tolerance: f64,
    ) -> Option<DividerHit> {
        match self {
            PaneNode::Leaf(_) | PaneNode::Placeholder => None,
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let divider = compute_divider_rect(rect, *direction, *ratio);
                let (first_rect, second_rect) = split_rect(rect, *direction, *ratio);
                let on_divider = match direction {
                    SplitDirection::Vertical => {
                        let div_center = divider.x as f64 + divider.width as f64 / 2.0;
                        (px - div_center).abs() < tolerance + divider.width as f64 / 2.0
                            && py >= rect.y as f64
                            && py < (rect.y + rect.height) as f64
                    }
                    SplitDirection::Horizontal => {
                        let div_center = divider.y as f64 + divider.height as f64 / 2.0;
                        (py - div_center).abs() < tolerance + divider.height as f64 / 2.0
                            && px >= rect.x as f64
                            && px < (rect.x + rect.width) as f64
                    }
                };
                if on_divider {
                    return Some(DividerHit {
                        direction: *direction,
                        container_rect: rect,
                    });
                }
                first
                    .divider_at_position(first_rect, px, py, tolerance)
                    .or_else(|| second.divider_at_position(second_rect, px, py, tolerance))
            }
        }
    }

    fn update_divider_ratio(&mut self, rect: PaneRect, hit: &DividerHit, px: f64, py: f64) -> bool {
        match self {
            PaneNode::Leaf(_) | PaneNode::Placeholder => false,
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                if *direction == hit.direction
                    && rect.x == hit.container_rect.x
                    && rect.y == hit.container_rect.y
                    && rect.width == hit.container_rect.width
                    && rect.height == hit.container_rect.height
                {
                    let new_ratio = match direction {
                        SplitDirection::Vertical => {
                            ((px - rect.x as f64) / rect.width as f64).clamp(0.1, 0.9)
                        }
                        SplitDirection::Horizontal => {
                            ((py - rect.y as f64) / rect.height as f64).clamp(0.1, 0.9)
                        }
                    };
                    *ratio = new_ratio;
                    return true;
                }
                let (first_rect, second_rect) = split_rect(rect, *direction, *ratio);
                first.update_divider_ratio(first_rect, hit, px, py)
                    || second.update_divider_ratio(second_rect, hit, px, py)
            }
        }
    }

    fn neighbor_in_direction(
        &self,
        rect: PaneRect,
        active_id: PaneId,
        nav_direction: NavDirection,
    ) -> Option<PaneId> {
        let leaves = self.collect_leaves_with_rects(rect);
        let active_rect = leaves.iter().find(|(id, _)| *id == active_id)?.1;
        let active_cx = active_rect.x as f64 + active_rect.width as f64 / 2.0;
        let active_cy = active_rect.y as f64 + active_rect.height as f64 / 2.0;
        let mut best: Option<(PaneId, f64)> = None;
        for (id, r) in &leaves {
            if *id == active_id {
                continue;
            }
            let cx = r.x as f64 + r.width as f64 / 2.0;
            let cy = r.y as f64 + r.height as f64 / 2.0;
            let valid = match nav_direction {
                NavDirection::Left => cx < active_cx,
                NavDirection::Right => cx > active_cx,
                NavDirection::Up => cy < active_cy,
                NavDirection::Down => cy > active_cy,
            };
            if !valid {
                continue;
            }
            let dist = ((cx - active_cx).powi(2) + (cy - active_cy).powi(2)).sqrt();
            if best.is_none() || dist < best.unwrap().1 {
                best = Some((*id, dist));
            }
        }
        best.map(|(id, _)| id)
    }
}

enum RemoveResult {
    NotFound,
    PromoteSibling,
    PromoteFirst,
}

#[derive(Debug, Clone, Copy)]
enum NavDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy)]
struct DividerHit {
    direction: SplitDirection,
    container_rect: PaneRect,
}

fn split_rect(rect: PaneRect, direction: SplitDirection, ratio: f64) -> (PaneRect, PaneRect) {
    let divider = PANE_DIVIDER_WIDTH;
    match direction {
        SplitDirection::Vertical => {
            let first_w = ((rect.width as f64 - divider as f64) * ratio) as u32;
            let second_x = rect.x + first_w + divider;
            let second_w = rect.width.saturating_sub(first_w + divider);
            (
                PaneRect {
                    x: rect.x,
                    y: rect.y,
                    width: first_w,
                    height: rect.height,
                },
                PaneRect {
                    x: second_x,
                    y: rect.y,
                    width: second_w,
                    height: rect.height,
                },
            )
        }
        SplitDirection::Horizontal => {
            let first_h = ((rect.height as f64 - divider as f64) * ratio) as u32;
            let second_y = rect.y + first_h + divider;
            let second_h = rect.height.saturating_sub(first_h + divider);
            (
                PaneRect {
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: first_h,
                },
                PaneRect {
                    x: rect.x,
                    y: second_y,
                    width: rect.width,
                    height: second_h,
                },
            )
        }
    }
}

fn compute_divider_rect(rect: PaneRect, direction: SplitDirection, ratio: f64) -> PaneRect {
    let divider = PANE_DIVIDER_WIDTH;
    match direction {
        SplitDirection::Vertical => {
            let first_w = ((rect.width as f64 - divider as f64) * ratio) as u32;
            PaneRect {
                x: rect.x + first_w,
                y: rect.y,
                width: divider,
                height: rect.height,
            }
        }
        SplitDirection::Horizontal => {
            let first_h = ((rect.height as f64 - divider as f64) * ratio) as u32;
            PaneRect {
                x: rect.x,
                y: rect.y + first_h,
                width: rect.width,
                height: divider,
            }
        }
    }
}

struct Tab {
    pane_root: PaneNode,
    active_pane_id: PaneId,
}

impl Tab {
    fn new(terminal: Terminal, child: Child) -> Self {
        let root = PaneNode::new_leaf(terminal, child);
        let id = root.first_leaf_id();
        Self {
            pane_root: root,
            active_pane_id: id,
        }
    }

    fn title(&self) -> &str {
        self.pane_root
            .find_leaf(self.active_pane_id)
            .map(|l| l.title.as_str())
            .unwrap_or("Terminal")
    }
}

pub struct App {
    config: Config,
    window: Option<Rc<Window>>,
    renderer: Option<Renderer>,
    tabs: Vec<Tab>,
    active_tab: usize,
    #[allow(dead_code)]
    clipboard: Option<Clipboard>,
    modifiers: ModifiersState,
    mouse_cell: (u16, u16),
    mouse_pixel: (f64, f64),
    mouse_buttons: [bool; 3],
    last_render: Instant,
    needs_redraw: bool,
    focused: bool,
    tab_bar_height: u32,
    scrollbar_dragging: bool,
    scrollbar_drag_start_y: f64,
    scrollbar_drag_start_offset: usize,
    divider_dragging: Option<DividerHit>,
}

impl App {
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
            divider_dragging: None,
        })
    }

    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new()?;
        let window = WindowBuilder::new()
            .with_title("Mochi Terminal")
            .with_inner_size(LogicalSize::new(800, 600))
            .build(&event_loop)?;
        let window = Rc::new(window);
        self.init_graphics(window.clone())?;
        event_loop.run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);
            match event {
                Event::WindowEvent { event, .. } => {
                    self.handle_window_event(event, elwt);
                }
                Event::AboutToWait => {
                    self.poll_pty();
                    if !self.check_child() {
                        elwt.exit();
                        return;
                    }
                    if self.needs_redraw {
                        self.render();
                    }
                }
                _ => {}
            }
        })?;
        Ok(())
    }

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

    fn content_rect(&self) -> PaneRect {
        let (width, height) = if let Some(window) = &self.window {
            let size = window.inner_size();
            (size.width, size.height)
        } else {
            (800, 600)
        };
        PaneRect {
            x: 0,
            y: self.tab_bar_height,
            width,
            height: height.saturating_sub(self.tab_bar_height),
        }
    }

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
                self.tabs.push(Tab::new(terminal, child));
                self.active_tab = self.tabs.len() - 1;
                self.needs_redraw = true;
            }
            Err(e) => {
                log::error!("Failed to create new tab: {}", e);
            }
        }
    }

    fn close_current_tab(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        self.needs_redraw = true;
        true
    }

    fn close_active_pane(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }
        let tab = &mut self.tabs[self.active_tab];
        if tab.pane_root.leaf_count() <= 1 {
            return self.close_current_tab();
        }
        let active_id = tab.active_pane_id;
        let result = tab.pane_root.remove_leaf(active_id);
        match result {
            RemoveResult::PromoteSibling => {
                if let PaneNode::Split { second, .. } = &mut tab.pane_root {
                    let promoted = std::mem::replace(second.as_mut(), PaneNode::Placeholder);
                    tab.pane_root = promoted;
                }
            }
            RemoveResult::PromoteFirst => {
                if let PaneNode::Split { first, .. } = &mut tab.pane_root {
                    let promoted = std::mem::replace(first.as_mut(), PaneNode::Placeholder);
                    tab.pane_root = promoted;
                }
            }
            RemoveResult::NotFound => {}
        }
        let tab = &mut self.tabs[self.active_tab];
        let ids = tab.pane_root.collect_leaf_ids();
        if !ids.contains(&tab.active_pane_id) {
            tab.active_pane_id = tab.pane_root.first_leaf_id();
        }
        self.resize_panes_in_tab(self.active_tab);
        self.needs_redraw = true;
        true
    }

    #[allow(dead_code)]
    fn switch_to_tab(&mut self, index: usize) {
        if index < self.tabs.len() && index != self.active_tab {
            self.active_tab = index;
            self.needs_redraw = true;
        }
    }

    fn split_active_pane(&mut self, direction: SplitDirection) {
        if self.tabs.is_empty() {
            return;
        }
        let Some(renderer) = &self.renderer else {
            return;
        };
        let cell_size = renderer.cell_size();
        let content_rect = self.content_rect();
        let tab = &mut self.tabs[self.active_tab];
        let active_id = tab.active_pane_id;
        let leaves = tab.pane_root.collect_leaves_with_rects(content_rect);
        let current_rect = leaves.iter().find(|(id, _)| *id == active_id);
        let (cols, rows) = if let Some((_, rect)) = current_rect {
            let (_, new_rect) = split_rect(*rect, direction, 0.5);
            let c = (new_rect.width as f32 / cell_size.width) as usize;
            let r = (new_rect.height as f32 / cell_size.height) as usize;
            (c.max(1), r.max(1))
        } else {
            (80, 24)
        };
        let new_terminal = Terminal::new(cols, rows);
        match Child::spawn_shell(WindowSize::new(cols as u16, rows as u16)) {
            Ok(new_child) => {
                let _ = new_child.set_nonblocking(true);
                let ids_before = tab.pane_root.collect_leaf_ids();
                if tab
                    .pane_root
                    .split_leaf(active_id, direction, new_terminal, new_child)
                {
                    let all_ids = tab.pane_root.collect_leaf_ids();
                    let new_id = all_ids
                        .iter()
                        .find(|id| !ids_before.contains(id))
                        .copied()
                        .unwrap_or(active_id);
                    tab.active_pane_id = new_id;
                    self.resize_panes_in_tab(self.active_tab);
                    self.needs_redraw = true;
                }
            }
            Err(e) => {
                log::error!("Failed to create new pane: {}", e);
            }
        }
    }

    fn navigate_pane(&mut self, nav_direction: NavDirection) {
        if self.tabs.is_empty() {
            return;
        }
        let content_rect = self.content_rect();
        let tab = &mut self.tabs[self.active_tab];
        if let Some(neighbor_id) =
            tab.pane_root
                .neighbor_in_direction(content_rect, tab.active_pane_id, nav_direction)
        {
            tab.active_pane_id = neighbor_id;
            self.needs_redraw = true;
        }
    }

    fn resize_panes_in_tab(&mut self, tab_idx: usize) {
        if tab_idx >= self.tabs.len() {
            return;
        }
        let Some(renderer) = &self.renderer else {
            return;
        };
        let cell_size = renderer.cell_size();
        let content_rect = self.content_rect();
        let tab = &mut self.tabs[tab_idx];
        let leaves = tab.pane_root.collect_leaves_with_rects(content_rect);
        for (id, rect) in leaves {
            let cols = (rect.width as f32 / cell_size.width) as usize;
            let rows = (rect.height as f32 / cell_size.height) as usize;
            if cols > 0 && rows > 0 {
                if let Some(leaf) = tab.pane_root.find_leaf_mut(id) {
                    leaf.terminal.resize(cols, rows);
                    let _ = leaf.child.resize(WindowSize::new(cols as u16, rows as u16));
                }
            }
        }
    }

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
                } else {
                    self.switch_to_tab(tab_index);
                }
            }
        }
    }

    fn handle_resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        renderer.resize(size.width, size.height);
        let cell_size = renderer.cell_size();
        self.tab_bar_height = compute_tab_bar_height(&cell_size);
        for i in 0..self.tabs.len() {
            self.resize_panes_in_tab(i);
        }
        self.needs_redraw = true;
    }

    fn handle_key_input(&mut self, event: &winit::event::KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }
        let ctrl_shift = self.modifiers.control_key() && self.modifiers.shift_key();
        if ctrl_shift {
            match &event.logical_key {
                Key::Character(c) if c.to_lowercase() == "c" => {
                    self.handle_copy();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "v" => {
                    self.handle_paste();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "f" => {
                    self.handle_find();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "r" => {
                    self.handle_reload_config();
                    return;
                }
                #[cfg(target_os = "macos")]
                Key::Character(c) if c.to_lowercase() == "t" => {
                    self.handle_toggle_theme();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "d" => {
                    self.split_active_pane(SplitDirection::Vertical);
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "e" => {
                    self.split_active_pane(SplitDirection::Horizontal);
                    return;
                }
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
                Key::Character(c) if c.to_lowercase() == "t" && !self.modifiers.shift_key() => {
                    self.create_new_tab();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "w" => {
                    if !self.close_active_pane() {
                        self.tabs.clear();
                    }
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "d" && !self.modifiers.shift_key() => {
                    self.split_active_pane(SplitDirection::Vertical);
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "d" && self.modifiers.shift_key() => {
                    self.split_active_pane(SplitDirection::Horizontal);
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
        let Some(leaf) = tab.pane_root.find_leaf_mut(tab.active_pane_id) else {
            return;
        };
        if let Some(text) = event.text_with_all_modifiers() {
            if !text.is_empty() {
                let first_char = text.chars().next().unwrap();
                let char_code = first_char as u32;
                if (1..=26).contains(&char_code) || char_code == 0x7F {
                    let _ = leaf.child.write_all(&[first_char as u8]);
                    return;
                }
            }
        }
        if let Key::Character(c) = &event.logical_key {
            if let Some(ch) = c.chars().next() {
                let char_code = ch as u32;
                if (1..=26).contains(&char_code) || char_code == 0x7F {
                    let _ = leaf.child.write_all(&[ch as u8]);
                    return;
                }
            }
        }
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
                _ => {}
            }
        }
        let application_cursor_keys = leaf.terminal.screen().modes().cursor_keys_application;
        if let Some(data) = encode_key(&event.logical_key, self.modifiers, application_cursor_keys)
        {
            let _ = leaf.child.write_all(&data);
        }
    }

    fn change_font_size(&mut self, delta: f32) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        let current_size = renderer.font_size();
        let new_size = (current_size + delta).clamp(8.0, 72.0);
        if (new_size - current_size).abs() < 0.1 {
            return;
        }
        renderer.set_font_size(new_size);
        let cell_size = renderer.cell_size();
        self.tab_bar_height = compute_tab_bar_height(&cell_size);
        for i in 0..self.tabs.len() {
            self.resize_panes_in_tab(i);
        }
        self.needs_redraw = true;
    }

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
        for i in 0..self.tabs.len() {
            self.resize_panes_in_tab(i);
        }
        self.needs_redraw = true;
    }

    fn handle_mouse_input(&mut self, button: MouseButton, state: ElementState) {
        if self.tabs.is_empty() {
            return;
        }
        if button == MouseButton::Left
            && state == ElementState::Pressed
            && self.mouse_pixel.1 < self.tab_bar_height as f64
        {
            self.handle_tab_bar_click(self.mouse_pixel.0);
            return;
        }
        if button == MouseButton::Left {
            if state == ElementState::Pressed {
                let content_rect = self.content_rect();
                let tab = &self.tabs[self.active_tab];
                if let Some(hit) = tab.pane_root.divider_at_position(
                    content_rect,
                    self.mouse_pixel.0,
                    self.mouse_pixel.1,
                    4.0,
                ) {
                    self.divider_dragging = Some(hit);
                    return;
                }
                if let Some(pane_id) = tab.pane_root.pane_at_position(
                    content_rect,
                    self.mouse_pixel.0,
                    self.mouse_pixel.1,
                ) {
                    if pane_id != tab.active_pane_id {
                        let tab = &mut self.tabs[self.active_tab];
                        tab.active_pane_id = pane_id;
                        self.needs_redraw = true;
                        return;
                    }
                }
            } else if self.divider_dragging.is_some() {
                self.divider_dragging = None;
                self.resize_panes_in_tab(self.active_tab);
                return;
            }
        }
        if button == MouseButton::Left {
            if state == ElementState::Pressed {
                let content_rect = self.content_rect();
                let tab = &self.tabs[self.active_tab];
                let leaves = tab.pane_root.collect_leaves_with_rects(content_rect);
                if let Some((_, pane_rect)) =
                    leaves.iter().find(|(id, _)| *id == tab.active_pane_id)
                {
                    let scrollbar_width = 12.0;
                    let pane_right = (pane_rect.x + pane_rect.width) as f64;
                    if self.mouse_pixel.0 >= pane_right - scrollbar_width
                        && self.mouse_pixel.0 < pane_right
                        && self.mouse_pixel.1 >= pane_rect.y as f64
                        && self.mouse_pixel.1 < (pane_rect.y + pane_rect.height) as f64
                    {
                        if let Some(leaf) = tab.pane_root.find_leaf(tab.active_pane_id) {
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
        let tab = &mut self.tabs[self.active_tab];
        let active_id = tab.active_pane_id;
        let Some(leaf) = tab.pane_root.find_leaf_mut(active_id) else {
            return;
        };
        let modes = leaf.terminal.screen().modes().clone();
        if !modes.mouse_tracking_enabled() {
            if button == MouseButton::Left {
                let col = self.mouse_cell.0 as usize;
                let row = self.mouse_cell.1 as isize - leaf.scroll_offset as isize;
                if state == ElementState::Pressed {
                    leaf.terminal
                        .screen_mut()
                        .selection_mut()
                        .start(Point::new(col, row), SelectionType::Normal);
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

    fn handle_mouse_motion(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        self.mouse_pixel = (position.x, position.y);
        if self.tabs.is_empty() {
            return;
        }
        if let Some(hit) = self.divider_dragging {
            let content_rect = self.content_rect();
            let tab = &mut self.tabs[self.active_tab];
            tab.pane_root
                .update_divider_ratio(content_rect, &hit, position.x, position.y);
            self.needs_redraw = true;
            return;
        }
        if self.scrollbar_dragging {
            let content_rect = self.content_rect();
            let tab = &mut self.tabs[self.active_tab];
            let active_id = tab.active_pane_id;
            let leaves = tab.pane_root.collect_leaves_with_rects(content_rect);
            if let Some((_, pane_rect)) = leaves.iter().find(|(id, _)| *id == active_id) {
                let pane_height = pane_rect.height as f64;
                if let Some(leaf) = tab.pane_root.find_leaf_mut(active_id) {
                    let scrollback_len = leaf.terminal.screen().scrollback().len();
                    let visible_rows = leaf.terminal.screen().rows();
                    if scrollback_len > 0 && pane_height > 0.0 {
                        let delta_y = position.y - self.scrollbar_drag_start_y;
                        let total_lines = scrollback_len + visible_rows;
                        let thumb_height =
                            ((visible_rows as f64 / total_lines as f64) * pane_height).max(20.0);
                        let scroll_range = pane_height - thumb_height;
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
        let Some(renderer) = &self.renderer else {
            return;
        };
        let cell_size = renderer.cell_size();
        let content_rect = self.content_rect();
        let tab = &mut self.tabs[self.active_tab];
        let active_id = tab.active_pane_id;
        let leaves = tab.pane_root.collect_leaves_with_rects(content_rect);
        if let Some((_, pane_rect)) = leaves.iter().find(|(id, _)| *id == active_id) {
            let local_x = (position.x - pane_rect.x as f64).max(0.0);
            let local_y = (position.y - pane_rect.y as f64).max(0.0);
            let col = (local_x / cell_size.width as f64) as u16;
            let row = (local_y / cell_size.height as f64) as u16;
            if col == self.mouse_cell.0 && row == self.mouse_cell.1 {
                return;
            }
            self.mouse_cell = (col, row);
            let Some(leaf) = tab.pane_root.find_leaf_mut(active_id) else {
                return;
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

    fn handle_mouse_scroll(&mut self, delta: MouseScrollDelta) {
        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];
        let active_id = tab.active_pane_id;
        let Some(leaf) = tab.pane_root.find_leaf_mut(active_id) else {
            return;
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

    fn handle_copy(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let tab = &self.tabs[self.active_tab];
        let Some(leaf) = tab.pane_root.find_leaf(tab.active_pane_id) else {
            return;
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
            log::warn!("Failed to copy: {}", e);
        }
    }

    fn handle_paste(&mut self) {
        let Some(clipboard) = &mut self.clipboard else {
            return;
        };
        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];
        let active_id = tab.active_pane_id;
        let Some(leaf) = tab.pane_root.find_leaf_mut(active_id) else {
            return;
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
                    log::warn!("Paste error: {}", e);
                }
            }
            Err(e) => {
                log::warn!("Clipboard error: {}", e);
            }
        }
    }

    fn handle_find(&mut self) {
        log::info!("Find requested - not yet implemented");
    }

    #[cfg(target_os = "macos")]
    fn handle_new_window(&mut self) {
        if let Ok(exe_path) = std::env::current_exe() {
            match std::process::Command::new(&exe_path).spawn() {
                Ok(child) => {
                    std::thread::spawn(move || {
                        let mut child = child;
                        let _ = child.wait();
                    });
                }
                Err(e) => {
                    log::error!("Failed to spawn window: {}", e);
                }
            }
        }
    }

    fn handle_reload_config(&mut self) {
        match Config::load() {
            Some(new_config) => {
                self.config.theme = new_config.theme;
                self.config.font = new_config.font.clone();
                self.config.keybindings = new_config.keybindings.clone();
                self.config.security = new_config.security.clone();
                if let Some(renderer) = &mut self.renderer {
                    renderer.set_colors(self.config.effective_colors());
                }
                self.needs_redraw = true;
            }
            None => {
                log::warn!("No config file found");
            }
        }
    }

    #[allow(dead_code)]
    fn handle_toggle_theme(&mut self) {
        let new_theme = self.config.theme.next();
        self.config.theme = new_theme;
        if let Some(renderer) = &mut self.renderer {
            renderer.set_colors(self.config.effective_colors());
        }
        self.needs_redraw = true;
    }

    fn handle_focus(&mut self, focused: bool) {
        self.focused = focused;
        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];
        let active_id = tab.active_pane_id;
        if let Some(leaf) = tab.pane_root.find_leaf_mut(active_id) {
            if leaf.terminal.screen().modes().focus_events {
                let data = encode_focus(focused);
                let _ = leaf.child.write_all(&data);
            }
        }
    }

    fn poll_pty(&mut self) {
        let mut buf = [0u8; 65536];
        let mut any_output = false;
        for (tab_idx, tab) in self.tabs.iter_mut().enumerate() {
            tab.pane_root.for_each_leaf_mut(&mut |leaf: &mut LeafPane| {
                let mut received_output = false;
                loop {
                    match leaf.child.pty_mut().try_read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            leaf.terminal.process(&buf[..n]);
                            received_output = true;
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                        Err(_) => break,
                    }
                }
                if received_output {
                    any_output = true;
                    if leaf.scroll_offset > 0 {
                        leaf.scroll_offset = 0;
                    }
                }
                if leaf.terminal.take_title_changed() {
                    leaf.title = leaf.terminal.title().to_string();
                }
                if leaf.terminal.take_bell() {
                    log::debug!("Bell!");
                }
                let responses = leaf.terminal.take_pending_responses();
                for response in responses {
                    if let Err(e) = leaf.child.write_all(&response) {
                        log::warn!("Response error: {}", e);
                    }
                }
            });
            if tab_idx == self.active_tab {
                let active_pane_id = tab.active_pane_id;
                if let Some(leaf) = tab.pane_root.find_leaf(active_pane_id) {
                    if let Some(window) = &self.window {
                        let pane_count = tab.pane_root.leaf_count();
                        if pane_count > 1 {
                            window.set_title(&format!("{} [{} panes]", leaf.title, pane_count));
                        } else {
                            window.set_title(&leaf.title);
                        }
                    }
                }
            }
        }
        if any_output {
            self.needs_redraw = true;
        }
    }

    fn render(&mut self) {
        if self.renderer.is_none() || self.tabs.is_empty() {
            return;
        }
        let content_rect = self.content_rect();
        let tab_bar_height = self.tab_bar_height;
        let active_tab = self.active_tab;
        let tab_infos: Vec<TabInfo<'_>> = self
            .tabs
            .iter()
            .map(|t| TabInfo { title: t.title() })
            .collect();
        let tab = &self.tabs[active_tab];
        let leaves = tab.pane_root.collect_leaves_with_rects(content_rect);
        let pane_renders: Vec<_> = leaves
            .iter()
            .filter_map(|(id, rect)| {
                tab.pane_root.find_leaf(*id).map(|leaf| {
                    (
                        *id,
                        *rect,
                        leaf.terminal.screen(),
                        leaf.scroll_offset,
                        *id == tab.active_pane_id,
                    )
                })
            })
            .collect();
        let dividers = tab.pane_root.collect_dividers(content_rect);
        let renderer = self.renderer.as_mut().unwrap();
        if let Err(e) = renderer.render_panes(
            &pane_renders,
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

    fn check_child(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }
        self.tabs.retain_mut(|tab| {
            let alive = tab.pane_root.retain_living();
            if alive {
                let ids = tab.pane_root.collect_leaf_ids();
                if !ids.contains(&tab.active_pane_id) {
                    tab.active_pane_id = tab.pane_root.first_leaf_id();
                }
            }
            alive
        });
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len().saturating_sub(1);
        }
        !self.tabs.is_empty()
    }
}
