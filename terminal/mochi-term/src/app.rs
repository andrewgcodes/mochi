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
use crate::renderer::{PaneRect, PaneRenderInfo, Renderer, TabInfo};
use crate::terminal::Terminal;

/// Padding added to cell height to compute tab bar height
const TAB_BAR_PADDING: u32 = 8;
/// Maximum width of a single tab in pixels
const TAB_MAX_WIDTH: u32 = 200;
/// Width of the close button area in each tab
const CLOSE_BTN_WIDTH: u32 = 20;
/// Width of the new tab (+) button
const NEW_TAB_BTN_WIDTH: u32 = 32;
/// Width of divider between panes in pixels
const DIVIDER_WIDTH: u32 = 4;

/// Compute tab bar height from the current cell size so it scales with HiDPI / font size.
fn compute_tab_bar_height(cell_size: &crate::renderer::CellSize) -> u32 {
    cell_size.height as u32 + TAB_BAR_PADDING
}

type PaneId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SplitDirection {
    Horizontal,
    Vertical,
}

#[allow(clippy::large_enum_variant)]
enum PaneNode {
    Leaf {
        id: PaneId,
        terminal: Terminal,
        child: Child,
        scroll_offset: usize,
        title: String,
    },
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

impl PaneNode {
    fn new_leaf(id: PaneId, terminal: Terminal, child: Child) -> Self {
        PaneNode::Leaf {
            id,
            terminal,
            child,
            scroll_offset: 0,
            title: String::from("Terminal"),
        }
    }

    fn leaf_ids(&self) -> Vec<PaneId> {
        match self {
            PaneNode::Leaf { id, .. } => vec![*id],
            PaneNode::Split { first, second, .. } => {
                let mut ids = first.leaf_ids();
                ids.extend(second.leaf_ids());
                ids
            }
        }
    }

    fn pane_count(&self) -> usize {
        match self {
            PaneNode::Leaf { .. } => 1,
            PaneNode::Split { first, second, .. } => first.pane_count() + second.pane_count(),
        }
    }

    fn contains_pane(&self, pane_id: PaneId) -> bool {
        match self {
            PaneNode::Leaf { id, .. } => *id == pane_id,
            PaneNode::Split { first, second, .. } => {
                first.contains_pane(pane_id) || second.contains_pane(pane_id)
            }
        }
    }

    fn find_leaf_mut(
        &mut self,
        pane_id: PaneId,
    ) -> Option<(&mut Terminal, &mut Child, &mut usize, &mut String)> {
        match self {
            PaneNode::Leaf {
                id,
                terminal,
                child,
                scroll_offset,
                title,
            } if *id == pane_id => Some((terminal, child, scroll_offset, title)),
            PaneNode::Split { first, second, .. } => first
                .find_leaf_mut(pane_id)
                .or_else(|| second.find_leaf_mut(pane_id)),
            _ => None,
        }
    }

    fn find_leaf_ref(&self, pane_id: PaneId) -> Option<(&Terminal, &Child, usize)> {
        match self {
            PaneNode::Leaf {
                id,
                terminal,
                child,
                scroll_offset,
                ..
            } if *id == pane_id => Some((terminal, child, *scroll_offset)),
            PaneNode::Split { first, second, .. } => first
                .find_leaf_ref(pane_id)
                .or_else(|| second.find_leaf_ref(pane_id)),
            _ => None,
        }
    }

    fn pane_title(&self, pane_id: PaneId) -> Option<&str> {
        match self {
            PaneNode::Leaf { id, title, .. } if *id == pane_id => Some(title),
            PaneNode::Split { first, second, .. } => first
                .pane_title(pane_id)
                .or_else(|| second.pane_title(pane_id)),
            _ => None,
        }
    }

    fn compute_layout(&self, rect: PaneRect) -> Vec<(PaneId, PaneRect)> {
        match self {
            PaneNode::Leaf { id, .. } => vec![(*id, rect)],
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (r1, r2) = Self::split_rect(rect, *direction, *ratio);
                let mut out = first.compute_layout(r1);
                out.extend(second.compute_layout(r2));
                out
            }
        }
    }

    fn compute_dividers(&self, rect: PaneRect) -> Vec<PaneRect> {
        match self {
            PaneNode::Leaf { .. } => vec![],
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (r1, r2) = Self::split_rect(rect, *direction, *ratio);
                let dr = Self::divider_rect(rect, *direction, *ratio);
                let mut out = vec![dr];
                out.extend(first.compute_dividers(r1));
                out.extend(second.compute_dividers(r2));
                out
            }
        }
    }

    fn split_rect(rect: PaneRect, direction: SplitDirection, ratio: f32) -> (PaneRect, PaneRect) {
        match direction {
            SplitDirection::Vertical => {
                let usable = rect.width.saturating_sub(DIVIDER_WIDTH);
                let first_w = (usable as f32 * ratio) as u32;
                let second_w = usable.saturating_sub(first_w);
                (
                    PaneRect {
                        x: rect.x,
                        y: rect.y,
                        width: first_w,
                        height: rect.height,
                    },
                    PaneRect {
                        x: rect.x + first_w + DIVIDER_WIDTH,
                        y: rect.y,
                        width: second_w,
                        height: rect.height,
                    },
                )
            }
            SplitDirection::Horizontal => {
                let usable = rect.height.saturating_sub(DIVIDER_WIDTH);
                let first_h = (usable as f32 * ratio) as u32;
                let second_h = usable.saturating_sub(first_h);
                (
                    PaneRect {
                        x: rect.x,
                        y: rect.y,
                        width: rect.width,
                        height: first_h,
                    },
                    PaneRect {
                        x: rect.x,
                        y: rect.y + first_h + DIVIDER_WIDTH,
                        width: rect.width,
                        height: second_h,
                    },
                )
            }
        }
    }

    fn divider_rect(rect: PaneRect, direction: SplitDirection, ratio: f32) -> PaneRect {
        match direction {
            SplitDirection::Vertical => {
                let usable = rect.width.saturating_sub(DIVIDER_WIDTH);
                let first_w = (usable as f32 * ratio) as u32;
                PaneRect {
                    x: rect.x + first_w,
                    y: rect.y,
                    width: DIVIDER_WIDTH,
                    height: rect.height,
                }
            }
            SplitDirection::Horizontal => {
                let usable = rect.height.saturating_sub(DIVIDER_WIDTH);
                let first_h = (usable as f32 * ratio) as u32;
                PaneRect {
                    x: rect.x,
                    y: rect.y + first_h,
                    width: rect.width,
                    height: DIVIDER_WIDTH,
                }
            }
        }
    }

    fn resize_pane(&mut self, pane_id: PaneId, cols: usize, rows: usize) {
        match self {
            PaneNode::Leaf {
                id,
                terminal,
                child,
                ..
            } if *id == pane_id => {
                terminal.resize(cols, rows);
                let _ = child.resize(WindowSize::new(cols as u16, rows as u16));
            }
            PaneNode::Split { first, second, .. } => {
                first.resize_pane(pane_id, cols, rows);
                second.resize_pane(pane_id, cols, rows);
            }
            _ => {}
        }
    }

    fn for_each_leaf_mut<F>(&mut self, f: &mut F)
    where
        F: FnMut(PaneId, &mut Terminal, &mut Child, &mut usize, &mut String),
    {
        match self {
            PaneNode::Leaf {
                id,
                terminal,
                child,
                scroll_offset,
                title,
            } => {
                f(*id, terminal, child, scroll_offset, title);
            }
            PaneNode::Split { first, second, .. } => {
                first.for_each_leaf_mut(f);
                second.for_each_leaf_mut(f);
            }
        }
    }

    fn split_pane(
        self,
        target_id: PaneId,
        direction: SplitDirection,
        new_id: PaneId,
        new_terminal: Terminal,
        new_child: Child,
    ) -> (Self, bool) {
        match self {
            PaneNode::Leaf {
                id,
                terminal,
                child,
                scroll_offset,
                title,
            } => {
                if id == target_id {
                    let old_leaf = PaneNode::Leaf {
                        id,
                        terminal,
                        child,
                        scroll_offset,
                        title,
                    };
                    let new_leaf = PaneNode::new_leaf(new_id, new_terminal, new_child);
                    (
                        PaneNode::Split {
                            direction,
                            ratio: 0.5,
                            first: Box::new(old_leaf),
                            second: Box::new(new_leaf),
                        },
                        true,
                    )
                } else {
                    (
                        PaneNode::Leaf {
                            id,
                            terminal,
                            child,
                            scroll_offset,
                            title,
                        },
                        false,
                    )
                }
            }
            PaneNode::Split {
                direction: dir,
                ratio,
                first,
                second,
            } => {
                if first.contains_pane(target_id) {
                    let (new_first, found) =
                        (*first).split_pane(target_id, direction, new_id, new_terminal, new_child);
                    (
                        PaneNode::Split {
                            direction: dir,
                            ratio,
                            first: Box::new(new_first),
                            second,
                        },
                        found,
                    )
                } else {
                    let (new_second, found) =
                        (*second).split_pane(target_id, direction, new_id, new_terminal, new_child);
                    (
                        PaneNode::Split {
                            direction: dir,
                            ratio,
                            first,
                            second: Box::new(new_second),
                        },
                        found,
                    )
                }
            }
        }
    }

    fn remove_pane(self, target_id: PaneId) -> Option<Self> {
        match self {
            PaneNode::Leaf {
                id,
                terminal,
                child,
                scroll_offset,
                title,
            } => {
                if id == target_id {
                    None
                } else {
                    Some(PaneNode::Leaf {
                        id,
                        terminal,
                        child,
                        scroll_offset,
                        title,
                    })
                }
            }
            PaneNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let in_first = first.contains_pane(target_id);
                if in_first {
                    match (*first).remove_pane(target_id) {
                        Some(new_first) => Some(PaneNode::Split {
                            direction,
                            ratio,
                            first: Box::new(new_first),
                            second,
                        }),
                        None => Some(*second),
                    }
                } else {
                    match (*second).remove_pane(target_id) {
                        Some(new_second) => Some(PaneNode::Split {
                            direction,
                            ratio,
                            first,
                            second: Box::new(new_second),
                        }),
                        None => Some(*first),
                    }
                }
            }
        }
    }

    fn dead_pane_ids(&self) -> Vec<PaneId> {
        match self {
            PaneNode::Leaf { id, child, .. } => {
                if child.is_running() {
                    vec![]
                } else {
                    vec![*id]
                }
            }
            PaneNode::Split { first, second, .. } => {
                let mut ids = first.dead_pane_ids();
                ids.extend(second.dead_pane_ids());
                ids
            }
        }
    }
}

struct Tab {
    root: Option<PaneNode>,
    active_pane: PaneId,
    title: String,
}

impl Tab {
    fn new(id: PaneId, terminal: Terminal, child: Child) -> Self {
        Self {
            root: Some(PaneNode::new_leaf(id, terminal, child)),
            active_pane: id,
            title: String::from("Terminal"),
        }
    }

    fn sync_title(&mut self) {
        if let Some(root) = &self.root {
            if let Some(t) = root.pane_title(self.active_pane) {
                self.title = t.to_string();
            }
        }
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
    next_pane_id: PaneId,
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
            next_pane_id: 1,
        })
    }

    fn alloc_pane_id(&mut self) -> PaneId {
        let id = self.next_pane_id;
        self.next_pane_id += 1;
        id
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
                        log::info!("Child process exited");
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
        let pane_id = self.alloc_pane_id();
        let tab = Tab::new(pane_id, terminal, child);
        self.tabs.push(tab);
        self.active_tab = 0;
        self.window = Some(window);
        self.renderer = Some(renderer);
        Ok(())
    }

    fn content_rect(&self) -> PaneRect {
        let (w, h) = if let Some(r) = &self.renderer {
            (r.current_width(), r.current_height())
        } else {
            (0, 0)
        };
        PaneRect {
            x: 0,
            y: self.tab_bar_height,
            width: w,
            height: h.saturating_sub(self.tab_bar_height),
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
                let pane_id = self.alloc_pane_id();
                let tab = Tab::new(pane_id, terminal, child);
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

    #[allow(dead_code)]
    fn switch_to_tab(&mut self, index: usize) {
        if index < self.tabs.len() && index != self.active_tab {
            self.active_tab = index;
            self.needs_redraw = true;
            log::info!("Switched to tab {}", index + 1);
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
                    log::info!("Closed tab via click {}", tab_index + 1);
                } else {
                    self.switch_to_tab(tab_index);
                }
            }
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
        let content = self.content_rect();
        let new_id = self.alloc_pane_id();
        let tab = &mut self.tabs[self.active_tab];
        let target_id = tab.active_pane;
        let root = match tab.root.take() {
            Some(r) => r,
            None => return,
        };
        let layout = root.compute_layout(content);
        let target_rect = layout
            .iter()
            .find(|(pid, _)| *pid == target_id)
            .map(|(_, r)| *r);
        let target_rect = match target_rect {
            Some(r) => r,
            None => {
                tab.root = Some(root);
                return;
            }
        };
        let (_, new_rect) = PaneNode::split_rect(target_rect, direction, 0.5);
        let cols = (new_rect.width as f32 / cell_size.width) as usize;
        let rows = (new_rect.height as f32 / cell_size.height) as usize;
        let new_terminal = Terminal::new(cols.max(1), rows.max(1));
        let new_child = match Child::spawn_shell(WindowSize::new(cols as u16, rows as u16)) {
            Ok(c) => {
                let _ = c.set_nonblocking(true);
                c
            }
            Err(e) => {
                log::error!("Failed to spawn shell for new pane: {}", e);
                tab.root = Some(root);
                return;
            }
        };
        let (new_root, ok) = root.split_pane(target_id, direction, new_id, new_terminal, new_child);
        tab.root = Some(new_root);
        if ok {
            tab.active_pane = new_id;
            tab.sync_title();
            self.resize_all_panes_in_tab(self.active_tab);
            self.needs_redraw = true;
            log::info!(
                "Split pane {} {:?}, new pane {}",
                target_id,
                direction,
                new_id
            );
        }
    }

    fn close_active_pane(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];
        let root = match tab.root.take() {
            Some(r) => r,
            None => return,
        };
        if root.pane_count() <= 1 {
            tab.root = Some(root);
            if !self.close_current_tab() {
                self.tabs.clear();
            }
            return;
        }
        let dead_id = tab.active_pane;
        let ids_before = root.leaf_ids();
        let remaining = root.remove_pane(dead_id);
        tab.root = remaining;
        if let Some(ref r) = tab.root {
            let ids = r.leaf_ids();
            let old_idx = ids_before.iter().position(|id| *id == dead_id).unwrap_or(0);
            let new_active = if old_idx < ids.len() {
                ids[old_idx]
            } else {
                *ids.last().unwrap_or(&ids[0])
            };
            tab.active_pane = new_active;
            tab.sync_title();
        }
        self.resize_all_panes_in_tab(self.active_tab);
        self.needs_redraw = true;
    }

    fn focus_next_pane(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];
        let Some(root) = &tab.root else { return };
        let ids = root.leaf_ids();
        if ids.len() <= 1 {
            return;
        }
        let cur = ids
            .iter()
            .position(|id| *id == tab.active_pane)
            .unwrap_or(0);
        tab.active_pane = ids[(cur + 1) % ids.len()];
        tab.sync_title();
        self.needs_redraw = true;
    }

    fn focus_prev_pane(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];
        let Some(root) = &tab.root else { return };
        let ids = root.leaf_ids();
        if ids.len() <= 1 {
            return;
        }
        let cur = ids
            .iter()
            .position(|id| *id == tab.active_pane)
            .unwrap_or(0);
        let prev = if cur == 0 { ids.len() - 1 } else { cur - 1 };
        tab.active_pane = ids[prev];
        tab.sync_title();
        self.needs_redraw = true;
    }

    fn resize_all_panes_in_tab(&mut self, tab_idx: usize) {
        let Some(renderer) = &self.renderer else {
            return;
        };
        let cell_size = renderer.cell_size();
        let content = self.content_rect();
        let tab = &mut self.tabs[tab_idx];
        let Some(root) = &mut tab.root else { return };
        let layout = root.compute_layout(content);
        for (pid, rect) in layout {
            let cols = (rect.width as f32 / cell_size.width) as usize;
            let rows = (rect.height as f32 / cell_size.height) as usize;
            if cols > 0 && rows > 0 {
                root.resize_pane(pid, cols, rows);
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
        let tab_bar_height = self.tab_bar_height;
        let content = PaneRect {
            x: 0,
            y: tab_bar_height,
            width: size.width,
            height: size.height.saturating_sub(tab_bar_height),
        };
        for tab in &mut self.tabs {
            if let Some(root) = &mut tab.root {
                let layout = root.compute_layout(content);
                for (pid, rect) in layout {
                    let cols = (rect.width as f32 / cell_size.width) as usize;
                    let rows = (rect.height as f32 / cell_size.height) as usize;
                    if cols > 0 && rows > 0 {
                        root.resize_pane(pid, cols, rows);
                    }
                }
            }
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
                Key::Character(c) if c == "]" => {
                    self.focus_next_pane();
                    return;
                }
                Key::Character(c) if c == "[" => {
                    self.focus_prev_pane();
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
                Key::Character(c) if c.to_lowercase() == "t" => {
                    self.create_new_tab();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "w" => {
                    self.close_active_pane();
                    return;
                }
                Key::Character(c) if c.to_lowercase() == "d" => {
                    self.split_active_pane(SplitDirection::Vertical);
                    return;
                }
                Key::Character(c) if c == "]" => {
                    self.focus_next_pane();
                    return;
                }
                Key::Character(c) if c == "[" => {
                    self.focus_prev_pane();
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
                    self.close_active_pane();
                    return;
                }
                _ => {}
            }
        }

        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];
        let active_pane = tab.active_pane;
        let root = match tab.root.as_mut() {
            Some(r) => r,
            None => return,
        };
        let (terminal, child, _, _) = match root.find_leaf_mut(active_pane) {
            Some(leaf) => leaf,
            None => return,
        };

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
                    let _ = child.write_all(&[first_char as u8]);
                    return;
                }
            }
        }

        if let Key::Character(c) = &event.logical_key {
            if let Some(ch) = c.chars().next() {
                let char_code = ch as u32;
                if (1..=26).contains(&char_code) || char_code == 0x7F {
                    log::debug!(
                        "Sending control character from logical_key: {:?} (0x{:02x})",
                        ch,
                        ch as u8
                    );
                    let _ = child.write_all(&[ch as u8]);
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

        let application_cursor_keys = terminal.screen().modes().cursor_keys_application;
        if let Some(data) = encode_key(&event.logical_key, self.modifiers, application_cursor_keys)
        {
            log::debug!("Sending key data: {:?}", data);
            let _ = child.write_all(&data);
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
        let content = self.content_rect();
        for tab in &mut self.tabs {
            if let Some(root) = &mut tab.root {
                let layout = root.compute_layout(content);
                for (pid, rect) in layout {
                    let cols = (rect.width as f32 / cell_size.width) as usize;
                    let rows = (rect.height as f32 / cell_size.height) as usize;
                    if cols > 0 && rows > 0 {
                        root.resize_pane(pid, cols, rows);
                    }
                }
            }
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
        let content = self.content_rect();
        for tab in &mut self.tabs {
            if let Some(root) = &mut tab.root {
                let layout = root.compute_layout(content);
                for (pid, rect) in layout {
                    let cols = (rect.width as f32 / cell_size.width) as usize;
                    let rows = (rect.height as f32 / cell_size.height) as usize;
                    if cols > 0 && rows > 0 {
                        root.resize_pane(pid, cols, rows);
                    }
                }
            }
        }
        self.needs_redraw = true;
    }

    fn pane_at_pixel(&self, px: f64, py: f64) -> Option<(PaneId, u16, u16)> {
        if self.tabs.is_empty() {
            return None;
        }
        let tab = &self.tabs[self.active_tab];
        let Some(root) = &tab.root else { return None };
        let Some(renderer) = &self.renderer else {
            return None;
        };
        let cell_size = renderer.cell_size();
        let content = self.content_rect();
        let layout = root.compute_layout(content);
        for (pid, rect) in &layout {
            let rx = rect.x as f64;
            let ry = rect.y as f64;
            let rw = rect.width as f64;
            let rh = rect.height as f64;
            if px >= rx && px < rx + rw && py >= ry && py < ry + rh {
                let col = ((px - rx) / cell_size.width as f64) as u16;
                let row = ((py - ry) / cell_size.height as f64) as u16;
                return Some((*pid, col, row));
            }
        }
        None
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
                if let Some(window) = &self.window {
                    let window_width = window.inner_size().width as f64;
                    let scrollbar_width = 12.0;
                    if self.mouse_pixel.0 >= window_width - scrollbar_width
                        && self.mouse_pixel.1 >= self.tab_bar_height as f64
                    {
                        let tab = &self.tabs[self.active_tab];
                        let active_pane = tab.active_pane;
                        if let Some(root) = &tab.root {
                            if let Some((terminal, _, scroll_offset)) =
                                root.find_leaf_ref(active_pane)
                            {
                                let scrollback_len = terminal.screen().scrollback().len();
                                if scrollback_len > 0 {
                                    self.scrollbar_dragging = true;
                                    self.scrollbar_drag_start_y = self.mouse_pixel.1;
                                    self.scrollbar_drag_start_offset = scroll_offset;
                                    return;
                                }
                            }
                        }
                    }
                }
            } else if self.scrollbar_dragging {
                self.scrollbar_dragging = false;
                return;
            }
        }

        if button == MouseButton::Left && state == ElementState::Pressed {
            if let Some((pid, _, _)) = self.pane_at_pixel(self.mouse_pixel.0, self.mouse_pixel.1) {
                let tab = &mut self.tabs[self.active_tab];
                if tab.active_pane != pid {
                    tab.active_pane = pid;
                    tab.sync_title();
                    self.needs_redraw = true;
                }
            }
        }

        let tab = &mut self.tabs[self.active_tab];
        let active_pane = tab.active_pane;
        let root = match tab.root.as_mut() {
            Some(r) => r,
            None => return,
        };
        let (terminal, child, scroll_offset, _) = match root.find_leaf_mut(active_pane) {
            Some(leaf) => leaf,
            None => return,
        };
        let modes = terminal.screen().modes().clone();

        if !modes.mouse_tracking_enabled() {
            if button == MouseButton::Left {
                let col = self.mouse_cell.0 as usize;
                let row = self.mouse_cell.1 as isize - *scroll_offset as isize;
                if state == ElementState::Pressed {
                    terminal
                        .screen_mut()
                        .selection_mut()
                        .start(Point::new(col, row), SelectionType::Normal);
                    self.needs_redraw = true;
                } else {
                    terminal.screen_mut().selection_mut().finish();
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
            let _ = child.write_all(&data);
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

        if self.scrollbar_dragging {
            if let Some(window) = &self.window {
                let tab = &mut self.tabs[self.active_tab];
                let active_pane = tab.active_pane;
                if let Some(root) = &mut tab.root {
                    if let Some((terminal, _, scroll_offset, _)) = root.find_leaf_mut(active_pane) {
                        let window_height = (window.inner_size().height as f64
                            - self.tab_bar_height as f64)
                            .max(1.0);
                        let scrollback_len = terminal.screen().scrollback().len();
                        let visible_rows = terminal.screen().rows();
                        if scrollback_len > 0 && window_height > 0.0 {
                            let delta_y = position.y - self.scrollbar_drag_start_y;
                            let total_lines = scrollback_len + visible_rows;
                            let thumb_height = ((visible_rows as f64 / total_lines as f64)
                                * window_height)
                                .max(20.0);
                            let scroll_range = window_height - thumb_height;
                            if scroll_range > 0.0 {
                                let scroll_delta =
                                    (-delta_y / scroll_range * scrollback_len as f64) as isize;
                                let new_offset = (self.scrollbar_drag_start_offset as isize
                                    + scroll_delta)
                                    .max(0)
                                    .min(scrollback_len as isize)
                                    as usize;
                                if new_offset != *scroll_offset {
                                    *scroll_offset = new_offset;
                                    self.needs_redraw = true;
                                }
                            }
                        }
                    }
                }
            }
            return;
        }

        let (col, row) = if let Some((_, c, r)) = self.pane_at_pixel(position.x, position.y) {
            (c, r)
        } else {
            let Some(renderer) = &self.renderer else {
                return;
            };
            let cell_size = renderer.cell_size();
            let c = (position.x / cell_size.width as f64) as u16;
            let adjusted_y = (position.y - self.tab_bar_height as f64).max(0.0);
            let r = (adjusted_y / cell_size.height as f64) as u16;
            (c, r)
        };

        if col == self.mouse_cell.0 && row == self.mouse_cell.1 {
            return;
        }
        self.mouse_cell = (col, row);

        let tab = &mut self.tabs[self.active_tab];
        let active_pane = tab.active_pane;
        let root = match tab.root.as_mut() {
            Some(r) => r,
            None => return,
        };
        let (terminal, child, scroll_offset, _) = match root.find_leaf_mut(active_pane) {
            Some(leaf) => leaf,
            None => return,
        };
        let modes = terminal.screen().modes().clone();

        if !modes.mouse_tracking_enabled() && self.mouse_buttons[0] {
            let sel_col = col as usize;
            let sel_row = row as isize - *scroll_offset as isize;
            terminal
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
                let _ = child.write_all(&data);
            }
        }
    }

    fn handle_mouse_scroll(&mut self, delta: MouseScrollDelta) {
        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];
        let active_pane = tab.active_pane;
        let root = match tab.root.as_mut() {
            Some(r) => r,
            None => return,
        };
        let (terminal, child, scroll_offset, _) = match root.find_leaf_mut(active_pane) {
            Some(leaf) => leaf,
            None => return,
        };
        let modes = terminal.screen().modes().clone();
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
                let _ = child.write_all(&data);
            }
        } else {
            let scrollback_len = terminal.screen().scrollback().len();
            if lines > 0 {
                *scroll_offset = (*scroll_offset + lines as usize).min(scrollback_len);
            } else {
                *scroll_offset = scroll_offset.saturating_sub((-lines) as usize);
            }
            self.needs_redraw = true;
        }
    }

    fn handle_copy(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let tab = &self.tabs[self.active_tab];
        let active_pane = tab.active_pane;
        let Some(root) = &tab.root else { return };
        let Some((terminal, _, _)) = root.find_leaf_ref(active_pane) else {
            return;
        };
        let screen = terminal.screen();
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

    fn handle_paste(&mut self) {
        let Some(clipboard) = &mut self.clipboard else {
            log::warn!("Clipboard not available");
            return;
        };
        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];
        let active_pane = tab.active_pane;
        let root = match tab.root.as_mut() {
            Some(r) => r,
            None => return,
        };
        let (terminal, child, _, _) = match root.find_leaf_mut(active_pane) {
            Some(leaf) => leaf,
            None => return,
        };
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

    fn handle_find(&mut self) {
        log::info!("Find requested (Ctrl+Shift+F) - search UI not yet implemented");
    }

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

    fn handle_focus(&mut self, focused: bool) {
        self.focused = focused;
        if self.tabs.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active_tab];
        let active_pane = tab.active_pane;
        if let Some(root) = &mut tab.root {
            if let Some((terminal, child, _, _)) = root.find_leaf_mut(active_pane) {
                if terminal.screen().modes().focus_events {
                    let data = encode_focus(focused);
                    let _ = child.write_all(&data);
                }
            }
        }
    }

    fn poll_pty(&mut self) {
        let mut buf = [0u8; 65536];
        for (tab_idx, tab) in self.tabs.iter_mut().enumerate() {
            let is_active_tab = tab_idx == self.active_tab;
            let active_pane = tab.active_pane;
            if let Some(root) = &mut tab.root {
                let needs_redraw = &mut self.needs_redraw;
                let window = &self.window;
                root.for_each_leaf_mut(&mut |pane_id, terminal, child, scroll_offset, title| {
                    let mut received_output = false;
                    loop {
                        match child.pty_mut().try_read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                terminal.process(&buf[..n]);
                                received_output = true;
                                if is_active_tab
                                    && pane_id == active_pane
                                    && !terminal.is_synchronized_output()
                                {
                                    *needs_redraw = true;
                                }
                            }
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                            Err(_) => break,
                        }
                    }
                    if received_output && *scroll_offset > 0 {
                        *scroll_offset = 0;
                    }
                    if terminal.take_title_changed() {
                        *title = terminal.title().to_string();
                        if is_active_tab && pane_id == active_pane {
                            if let Some(w) = window {
                                w.set_title(title);
                            }
                        }
                    }
                    if terminal.take_bell() {
                        log::debug!("Bell!");
                    }
                    let responses = terminal.take_pending_responses();
                    for response in responses {
                        if let Err(e) = child.write_all(&response) {
                            log::warn!("Failed to send response to PTY: {}", e);
                        }
                    }
                });
            }
            if is_active_tab {
                tab.sync_title();
            }
        }
    }

    fn render(&mut self) {
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        if self.tabs.is_empty() {
            return;
        }
        let tab = &self.tabs[self.active_tab];
        let Some(root) = &tab.root else { return };
        let active_pane_id = tab.active_pane;
        let has_multiple_panes = root.pane_count() > 1;
        let width = renderer.current_width();
        let height = renderer.current_height();
        let tab_bar_height = self.tab_bar_height;
        let content = PaneRect {
            x: 0,
            y: tab_bar_height,
            width,
            height: height.saturating_sub(tab_bar_height),
        };
        let layout = root.compute_layout(content);
        let dividers = root.compute_dividers(content);
        let mut pane_infos: Vec<PaneRenderInfo<'_>> = Vec::new();
        for (pane_id, rect) in &layout {
            if let Some((terminal, _, scroll_offset)) = root.find_leaf_ref(*pane_id) {
                let screen = terminal.screen();
                let selection = screen.selection();
                let is_active = *pane_id == active_pane_id && has_multiple_panes;
                pane_infos.push(PaneRenderInfo {
                    screen,
                    selection,
                    scroll_offset,
                    rect: *rect,
                    is_active,
                });
            }
        }
        let tab_infos: Vec<TabInfo<'_>> = self
            .tabs
            .iter()
            .map(|t| TabInfo { title: &t.title })
            .collect();
        if let Err(e) = renderer.render(
            &pane_infos,
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

    fn check_child(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }
        let mut tabs_to_remove = Vec::new();
        for (tab_idx, tab) in self.tabs.iter_mut().enumerate() {
            if let Some(root) = tab.root.take() {
                let dead_ids = root.dead_pane_ids();
                if dead_ids.is_empty() {
                    tab.root = Some(root);
                    continue;
                }
                let mut current = Some(root);
                for dead_id in &dead_ids {
                    if let Some(r) = current.take() {
                        current = r.remove_pane(*dead_id);
                    }
                }
                match current {
                    Some(remaining) => {
                        if dead_ids.contains(&tab.active_pane) {
                            let ids = remaining.leaf_ids();
                            tab.active_pane = ids.first().copied().unwrap_or(0);
                        }
                        tab.root = Some(remaining);
                        tab.sync_title();
                    }
                    None => {
                        tabs_to_remove.push(tab_idx);
                    }
                }
            } else {
                tabs_to_remove.push(tab_idx);
            }
        }
        for idx in tabs_to_remove.into_iter().rev() {
            self.tabs.remove(idx);
        }
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len().saturating_sub(1);
        }
        !self.tabs.is_empty()
    }
}
